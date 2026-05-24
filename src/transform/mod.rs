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
    ChatCompletionChunk, ChatRequest, ChatResponse, Choice, Delta as ChatDelta,
    Message as ChatMessage, StreamingChoice as ChatStreamingChoice, ToolCall as ChatToolCall,
    Usage as ChatUsage,
};
use crate::models::response::ResponsesStreamChunk;
use crate::models::response::{
    ContentBlock, FunctionCallOutputFunction, InputText, Item, MessageItem, OutputItem,
    ResponsesRequest, ResponsesResponse, ToolCallOutput, Usage as ResponsesUsage,
};

fn usage_u64_to_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Transform Chat Completions Request → Responses API Request
#[allow(dead_code)]
pub fn transform_chat_to_responses_request(chat_req: &ChatRequest) -> ResponsesRequest {
    // Extract system message as instructions
    let instructions = chat_req
        .messages
        .iter()
        .find(|m| m.role == "system")
        .and_then(|m| m.content.clone());

    // Convert messages, filtering out system (extracted as instructions above)
    let input: Vec<Item> = chat_req
        .messages
        .iter()
        .filter(|m| m.role != "system")
        .flat_map(|msg| {
            let mut items = Vec::new();

            // Handle text content
            if let Some(content) = &msg.content {
                if !content.is_empty() {
                    items.push(Item::Message(MessageItem {
                        id: None,
                        role: msg.role.clone(),
                        content: vec![ContentBlock::InputText(InputText {
                            text: content.clone(),
                        })],
                        end_turn: None,
                        phase: None,
                    }));
                }
            }

            // Handle tool calls as separate items
            if let Some(tool_calls) = &msg.tool_calls {
                for tc in tool_calls {
                    items.push(Item::FunctionCall(
                        crate::models::response::FunctionCallItem {
                            call_id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    ));
                }
            }

            // Handle tool call outputs
            if let Some(tool_call_id) = &msg.tool_call_id {
                if let Some(content) = &msg.content {
                    items.push(Item::FunctionCallOutput(
                        crate::models::response::FunctionCallOutputItem {
                            call_id: tool_call_id.clone(),
                            output: content.clone(),
                        },
                    ));
                }
            }

            items
        })
        .collect();

    let tools = chat_req
        .tools
        .as_ref()
        .map(|tools| {
            tools
                .iter()
                .filter_map(|t| {
                    // Only convert tools that have a function definition
                    t.function.as_ref().map(|f| crate::models::response::Tool {
                        tool_type: t.tool_type.clone(),
                        function: Some(crate::models::response::FunctionDefinition {
                            name: Some(f.name.clone()),
                            description: f.description.clone(),
                            parameters: f.parameters.clone(),
                            strict: Some(true), // Responses API defaults to strict
                        }),
                        // Non-function tool fields default to None
                        vector_store_ids: None,
                        display_width: None,
                        display_height: None,
                        environment: None,
                        server_label: None,
                        server_description: None,
                        server_url: None,
                        require_approval: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    ResponsesRequest {
        model: chat_req.model.clone(),
        input,
        instructions,
        tools,
        tool_choice: chat_req
            .tool_choice
            .as_ref()
            .and_then(|choice| serde_json::to_value(choice).ok()),
        parallel_tool_calls: Some(chat_req.parallel_tool_calls),
        temperature: chat_req.temperature,
        top_p: chat_req.top_p,
        max_tokens: chat_req.max_tokens,
        stream: chat_req.stream,
        include: vec![],
        text: chat_req
            .response_format
            .as_ref()
            .map(|rf| crate::models::response::TextFormat {
                format: Some(match rf {
                    crate::models::chat::ResponseFormat::Text => {
                        crate::models::response::TextFormatType::Text
                    }
                    crate::models::chat::ResponseFormat::JsonObject => {
                        crate::models::response::TextFormatType::JsonObject
                    }
                    crate::models::chat::ResponseFormat::JsonSchema => {
                        crate::models::response::TextFormatType::JsonSchema
                    }
                }),
            }),
        structured_output: None,
        store: None,
        previous_response_id: None,
        metadata: None,
        model_settings: None,
        reasoning: None,
        stop: chat_req.stop.clone(),
        seed: chat_req.seed,
        user: chat_req.user.clone(),
        service_tier: None,
        prompt_cache_key: None,
        namespace: None,
    }
}

/// Transform Responses API Request → Chat Completions Request
pub fn transform_responses_to_chat_request(responses_req: &ResponsesRequest) -> ChatRequest {
    let mut messages = Vec::new();

    // Add instructions as system message
    if let Some(ref instructions) = responses_req.instructions {
        if !instructions.is_empty() {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(instructions.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    // Convert all item types
    for item in &responses_req.input {
        match item {
            Item::Message(msg) => {
                // Preserve the full message payload instead of only the first content block.
                let content = msg
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        ContentBlock::InputText(text) => Some(text.text.clone()),
                        ContentBlock::InputImage(img) => {
                            Some(format!("[Image: {}]", img.image_url))
                        }
                        ContentBlock::OutputText(text) => Some(text.text.clone()),
                        ContentBlock::Refusal(refusal) => Some(refusal.refusal.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                messages.push(ChatMessage {
                    role: msg.role.clone(),
                    content: Some(content),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
            Item::FunctionCall(func) => {
                messages.push(ChatMessage {
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
                });
            }
            Item::FunctionCallOutput(output) => {
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: Some(output.output.clone()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: Some(output.call_id.clone()),
                });
            }
            Item::Reasoning(reasoning) => {
                // Convert reasoning to assistant message
                let content = reasoning
                    .reasoning
                    .clone()
                    .or_else(|| reasoning.summary.iter().find_map(|s| s.text.clone()))
                    .unwrap_or_default();

                if !content.is_empty() {
                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: Some(content),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
            }
            Item::LocalShellCall(shell) => {
                // Convert local shell call to tool call
                let args = serde_json::to_string(&shell.action).unwrap_or_default();
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: None,
                    name: None,
                    tool_calls: Some(vec![crate::models::chat::ToolCall {
                        id: shell.call_id.clone().unwrap_or_default(),
                        call_type: "function".to_string(),
                        function: crate::models::chat::FunctionCall {
                            name: format!("shell_{}", shell.action.action_type),
                            arguments: args,
                        },
                    }]),
                    tool_call_id: None,
                });
            }
            Item::ToolSearchCall(search) => {
                // Convert tool search to function call
                let args = serde_json::to_string(&search.arguments).unwrap_or_default();
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: None,
                    name: None,
                    tool_calls: Some(vec![crate::models::chat::ToolCall {
                        id: search.call_id.clone().unwrap_or_default(),
                        call_type: "function".to_string(),
                        function: crate::models::chat::FunctionCall {
                            name: "tool_search".to_string(),
                            arguments: args,
                        },
                    }]),
                    tool_call_id: None,
                });
            }
            Item::CustomToolCall(custom) => {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: None,
                    name: None,
                    tool_calls: Some(vec![crate::models::chat::ToolCall {
                        id: custom.call_id.clone(),
                        call_type: "function".to_string(),
                        function: crate::models::chat::FunctionCall {
                            name: custom.name.clone(),
                            arguments: custom.input.clone(),
                        },
                    }]),
                    tool_call_id: None,
                });
            }
            Item::CustomToolCallOutput(output) => {
                let content = match &output.output {
                    crate::models::response::FunctionCallOutputPayload::Text(t) => t.clone(),
                    crate::models::response::FunctionCallOutputPayload::ContentItems(items) => {
                        items
                            .iter()
                            .filter_map(|i| match i {
                                crate::models::response::FunctionCallOutputContentItem::Text {
                                    text,
                                } => Some(text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                };
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: Some(content),
                    name: None,
                    tool_calls: None,
                    tool_call_id: Some(output.call_id.clone()),
                });
            }
            Item::WebSearchCall(web_search) => {
                if let Some(action) = &web_search.action {
                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: None,
                        name: None,
                        tool_calls: Some(vec![crate::models::chat::ToolCall {
                            id: web_search.id.clone().unwrap_or_default(),
                            call_type: "function".to_string(),
                            function: crate::models::chat::FunctionCall {
                                name: "web_search".to_string(),
                                arguments: serde_json::to_string(&serde_json::json!({
                                    "query": action.query
                                }))
                                .unwrap_or_default(),
                            },
                        }]),
                        tool_call_id: None,
                    });
                }
            }
            Item::McpToolCallOutput(mcp_output) => {
                let content = mcp_output
                    .output
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        crate::models::response::McpContent::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: Some(content),
                    name: None,
                    tool_calls: None,
                    tool_call_id: Some(mcp_output.call_id.clone()),
                });
            }
            // Skip other types that don't map to Chat API
            _ => {
                tracing::debug!("Skipping item type in Responses → Chat conversion");
            }
        }
    }

    let tools = if responses_req.tools.is_empty() {
        None
    } else {
        Some(
            responses_req
                .tools
                .iter()
                .filter_map(|t| {
                    // Only convert tools that have a function definition
                    t.function.as_ref().map(|f| crate::models::chat::Tool {
                        tool_type: t.tool_type.clone(),
                        function: Some(crate::models::chat::FunctionDefinition {
                            name: f.name.clone().unwrap_or_default(),
                            description: f.description.clone(),
                            parameters: f.parameters.clone(),
                        }),
                    })
                })
                .collect(),
        )
    };

    let response_format = responses_req.text.as_ref().and_then(|t| {
        t.format.as_ref().map(|f| match f {
            crate::models::response::TextFormatType::Text => {
                crate::models::chat::ResponseFormat::Text
            }
            crate::models::response::TextFormatType::JsonObject => {
                crate::models::chat::ResponseFormat::JsonObject
            }
            crate::models::response::TextFormatType::JsonSchema => {
                crate::models::chat::ResponseFormat::JsonSchema
            }
        })
    });

    let tool_choice = responses_req
        .tool_choice
        .as_ref()
        .and_then(|choice| serde_json::from_value(choice.clone()).ok());

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
        tool_choice,
        parallel_tool_calls: responses_req.parallel_tool_calls.unwrap_or(true),
    }
}

/// Transform Chat Completions Response → Responses API Response
pub fn transform_chat_to_responses_response(chat_resp: &ChatResponse) -> ResponsesResponse {
    let output: Vec<OutputItem> = chat_resp
        .choices
        .iter()
        .enumerate()
        .map(|(idx, choice)| {
            let content_text = choice.message.content.clone().unwrap_or_default();
            let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
                calls
                    .iter()
                    .map(|tc| ToolCallOutput {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: FunctionCallOutputFunction {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect()
            });

            OutputItem::Message(crate::models::response::MessageOutput {
                index: idx as u32,
                id: None,
                role: choice.message.role.clone(),
                content: vec![ContentBlock::OutputText(
                    crate::models::response::OutputText {
                        text: content_text,
                        annotations: None,
                    },
                )],
                status: choice.finish_reason.clone(),
                end_turn: None,
                phase: None,
                tool_calls,
            })
        })
        .collect();

    let usage = chat_resp.usage.as_ref().map(|u| ResponsesUsage {
        input_tokens: u64::from(u.prompt_tokens),
        input_tokens_details: None,
        output_tokens: u64::from(u.completion_tokens),
        output_tokens_details: None,
        total_tokens: u64::from(u.total_tokens),
    });

    // Extract text from output for backwards compatibility
    let text = output.iter().find_map(|item| {
        if let OutputItem::Message(msg) = item {
            msg.content.iter().find_map(|c| {
                if let ContentBlock::OutputText(text) = c {
                    Some(text.text.clone())
                } else {
                    None
                }
            })
        } else {
            None
        }
    });

    ResponsesResponse {
        id: chat_resp.id.clone(),
        object: "response".to_string(),
        created: chat_resp.created,
        model: chat_resp.model.clone(),
        output,
        text,
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
        .filter_map(|item| match item {
            OutputItem::Message(msg) => {
                let content = msg.content.iter().find_map(|c| match c {
                    ContentBlock::OutputText(text) => Some(text.text.clone()),
                    _ => None,
                });

                let tool_calls = msg.tool_calls.as_ref().map(|calls| {
                    calls
                        .iter()
                        .map(|tc| crate::models::chat::ToolCall {
                            id: tc.id.clone(),
                            call_type: tc.call_type.clone(),
                            function: crate::models::chat::FunctionCall {
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            },
                        })
                        .collect()
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
        })
        .collect();

    let usage = responses_resp.usage.as_ref().map(|u| ChatUsage {
        prompt_tokens: usage_u64_to_u32(u.input_tokens),
        completion_tokens: usage_u64_to_u32(u.output_tokens),
        total_tokens: usage_u64_to_u32(u.total_tokens),
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
    let output: Vec<OutputItem> = chat_chunk
        .choices
        .iter()
        .filter_map(|choice| {
            let delta = choice.delta.as_ref()?;
            let tool_calls = delta.tool_calls.as_ref().map(|calls| {
                calls
                    .iter()
                    .map(|tc| ToolCallOutput {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: FunctionCallOutputFunction {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect::<Vec<_>>()
            });
            let content = delta
                .content
                .as_ref()
                .map(|text| {
                    vec![ContentBlock::OutputText(
                        crate::models::response::OutputText {
                            text: text.clone(),
                            annotations: None,
                        },
                    )]
                })
                .unwrap_or_default();

            let has_tool_calls = tool_calls
                .as_ref()
                .map(|calls| !calls.is_empty())
                .unwrap_or(false);

            // Skip metadata-only deltas such as role announcements that carry
            // no text, tool calls, finish reason, or usage payload.
            if content.is_empty() && !has_tool_calls && choice.finish_reason.is_none() {
                return None;
            }

            Some(OutputItem::Message(
                crate::models::response::MessageOutput {
                    index: choice.index,
                    id: None,
                    role: delta
                        .role
                        .clone()
                        .unwrap_or_else(|| "assistant".to_string()),
                    content,
                    status: choice.finish_reason.clone(),
                    end_turn: None,
                    phase: None,
                    tool_calls,
                },
            ))
        })
        .collect();

    let usage = chat_chunk.usage.as_ref().map(|u| ResponsesUsage {
        input_tokens: u64::from(u.prompt_tokens),
        input_tokens_details: None,
        output_tokens: u64::from(u.completion_tokens),
        output_tokens_details: None,
        total_tokens: u64::from(u.total_tokens),
    });

    // Extract text from output for backwards compatibility
    let text = output.iter().find_map(|item| {
        if let OutputItem::Message(msg) = item {
            msg.content.iter().find_map(|c| {
                if let ContentBlock::OutputText(text) = c {
                    Some(text.text.clone())
                } else {
                    None
                }
            })
        } else {
            None
        }
    });

    ResponsesStreamChunk {
        id: chat_chunk.id.clone(),
        object: "response.chunk".to_string(),
        created: chat_chunk.created,
        model: chat_chunk.model.clone(),
        output,
        text,
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
        .filter_map(|item| match item {
            OutputItem::Message(msg) => {
                let content = msg.content.iter().find_map(|block| match block {
                    ContentBlock::OutputText(text) => Some(text.text.clone()),
                    _ => None,
                });

                Some(ChatStreamingChoice {
                    index: msg.index,
                    delta: Some(ChatDelta {
                        role: Some(msg.role.clone()),
                        content,
                        tool_calls: msg.tool_calls.as_ref().map(|calls| {
                            calls
                                .iter()
                                .map(|tc| ChatToolCall {
                                    id: tc.id.clone(),
                                    call_type: tc.call_type.clone(),
                                    function: crate::models::chat::FunctionCall {
                                        name: tc.function.name.clone(),
                                        arguments: tc.function.arguments.clone(),
                                    },
                                })
                                .collect()
                        }),
                        reasoning_content: None,
                        reasoning_summary_text: None,
                    }),
                    finish_reason: msg.status.clone(),
                    logprobs: None,
                })
            }
            _ => None,
        })
        .collect();

    // Convert usage from response format to chat format
    let usage = responses_chunk.usage.as_ref().map(|u| ChatUsage {
        prompt_tokens: usage_u64_to_u32(u.input_tokens),
        completion_tokens: usage_u64_to_u32(u.output_tokens),
        total_tokens: usage_u64_to_u32(u.total_tokens),
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
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
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
                id: None,
                role: "user".to_string(),
                content: vec![ContentBlock::InputText(InputText {
                    text: "Hello".to_string(),
                })],
                end_turn: None,
                phase: None,
            })],
            instructions: None,
            tools: vec![],
            tool_choice: None,
            parallel_tool_calls: None,
            temperature: Some(0.7),
            top_p: None,
            max_tokens: Some(1000),
            stream: None,
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
        };

        let chat_req = transform_responses_to_chat_request(&responses_req);

        assert_eq!(chat_req.model, "gpt-4");
        assert_eq!(chat_req.messages.len(), 1);
        assert_eq!(chat_req.messages[0].content.as_ref().unwrap(), "Hello");
    }

    #[test]
    fn test_transform_responses_to_chat_request_merges_multiple_content_blocks() {
        let responses_req = ResponsesRequest {
            model: "gpt-4".to_string(),
            input: vec![Item::Message(MessageItem {
                id: None,
                role: "user".to_string(),
                content: vec![
                    ContentBlock::InputText(InputText {
                        text: "Hello".to_string(),
                    }),
                    ContentBlock::InputText(InputText {
                        text: "world".to_string(),
                    }),
                    ContentBlock::Refusal(crate::models::response::RefusalContent {
                        refusal: "ignored".to_string(),
                    }),
                ],
                end_turn: None,
                phase: None,
            })],
            instructions: None,
            tools: vec![],
            tool_choice: None,
            parallel_tool_calls: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: None,
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
        };

        let chat_req = transform_responses_to_chat_request(&responses_req);

        assert_eq!(chat_req.messages.len(), 1);
        assert_eq!(
            chat_req.messages[0].content.as_deref(),
            Some("Hello\nworld\nignored")
        );
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
            output: vec![OutputItem::Message(
                crate::models::response::MessageOutput {
                    index: 0,
                    id: None,
                    role: "assistant".to_string(),
                    content: vec![ContentBlock::OutputText(
                        crate::models::response::OutputText {
                            text: "Hello!".to_string(),
                            annotations: None,
                        },
                    )],
                    status: Some("completed".to_string()),
                    end_turn: None,
                    phase: None,
                    tool_calls: None,
                },
            )],
            text: Some("Hello!".to_string()),
            usage: Some(ResponsesUsage {
                input_tokens: 10,
                input_tokens_details: None,
                output_tokens: 5,
                output_tokens_details: None,
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

    #[test]
    fn test_transform_chat_stream_to_responses_stream() {
        let chat_chunk = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4o".to_string(),
            choices: vec![ChatStreamingChoice {
                index: 0,
                delta: Some(ChatDelta {
                    role: Some("assistant".to_string()),
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                        reasoning_content: None,
                        reasoning_summary_text: None,}),
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        let responses_chunk = transform_chat_stream_to_responses_stream(&chat_chunk);

        assert_eq!(responses_chunk.id, "chatcmpl-123");
        assert_eq!(responses_chunk.model, "gpt-4o");
        assert_eq!(responses_chunk.output.len(), 1);
        assert_eq!(responses_chunk.usage.as_ref().unwrap().total_tokens, 15);

        match &responses_chunk.output[0] {
            OutputItem::Message(message) => {
                assert_eq!(message.index, 0);
                assert_eq!(message.role, "assistant");
                let content = message
                    .content
                    .iter()
                    .find_map(|block| match block {
                        ContentBlock::OutputText(text) => Some(text.text.as_str()),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(content, "Hello");
            }
            other => panic!("expected message output item, got {other:?}"),
        }
    }

    #[test]
    fn test_transform_chat_stream_skips_empty_metadata_only_chunk() {
        let chat_chunk = ChatCompletionChunk {
            id: "chatcmpl-empty".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![ChatStreamingChoice {
                index: 0,
                delta: Some(ChatDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: None,
                        reasoning_content: None,
                        reasoning_summary_text: None,}),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };

        let responses_chunk = transform_chat_stream_to_responses_stream(&chat_chunk);

        assert!(
            responses_chunk.output.is_empty(),
            "metadata-only chunk should not emit an empty responses message"
        );
    }

    /// Round-trip test: Chat → Responses → Chat preserves data integrity
    #[test]
    fn test_round_trip_chat_to_responses_to_chat_request() {
        let original_chat_req = ChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: Some("You are helpful".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: Some("Hello".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(1000),
            stream: Some(false),
            stop: Some(vec!["STOP".to_string()]),
            n: 1,
            stream_options: None,
            include_usage: Some(true),
            response_format: None,
            seed: Some(42),
            organization: None,
            presence_penalty: Some(0.1),
            frequency_penalty: Some(0.2),
            logit_bias: None,
            user: Some("test-user".to_string()),
            tools: None,
            tool_choice: None,
            parallel_tool_calls: true,
        };

        // Transform Chat → Responses
        let responses_req = transform_chat_to_responses_request(&original_chat_req);

        // Transform Responses → Chat
        let round_trip_chat_req = transform_responses_to_chat_request(&responses_req);

        // Verify critical fields are preserved
        assert_eq!(round_trip_chat_req.model, original_chat_req.model);
        assert_eq!(
            round_trip_chat_req.messages.len(),
            original_chat_req.messages.len()
        );
        assert_eq!(
            round_trip_chat_req.temperature,
            original_chat_req.temperature
        );
        assert_eq!(round_trip_chat_req.top_p, original_chat_req.top_p);
        assert_eq!(round_trip_chat_req.max_tokens, original_chat_req.max_tokens);
        assert_eq!(round_trip_chat_req.stream, original_chat_req.stream);
        assert_eq!(round_trip_chat_req.seed, original_chat_req.seed);
        assert_eq!(round_trip_chat_req.user, original_chat_req.user);

        // Verify message content is preserved
        for (i, msg) in round_trip_chat_req.messages.iter().enumerate() {
            assert_eq!(msg.role, original_chat_req.messages[i].role);
            assert_eq!(msg.content, original_chat_req.messages[i].content);
        }
    }

    /// Round-trip test: Chat Response → Responses Response → Chat Response preserves data
    #[test]
    fn test_round_trip_chat_to_responses_to_chat_response() {
        let original_chat_resp = ChatResponse {
            id: "chatcmpl-test123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4o".to_string(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello! How can I help?".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 15,
                completion_tokens: 10,
                total_tokens: 25,
            }),
            service_tier: None,
            finish_reason: Some("stop".to_string()),
            extra: std::collections::HashMap::new(),
        };

        // Transform Chat Response → Responses Response
        let responses_resp = transform_chat_to_responses_response(&original_chat_resp);

        // Transform Responses Response → Chat Response
        let round_trip_chat_resp = transform_responses_to_chat_response(&responses_resp);

        // Verify critical fields are preserved
        assert_eq!(round_trip_chat_resp.id, original_chat_resp.id);
        assert_eq!(round_trip_chat_resp.model, original_chat_resp.model);
        assert_eq!(round_trip_chat_resp.created, original_chat_resp.created);
        assert_eq!(
            round_trip_chat_resp.choices.len(),
            original_chat_resp.choices.len()
        );

        // Verify usage is preserved
        assert!(round_trip_chat_resp.usage.is_some());
        let usage = round_trip_chat_resp.usage.unwrap();
        let original_usage = original_chat_resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, original_usage.prompt_tokens);
        assert_eq!(usage.completion_tokens, original_usage.completion_tokens);
        assert_eq!(usage.total_tokens, original_usage.total_tokens);

        // Verify choice content is preserved
        assert_eq!(
            round_trip_chat_resp.choices[0].message.content,
            original_chat_resp.choices[0].message.content
        );
        assert_eq!(
            round_trip_chat_resp.choices[0].message.role,
            original_chat_resp.choices[0].message.role
        );
    }

    /// Test: Deserializing non-function tools
    #[test]
    fn test_deserialize_web_search_tool() {
        let json = r#"{"type": "web_search"}"#;
        let tool: crate::models::response::Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.tool_type, "web_search");
        // Function fields should all be None for non-function tools
        if let Some(ref func) = tool.function {
            assert!(func.name.is_none());
            assert!(func.description.is_none());
            assert!(func.parameters.is_none());
            assert!(func.strict.is_none());
        }
        assert!(tool.vector_store_ids.is_none());
        assert!(tool.display_width.is_none());
    }

    #[test]
    fn test_deserialize_file_search_tool() {
        let json = r#"{
            "type": "file_search",
            "vector_store_ids": ["vs_abc123", "vs_def456"]
        }"#;
        let tool: crate::models::response::Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.tool_type, "file_search");
        // Function fields should all be None for non-function tools
        if let Some(ref func) = tool.function {
            assert!(func.name.is_none());
            assert!(func.description.is_none());
            assert!(func.parameters.is_none());
            assert!(func.strict.is_none());
        }
        assert_eq!(
            tool.vector_store_ids,
            Some(vec!["vs_abc123".to_string(), "vs_def456".to_string()])
        );
    }

    #[test]
    fn test_deserialize_computer_use_tool() {
        let json = r#"{
            "type": "computer_use",
            "display_width": 1024,
            "display_height": 768,
            "environment": "mac"
        }"#;
        let tool: crate::models::response::Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.tool_type, "computer_use");
        // Function fields should all be None for non-function tools
        if let Some(ref func) = tool.function {
            assert!(func.name.is_none());
            assert!(func.description.is_none());
            assert!(func.parameters.is_none());
            assert!(func.strict.is_none());
        }
        assert_eq!(tool.display_width, Some(1024));
        assert_eq!(tool.display_height, Some(768));
        assert_eq!(tool.environment, Some("mac".to_string()));
    }

    #[test]
    fn test_deserialize_mcp_tool() {
        let json = r#"{
            "type": "mcp",
            "server_label": "dmcp",
            "server_description": "Dice rolling server",
            "server_url": "https://example.com/sse",
            "require_approval": "never"
        }"#;
        let tool: crate::models::response::Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.tool_type, "mcp");
        // Function fields should all be None for non-function tools
        if let Some(ref func) = tool.function {
            assert!(func.name.is_none());
            assert!(func.description.is_none());
            assert!(func.parameters.is_none());
            assert!(func.strict.is_none());
        }
        assert_eq!(tool.server_label, Some("dmcp".to_string()));
        assert_eq!(
            tool.server_description,
            Some("Dice rolling server".to_string())
        );
        assert_eq!(tool.server_url, Some("https://example.com/sse".to_string()));
        assert_eq!(tool.require_approval, Some("never".to_string()));
    }

    #[test]
    fn test_serialize_web_search_tool() {
        let tool = crate::models::response::Tool {
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
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert_eq!(json, r#"{"type":"web_search"}"#);
    }

    #[test]
    fn test_serialize_function_tool() {
        // Test that function tool serializes with internally-tagged format
        let tool = crate::models::response::Tool {
            tool_type: "function".to_string(),
            function: Some(crate::models::response::FunctionDefinition {
                name: Some("get_weather".to_string()),
                description: Some("Get weather".to_string()),
                parameters: Some(serde_json::json!({"type": "object"})),
                strict: Some(true),
            }),
            vector_store_ids: None,
            display_width: None,
            display_height: None,
            environment: None,
            server_label: None,
            server_description: None,
            server_url: None,
            require_approval: None,
        };
        let json = serde_json::to_string(&tool).unwrap();
        // Should serialize with fields at top level (internally-tagged)
        assert!(json.contains(r#""type":"function""#));
        assert!(json.contains(r#""name":"get_weather""#));
        assert!(json.contains(r#""description":"Get weather""#));
        assert!(json.contains(r#""strict":true"#));
        // Should NOT contain null values
        assert!(!json.contains(":null"));
    }

    #[test]
    fn test_responses_request_with_mixed_tools() {
        let json = r#"{
            "model": "gpt-4",
            "tools": [
                {"type": "web_search"},
                {"type": "file_search", "vector_store_ids": ["vs_123"]},
                {"type": "computer_use", "display_width": 1024, "display_height": 768, "environment": "mac"},
                {"type": "mcp", "server_label": "custom", "server_url": "https://example.com/sse", "require_approval": "never"},
                {"type": "function", "name": "get_weather", "description": "Get weather", "parameters": {"type": "object"}, "strict": true}
            ],
            "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "Hello"}]}
            ]
        }"#;

        let req: crate::models::response::ResponsesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.tools.len(), 5);

        // Verify tool types
        assert_eq!(req.tools[0].tool_type, "web_search");
        assert_eq!(req.tools[1].tool_type, "file_search");
        assert_eq!(req.tools[2].tool_type, "computer_use");
        assert_eq!(req.tools[3].tool_type, "mcp");
        assert_eq!(req.tools[4].tool_type, "function");

        // Verify non-function tools have no function fields (or all fields are None)
        for i in 0..4 {
            if let Some(ref func) = req.tools[i].function {
                assert!(func.name.is_none());
                assert!(func.description.is_none());
                assert!(func.parameters.is_none());
                assert!(func.strict.is_none());
            }
        }

        // Verify function tool has function fields
        assert!(req.tools[4].function.is_some());
        let func = req.tools[4].function.as_ref().unwrap();
        assert_eq!(func.name, Some("get_weather".to_string()));
        assert_eq!(func.description, Some("Get weather".to_string()));
        assert!(func.parameters.is_some());
        assert_eq!(func.strict, Some(true));
    }
}
