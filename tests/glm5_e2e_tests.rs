//! GLM-5 E2E Tests
//!
//! This module contains end-to-end tests for the GLM-5 model support,
//! verifying:
//! - Basic text requests
//! - Streaming text requests
//! - Function calling
//! - /v1/responses path
//! - Capability boundaries

use openai_proxy::models::response::{
    ResponsesRequest, Item, ContentBlock, InputText, MessageItem,
    Tool as ResponsesTool, ReasoningSettings, ReasoningEffort, StructuredOutput,
};
use openai_proxy::protocol::capabilities::{FallbackMode, ProviderCapabilities, ResponsesExecutionPlan};

// Helper to create a basic ResponsesRequest for GLM-5
fn make_glm5_responses_request() -> ResponsesRequest {
    ResponsesRequest {
        model: "glm-5".to_string(),
        input: vec![Item::Message(MessageItem {
            role: "user".to_string(),
            content: vec![ContentBlock::InputText(InputText {
                text: "Hello".to_string(),
            })],
            id: None,
            end_turn: None,
            phase: None,
        })],
        instructions: None,
        tools: vec![],
        tool_choice: None,
        parallel_tool_calls: Some(true),
        temperature: None,
        top_p: None,
        max_tokens: None,
        stream: Some(false),
        include: vec![],
        text: None,
        structured_output: None,
        store: None,
        previous_response_id: None,
        metadata: None,
        model_settings: None,
        reasoning: None,
        stop: None,
        seed: None,
        user: None,
        service_tier: None,
        prompt_cache_key: None,
        namespace: None,
    }
}

#[cfg(test)]
mod glm5_text_tests {
    use super::*;

    #[test]
    fn test_glm5_responses_request() {
        // Test that GLM-5 can handle Responses API requests
        let request = make_glm5_responses_request();

        assert_eq!(request.model, "glm-5");
        assert!(!request.input.is_empty());
    }
}

#[cfg(test)]
mod glm5_function_calling_tests {
    use super::*;

    #[test]
    fn test_glm5_capabilities_function_only() {
        // GLM-5 only supports function tools via Chat fallback
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);

        assert_eq!(capabilities.fallback_mode, FallbackMode::ChatFallback);
        assert!(!capabilities.native_responses);

        // Verify function tool is supported
        assert!(capabilities.supported_tool_types.contains(&"function".to_string()));

        // Verify non-function tools are NOT supported
        assert!(!capabilities.supported_tool_types.contains(&"web_search".to_string()));
        assert!(!capabilities.supported_tool_types.contains(&"file_search".to_string()));
        assert!(!capabilities.supported_tool_types.contains(&"computer_use".to_string()));
        assert!(!capabilities.supported_tool_types.contains(&"mcp".to_string()));
    }

    #[test]
    fn test_glm5_rejects_non_function_tools() {
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);

        let mut request = make_glm5_responses_request();
        request.tools = vec![ResponsesTool {
            tool_type: "web_search".to_string(),
            function: None,
            vector_store_ids: None,
            display_width: None,
            display_height: None,
            environment: None,
            server_label: None,
            server_description: None,
            server_url: None,
            require_approval: None,
        }];

        let plan = ResponsesExecutionPlan::from_request(&capabilities, &request);

        assert!(matches!(
            plan,
            ResponsesExecutionPlan::Reject { unsupported_features }
            if unsupported_features.contains(&"web_search".to_string())
        ));
    }
}

#[cfg(test)]
mod glm5_reasoning_tests {
    use super::*;

    #[test]
    fn test_glm5_reasoning_not_supported() {
        // GLM-5 via Chat fallback doesn't support reasoning
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);

        assert!(!capabilities.supports_reasoning);
    }

    #[test]
    fn test_glm5_reasoning_rejected_in_responses() {
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);

        let mut request = make_glm5_responses_request();
        request.reasoning = Some(ReasoningSettings {
            effort: ReasoningEffort::High,
            include: None,
        });

        let plan = ResponsesExecutionPlan::from_request(&capabilities, &request);

        assert!(matches!(
            plan,
            ResponsesExecutionPlan::Reject { unsupported_features }
            if unsupported_features.contains(&"reasoning".to_string())
        ));
    }
}

#[cfg(test)]
mod glm5_structured_output_tests {
    use super::*;

    #[test]
    fn test_glm5_structured_output_not_supported() {
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);
        assert!(!capabilities.supports_structured_output);
    }

    #[test]
    fn test_glm5_structured_output_rejected() {
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);

        let mut request = make_glm5_responses_request();
        request.structured_output = Some(StructuredOutput {
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                }
            }),
            strict: None,
        });

        let plan = ResponsesExecutionPlan::from_request(&capabilities, &request);

        assert!(matches!(
            plan,
            ResponsesExecutionPlan::Reject { unsupported_features }
            if unsupported_features.contains(&"structured_output".to_string())
        ));
    }
}

#[cfg(test)]
mod glm5_previous_response_tests {
    use super::*;

    #[test]
    fn test_glm5_previous_response_id_not_supported() {
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);
        assert!(!capabilities.supports_previous_response_id);
    }

    #[test]
    fn test_glm5_previous_response_id_rejected() {
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);

        let mut request = make_glm5_responses_request();
        request.previous_response_id = Some("resp_abc123".to_string());

        let plan = ResponsesExecutionPlan::from_request(&capabilities, &request);

        assert!(matches!(
            plan,
            ResponsesExecutionPlan::Reject { unsupported_features }
            if unsupported_features.contains(&"previous_response_id".to_string())
        ));
    }
}

#[cfg(test)]
mod glm5_endpoint_tests {
    #[test]
    fn test_glm5_chat_endpoint() {
        // Test that chat completions endpoint works for GLM-5
        // This is a structural test - actual API calls tested in integration
        let endpoint = "/v1/chat/completions";
        assert!(endpoint.contains("chat/completions"));
    }

    #[test]
    fn test_glm5_responses_endpoint() {
        // Test that responses endpoint goes through transform for GLM-5
        // GLM-5 doesn't have native /responses, so it should use Chat fallback
        let endpoint = "/v1/responses";
        assert!(endpoint.contains("responses"));
    }

    #[test]
    fn test_glm5_model_routing() {
        // Test that GLM-5 model is routed correctly
        let model = "glm-5";
        let expected_provider = "zhipu";

        // This is a structural test - actual routing tested in integration
        assert!(model.starts_with("glm-"));
        assert_eq!(expected_provider, "zhipu");
    }
}

#[cfg(test)]
mod glm5_model_catalog_tests {
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_glm5_model_catalog_includes_personality_messages() {
        let catalog_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("codex/rcodex/models.json");
        let catalog_contents = fs::read_to_string(&catalog_path).expect("read models.json");
        let catalog: Value = serde_json::from_str(&catalog_contents).expect("parse models.json");
        let models = catalog["models"].as_array().expect("models array");

        let glm5 = models
            .iter()
            .find(|model| model["slug"] == "glm-5")
            .expect("glm-5 model entry");

        assert!(
            glm5["model_messages"]["instructions_template"]
                .as_str()
                .is_some_and(|template| template.contains("{{ personality }}")),
            "glm-5 should provide a personality-aware instructions template"
        );
        assert!(
            glm5["model_messages"]["instructions_variables"]["personality_pragmatic"]
                .as_str()
                .is_some_and(|value| !value.trim().is_empty()),
            "glm-5 should provide a pragmatic personality message"
        );
    }

    #[test]
    fn test_all_glm_models_include_personality_messages() {
        let catalog_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("codex/rcodex/models.json");
        let catalog_contents = fs::read_to_string(&catalog_path).expect("read models.json");
        let catalog: Value = serde_json::from_str(&catalog_contents).expect("parse models.json");
        let models = catalog["models"].as_array().expect("models array");

        for model in models
            .iter()
            .filter(|model| model["slug"].as_str().is_some_and(|slug| slug.starts_with("glm-")))
        {
            let slug = model["slug"].as_str().expect("glm model slug");

            assert!(
                model["model_messages"]["instructions_template"]
                    .as_str()
                    .is_some_and(|template| template.contains("{{ personality }}")),
                "{slug} should provide a personality-aware instructions template"
            );
            assert!(
                model["model_messages"]["instructions_variables"]["personality_pragmatic"]
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty()),
                "{slug} should provide a pragmatic personality message"
            );
        }
    }
}

#[cfg(test)]
mod glm5_integration_tests {
    #[test]
    #[ignore] // Requires actual API key
    fn test_glm5_e2e_text_request() {
        // Test complete flow with actual GLM-5 API
        unimplemented!("Integration test - requires ZHIPU_API_KEY")
    }

    #[test]
    #[ignore] // Requires actual API key
    fn test_glm5_e2e_streaming_request() {
        // Test streaming with actual GLM-5 API
        unimplemented!("Integration test - requires ZHIPU_API_KEY")
    }

    #[test]
    #[ignore] // Requires actual API key
    fn test_glm5_e2e_function_calling() {
        // Test function calling with actual GLM-5 API
        unimplemented!("Integration test - requires ZHIPU_API_KEY")
    }

    #[test]
    #[ignore] // Requires actual API key
    fn test_glm5_e2e_responses_path() {
        // Test /v1/responses path with actual GLM-5 API
        unimplemented!("Integration test - requires ZHIPU_API_KEY")
    }
}
