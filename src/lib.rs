//!
//! OpenAI Proxy Library
//!
//! This library provides an API proxy server that transforms between
//! OpenAI Chat Completions API and Responses API formats.

pub mod auth;           // Authentication module
pub mod config;
pub mod db;
pub mod error;
pub mod error_response;
pub mod handlers;
pub mod integration;
pub mod logging;
pub mod models;
pub mod protocol;
pub mod providers;
pub mod providers_new; // New provider architecture
pub mod server;
pub mod setup;         // Setup snippets module
pub mod sse;
pub mod streaming_new; // New streaming SSE layer
pub mod transform;
pub mod transform_new;  // New complete transform layer
pub mod util;          // Utility modules

// Re-export commonly used types
pub use auth::{AuthContext, AuthMode, AuthGuardResult, check_auth};
pub use config::Config;
pub use error::Error;
pub use handlers::AppState;
pub use setup::{build_snippet_bundle, build_cc_switch_files, resolve_snippet_target, CcSwitchFiles, HostConfig, ProviderTarget, SnippetBundle};

// Re-export new modules for easy access
pub use transform_new::{req_to_chat, chat_to_responses};
pub use streaming_new::{SseEventBuilder, StreamingState};
pub use providers_new::{ProviderRegistry, MimoProvider, ErrorEnhancer, EnhancedError};
