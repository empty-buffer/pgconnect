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

    // MongoDB-specific shortcut: allow creating connection from full URI.
    if db_type == DbType::Mongodb {
        let setup_mode_items = ["Manual fields", "MongoDB URI"];
        let setup_mode = Select::with_theme(&theme)
            .with_prompt("MongoDB setup mode")
            .items(&setup_mode_items)
            .default(0)
            .interact_opt()?;

        if matches!(setup_mode, Some(1)) {
            let uri: String = Input::with_theme(&theme)
                .with_prompt("MongoDB URI")
                .with_initial_text(
                    existing
                        .map(|c| c.mongo_uri.clone())
                        .filter(|u| !u.is_empty())
                        .unwrap_or_else(|| {
                            "mongodb://user:password@localhost:27017/mydb?authSource=admin"
                                .to_string()
                        }),
                )
                .interact_text()?;

            let parsed = parse_mongodb_uri(&uri)?;
            return Ok(Connection::new(
                name,
                DbType::Mongodb,
                parsed.host,
                parsed.port,
                parsed.database,
                parsed.auth_source,
                parsed.mongo_uri,
                parsed.username,
                parsed.password,
            ));
        }
    }

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

    let auth_source: String = if db_type == DbType::Mongodb {
        Input::with_theme(&theme)
            .with_prompt("Auth source")
            .with_initial_text(
                existing
                    .map(|c| c.auth_source.clone())
                    .unwrap_or_else(|| "admin".to_string()),
            )
            .allow_empty(true)
            .interact_text()?
    } else {
        "".to_string()
    };

    let username: String = Input::with_theme(&theme)
        .with_prompt("Username")
        .with_initial_text(existing.map(|c| c.username.clone()).unwrap_or_default())
        .interact_text()?;

    let password: String = Password::with_theme(&theme)
        .with_prompt("Password")
        .interact()?;

    let mongo_uri = if db_type == DbType::Mongodb {
        build_mongodb_uri(&host, port, &database, &auth_source, &username, &password)
    } else {
        String::new()
    };

    Ok(Connection::new(
        name,
        db_type,
        host,
        port,
        database,
        auth_source,
        mongo_uri,
        username,
        password,
    ))
}

struct ParsedMongoUri {
    host: String,
    port: u16,
    database: String,
    auth_source: String,
    mongo_uri: String,
    username: String,
    password: String,
}

fn parse_mongodb_uri(uri: &str) -> Result<ParsedMongoUri> {
    let scheme_end = uri
        .find("://")
        .ok_or_else(|| Error::core("MongoDB URI must include scheme"))?;
    let scheme = &uri[..scheme_end];
    if scheme != "mongodb" && scheme != "mongodb+srv" {
        return Err(Error::core(
            "MongoDB URI must start with mongodb:// or mongodb+srv://",
        ));
    }

    let remainder = &uri[scheme_end + 3..];
    if remainder.is_empty() {
        return Err(Error::core("MongoDB URI must include host information"));
    }

    let (authority, path_and_query) = match remainder.find('/') {
        Some(i) => (&remainder[..i], &remainder[i + 1..]),
        None => (remainder, ""),
    };

    let (userinfo, hosts_part) = match authority.rsplit_once('@') {
        Some((u, h)) => (Some(u), h),
        None => (None, authority),
    };

    if hosts_part.is_empty() {
        return Err(Error::core("MongoDB URI must include a host"));
    }

    let first_host = hosts_part
        .split(',')
        .next()
        .ok_or_else(|| Error::core("MongoDB URI must include a host"))?;

    let (host, port) = parse_mongo_host_port(first_host, scheme == "mongodb+srv")?;

    let (database_path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path_and_query, ""),
    };

    let database = if database_path.is_empty() {
        "admin".to_string()
    } else {
        database_path.to_string()
    };

    let auth_source = query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(k, v)| (k == "authSource").then(|| v.to_string()))
        .unwrap_or_else(|| "admin".to_string());

    let (username, password) = match userinfo {
        Some(ui) => {
            let (u, p) = match ui.split_once(':') {
                Some((u, p)) => (u, p),
                None => (ui, ""),
            };
            (
                urlencoding::decode(u)
                    .map_err(|e| {
                        Error::core(format!("Invalid username encoding in MongoDB URI: {}", e))
                    })?
                    .into_owned(),
                urlencoding::decode(p)
                    .map_err(|e| {
                        Error::core(format!("Invalid password encoding in MongoDB URI: {}", e))
                    })?
                    .into_owned(),
            )
        }
        None => (String::new(), String::new()),
    };

    Ok(ParsedMongoUri {
        host,
        port,
        database,
        auth_source,
        mongo_uri: uri.to_string(),
        username,
        password,
    })
}

fn parse_mongo_host_port(host_part: &str, is_srv: bool) -> Result<(String, u16)> {
    if is_srv {
        return Ok((host_part.to_string(), 27017));
    }

    if let Some((host, port_str)) = host_part.rsplit_once(':') {
        let port = port_str
            .parse::<u16>()
            .map_err(|_| Error::core(format!("Invalid MongoDB port: {}", port_str)))?;
        return Ok((host.to_string(), port));
    }

    Ok((host_part.to_string(), 27017))
}

fn build_mongodb_uri(
    host: &str,
    port: u16,
    database: &str,
    auth_source: &str,
    username: &str,
    password: &str,
) -> String {
    format!(
        "mongodb://{}:{}@{}:{}/{}?authSource={}",
        urlencoding::encode(username),
        urlencoding::encode(password),
        host,
        port,
        urlencoding::encode(database),
        urlencoding::encode(if auth_source.is_empty() {
            "admin"
        } else {
            auth_source
        })
    )
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

    let uri = if conn.mongo_uri.is_empty() {
        build_mongodb_uri(
            &conn.host,
            conn.port,
            &conn.database,
            &conn.auth_source,
            &conn.username,
            &conn.password,
        )
    } else {
        conn.mongo_uri.clone()
    };

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
        if conn.db_type == DbType::Mongodb {
            println!(
                "    AuthSource: {}",
                if conn.auth_source.is_empty() {
                    "admin"
                } else {
                    &conn.auth_source
                }
            );
        }
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
