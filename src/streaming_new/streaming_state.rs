//! Streaming state management for Chat Completions to Responses conversion
//! 
//! This module tracks the state of streaming responses and accumulates
//! data across chunks before emitting SSE events.

use std::collections::HashMap;
use crate::models::chat::ChatCompletionChunk;

/// Accumulated state for a streaming tool call
#[derive(Debug, Clone)]
pub struct ToolCallState {
    /// The ID of the tool call
    pub call_id: String,
    /// The name of the function being called
    pub name: String,
    /// The accumulated arguments
    pub arguments: String,
}

/// Complete streaming state
#[derive(Debug, Clone)]
pub struct StreamingState {
    /// The response ID
    pub response_id: String,
    /// The model being used
    pub model: String,
    /// Accumulated text content
    pub text_buffer: String,
    /// Tool calls indexed by call_id
    pub tool_calls: HashMap<String, ToolCallState>,
    /// Order of tool calls (by call_id)
    pub tool_call_order: Vec<String>,
    /// Whether we've seen the first chunk
    pub started: bool,
    /// Current output index
    pub output_index: u32,
}

impl StreamingState {
    /// Create a new streaming state
    pub fn new(response_id: String, model: String) -> Self {
        Self {
            response_id,
            model,
            text_buffer: String::new(),
            tool_calls: HashMap::new(),
            tool_call_order: Vec::new(),
            started: false,
            output_index: 0,
        }
    }

    /// Process a chunk and update state
    pub fn process_chunk(&mut self, chunk: &ChatCompletionChunk) {
        self.started = true;
        
        for choice in &chunk.choices {
            if let Some(delta) = &choice.delta {
                // Accumulate text content
                if let Some(ref content) = delta.content {
                    self.text_buffer.push_str(content);
                }
                
                // Accumulate tool calls
                if let Some(ref tool_calls) = delta.tool_calls {
                    for tc in tool_calls {
                        let call_id = tc.id.clone();
                        
                        if let Some(existing) = self.tool_calls.get_mut(&call_id) {
                            // Append to existing tool call
                            existing.arguments.push_str(&tc.function.arguments);
                        } else {
                            // Start new tool call
                            let state = ToolCallState {
                                call_id: call_id.clone(),
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            };
                            self.tool_calls.insert(call_id.clone(), state);
                            self.tool_call_order.push(call_id);
                        }
                    }
                }
            }
        }
    }

    /// Check if we have any content to emit
    pub fn has_content(&self) -> bool {
        !self.text_buffer.is_empty() || !self.tool_calls.is_empty()
    }

    /// Get the next output index and increment
    pub fn next_output_index(&mut self) -> u32 {
        let idx = self.output_index;
        self.output_index += 1;
        idx
    }

    /// Reset for a new response
    pub fn reset(&mut self, response_id: String, model: String) {
        self.response_id = response_id;
        self.model = model;
        self.text_buffer.clear();
        self.tool_calls.clear();
        self.tool_call_order.clear();
        self.started = false;
        self.output_index = 0;
    }
}

impl Default for StreamingState {
    fn default() -> Self {
        Self {
            response_id: String::new(),
            model: String::new(),
            text_buffer: String::new(),
            tool_calls: HashMap::new(),
            tool_call_order: Vec::new(),
            started: false,
            output_index: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::{StreamingChoice, Delta};

    #[test]
    fn test_streaming_state_basic() {
        let mut state = StreamingState::new("resp_123".to_string(), "gpt-4".to_string());
        assert_eq!(state.response_id, "resp_123");
        assert!(!state.started);
        
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
        
        state.process_chunk(&chunk);
        assert!(state.started);
        assert_eq!(state.text_buffer, "Hello");
    }

    #[test]
    fn test_streaming_state_tool_calls() {
        let mut state = StreamingState::new("resp_456".to_string(), "gpt-4".to_string());
        
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
        
        state.process_chunk(&chunk);
        assert_eq!(state.tool_calls.len(), 1);
        assert!(state.tool_call_order.contains(&"call_123".to_string()));
        
        // Process another chunk to continue the tool call
        // Note: streaming sends partial JSON strings that get concatenated
        let chunk2 = ChatCompletionChunk {
            id: "chunk-2".to_string(),
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
                                arguments: r#"Beijing"}"#.to_string(),
                            },
                        },
                    ]),
                }),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };
        
        state.process_chunk(&chunk2);
        
        // The second chunk is appended to the first (same call_id)
        // Chunk1: {"city":" + Chunk2: Beijing"} = {"city":Beijing"}
        if let Some(tc) = state.tool_calls.get("call_123") {
            assert_eq!(tc.arguments, r#"{"city":Beijing"}"#);
        }
    }
}
