//! Data models for the API
//!
//! This module contains all the data models for:
//! - Chat Completions API (OpenAI compatible)
//! - Responses API
//! - Streaming responses
//! - Provider-specific models

pub mod chat;
pub mod chat_extended; // Extended types for thinking mode, annotations, etc.
pub mod response;
pub mod streaming;

// Re-export commonly used types
pub use chat::{ChatRequest, ChatResponse};
pub use chat_extended::{
    Annotation, ChatContentPart, ContentOrParts, ExtendedMessage, ExtendedToolCall,
    ImageUrl, ReasoningEffort, ThinkingConfig, ThinkingType, UrlCitation, UserLocation,
    WebSearchTool,
};
