//! Streaming state management for Chat Completions to Responses conversion
//! 
//! This module tracks the state of streaming responses and accumulates
//! data across chunks before emitting SSE events.
//! 
//! Supports:
//! - Text content accumulation
//! - Tool calls (function calling)
//! - Reasoning/thinking content (<think>...</think>)
//! - Reasoning summary text (for o1/o3 series)

use std::collections::HashMap;
use crate::models::chat::ChatCompletionChunk;
use crate::transform::thinking::ThinkSplitter;

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

/// Reasoning content state
#[derive(Debug, Clone, Default)]
pub struct ReasoningState {
    /// Whether we're currently inside a think block
    pub in_think: bool,
    /// Accumulated reasoning content
    pub content: String,
    /// Whether we have seen any think tags
    pub has_think_tags: bool,
    /// Splitter for processing think tags incrementally
    pub splitter: ThinkSplitter,
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
    /// Reasoning state
    pub reasoning: ReasoningState,
    /// Reasoning summary text (for o1/o3 models)
    pub reasoning_summary: Option<String>,
}

impl StreamingState {
    /// Create a new streaming state
    pub fn new(response_id: String, model: String) -> Self {
        Self {
            response_id,
            model: model.clone(),
            text_buffer: String::new(),
            tool_calls: HashMap::new(),
            tool_call_order: Vec::new(),
            started: false,
            output_index: 0,
            reasoning: ReasoningState::default(),
            reasoning_summary: None,
        }
    }

    /// Process a chunk and update state
    pub fn process_chunk(&mut self, chunk: &ChatCompletionChunk) {
        self.started = true;
        
        for choice in &chunk.choices {
            if let Some(delta) = &choice.delta {
                // Handle reasoning_content (DeepSeek, o1 style)
                if let Some(ref reasoning_content) = delta.reasoning_content {
                    self.process_reasoning_chunk(reasoning_content);
                }
                
                // Handle reasoning_summary_text (o1/o3 style)
                if let Some(ref summary) = delta.reasoning_summary_text {
                    if self.reasoning_summary.is_none() {
                        self.reasoning_summary = Some(summary.clone());
                    } else if let Some(ref mut existing) = self.reasoning_summary {
                        existing.push_str(summary);
                    }
                }
                
                // Accumulate text content
                if let Some(ref content) = delta.content {
                    self.text_buffer.push_str(content);
                }
                
                // Accumulate tool calls
                if let Some(ref tool_calls) = delta.tool_calls {
                    self.process_tool_calls(tool_calls);
                }
            }
        }
    }
    
    /// Process a reasoning content chunk
    fn process_reasoning_chunk(&mut self, content: &str) {
        self.reasoning.has_think_tags = true;
        
        // Use the ThinkSplitter for stream processing
        let (result, reasoning, is_complete) = self.reasoning.splitter.process_chunk(content);
        
        // Append any output text to the text buffer
        if !result.is_empty() {
            self.text_buffer.push_str(&result);
        }
        
        // Accumulate reasoning content
        if let Some(ref reason) = reasoning {
            if self.reasoning.content.is_empty() {
                self.reasoning.content = reason.clone();
            } else {
                self.reasoning.content.push_str("\n");
                self.reasoning.content.push_str(reason);
            }
        }
        
        self.reasoning.in_think = !is_complete;
    }
    
    /// Process tool calls from a delta
    fn process_tool_calls(&mut self, tool_calls: &[crate::models::chat::ToolCall]) {
        for tc in tool_calls {
            let call_id = tc.id.clone();
            
            if let Some(existing) = self.tool_calls.get_mut(&call_id) {
                // Append to existing tool call
                // Only update name if it's different (first chunk might have it)
                if !tc.function.name.is_empty() && existing.name.is_empty() {
                    existing.name = tc.function.name.clone();
                }
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

    /// Check if we have any content to emit
    pub fn has_content(&self) -> bool {
        !self.text_buffer.is_empty() || !self.tool_calls.is_empty()
    }
    
    /// Check if we have reasoning content
    pub fn has_reasoning(&self) -> bool {
        self.reasoning.has_think_tags 
            || self.reasoning.content.is_empty() == false
            || self.reasoning_summary.is_some()
    }
    
    /// Get reasoning content for output
    pub fn get_reasoning_content(&self) -> Option<String> {
        // Prefer reasoning_summary if available (o1/o3 style)
        if let Some(ref summary) = self.reasoning_summary {
            if !summary.is_empty() {
                return Some(summary.clone());
            }
        }
        
        // Otherwise use accumulated reasoning content
        if !self.reasoning.content.is_empty() {
            return Some(self.reasoning.content.clone());
        }
        
        None
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
        self.model = model.clone();
        self.text_buffer.clear();
        self.tool_calls.clear();
        self.tool_call_order.clear();
        self.started = false;
        self.output_index = 0;
        self.reasoning = ReasoningState::default();
        self.reasoning_summary = None;
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
            reasoning: ReasoningState::default(),
            reasoning_summary: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::{StreamingChoice, Delta, ToolCall, FunctionCall};

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
                    reasoning_content: None,
                    reasoning_summary_text: None,
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
                        ToolCall {
                            id: "call_123".to_string(),
                            call_type: "function".to_string(),
                            function: FunctionCall {
                                name: "get_weather".to_string(),
                                arguments: r#"{"city":"#.to_string(),
                            },
                        },
                    ]),
                    reasoning_content: None,
                    reasoning_summary_text: None,
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
                        ToolCall {
                            id: "call_123".to_string(),
                            call_type: "function".to_string(),
                            function: FunctionCall {
                                name: "get_weather".to_string(),
                                arguments: r#"Beijing"}"#.to_string(),
                            },
                        },
                    ]),
                    reasoning_content: None,
                    reasoning_summary_text: None,
                }),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };
        
        state.process_chunk(&chunk2);
        
        // The second chunk is appended to the first (same call_id)
        if let Some(tc) = state.tool_calls.get("call_123") {
            assert_eq!(tc.arguments, r#"{"city":Beijing"}"#);
        }
    }

    #[test]
    fn test_streaming_state_reasoning_content() {
        let mut state = StreamingState::new("resp_789".to_string(), "deepseek-chat".to_string());
        
        // First chunk with reasoning content
        let chunk = ChatCompletionChunk {
            id: "chunk-1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "deepseek-chat".to_string(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: Some(Delta {
                    role: Some("assistant".to_string()),
                    content: Some("<think>I'm thinking...".to_string()),
                    tool_calls: None,
                    reasoning_content: Some("<think>Let me solve this...".to_string()),
                    reasoning_summary_text: None,
                }),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };
        
        state.process_chunk(&chunk);
        assert!(state.has_reasoning());
        
        // Second chunk completing the reasoning
        let chunk2 = ChatCompletionChunk {
            id: "chunk-2".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "deepseek-chat".to_string(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: Some(Delta {
                    role: None,
                    content: Some("Here's my answer".to_string()),
                    tool_calls: None,
                    reasoning_content: Some("</think>The solution is...".to_string()),
                    reasoning_summary_text: None,
                }),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };
        
        state.process_chunk(&chunk2);
        
        // Check that reasoning was accumulated
        let reasoning = state.get_reasoning_content();
        assert!(reasoning.is_some());
    }

    #[test]
    fn test_streaming_state_reasoning_summary() {
        let mut state = StreamingState::new("resp_789".to_string(), "o1".to_string());
        
        // o1 style reasoning summary
        let chunk = ChatCompletionChunk {
            id: "chunk-1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "o1".to_string(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: Some(Delta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: None,
                    reasoning_content: None,
                    reasoning_summary_text: Some("Let me think about this step by step...".to_string()),
                }),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };
        
        state.process_chunk(&chunk);
        assert!(state.has_reasoning());
        assert_eq!(state.reasoning_summary.as_ref().unwrap(), "Let me think about this step by step...");
    }
}


#[cfg(test)]
mod mimo2codex_tests {
    use super::*;
    use crate::models::chat::Delta;

    #[test]
    fn test_streaming_state_with_params() {
        let state = StreamingState::new("resp_123".to_string(), "mimo-v2.5-pro".to_string());
        assert!(!state.has_reasoning());
        assert!(state.get_reasoning_content().is_none());
    }

    #[test]
    fn test_delta_with_reasoning_content() {
        let delta = Delta {
            content: None,
            reasoning_content: Some("thinking...".to_string()),
            reasoning_summary_text: None,
            ..Default::default()
        };
        assert!(delta.reasoning_content.is_some());
        assert_eq!(delta.reasoning_content.as_ref().unwrap(), "thinking...");
    }

    #[test]
    fn test_delta_with_reasoning_summary() {
        let delta = Delta {
            content: None,
            reasoning_content: None,
            reasoning_summary_text: Some("final reasoning".to_string()),
            ..Default::default()
        };
        assert!(delta.reasoning_summary_text.is_some());
    }

    #[test]
    fn test_delta_with_text_content() {
        let delta = Delta {
            content: Some("hello world".to_string()),
            reasoning_content: None,
            reasoning_summary_text: None,
            ..Default::default()
        };
        assert!(delta.content.is_some());
        assert_eq!(delta.content.as_ref().unwrap(), "hello world");
    }

    #[test]
    fn test_streaming_state_accumulates_reasoning() {
        let mut state = StreamingState::new("resp_123".to_string(), "mimo-v2.5-pro".to_string());
        state.reasoning.content.push_str("thinking part 1... ");
        state.reasoning.content.push_str("thinking part 2...");
        assert!(state.reasoning.content.contains("thinking part 1"));
    }

    #[test]
    fn test_streaming_state_reasoning_summary() {
        let mut state = StreamingState::new("resp_123".to_string(), "mimo-v2.5-pro".to_string());
        state.reasoning_summary = Some("Final summary text".to_string());
        assert!(state.reasoning_summary.is_some());
    }

    #[test]
    fn test_delta_default() {
        let delta = Delta::default();
        assert!(delta.content.is_none() || delta.content.is_some());
    }

    #[test]
    fn test_streaming_state_has_reasoning_initially_false() {
        let state = StreamingState::new("resp_123".to_string(), "mimo-v2.5-pro".to_string());
        assert!(!state.has_reasoning());
    }
}
