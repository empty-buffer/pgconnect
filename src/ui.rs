use crate::error::{Error, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use std::process::Command;

use crate::client::{command_exists, detect_available_clients_for, ClientType};
use crate::connection::{Connection, DbType};
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

fn allowed_client_names(db_type: DbType) -> String {
    ClientType::allowed_for(db_type)
        .iter()
        .map(|c| c.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Prompts user to select preferred client for a database type.
pub fn select_preferred_client(db: &Database, db_type: DbType) -> Result<ClientType> {
    let available = detect_available_clients_for(db_type);

    if available.is_empty() {
        return Err(Error::core(format!(
            "No client installed for {}. Install one of: {}",
            db_type.as_str(),
            allowed_client_names(db_type)
        )));
    }

    if available.len() == 1 {
        let client = available[0];
        db.set_preferred_client(db_type, client.as_str())?;
        println!(
            "Using {} as default client for {}.\n",
            client.as_str(),
            db_type.as_str()
        );
        return Ok(client);
    }

    println!(
        "Multiple clients available for {}. Choose your preferred client:\n",
        db_type.as_str()
    );

    let items: Vec<String> = available.iter().map(|c| c.as_str().to_string()).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Preferred client for {}", db_type.as_str()))
        .items(&items)
        .default(0)
        .interact_opt()?;

    let client = match selection {
        Some(index) => available[index],
        None => available[0],
    };

    db.set_preferred_client(db_type, client.as_str())?;
    println!(
        "\n{} set as preferred client for {}.\n",
        client.as_str(),
        db_type.as_str()
    );
    Ok(client)
}

/// Changes the preferred client for a database type (`pgconnect set-client`).
pub fn change_preferred_client(db: &Database) -> Result<()> {
    let db_type_items = ["postgres", "mongodb", "redis"];
    let db_type_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Database type")
        .items(&db_type_items)
        .default(0)
        .interact_opt()?;

    let db_type = match db_type_idx {
        Some(0) => DbType::Postgres,
        Some(1) => DbType::Mongodb,
        Some(2) => DbType::Redis,
        None => return Ok(()),
        _ => DbType::Postgres,
    };

    if let Ok(Some(current)) = db.get_preferred_client(db_type) {
        if let Some(client_type) = ClientType::from_str(&current) {
            println!(
                "Current preferred client for {}: {}\n",
                db_type.as_str(),
                client_type.as_str()
            );
        }
    }

    select_preferred_client(db, db_type)?;
    Ok(())
}

fn ensure_preferred_client(db: &Database, db_type: DbType) -> Result<()> {
    if db.get_preferred_client(db_type)?.is_some() {
        return Ok(());
    }

    let available = detect_available_clients_for(db_type);
    if available.is_empty() {
        return Ok(());
    }

    if available.len() == 1 {
        db.set_preferred_client(db_type, available[0].as_str())?;
        return Ok(());
    }

    // Postgres: psql and pgcli both installed — ask user
    select_preferred_client(db, db_type)?;
    Ok(())
}

/// Resolves which client to use for a connection (stored preference must match db type).
fn resolve_client(db: &Database, conn: &Connection) -> Result<ClientType> {
    let available = detect_available_clients_for(conn.db_type);
    if available.is_empty() {
        return Err(Error::core(format!(
            "No client installed for {}. Install one of: {}",
            conn.db_type.as_str(),
            allowed_client_names(conn.db_type)
        )));
    }

    if let Some(stored) = db.get_preferred_client(conn.db_type)? {
        if let Some(client) = ClientType::from_str(&stored) {
            if client.is_compatible_with(conn.db_type) && available.contains(&client) {
                return Ok(client);
            }
        }
    }

    if available.len() == 1 {
        return Ok(available[0]);
    }

    Err(Error::core(format!(
        "Preferred client not set for {}. Run: pgconnect set-client",
        conn.db_type.as_str()
    )))
}

/// Ensures the database is initialized, unlocked, and has preferred clients where possible
pub fn ensure_unlocked(db: &mut Database) -> Result<()> {
    if !db.is_initialized()? {
        setup_master_password(db)?;
    } else if !db.is_unlocked() {
        unlock_database(db)?;
    }

    for db_type in DbType::ALL {
        ensure_preferred_client(db, db_type)?;
    }

    Ok(())
}

/// Prompts user for connection details
pub fn prompt_connection_details(existing: Option<&Connection>) -> Result<Connection> {
    let theme = ColorfulTheme::default();

    let db_type_items = ["postgres", "mongodb", "redis"];
    let db_type_default = existing
        .map(|c| match c.db_type {
            DbType::Postgres => 0,
            DbType::Mongodb => 1,
            DbType::Redis => 2,
        })
        .unwrap_or(0);

    let db_type_idx = Select::with_theme(&theme)
        .with_prompt("Database type")
        .items(&db_type_items)
        .default(db_type_default)
        .interact_opt()?;

    let db_type = match db_type_idx {
        Some(0) => DbType::Postgres,
        Some(1) => DbType::Mongodb,
        Some(2) => DbType::Redis,
        None => existing.map(|c| c.db_type).unwrap_or(DbType::Postgres),
        _ => DbType::Postgres,
    };

    let name: String = Input::with_theme(&theme)
        .with_prompt("Connection name")
        .with_initial_text(existing.map(|c| c.name.clone()).unwrap_or_default())
        .interact_text()?;

    let host: String = Input::with_theme(&theme)
        .with_prompt("Host")
        .with_initial_text(
            existing
                .map(|c| c.host.clone())
                .unwrap_or_else(|| "localhost".to_string()),
        )
        .interact_text()?;

    let port: u16 =
        Input::with_theme(&theme)
            .with_prompt("Port")
            .with_initial_text(existing.map(|c| c.port.to_string()).unwrap_or_else(
                || match db_type {
                    DbType::Postgres => "5432".to_string(),
                    DbType::Mongodb => "27017".to_string(),
                    DbType::Redis => "6379".to_string(),
                },
            ))
            .interact_text()?;

    let database: String =
        Input::with_theme(&theme)
            .with_prompt(match db_type {
                DbType::Redis => "Database (redis DB number)",
                _ => "Database name",
            })
            .with_initial_text(existing.map(|c| c.database.clone()).unwrap_or_else(
                || match db_type {
                    DbType::Postgres => "".to_string(),
                    DbType::Mongodb => "admin".to_string(),
                    DbType::Redis => "0".to_string(),
                },
            ))
            .interact_text()?;

    let username: String = Input::with_theme(&theme)
        .with_prompt("Username")
        .with_initial_text(existing.map(|c| c.username.clone()).unwrap_or_default())
        .interact_text()?;

    let password: String = Password::with_theme(&theme)
        .with_prompt("Password")
        .interact()?;

    Ok(Connection::new(
        name, db_type, host, port, database, username, password,
    ))
}

/// Shows interactive connection selector and launches the preferred client
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
            let client = resolve_client(db, conn)?;
            launch_client(conn, client)?;
        }
        None => {
            println!("No selection made.");
        }
    }

    Ok(())
}

/// Launches the appropriate client with the given connection
pub fn launch_client(conn: &Connection, client: ClientType) -> Result<()> {
    if !client.is_compatible_with(conn.db_type) {
        return Err(Error::core(format!(
            "{} cannot be used with {} connections (allowed: {})",
            client.as_str(),
            conn.db_type.as_str(),
            allowed_client_names(conn.db_type)
        )));
    }

    match conn.db_type {
        DbType::Postgres => launch_postgres(conn, client),
        DbType::Mongodb => launch_mongodb(conn, client),
        DbType::Redis => launch_redis(conn, client),
    }
}

fn launch_postgres(conn: &Connection, client: ClientType) -> Result<()> {
    println!(
        "Connecting to {} using {}...\n",
        conn.display_name(),
        client.as_str()
    );

    match client {
        ClientType::Psql => {
            let status = Command::new(client.as_str())
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
                return Err(Error::core(format!(
                    "{} exited with error",
                    client.as_str()
                )));
            }
        }
        ClientType::Pgcli => {
            let encoded_password = urlencoding::encode(&conn.password);
            let uri = format!(
                "postgresql://{}:{}@{}:{}/{}",
                urlencoding::encode(&conn.username),
                encoded_password,
                urlencoding::encode(&conn.host),
                conn.port,
                urlencoding::encode(&conn.database)
            );

            let status = Command::new(client.as_str()).arg(&uri).status()?;

            if !status.success() {
                return Err(Error::core(format!(
                    "{} exited with error",
                    client.as_str()
                )));
            }
        }
        ClientType::Mongosh | ClientType::RedisCli => unreachable!(),
    }

    Ok(())
}

fn launch_mongodb(conn: &Connection, client: ClientType) -> Result<()> {
    debug_assert_eq!(client, ClientType::Mongosh);
    println!(
        "Connecting to {} using {}...\n",
        conn.display_name(),
        client.as_str()
    );

    // mongosh uses MongoDB URI format: mongodb://user:password@host:port/database
    let encoded_password = urlencoding::encode(&conn.password);
    let uri = format!(
        "mongodb://{}:{}@{}:{}/{}",
        urlencoding::encode(&conn.username),
        encoded_password,
        urlencoding::encode(&conn.host),
        conn.port,
        urlencoding::encode(&conn.database)
    );

    let status = Command::new(client.as_str()).arg(&uri).status()?;

    if !status.success() {
        return Err(Error::core(format!(
            "{} exited with error",
            client.as_str()
        )));
    }

    Ok(())
}

fn launch_redis(conn: &Connection, client: ClientType) -> Result<()> {
    debug_assert_eq!(client, ClientType::RedisCli);
    println!(
        "Connecting to {} using {}...\n",
        conn.display_name(),
        client.as_str()
    );

    let encoded_password = urlencoding::encode(&conn.password);
    let encoded_user = urlencoding::encode(&conn.username);
    let auth_part = if conn.username.is_empty() {
        format!(":{}", encoded_password)
    } else {
        format!("{}:{}", encoded_user, encoded_password)
    };

    let uri = format!(
        "redis://{}@{}:{}/{}",
        auth_part,
        urlencoding::encode(&conn.host),
        conn.port,
        urlencoding::encode(&conn.database)
    );

    let status = Command::new(client.as_str()).arg("-u").arg(&uri).status()?;

    if !status.success() {
        return Err(Error::core(format!(
            "{} exited with error",
            client.as_str()
        )));
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
        println!("  {}", conn.name);
        println!("    Type:     {}", conn.db_type.as_str());
        println!("    Host:     {}:{}", conn.host, conn.port);
        println!("    Database: {}", conn.database);
        println!("    Username: {}", conn.username);
        println!();
    }

    Ok(())
}

pub fn show_status() -> Result<()> {
    for client in ClientType::ALL {
        let installed = command_exists(client.as_str());
        println!(
            "{} - {}",
            client.as_str(),
            if installed { "installed" } else { "missing" }
        );
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
