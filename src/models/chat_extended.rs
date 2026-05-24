//! Extended Chat API types for MiMo, DeepSeek, and other providers
//!
//! This module contains additional types that are not part of the standard
//! OpenAI Chat Completions API but are needed for provider-specific features:
//! - Thinking mode (reasoning_content)
//! - URL citations (annotations)
//! - Web search tools
//! - Image URL content parts

use serde::{Deserialize, Serialize};

// ============================================================================
// Content Parts - for messages with mixed content (text + images)
// ============================================================================

/// Content that can be either plain text or structured parts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentOrParts {
    /// Plain text content
    Text(String),
    /// Structured content parts (for multimodal messages)
    Parts(Vec<ChatContentPart>),
}

impl ContentOrParts {
    /// Get text content as string
    pub fn as_text(&self) -> String {
        match self {
            ContentOrParts::Text(s) => s.clone(),
            ContentOrParts::Parts(parts) => parts
                .iter()
                .filter_map(|p| p.as_text())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    /// Check if this contains images
    pub fn has_images(&self) -> bool {
        match self {
            ContentOrParts::Text(_) => false,
            ContentOrParts::Parts(parts) => parts.iter().any(|p| p.is_image()),
        }
    }
}

/// Content part types for multimodal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatContentPart {
    /// Text content part
    Text {
        /// The text content
        text: String,
    },
    /// Image URL content part
    ImageUrl {
        /// The image URL
        image_url: ImageUrl,
    },
}

/// Image URL with optional detail level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUrl {
    /// The URL of the image
    pub url: String,
    /// Detail level: "auto", "low", or "high"
    #[serde(default)]
    pub detail: Option<String>,
}

impl ChatContentPart {
    /// Get text content if this is a text part
    pub fn as_text(&self) -> Option<String> {
        match self {
            ChatContentPart::Text { text } => Some(text.clone()),
            _ => None,
        }
    }

    /// Check if this is an image
    pub fn is_image(&self) -> bool {
        matches!(self, ChatContentPart::ImageUrl { .. })
    }
}

// ============================================================================
// Thinking Mode Types
// ============================================================================

/// Thinking configuration for providers that support reasoning mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ThinkingConfig {
    /// Enable or disable thinking mode
    #[serde(rename = "type")]
    pub thinking_type: ThinkingType,
}

impl ThinkingConfig {
    /// Create an enabled thinking config
    pub fn enabled() -> Self {
        Self {
            thinking_type: ThinkingType::Enabled,
        }
    }

    /// Create a disabled thinking config
    pub fn disabled() -> Self {
        Self {
            thinking_type: ThinkingType::Disabled,
        }
    }
}

/// Thinking type options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingType {
    Enabled,
    Disabled,
    Auto,
}

impl Default for ThinkingType {
    fn default() -> Self {
        ThinkingType::Enabled
    }
}

/// Reasoning effort level (DeepSeek-style)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    None,
}

impl Default for ReasoningEffort {
    fn default() -> Self {
        ReasoningEffort::High
    }
}

// ============================================================================
// Annotation Types
// ============================================================================

/// URL citation annotation from web search results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UrlCitation {
    /// The URL of the citation
    pub url: String,
    /// The title of the cited page
    #[serde(default)]
    pub title: Option<String>,
    /// A snippet from the cited content
    #[serde(default)]
    pub snippet: Option<String>,
    /// Start index in the text
    #[serde(default, alias = "start_index")]
    pub start_index: Option<usize>,
    /// End index in the text
    #[serde(default, alias = "end_index")]
    pub end_index: Option<usize>,
}

/// Generic annotation type for extensibility
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Annotation {
    /// URL citation
    UrlCitation(UrlCitation),
    /// Other annotation types
    Other(serde_json::Value),
}

// ============================================================================
// Web Search Tool Types
// ============================================================================

/// Web search tool configuration (MiMo style)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchTool {
    /// Tool type identifier
    #[serde(rename = "type")]
    pub tool_type: String,

    /// User location for localized search results
    #[serde(default)]
    pub user_location: Option<UserLocation>,

    /// Maximum number of keywords to search for
    #[serde(default)]
    pub max_keyword: Option<u32>,

    /// Force search even if cached
    #[serde(default)]
    pub force_search: Option<bool>,

    /// Maximum number of results
    #[serde(default)]
    pub limit: Option<u32>,
}

impl WebSearchTool {
    /// Create a default web search tool
    pub fn default_config() -> Self {
        Self {
            tool_type: "web_search".to_string(),
            user_location: None,
            max_keyword: None,
            force_search: None,
            limit: None,
        }
    }
}

/// User location for web search
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLocation {
    /// Location type
    #[serde(rename = "type", default)]
    pub location_type: String,

    /// Country code
    #[serde(default)]
    pub country: Option<String>,

    /// Region/state
    #[serde(default)]
    pub region: Option<String>,

    /// City name
    #[serde(default)]
    pub city: Option<String>,

    /// District
    #[serde(default)]
    pub district: Option<String>,

    /// Longitude coordinate
    #[serde(default)]
    pub longitude: Option<f64>,

    /// Latitude coordinate
    #[serde(default)]
    pub latitude: Option<f64>,
}

// ============================================================================
// Extended Message Type
// ============================================================================

/// Extended message type with provider-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedMessage {
    /// The role of the message author
    pub role: String,

    /// The contents of the message (supports content parts)
    #[serde(default)]
    pub content: Option<ContentOrParts>,

    /// The name of the author of this message
    #[serde(default)]
    pub name: Option<String>,

    /// Tool calls that the model wants to make
    #[serde(default)]
    pub tool_calls: Option<Vec<ExtendedToolCall>>,

    /// Tool call ID that this message is responding to
    #[serde(default)]
    pub tool_call_id: Option<String>,

    /// Reasoning/thinking content (MiMo, DeepSeek style)
    #[serde(default, alias = "reasoning_content")]
    pub reasoning_content: Option<String>,

    /// Annotations like URL citations from web search
    #[serde(default)]
    pub annotations: Option<Vec<Annotation>>,
}

/// Extended tool call with optional annotations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedToolCall {
    /// The ID of the tool call
    pub id: String,

    /// The type of the tool
    #[serde(rename = "type", default)]
    pub call_type: String,

    /// The function that the model called
    pub function: ExtendedFunctionCall,

    /// Optional index for parallel tool calls
    #[serde(default)]
    pub index: Option<u32>,
}

/// Extended function call with full details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedFunctionCall {
    /// The name of the function to call
    pub name: String,

    /// The arguments to pass to the function
    pub arguments: String,
}

// ============================================================================
// Conversion Utilities
// ============================================================================

impl ExtendedMessage {
    /// Convert to standard message format
    pub fn to_standard_message(&self) -> crate::models::chat::Message {
        crate::models::chat::Message {
            role: self.role.clone(),
            content: Some(self.content_to_string()),
            name: self.name.clone(),
            tool_calls: self.tool_calls.as_ref().map(|calls| {
                calls
                    .iter()
                    .map(|call| crate::models::chat::ToolCall {
                        id: call.id.clone(),
                        call_type: call.call_type.clone(),
                        function: crate::models::chat::FunctionCall {
                            name: call.function.name.clone(),
                            arguments: call.function.arguments.clone(),
                        },
                    })
                    .collect()
            }),
            tool_call_id: self.tool_call_id.clone(),
        }
    }

    /// Convert content to string
    fn content_to_string(&self) -> String {
        match &self.content {
            Some(ContentOrParts::Text(s)) => s.clone(),
            Some(ContentOrParts::Parts(parts)) => parts
                .iter()
                .filter_map(|p| p.as_text())
                .collect::<Vec<_>>()
                .join("\n"),
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thinking_config_enabled() {
        let config = ThinkingConfig::enabled();
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, r#"{"type":"enabled"}"#);
    }

    #[test]
    fn test_thinking_config_disabled() {
        let config = ThinkingConfig::disabled();
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, r#"{"type":"disabled"}"#);
    }

    #[test]
    fn test_reasoning_effort_serialization() {
        let effort = ReasoningEffort::High;
        let json = serde_json::to_string(&effort).unwrap();
        assert_eq!(json, "\"high\"");
    }

    #[test]
    fn test_web_search_tool_default() {
        let tool = WebSearchTool::default_config();
        assert_eq!(tool.tool_type, "web_search");
    }

    #[test]
    fn test_content_parts_has_images() {
        let parts = vec![ChatContentPart::Text {
            text: "Hello".to_string(),
        }];
        let content = ContentOrParts::Parts(parts);
        assert!(!content.has_images());

        let parts_with_image = vec![
            ChatContentPart::Text {
                text: "Hello".to_string(),
            },
            ChatContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/image.png".to_string(),
                    detail: None,
                },
            },
        ];
        let content_with_image = ContentOrParts::Parts(parts_with_image);
        assert!(content_with_image.has_images());
    }

    #[test]
    fn test_url_citation_roundtrip() {
        let citation = UrlCitation {
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            snippet: Some("A test snippet".to_string()),
            start_index: Some(0),
            end_index: Some(10),
        };
        let json = serde_json::to_string(&citation).unwrap();
        let parsed: UrlCitation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, citation.url);
    }
}
