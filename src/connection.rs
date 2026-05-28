#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbType {
    Postgres,
    Mongodb,
    Redis,
}

impl DbType {
    pub const ALL: [DbType; 3] = [DbType::Postgres, DbType::Mongodb, DbType::Redis];

    pub fn as_str(&self) -> &'static str {
        match self {
            DbType::Postgres => "postgres",
            DbType::Mongodb => "mongodb",
            DbType::Redis => "redis",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "postgres" => Some(DbType::Postgres),
            "mongodb" => Some(DbType::Mongodb),
            "redis" => Some(DbType::Redis),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    #[allow(dead_code)]
    pub id: i64,
    pub name: String,
    pub db_type: DbType,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl Connection {
    pub fn new(
        name: String,
        db_type: DbType,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
    ) -> Self {
        Self {
            id: 0,
            name,
            db_type,
            host,
            port,
            database,
            username,
            password,
        }
    }

    pub fn display_name(&self) -> String {
        format!(
            "{} [{}] ({}@{}:{}/{})",
            self.name,
            self.db_type.as_str(),
            self.username,
            self.host,
            self.port,
            self.database
        )
    }
}
