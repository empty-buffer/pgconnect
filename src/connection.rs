#[derive(Debug, Clone)]
pub struct Connection {
    #[allow(dead_code)]
    pub id: i64,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl Connection {
    pub fn new(
        name: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
    ) -> Self {
        Self {
            id: 0,
            name,
            host,
            port,
            database,
            username,
            password,
        }
    }

    pub fn display_name(&self) -> String {
        format!("{} ({}@{}:{}/{})", self.name, self.username, self.host, self.port, self.database)
    }
}
