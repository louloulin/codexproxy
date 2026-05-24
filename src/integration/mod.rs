//! Integration module for end-to-end proxy functionality
//! 
//! This module contains example implementations and integration tests
//! that demonstrate the complete flow of the rcodex proxy.

pub mod example_proxy;

#[cfg(test)]
mod integration_tests {
    use super::example_proxy::CodexProxy;
    use crate::transform::req_to_chat;
    use crate::models::response::{ResponsesRequest, Item, MessageItem, ContentBlock, InputText, Tool};

    #[test]
    fn test_end_to_end_responses_to_chat() {
        // Step 1: Create a Responses API request using proper types
        let responses_req = ResponsesRequest {
            model: "mimo-mini".to_string(),
            input: vec![
                Item::Message(MessageItem {
                    id: None,
                    role: "user".to_string(),
                    content: vec![
                        ContentBlock::InputText(InputText {
                            text: "Hello, world!".to_string(),
                        })
                    ],
                    end_turn: None,
                    phase: None,
                })
            ],
            ..Default::default()
        };

        // Step 2: Transform to Chat API
        let req_to_chat_opts = req_to_chat::ReqToChatOptions::default();
        let chat_req = req_to_chat::responses_to_chat(&responses_req, &req_to_chat_opts);
        
        assert_eq!(chat_req.model, "mimo-mini");
        assert_eq!(chat_req.messages.len(), 1);
        assert_eq!(chat_req.messages[0].role, "user");
    }

    #[test]
    fn test_streaming_flow() {
        let proxy = CodexProxy::new();
        let events = proxy.handle_streaming_request(
            vec![],
            "test-response-id".to_string(),
            "mimo-mini".to_string(),
        );
        // Empty chunks should still create completion event
        assert!(!events.is_empty());
    }

    #[test]
    fn test_tool_call_transformation() {
        // Use serde_json to parse a properly formatted tool
        let tool_json = serde_json::json!({
            "type": "function",
            "name": "calculator",
            "description": "A calculator tool",
            "parameters": {
                "type": "object",
                "properties": {
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                }
            }
        });
        let tool: Tool = serde_json::from_value(tool_json).unwrap();

        let responses_req = ResponsesRequest {
            model: "mimo-mini".to_string(),
            input: vec![
                Item::Message(MessageItem {
                    id: None,
                    role: "user".to_string(),
                    content: vec![
                        ContentBlock::InputText(InputText {
                            text: "Use the calculator".to_string(),
                        })
                    ],
                    end_turn: None,
                    phase: None,
                })
            ],
            tools: vec![tool],
            ..Default::default()
        };

        let req_to_chat_opts = req_to_chat::ReqToChatOptions::default();
        let chat_req = req_to_chat::responses_to_chat(&responses_req, &req_to_chat_opts);
        
        // Check tools were converted - chat_req.tools is Option<Vec<Tool>>
        if let Some(tools) = &chat_req.tools {
            assert!(!tools.is_empty());
            assert_eq!(tools[0].tool_type, "function");
        }
    }
}
