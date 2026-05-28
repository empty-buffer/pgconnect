use crate::error::{Error, Result};
use directories::ProjectDirs;
use rusqlite::{params, Connection as SqliteConnection};
use std::fs;
use std::path::PathBuf;

use crate::client::ClientType;
use crate::connection::{Connection, DbType};
use crate::crypto::{
    decrypt_password, derive_key, encrypt_password, generate_salt, hash_master_password,
    verify_master_password,
};

pub struct Database {
    conn: SqliteConnection,
    encryption_key: Option<[u8; 32]>,
}

impl Database {
    /// Opens or creates the database at the default location
    pub fn open() -> Result<Self> {
        let db_path = Self::get_db_path()?;

        // Ensure parent directory exists
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

    /// Gets the database file path
    fn get_db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "pgconnect", "pgconnect")
            .ok_or_else(|| Error::core("Could not determine data directory"))?;

        let data_dir = proj_dirs.data_dir();
        Ok(data_dir.join("pgconnect.db"))
    }

    /// Initializes the database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value BLOB NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS connections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                db_type TEXT NOT NULL DEFAULT 'postgres',
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                database TEXT NOT NULL,
                username TEXT NOT NULL,
                encrypted_password BLOB NOT NULL,
                nonce BLOB NOT NULL
            )",
            [],
        )?;

        // Best-effort schema migration for older DBs (pre db_type column)
        if !self.column_exists("connections", "db_type")? {
            self.conn.execute(
                "ALTER TABLE connections ADD COLUMN db_type TEXT NOT NULL DEFAULT 'postgres'",
                [],
            )?;
        }

        Ok(())
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

    /// Checks if the master password has been set up
    pub fn is_initialized(&self) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM settings WHERE key = 'master_hash'",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Sets up the master password (first time setup)
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

    /// Unlocks the database with the master password
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

    /// Checks if database is unlocked
    pub fn is_unlocked(&self) -> bool {
        self.encryption_key.is_some()
    }

    /// Gets the encryption key, returning error if not unlocked
    fn get_key(&self) -> Result<&[u8; 32]> {
        self.encryption_key
            .as_ref()
            .ok_or_else(|| Error::core("Database not unlocked"))
    }

    /// Adds a new connection
    pub fn add_connection(&self, conn: &Connection) -> Result<i64> {
        let key = self.get_key()?;
        let (encrypted_password, nonce) = encrypt_password(&conn.password, key)?;

        self.conn.execute(
            "INSERT INTO connections (name, db_type, host, port, database, username, encrypted_password, nonce)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                conn.name,
                conn.db_type.as_str(),
                conn.host,
                conn.port as i64,
                conn.database,
                conn.username,
                encrypted_password,
                &nonce[..]
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Gets all connections (with decrypted passwords)
    pub fn get_all_connections(&self) -> Result<Vec<Connection>> {
        let key = self.get_key()?;

        let mut stmt = self.conn.prepare(
            "SELECT id, name, db_type, host, port, database, username, encrypted_password, nonce
             FROM connections ORDER BY name",
        )?;

        let connections = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;

        let mut result = Vec::new();
        for (id, name, db_type, host, port, database, username, encrypted_password, nonce) in
            connections
        {
            let password = decrypt_password(&encrypted_password, &nonce, key)?;
            result.push(Connection {
                id,
                name,
                db_type: crate::connection::DbType::from_str(&db_type)
                    .unwrap_or(crate::connection::DbType::Postgres),
                host,
                port: port as u16,
                database,
                username,
                password,
            });
        }

        Ok(result)
    }

    /// Gets a connection by name
    pub fn get_connection_by_name(&self, name: &str) -> Result<Option<Connection>> {
        let key = self.get_key()?;

        let result = self.conn.query_row(
            "SELECT id, name, db_type, host, port, database, username, encrypted_password, nonce
             FROM connections WHERE name = ?",
            params![name],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                ))
            },
        );

        match result {
            Ok((id, name, db_type, host, port, database, username, encrypted_password, nonce)) => {
                let password = decrypt_password(&encrypted_password, &nonce, key)?;
                Ok(Some(Connection {
                    id,
                    name,
                    db_type: crate::connection::DbType::from_str(&db_type)
                        .unwrap_or(crate::connection::DbType::Postgres),
                    host,
                    port: port as u16,
                    database,
                    username,
                    password,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Updates an existing connection (looked up by `original_name`)
    pub fn update_connection(&self, original_name: &str, conn: &Connection) -> Result<()> {
        let key = self.get_key()?;
        let (encrypted_password, nonce) = encrypt_password(&conn.password, key)?;

        let rows = self.conn.execute(
            "UPDATE connections 
             SET name = ?, db_type = ?, host = ?, port = ?, database = ?, username = ?, encrypted_password = ?, nonce = ?
             WHERE name = ?",
            params![
                conn.name,
                conn.db_type.as_str(),
                conn.host,
                conn.port as i64,
                conn.database,
                conn.username,
                encrypted_password,
                &nonce[..],
                original_name
            ],
        )?;

        if rows == 0 {
            return Err(Error::core(format!(
                "Connection '{}' not found",
                original_name
            )));
        }

        Ok(())
    }

    /// Removes a connection by name
    pub fn remove_connection(&self, name: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM connections WHERE name = ?", params![name])?;

        Ok(rows > 0)
    }

    /// Gets connection count
    #[allow(dead_code)]
    pub fn connection_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM connections", [], |row| row.get(0))?;
        Ok(count as usize)
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

    /// Gets the preferred client for a database type.
    pub fn get_preferred_client(&self, db_type: DbType) -> Result<Option<String>> {
        if let Some(client) = self.get_setting(&Self::preferred_client_key(db_type))? {
            return Ok(Some(client));
        }

        // Legacy single key (postgres only)
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

    /// Sets the preferred client for a database type (must be compatible).
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
