//! SSE Event Builder for Responses API streaming
//! 
//! This module builds Server-Sent Events (SSE) for the Responses API.
//! Reference: https://platform.openai.com/docs/api-reference/responses/create#responses-streaming
//! 
//! Key requirements from mimo2codex:
//! - Each SSE event MUST include `type` in the JSON payload
//! - `sequence_number` must be included in all events
//! - Proper event ordering and state management

use super::streaming_state::StreamingState;
use crate::models::response::{ContentBlock, OutputItem, OutputText};

/// SSE Event types for Responses API streaming
#[derive(Debug, Clone)]
pub enum SseEvent {
    /// Event: response.created
    ResponseCreated {
        response_id: String,
        model: String,
        status: String,
    },
    /// Event: response.output_item.added
    OutputItemAdded {
        output_index: u32,
        item_id: String,
        item_type: String,
    },
    /// Event: response.reasoning_summary_text.delta
    ReasoningDelta {
        content: String,
    },
    /// Event: response.output_text.delta
    TextDelta {
        content: String,
    },
    /// Event: response.output_text.done
    TextDone {
        content: String,
    },
    /// Event: response.output_text.annotation.added
    AnnotationAdded {
        annotation_type: String,
        text: String,
    },
    /// Event: response.function_call.id.delta
    FunctionCallIdDelta {
        call_id: String,
        name: String,
    },
    /// Event: response.function_call.arguments.delta
    FunctionCallDelta {
        call_id: String,
        arguments: String,
    },
    /// Event: response.function_call.done
    FunctionCallDone {
        call_id: String,
        name: String,
        arguments: String,
    },
    /// Event: response.done
    ResponseDone {
        response_id: String,
        status: String,
    },
    /// Raw SSE data string (for custom events)
    Raw {
        event: String,
        data: String,
    },
}

impl SseEvent {
    /// Convert event to SSE format string
    /// 
    /// Note: According to mimo2codex, each event MUST include `type` in the JSON payload
    /// because Codex client parses events from the data field, not the SSE event header.
    /// Missing `type` leads to "stream disconnected before completion" errors.
    pub fn to_sse_string(&self) -> String {
        match self {
            SseEvent::ResponseCreated { response_id, model, status } => {
                format!(
                    "event: response.created\ndata: {{\"type\":\"response.created\",\"response_id\":\"{}\",\"model\":\"{}\",\"status\":\"{}\"}}\n\n",
                    response_id, model, status
                )
            }
            SseEvent::OutputItemAdded { output_index, item_id, item_type } => {
                format!(
                    "event: response.output_item.added\ndata: {{\"type\":\"response.output_item.added\",\"output_index\":{},\"item\":{{\"id\":\"{}\",\"type\":\"{}\"}}}}\n\n",
                    output_index, item_id, item_type
                )
            }
            SseEvent::ReasoningDelta { content } => {
                let escaped = Self::escape_json(content);
                format!(
                    "event: response.reasoning_summary_text.delta\ndata: {{\"type\":\"response.reasoning_summary_text.delta\",\"content\":\"{}\"}}\n\n",
                    escaped
                )
            }
            SseEvent::TextDelta { content } => {
                let escaped = Self::escape_json(content);
                format!(
                    "event: response.output_text.delta\ndata: {{\"type\":\"response.output_text.delta\",\"content\":\"{}\"}}\n\n",
                    escaped
                )
            }
            SseEvent::TextDone { content } => {
                let escaped = Self::escape_json(content);
                format!(
                    "event: response.output_text.done\ndata: {{\"type\":\"response.output_text.done\",\"content\":\"{}\"}}\n\n",
                    escaped
                )
            }
            SseEvent::AnnotationAdded { annotation_type, text } => {
                let escaped_text = Self::escape_json(text);
                format!(
                    "event: response.output_text.annotation.added\ndata: {{\"type\":\"response.output_text.annotation.added\",\"annotation\":{{\"type\":\"{}\",\"text\":\"{}\"}}}}\n\n",
                    annotation_type, escaped_text
                )
            }
            SseEvent::FunctionCallIdDelta { call_id, name } => {
                format!(
                    "event: response.function_call.id.delta\ndata: {{\"type\":\"response.function_call.id.delta\",\"call_id\":\"{}\",\"name\":\"{}\"}}\n\n",
                    call_id, name
                )
            }
            SseEvent::FunctionCallDelta { call_id, arguments } => {
                let escaped_args = Self::escape_json(arguments);
                format!(
                    "event: response.function_call.arguments.delta\ndata: {{\"type\":\"response.function_call.arguments.delta\",\"call_id\":\"{}\",\"arguments\":\"{}\"}}\n\n",
                    call_id, escaped_args
                )
            }
            SseEvent::FunctionCallDone { call_id, name, arguments } => {
                let escaped_args = Self::escape_json(arguments);
                format!(
                    "event: response.function_call.done\ndata: {{\"type\":\"response.function_call.done\",\"call_id\":\"{}\",\"name\":\"{}\",\"arguments\":\"{}\"}}\n\n",
                    call_id, name, escaped_args
                )
            }
            SseEvent::ResponseDone { response_id, status } => {
                format!(
                    "event: response.done\ndata: {{\"type\":\"response.done\",\"response_id\":\"{}\",\"status\":\"{}\"}}\n\n",
                    response_id, status
                )
            }
            SseEvent::Raw { event, data } => {
                format!("event: {}\ndata: {}\n\n", event, data)
            }
        }
    }
    
    /// Escape special characters for JSON string values
    fn escape_json(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
            .replace('\u{08}', "\\b")  // backspace
            .replace('\u{0C}', "\\f")   // form feed
    }
    
    /// Get the SSE event type name
    pub fn event_type(&self) -> String {
        match self {
            SseEvent::ResponseCreated { .. } => "response.created".to_string(),
            SseEvent::OutputItemAdded { .. } => "response.output_item.added".to_string(),
            SseEvent::ReasoningDelta { .. } => "response.reasoning_summary_text.delta".to_string(),
            SseEvent::TextDelta { .. } => "response.output_text.delta".to_string(),
            SseEvent::TextDone { .. } => "response.output_text.done".to_string(),
            SseEvent::AnnotationAdded { .. } => "response.output_text.annotation.added".to_string(),
            SseEvent::FunctionCallIdDelta { .. } => "response.function_call.id.delta".to_string(),
            SseEvent::FunctionCallDelta { .. } => "response.function_call.arguments.delta".to_string(),
            SseEvent::FunctionCallDone { .. } => "response.function_call.done".to_string(),
            SseEvent::ResponseDone { .. } => "response.done".to_string(),
            SseEvent::Raw { event, .. } => event.clone(),
        }
    }
}

/// SSE Event builder for converting streaming state to events
#[derive(Debug, Clone)]
pub struct SseEventBuilder {
    /// Current state
    state: StreamingState,
    /// Previous text buffer length (for delta calculation)
    prev_text_len: usize,
    /// Previous tool call arguments length (for delta calculation)
    prev_tool_args: std::collections::HashMap<String, usize>,
    /// Sequence number for ordering events
    sequence_number: u32,
    /// Whether reasoning mode is enabled
    enable_reasoning: bool,
    /// Active output index for current item
    active_output_index: u32,
}

impl SseEventBuilder {
    /// Create a new SSE event builder
    pub fn new(response_id: String, model: String) -> Self {
        Self {
            state: StreamingState::new(response_id, model),
            prev_text_len: 0,
            prev_tool_args: std::collections::HashMap::new(),
            sequence_number: 0,
            enable_reasoning: false,
            active_output_index: 0,
        }
    }
    
    /// Enable reasoning mode
    pub fn with_reasoning(mut self, enable: bool) -> Self {
        self.enable_reasoning = enable;
        self
    }
    
    /// Get next sequence number and increment
    fn next_seq(&mut self) -> u32 {
        let seq = self.sequence_number;
        self.sequence_number += 1;
        seq
    }
    
    /// Process a chunk and generate SSE events
    pub fn process_chunk(&mut self, chunk: &crate::models::chat::ChatCompletionChunk) -> Vec<SseEvent> {
        let mut events = Vec::new();
        
        // Emit response.created on first chunk
        if !self.state.started {
            events.push(SseEvent::ResponseCreated {
                response_id: self.state.response_id.clone(),
                model: self.state.model.clone(),
                status: "in_progress".to_string(),
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
        for (call_id, tc_state) in &self.state.tool_calls {
            let prev_len = self.prev_tool_args.get(call_id).copied().unwrap_or(0);
            if tc_state.arguments.len() > prev_len {
                let delta = &tc_state.arguments[prev_len..];
                if !delta.is_empty() {
                    events.push(SseEvent::FunctionCallDelta {
                        call_id: call_id.clone(),
                        arguments: delta.to_string(),
                    });
                }
                self.prev_tool_args.insert(call_id.clone(), tc_state.arguments.len());
            }
        }
        
        events
    }
    
    /// Generate done events for finalization
    pub fn generate_done_events(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        
        // Emit text done if there's accumulated text
        if !self.state.text_buffer.is_empty() {
            events.push(SseEvent::TextDone {
                content: self.state.text_buffer.clone(),
            });
        }
        
        // Emit function call done for each tool call
        for (_call_id, tc_state) in &self.state.tool_calls {
            events.push(SseEvent::FunctionCallDone {
                call_id: tc_state.call_id.clone(),
                name: tc_state.name.clone(),
                arguments: tc_state.arguments.clone(),
            });
        }
        
        // Emit final response.done
        events.push(SseEvent::ResponseDone {
            response_id: self.state.response_id.clone(),
            status: "completed".to_string(),
        });
        
        events
    }
    
    /// Get the current state
    pub fn state(&self) -> &StreamingState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::{StreamingChoice, Delta};

    #[test]
    fn test_sse_event_created() {
        let event = SseEvent::ResponseCreated {
            response_id: "resp_123".to_string(),
            model: "gpt-4".to_string(),
            status: "in_progress".to_string(),
        };
        
        let sse = event.to_sse_string();
        assert!(sse.contains("event: response.created"));
        assert!(sse.contains("\"type\":\"response.created\""));
        assert!(sse.contains("\"response_id\":\"resp_123\""));
    }

    #[test]
    fn test_sse_text_delta() {
        let event = SseEvent::TextDelta {
            content: "Hello, world!".to_string(),
        };
        
        let sse = event.to_sse_string();
        assert!(sse.contains("event: response.output_text.delta"));
        assert!(sse.contains("\"type\":\"response.output_text.delta\""));
        assert!(sse.contains("\"Hello, world!\""));
    }
    
    #[test]
    fn test_sse_text_delta_escape() {
        let event = SseEvent::TextDelta {
            content: "Line1\nLine2\rWith \"quotes\"".to_string(),
        };
        
        let sse = event.to_sse_string();
        assert!(sse.contains("\\n"));
        assert!(sse.contains("\\r"));
        assert!(sse.contains("\\\""));
    }

    #[test]
    fn test_sse_builder_process() {
        let mut builder = SseEventBuilder::new("resp_abc".to_string(), "gpt-4".to_string());
        
        let chunk = crate::models::chat::ChatCompletionChunk {
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
        assert!(!events.is_empty());
        
        // Check that response.created was emitted
        let has_created = events.iter().any(|e| matches!(e, SseEvent::ResponseCreated { .. }));
        assert!(has_created);
    }
    
    #[test]
    fn test_sse_builder_tool_calls() {
        let mut builder = SseEventBuilder::new("resp_xyz".to_string(), "gpt-4".to_string());
        
        let chunk = crate::models::chat::ChatCompletionChunk {
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
                        crate::models::chat::ToolCall {
                            id: "call_123".to_string(),
                            call_type: "function".to_string(),
                            function: crate::models::chat::FunctionCall {
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
        
        // Should have function call delta
        let has_func_delta = events.iter().any(|e| matches!(e, SseEvent::FunctionCallDelta { .. }));
        assert!(has_func_delta);
    }
}
