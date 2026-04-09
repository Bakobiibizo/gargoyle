use std::path::Path;
use tracing::Level;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

use crate::config::GargoyleConfig;

/// Initialize the tracing subscriber with structured logging.
///
/// Configures:
/// - Console output with colors (debug builds) or JSON (release builds)
/// - File output with daily rotation
/// - Environment-based filtering via RUST_LOG or config
pub fn init_logging() {
    let config = &GargoyleConfig::global().logging;

    // Build the env filter from config or RUST_LOG
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    // Console layer - pretty for dev, JSON for release
    let console_layer = if cfg!(debug_assertions) {
        fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE)
            .boxed()
    } else {
        fmt::layer()
            .json()
            .with_target(true)
            .with_span_events(FmtSpan::CLOSE)
            .boxed()
    };

    // File layer with daily rotation (if configured)
    if config.file_enabled {
        let log_dir = Path::new(&config.log_dir);
        let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, &config.file_prefix);

        let file_layer = fmt::layer()
            .json()
            .with_writer(file_appender)
            .with_target(true)
            .with_span_events(FmtSpan::CLOSE);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .init();
    }

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Gargoyle logging initialized"
    );
}

/// Log level from string (for config deserialization)
pub fn parse_level(level: &str) -> Level {
    match level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}
