//! Transform layer for API conversion
//!
//! This module implements transformation logic between:
//! - Chat Completions API ↔ Responses API
//!
//! # Request Transformations
//! - [`transform_chat_to_responses_request`] - Convert Chat Completions request to Responses API
//! - [`transform_responses_to_chat_request`] - Convert Responses API request to Chat Completions
//!
//! # Response Transformations
//! - [`transform_chat_to_responses_response`] - Convert Chat Completions response to Responses API
//! - [`transform_responses_to_chat_response`] - Convert Responses API response to Chat Completions
//!
//! # Streaming Transformations
//! - [`transform_chat_stream_to_responses_stream`] - Convert Chat streaming to Responses streaming
//! - [`transform_responses_stream_to_chat_stream`] - Convert Responses streaming to Chat streaming

use crate::models::chat::{
    ChatCompletionChunk, ChatRequest, ChatResponse, Choice,
    Message as ChatMessage, Usage as ChatUsage, Delta as ChatDelta,
    StreamingChoice as ChatStreamingChoice, ToolCall as ChatToolCall,
};
use crate::models::response::{
    ContentBlock, InputText, Item, MessageItem, OutputItem,
    ResponsesRequest, ResponsesResponse, ToolCallOutput, FunctionCallOutputFunction,
    Usage as ResponsesUsage,
};
use crate::models::response::StreamOutputItem;
use crate::models::response::ResponsesStreamChunk;

/// Transform Chat Completions Request → Responses API Request
#[allow(dead_code)]
pub fn transform_chat_to_responses_request(chat_req: &ChatRequest) -> ResponsesRequest {
    let input: Vec<Item> = chat_req
        .messages
        .iter()
        .map(|msg| {
            Item::Message(MessageItem {
                role: msg.role.clone(),
                content: vec![ContentBlock::InputText(InputText {
                    text: msg.content.clone().unwrap_or_default(),
                })],
            })
        })
        .collect();

    let tools = chat_req.tools.as_ref().map(|tools| {
        tools.iter().map(|t| {
            crate::models::response::Tool {
                tool_type: t.tool_type.clone(),
                function: crate::models::response::FunctionDefinition {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: t.function.parameters.clone(),
                    strict: None,
                },
            }
        }).collect()
    }).unwrap_or_default();

    ResponsesRequest {
        model: chat_req.model.clone(),
        input,
        instructions: None,
        tools,
        temperature: chat_req.temperature,
        top_p: chat_req.top_p,
        max_tokens: chat_req.max_tokens,
        stream: chat_req.stream,
        text: chat_req.response_format.as_ref().map(|rf| {
            crate::models::response::TextFormat {
                format: Some(match rf {
                    crate::models::chat::ResponseFormat::Text => crate::models::response::TextFormatType::Text,
                    crate::models::chat::ResponseFormat::JsonObject => crate::models::response::TextFormatType::JsonObject,
                    crate::models::chat::ResponseFormat::JsonSchema => crate::models::response::TextFormatType::JsonSchema,
                }),
            }
        }),
        structured_output: None,
        store: None,
        metadata: None,
        model_settings: None,
        reasoning: None,
        stop: chat_req.stop.clone(),
        seed: chat_req.seed,
        user: chat_req.user.clone(),
    }
}

/// Transform Responses API Request → Chat Completions Request
pub fn transform_responses_to_chat_request(responses_req: &ResponsesRequest) -> ChatRequest {
    let messages: Vec<ChatMessage> = responses_req
        .input
        .iter()
        .filter_map(|item| {
            match item {
                Item::Message(msg) => Some(ChatMessage {
                    role: msg.role.clone(),
                    content: msg.content.iter().find_map(|c| {
                        match c {
                            ContentBlock::InputText(text) => Some(text.text.clone()),
                            _ => None,
                        }
                    }),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                }),
                Item::FunctionCall(func) => Some(ChatMessage {
                    role: "assistant".to_string(),
                    content: None,
                    name: None,
                    tool_calls: Some(vec![crate::models::chat::ToolCall {
                        id: func.call_id.clone(),
                        call_type: "function".to_string(),
                        function: crate::models::chat::FunctionCall {
                            name: func.name.clone(),
                            arguments: func.arguments.clone(),
                        },
                    }]),
                    tool_call_id: None,
                }),
                _ => None,
            }
        })
        .collect();

    let tools = if responses_req.tools.is_empty() {
        None
    } else {
        Some(responses_req.tools.iter().map(|t| {
            crate::models::chat::Tool {
                tool_type: t.tool_type.clone(),
                function: crate::models::chat::FunctionDefinition {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: t.function.parameters.clone(),
                },
            }
        }).collect())
    };

    let response_format = responses_req.text.as_ref().and_then(|t| {
        t.format.as_ref().map(|f| {
            match f {
                crate::models::response::TextFormatType::Text => crate::models::chat::ResponseFormat::Text,
                crate::models::response::TextFormatType::JsonObject => crate::models::chat::ResponseFormat::JsonObject,
                crate::models::response::TextFormatType::JsonSchema => crate::models::chat::ResponseFormat::JsonSchema,
            }
        })
    });

    ChatRequest {
        model: responses_req.model.clone(),
        messages,
        temperature: responses_req.temperature,
        top_p: responses_req.top_p,
        max_tokens: responses_req.max_tokens,
        stream: responses_req.stream,
        stop: responses_req.stop.clone(),
        n: 1,
        stream_options: None,
        include_usage: Some(true),
        response_format,
        seed: responses_req.seed,
        organization: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: responses_req.user.clone(),
        tools,
        tool_choice: None,
        parallel_tool_calls: true,
    }
}

/// Transform Chat Completions Response → Responses API Response
pub fn transform_chat_to_responses_response(chat_resp: &ChatResponse) -> ResponsesResponse {
    let output: Vec<OutputItem> = chat_resp.choices.iter().enumerate().map(|(idx, choice)| {
        let content_text = choice.message.content.clone().unwrap_or_default();
        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls.iter().map(|tc| {
                ToolCallOutput {
                    id: tc.id.clone(),
                    call_type: tc.call_type.clone(),
                    function: FunctionCallOutputFunction {
                        name: tc.function.name.clone(),
                        arguments: tc.function.arguments.clone(),
                    },
                }
            }).collect()
        });

        OutputItem::Message(crate::models::response::MessageOutput {
            index: idx as u32,
            role: choice.message.role.clone(),
            content: vec![ContentBlock::OutputText(crate::models::response::OutputText {
                text: content_text,
                annotations: None,
            })],
            status: choice.finish_reason.clone(),
            tool_calls,
        })
    }).collect();

    let usage = chat_resp.usage.as_ref().map(|u| ResponsesUsage {
        input_tokens: u.prompt_tokens,
        output_tokens: u.completion_tokens,
        tokens: None,
        total_tokens: u.total_tokens,
    });

    ResponsesResponse {
        id: chat_resp.id.clone(),
        object: "response".to_string(),
        created: chat_resp.created,
        model: chat_resp.model.clone(),
        output,
        usage,
        finish_reason: chat_resp.finish_reason.clone(),
        extra: chat_resp.extra.clone(),
    }
}

/// Transform Responses API Response → Chat Completions Response
#[allow(dead_code)]
pub fn transform_responses_to_chat_response(responses_resp: &ResponsesResponse) -> ChatResponse {
    let choices: Vec<Choice> = responses_resp
        .output
        .iter()
        .filter_map(|item| {
            match item {
                OutputItem::Message(msg) => {
                    let content = msg.content.iter().find_map(|c| {
                        match c {
                            ContentBlock::OutputText(text) => Some(text.text.clone()),
                            _ => None,
                        }
                    });

                    let tool_calls = msg.tool_calls.as_ref().map(|calls| {
                        calls.iter().map(|tc| {
                            crate::models::chat::ToolCall {
                                id: tc.id.clone(),
                                call_type: tc.call_type.clone(),
                                function: crate::models::chat::FunctionCall {
                                    name: tc.function.name.clone(),
                                    arguments: tc.function.arguments.clone(),
                                },
                            }
                        }).collect()
                    });

                    Some(Choice {
                        index: msg.index,
                        message: ChatMessage {
                            role: msg.role.clone(),
                            content,
                            name: None,
                            tool_calls,
                            tool_call_id: None,
                        },
                        finish_reason: msg.status.clone(),
                        logprobs: None,
                    })
                }
                _ => None,
            }
        })
        .collect();

    let usage = responses_resp.usage.as_ref().map(|u| ChatUsage {
        prompt_tokens: u.input_tokens,
        completion_tokens: u.output_tokens,
        total_tokens: u.total_tokens,
    });

    ChatResponse {
        id: responses_resp.id.clone(),
        object: "chat.completion".to_string(),
        created: responses_resp.created,
        model: responses_resp.model.clone(),
        choices,
        usage,
        service_tier: None,
        finish_reason: responses_resp.finish_reason.clone(),
        extra: responses_resp.extra.clone(),
    }
}

/// Transform Chat Completions streaming chunk → Responses API streaming chunk
pub fn transform_chat_stream_to_responses_stream(
    chat_chunk: &ChatCompletionChunk,
) -> ResponsesStreamChunk {
    let output: Vec<StreamOutputItem> = chat_chunk
        .choices
        .iter()
        .filter_map(|choice| {
            let delta = choice.delta.as_ref()?;
            Some(StreamOutputItem::Message(crate::models::response::StreamMessageDelta {
                index: choice.index,
                delta: Some(crate::models::response::MessageDelta {
                    role: delta.role.clone(),
                    content: delta.content.clone(),
                    tool_calls: delta.tool_calls.as_ref().map(|calls| {
                        calls.iter().map(|tc| {
                            ToolCallOutput {
                                id: tc.id.clone(),
                                call_type: tc.call_type.clone(),
                                function: FunctionCallOutputFunction {
                                    name: tc.function.name.clone(),
                                    arguments: tc.function.arguments.clone(),
                                },
                            }
                        }).collect()
                    }),
                }),
                status: choice.finish_reason.clone(),
            }))
        })
        .collect();

    // For streaming chunks, usage is typically None until the final chunk
    let usage = None;

    ResponsesStreamChunk {
        id: chat_chunk.id.clone(),
        object: "response.stream".to_string(),
        created: chat_chunk.created,
        model: chat_chunk.model.clone(),
        output,
        usage,
    }
}

/// Transform Responses API streaming chunk → Chat Completions streaming chunk
#[allow(dead_code)]
pub fn transform_responses_stream_to_chat_stream(
    responses_chunk: &ResponsesStreamChunk,
) -> ChatCompletionChunk {
    let choices: Vec<ChatStreamingChoice> = responses_chunk
        .output
        .iter()
        .filter_map(|item| {
            match item {
                StreamOutputItem::Message(msg) => {
                    let delta = msg.delta.as_ref()?;
                    Some(ChatStreamingChoice {
                        index: msg.index,
                        delta: Some(ChatDelta {
                            role: delta.role.clone(),
                            content: delta.content.clone(),
                            tool_calls: delta.tool_calls.as_ref().map(|calls| {
                                calls.iter().map(|tc| {
                                    ChatToolCall {
                                        id: tc.id.clone(),
                                        call_type: tc.call_type.clone(),
                                        function: crate::models::chat::FunctionCall {
                                            name: tc.function.name.clone(),
                                            arguments: tc.function.arguments.clone(),
                                        },
                                    }
                                }).collect()
                            }),
                        }),
                        finish_reason: msg.status.clone(),
                        logprobs: None,
                    })
                }
                _ => None,
            }
        })
        .collect();

    // Convert usage from response format to chat format
    let usage = responses_chunk.usage.as_ref().map(|u| ChatUsage {
        prompt_tokens: u.input_tokens,
        completion_tokens: u.output_tokens,
        total_tokens: u.total_tokens,
    });

    ChatCompletionChunk {
        id: responses_chunk.id.clone(),
        object: "chat.completion.chunk".to_string(),
        created: responses_chunk.created,
        model: responses_chunk.model.clone(),
        choices,
        usage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_chat_to_responses_request() {
        let chat_req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: Some("Hello".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.7),
            top_p: None,
            max_tokens: Some(1000),
            stream: None,
            stop: None,
            n: 1,
            stream_options: None,
            include_usage: Some(true),
            response_format: None,
            seed: None,
            organization: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: true,
        };

        let responses_req = transform_chat_to_responses_request(&chat_req);

        assert_eq!(responses_req.model, "gpt-4");
        assert_eq!(responses_req.input.len(), 1);
        assert_eq!(responses_req.temperature, Some(0.7));
        assert_eq!(responses_req.max_tokens, Some(1000));
    }

    #[test]
    fn test_transform_responses_to_chat_request() {
        let responses_req = ResponsesRequest {
            model: "gpt-4".to_string(),
            input: vec![Item::Message(MessageItem {
                role: "user".to_string(),
                content: vec![ContentBlock::InputText(InputText {
                    text: "Hello".to_string(),
                })],
            })],
            instructions: None,
            tools: vec![],
            temperature: Some(0.7),
            top_p: None,
            max_tokens: Some(1000),
            stream: None,
            text: None,
            structured_output: None,
            store: None,
            metadata: None,
            model_settings: None,
            reasoning: None,
            stop: None,
            seed: None,
            user: None,
        };

        let chat_req = transform_responses_to_chat_request(&responses_req);

        assert_eq!(chat_req.model, "gpt-4");
        assert_eq!(chat_req.messages.len(), 1);
        assert_eq!(chat_req.messages[0].content.as_ref().unwrap(), "Hello");
    }

    #[test]
    fn test_transform_chat_to_responses_response() {
        let chat_resp = ChatResponse {
            id: "chat-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
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
            finish_reason: Some("stop".to_string()),
            extra: std::collections::HashMap::new(),
        };

        let responses_resp = transform_chat_to_responses_response(&chat_resp);

        assert_eq!(responses_resp.id, "chat-123");
        assert_eq!(responses_resp.model, "gpt-4");
        assert!(responses_resp.output.len() > 0);
    }

    #[test]
    fn test_transform_responses_to_chat_response() {
        let responses_resp = ResponsesResponse {
            id: "resp-123".to_string(),
            object: "response".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            output: vec![OutputItem::Message(crate::models::response::MessageOutput {
                index: 0,
                role: "assistant".to_string(),
                content: vec![ContentBlock::OutputText(crate::models::response::OutputText {
                    text: "Hello!".to_string(),
                    annotations: None,
                })],
                status: Some("completed".to_string()),
                tool_calls: None,
            })],
            usage: Some(ResponsesUsage {
                input_tokens: 10,
                output_tokens: 5,
                tokens: None,
                total_tokens: 15,
            }),
            finish_reason: Some("stop".to_string()),
            extra: std::collections::HashMap::new(),
        };

        let chat_resp = transform_responses_to_chat_response(&responses_resp);

        assert_eq!(chat_resp.id, "resp-123");
        assert_eq!(chat_resp.model, "gpt-4");
        assert!(chat_resp.choices.len() > 0);
    }
}
