//! Responses API models
//!
//! These models represent the new OpenAI Responses API format.

use serde::{Deserialize, Deserializer, Serialize};
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

/// Message phase for distinguishing intermediate vs final content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessagePhase {
    /// Intermediate commentary or thinking
    Commentary,
    /// Final answer content
    FinalAnswer,
}

/// Input item in Responses API
#[derive(Debug, Clone, Serialize)]
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
    /// Local shell call (computer_use tool)
    LocalShellCall(LocalShellCallItem),
    /// Tool search call (file_search tool)
    ToolSearchCall(ToolSearchCallItem),
    /// Custom tool call
    CustomToolCall(CustomToolCallItem),
    /// Custom tool call output
    CustomToolCallOutput(CustomToolCallOutputItem),
    /// Tool search output
    ToolSearchOutput(ToolSearchOutputItem),
    /// Web search call
    WebSearchCall(WebSearchCallItem),
    /// Image generation call
    ImageGenerationCall(ImageGenerationCallItem),
    /// MCP tool call output
    McpToolCallOutput(McpToolCallOutputItem),
}

/// Message input item
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageItem {
    /// Optional ID of the message
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The role of the message
    pub role: String,

    /// The content of the message
    pub content: Vec<ContentBlock>,

    /// Whether this is the end of the turn
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_turn: Option<bool>,

    /// The phase of the message (commentary or final answer)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<MessagePhase>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TaggedItem {
    Message(MessageItem),
    Reasoning(ReasoningItem),
    FunctionCall(FunctionCallItem),
    FunctionCallOutput(FunctionCallOutputItem),
    LocalShellCall(LocalShellCallItem),
    ToolSearchCall(ToolSearchCallItem),
    CustomToolCall(CustomToolCallItem),
    CustomToolCallOutput(CustomToolCallOutputItem),
    ToolSearchOutput(ToolSearchOutputItem),
    WebSearchCall(WebSearchCallItem),
    ImageGenerationCall(ImageGenerationCallItem),
    McpToolCallOutput(McpToolCallOutputItem),
}

impl From<TaggedItem> for Item {
    fn from(value: TaggedItem) -> Self {
        match value {
            TaggedItem::Message(item) => Self::Message(item),
            TaggedItem::Reasoning(item) => Self::Reasoning(item),
            TaggedItem::FunctionCall(item) => Self::FunctionCall(item),
            TaggedItem::FunctionCallOutput(item) => Self::FunctionCallOutput(item),
            TaggedItem::LocalShellCall(item) => Self::LocalShellCall(item),
            TaggedItem::ToolSearchCall(item) => Self::ToolSearchCall(item),
            TaggedItem::CustomToolCall(item) => Self::CustomToolCall(item),
            TaggedItem::CustomToolCallOutput(item) => Self::CustomToolCallOutput(item),
            TaggedItem::ToolSearchOutput(item) => Self::ToolSearchOutput(item),
            TaggedItem::WebSearchCall(item) => Self::WebSearchCall(item),
            TaggedItem::ImageGenerationCall(item) => Self::ImageGenerationCall(item),
            TaggedItem::McpToolCallOutput(item) => Self::McpToolCallOutput(item),
        }
    }
}

impl<'de> Deserialize<'de> for Item {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        if value.get("type").is_some() {
            return TaggedItem::deserialize(value)
                .map(Into::into)
                .map_err(serde::de::Error::custom);
        }

        MessageItem::deserialize(value)
            .map(Self::Message)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageItemCompat {
    #[serde(default)]
    id: Option<String>,
    role: String,
    content: MessageContentCompat,
    #[serde(default)]
    end_turn: Option<bool>,
    #[serde(default)]
    phase: Option<MessagePhase>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MessageContentCompat {
    Blocks(Vec<ContentBlock>),
    Text(String),
}

impl From<MessageItemCompat> for MessageItem {
    fn from(value: MessageItemCompat) -> Self {
        let content = match value.content {
            MessageContentCompat::Blocks(blocks) => blocks,
            MessageContentCompat::Text(text) => vec![ContentBlock::InputText(InputText { text })],
        };

        Self {
            id: value.id,
            role: value.role,
            content,
            end_turn: value.end_turn,
            phase: value.phase,
        }
    }
}

impl<'de> Deserialize<'de> for MessageItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        MessageItemCompat::deserialize(deserializer)
            .map(Into::into)
            .map_err(serde::de::Error::custom)
    }
}

/// Reasoning input item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningItem {
    /// Optional ID of the reasoning
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The reasoning/thinking content (encrypted in some cases)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,

    /// Summary of reasoning steps
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub summary: Vec<ReasoningSummaryPart>,

    /// Plain text reasoning content (for compatibility)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

/// A reasoning summary part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningSummaryPart {
    /// The summary text
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// The type of summary part
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_type: Option<String>,
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

// ============================================================================
// Non-Function Tool Types (Codex CLI Protocol)
// ============================================================================

/// Local shell call item (computer_use tool)
/// Emitted when the model wants to execute a shell command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalShellCallItem {
    /// Optional ID (legacy field)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Call ID for the shell call
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,

    /// Status of the call (in_progress, completed, failed)
    pub status: String,

    /// The action to perform
    pub action: LocalShellAction,
}

/// Local shell action definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalShellAction {
    /// Type of action (exec, read, write, etc.)
    #[serde(rename = "type")]
    pub action_type: String,

    /// Command to execute (for exec type)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Arguments for the command
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<String>>,

    /// Working directory
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    /// Timeout in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,

    /// File path (for read/write types)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Content to write (for write type)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Tool search call item (file_search tool)
/// Emitted when the model searches for available tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSearchCallItem {
    /// Optional ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Call ID for the search
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,

    /// Status of the search
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Execution mode: "client" or "remote"
    pub execution: String,

    /// Search arguments
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// Custom tool call item
/// Emitted when the model calls a custom (non-function) tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomToolCallItem {
    /// Optional ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Call ID for the custom tool call
    pub call_id: String,

    /// Status of the call
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Name of the custom tool
    pub name: String,

    /// Input to the tool (JSON string)
    pub input: String,
}

/// Custom tool call output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomToolCallOutputItem {
    /// Call ID that this output corresponds to
    pub call_id: String,

    /// Name of the tool (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Output from the tool
    pub output: FunctionCallOutputPayload,
}

/// Tool search output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSearchOutputItem {
    /// Call ID for the search
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,

    /// Status of the search
    pub status: String,

    /// Execution mode
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<String>,

    /// Tools found in the search
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
}

/// Tool definition returned by tool search
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    /// ID of the tool
    pub id: String,

    /// Name of the tool
    pub name: String,

    /// Description of the tool
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Parameters schema
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// Web search call item
/// Emitted when the model triggers a web search
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchCallItem {
    /// Optional ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Status of the search
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// The search action
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<WebSearchAction>,
}

/// Web search action definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchAction {
    /// Type of action (search)
    #[serde(rename = "type")]
    pub action_type: String,

    /// Search query
    pub query: String,
}

/// Image generation call item
/// Emitted when the model triggers image generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGenerationCallItem {
    /// ID of the image generation call
    pub id: String,

    /// Status of the generation
    pub status: String,

    /// Revised prompt used for generation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,

    /// The generated image result
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// MCP tool call output item
/// Emitted as the result of an MCP tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallOutputItem {
    /// Call ID that this output corresponds to
    pub call_id: String,

    /// The result from the MCP tool
    pub output: McpToolResult,
}

/// MCP tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResult {
    /// Content items from the tool
    pub content: Vec<McpContent>,

    /// Whether this is an error
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// MCP content item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpContent {
    /// Text content
    Text { text: String },
    /// Image content
    Image { data: String, mime_type: String },
    /// Resource content
    Resource { resource: McpResource },
}

/// MCP resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResource {
    /// URI of the resource
    pub uri: String,

    /// Name of the resource
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// MIME type of the resource
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Text content
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Function call output payload
/// Supports both text and structured content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionCallOutputPayload {
    /// Plain text output
    Text(String),
    /// Structured content items
    ContentItems(Vec<FunctionCallOutputContentItem>),
}

/// Function call output content item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FunctionCallOutputContentItem {
    /// Text content
    Text { text: String },
    /// Image content
    Image { image_url: String },
}

/// Content block in a message
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Input text content
    InputText(InputText),
    /// Input image content (Codex CLI compatible)
    InputImage(InputImage),
    /// Image content (legacy alias for InputImage)
    Image(ImageContent),
    /// Output text content
    OutputText(OutputText),
    /// Refusal content
    Refusal(RefusalContent),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TaggedContentBlock {
    InputText(InputText),
    InputImage(InputImage),
    Image(ImageContent),
    OutputText(OutputText),
    Refusal(RefusalContent),
}

impl From<TaggedContentBlock> for ContentBlock {
    fn from(value: TaggedContentBlock) -> Self {
        match value {
            TaggedContentBlock::InputText(block) => Self::InputText(block),
            TaggedContentBlock::InputImage(block) => Self::InputImage(block),
            TaggedContentBlock::Image(block) => Self::Image(block),
            TaggedContentBlock::OutputText(block) => Self::OutputText(block),
            TaggedContentBlock::Refusal(block) => Self::Refusal(block),
        }
    }
}

#[derive(Debug, Deserialize)]
struct BareTextContentBlock {
    text: String,
}

impl<'de> Deserialize<'de> for ContentBlock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        if value.get("type").is_some() {
            return TaggedContentBlock::deserialize(value)
                .map(Into::into)
                .map_err(serde::de::Error::custom);
        }

        if let Some(text) = value.as_str() {
            return Ok(Self::InputText(InputText {
                text: text.to_string(),
            }));
        }

        BareTextContentBlock::deserialize(value)
            .map(|block| Self::InputText(InputText { text: block.text }))
            .map_err(serde::de::Error::custom)
    }
}

/// Input text content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputText {
    /// The text content
    pub text: String,
}

/// Input image content (Codex CLI style)
/// Simplified structure matching the Codex CLI protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputImage {
    /// The image URL (can be a data: URL or https: URL)
    pub image_url: String,

    /// Detail level for the image
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetail>,
}

/// Image detail level
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    /// Auto (default)
    #[default]
    Auto,
    /// Low detail
    Low,
    /// High detail
    High,
}

/// Refusal content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusalContent {
    /// The refusal message
    pub refusal: String,
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

/// Responses API streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsesStreamChunk {
    /// Unique identifier for the response stream
    pub id: String,

    /// The object type
    pub object: String,

    /// Unix timestamp when the chunk was created
    pub created: u64,

    /// Model used for the response
    pub model: String,

    /// Output items included in this chunk
    pub output: Vec<OutputItem>,

    /// Usage statistics, usually only present on the terminal chunk
    #[serde(default)]
    pub usage: Option<Usage>,
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
    /// Local shell call output (computer_use)
    LocalShellCall(LocalShellCallOutput),
    /// Tool search call output (file_search)
    ToolSearchCall(ToolSearchCallOutput),
    /// Custom tool call output
    CustomToolCall(CustomToolCallOutput),
    /// Custom tool call output result
    CustomToolCallOutput(CustomToolCallOutputResult),
    /// Tool search output result
    ToolSearchOutput(ToolSearchOutputResult),
    /// Web search call output
    WebSearchCall(WebSearchCallOutput),
    /// Image generation call output
    ImageGenerationCall(ImageGenerationCallOutput),
    /// MCP tool call output
    McpToolCallOutput(McpToolCallOutputResult),
}

/// Message output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageOutput {
    /// Index of the output item
    pub index: u32,

    /// Optional ID of the message
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The role of the message
    pub role: String,

    /// Content of the message
    pub content: Vec<ContentBlock>,

    /// Status of the message
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Whether this is the end of the turn
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_turn: Option<bool>,

    /// Phase of the message (commentary or final_answer)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<MessagePhase>,

    /// Tool calls made in this message
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

// ============================================================================
// Additional Output Types for Codex CLI Protocol
// ============================================================================

/// Reasoning output item (enhanced)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningOutput {
    /// Index of the output item
    pub index: u32,

    /// Optional ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Encrypted reasoning content (for some providers)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,

    /// Summary of reasoning steps
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub summary: Vec<ReasoningSummaryPart>,
}

/// Function call output item (enhanced)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallOutput {
    /// Index of the output item
    pub index: u32,

    /// Optional ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// ID of the function call
    pub call_id: String,

    /// The name of the function
    pub name: String,

    /// The arguments for the function (JSON string)
    pub arguments: String,

    /// Status of the call
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Local shell call output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalShellCallOutput {
    /// Index of the output item
    pub index: u32,

    /// Optional ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Call ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,

    /// Status of the call
    pub status: String,

    /// The action that was performed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<LocalShellAction>,
}

/// Tool search call output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSearchCallOutput {
    /// Index of the output item
    pub index: u32,

    /// Optional ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Call ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,

    /// Status of the search
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Execution mode
    pub execution: String,

    /// Search arguments
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// Custom tool call output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomToolCallOutput {
    /// Index of the output item
    pub index: u32,

    /// Optional ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Call ID
    pub call_id: String,

    /// Status of the call
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Name of the custom tool
    pub name: String,

    /// Input to the tool (JSON string)
    pub input: String,
}

/// Custom tool call output result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomToolCallOutputResult {
    /// Index of the output item
    pub index: u32,

    /// Call ID that this output corresponds to
    pub call_id: String,

    /// Name of the tool (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Output from the tool
    pub output: FunctionCallOutputPayload,
}

/// Tool search output result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSearchOutputResult {
    /// Index of the output item
    pub index: u32,

    /// Call ID for the search
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,

    /// Status of the search
    pub status: String,

    /// Execution mode
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<String>,

    /// Tools found in the search
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
}

/// Web search call output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchCallOutput {
    /// Index of the output item
    pub index: u32,

    /// Optional ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Status of the search
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// The search action
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<WebSearchAction>,
}

/// Image generation call output item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGenerationCallOutput {
    /// Index of the output item
    pub index: u32,

    /// ID of the image generation call
    pub id: String,

    /// Status of the generation
    pub status: String,

    /// Revised prompt used for generation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,

    /// The generated image result
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// MCP tool call output result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallOutputResult {
    /// Index of the output item
    pub index: u32,

    /// Call ID that this output corresponds to
    pub call_id: String,

    /// The result from the MCP tool
    pub output: McpToolResult,
}

// ============================================================================
// Enhanced Usage Statistics
// ============================================================================

/// Usage statistics (enhanced with detailed token info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the input
    #[serde(default)]
    pub input_tokens: u64,

    /// Detailed input token breakdown
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens_details: Option<InputTokensDetails>,

    /// Number of tokens in the output
    #[serde(default)]
    pub output_tokens: u64,

    /// Detailed output token breakdown
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<OutputTokensDetails>,

    /// Total number of tokens
    #[serde(default)]
    pub total_tokens: u64,
}

/// Detailed input token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputTokensDetails {
    /// Tokens served from cache
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u64>,
}

/// Detailed output token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputTokensDetails {
    /// Tokens used for reasoning
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
}

/// Detailed usage statistics (alias for Usage with detailed info)
pub type DetailedUsage = Usage;

#[cfg(test)]
mod tests {
    use super::ResponsesRequest;
    use serde_json::json;

    #[test]
    fn test_responses_request_accepts_message_input_without_explicit_type() {
        let value = json!({
            "model": "glm-5",
            "input": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        });

        let request = serde_json::from_value::<ResponsesRequest>(value);

        assert!(
            request.is_ok(),
            "expected request without item type to deserialize, got: {request:?}"
        );
    }

    #[test]
    fn test_responses_request_accepts_message_content_as_string() {
        let value = json!({
            "model": "glm-5",
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": "Hello"
                }
            ]
        });

        let request = serde_json::from_value::<ResponsesRequest>(value);

        assert!(
            request.is_ok(),
            "expected string content to deserialize, got: {request:?}"
        );
    }

    #[test]
    fn test_responses_request_accepts_message_content_blocks_without_explicit_type() {
        let value = json!({
            "model": "glm-5",
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [
                        {
                            "text": "Hello"
                        }
                    ]
                }
            ]
        });

        let request = serde_json::from_value::<ResponsesRequest>(value);

        assert!(
            request.is_ok(),
            "expected shorthand content block to deserialize, got: {request:?}"
        );
    }
}
