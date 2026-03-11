//! Streaming response models
//!
//! This module contains models for handling streaming (SSE) responses.

use serde::{Deserialize, Serialize};

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
