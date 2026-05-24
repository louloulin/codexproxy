//! New Transform Layer
//! 
//! This module provides complete transformation between:
//! - Responses API ↔ Chat Completions API
//! - Thinking/Reasoning mode handling
//! - MiniMax/MiMo compatibility features
//! - Image materialization

pub mod req_to_chat;
pub mod chat_to_responses;
pub mod compat;
pub mod thinking;
pub mod thinking_inject;
pub mod image_util;

pub use req_to_chat::{responses_to_chat, ReqToChatOptions};
pub use chat_to_responses::{chat_to_responses, ChatToResponsesOptions, chat_chunks_to_responses_output};
pub use compat::{apply_compat, CompatOptions};
pub use thinking::{
    extract_inline_think, contains_think_tags, count_think_tags,
    extract_think_content, ThinkSplitter, 
};
pub use thinking_inject::{
    should_enable_thinking, get_thinking_config, supports_thinking,
    get_reasoning_effort, should_remove_temperature, get_thinking_temperature,
};
