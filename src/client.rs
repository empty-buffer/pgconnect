use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientType {
    Psql,
    Pgcli,
}

impl ClientType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientType::Psql => "psql",
            ClientType::Pgcli => "pgcli",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "psql" => Some(ClientType::Psql),
            "pgcli" => Some(ClientType::Pgcli),
            _ => None,
        }
    }
}

/// Checks if a command exists in PATH
fn command_exists(command: &str) -> bool {
    let output = if cfg!(target_os = "windows") {
        Command::new("where")
            .arg(command)
            .output()
    } else {
        Command::new("which")
            .arg(command)
            .output()
    };

    output
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detects which PostgreSQL clients are available in the system
pub fn detect_available_clients() -> Vec<ClientType> {
    let mut available = Vec::new();

    if command_exists("psql") {
        available.push(ClientType::Psql);
    }

    if command_exists("pgcli") {
        available.push(ClientType::Pgcli);
    }

    available
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_type_from_str() {
        assert_eq!(ClientType::from_str("psql"), Some(ClientType::Psql));
        assert_eq!(ClientType::from_str("pgcli"), Some(ClientType::Pgcli));
        assert_eq!(ClientType::from_str("invalid"), None);
    }

    #[test]
    fn test_client_type_as_str() {
        assert_eq!(ClientType::Psql.as_str(), "psql");
        assert_eq!(ClientType::Pgcli.as_str(), "pgcli");
    }
}
