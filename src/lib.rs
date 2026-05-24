//! OpenAI Proxy Library
//!
//! This library provides an API proxy server that transforms between
//! OpenAI Chat Completions API and Responses API formats.

pub mod config;
pub mod error;
pub mod error_response;
pub mod handlers;
pub mod logging;
pub mod models;
pub mod protocol;
pub mod providers;
pub mod server;
pub mod sse;
pub mod transform;
pub mod transform_new; // New complete transform layer
pub mod streaming_new; // New streaming SSE layer
pub mod providers_new; // New provider architecture

// Re-export commonly used types
pub use config::Config;
pub use error::Error;
pub use handlers::AppState;

// Re-export new modules for easy access
pub use transform_new::{req_to_chat, chat_to_responses};
pub use streaming_new::{SseEventBuilder, StreamingState};
pub use providers_new::{ProviderRegistry, MimoProvider, ErrorEnhancer, EnhancedError};
pub mod integration; // Integration examples and usage demonstrations
pub mod db; // Database schema and repositories
