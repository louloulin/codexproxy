//! Unified Responses SSE event builder.
//!
//! This module provides a single source of truth for building SSE events
//! for the Responses API protocol. It handles both collected (non-streaming)
//! and streamed event generation consistently.
//!
//! Key principles:
//! - All events follow the Responses API SSE format
//! - Events are generated from canonical representations
//! - The builder ensures event sequence integrity

use crate::models::chat::ChatCompletionChunk;
use crate::models::response::{
    ContentBlock, DetailedUsage, FunctionCallOutput, MessageOutput, OutputItem, OutputText,
    ResponsesStreamChunk, Usage,
};
use crate::models::streaming::{
    RateLimitSnapshot, ResponseError, ResponseEvent, ResponseSnapshot,
};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;

/// Builder for Responses API SSE events.
///
/// This builder creates SSE event strings that conform to the Responses API
/// protocol, supporting both collected responses and streaming scenarios.
#[derive(Debug, Clone)]
pub struct ResponsesEventBuilder {
    response_id: String,
    model: String,
    created_at: u64,
    output_index: u32,
}

impl ResponsesEventBuilder {
    /// Create a new builder with response metadata.
    pub fn new(response_id: String, model: String) -> Self {
        Self {
            response_id,
            model,
            created_at: current_timestamp(),
            output_index: 0,
        }
    }

    /// Set the creation timestamp.
    #[allow(dead_code)]
    pub fn with_created_at(mut self, created_at: u64) -> Self {
        self.created_at = created_at;
        self
    }

    /// Build the response.created event.
    pub fn build_created_event(&self) -> String {
        let snapshot = self.build_snapshot("in_progress");
        let event = ResponseEvent::Created { response: snapshot };
        serialize_event(&event)
    }

    /// Build the response.in_progress event.
    pub fn build_in_progress_event(&self) -> String {
        let snapshot = self.build_snapshot("in_progress");
        let event = ResponseEvent::InProgress { response: snapshot };
        serialize_event(&event)
    }

    /// Build an output_item.added event.
    pub fn build_output_item_added(&mut self, item: &OutputItem) -> String {
        let event = ResponseEvent::OutputItemAdded {
            output_index: self.output_index,
            item: item.clone(),
        };
        serialize_event(&event)
    }

    /// Build an output_item.done event.
    #[allow(dead_code)]
    pub fn build_output_item_done(&mut self, item: &OutputItem) -> String {
        let event = ResponseEvent::OutputItemDone {
            output_index: self.output_index,
            item: item.clone(),
        };
        serialize_event(&event)
    }

    /// Build an output_text.delta event.
    pub fn build_text_delta(&self, delta: &str) -> String {
        let event = ResponseEvent::OutputTextDelta {
            output_index: self.output_index,
            content_index: Some(0),
            delta: delta.to_string(),
        };
        serialize_event(&event)
    }

    /// Build an output_text.done event.
    pub fn build_text_done(&self, text: &str) -> String {
        let event = ResponseEvent::OutputTextDone {
            output_index: self.output_index,
            text: text.to_string(),
        };
        serialize_event(&event)
    }

    /// Build a function_call_arguments.delta event.
    pub fn build_function_delta(&self, call_id: &str, delta: &str) -> String {
        let event = ResponseEvent::FunctionCallArgumentsDelta {
            output_index: self.output_index,
            call_id: call_id.to_string(),
            delta: delta.to_string(),
        };
        serialize_event(&event)
    }

    /// Build a function_call_arguments.done event.
    pub fn build_function_done(&self, call_id: &str, arguments: &str) -> String {
        let event = ResponseEvent::FunctionCallArgumentsDone {
            output_index: self.output_index,
            call_id: call_id.to_string(),
            arguments: arguments.to_string(),
        };
        serialize_event(&event)
    }

    /// Build a reasoning_summary_part.added event.
    #[allow(dead_code)]
    pub fn build_reasoning_summary_added(&self, summary_index: u32) -> String {
        let event = ResponseEvent::ReasoningSummaryPartAdded {
            output_index: self.output_index,
            summary_index,
        };
        serialize_event(&event)
    }

    /// Build a reasoning_summary_text.delta event.
    #[allow(dead_code)]
    pub fn build_reasoning_delta(&self, summary_index: u32, delta: &str) -> String {
        let event = ResponseEvent::ReasoningSummaryTextDelta {
            output_index: self.output_index,
            summary_index,
            delta: delta.to_string(),
        };
        serialize_event(&event)
    }

    /// Build a reasoning_summary_text.done event.
    #[allow(dead_code)]
    pub fn build_reasoning_done(&self, summary_index: u32, text: &str) -> String {
        let event = ResponseEvent::ReasoningSummaryTextDone {
            output_index: self.output_index,
            summary_index,
            text: text.to_string(),
        };
        serialize_event(&event)
    }

    /// Build the response.completed event.
    #[allow(dead_code)]
    pub fn build_completed_event(&self, output: Vec<OutputItem>, usage: Option<DetailedUsage>) -> String {
        let mut snapshot = self.build_snapshot("completed");
        snapshot.output = output;
        snapshot.usage = usage;
        let event = ResponseEvent::Completed { response: snapshot };
        serialize_event(&event)
    }

    /// Build the response.failed event.
    #[allow(dead_code)]
    pub fn build_failed_event(&self, error: &ResponseError) -> String {
        let mut snapshot = self.build_snapshot("failed");
        snapshot.error = Some(error.clone());
        let event = ResponseEvent::Failed { response: snapshot };
        serialize_event(&event)
    }

    /// Build the response.incomplete event.
    #[allow(dead_code)]
    pub fn build_incomplete_event(&self, reason: &str) -> String {
        let mut snapshot = self.build_snapshot("incomplete");
        snapshot.incomplete_details = Some(serde_json::json!({ "reason": reason }));
        let event = ResponseEvent::Incomplete { response: snapshot };
        serialize_event(&event)
    }

    /// Build the [DONE] marker.
    pub fn build_done_marker() -> String {
        "data: [DONE]\n\n".to_string()
    }

    /// Build a server_model event.
    #[allow(dead_code)]
    pub fn build_server_model_event(&self, model: &str) -> String {
        let event = ResponseEvent::ServerModel {
            model: model.to_string(),
        };
        serialize_event(&event)
    }

    /// Build a rate_limits event.
    #[allow(dead_code)]
    pub fn build_rate_limits_event(&self, rate_limits: RateLimitSnapshot) -> String {
        let event = ResponseEvent::RateLimits { rate_limits };
        serialize_event(&event)
    }

    fn build_snapshot(&self, status: &str) -> ResponseSnapshot {
        ResponseSnapshot {
            id: self.response_id.clone(),
            object: "response".to_string(),
            created_at: self.created_at,
            status: status.to_string(),
            model: self.model.clone(),
            output: Vec::new(),
            usage: None,
            error: None,
            incomplete_details: None,
            extra: HashMap::new(),
        }
    }

    /// Increment the output index for the next item.
    pub fn next_output_index(&mut self) {
        self.output_index += 1;
    }
}

/// Serialize an event to SSE format.
fn serialize_event<T: Serialize>(event: &T) -> String {
    format!(
        "data: {}\n\n",
        serde_json::to_string(event).unwrap_or_default()
    )
}

/// Get current Unix timestamp.
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ============================================================================
// Stream transformer for Responses chunks
// ============================================================================

/// Transform Responses API streaming chunks into SSE event strings.
///
/// This is used for providers that return Responses API format chunks
/// that need to be converted to SSE events.
pub struct ResponsesStreamTransformer {
    builder: ResponsesEventBuilder,
}

impl ResponsesStreamTransformer {
    /// Create a new transformer.
    #[allow(dead_code)]
    pub fn new(response_id: String, model: String) -> Self {
        Self {
            builder: ResponsesEventBuilder::new(response_id, model),
        }
    }

    /// Transform a ResponsesStreamChunk into SSE events.
    #[allow(dead_code)]
    pub fn transform_chunk(&mut self, chunk: &ResponsesStreamChunk) -> Vec<String> {
        let mut events = Vec::new();

        // Skip empty chunks unless they have usage
        if chunk.output.is_empty() && chunk.usage.is_none() {
            return events;
        }

        for item in &chunk.output {
            events.push(self.builder.build_output_item_added(item));
        }

        events
    }

    /// Build the completed event.
    #[allow(dead_code)]
    pub fn build_completed(&self, output: Vec<OutputItem>, usage: Option<DetailedUsage>) -> String {
        self.builder.build_completed_event(output, usage)
    }
}

fn responses_usage_from_chat_usage(usage: &crate::models::chat::Usage) -> Usage {
    Usage {
        input_tokens: u64::from(usage.prompt_tokens),
        input_tokens_details: None,
        output_tokens: u64::from(usage.completion_tokens),
        output_tokens_details: None,
        total_tokens: u64::from(usage.total_tokens),
    }
}

fn message_output_for_stream(
    id: String,
    index: u32,
    role: String,
    text: String,
    status: Option<String>,
) -> OutputItem {
    OutputItem::Message(MessageOutput {
        index,
        id: Some(id),
        role,
        content: vec![ContentBlock::OutputText(OutputText {
            text,
            annotations: Some(vec![]),
        })],
        status,
        end_turn: None,
        phase: None,
        tool_calls: None,
    })
}

fn function_call_output_for_stream(
    index: u32,
    call_id: String,
    name: String,
    arguments: String,
    status: Option<String>,
) -> OutputItem {
    OutputItem::FunctionCall(FunctionCallOutput {
        index,
        id: None,
        call_id,
        name,
        arguments,
        status,
    })
}

fn response_snapshot_json(
    id: &str,
    created_at: u64,
    model: &str,
    status: &str,
    output: Vec<OutputItem>,
    usage: Option<Usage>,
    text: Option<&str>,
) -> serde_json::Value {
    let text_field = if status == "completed" {
        if let Some(text_str) = text {
            json!(text_str)
        } else {
            let extracted_text = output.iter().find_map(|item| {
                if let OutputItem::Message(msg) = item {
                    msg.content.iter().find_map(|content| {
                        if let ContentBlock::OutputText(text) = content {
                            Some(text.text.clone())
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            });
            match extracted_text {
                Some(text) => json!(text),
                None => json!({"format": {"type": "text"}}),
            }
        }
    } else {
        json!({"format": {"type": "text"}})
    };

    json!({
        "id": id,
        "object": "response",
        "created_at": created_at,
        "status": status,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "max_output_tokens": null,
        "model": model,
        "output": output,
        "parallel_tool_calls": true,
        "previous_response_id": null,
        "reasoning": {
            "effort": null,
            "summary": null
        },
        "store": false,
        "temperature": null,
        "text": text_field,
        "tool_choice": "auto",
        "tools": [],
        "top_p": null,
        "truncation": "disabled",
        "usage": usage,
        "user": null,
        "metadata": {}
    })
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Protocol-core state tracker for streaming ChatCompletionChunk -> Responses SSE payloads.
#[derive(Debug, Default)]
pub struct ResponsesChatStreamState {
    initial_events_emitted: bool,
    added_indices: HashMap<u32, bool>,
    text_by_index: HashMap<u32, String>,
    final_usage: Option<Usage>,
    is_final: bool,
    text_done_indices: HashMap<u32, bool>,
    item_done_indices: HashMap<u32, bool>,
}

impl ResponsesChatStreamState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_chunk(&mut self, chunk: &ChatCompletionChunk) -> Vec<String> {
        let mut events = Vec::new();

        if chunk.usage.is_some() && !self.is_final {
            self.is_final = true;
            self.final_usage = chunk.usage.as_ref().map(responses_usage_from_chat_usage);
        }

        if !self.initial_events_emitted {
            events.push(
                serde_json::to_string(&json!({
                    "type": "response.created",
                    "response": response_snapshot_json(
                        &chunk.id,
                        chunk.created,
                        &chunk.model,
                        "in_progress",
                        vec![],
                        None,
                        None
                    )
                }))
                .expect("response.created should serialize"),
            );
            events.push(
                serde_json::to_string(&json!({
                    "type": "response.in_progress",
                    "response": response_snapshot_json(
                        &chunk.id,
                        chunk.created,
                        &chunk.model,
                        "in_progress",
                        vec![],
                        None,
                        None
                    )
                }))
                .expect("response.in_progress should serialize"),
            );
            events.push(
                serde_json::to_string(&json!({
                    "type": "response.server_model",
                    "model": chunk.model
                }))
                .expect("response.server_model should serialize"),
            );
            events.push(
                serde_json::to_string(&json!({
                    "type": "response.server_reasoning_included",
                    "reasoning": {
                        "included": false
                    }
                }))
                .expect("response.server_reasoning_included should serialize"),
            );

            self.initial_events_emitted = true;
        }

        for choice in &chunk.choices {
            let Some(delta) = &choice.delta else {
                continue;
            };

            let index = choice.index;

            if !self.added_indices.get(&index).copied().unwrap_or(false) {
                let role = delta.role.clone().unwrap_or_else(|| "assistant".to_string());
                let item_id = format!("msg_{}_{}", chunk.id, index);

                events.push(
                    serde_json::to_string(&json!({
                        "type": "response.output_item.added",
                        "output_index": index,
                        "item": {
                            "type": "message",
                            "id": item_id,
                            "role": role,
                            "status": "in_progress",
                            "content": [{
                                "type": "output_text",
                                "text": ""
                            }]
                        }
                    }))
                    .expect("response.output_item.added should serialize"),
                );

                self.added_indices.insert(index, true);
            }

            if let Some(content) = &delta.content {
                if !content.is_empty() {
                    let accumulated = self.text_by_index.entry(index).or_default();
                    accumulated.push_str(content);

                    let item_id = format!("msg_{}_{}", chunk.id, index);
                    events.push(
                        serde_json::to_string(&json!({
                            "type": "response.output_text.delta",
                            "item_id": item_id,
                            "output_index": index,
                            "content_index": 0,
                            "delta": content
                        }))
                        .expect("response.output_text.delta should serialize"),
                    );
                }
            }

            if choice.finish_reason.is_some() {
                let full_text = self.text_by_index.get(&index).cloned().unwrap_or_default();
                let item_id = format!("msg_{}_{}", chunk.id, index);

                if !self.text_done_indices.get(&index).copied().unwrap_or(false) {
                    events.push(
                        serde_json::to_string(&json!({
                            "type": "response.output_text.done",
                            "item_id": item_id,
                            "output_index": index,
                            "content_index": 0,
                            "text": full_text
                        }))
                        .expect("response.output_text.done should serialize"),
                    );
                    self.text_done_indices.insert(index, true);
                }

                if !self.item_done_indices.get(&index).copied().unwrap_or(false) {
                    let final_item = OutputItem::Message(MessageOutput {
                        index,
                        id: Some(item_id.clone()),
                        role: "assistant".to_string(),
                        content: vec![ContentBlock::OutputText(OutputText {
                            text: full_text,
                            annotations: Some(vec![]),
                        })],
                        status: Some("completed".to_string()),
                        end_turn: None,
                        phase: None,
                        tool_calls: None,
                    });

                    events.push(
                        serde_json::to_string(&json!({
                            "type": "response.output_item.done",
                            "output_index": index,
                            "item": final_item
                        }))
                        .expect("response.output_item.done should serialize"),
                    );
                    self.item_done_indices.insert(index, true);
                }
            }
        }

        if self.is_final {
            let output: Vec<OutputItem> = self
                .added_indices
                .keys()
                .copied()
                .collect::<Vec<_>>()
                .into_iter()
                .map(|index| {
                    let text = self.text_by_index.get(&index).cloned().unwrap_or_default();
                    message_output_for_stream(
                        format!("msg_{}_{}", chunk.id, index),
                        index,
                        "assistant".to_string(),
                        text,
                        Some("completed".to_string()),
                    )
                })
                .collect();

            events.push(
                serde_json::to_string(&json!({
                    "type": "response.completed",
                    "response": response_snapshot_json(
                        &chunk.id,
                        chunk.created,
                        &chunk.model,
                        "completed",
                        output,
                        self.final_usage.clone(),
                        None
                    )
                }))
                .expect("response.completed should serialize"),
            );
            events.push("[DONE]".to_string());
        }

        events
    }
}

pub fn responses_protocol_payloads_from_chat_chunks(chunks: &[ChatCompletionChunk]) -> Vec<String> {
    if chunks.is_empty() {
        let completed_payload = serde_json::to_string(&json!({
            "type": "response.completed",
            "response": response_snapshot_json(
                "resp_empty",
                current_unix_timestamp(),
                "unknown",
                "completed",
                vec![],
                None,
                None
            )
        }))
        .expect("empty completed payload should serialize");

        return vec![completed_payload, "[DONE]".to_string()];
    }

    let mut payloads = vec![
        serde_json::to_string(&json!({
            "type": "response.created",
            "response": response_snapshot_json(
                &chunks[0].id,
                chunks[0].created,
                &chunks[0].model,
                "in_progress",
                vec![],
                None,
                None
            )
        }))
        .expect("response.created should serialize"),
        serde_json::to_string(&json!({
            "type": "response.in_progress",
            "response": response_snapshot_json(
                &chunks[0].id,
                chunks[0].created,
                &chunks[0].model,
                "in_progress",
                vec![],
                None,
                None
            )
        }))
        .expect("response.in_progress should serialize"),
    ];

    let mut text_by_index: HashMap<u32, String> = HashMap::new();
    let mut item_id_by_index: HashMap<u32, String> = HashMap::new();
    let mut added_indices: HashMap<u32, bool> = HashMap::new();
    let mut done_indices: HashMap<u32, bool> = HashMap::new();
    let mut tool_call_output_index_by_id: HashMap<String, u32> = HashMap::new();
    let mut tool_call_arguments_by_id: HashMap<String, String> = HashMap::new();
    let mut tool_call_name_by_id: HashMap<String, String> = HashMap::new();
    let mut tool_call_done_by_id: HashMap<String, bool> = HashMap::new();
    let mut final_usage: Option<Usage> = None;
    let mut final_output: Vec<OutputItem> = Vec::new();
    let mut next_tool_output_index = chunks
        .iter()
        .flat_map(|chunk| chunk.choices.iter().map(|choice| choice.index))
        .max()
        .unwrap_or(0)
        .saturating_add(1);

    for chunk in chunks {
        if let Some(usage) = &chunk.usage {
            final_usage = Some(responses_usage_from_chat_usage(usage));
        }

        for choice in &chunk.choices {
            let Some(delta) = &choice.delta else {
                continue;
            };

            let item_id = item_id_by_index
                .entry(choice.index)
                .or_insert_with(|| format!("msg_{}_{}", chunk.id, choice.index))
                .clone();

            if !added_indices.get(&choice.index).copied().unwrap_or(false) {
                payloads.push(
                    serde_json::to_string(&json!({
                        "type": "response.output_item.added",
                        "output_index": choice.index,
                        "item": message_output_for_stream(
                            item_id.clone(),
                            choice.index,
                            delta.role.clone().unwrap_or_else(|| "assistant".to_string()),
                            String::new(),
                            Some("in_progress".to_string())
                        )
                    }))
                    .expect("response.output_item.added should serialize"),
                );
                added_indices.insert(choice.index, true);
            }

            if let Some(content) = &delta.content {
                if !content.is_empty() {
                    let accumulated = text_by_index.entry(choice.index).or_default();
                    accumulated.push_str(content);

                    payloads.push(
                        serde_json::to_string(&json!({
                            "type": "response.output_text.delta",
                            "item_id": item_id,
                            "output_index": choice.index,
                            "content_index": 0,
                            "delta": content
                        }))
                        .expect("response.output_text.delta should serialize"),
                    );
                }
            }

            if let Some(tool_calls) = &delta.tool_calls {
                for tool_call in tool_calls {
                    let output_index = *tool_call_output_index_by_id
                        .entry(tool_call.id.clone())
                        .or_insert_with(|| {
                            let index = next_tool_output_index;
                            next_tool_output_index = next_tool_output_index.saturating_add(1);
                            index
                        });

                    tool_call_name_by_id
                        .insert(tool_call.id.clone(), tool_call.function.name.clone());
                    let accumulated_arguments = tool_call_arguments_by_id
                        .entry(tool_call.id.clone())
                        .and_modify(|arguments| arguments.push_str(&tool_call.function.arguments))
                        .or_insert_with(|| tool_call.function.arguments.clone())
                        .clone();

                    if !tool_call_done_by_id.get(&tool_call.id).copied().unwrap_or(false)
                        && accumulated_arguments == tool_call.function.arguments
                    {
                        payloads.push(
                            serde_json::to_string(&json!({
                                "type": "response.output_item.added",
                                "output_index": output_index,
                                "item": function_call_output_for_stream(
                                    output_index,
                                    tool_call.id.clone(),
                                    tool_call.function.name.clone(),
                                    String::new(),
                                    Some("in_progress".to_string())
                                )
                            }))
                            .expect("function call output_item.added should serialize"),
                        );
                    }

                    payloads.push(
                        serde_json::to_string(&json!({
                            "type": "response.function_call_arguments.delta",
                            "output_index": output_index,
                            "call_id": tool_call.id,
                            "delta": tool_call.function.arguments,
                        }))
                        .expect("response.function_call_arguments.delta should serialize"),
                    );
                }
            }

            let should_finalize_message = (delta.content.is_some()
                || text_by_index
                    .get(&choice.index)
                    .map(|text| !text.is_empty())
                    .unwrap_or(false))
                && !done_indices.get(&choice.index).copied().unwrap_or(false);

            if choice.finish_reason.is_some() && should_finalize_message {
                let full_text = text_by_index.get(&choice.index).cloned().unwrap_or_default();
                let final_item = message_output_for_stream(
                    item_id.clone(),
                    choice.index,
                    "assistant".to_string(),
                    full_text.clone(),
                    Some("completed".to_string()),
                );

                payloads.push(
                    serde_json::to_string(&json!({
                        "type": "response.output_text.done",
                        "item_id": item_id,
                        "output_index": choice.index,
                        "content_index": 0,
                        "text": full_text
                    }))
                    .expect("response.output_text.done should serialize"),
                );
                payloads.push(
                    serde_json::to_string(&json!({
                        "type": "response.output_item.done",
                        "output_index": choice.index,
                        "item": final_item.clone()
                    }))
                    .expect("response.output_item.done should serialize"),
                );

                final_output.push(final_item);
                done_indices.insert(choice.index, true);
            }

            if choice.finish_reason.is_some() {
                let tool_call_ids: Vec<String> = delta
                    .tool_calls
                    .as_ref()
                    .map(|tool_calls| tool_calls.iter().map(|tool_call| tool_call.id.clone()).collect())
                    .unwrap_or_default();

                for tool_call_id in tool_call_ids {
                    if tool_call_done_by_id.get(&tool_call_id).copied().unwrap_or(false) {
                        continue;
                    }

                    let arguments = tool_call_arguments_by_id
                        .get(&tool_call_id)
                        .cloned()
                        .unwrap_or_default();
                    let name = tool_call_name_by_id
                        .get(&tool_call_id)
                        .cloned()
                        .unwrap_or_else(|| "function".to_string());
                    let output_index = tool_call_output_index_by_id
                        .get(&tool_call_id)
                        .copied()
                        .unwrap_or_else(|| {
                            let index = next_tool_output_index;
                            next_tool_output_index = next_tool_output_index.saturating_add(1);
                            index
                        });

                    payloads.push(
                        serde_json::to_string(&json!({
                            "type": "response.function_call_arguments.done",
                            "output_index": output_index,
                            "call_id": tool_call_id,
                            "arguments": arguments,
                        }))
                        .expect("response.function_call_arguments.done should serialize"),
                    );

                    let final_item = function_call_output_for_stream(
                        output_index,
                        tool_call_id.clone(),
                        name,
                        tool_call_arguments_by_id
                            .get(&tool_call_id)
                            .cloned()
                            .unwrap_or_default(),
                        Some("completed".to_string()),
                    );
                    payloads.push(
                        serde_json::to_string(&json!({
                            "type": "response.output_item.done",
                            "output_index": output_index,
                            "item": final_item.clone()
                        }))
                        .expect("function call response.output_item.done should serialize"),
                    );

                    final_output.push(final_item);
                    tool_call_done_by_id.insert(tool_call_id, true);
                }
            }
        }
    }

    let last_chunk = chunks.last().expect("non-empty chunks");
    payloads.push(
        serde_json::to_string(&json!({
            "type": "response.completed",
            "response": response_snapshot_json(
                &last_chunk.id,
                last_chunk.created,
                &last_chunk.model,
                "completed",
                final_output,
                final_usage,
                None
            )
        }))
        .expect("response.completed should serialize"),
    );
    payloads.push("[DONE]".to_string());

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::{ChatCompletionChunk, Delta, StreamingChoice, Usage};

    #[test]
    fn test_event_builder_creates_valid_sse() {
        let builder = ResponsesEventBuilder::new(
            "resp_123".to_string(),
            "gpt-4o".to_string(),
        );

        let created = builder.build_created_event();
        assert!(created.starts_with("data: "));
        assert!(created.ends_with("\n\n"));
        assert!(created.contains("response.created"));
    }

    #[test]
    fn test_event_builder_text_delta() {
        let builder = ResponsesEventBuilder::new(
            "resp_123".to_string(),
            "gpt-4o".to_string(),
        );

        let delta = builder.build_text_delta("Hello");
        assert!(delta.contains("response.output_text.delta"));
        assert!(delta.contains("\"delta\":\"Hello\""));
    }

    #[test]
    fn test_event_builder_function_delta() {
        let builder = ResponsesEventBuilder::new(
            "resp_123".to_string(),
            "gpt-4o".to_string(),
        );

        let delta = builder.build_function_delta("call_123", "{\"arg\": \"value\"}");
        assert!(delta.contains("response.function_call_arguments.delta"));
        assert!(delta.contains("\"call_id\":\"call_123\""));
    }

    #[test]
    fn test_event_builder_done_marker() {
        let marker = ResponsesEventBuilder::build_done_marker();
        assert_eq!(marker, "data: [DONE]\n\n");
    }

    #[test]
    fn test_chat_chunk_protocol_core_emits_minimal_exec_lifecycle() {
        let chunks = vec![
            ChatCompletionChunk {
                id: "resp_proto".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567890,
                model: "glm-4-flash".to_string(),
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
            },
            ChatCompletionChunk {
                id: "resp_proto".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567891,
                model: "glm-4-flash".to_string(),
                choices: vec![StreamingChoice {
                    index: 0,
                    delta: Some(Delta {
                        role: None,
                        content: None,
                        tool_calls: None,
                    }),
                    finish_reason: Some("stop".to_string()),
                    logprobs: None,
                }],
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
            },
        ];

        let payloads = responses_protocol_payloads_from_chat_chunks(&chunks);

        assert!(payloads.iter().any(|payload| payload.contains(r#""type":"response.output_item.added""#)));
        assert!(payloads.iter().any(|payload| payload.contains(r#""type":"response.output_text.delta""#)));
        assert!(payloads.iter().any(|payload| payload.contains(r#""type":"response.output_item.done""#)));
        assert!(payloads.iter().any(|payload| payload.contains(r#""type":"response.completed""#)));
        assert_eq!(payloads.last().map(String::as_str), Some("[DONE]"));
    }

    #[test]
    fn test_chat_chunk_protocol_core_tracks_stream_state_across_chunks() {
        let first = ChatCompletionChunk {
            id: "resp_stream".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "glm-4-flash".to_string(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: Some(Delta {
                    role: Some("assistant".to_string()),
                    content: Some("Hel".to_string()),
                    tool_calls: None,
                }),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };
        let second = ChatCompletionChunk {
            id: "resp_stream".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567891,
            model: "glm-4-flash".to_string(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: Some(Delta {
                    role: None,
                    content: Some("lo".to_string()),
                    tool_calls: None,
                }),
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        let mut state = ResponsesChatStreamState::new();
        let first_payloads = state.process_chunk(&first);
        let second_payloads = state.process_chunk(&second);

        assert!(first_payloads.iter().any(|payload| payload.contains(r#""type":"response.output_item.added""#)));
        assert!(first_payloads.iter().any(|payload| payload.contains(r#""type":"response.output_text.delta""#)));
        assert!(second_payloads.iter().any(|payload| payload.contains(r#""type":"response.output_item.done""#) && payload.contains(r#""Hello""#)));
        assert!(second_payloads.iter().any(|payload| payload.contains(r#""type":"response.completed""#)));
        assert_eq!(second_payloads.last().map(String::as_str), Some("[DONE]"));
    }

    #[test]
    fn test_responses_protocol_payloads_emit_completed_without_usage_on_terminal_chunk() {
        let chunks = vec![
            ChatCompletionChunk {
                id: "resp_no_usage".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567890,
                model: "glm-5".to_string(),
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
            },
            ChatCompletionChunk {
                id: "resp_no_usage".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567891,
                model: "glm-5".to_string(),
                choices: vec![StreamingChoice {
                    index: 0,
                    delta: Some(Delta {
                        role: None,
                        content: None,
                        tool_calls: None,
                    }),
                    finish_reason: Some("stop".to_string()),
                    logprobs: None,
                }],
                usage: None,
            },
        ];

        let payloads = responses_protocol_payloads_from_chat_chunks(&chunks);

        assert!(payloads.iter().any(|payload| payload.contains(r#""type":"response.completed""#)));
        assert_eq!(payloads.last().map(String::as_str), Some("[DONE]"));
    }
}
