use crate::connection::{Connection, DbType};
use crate::error::Result;
use rusqlite::{params, Connection as SqliteConnection, OptionalExtension};

pub struct RedisRepo;

impl RedisRepo {
    pub fn insert(
        conn: &SqliteConnection,
        c: &Connection,
        encrypted_password: Vec<u8>,
        nonce: Vec<u8>,
    ) -> Result<i64> {
        conn.execute(
            "INSERT INTO redis_connections
             (name, host, port, database, username, encrypted_password, nonce)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                c.name,
                c.host,
                c.port as i64,
                c.database,
                c.username,
                encrypted_password,
                nonce
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn fetch_all(
        conn: &SqliteConnection,
    ) -> Result<Vec<(i64, String, String, i64, String, String, Vec<u8>, Vec<u8>)>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, host, port, database, username, encrypted_password, nonce
             FROM redis_connections",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;
        Ok(rows)
    }

    pub fn get_by_name(
        conn: &SqliteConnection,
        name: &str,
    ) -> Result<Option<(i64, String, i64, String, String, Vec<u8>, Vec<u8>)>> {
        let row = conn
            .query_row(
                "SELECT id, host, port, database, username, encrypted_password, nonce
                 FROM redis_connections WHERE name = ?",
                params![name],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn delete_by_name(conn: &SqliteConnection, name: &str) -> Result<usize> {
        Ok(conn.execute(
            "DELETE FROM redis_connections WHERE name = ?",
            params![name],
        )?)
    }

    pub fn count(conn: &SqliteConnection) -> Result<i64> {
        Ok(conn.query_row("SELECT COUNT(*) FROM redis_connections", [], |r| r.get(0))?)
    }

    pub fn into_connection(
        id: i64,
        name: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
    ) -> Connection {
        Connection {
            id,
            name,
            db_type: DbType::Redis,
            host,
            port,
            database,
            auth_source: String::new(),
            mongo_uri: String::new(),
            username,
            password,
        }
    }
}
