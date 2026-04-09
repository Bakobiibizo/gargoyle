use anyhow::Result;
use clap::Parser;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

use gargoyle_lib::config::GargoyleConfig;
use gargoyle_lib::db;
use gargoyle_lib::mcp::tools::GargoyleMcp;

#[derive(Parser)]
#[command(name = "gargoyle-mcp", about = "Gargoyle knowledge graph MCP server")]
struct Cli {
    /// Path to the SQLite database file
    #[arg(long, default_value = "./gargoyle.db")]
    db: String,

    /// Path to the config directory (containing gargoyle.toml)
    /// Defaults to ../config relative to the database file
    #[arg(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout stays clean for MCP JSON-RPC
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // Resolve config directory: explicit --config, or ../config relative to DB
    let db_path = std::path::Path::new(&cli.db);
    let config_dir = if let Some(ref config) = cli.config {
        std::path::PathBuf::from(config)
    } else if let Some(parent) = db_path.parent() {
        // DB is typically at src-tauri/gargoyle.db, config at config/
        parent.join("../config")
    } else {
        std::path::PathBuf::from("./config")
    };

    // Initialize global config before anything else uses it
    if config_dir.join("gargoyle.toml").exists() {
        tracing::info!(config_dir = %config_dir.display(), "Loading config");
        GargoyleConfig::init_from_dir(&config_dir);
    } else {
        tracing::warn!(
            config_dir = %config_dir.display(),
            "Config directory not found, using defaults"
        );
    }

    tracing::info!(db = %cli.db, "Starting gargoyle-mcp server");

    // Open the database
    let conn = db::connection::create_connection(&cli.db)?;
    db::migrations::run_migrations(&conn)?;
    tracing::info!("Database ready");

    // Create and serve the MCP server over stdio
    let server = GargoyleMcp::new(conn).serve(stdio()).await?;
    server.waiting().await?;

    Ok(())
}
