//! Chat Completions API models (OpenAI compatible)
//!
//! These models represent the standard OpenAI Chat Completions API request/response format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Chat Completions Request
/// POST /v1/chat/completions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    /// ID of the model to use
    pub model: String,

    /// A list of messages comprising the conversation
    pub messages: Vec<Message>,

    /// Controls which sampling temperature to use, between 0 and 2
    #[serde(default)]
    pub temperature: Option<f32>,

    /// An alternative to sampling with temperature, called nucleus sampling
    #[serde(default)]
    pub top_p: Option<f32>,

    /// The maximum number of tokens to generate in the chat completion
    #[serde(default)]
    pub max_tokens: Option<u32>,

    /// Whether to stream back partial progress
    #[serde(default)]
    pub stream: Option<bool>,

    /// Up to 4 sequences where the API will stop generating further tokens
    #[serde(default)]
    pub stop: Option<Vec<String>>,

    /// Number of chat completion choices to generate
    #[serde(default = "default_n")]
    pub n: u32,

    /// If set, partial message deltas will be sent
    #[serde(default)]
    pub stream_options: Option<StreamOptions>,

    /// Include usage information in the response
    #[serde(default = "default_include_usage")]
    pub include_usage: Option<bool>,

    /// Specify the format that the model must output
    #[serde(default)]
    pub response_format: Option<ResponseFormat>,

    /// The seed for deterministic sampling
    #[serde(default)]
    pub seed: Option<i64>,

    /// A unique identifier for your organization
    #[serde(default)]
    pub organization: Option<String>,

    /// What sampling temperature to use, between 0 and 2.0
    #[serde(default)]
    pub presence_penalty: Option<f32>,

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency
    #[serde(default)]
    pub frequency_penalty: Option<f32>,

    /// Modify the likelihood of specified tokens appearing in the completion
    #[serde(default)]
    pub logit_bias: Option<HashMap<String, f32>>,

    /// A unique identifier for the end-user
    #[serde(default)]
    pub user: Option<String>,

    /// A list of tools the model may call
    #[serde(default)]
    pub tools: Option<Vec<Tool>>,

    /// Controls which (if any) tool is called by the model
    #[serde(default)]
    pub tool_choice: Option<ToolChoice>,

    /// Whether to enable parallel function calling
    #[serde(default = "default_parallel_tool_calls")]
    pub parallel_tool_calls: bool,
}

fn default_n() -> u32 {
    1
}

fn default_include_usage() -> Option<bool> {
    Some(true)
}

fn default_parallel_tool_calls() -> bool {
    true
}

/// Message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The role of the message author
    pub role: String,

    /// The contents of the message
    pub content: Option<String>,

    /// The name of the author of this message
    #[serde(default)]
    pub name: Option<String>,

    /// Tool calls that the model wants to make
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Tool call ID that this message is responding to
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

/// Tool that can be called by the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The type of the tool. Values: "function", "web_search", "file_search", "computer_use", "mcp"
    #[serde(rename = "type")]
    pub tool_type: String,

    /// The function definition (only for type "function")
    #[serde(default)]
    pub function: Option<FunctionDefinition>,
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// The name of the function
    pub name: String,

    /// A description of what the function does
    #[serde(default)]
    pub description: Option<String>,

    /// The parameters the function accepts
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
}

/// Tool call made by the model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    /// The ID of the tool call
    pub id: String,

    /// The type of the tool. Currently, only "function" is supported
    #[serde(rename = "type")]
    pub call_type: String,

    /// The function that the model called
    pub function: FunctionCall,
}

/// Function call made by the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function to call
    pub name: String,

    /// The arguments to call the function with, as a JSON string
    pub arguments: String,
}

/// Controls which tool is called by the model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Specific tool to call
    Tool(ToolChoiceAuto),
    /// Let the model decide
    String(String),
}

/// Tool choice automatic
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolChoiceAuto {
    /// The type of tool choice. Currently, only "function" is supported
    #[serde(rename = "type")]
    pub choice_type: String,

    /// The function to call
    #[serde(default)]
    pub function: Option<ToolChoiceFunction>,
}

/// Function to call for tool choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    /// The name of the function
    pub name: String,
}

/// Response format specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseFormat {
    /// Plain text
    Text,
    /// JSON object
    JsonObject,
    /// JSON schema
    JsonSchema,
}

/// Stream options for partial progress
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamOptions {
    /// Whether to include usage information in the response
    #[serde(default)]
    pub include_usage: Option<bool>,
}

/// Chat Completions Response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    /// A unique identifier for the chat completion
    pub id: String,

    /// The object type
    pub object: String,

    /// The Unix timestamp (in seconds) when the chat completion was created
    pub created: u64,

    /// The model used for the chat completion
    pub model: String,

    /// The choices of the chat completion
    pub choices: Vec<Choice>,

    /// Usage statistics for the request
    #[serde(default)]
    pub usage: Option<Usage>,

    /// This fingerprint represents the backend configuration that the model runs with
    #[serde(default)]
    pub service_tier: Option<String>,

    /// The reason the model stopped generating tokens
    #[serde(default)]
    pub finish_reason: Option<String>,

    /// A list of additional properties
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Choice in the chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    /// The index of the choice
    pub index: u32,

    /// The message generated by the model
    pub message: Message,

    /// The reason the model stopped generating tokens
    #[serde(default)]
    pub finish_reason: Option<String>,

    /// Log probability information for the choice
    #[serde(default)]
    pub logprobs: Option<LogProbs>,
}

/// Log probability information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogProbs {
    /// The log probability of each token
    pub content: Option<Vec<LogProb>>,

    /// The log probability of each token in the prompt
    #[serde(default)]
    pub refusal: Option<Vec<LogProb>>,
}

/// Log probability for a single token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogProb {
    /// The token
    pub token: String,

    /// The log probability of the token
    pub logprob: f32,

    /// Bytes representing the token
    #[serde(default)]
    pub bytes: Option<Vec<u8>>,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt
    #[serde(alias = "promptTokens")]
    pub prompt_tokens: u32,

    /// Number of tokens in the completion
    #[serde(alias = "completionTokens")]
    pub completion_tokens: u32,

    /// Total number of tokens
    #[serde(alias = "totalTokens")]
    pub total_tokens: u32,
}

/// Streaming chunk for SSE responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatCompletionChunk {
    /// A unique identifier for the chat completion chunk
    pub id: String,

    /// The object type
    pub object: String,

    /// The Unix timestamp (in seconds) when the chat completion chunk was created
    pub created: u64,

    /// The model used for the chat completion chunk
    pub model: String,

    /// The choices of the chat completion chunk
    pub choices: Vec<StreamingChoice>,

    /// Usage statistics for the request
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// Streaming choice
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingChoice {
    /// The index of the choice
    pub index: u32,

    /// The delta content
    #[serde(default)]
    pub delta: Option<Delta>,

    /// The reason the model stopped generating tokens
    #[serde(default)]
    pub finish_reason: Option<String>,

    /// Log probability information for the choice
    #[serde(default)]
    pub logprobs: Option<LogProbs>,
}

/// Delta content in streaming response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Delta {
    /// The role of the message author
    #[serde(default)]
    pub role: Option<String>,

    /// The contents of the message
    #[serde(default)]
    pub content: Option<String>,

    /// Tool calls that the model wants to make
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}
