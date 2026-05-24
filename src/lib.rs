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

// Re-export commonly used types
pub use config::Config;
pub use error::Error;
pub use handlers::AppState;
