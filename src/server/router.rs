use crate::config::Config;
use crate::handlers::{self, AppState};
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub async fn create_server(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Create application state with providers
    let state = Arc::new(AppState::new(config.clone()));

    let app = Router::new()
        .route("/", get(handlers::health_check))
        .route("/health", get(handlers::health_check))
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .route("/v1/responses", post(handlers::responses))
        // Zhipu direct endpoint - bypasses transform layer
        .route(
            "/v1/providers/zhipu/chat/completions",
            post(handlers::zhipu_chat_completions),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
