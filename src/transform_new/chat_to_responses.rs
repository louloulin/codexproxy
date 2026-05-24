//! Chat Completions API to Responses API transformer

use crate::models::chat::{ChatResponse, ChatCompletionChunk};
use crate::models::response::{
    ContentBlock, FunctionCallOutput, FunctionCallOutputItem, MessageOutput, OutputItem, OutputText,
    ResponsesObject, Usage,
};

/// Options for Chat to Responses conversion
#[derive(Debug, Clone, Default)]
pub struct ChatToResponsesOptions {
    pub extract_inline_think: bool,
    pub expose_reasoning: bool,
}

/// Convert Chat response to Responses object
pub fn chat_to_responses(chat: &ChatResponse, _opts: &ChatToResponsesOptions) -> ResponsesObject {
    let mut output: Vec<OutputItem> = Vec::new();
    
    if let Some(choice) = chat.choices.first() {
        let msg = &choice.message;
        
        // Handle content
        if let Some(ref content) = msg.content {
            output.push(OutputItem::Message(MessageOutput {
                index: output.len() as u32,
                id: Some(new_id()),
                role: "assistant".to_string(),
                status: Some("completed".to_string()),
                content: vec![ContentBlock::OutputText(OutputText {
                    text: content.clone(),
                    annotations: None,
                })],
                end_turn: None,
                phase: None,
                tool_calls: None,
            }));
        }
        
        // Handle tool calls
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                output.push(OutputItem::FunctionCall(FunctionCallOutput {
                    index: output.len() as u32,
                    id: Some(new_id()),
                    call_id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    arguments: tc.function.arguments.clone(),
                    status: Some("completed".to_string()),
                }));
            }
        }
    }
    
    let usage = chat.usage.as_ref().map(|u| Usage {
        input_tokens: u.prompt_tokens as u64,
        input_tokens_details: None,
        output_tokens: u.completion_tokens as u64,
        output_tokens_details: None,
        total_tokens: u.total_tokens as u64,
    });
    
    ResponsesObject {
        id: new_response_id(chat.id.as_str()),
        object: "response".to_string(),
        status: "completed".to_string(),
        model: chat.model.clone(),
        output,
        usage,
        ..Default::default()
    }
}

/// Streaming tool call state for accumulation
struct StreamingToolCall {
    call_id: String,
    name: String,
    arguments: String,
}

/// Convert Chat streaming chunks to Responses output items
pub fn chat_chunks_to_responses_output(chunks: &[ChatCompletionChunk]) -> Vec<OutputItem> {
    let mut output: Vec<OutputItem> = Vec::new();
    let mut content_buffer = String::new();
    let mut tool_call_map: std::collections::HashMap<usize, StreamingToolCall> = std::collections::HashMap::new();
    let mut last_tool_call_order: Vec<String> = Vec::new(); // Track order by call_id
    
    for chunk in chunks {
        for choice in &chunk.choices {
            if let Some(delta) = &choice.delta {
                // Handle content
                if let Some(ref content) = delta.content {
                    content_buffer.push_str(content);
                }
                
                // Handle tool calls - accumulate by id
                if let Some(ref tool_calls) = delta.tool_calls {
                    for tc_delta in tool_calls {
                        let call_id = tc_delta.id.clone();
                        
                        // Check if we already have this tool call
                        if let Some(existing_idx) = last_tool_call_order.iter().position(|id| id == &call_id) {
                            // Append to existing tool call
                            if let Some(tc) = tool_call_map.get_mut(&existing_idx) {
                                tc.arguments.push_str(&tc_delta.function.arguments);
                            }
                        } else {
                            // Start new tool call
                            let idx = tool_call_map.len();
                            tool_call_map.insert(idx, StreamingToolCall {
                                call_id: call_id.clone(),
                                name: tc_delta.function.name.clone(),
                                arguments: tc_delta.function.arguments.clone(),
                            });
                            last_tool_call_order.push(call_id);
                        }
                    }
                }
            }
        }
    }
    
    // Flush content buffer as message output
    if !content_buffer.is_empty() {
        output.push(OutputItem::Message(MessageOutput {
            index: output.len() as u32,
            id: Some(new_id()),
            role: "assistant".to_string(),
            status: Some("completed".to_string()),
            content: vec![ContentBlock::OutputText(OutputText {
                text: content_buffer,
                annotations: None,
            })],
            end_turn: None,
            phase: None,
            tool_calls: None,
        }));
    }
    
    // Output tool calls in order
    for idx in 0..tool_call_map.len() {
        if let Some(tc) = tool_call_map.get(&idx) {
            output.push(OutputItem::FunctionCall(FunctionCallOutput {
                index: output.len() as u32,
                id: Some(tc.call_id.clone()),
                call_id: tc.call_id.clone(),
                name: tc.name.clone(),
                arguments: tc.arguments.clone(),
                status: Some("completed".to_string()),
            }));
        }
    }
    
    output
}

/// Convert function call output item (tool result) to Chat format
pub fn function_call_output_to_chat(item: &FunctionCallOutputItem) -> crate::models::chat::Message {
    crate::models::chat::Message {
        role: "tool".to_string(),
        content: Some(item.output.clone()),
        name: None,
        tool_call_id: Some(item.call_id.clone()),
        tool_calls: None,
    }
}

fn new_id() -> String {
    format!("id_{}", uuid::Uuid::new_v4())
}

fn new_response_id(prefix: &str) -> String {
    if prefix.starts_with("chatcmpl-") {
        prefix.replace("chatcmpl-", "resp_")
    } else {
        format!("resp_{}", prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::models::chat::{Choice, FunctionCall, Message, ToolCall, Usage as ChatUsage};

    fn create_test_chat_response() -> ChatResponse {
        ChatResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some("Hello!".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
            service_tier: None,
            finish_reason: None,
            extra: HashMap::new(),
        }
    }

    #[test]
    fn test_simple_chat_to_responses() {
        let chat = create_test_chat_response();
        
        let responses = chat_to_responses(&chat, &ChatToResponsesOptions::default());
        assert_eq!(responses.model, "gpt-4");
        assert_eq!(responses.output.len(), 1);
    }

    #[test]
    fn test_tool_call_chat_to_responses() {
        let mut chat = create_test_chat_response();
        chat.id = "chatcmpl-456".to_string();
        chat.choices[0].message.content = None;
        chat.choices[0].message.tool_calls = Some(vec![
            ToolCall {
                id: "call_123".to_string(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: "get_weather".to_string(),
                    arguments: r#"{"city":"Beijing"}"#.to_string(),
                },
            },
        ]);
        chat.choices[0].finish_reason = Some("tool_calls".to_string());
        
        let responses = chat_to_responses(&chat, &ChatToResponsesOptions::default());
        assert_eq!(responses.model, "gpt-4");
        assert_eq!(responses.output.len(), 1);
        
        if let OutputItem::FunctionCall(fc) = &responses.output[0] {
            assert_eq!(fc.name, "get_weather");
            assert_eq!(fc.arguments, r#"{"city":"Beijing"}"#);
        } else {
            panic!("Expected FunctionCall output");
        }
    }

    #[test]
    fn test_streaming_chunks_to_responses() {
        use crate::models::chat::{ChatCompletionChunk, StreamingChoice, Delta};
        
        let chunks = vec![
            ChatCompletionChunk {
                id: "chunk-1".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567890,
                model: "gpt-4".to_string(),
                choices: vec![StreamingChoice {
                    index: 0,
                    delta: Some(Delta {
                        role: Some("assistant".to_string()),
                        content: Some("Hello".to_string()),
                        tool_calls: None,
                        reasoning_content: None,
                        reasoning_summary_text: None,}),
                    finish_reason: None,
                    logprobs: None,
                }],
                usage: None,
            },
            ChatCompletionChunk {
                id: "chunk-2".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567890,
                model: "gpt-4".to_string(),
                choices: vec![StreamingChoice {
                    index: 0,
                    delta: Some(Delta {
                        role: None,
                        content: Some(" World".to_string()),
                        tool_calls: None,
                        reasoning_content: None,
                        reasoning_summary_text: None,}),
                    finish_reason: None,
                    logprobs: None,
                }],
                usage: None,
            },
        ];
        
        let output = chat_chunks_to_responses_output(&chunks);
        assert_eq!(output.len(), 1);
        
        if let OutputItem::Message(msg) = &output[0] {
            if let ContentBlock::OutputText(text) = &msg.content[0] {
                assert_eq!(text.text, "Hello World");
            }
        }
    }
}

