use rusqlite::Connection;
use std::sync::Mutex;

pub mod agents;
pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod logging;
pub mod mcp;
pub mod models;
pub mod patch;
pub mod schema;
pub mod services;
pub mod validation;

pub struct AppState {
    pub db: Mutex<Option<Connection>>,
}
