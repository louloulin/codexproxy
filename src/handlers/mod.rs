//! API handlers for Chat Completions and Responses endpoints
//!
//! This module implements the actual API endpoints with:
//! - Provider integration
//! - Transform layer for API conversion
//! - Streaming support
//!
//! ## Module Structure
//!
//! - `utils.rs` - AppState, helper functions, and unit tests
//! - `chat.rs` - chat_completions and zhipu_chat_completions handlers
//! - `responses.rs` - responses handler
//! - `admin/` - Admin API handlers

// Re-export utils for use in tests and other modules
pub mod admin;
pub mod chat;
pub mod responses;
pub mod utils;

// Re-export types and handlers for external use
pub use utils::AppState;
pub use chat::{chat_completions, zhipu_chat_completions};
pub use responses::responses;

/// Health check endpoint
pub async fn health_check() -> &'static str {
    "OK"
}
