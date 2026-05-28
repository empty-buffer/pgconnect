mod client;
mod connection;
mod crypto;
mod db;
mod error;
mod ui;

use self::error::Result;
// use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::db::Database;
use crate::ui::{
    change_preferred_client, confirm_remove, ensure_unlocked, interactive_select, list_connections,
    prompt_connection_details, show_status,
};

#[derive(Parser)]
#[command(name = "pgconnect")]
#[command(about = "PostgreSQL connection manager with encrypted storage")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new connection
    Add,
    /// List all saved connections
    List,
    /// Edit an existing connection
    Edit {
        /// Name of the connection to edit
        name: String,
    },
    /// Remove a connection
    Remove {
        /// Name of the connection to remove
        name: String,
    },
    /// Set preferred client (psql, pgcli, mongosh, or redis-cli)
    SetClient,
    /// Show client/tool installation status
    Status,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut db = Database::open()?;

    match cli.command {
        None => {
            // Interactive selector mode
            ensure_unlocked(&mut db)?;
            interactive_select(&db)?;
        }
        Some(Commands::Add) => {
            ensure_unlocked(&mut db)?;
            let conn = prompt_connection_details(None)?;

            match db.add_connection(&conn) {
                Ok(_) => println!("\nConnection '{}' added successfully!", conn.name),
                Err(e) => {
                    if e.to_string().contains("UNIQUE constraint failed") {
                        println!(
                            "\nError: A connection with name '{}' already exists.",
                            conn.name
                        );
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Some(Commands::List) => {
            ensure_unlocked(&mut db)?;
            list_connections(&db)?;
        }
        Some(Commands::Edit { name }) => {
            ensure_unlocked(&mut db)?;

            let existing = db.get_connection_by_name(&name)?;
            match existing {
                Some(conn) => {
                    println!("Editing connection '{}'\n", name);
                    let updated = prompt_connection_details(Some(&conn))?;
                    match db.update_connection(&name, &updated) {
                        Ok(_) => println!("\nConnection '{}' updated successfully!", updated.name),
                        Err(e) => {
                            if e.to_string().contains("UNIQUE constraint failed") {
                                println!(
                                    "\nError: A connection with name '{}' already exists.",
                                    updated.name
                                );
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
                None => {
                    println!("Connection '{}' not found.", name);
                }
            }
        }
        Some(Commands::Remove { name }) => {
            ensure_unlocked(&mut db)?;

            if db.get_connection_by_name(&name)?.is_none() {
                println!("Connection '{}' not found.", name);
                return Ok(());
            }

            if confirm_remove(&name)? {
                db.remove_connection(&name)?;
                println!("Connection '{}' removed.", name);
            } else {
                println!("Cancelled.");
            }
        }
        Some(Commands::SetClient) => {
            change_preferred_client(&db)?;
        }
        Some(Commands::Status) => {
            show_status()?;
        }
    }

    Ok(())
}
