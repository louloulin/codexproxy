//! Responses API SSE event parsing
//!
//! Implements parsing for Server-Sent Events from the Responses API,
//! following the Codex CLI protocol specification.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::response::{OutputItem, Usage};
use crate::models::streaming::{
    ContentPartType, RateLimitSnapshot, ResponseError, ResponseEvent,
};

/// SSE event parsing error
#[derive(Debug, Error)]
pub enum SseError {
    /// JSON parsing error
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Invalid event format
    #[error("Invalid event format: {0}")]
    InvalidFormat(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Unknown event type
    #[error("Unknown event type: {0}")]
    UnknownType(String),
}

/// Raw SSE event wrapper for parsing
#[derive(Debug, Clone, Deserialize)]
pub struct RawSseEvent {
    /// Event type
    #[serde(rename = "type")]
    pub event_type: String,

    /// Event data (flattened)
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Parse a Responses API SSE event from raw data
///
/// # Arguments
/// * `data` - The raw SSE data string (without "data: " prefix)
///
/// # Returns
/// * `Ok(Some(ResponseEvent))` - Successfully parsed event
/// * `Ok(None)` - Skip/ignore this event (e.g., [DONE])
/// * `Err(SseError)` - Parsing error
pub fn parse_responses_sse_event(data: &str) -> Result<Option<ResponseEvent>, SseError> {
    // Skip empty data
    let data = data.trim();
    if data.is_empty() {
        return Ok(None);
    }

    // Handle [DONE] marker
    if data == "[DONE]" {
        return Ok(None);
    }

    // Parse the raw event
    let raw: RawSseEvent = serde_json::from_str(data)?;

    // Convert based on event type
    let event = match raw.event_type.as_str() {
        // Response lifecycle events
        "response.created" => parse_created_event(&raw.data)?,
        "response.completed" => parse_completed_event(&raw.data)?,
        "response.failed" => parse_failed_event(&raw.data)?,
        "response.incomplete" => parse_incomplete_event(&raw.data)?,

        // Output item events
        "response.output_item.added" => parse_output_item_added(&raw.data)?,
        "response.output_item.done" => parse_output_item_done(&raw.data)?,

        // Content events
        "response.content_part.added" => parse_content_part_added(&raw.data)?,
        "response.output_text.delta" => parse_output_text_delta(&raw.data)?,
        "response.output_text.done" => parse_output_text_done(&raw.data)?,

        // Reasoning events
        "response.reasoning_summary_part.added" => parse_reasoning_summary_part_added(&raw.data)?,
        "response.reasoning_summary_text.delta" => {
            parse_reasoning_summary_text_delta(&raw.data)?
        }
        "response.reasoning_summary_text.done" => parse_reasoning_summary_text_done(&raw.data)?,

        // Function call events
        "response.function_call_arguments.delta" => {
            parse_function_call_arguments_delta(&raw.data)?
        }
        "response.function_call_arguments.done" => {
            parse_function_call_arguments_done(&raw.data)?
        }

        // Server info events
        "response.server_model" => parse_server_model(&raw.data)?,
        "response.server_reasoning_included" => parse_server_reasoning_included(&raw.data)?,
        "response.models_etag" => parse_models_etag(&raw.data)?,
        "response.rate_limits" => parse_rate_limits(&raw.data)?,

        // Unknown event type - skip with warning
        unknown => {
            tracing::debug!("Skipping unknown SSE event type: {}", unknown);
            return Ok(None);
        }
    };

    Ok(event)
}

// ============================================================================
// Event Parsers
// ============================================================================

#[derive(Debug, Deserialize)]
struct CreatedEventData {
    id: String,
    object: String,
    created: u64,
    model: String,
}

fn parse_created_event(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: CreatedEventData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::Created {
        id: event.id,
        object: event.object,
        created: event.created,
        model: event.model,
    })
}

#[derive(Debug, Deserialize)]
struct CompletedEventData {
    response_id: String,
    #[serde(default)]
    token_usage: Option<Usage>,
}

fn parse_completed_event(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: CompletedEventData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::Completed {
        response_id: event.response_id,
        token_usage: event.token_usage,
    })
}

#[derive(Debug, Deserialize)]
struct FailedEventData {
    response_id: String,
    error: ResponseError,
}

fn parse_failed_event(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: FailedEventData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::Failed {
        response_id: event.response_id,
        error: event.error,
    })
}

#[derive(Debug, Deserialize)]
struct IncompleteEventData {
    response_id: String,
    reason: String,
}

fn parse_incomplete_event(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: IncompleteEventData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::Incomplete {
        response_id: event.response_id,
        reason: event.reason,
    })
}

#[derive(Debug, Deserialize)]
struct OutputItemEventData {
    output_index: u32,
    item: OutputItem,
}

fn parse_output_item_added(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: OutputItemEventData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::OutputItemAdded {
        output_index: event.output_index,
        item: event.item,
    })
}

fn parse_output_item_done(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: OutputItemEventData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::OutputItemDone {
        output_index: event.output_index,
        item: event.item,
    })
}

#[derive(Debug, Deserialize)]
struct ContentPartAddedData {
    output_index: u32,
    content_index: u32,
    part: ContentPartType,
}

fn parse_content_part_added(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: ContentPartAddedData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::ContentPartAdded {
        output_index: event.output_index,
        content_index: event.content_index,
        part: event.part,
    })
}

#[derive(Debug, Deserialize)]
struct OutputTextDeltaData {
    output_index: u32,
    #[serde(default)]
    content_index: Option<u32>,
    delta: String,
}

fn parse_output_text_delta(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: OutputTextDeltaData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::OutputTextDelta {
        output_index: event.output_index,
        content_index: event.content_index,
        delta: event.delta,
    })
}

#[derive(Debug, Deserialize)]
struct OutputTextDoneData {
    output_index: u32,
    text: String,
}

fn parse_output_text_done(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: OutputTextDoneData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::OutputTextDone {
        output_index: event.output_index,
        text: event.text,
    })
}

#[derive(Debug, Deserialize)]
struct ReasoningSummaryPartAddedData {
    output_index: u32,
    summary_index: u32,
}

fn parse_reasoning_summary_part_added(
    data: &serde_json::Value,
) -> Result<ResponseEvent, SseError> {
    let event: ReasoningSummaryPartAddedData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::ReasoningSummaryPartAdded {
        output_index: event.output_index,
        summary_index: event.summary_index,
    })
}

#[derive(Debug, Deserialize)]
struct ReasoningSummaryTextDeltaData {
    output_index: u32,
    summary_index: u32,
    delta: String,
}

fn parse_reasoning_summary_text_delta(
    data: &serde_json::Value,
) -> Result<ResponseEvent, SseError> {
    let event: ReasoningSummaryTextDeltaData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::ReasoningSummaryTextDelta {
        output_index: event.output_index,
        summary_index: event.summary_index,
        delta: event.delta,
    })
}

#[derive(Debug, Deserialize)]
struct ReasoningSummaryTextDoneData {
    output_index: u32,
    summary_index: u32,
    text: String,
}

fn parse_reasoning_summary_text_done(
    data: &serde_json::Value,
) -> Result<ResponseEvent, SseError> {
    let event: ReasoningSummaryTextDoneData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::ReasoningSummaryTextDone {
        output_index: event.output_index,
        summary_index: event.summary_index,
        text: event.text,
    })
}

#[derive(Debug, Deserialize)]
struct FunctionCallArgumentsDeltaData {
    output_index: u32,
    call_id: String,
    delta: String,
}

fn parse_function_call_arguments_delta(
    data: &serde_json::Value,
) -> Result<ResponseEvent, SseError> {
    let event: FunctionCallArgumentsDeltaData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::FunctionCallArgumentsDelta {
        output_index: event.output_index,
        call_id: event.call_id,
        delta: event.delta,
    })
}

#[derive(Debug, Deserialize)]
struct FunctionCallArgumentsDoneData {
    output_index: u32,
    call_id: String,
    arguments: String,
}

fn parse_function_call_arguments_done(
    data: &serde_json::Value,
) -> Result<ResponseEvent, SseError> {
    let event: FunctionCallArgumentsDoneData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::FunctionCallArgumentsDone {
        output_index: event.output_index,
        call_id: event.call_id,
        arguments: event.arguments,
    })
}

#[derive(Debug, Deserialize)]
struct ServerModelData {
    model: String,
}

fn parse_server_model(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: ServerModelData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::ServerModel { model: event.model })
}

#[derive(Debug, Deserialize)]
struct ServerReasoningIncludedData {
    included: bool,
}

fn parse_server_reasoning_included(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: ServerReasoningIncludedData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::ServerReasoningIncluded {
        included: event.included,
    })
}

#[derive(Debug, Deserialize)]
struct ModelsEtagData {
    etag: String,
}

fn parse_models_etag(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: ModelsEtagData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::ModelsEtag { etag: event.etag })
}

#[derive(Debug, Deserialize)]
struct RateLimitsData {
    rate_limits: RateLimitSnapshot,
}

fn parse_rate_limits(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    let event: RateLimitsData = serde_json::from_value(data.clone())?;
    Ok(ResponseEvent::RateLimits {
        rate_limits: event.rate_limits,
    })
}

// ============================================================================
// SSE Stream Parser
// ============================================================================

/// SSE stream parser that accumulates partial data
#[derive(Debug, Default)]
pub struct SseStreamParser {
    /// Buffer for partial lines
    buffer: String,
}

impl SseStreamParser {
    /// Create a new SSE stream parser
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse incoming SSE data and return complete events
    ///
    /// # Arguments
    /// * `data` - Raw bytes received from the stream
    ///
    /// # Returns
    /// Vector of parsed events (may be empty if data is incomplete)
    pub fn parse(&mut self, data: &str) -> Vec<Result<ResponseEvent, SseError>> {
        self.buffer.push_str(data);

        let mut events = Vec::new();

        // Process complete lines
        while let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer[..pos].trim().to_string();
            self.buffer.drain(..=pos);

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Parse data lines
            if let Some(data) = line.strip_prefix("data: ") {
                match parse_responses_sse_event(data) {
                    Ok(Some(event)) => events.push(Ok(event)),
                    Ok(None) => {} // Skip event
                    Err(e) => events.push(Err(e)),
                }
            }
        }

        events
    }

    /// Check if there's remaining data in the buffer
    pub fn has_remaining(&self) -> bool {
        !self.buffer.trim().is_empty()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_created_event() {
        let data = r#"{"type":"response.created","id":"resp_123","object":"response","created":1234567890,"model":"gpt-4"}"#;
        let result = parse_responses_sse_event(data).unwrap().unwrap();

        match result {
            ResponseEvent::Created { id, model, .. } => {
                assert_eq!(id, "resp_123");
                assert_eq!(model, "gpt-4");
            }
            _ => panic!("Expected Created event"),
        }
    }

    #[test]
    fn test_parse_output_text_delta() {
        let data = r#"{"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"Hello"}"#;
        let result = parse_responses_sse_event(data).unwrap().unwrap();

        match result {
            ResponseEvent::OutputTextDelta { delta, .. } => {
                assert_eq!(delta, "Hello");
            }
            _ => panic!("Expected OutputTextDelta event"),
        }
    }

    #[test]
    fn test_parse_done_marker() {
        let result = parse_responses_sse_event("[DONE]").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_sse_stream_parser() {
        let mut parser = SseStreamParser::new();

        let input = "data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"delta\":\"Hi\"}\n\ndata: {\"type\":\"response.completed\",\"response_id\":\"resp_123\"}\n\n";

        let events = parser.parse(input);
        assert_eq!(events.len(), 2);
    }
}
