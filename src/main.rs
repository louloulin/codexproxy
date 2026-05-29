mod codex;
mod config;
mod db;
mod setup;
mod error;
mod error_response;
mod handlers;
mod logging;
mod models;
mod protocol;
mod providers;
mod server;
mod transform;
mod auth;
mod sse;
mod streaming_new;
mod util;
mod integration;
mod upstream;

use crate::config::Config;
use crate::logging::init_logging;
use crate::server::create_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load()?;

    // Initialize logging after config is loaded so file sink and level come from config.
    init_logging(&config.logging)?;

    tracing::info!(
        "Starting OpenAI Proxy Server on {}:{}",
        config.server.host,
        config.server.port
    );

    // Create and run server (with DB and auth)
    create_server(config).await?;

    Ok(())
}
