/// The module's database namespace: a PostgreSQL schema, a MySQL database, or
/// the ATTACHed SQLite file — chosen by the administrator's engine at run time.
pub const SCHEMA: &str = "forum";

pub mod config;
pub mod errors;
pub mod events;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod router;
pub mod services;
pub mod state;
pub mod workers;
