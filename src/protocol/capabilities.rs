//! Provider capability planning for Responses API requests.
//!
//! The proxy supports multiple providers with different protocol surfaces.
//! These types let handlers decide whether a request can be served natively,
//! requires a lossy chat fallback, or must be rejected explicitly.

use crate::models::response::{ContentBlock, Item, ResponsesRequest};

/// Preferred protocol fallback mode for a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackMode {
    /// The provider can handle Responses requests natively.
    NativeResponses,
    /// The provider can only serve the request via a Chat fallback path.
    ChatFallback,
}

/// Declares which Responses features a provider can preserve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub fallback_mode: FallbackMode,
    pub native_responses: bool,
    pub supports_previous_response_id: bool,
    pub supports_include: bool,
    pub supports_reasoning: bool,
    pub supports_structured_output: bool,
    pub supports_store: bool,
    pub supports_metadata: bool,
    pub supports_model_settings: bool,
    pub supports_prompt_cache_key: bool,
    pub supports_multimodal_inputs: bool,
    pub supports_namespace: bool,
    pub supported_tool_types: Vec<String>,
}

impl ProviderCapabilities {
    pub fn native_responses(supported_tool_types: Vec<String>) -> Self {
        Self {
            fallback_mode: FallbackMode::NativeResponses,
            native_responses: true,
            supports_previous_response_id: true,
            supports_include: true,
            supports_reasoning: true,
            supports_structured_output: true,
            supports_store: true,
            supports_metadata: true,
            supports_model_settings: true,
            supports_prompt_cache_key: true,
            supports_multimodal_inputs: true,
            supports_namespace: true,
            supported_tool_types,
        }
    }

    pub fn chat_fallback(supported_tool_types: Vec<String>) -> Self {
        Self {
            fallback_mode: FallbackMode::ChatFallback,
            native_responses: false,
            supports_previous_response_id: false,
            supports_include: false,
            supports_reasoning: false,
            supports_structured_output: false,
            supports_store: false,
            supports_metadata: false,
            supports_model_settings: false,
            supports_prompt_cache_key: false,
            supports_multimodal_inputs: false,
            supports_namespace: false,
            supported_tool_types,
        }
    }

    /// Chat fallback with reasoning support (for models with built-in reasoning)
    pub fn chat_fallback_with_reasoning(supported_tool_types: Vec<String>) -> Self {
        Self {
            fallback_mode: FallbackMode::ChatFallback,
            native_responses: false,
            supports_previous_response_id: false,
            supports_include: false,
            supports_reasoning: true, // Enable reasoning for Codex CLI
            supports_structured_output: false,
            supports_store: false,
            supports_metadata: false,
            supports_model_settings: false,
            supports_prompt_cache_key: false,
            supports_multimodal_inputs: false,
            supports_namespace: false,
            supported_tool_types,
        }
    }

    /// Chat fallback with full support for Codex CLI (supports reasoning, store, web_search)
    pub fn chat_fallback_with_full_support(supported_tool_types: Vec<String>) -> Self {
        Self {
            fallback_mode: FallbackMode::ChatFallback,
            native_responses: false,
            supports_previous_response_id: false,
            supports_include: false,
            supports_reasoning: true, // Enable reasoning for Codex CLI
            supports_structured_output: false,
            supports_store: true, // Codex CLI uses store
            supports_metadata: false,
            supports_model_settings: true, // Codex CLI uses model settings
            supports_prompt_cache_key: true, // Codex CLI may use prompt caching
            supports_multimodal_inputs: false,
            supports_namespace: true, // Codex CLI uses namespace
            supported_tool_types,
        }
    }
}

/// The chosen execution strategy for a Responses request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponsesExecutionPlan {
    NativeResponses,
    ChatFallback,
    Reject { unsupported_features: Vec<String> },
}

impl ResponsesExecutionPlan {
    pub fn from_request(
        capabilities: &ProviderCapabilities,
        request: &ResponsesRequest,
    ) -> Self {
        tracing::info!(
            native_responses = capabilities.native_responses,
            "checking execution plan"
        );
        if capabilities.native_responses {
            return Self::NativeResponses;
        }

        let unsupported_features = unsupported_features_for_chat_fallback(capabilities, request);
        if unsupported_features.is_empty() {
            Self::ChatFallback
        } else {
            Self::Reject {
                unsupported_features,
            }
        }
    }
}

fn unsupported_features_for_chat_fallback(
    capabilities: &ProviderCapabilities,
    request: &ResponsesRequest,
) -> Vec<String> {
    let mut unsupported = Vec::new();

    tracing::info!(
        supports_namespace = capabilities.supports_namespace,
        has_namespace_in_request = request.namespace.is_some(),
        namespace_value = ?request.namespace,
        "checking namespace capability"
    );

    // Check all fields that might cause rejection
    // Note: include and metadata are handled separately (logged but not rejected)
    if request.previous_response_id.is_some() && !capabilities.supports_previous_response_id {
        unsupported.push("previous_response_id".to_string());
    }
    if request.reasoning.is_some() && !capabilities.supports_reasoning {
        unsupported.push("reasoning".to_string());
    }
    if request.structured_output.is_some() && !capabilities.supports_structured_output {
        unsupported.push("structured_output".to_string());
    }
    if request.store.is_some() && !capabilities.supports_store {
        unsupported.push("store".to_string());
    }
    if request.model_settings.is_some() && !capabilities.supports_model_settings {
        unsupported.push("model_settings".to_string());
    }
    if request.prompt_cache_key.is_some() && !capabilities.supports_prompt_cache_key {
        unsupported.push("prompt_cache_key".to_string());
    }
    if request.namespace.is_some() && !capabilities.supports_namespace {
        unsupported.push("namespace".to_string());
    }

    tracing::info!(
        unsupported = ?unsupported,
        "unsupported features"
    );

    for tool in &request.tools {
        tracing::info!(
            tool_type = %tool.tool_type,
            supported_types = ?capabilities.supported_tool_types,
            "checking tool type"
        );
        if !capabilities
            .supported_tool_types
            .iter()
            .any(|supported| supported == &tool.tool_type)
        {
            unsupported.push(tool.tool_type.clone());
        }
    }

    // For chat fallback, certain metadata fields (include, metadata) can be safely ignored
    // without rejecting the request. Only log warnings for these.
    if !request.include.is_empty() && !capabilities.supports_include {
        tracing::warn!(
            include_present = true,
            supports_include = capabilities.supports_include,
            "include field present but not supported - will be ignored in chat fallback"
        );
        // Don't add to unsupported - it's not a rejection reason for chat fallback
    }
    if request.metadata.is_some() && !capabilities.supports_metadata {
        tracing::warn!(
            metadata_present = true,
            supports_metadata = capabilities.supports_metadata,
            "metadata field present but not supported - will be ignored in chat fallback"
        );
        // Don't add to unsupported - it's not a rejection reason for chat fallback
    }

    if !capabilities.supports_multimodal_inputs {
        for item in &request.input {
            match item {
                Item::Message(message) => {
                    let contains_non_text = message.content.iter().any(|block| {
                        !matches!(
                            block,
                            ContentBlock::InputText(_)
                                | ContentBlock::OutputText(_)
                                | ContentBlock::Refusal(_)
                        )
                    });
                    if contains_non_text {
                        unsupported.push("multimodal_input".to_string());
                        break;
                    }
                }
                Item::ToolSearchOutput(_) => unsupported.push("tool_search_output".to_string()),
                Item::ImageGenerationCall(_) => {
                    unsupported.push("image_generation_call".to_string())
                }
                _ => {}
            }
        }
    }

    unsupported.sort();
    unsupported.dedup();

    tracing::info!(
        final_unsupported = ?unsupported,
        tool_count = request.tools.len(),
        "final unsupported features list"
    );

    unsupported
}

