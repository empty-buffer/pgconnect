use std::process::Command;

use crate::connection::DbType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientType {
    Psql,
    Pgcli,
    Mongosh,
    RedisCli,
}

impl ClientType {
    pub const ALL: [ClientType; 4] = [
        ClientType::Psql,
        ClientType::Pgcli,
        ClientType::Mongosh,
        ClientType::RedisCli,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ClientType::Psql => "psql",
            ClientType::Pgcli => "pgcli",
            ClientType::Mongosh => "mongosh",
            ClientType::RedisCli => "redis-cli",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "psql" => Some(ClientType::Psql),
            "pgcli" => Some(ClientType::Pgcli),
            "mongosh" => Some(ClientType::Mongosh),
            "redis-cli" => Some(ClientType::RedisCli),
            _ => None,
        }
    }

    /// Clients allowed for a given connection database type.
    pub fn allowed_for(db_type: DbType) -> &'static [ClientType] {
        match db_type {
            DbType::Postgres => &[ClientType::Psql, ClientType::Pgcli],
            DbType::Mongodb => &[ClientType::Mongosh],
            DbType::Redis => &[ClientType::RedisCli],
        }
    }

    pub fn is_compatible_with(self, db_type: DbType) -> bool {
        Self::allowed_for(db_type).contains(&self)
    }
}

/// Checks if a command exists in PATH
pub fn command_exists(command: &str) -> bool {
    let output = if cfg!(target_os = "windows") {
        Command::new("where").arg(command).output()
    } else {
        Command::new("which").arg(command).output()
    };

    output.map(|o| o.status.success()).unwrap_or(false)
}

/// Installed clients compatible with `db_type`.
pub fn detect_available_clients_for(db_type: DbType) -> Vec<ClientType> {
    ClientType::allowed_for(db_type)
        .iter()
        .copied()
        .filter(|c| command_exists(c.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_type_from_str() {
        assert_eq!(ClientType::from_str("psql"), Some(ClientType::Psql));
        assert_eq!(ClientType::from_str("pgcli"), Some(ClientType::Pgcli));
        assert_eq!(ClientType::from_str("mongosh"), Some(ClientType::Mongosh));
        assert_eq!(
            ClientType::from_str("redis-cli"),
            Some(ClientType::RedisCli)
        );
        assert_eq!(ClientType::from_str("invalid"), None);
    }

    #[test]
    fn test_client_type_as_str() {
        assert_eq!(ClientType::Psql.as_str(), "psql");
        assert_eq!(ClientType::Pgcli.as_str(), "pgcli");
        assert_eq!(ClientType::Mongosh.as_str(), "mongosh");
        assert_eq!(ClientType::RedisCli.as_str(), "redis-cli");
    }

    #[test]
    fn test_client_compatibility() {
        assert!(ClientType::Psql.is_compatible_with(DbType::Postgres));
        assert!(ClientType::Pgcli.is_compatible_with(DbType::Postgres));
        assert!(!ClientType::Mongosh.is_compatible_with(DbType::Postgres));
        assert!(ClientType::Mongosh.is_compatible_with(DbType::Mongodb));
        assert!(ClientType::RedisCli.is_compatible_with(DbType::Redis));
        assert!(!ClientType::RedisCli.is_compatible_with(DbType::Mongodb));
    }
}
