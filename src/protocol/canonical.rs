//! Internal canonical protocol model for the proxy.
//!
//! This module defines the internal representation that all providers
//! are adapted to and from. It is NOT the wire format - that is handled
//! by the provider adapters.
//!
//! The canonical model prioritizes:
//! - Responses-first semantics
//! - Complete tool definition preservation
//! - Structured reasoning and output support
//! - Multi-item input/output handling

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Canonical request that all handlers transform to internally.
///
/// This represents the superset of all capabilities we might need
/// to handle, regardless of which wire protocol the client uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalRequest {
    /// The model to use
    pub model: String,

    /// System-level instructions (equivalent to Responses API instructions)
    pub instructions: Option<String>,

    /// Input items for processing (using wire format internally)
    pub input: Vec<crate::models::response::Item>,

    /// Available tools (using wire format internally)
    pub tools: Vec<crate::models::response::Tool>,

    /// Tool choice configuration
    pub tool_choice: Option<serde_json::Value>,

    /// Whether to allow parallel tool calls
    pub parallel_tool_calls: bool,

    /// Sampling parameters
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub seed: Option<i64>,

    /// Reasoning configuration
    pub reasoning: Option<crate::models::response::ReasoningSettings>,

    /// Structured output configuration
    pub structured_output: Option<crate::models::response::StructuredOutput>,

    /// Text format specification
    pub text_format: Option<crate::models::response::TextFormat>,

    /// Response continuation
    pub previous_response_id: Option<String>,

    /// Storage settings
    pub store: Option<bool>,

    /// Prompt cache
    pub prompt_cache_key: Option<String>,

    /// Response inclusion options
    pub include: Vec<String>,

    /// Metadata for tracking
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// Model-specific settings
    pub model_settings: Option<crate::models::response::ModelSettings>,

    /// Whether to stream the response
    pub stream: bool,
}

/// Tool types that the canonical model recognizes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalToolType {
    Function,
    WebSearch,
    FileSearch,
    ComputerUse,
    Mcp,
}

impl CanonicalToolType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "function" => Some(Self::Function),
            "web_search" => Some(Self::WebSearch),
            "file_search" => Some(Self::FileSearch),
            "computer_use" | "computer" => Some(Self::ComputerUse),
            "mcp" => Some(Self::Mcp),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::WebSearch => "web_search",
            Self::FileSearch => "file_search",
            Self::ComputerUse => "computer_use",
            Self::Mcp => "mcp",
        }
    }
}

/// Tool preservation status after transformation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPreservationStatus {
    /// Tool is fully preserved
    Preserved,
    /// Tool is transformed to an equivalent representation
    Transformed(String),
    /// Tool is dropped (with reason)
    Dropped(String),
}

/// Canonical response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalResponse {
    /// Unique response identifier
    pub id: String,
    /// Object type
    pub object: String,
    /// Unix timestamp
    pub created_at: u64,
    /// Status
    pub status: String,
    /// Model used
    pub model: String,
    /// Output items (using wire format internally)
    pub output: Vec<crate::models::response::OutputItem>,
    /// Usage statistics
    pub usage: Option<crate::models::response::Usage>,
    /// Error if any
    pub error: Option<CanonicalError>,
    /// Incomplete details if truncated
    pub incomplete_details: Option<serde_json::Value>,
    /// Response headers/metadata
    pub metadata: HashMap<String, String>,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub input_tokens_details: Option<CanonicalInputTokensDetails>,
    pub output_tokens_details: Option<CanonicalOutputTokensDetails>,
}

/// Input token details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalInputTokensDetails {
    pub cached_tokens: Option<u32>,
    pub audio_tokens: Option<u32>,
}

/// Output token details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalOutputTokensDetails {
    pub reasoning_tokens: Option<u32>,
    pub audio_tokens: Option<u32>,
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalError {
    pub code: String,
    pub message: String,
    pub param: Option<String>,
    pub internal_trace_id: Option<String>,
}

/// Canonical streaming event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CanonicalStreamEvent {
    /// Response created
    Created {
        response_id: String,
        model: String,
    },
    /// Response in progress
    InProgress {
        response_id: String,
    },
    /// Output item added
    OutputItemAdded {
        output_index: u32,
        item: crate::models::response::OutputItem,
    },
    /// Content part added
    ContentPartAdded {
        output_index: u32,
        content_index: u32,
        part: crate::models::response::ContentBlock,
    },
    /// Text delta
    TextDelta {
        output_index: u32,
        content_index: Option<u32>,
        delta: String,
    },
    /// Text done
    TextDone {
        output_index: u32,
        content_index: Option<u32>,
        text: String,
    },
    /// Function call arguments delta
    FunctionCallArgumentsDelta {
        output_index: u32,
        call_id: String,
        delta: String,
    },
    /// Function call arguments done
    FunctionCallArgumentsDone {
        output_index: u32,
        call_id: String,
        arguments: String,
    },
    /// Output item done
    OutputItemDone {
        output_index: u32,
        item: crate::models::response::OutputItem,
    },
    /// Reasoning summary part added
    ReasoningSummaryPartAdded {
        output_index: u32,
        summary_index: u32,
    },
    /// Reasoning summary text delta
    ReasoningSummaryTextDelta {
        output_index: u32,
        summary_index: u32,
        delta: String,
    },
    /// Reasoning summary text done
    ReasoningSummaryTextDone {
        output_index: u32,
        summary_index: u32,
        text: String,
    },
    /// Response completed
    Completed {
        response_id: String,
    },
    /// Response failed
    Failed {
        response_id: String,
        error: CanonicalError,
    },
    /// Response incomplete
    Incomplete {
        response_id: String,
        reason: Option<String>,
    },
    /// Rate limits
    RateLimits {
        requests_remaining: u64,
        tokens_remaining: u64,
    },
    /// Server model info
    ServerModel {
        model: String,
    },
    /// Usage stats
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
    },
}

// ============================================================================
// Conversions to/from canonical model
// ============================================================================

impl CanonicalRequest {
    /// Convert from OpenAI Responses API request
    #[allow(dead_code)]
    pub fn from_responses_request(req: &crate::models::response::ResponsesRequest) -> Self {
        Self {
            model: req.model.clone(),
            instructions: req.instructions.clone(),
            input: req.input.clone(),
            tools: req.tools.clone(),
            tool_choice: req.tool_choice.clone(),
            parallel_tool_calls: req.parallel_tool_calls.unwrap_or(true),
            temperature: req.temperature,
            top_p: req.top_p,
            max_tokens: req.max_tokens,
            seed: req.seed,
            reasoning: req.reasoning.clone(),
            structured_output: req.structured_output.clone(),
            text_format: req.text.clone(),
            previous_response_id: req.previous_response_id.clone(),
            store: req.store,
            prompt_cache_key: req.prompt_cache_key.clone(),
            include: req.include.clone(),
            metadata: req.metadata.clone(),
            model_settings: req.model_settings.clone(),
            stream: req.stream.unwrap_or(false),
        }
    }

    /// Convert to OpenAI Responses API request
    #[allow(dead_code)]
    pub fn to_responses_request(&self) -> crate::models::response::ResponsesRequest {
        crate::models::response::ResponsesRequest {
            model: self.model.clone(),
            input: self.input.clone(),
            instructions: self.instructions.clone(),
            tools: self.tools.clone(),
            tool_choice: self.tool_choice.clone(),
            parallel_tool_calls: Some(self.parallel_tool_calls),
            temperature: self.temperature,
            top_p: self.top_p,
            max_tokens: self.max_tokens,
            stream: Some(self.stream),
            include: self.include.clone(),
            text: self.text_format.clone(),
            structured_output: self.structured_output.clone(),
            previous_response_id: self.previous_response_id.clone(),
            store: self.store,
            metadata: self.metadata.clone(),
            reasoning: self.reasoning.clone(),
            prompt_cache_key: self.prompt_cache_key.clone(),
            model_settings: self.model_settings.clone(),
            stop: None,
            seed: self.seed,
            user: None,
            service_tier: None,
            namespace: None,
        }
    }

    /// Get the list of tool types used in this request
    #[allow(dead_code)]
    pub fn tool_types(&self) -> Vec<CanonicalToolType> {
        self.tools
            .iter()
            .filter_map(|t| CanonicalToolType::from_str(&t.tool_type))
            .collect()
    }

    /// Check if a specific tool type is used
    #[allow(dead_code)]
    pub fn has_tool_type(&self, tool_type: CanonicalToolType) -> bool {
        self.tools.iter().any(|t| t.tool_type == tool_type.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(tool_type: &str) -> crate::models::response::Tool {
        crate::models::response::Tool {
            tool_type: tool_type.to_string(),
            function: if tool_type == "function" {
                Some(crate::models::response::FunctionDefinition {
                    name: Some("get_weather".to_string()),
                    description: Some("Get weather".to_string()),
                    parameters: None,
                    strict: None,
                })
            } else {
                None
            },
            vector_store_ids: None,
            display_width: None,
            display_height: None,
            environment: None,
            server_label: None,
            server_description: None,
            server_url: None,
            require_approval: None,
        }
    }

    #[test]
    fn test_canonical_request_roundtrip() {
        let original = CanonicalRequest {
            model: "gpt-4o".to_string(),
            instructions: Some("You are a helpful assistant.".to_string()),
            input: vec![],
            tools: vec![make_tool("function")],
            tool_choice: None,
            parallel_tool_calls: true,
            temperature: Some(0.7),
            top_p: None,
            max_tokens: Some(1000),
            seed: None,
            reasoning: None,
            structured_output: None,
            text_format: None,
            previous_response_id: None,
            store: None,
            prompt_cache_key: None,
            include: vec![],
            metadata: None,
            model_settings: None,
            stream: false,
        };

        let responses_req = original.to_responses_request();
        let canonical = CanonicalRequest::from_responses_request(&responses_req);

        assert_eq!(original.model, canonical.model);
        assert_eq!(original.instructions, canonical.instructions);
        assert_eq!(original.tools.len(), canonical.tools.len());
    }

    #[test]
    fn test_tool_type_recognition() {
        let req = CanonicalRequest {
            model: "gpt-4o".to_string(),
            instructions: None,
            input: vec![],
            tools: vec![make_tool("function"), make_tool("web_search")],
            tool_choice: None,
            parallel_tool_calls: true,
            temperature: None,
            top_p: None,
            max_tokens: None,
            seed: None,
            reasoning: None,
            structured_output: None,
            text_format: None,
            previous_response_id: None,
            store: None,
            prompt_cache_key: None,
            include: vec![],
            metadata: None,
            model_settings: None,
            stream: false,
        };

        assert!(req.has_tool_type(CanonicalToolType::Function));
        assert!(req.has_tool_type(CanonicalToolType::WebSearch));
        assert!(!req.has_tool_type(CanonicalToolType::FileSearch));
        assert!(!req.has_tool_type(CanonicalToolType::ComputerUse));
        assert!(!req.has_tool_type(CanonicalToolType::Mcp));

        let tool_types = req.tool_types();
        assert!(tool_types.contains(&CanonicalToolType::Function));
        assert!(tool_types.contains(&CanonicalToolType::WebSearch));
    }

    #[test]
    fn test_tool_type_from_str() {
        assert_eq!(
            CanonicalToolType::from_str("function"),
            Some(CanonicalToolType::Function)
        );
        assert_eq!(
            CanonicalToolType::from_str("web_search"),
            Some(CanonicalToolType::WebSearch)
        );
        assert_eq!(
            CanonicalToolType::from_str("file_search"),
            Some(CanonicalToolType::FileSearch)
        );
        assert_eq!(
            CanonicalToolType::from_str("computer_use"),
            Some(CanonicalToolType::ComputerUse)
        );
        assert_eq!(
            CanonicalToolType::from_str("computer"),
            Some(CanonicalToolType::ComputerUse)
        );
        assert_eq!(CanonicalToolType::from_str("mcp"), Some(CanonicalToolType::Mcp));
        assert_eq!(CanonicalToolType::from_str("unknown"), None);
    }
}
