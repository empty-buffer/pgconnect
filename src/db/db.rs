use crate::client::ClientType;
use crate::connection::{Connection, DbType};
use crate::crypto::{
    decrypt_password, derive_key, encrypt_password, generate_salt, hash_master_password,
    verify_master_password,
};
use crate::db::mongo::MongoRepo;
use crate::db::postgres::PostgresRepo;
use crate::db::redis::RedisRepo;
use crate::error::{Error, Result};
use directories::ProjectDirs;
use rusqlite::{params, Connection as SqliteConnection};
use std::fs;
use std::path::PathBuf;

pub struct Database {
    conn: SqliteConnection,
    encryption_key: Option<[u8; 32]>,
}

struct SqlMigration {
    version: i64,
    name: String,
    sql: String,
}

const MIGRATION_0001_INIT: &str = include_str!("../../migrations/0001_init.sql");
const MIGRATION_0002_LEGACY_SPLIT: &str = include_str!("../../migrations/0002_legacy_split.sql");

impl Database {
    pub fn open() -> Result<Self> {
        let db_path = Self::get_db_path()?;
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = SqliteConnection::open(&db_path)?;
        let db = Self {
            conn,
            encryption_key: None,
        };
        db.init_schema()?;
        Ok(db)
    }

    fn get_db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "pgconnect", "pgconnect")
            .ok_or_else(|| Error::core("Could not determine data directory"))?;
        Ok(proj_dirs.data_dir().join("pgconnect.db"))
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        self.apply_migrations()?;
        Ok(())
    }

    fn apply_migrations(&self) -> Result<()> {
        for migration in Self::load_sql_migrations()? {
            if self.migration_applied(migration.version)? {
                continue;
            }

            if migration.version == 2 {
                if !self.table_exists("connections")? {
                    self.mark_migration_applied(migration.version, &migration.name)?;
                    continue;
                }
                self.prepare_legacy_connections_table()?;
            }

            self.conn.execute_batch(&migration.sql)?;
            self.mark_migration_applied(migration.version, &migration.name)?;
        }

        Ok(())
    }

    fn load_sql_migrations() -> Result<Vec<SqlMigration>> {
        Ok(vec![
            SqlMigration {
                version: 1,
                name: "0001_init.sql".to_string(),
                sql: MIGRATION_0001_INIT.to_string(),
            },
            SqlMigration {
                version: 2,
                name: "0002_legacy_split.sql".to_string(),
                sql: MIGRATION_0002_LEGACY_SPLIT.to_string(),
            },
        ])
    }

    fn migration_applied(&self, version: i64) -> Result<bool> {
        let exists: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?",
            params![version],
            |row| row.get(0),
        )?;
        Ok(exists > 0)
    }

    fn mark_migration_applied(&self, version: i64, name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (?, ?)",
            params![version, name],
        )?;
        Ok(())
    }

    fn table_exists(&self, table: &str) -> Result<bool> {
        let exists: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
            params![table],
            |row| row.get(0),
        )?;
        Ok(exists > 0)
    }

    fn column_exists(&self, table: &str, column: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare(&format!("PRAGMA table_info({})", table))?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn prepare_legacy_connections_table(&self) -> Result<()> {
        if !self.column_exists("connections", "db_type")? {
            self.conn.execute(
                "ALTER TABLE connections ADD COLUMN db_type TEXT NOT NULL DEFAULT 'postgres'",
                [],
            )?;
        }
        if !self.column_exists("connections", "auth_source")? {
            self.conn.execute(
                "ALTER TABLE connections ADD COLUMN auth_source TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }
        if !self.column_exists("connections", "mongo_uri")? {
            self.conn.execute(
                "ALTER TABLE connections ADD COLUMN mongo_uri TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }
        Ok(())
    }

    pub fn is_initialized(&self) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM settings WHERE key = 'master_hash'",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn setup_master_password(&mut self, master_password: &str) -> Result<()> {
        if self.is_initialized()? {
            return Err(Error::core("Master password already set up"));
        }

        let hash = hash_master_password(master_password)?;
        let salt = generate_salt();
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES ('master_hash', ?)",
            params![hash.as_bytes()],
        )?;
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES ('salt', ?)",
            params![&salt[..]],
        )?;
        self.encryption_key = Some(derive_key(master_password, &salt)?);
        Ok(())
    }

    pub fn unlock(&mut self, master_password: &str) -> Result<bool> {
        let hash: Vec<u8> = self.conn.query_row(
            "SELECT value FROM settings WHERE key = 'master_hash'",
            [],
            |row| row.get(0),
        )?;
        let hash_str = String::from_utf8(hash)?;
        if !verify_master_password(master_password, &hash_str)? {
            return Ok(false);
        }

        let salt: Vec<u8> =
            self.conn
                .query_row("SELECT value FROM settings WHERE key = 'salt'", [], |row| {
                    row.get(0)
                })?;
        self.encryption_key = Some(derive_key(master_password, &salt)?);
        Ok(true)
    }

    pub fn is_unlocked(&self) -> bool {
        self.encryption_key.is_some()
    }

    fn get_key(&self) -> Result<&[u8; 32]> {
        self.encryption_key
            .as_ref()
            .ok_or_else(|| Error::core("Database not unlocked"))
    }

    pub fn add_connection(&self, conn: &Connection) -> Result<i64> {
        if self.get_connection_by_name(&conn.name)?.is_some() {
            return Err(Error::core(format!(
                "A connection with name '{}' already exists",
                conn.name
            )));
        }
        self.insert_connection(conn)
    }

    fn insert_connection(&self, conn: &Connection) -> Result<i64> {
        let key = self.get_key()?;
        let (encrypted_password, nonce) = encrypt_password(&conn.password, key)?;
        let nonce_vec = nonce.to_vec();

        match conn.db_type {
            DbType::Postgres => {
                PostgresRepo::insert(&self.conn, conn, encrypted_password, nonce_vec)
            }
            DbType::Mongodb => MongoRepo::insert(&self.conn, conn, encrypted_password, nonce_vec),
            DbType::Redis => RedisRepo::insert(&self.conn, conn, encrypted_password, nonce_vec),
        }
    }

    pub fn get_all_connections(&self) -> Result<Vec<Connection>> {
        let key = self.get_key()?;
        let mut all = Vec::new();

        for (id, name, host, port, database, username, enc, nonce) in
            PostgresRepo::fetch_all(&self.conn)?
        {
            let password = decrypt_password(&enc, &nonce, key)?;
            all.push(PostgresRepo::into_connection(
                id,
                name,
                host,
                port as u16,
                database,
                username,
                password,
            ));
        }

        for row in MongoRepo::fetch_all(&self.conn)? {
            let password = decrypt_password(&row.8, &row.9, key)?;
            all.push(MongoRepo::into_connection(row, password));
        }

        for (id, name, host, port, database, username, enc, nonce) in
            RedisRepo::fetch_all(&self.conn)?
        {
            let password = decrypt_password(&enc, &nonce, key)?;
            all.push(RedisRepo::into_connection(
                id,
                name,
                host,
                port as u16,
                database,
                username,
                password,
            ));
        }

        all.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(all)
    }

    pub fn get_connection_by_name(&self, name: &str) -> Result<Option<Connection>> {
        let key = self.get_key()?;

        if let Some((id, host, port, database, username, enc, nonce)) =
            PostgresRepo::get_by_name(&self.conn, name)?
        {
            let password = decrypt_password(&enc, &nonce, key)?;
            return Ok(Some(PostgresRepo::into_connection(
                id,
                name.to_string(),
                host,
                port as u16,
                database,
                username,
                password,
            )));
        }

        if let Some(row) = MongoRepo::get_by_name(&self.conn, name)? {
            let password = decrypt_password(&row.8, &row.9, key)?;
            return Ok(Some(MongoRepo::into_connection(row, password)));
        }

        if let Some((id, host, port, database, username, enc, nonce)) =
            RedisRepo::get_by_name(&self.conn, name)?
        {
            let password = decrypt_password(&enc, &nonce, key)?;
            return Ok(Some(RedisRepo::into_connection(
                id,
                name.to_string(),
                host,
                port as u16,
                database,
                username,
                password,
            )));
        }

        Ok(None)
    }

    pub fn update_connection(&self, original_name: &str, conn: &Connection) -> Result<()> {
        let existing = self
            .get_connection_by_name(original_name)?
            .ok_or_else(|| Error::core(format!("Connection '{}' not found", original_name)))?;

        if conn.name != original_name && self.get_connection_by_name(&conn.name)?.is_some() {
            return Err(Error::core(format!(
                "A connection with name '{}' already exists",
                conn.name
            )));
        }

        self.remove_from_table(existing.db_type, original_name)?;
        if let Err(e) = self.insert_connection(conn) {
            let _ = self.insert_connection(&existing);
            return Err(e);
        }
        Ok(())
    }

    fn remove_from_table(&self, db_type: DbType, name: &str) -> Result<()> {
        match db_type {
            DbType::Postgres => {
                PostgresRepo::delete_by_name(&self.conn, name)?;
            }
            DbType::Mongodb => {
                MongoRepo::delete_by_name(&self.conn, name)?;
            }
            DbType::Redis => {
                RedisRepo::delete_by_name(&self.conn, name)?;
            }
        }
        Ok(())
    }

    pub fn remove_connection(&self, name: &str) -> Result<bool> {
        let mut removed = 0usize;
        removed += PostgresRepo::delete_by_name(&self.conn, name)?;
        removed += MongoRepo::delete_by_name(&self.conn, name)?;
        removed += RedisRepo::delete_by_name(&self.conn, name)?;
        Ok(removed > 0)
    }

    #[allow(dead_code)]
    pub fn connection_count(&self) -> Result<usize> {
        Ok((PostgresRepo::count(&self.conn)?
            + MongoRepo::count(&self.conn)?
            + RedisRepo::count(&self.conn)?) as usize)
    }

    fn preferred_client_key(db_type: DbType) -> String {
        format!("preferred_client:{}", db_type.as_str())
    }

    fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT value FROM settings WHERE key = ?",
            params![key],
            |row| {
                let value: Vec<u8> = row.get(0)?;
                String::from_utf8(value).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        Box::new(e),
                    )
                })
            },
        );
        match result {
            Ok(client) => Ok(Some(client)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_preferred_client(&self, db_type: DbType) -> Result<Option<String>> {
        if let Some(client) = self.get_setting(&Self::preferred_client_key(db_type))? {
            return Ok(Some(client));
        }
        if db_type == DbType::Postgres {
            if let Some(client) = self.get_setting("preferred_client")? {
                if ClientType::from_str(&client)
                    .is_some_and(|c| c.is_compatible_with(DbType::Postgres))
                {
                    return Ok(Some(client));
                }
            }
        }
        Ok(None)
    }

    pub fn set_preferred_client(&self, db_type: DbType, client: &str) -> Result<()> {
        let client_type = ClientType::from_str(client)
            .ok_or_else(|| Error::core(format!("Unknown client: {}", client)))?;
        if !client_type.is_compatible_with(db_type) {
            let allowed: Vec<&str> = ClientType::allowed_for(db_type)
                .iter()
                .map(|c| c.as_str())
                .collect();
            return Err(Error::core(format!(
                "{} cannot be used with {} connections (allowed: {})",
                client,
                db_type.as_str(),
                allowed.join(", ")
            )));
        }

        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
            params![Self::preferred_client_key(db_type), client.as_bytes()],
        )?;
        Ok(())
    }
}
