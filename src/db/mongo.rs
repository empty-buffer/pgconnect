use crate::connection::{Connection, DbType};
use crate::error::Result;
use rusqlite::{params, Connection as SqliteConnection, OptionalExtension};

pub struct MongoRepo;

pub type MongoRow = (
    i64,
    String,
    String,
    i64,
    String,
    String,
    String,
    String,
    Vec<u8>,
    Vec<u8>,
);

impl MongoRepo {
    pub fn insert(
        conn: &SqliteConnection,
        c: &Connection,
        encrypted_password: Vec<u8>,
        nonce: Vec<u8>,
    ) -> Result<i64> {
        conn.execute(
            "INSERT INTO mongodb_connections
             (name, host, port, database, auth_source, mongo_uri, username, encrypted_password, nonce)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                c.name,
                c.host,
                c.port as i64,
                c.database,
                c.auth_source,
                c.mongo_uri,
                c.username,
                encrypted_password,
                nonce
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn fetch_all(conn: &SqliteConnection) -> Result<Vec<MongoRow>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, host, port, database, auth_source, mongo_uri, username, encrypted_password, nonce
             FROM mongodb_connections",
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
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                    row.get::<_, Vec<u8>>(9)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;
        Ok(rows)
    }

    pub fn get_by_name(conn: &SqliteConnection, name: &str) -> Result<Option<MongoRow>> {
        let row = conn
            .query_row(
                "SELECT id, host, port, database, auth_source, mongo_uri, username, encrypted_password, nonce
                 FROM mongodb_connections WHERE name = ?",
                params![name],
                |row| {
                    Ok((
                        row.get(0)?,
                        name.to_string(),
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn delete_by_name(conn: &SqliteConnection, name: &str) -> Result<usize> {
        Ok(conn.execute(
            "DELETE FROM mongodb_connections WHERE name = ?",
            params![name],
        )?)
    }

    pub fn count(conn: &SqliteConnection) -> Result<i64> {
        Ok(conn.query_row("SELECT COUNT(*) FROM mongodb_connections", [], |r| r.get(0))?)
    }

    pub fn into_connection(row: MongoRow, password: String) -> Connection {
        let (id, name, host, port, database, auth_source, mongo_uri, username, _encrypted, _nonce) =
            row;
        Connection {
            id,
            name,
            db_type: DbType::Mongodb,
            host,
            port: port as u16,
            database,
            auth_source,
            mongo_uri,
            username,
            password,
        }
    }
}
