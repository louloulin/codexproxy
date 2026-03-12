//! Responses API models
//!
//! These models represent the new OpenAI Responses API format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Responses API Request
/// POST /v1/responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsesRequest {
    /// ID of the model to use
    pub model: String,

    /// Input items for the model to process
    #[serde(default)]
    pub input: Vec<Item>,

    /// Optional instructions for the model
    #[serde(default)]
    pub instructions: Option<String>,

    /// List of available tools for the model to call
    #[serde(default)]
    pub tools: Vec<Tool>,

    /// Temperature value for sampling
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Nucleus sampling parameter
    #[serde(default)]
    pub top_p: Option<f32>,

    /// Maximum number of tokens to generate
    #[serde(default)]
    pub max_tokens: Option<u32>,

    /// Whether to stream the response
    #[serde(default)]
    pub stream: Option<bool>,

    /// Format specification for text output
    #[serde(default)]
    pub text: Option<TextFormat>,

    /// Enable structured outputs
    #[serde(default)]
    pub structured_output: Option<StructuredOutput>,

    /// Store the response for future retrieval
    #[serde(default)]
    pub store: Option<bool>,

    /// Metadata about the request
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// Model settings
    #[serde(default)]
    pub model_settings: Option<ModelSettings>,

    /// Settings for reasoning effort
    #[serde(default)]
    pub reasoning: Option<ReasoningSettings>,

    /// Stop sequences
    #[serde(default)]
    pub stop: Option<Vec<String>>,

    /// Seed for deterministic sampling
    #[serde(default)]
    pub seed: Option<i64>,

    /// User identifier
    #[serde(default)]
    pub user: Option<String>,
}

/// Input item in Responses API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    /// A message input item
    Message(MessageItem),
    /// A reasoning input item
    Reasoning(ReasoningItem),
    /// A function call input item
    FunctionCall(FunctionCallItem),
    /// A function call output item
    FunctionCallOutput(FunctionCallOutputItem),
}

/// Message input item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageItem {
    /// The role of the message
    pub role: String,

    /// The content of the message
    pub content: Vec<ContentBlock>,
}

/// Reasoning input item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningItem {
    /// The reasoning/thinking content
    pub reasoning: String,

    /// The type of reasoning (e.g., "summary")
    #[serde(default)]
    pub summary: Option<String>,
}

/// Function call input item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallItem {
    /// The ID of the function call
    pub call_id: String,

    /// The name of the function being called
    pub name: String,

    /// The arguments for the function
    pub arguments: String,
}

/// Function call output item (for tool results)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallOutputItem {
    /// The ID of the function call
    pub call_id: String,

    /// The output of the function call
    pub output: String,
}

/// Content block in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Input text content
    InputText(InputText),
    /// Image content
    Image(ImageContent),
    /// Output text content
    OutputText(OutputText),
}

/// Input text content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputText {
    /// The text content
    pub text: String,
}

/// Image content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// The image data (URL or base64)
    pub source: ImageSource,
}

/// Image source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Image from URL
    Url(UrlImageSource),
    /// Image from base64 encoded data
    Base64(Base64ImageSource),
}

/// URL image source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlImageSource {
    /// The URL of the image
    pub url: String,
    /// MIME type of the image
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// Base64 encoded image source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Base64ImageSource {
    /// The base64 encoded image data
    pub data: String,
    /// MIME type of the image
    pub mime_type: String,
}

/// Output text content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputText {
    /// The text content
    pub text: String,
    /// Whether this is the final text
    #[serde(default)]
    pub annotations: Option<Vec<Annotation>>,
}

/// Annotation on the text
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Annotation {
    /// File citation
    FileCitation(FileCitation),
    /// URL citation
    UrlCitation(UrlCitation),
}

/// File citation annotation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCitation {
    /// The ID of the cited file
    #[serde(rename = "file_id")]
    pub file_id: String,

    /// The index of the citation
    pub index: Option<u32>,
}

/// URL citation annotation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlCitation {
    /// The cited URL
    pub url: String,

    /// The title of the cited content
    #[serde(default)]
    pub title: Option<String>,
}

/// Tool definition for the model to call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The type of tool. Supported values: "function", "computer", "web_search", "file_search", "mcp"
    #[serde(rename = "type")]
    pub tool_type: String,

    /// The function definition (only for type "function")
    /// Uses flatten to support both internally-tagged and externally-tagged formats
    #[serde(flatten, default)]
    pub function: Option<FunctionDefinition>,

    // File search fields
    /// Vector store IDs for file_search tool
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_store_ids: Option<Vec<String>>,

    // Computer use fields
    /// Display width for computer_use tool
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_width: Option<u32>,

    /// Display height for computer_use tool
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_height: Option<u32>,

    /// Environment for computer_use tool (mac, windows, linux, ubuntu)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,

    // MCP fields
    /// Server label for MCP tool
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_label: Option<String>,

    /// Server description for MCP tool
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_description: Option<String>,

    /// Server URL for MCP tool
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,

    /// Require approval setting for MCP tool (never, always)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_approval: Option<String>,
}

/// Function definition (used with #[serde(flatten)] in Tool)
/// When flattened, these fields appear at the top level of the tool JSON
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FunctionDefinition {
    /// The name of the function
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// A description of what the function does
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The parameters the function accepts (JSON Schema)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,

    /// Strict mode for parameter matching
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Text format specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFormat {
    /// The type of format
    #[serde(default)]
    pub format: Option<TextFormatType>,
}

/// Text format type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextFormatType {
    /// Plain text
    Text,
    /// JSON object
    JsonObject,
    /// JSON schema
    JsonSchema,
}

/// Structured output specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredOutput {
    /// The JSON schema for structured output
    pub schema: serde_json::Value,

    /// Whether to use strict mode
    #[serde(default)]
    pub strict: Option<bool>,
}

/// Model settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettings {
    /// Temperature for sampling
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Nucleus sampling parameter
    #[serde(default)]
    pub top_p: Option<f32>,

    /// Maximum tokens to generate
    #[serde(default)]
    pub max_tokens: Option<u32>,

    /// Presence penalty
    #[serde(default)]
    pub presence_penalty: Option<f32>,

    /// Frequency penalty
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
}

/// Reasoning effort settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    /// Low reasoning effort
    Low,
    /// Medium reasoning effort
    Medium,
    /// High reasoning effort
    High,
}

/// Reasoning settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningSettings {
    /// The effort level for reasoning
    pub effort: ReasoningEffort,

    /// Whether to include the reasoning in the response
    #[serde(default)]
    pub include: Option<bool>,
}

/// Responses API Response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsesResponse {
    /// Unique identifier for the response
    pub id: String,

    /// The object type
    pub object: String,

    /// Unix timestamp when the response was created
    pub created: u64,

    /// Model used for the response
    pub model: String,

    /// Output items from the model
    pub output: Vec<OutputItem>,

    /// Usage statistics
    #[serde(default)]
    pub usage: Option<Usage>,

    /// The reason the model stopped generating
    #[serde(default)]
    pub finish_reason: Option<String>,

    /// Additional properties
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Output item from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputItem {
    /// Message output
    Message(MessageOutput),
    /// Reasoning output
    Reasoning(ReasoningOutput),
    /// Function call output
    FunctionCall(FunctionCallOutput),
}

/// Message output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageOutput {
    /// Index of the output item
    pub index: u32,

    /// The role of the message
    pub role: String,

    /// Content of the message
    pub content: Vec<ContentBlock>,

    /// Status of the message
    #[serde(default)]
    pub status: Option<String>,

    /// Tool calls made in this message
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCallOutput>>,
}

/// Tool call output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallOutput {
    /// ID of the tool call
    pub id: String,

    /// The type of tool call
    #[serde(rename = "type")]
    pub call_type: String,

    /// The function that was called
    pub function: FunctionCallOutputFunction,
}

/// Function call output function
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallOutputFunction {
    /// The name of the function
    pub name: String,

    /// The arguments for the function
    pub arguments: String,
}

/// Reasoning output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningOutput {
    /// Index of the output item
    pub index: u32,

    /// The reasoning content
    pub reasoning: String,

    /// Summary of the reasoning
    #[serde(default)]
    pub summary: Option<Vec<ReasoningSummary>>,
}

/// Reasoning summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningSummary {
    /// The summary text
    pub summary: String,
}

/// Function call output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallOutput {
    /// Index of the output item
    pub index: u32,

    /// ID of the function call
    pub call_id: String,

    /// The name of the function
    pub name: String,

    /// The arguments for the function
    pub arguments: String,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    /// Number of tokens in the input
    pub input_tokens: u32,

    /// Number of tokens in the output
    pub output_tokens: u32,

    /// Number of tokens in the reasoning
    #[serde(default)]
    pub tokens: Option<u32>,

    /// Total number of tokens
    pub total_tokens: u32,
}

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsesStreamChunk {
    /// Unique identifier
    pub id: String,

    /// Object type
    pub object: String,

    /// Unix timestamp
    pub created: u64,

    /// Model used
    pub model: String,

    /// Output items in the chunk
    #[serde(default)]
    pub output: Vec<StreamOutputItem>,

    /// Usage in the chunk
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// Stream output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamOutputItem {
    /// Message delta
    Message(StreamMessageDelta),
    /// Reasoning delta
    Reasoning(StreamReasoningDelta),
    /// Function call delta
    FunctionCall(StreamFunctionCallDelta),
}

/// Stream message delta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamMessageDelta {
    /// Index
    pub index: u32,

    /// Delta content
    #[serde(default)]
    pub delta: Option<MessageDelta>,

    /// Status
    #[serde(default)]
    pub status: Option<String>,
}

/// Message delta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDelta {
    /// Role
    #[serde(default)]
    pub role: Option<String>,

    /// Content
    #[serde(default)]
    pub content: Option<String>,

    /// Tool calls
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCallOutput>>,
}

/// Stream reasoning delta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamReasoningDelta {
    /// Index
    pub index: u32,

    /// Delta reasoning
    #[serde(default)]
    pub delta: Option<String>,
}

/// Stream function call delta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamFunctionCallDelta {
    /// Index
    pub index: u32,

    /// Delta arguments
    #[serde(default)]
    pub delta: Option<String>,
}
