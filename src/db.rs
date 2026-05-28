#[path = "db/db.rs"]
mod database;
pub mod mongo;
pub mod postgres;
pub mod redis;

pub use database::Database;
