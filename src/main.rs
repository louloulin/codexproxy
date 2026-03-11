mod config;
mod error;
mod error_response;
mod handlers;
mod logging;
mod models;
mod providers;
mod server;
mod transform;

use crate::config::Config;
use crate::logging::init_logging;
use crate::server::create_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging();

    // Load configuration
    let config = Config::load()?;

    tracing::info!(
        "Starting OpenAI Proxy Server on {}:{}",
        config.server.host,
        config.server.port
    );

    // Create and run server
    create_server(config).await?;

    Ok(())
}
