use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, Input, Password, Select, Confirm};
use std::process::Command;

use crate::client::{detect_available_clients, ClientType};
use crate::connection::Connection;
use crate::db::Database;

/// Prompts for and sets up master password (first time)
pub fn setup_master_password(db: &mut Database) -> Result<()> {
    println!("Welcome to pgconnect! Let's set up your master password.");
    println!("This password will be used to encrypt your database credentials.\n");

    let password: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter master password")
        .with_confirmation("Confirm master password", "Passwords don't match")
        .interact()?;

    db.setup_master_password(&password)?;
    println!("\nMaster password set successfully!\n");
    Ok(())
}

/// Prompts for master password and unlocks the database
pub fn unlock_database(db: &mut Database) -> Result<()> {
    loop {
        let password: String = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter master password")
            .interact()?;

        if db.unlock(&password)? {
            return Ok(());
        }

        println!("Incorrect password. Please try again.\n");
    }
}

/// Prompts user to select preferred client (first time setup)
pub fn select_preferred_client(db: &Database) -> Result<ClientType> {
    let available = detect_available_clients();

    if available.is_empty() {
        return Err(anyhow!("No PostgreSQL clients found. Please install psql or pgcli."));
    }

    if available.len() == 1 {
        let client = available[0];
        db.set_preferred_client(client.as_str())?;
        println!("Using {} as default client.\n", client.as_str());
        return Ok(client);
    }

    // Both available - prompt user
    println!("Multiple PostgreSQL clients detected. Please choose your preferred client:\n");
    
    let items: Vec<String> = available.iter().map(|c| c.as_str().to_string()).collect();
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select preferred client")
        .items(&items)
        .default(0)
        .interact_opt()?;

    match selection {
        Some(index) => {
            let client = available[index];
            db.set_preferred_client(client.as_str())?;
            println!("\n{} set as default client.\n", client.as_str());
            Ok(client)
        }
        None => {
            // Default to first available if user cancels
            let client = available[0];
            db.set_preferred_client(client.as_str())?;
            Ok(client)
        }
    }
}

/// Changes the preferred client (can be called anytime via CLI command)
pub fn change_preferred_client(db: &Database) -> Result<()> {
    let available = detect_available_clients();

    if available.is_empty() {
        return Err(anyhow!("No PostgreSQL clients found. Please install psql or pgcli."));
    }

    // Show current preference if set
    if let Ok(Some(current)) = db.get_preferred_client() {
        if let Some(client_type) = ClientType::from_str(&current) {
            println!("Current preferred client: {}\n", client_type.as_str());
        }
    }

    if available.len() == 1 {
        let client = available[0];
        db.set_preferred_client(client.as_str())?;
        println!("{} is the only available client and has been set as default.\n", client.as_str());
        return Ok(());
    }

    // Multiple available - prompt user
    println!("Available PostgreSQL clients:\n");
    
    let items: Vec<String> = available.iter().map(|c| c.as_str().to_string()).collect();
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select preferred client")
        .items(&items)
        .default(0)
        .interact_opt()?;

    match selection {
        Some(index) => {
            let client = available[index];
            db.set_preferred_client(client.as_str())?;
            println!("\n{} set as preferred client.\n", client.as_str());
            Ok(())
        }
        None => {
            println!("No change made.");
            Ok(())
        }
    }
}

/// Ensures the database is initialized, unlocked, and has a preferred client set
pub fn ensure_unlocked(db: &mut Database) -> Result<()> {
    if !db.is_initialized()? {
        setup_master_password(db)?;
        // Set up preferred client on first initialization
        select_preferred_client(db)?;
    } else if !db.is_unlocked() {
        unlock_database(db)?;
    } else {
        // Check if preferred client is set, if not, set it
        if db.get_preferred_client()?.is_none() {
            select_preferred_client(db)?;
        }
    }
    Ok(())
}

/// Prompts user for connection details
pub fn prompt_connection_details(existing: Option<&Connection>) -> Result<Connection> {
    let theme = ColorfulTheme::default();

    let name: String = Input::with_theme(&theme)
        .with_prompt("Connection name")
        .with_initial_text(existing.map(|c| c.name.clone()).unwrap_or_default())
        .interact_text()?;

    let host: String = Input::with_theme(&theme)
        .with_prompt("Host")
        .with_initial_text(existing.map(|c| c.host.clone()).unwrap_or_else(|| "localhost".to_string()))
        .interact_text()?;

    let port: u16 = Input::with_theme(&theme)
        .with_prompt("Port")
        .with_initial_text(existing.map(|c| c.port.to_string()).unwrap_or_else(|| "5432".to_string()))
        .interact_text()?;

    let database: String = Input::with_theme(&theme)
        .with_prompt("Database name")
        .with_initial_text(existing.map(|c| c.database.clone()).unwrap_or_default())
        .interact_text()?;

    let username: String = Input::with_theme(&theme)
        .with_prompt("Username")
        .with_initial_text(existing.map(|c| c.username.clone()).unwrap_or_default())
        .interact_text()?;

    let password: String = Password::with_theme(&theme)
        .with_prompt("Password")
        .interact()?;

    Ok(Connection::new(name, host, port, database, username, password))
}

/// Shows interactive connection selector and launches the preferred client
pub fn interactive_select(db: &Database) -> Result<()> {
    let connections = db.get_all_connections()?;

    if connections.is_empty() {
        println!("No connections saved. Use 'pgconnect add' to add one.");
        return Ok(());
    }

    // Get preferred client
    let client_str = db.get_preferred_client()?
        .ok_or_else(|| anyhow!("Preferred client not set. Please run setup again."))?;
    let client = ClientType::from_str(&client_str)
        .ok_or_else(|| anyhow!("Invalid preferred client: {}", client_str))?;

    let items: Vec<String> = connections.iter().map(|c| c.display_name()).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a connection")
        .items(&items)
        .default(0)
        .interact_opt()?;

    match selection {
        Some(index) => {
            let conn = &connections[index];
            launch_client(conn, client)?;
        }
        None => {
            println!("No selection made.");
        }
    }

    Ok(())
}

/// Launches the appropriate PostgreSQL client with the given connection
pub fn launch_client(conn: &Connection, client: ClientType) -> Result<()> {
    println!("Connecting to {} using {}...\n", conn.display_name(), client.as_str());

    match client {
        ClientType::Psql => {
            // Set PGPASSWORD environment variable for psql
            let status = Command::new("psql")
                .env("PGPASSWORD", &conn.password)
                .arg("-h")
                .arg(&conn.host)
                .arg("-p")
                .arg(conn.port.to_string())
                .arg("-U")
                .arg(&conn.username)
                .arg("-d")
                .arg(&conn.database)
                .status()?;

            if !status.success() {
                return Err(anyhow!("psql exited with error"));
            }
        }
        ClientType::Pgcli => {
            // pgcli uses connection URI format: postgresql://user:password@host:port/database
            let encoded_password = urlencoding::encode(&conn.password);
            let uri = format!(
                "postgresql://{}:{}@{}:{}/{}",
                urlencoding::encode(&conn.username),
                encoded_password,
                urlencoding::encode(&conn.host),
                conn.port,
                urlencoding::encode(&conn.database)
            );

            let status = Command::new("pgcli")
                .arg(&uri)
                .status()?;

            if !status.success() {
                return Err(anyhow!("pgcli exited with error"));
            }
        }
    }

    Ok(())
}

/// Lists all connections
pub fn list_connections(db: &Database) -> Result<()> {
    let connections = db.get_all_connections()?;

    if connections.is_empty() {
        println!("No connections saved.");
        return Ok(());
    }

    println!("Saved connections:\n");
    for conn in connections {
        println!("  {} ", conn.name);
        println!("    Host:     {}:{}", conn.host, conn.port);
        println!("    Database: {}", conn.database);
        println!("    Username: {}", conn.username);
        println!();
    }

    Ok(())
}

/// Confirms removal of a connection
pub fn confirm_remove(name: &str) -> Result<bool> {
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Remove connection '{}'?", name))
        .default(false)
        .interact()?;

    Ok(confirmed)
}
