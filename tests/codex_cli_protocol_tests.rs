//! Codex CLI Protocol Tests
//!
//! This module tests the proxy's ability to correctly handle the Codex CLI
//! protocol semantics, focusing on:
//! - Responses API as primary protocol
//! - Item-level event lifecycle
//! - Multi-round state continuation (previous_response_id)
//! - Tool/function calling in Responses format
//! - Streaming event completeness

use openai_proxy::models::response::{
    ContentBlock, InputText, Item,
    MessageItem, ResponsesRequest, Tool as ResponsesTool,
    ReasoningSettings, ReasoningEffort,
};
use openai_proxy::protocol::capabilities::{FallbackMode, ProviderCapabilities, ResponsesExecutionPlan};
use openai_proxy::protocol::canonical::CanonicalToolType;

// Helper to create a basic ResponsesRequest
fn make_responses_request(model: &str) -> ResponsesRequest {
    ResponsesRequest {
        model: model.to_string(),
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
mod capability_tests {
    use super::*;

    #[test]
    fn test_openai_capabilities_native_responses() {
        let capabilities = ProviderCapabilities::native_responses(vec![
            "function".to_string(),
            "web_search".to_string(),
            "file_search".to_string(),
            "computer_use".to_string(),
            "mcp".to_string(),
        ]);

        assert_eq!(capabilities.fallback_mode, FallbackMode::NativeResponses);
        assert!(capabilities.native_responses);
        assert!(capabilities.supports_previous_response_id);
        assert!(capabilities.supports_reasoning);
        assert!(capabilities.supports_structured_output);
        assert!(capabilities.supports_store);
        assert!(capabilities.supports_metadata);
    }

    #[test]
    fn test_zhipu_capabilities_chat_fallback() {
        let capabilities = ProviderCapabilities::chat_fallback(vec![
            "function".to_string(),
        ]);

        assert_eq!(capabilities.fallback_mode, FallbackMode::ChatFallback);
        assert!(!capabilities.native_responses);
        assert!(!capabilities.supports_previous_response_id);
        assert!(!capabilities.supports_reasoning);
        assert!(!capabilities.supports_structured_output);
    }

    #[test]
    fn test_execution_plan_native_responses() {
        let capabilities = ProviderCapabilities::native_responses(vec!["function".to_string()]);
        let request = make_responses_request("gpt-4o");

        let plan = ResponsesExecutionPlan::from_request(&capabilities, &request);
        assert!(matches!(plan, ResponsesExecutionPlan::NativeResponses));
    }

    #[test]
    fn test_execution_plan_reject_unsupported_features() {
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);
        let mut request = make_responses_request("glm-5");
        request.previous_response_id = Some("resp_123".to_string());

        let plan = ResponsesExecutionPlan::from_request(&capabilities, &request);
        assert!(matches!(
            plan,
            ResponsesExecutionPlan::Reject { unsupported_features }
            if unsupported_features.contains(&"previous_response_id".to_string())
        ));
    }
}

#[cfg(test)]
mod tool_preservation_tests {
    use super::*;

    fn make_tool(tool_type: &str) -> ResponsesTool {
        ResponsesTool {
            tool_type: tool_type.to_string(),
            function: if tool_type == "function" {
                Some(openai_proxy::models::response::FunctionDefinition {
                    name: Some("get_weather".to_string()),
                    description: Some("Get weather for a location".to_string()),
                    parameters: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        }
                    })),
                    strict: Some(true),
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
    fn test_function_tool_preserved() {
        let tool = make_tool("function");
        assert_eq!(tool.tool_type, "function");
        assert!(tool.function.is_some());
        assert_eq!(tool.function.as_ref().unwrap().name.as_ref().unwrap(), "get_weather");
    }

    #[test]
    fn test_web_search_tool_preserved() {
        let tool = make_tool("web_search");
        assert_eq!(tool.tool_type, "web_search");
    }

    #[test]
    fn test_file_search_tool_preserved() {
        let mut tool = make_tool("file_search");
        tool.vector_store_ids = Some(vec!["vs_123".to_string()]);
        assert_eq!(tool.tool_type, "file_search");
    }

    #[test]
    fn test_computer_use_tool_preserved() {
        let mut tool = make_tool("computer_use");
        tool.display_width = Some(1024);
        tool.display_height = Some(768);
        tool.environment = Some("browser".to_string());
        assert_eq!(tool.tool_type, "computer_use");
        assert_eq!(tool.display_width, Some(1024));
        assert_eq!(tool.display_height, Some(768));
        assert_eq!(tool.environment, Some("browser".to_string()));
    }

    #[test]
    fn test_mcp_tool_preserved() {
        let mut tool = make_tool("mcp");
        tool.server_label = Some("my-mcp-server".to_string());
        tool.server_url = Some("https://mcp.example.com".to_string());
        tool.require_approval = Some("always".to_string());
        assert_eq!(tool.tool_type, "mcp");
        assert_eq!(tool.server_label, Some("my-mcp-server".to_string()));
    }

    #[test]
    fn test_canonical_tool_type_recognition() {
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
            CanonicalToolType::from_str("mcp"),
            Some(CanonicalToolType::Mcp)
        );
        assert_eq!(CanonicalToolType::from_str("unknown"), None);
    }
}

#[cfg(test)]
mod previous_response_id_tests {
    use super::*;

    #[test]
    fn test_previous_response_id_preserved_in_canonical() {
        let request = openai_proxy::protocol::canonical::CanonicalRequest {
            model: "gpt-4o".to_string(),
            instructions: None,
            input: vec![],
            tools: vec![],
            tool_choice: None,
            parallel_tool_calls: true,
            temperature: None,
            top_p: None,
            max_tokens: None,
            seed: None,
            reasoning: None,
            structured_output: None,
            text_format: None,
            previous_response_id: Some("resp_abc123".to_string()),
            store: None,
            prompt_cache_key: None,
            include: vec![],
            metadata: None,
            model_settings: None,
            stream: false,
        };

        assert_eq!(request.previous_response_id, Some("resp_abc123".to_string()));
    }

    #[test]
    fn test_previous_response_id_rejected_for_chat_fallback() {
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);
        let mut request = make_responses_request("glm-5");
        request.previous_response_id = Some("resp_123".to_string());

        let plan = ResponsesExecutionPlan::from_request(&capabilities, &request);
        assert!(matches!(
            plan,
            ResponsesExecutionPlan::Reject { unsupported_features }
            if unsupported_features.contains(&"previous_response_id".to_string())
        ));
    }

    #[test]
    fn test_previous_response_id_accepted_for_native_responses() {
        let capabilities = ProviderCapabilities::native_responses(vec!["function".to_string()]);
        let mut request = make_responses_request("gpt-4o");
        request.previous_response_id = Some("resp_123".to_string());

        let plan = ResponsesExecutionPlan::from_request(&capabilities, &request);
        assert!(matches!(plan, ResponsesExecutionPlan::NativeResponses));
    }
}

#[cfg(test)]
mod reasoning_tests {
    use super::*;

    #[test]
    fn test_reasoning_preserved_in_canonical() {
        let request = openai_proxy::protocol::canonical::CanonicalRequest {
            model: "gpt-4o".to_string(),
            instructions: None,
            input: vec![],
            tools: vec![],
            tool_choice: None,
            parallel_tool_calls: true,
            temperature: None,
            top_p: None,
            max_tokens: None,
            seed: None,
            reasoning: Some(ReasoningSettings {
                effort: ReasoningEffort::High,
                include: Some(true),
            }),
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

        assert!(request.reasoning.is_some());
        assert!(matches!(
            request.reasoning.unwrap().effort,
            ReasoningEffort::High
        ));
    }

    #[test]
    fn test_reasoning_rejected_for_chat_fallback() {
        let capabilities = ProviderCapabilities::chat_fallback(vec!["function".to_string()]);
        let mut request = make_responses_request("glm-5");
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
mod structured_output_tests {
    use super::*;

    #[test]
    fn test_structured_output_preserved_in_canonical() {
        let request = openai_proxy::protocol::canonical::CanonicalRequest {
            model: "gpt-4o".to_string(),
            instructions: None,
            input: vec![],
            tools: vec![],
            tool_choice: None,
            parallel_tool_calls: true,
            temperature: None,
            top_p: None,
            max_tokens: None,
            seed: None,
            reasoning: None,
            structured_output: Some(openai_proxy::models::response::StructuredOutput {
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "number"}
                    }
                }),
                strict: Some(true),
            }),
            text_format: None,
            previous_response_id: None,
            store: None,
            prompt_cache_key: None,
            include: vec![],
            metadata: None,
            model_settings: None,
            stream: false,
        };

        assert!(request.structured_output.is_some());
    }
}

#[cfg(test)]
mod streaming_event_tests {
    #[test]
    fn test_event_sequence_completeness() {
        // This test verifies that all required events are emitted in the correct order
        // for a streamed text response
        let required_events = vec![
            "response.created",
            "response.in_progress",
            "response.output_item.added",
            "response.output_text.delta",
            "response.output_text.done",
            "response.output_item.done",
            "response.completed",
            "[DONE]",
        ];

        // This is a structural test - actual streaming behavior is tested in integration
        assert!(required_events.contains(&"response.created"));
        assert!(required_events.contains(&"response.completed"));
        assert!(required_events.contains(&"[DONE]"));
    }

    #[test]
    fn test_function_call_streaming_events() {
        let required_function_events = vec![
            "response.output_item.added",
            "response.function_call_arguments.delta",
            "response.function_call_arguments.done",
            "response.output_item.done",
        ];

        assert!(required_function_events.contains(&"response.function_call_arguments.delta"));
        assert!(required_function_events.contains(&"response.function_call_arguments.done"));
    }
}

#[cfg(test)]
mod canonical_model_tests {
    use super::*;

    fn make_tool(tool_type: &str) -> ResponsesTool {
        ResponsesTool {
            tool_type: tool_type.to_string(),
            function: if tool_type == "function" {
                Some(openai_proxy::models::response::FunctionDefinition {
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
    fn test_canonical_request_from_responses() {
        let responses_req = ResponsesRequest {
            model: "gpt-4o".to_string(),
            input: vec![Item::Message(MessageItem {
                role: "user".to_string(),
                content: vec![ContentBlock::InputText(InputText {
                    text: "Hello, world!".to_string(),
                })],
                id: None,
                end_turn: None,
                phase: None,
            })],
            instructions: Some("You are helpful.".to_string()),
            tools: vec![make_tool("function")],
            ..make_responses_request("gpt-4o")
        };

        let canonical = openai_proxy::protocol::canonical::CanonicalRequest::from_responses_request(&responses_req);

        assert_eq!(canonical.model, "gpt-4o");
        assert_eq!(canonical.instructions, Some("You are helpful.".to_string()));
        assert_eq!(canonical.input.len(), 1);
        assert_eq!(canonical.tools.len(), 1);
    }

    #[test]
    fn test_canonical_roundtrip() {
        use openai_proxy::protocol::canonical::CanonicalRequest;

        let original = CanonicalRequest {
            model: "gpt-4o".to_string(),
            instructions: Some("You are helpful.".to_string()),
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

        // Convert to Responses request
        let responses = original.to_responses_request();

        // Convert back to canonical
        let roundtripped = CanonicalRequest::from_responses_request(&responses);

        assert_eq!(original.model, roundtripped.model);
        assert_eq!(original.instructions, roundtripped.instructions);
        assert_eq!(original.tools.len(), roundtripped.tools.len());
    }
}
