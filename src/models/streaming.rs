//! Streaming response models
//!
//! This module contains models for handling streaming (SSE) responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::response::{
    DetailedUsage, FunctionCallOutputPayload, InputTokensDetails, McpContent, McpToolResult,
    OutputItem, OutputTokensDetails, ReasoningSummaryPart, ToolDefinition,
};

// ============================================================================
// Chat Completions Streaming (Legacy)
// ============================================================================

/// Chat Completions streaming chunk
/// This is sent as SSE data for streaming responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ChatCompletionChunk {
    /// Unique identifier for this chunk
    pub id: String,

    /// Object type
    pub object: String,

    /// Unix timestamp
    pub created: u64,

    /// Model used
    pub model: String,

    /// Choices in this chunk
    pub choices: Vec<StreamingChoice>,
}

/// Streaming choice
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct StreamingChoice {
    /// Index of this choice
    pub index: u32,

    /// Delta content (partial message)
    #[serde(default)]
    pub delta: Delta,

    /// Reason for completion
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Delta content in streaming response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Delta {
    /// Role of the message
    #[serde(default)]
    pub role: Option<String>,

    /// Content of the message
    #[serde(default)]
    pub content: Option<String>,

    /// Tool calls
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Tool call in streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ToolCall {
    /// ID of this tool call
    pub id: String,

    /// Type of tool call
    #[serde(rename = "type")]
    pub call_type: String,

    /// Function being called
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FunctionCall {
    /// Name of the function
    pub name: String,

    /// Arguments as JSON string
    pub arguments: String,
}

/// Usage information for streaming
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Usage {
    /// Tokens in the prompt
    #[serde(default)]
    pub prompt_tokens: Option<u32>,

    /// Tokens in the completion
    #[serde(default)]
    pub completion_tokens: Option<u32>,

    /// Total tokens
    #[serde(default)]
    pub total_tokens: Option<u32>,
}

/// SSE event types for streaming
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SseEvent {
    /// Chat completion chunk
    ChatCompletion(ChatCompletionChunk),
    /// Usage information
    Usage(Usage),
    /// Done signal
    Done,
}

impl SseEvent {
    /// Convert to SSE format string
    #[allow(dead_code)]
    pub fn to_sse(&self) -> String {
        match self {
            SseEvent::ChatCompletion(chunk) => {
                format!(
                    "data: {}\n\n",
                    serde_json::to_string(chunk).unwrap_or_default()
                )
            }
            SseEvent::Usage(usage) => {
                format!(
                    "data: {}\n\n",
                    serde_json::to_string(usage).unwrap_or_default()
                )
            }
            SseEvent::Done => "data: [DONE]\n\n".to_string(),
        }
    }
}

/// Helper to build streaming response
#[allow(dead_code)]
pub struct StreamingResponseBuilder {
    id: String,
    model: String,
    created: u64,
}

impl StreamingResponseBuilder {
    #[allow(dead_code)]
    pub fn new(model: String) -> Self {
        Self {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            model,
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    #[allow(dead_code)]
    pub fn build_chunk(
        &self,
        content: String,
        finish_reason: Option<String>,
    ) -> ChatCompletionChunk {
        ChatCompletionChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk".to_string(),
            created: self.created,
            model: self.model.clone(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: Delta {
                    role: Some("assistant".to_string()),
                    content: Some(content),
                    tool_calls: None,
                },
                finish_reason,
            }],
        }
    }
}

// ============================================================================
// Responses API Streaming Events (Codex CLI Protocol)
// ============================================================================

/// Streaming events for Responses API
/// Based on Codex CLI protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseEvent {
    /// Response created event
    #[serde(rename = "response.created")]
    Created {
        /// The lifecycle response snapshot
        response: ResponseSnapshot,
    },

    /// Response in-progress event
    #[serde(rename = "response.in_progress")]
    InProgress {
        /// The lifecycle response snapshot
        response: ResponseSnapshot,
    },

    /// Output item added event
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded {
        /// Index of the output item
        output_index: u32,
        /// The output item
        item: OutputItem,
    },

    /// Output item done event
    #[serde(rename = "response.output_item.done")]
    OutputItemDone {
        /// Index of the output item
        output_index: u32,
        /// The completed output item
        item: OutputItem,
    },

    /// Content part added event
    #[serde(rename = "response.content_part.added")]
    ContentPartAdded {
        /// Index of the output item
        output_index: u32,
        /// Index of the content part
        content_index: u32,
        /// The content part type
        part: ContentPartType,
    },

    /// Output text delta event
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta {
        /// Index of the output item
        output_index: u32,
        /// Index of the content part
        content_index: Option<u32>,
        /// The text delta
        delta: String,
    },

    /// Output text done event
    #[serde(rename = "response.output_text.done")]
    OutputTextDone {
        /// Index of the output item
        output_index: u32,
        /// The complete text
        text: String,
    },

    /// Reasoning summary part added event
    #[serde(rename = "response.reasoning_summary_part.added")]
    ReasoningSummaryPartAdded {
        /// Index of the reasoning item
        output_index: u32,
        /// Index of the summary part
        summary_index: u32,
    },

    /// Reasoning summary text delta event
    #[serde(rename = "response.reasoning_summary_text.delta")]
    ReasoningSummaryTextDelta {
        /// Index of the reasoning item
        output_index: u32,
        /// Index of the summary part
        summary_index: u32,
        /// The text delta
        delta: String,
    },

    /// Reasoning summary text done event
    #[serde(rename = "response.reasoning_summary_text.done")]
    ReasoningSummaryTextDone {
        /// Index of the reasoning item
        output_index: u32,
        /// Index of the summary part
        summary_index: u32,
        /// The complete text
        text: String,
    },

    /// Function call arguments delta event
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta {
        /// Index of the output item
        output_index: u32,
        /// Call ID
        call_id: String,
        /// The arguments delta
        delta: String,
    },

    /// Function call arguments done event
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone {
        /// Index of the output item
        output_index: u32,
        /// Call ID
        call_id: String,
        /// The complete arguments
        arguments: String,
    },

    /// Response completed event
    #[serde(rename = "response.completed")]
    Completed {
        /// The lifecycle response snapshot
        response: ResponseSnapshot,
    },

    /// Response failed event
    #[serde(rename = "response.failed")]
    Failed {
        /// The lifecycle response snapshot
        response: ResponseSnapshot,
    },

    /// Response incomplete event
    #[serde(rename = "response.incomplete")]
    Incomplete {
        /// The lifecycle response snapshot
        response: ResponseSnapshot,
    },

    /// Rate limits updated event
    #[serde(rename = "response.rate_limits")]
    RateLimits {
        /// Rate limit snapshot
        rate_limits: RateLimitSnapshot,
    },

    /// Server model event
    #[serde(rename = "response.server_model")]
    ServerModel {
        /// Model name
        model: String,
    },

    /// Server reasoning included event
    #[serde(rename = "response.server_reasoning_included")]
    ServerReasoningIncluded {
        /// Whether reasoning is included
        included: bool,
    },

    /// Models etag event
    #[serde(rename = "response.models_etag")]
    ModelsEtag {
        /// Etag value
        etag: String,
    },
}

/// Content part type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentPartType {
    /// Output text
    OutputText,
    /// Input image
    InputImage,
    /// Refusal
    Refusal,
}

/// Response error information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
}

/// Lifecycle snapshot for Responses protocol events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSnapshot {
    /// Unique identifier for the response
    pub id: String,
    /// Object type
    pub object: String,
    /// Unix timestamp when the response was created
    pub created_at: u64,
    /// Response status
    pub status: String,
    /// Model used for the response
    pub model: String,
    /// Output items produced so far
    #[serde(default)]
    pub output: Vec<OutputItem>,
    /// Usage statistics if available
    #[serde(default)]
    pub usage: Option<DetailedUsage>,
    /// Error snapshot for failed responses
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
    /// Provider-specific incomplete details
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incomplete_details: Option<serde_json::Value>,
    /// Additional lifecycle fields preserved for compatibility
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Rate limit snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitSnapshot {
    /// Number of requests remaining
    pub requests_remaining: u64,
    /// Number of tokens remaining
    pub tokens_remaining: u64,
    /// Total request limit
    pub requests_limit: u64,
    /// Total token limit
    pub tokens_limit: u64,
    /// Time until reset (seconds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_after_seconds: Option<u64>,
}

/// Detailed usage statistics (alias for response module type)
pub type StreamUsage = DetailedUsage;

// ============================================================================
// SSE Event Parsing
// ============================================================================

/// Parse a Responses API SSE event from raw data
#[allow(dead_code)]
pub fn parse_responses_sse_event(data: &str) -> Result<Option<ResponseEvent>, serde_json::Error> {
    // Skip empty data or [DONE] marker
    if data.is_empty() || data == "[DONE]" {
        return Ok(None);
    }

    serde_json::from_str(data).map(Some)
}

/// Build an SSE event string from a ResponseEvent
#[allow(dead_code)]
pub fn build_sse_event(event: &ResponseEvent) -> String {
    format!(
        "data: {}\n\n",
        serde_json::to_string(event).unwrap_or_default()
    )
}

/// Build a [DONE] SSE event
#[allow(dead_code)]
pub fn build_done_event() -> String {
    "data: [DONE]\n\n".to_string()
}
