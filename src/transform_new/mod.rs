//! New Transform Layer
//!
//! This module provides complete transformation between:
//! - Responses API ↔ Chat Completions API
//!
//! It supersedes the old `transform` module with full support for:
//! - Tool/function conversion
//! - Reasoning mode handling
//! - Streaming chunk aggregation
//! - MiniMax/MiMo compatibility features

pub mod req_to_chat;
pub mod chat_to_responses;
pub mod compat;

pub use req_to_chat::{responses_to_chat, ReqToChatOptions};
pub use chat_to_responses::{chat_to_responses, ChatToResponsesOptions, chat_chunks_to_responses_output};
pub use compat::{apply_compat, CompatOptions};
