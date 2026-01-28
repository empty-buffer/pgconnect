use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, Input, Password, Select, Confirm};
use std::process::Command;

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

/// Ensures the database is initialized and unlocked
pub fn ensure_unlocked(db: &mut Database) -> Result<()> {
    if !db.is_initialized()? {
        setup_master_password(db)?;
    } else if !db.is_unlocked() {
        unlock_database(db)?;
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

/// Shows interactive connection selector and launches psql
pub fn interactive_select(db: &Database) -> Result<()> {
    let connections = db.get_all_connections()?;

    if connections.is_empty() {
        println!("No connections saved. Use 'pgconnect add' to add one.");
        return Ok(());
    }

    let items: Vec<String> = connections.iter().map(|c| c.display_name()).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a connection")
        .items(&items)
        .default(0)
        .interact_opt()?;

    match selection {
        Some(index) => {
            let conn = &connections[index];
            launch_psql(conn)?;
        }
        None => {
            println!("No selection made.");
        }
    }

    Ok(())
}

/// Launches psql with the given connection
pub fn launch_psql(conn: &Connection) -> Result<()> {
    println!("Connecting to {}...\n", conn.display_name());

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
