use crate::config::Config;
use crate::handlers::{self, AppState};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use governor::{DefaultKeyedRateLimiter, Quota};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Rate limiting state wrapper
pub struct RateLimitState {
    pub limiter: Arc<DefaultKeyedRateLimiter<String>>,
}

impl RateLimitState {
    pub fn new(requests_per_minute: u32, burst: u32) -> Self {
        // Convert requests per minute to per second quota
        let per_second = requests_per_minute / 60;
        let quota = Quota::per_second(NonZeroU32::new(per_second.max(1)).unwrap())
            .allow_burst(NonZeroU32::new(burst.max(1)).unwrap());

        let limiter = Arc::new(DefaultKeyedRateLimiter::keyed(quota));
        Self { limiter }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(rate_limit_state): State<Arc<RateLimitState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    // Skip rate limiting for health check endpoints
    let path = request.uri().path();
    if path == "/" || path == "/health" {
        return Ok(next.run(request).await);
    }

    // Extract client IP from request
    // Try X-Forwarded-For header first (for proxied requests)
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            // Try X-Real-IP header
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Check rate limit
    match rate_limit_state.limiter.check_key(&client_ip) {
        Ok(_) => Ok(next.run(request).await),
        Err(_) => {
            tracing::warn!("Rate limit exceeded for client: {}", client_ip);
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded. Please try again later.",
            ))
        }
    }
}

/// Create the application router with all routes configured
pub fn create_router(state: Arc<AppState>) -> Router {
    let config = &state.config;
    let rate_limit_state = Arc::new(RateLimitState::new(
        config.server.rate_limit.requests_per_minute,
        config.server.rate_limit.burst,
    ));

    Router::new()
        .route("/", get(handlers::health_check))
        .route("/health", get(handlers::health_check))
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .route("/v1/responses", post(handlers::responses))
        // Zhipu direct endpoint - bypasses transform layer
        .route(
            "/v1/providers/zhipu/chat/completions",
            post(handlers::zhipu_chat_completions),
        )
        // Set body limit based on configuration
        .layer(axum::extract::DefaultBodyLimit::max(config.server.body_limit))
        .layer(
            ServiceBuilder::new()
                // Request tracing with richer logs
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|request: &Request<Body>| {
                            let content_length = request
                                .headers()
                                .get(axum::http::header::CONTENT_LENGTH)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("unknown");
                            let content_type = request
                                .headers()
                                .get(axum::http::header::CONTENT_TYPE)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("unknown");
                            tracing::info_span!(
                                "http_request",
                                method = %request.method(),
                                uri = %request.uri(),
                                content_length,
                                content_type
                            )
                        })
                        .on_request(|request: &Request<_>, _span: &tracing::Span| {
                            let content_length = request
                                .headers()
                                .get(axum::http::header::CONTENT_LENGTH)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("unknown");
                            let content_type = request
                                .headers()
                                .get(axum::http::header::CONTENT_TYPE)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("unknown");
                            tracing::debug!(
                                method = %request.method(),
                                uri = %request.uri(),
                                content_length,
                                content_type,
                                "incoming request"
                            );
                        })
                        .on_response(|response: &Response, latency: std::time::Duration, _span: &tracing::Span| {
                            let status = response.status();
                            let content_length = response
                                .headers()
                                .get(axum::http::header::CONTENT_LENGTH)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("unknown");
                            let content_type = response
                                .headers()
                                .get(axum::http::header::CONTENT_TYPE)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("unknown");
                            tracing::debug!(
                                status = %status,
                                latency_ms = latency.as_millis(),
                                content_length,
                                content_type,
                                "response sent"
                            );
                        }),
                )
                // CORS layer
                .layer(CorsLayer::permissive()),
        )
        // Apply rate limiting middleware
        .layer(middleware::from_fn_with_state(
            rate_limit_state,
            rate_limit_middleware,
        ))
        .with_state(state)
}

pub async fn create_server(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Create application state with providers
    let state = Arc::new(AppState::new(config.clone()));

    let app = create_router(state);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!(
        "Server listening on {} (rate limit: {} req/min, burst: {})",
        addr,
        config.server.rate_limit.requests_per_minute,
        config.server.rate_limit.burst
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
