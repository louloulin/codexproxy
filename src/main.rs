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
    // Load configuration
    let config = Config::load()?;
    
    // Initialize logging after config is loaded so file sink and level come from config.
    init_logging(&config.logging)?;

    tracing::info!(
        "Starting OpenAI Proxy Server on {}:{}",
        config.server.host,
        config.server.port
    );

    // Create and run server
    create_server(config).await?;

    Ok(())
}
