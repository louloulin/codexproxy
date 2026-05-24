//! Streaming SSE event generation module
//! 
//! This module handles converting Chat Completions streaming chunks
//! to Responses API SSE events.

mod sse_builder;
mod streaming_state;

pub use sse_builder::SseEventBuilder;
pub use streaming_state::StreamingState;
