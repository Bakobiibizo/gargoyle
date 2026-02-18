use rusqlite::Connection;
use std::sync::Mutex;

pub mod error;
pub mod db;
pub mod models;
pub mod schema;
pub mod patch;
pub mod validation;
pub mod services;
pub mod commands;
pub mod config;

pub struct AppState {
    pub db: Mutex<Option<Connection>>,
}
