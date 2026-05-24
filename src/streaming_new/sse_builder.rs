//! SSE Event Builder for Responses API streaming
//! 
//! This module builds Server-Sent Events (SSE) for the Responses API.
//! Reference: https://platform.openai.com/docs/api-reference/responses/create#responses-streaming

use super::streaming_state::StreamingState;
use crate::models::response::{ContentBlock, OutputItem, OutputText};

/// SSE Event types for Responses API streaming
#[derive(Debug, Clone)]
pub enum SseEvent {
    /// Event: response.created
    ResponseCreated {
        response_id: String,
        model: String,
    },
    /// Event: response.output_text.delta
    TextDelta {
        content: String,
    },
    /// Event: response.output_text.done
    TextDone {
        content: String,
    },
    /// Event: response.function_call_arguments.delta
    FunctionCallDelta {
        call_id: String,
        arguments: String,
    },
    /// Event: response.function_call_arguments.done
    FunctionCallDone {
        call_id: String,
        name: String,
        arguments: String,
    },
    /// Event: response.done
    ResponseDone {
        response_id: String,
    },
    /// Raw SSE data string
    Raw {
        event: String,
        data: String,
    },
}

impl SseEvent {
    /// Convert event to SSE format string
    pub fn to_sse_string(&self) -> String {
        match self {
            SseEvent::ResponseCreated { response_id, model } => {
                format!(
                    "event: response.created\ndata: {{\"response_id\":\"{}\",\"model\":\"{}\",\"status\":\"in_progress\"}}\n\n",
                    response_id, model
                )
            }
            SseEvent::TextDelta { content } => {
                // Escape newlines and special characters for SSE
                let escaped = content
                    .replace('\\', "\\\\")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r");
                format!(
                    "event: response.output_text.delta\ndata: {{\"content\":\"{}\"}}\n\n",
                    escaped
                )
            }
            SseEvent::TextDone { content } => {
                let escaped = content
                    .replace('\\', "\\\\")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r");
                format!(
                    "event: response.output_text.done\ndata: {{\"content\":\"{}\"}}\n\n",
                    escaped
                )
            }
            SseEvent::FunctionCallDelta { call_id, arguments } => {
                let escaped_args = arguments
                    .replace('\\', "\\\\")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r");
                format!(
                    "event: response.function_call_arguments.delta\ndata: {{\"call_id\":\"{}\",\"arguments\":\"{}\"}}\n\n",
                    call_id, escaped_args
                )
            }
            SseEvent::FunctionCallDone { call_id, name, arguments } => {
                let escaped_args = arguments
                    .replace('\\', "\\\\")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r");
                format!(
                    "event: response.function_call_arguments.done\ndata: {{\"call_id\":\"{}\",\"name\":\"{}\",\"arguments\":\"{}\"}}\n\n",
                    call_id, name, escaped_args
                )
            }
            SseEvent::ResponseDone { response_id } => {
                format!(
                    "event: done\ndata: {{\"response_id\":\"{}\"}}\n\n",
                    response_id
                )
            }
            SseEvent::Raw { event, data } => {
                format!("event: {}\ndata: {}\n\n", event, data)
            }
        }
    }
}

/// SSE Event builder for converting streaming state to events
#[derive(Debug, Clone)]
pub struct SseEventBuilder {
    /// Current state
    state: StreamingState,
    /// Previous text buffer (for delta calculation)
    prev_text_len: usize,
    /// Previous tool call arguments (for delta calculation)
    prev_tool_args: HashMap<String, usize>,
}

use std::collections::HashMap;

impl SseEventBuilder {
    /// Create a new SSE event builder
    pub fn new(response_id: String, model: String) -> Self {
        Self {
            state: StreamingState::new(response_id, model),
            prev_text_len: 0,
            prev_tool_args: HashMap::new(),
        }
    }

    /// Process a chunk and generate SSE events
    pub fn process_chunk(&mut self, chunk: &crate::models::chat::ChatCompletionChunk) -> Vec<SseEvent> {
        let mut events = Vec::new();
        
        // Emit response.created on first chunk
        if !self.state.started {
            events.push(SseEvent::ResponseCreated {
                response_id: self.state.response_id.clone(),
                model: self.state.model.clone(),
            });
        }
        
        // Update state
        self.state.process_chunk(chunk);
        
        // Generate text delta if there's new text
        if self.state.text_buffer.len() > self.prev_text_len {
            let delta = &self.state.text_buffer[self.prev_text_len..];
            if !delta.is_empty() {
                events.push(SseEvent::TextDelta {
                    content: delta.to_string(),
                });
            }
            self.prev_text_len = self.state.text_buffer.len();
        }
        
        // Generate tool call deltas
        for call_id in &self.state.tool_call_order {
            if let Some(tc) = self.state.tool_calls.get(call_id) {
                let prev_len = *self.prev_tool_args.get(call_id).unwrap_or(&0);
                if tc.arguments.len() > prev_len {
                    let delta = &tc.arguments[prev_len..];
                    if !delta.is_empty() {
                        events.push(SseEvent::FunctionCallDelta {
                            call_id: tc.call_id.clone(),
                            arguments: delta.to_string(),
                        });
                    }
                    self.prev_tool_args.insert(call_id.clone(), tc.arguments.len());
                }
            }
        }
        
        events
    }

    /// Generate done events for the response
    pub fn generate_done_events(&self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        
        // Text done if we have text
        if !self.state.text_buffer.is_empty() {
            events.push(SseEvent::TextDone {
                content: self.state.text_buffer.clone(),
            });
        }
        
        // Function call done for each tool call
        for call_id in &self.state.tool_call_order {
            if let Some(tc) = self.state.tool_calls.get(call_id) {
                events.push(SseEvent::FunctionCallDone {
                    call_id: tc.call_id.clone(),
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                });
            }
        }
        
        // Response done
        events.push(SseEvent::ResponseDone {
            response_id: self.state.response_id.clone(),
        });
        
        events
    }

    /// Get the accumulated output items
    pub fn to_output_items(&self) -> Vec<OutputItem> {
        let mut items = Vec::new();
        
        // Text content
        if !self.state.text_buffer.is_empty() {
            items.push(OutputItem::Message(crate::models::response::MessageOutput {
                index: items.len() as u32,
                id: Some(format!("msg_{}", uuid::Uuid::new_v4())),
                role: "assistant".to_string(),
                status: Some("completed".to_string()),
                content: vec![ContentBlock::OutputText(OutputText {
                    text: self.state.text_buffer.clone(),
                    annotations: None,
                })],
                end_turn: None,
                phase: None,
                tool_calls: None,
            }));
        }
        
        // Tool calls
        for call_id in &self.state.tool_call_order {
            if let Some(tc) = self.state.tool_calls.get(call_id) {
                items.push(OutputItem::FunctionCall(
                    crate::models::response::FunctionCallOutput {
                        index: items.len() as u32,
                        id: Some(tc.call_id.clone()),
                        call_id: tc.call_id.clone(),
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                        status: Some("completed".to_string()),
                    },
                ));
            }
        }
        
        items
    }

    /// Get reference to the internal state
    pub fn state(&self) -> &StreamingState {
        &self.state
    }

    /// Get mutable reference to the internal state
    pub fn state_mut(&mut self) -> &mut StreamingState {
        &mut self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::{StreamingChoice, Delta, ToolCall, FunctionCall};

    #[test]
    fn test_sse_event_created() {
        let event = SseEvent::ResponseCreated {
            response_id: "resp_123".to_string(),
            model: "gpt-4".to_string(),
        };
        
        let sse = event.to_sse_string();
        assert!(sse.contains("event: response.created"));
        assert!(sse.contains("resp_123"));
        assert!(sse.contains("gpt-4"));
    }

    #[test]
    fn test_sse_text_delta() {
        let event = SseEvent::TextDelta {
            content: "Hello".to_string(),
        };
        
        let sse = event.to_sse_string();
        assert!(sse.contains("event: response.output_text.delta"));
        assert!(sse.contains("Hello"));
    }

    #[test]
    fn test_sse_text_delta_escape() {
        let event = SseEvent::TextDelta {
            content: "Hello\nWorld".to_string(),
        };
        
        let sse = event.to_sse_string();
        assert!(sse.contains("\\n"));
    }

    #[test]
    fn test_sse_builder_process() {
        use crate::models::chat::ChatCompletionChunk;
        
        let mut builder = SseEventBuilder::new("resp_123".to_string(), "gpt-4".to_string());
        
        let chunk = ChatCompletionChunk {
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
                }),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };
        
        let events = builder.process_chunk(&chunk);
        
        // Should have created event and text delta
        assert!(events.iter().any(|e| matches!(e, SseEvent::ResponseCreated { .. })));
        assert!(events.iter().any(|e| matches!(e, SseEvent::TextDelta { content } if content == "Hello")));
    }

    #[test]
    fn test_sse_builder_tool_calls() {
        use crate::models::chat::ChatCompletionChunk;
        
        let mut builder = SseEventBuilder::new("resp_456".to_string(), "gpt-4".to_string());
        
        let chunk = ChatCompletionChunk {
            id: "chunk-1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: Some(Delta {
                    role: None,
                    content: None,
                    tool_calls: Some(vec![
                        ToolCall {
                            id: "call_123".to_string(),
                            call_type: "function".to_string(),
                            function: FunctionCall {
                                name: "get_weather".to_string(),
                                arguments: r#"{"city":"#.to_string(),
                            },
                        },
                    ]),
                }),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };
        
        let events = builder.process_chunk(&chunk);
        
        assert!(events.iter().any(|e| matches!(
            e, 
            SseEvent::FunctionCallDelta { call_id, arguments } 
            if call_id == "call_123" && arguments == r#"{"city":"#
        )));
    }
}
