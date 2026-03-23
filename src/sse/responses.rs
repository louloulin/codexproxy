//! Responses API SSE event parsing
//!
//! Implements parsing for Server-Sent Events from the Responses API,
//! following the Codex CLI protocol specification.

use serde::Deserialize;
use thiserror::Error;

use crate::models::response::{OutputItem, Usage};
use crate::models::streaming::{
    ContentPartType, RateLimitSnapshot, ResponseError, ResponseEvent, ResponseSnapshot,
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
        "response.in_progress" => parse_in_progress_event(&raw.data)?,
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
        "response.reasoning_summary_text.delta" => parse_reasoning_summary_text_delta(&raw.data)?,
        "response.reasoning_summary_text.done" => parse_reasoning_summary_text_done(&raw.data)?,

        // Function call events
        "response.function_call_arguments.delta" => parse_function_call_arguments_delta(&raw.data)?,
        "response.function_call_arguments.done" => parse_function_call_arguments_done(&raw.data)?,

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

    Ok(Some(event))
}

// ============================================================================
// Event Parsers
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CreatedEventData {
    Nested { response: ResponseSnapshot },
    Legacy {
        id: String,
        object: String,
        created: u64,
        model: String,
    },
}

fn parse_created_event(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    Ok(ResponseEvent::Created {
        response: parse_lifecycle_snapshot(data.clone(), "in_progress")?,
    })
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CompletedEventData {
    Nested { response: ResponseSnapshot },
    Legacy {
        response_id: String,
        #[serde(default)]
        token_usage: Option<Usage>,
    },
}

fn parse_completed_event(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    Ok(ResponseEvent::Completed {
        response: parse_lifecycle_snapshot(data.clone(), "completed")?,
    })
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FailedEventData {
    Nested { response: ResponseSnapshot },
    Legacy {
        response_id: String,
        error: ResponseError,
    },
}

fn parse_failed_event(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    Ok(ResponseEvent::Failed {
        response: parse_lifecycle_snapshot(data.clone(), "failed")?,
    })
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum IncompleteEventData {
    Nested { response: ResponseSnapshot },
    Legacy { response_id: String, reason: String },
}

fn parse_incomplete_event(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    Ok(ResponseEvent::Incomplete {
        response: parse_lifecycle_snapshot(data.clone(), "incomplete")?,
    })
}

fn parse_in_progress_event(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
    Ok(ResponseEvent::InProgress {
        response: parse_lifecycle_snapshot(data.clone(), "in_progress")?,
    })
}

fn parse_lifecycle_snapshot(
    data: serde_json::Value,
    fallback_status: &str,
) -> Result<ResponseSnapshot, SseError> {
    if let Ok(CreatedEventData::Nested { response }) = serde_json::from_value(data.clone()) {
        return Ok(response);
    }

    if let Ok(CompletedEventData::Nested { response }) = serde_json::from_value(data.clone()) {
        return Ok(response);
    }

    if let Ok(FailedEventData::Nested { response }) = serde_json::from_value(data.clone()) {
        return Ok(response);
    }

    if let Ok(IncompleteEventData::Nested { response }) = serde_json::from_value(data.clone()) {
        return Ok(response);
    }

    if let Ok(CreatedEventData::Legacy {
        id,
        object,
        created,
        model,
    }) = serde_json::from_value(data.clone())
    {
        return Ok(ResponseSnapshot {
            id,
            object,
            created_at: created,
            status: fallback_status.to_string(),
            model,
            output: Vec::new(),
            usage: None,
            error: None,
            incomplete_details: None,
            extra: Default::default(),
        });
    }

    if let Ok(CompletedEventData::Legacy {
        response_id,
        token_usage,
    }) = serde_json::from_value(data.clone())
    {
        return Ok(ResponseSnapshot {
            id: response_id,
            object: "response".to_string(),
            created_at: 0,
            status: fallback_status.to_string(),
            model: String::new(),
            output: Vec::new(),
            usage: token_usage,
            error: None,
            incomplete_details: None,
            extra: Default::default(),
        });
    }

    if let Ok(FailedEventData::Legacy { response_id, error }) = serde_json::from_value(data.clone())
    {
        return Ok(ResponseSnapshot {
            id: response_id,
            object: "response".to_string(),
            created_at: 0,
            status: fallback_status.to_string(),
            model: String::new(),
            output: Vec::new(),
            usage: None,
            error: Some(error),
            incomplete_details: None,
            extra: Default::default(),
        });
    }

    if let Ok(IncompleteEventData::Legacy { response_id, reason }) = serde_json::from_value(data) {
        return Ok(ResponseSnapshot {
            id: response_id,
            object: "response".to_string(),
            created_at: 0,
            status: fallback_status.to_string(),
            model: String::new(),
            output: Vec::new(),
            usage: None,
            error: None,
            incomplete_details: Some(serde_json::json!({ "reason": reason })),
            extra: Default::default(),
        });
    }

    Err(SseError::InvalidFormat(
        "invalid lifecycle event payload".to_string(),
    ))
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

fn parse_reasoning_summary_part_added(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
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

fn parse_reasoning_summary_text_delta(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
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

fn parse_reasoning_summary_text_done(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
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

fn parse_function_call_arguments_done(data: &serde_json::Value) -> Result<ResponseEvent, SseError> {
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
            ResponseEvent::Created { response } => {
                assert_eq!(response.id, "resp_123");
                assert_eq!(response.model, "gpt-4");
            }
            _ => panic!("Expected Created event"),
        }
    }

    #[test]
    fn test_parse_created_event_with_nested_response_object() {
        let data = r#"{"type":"response.created","response":{"id":"resp_123","object":"response","created_at":1234567890,"status":"in_progress","model":"gpt-4","output":[],"usage":null}}"#;
        let result = parse_responses_sse_event(data).unwrap().unwrap();

        assert!(matches!(result, ResponseEvent::Created { .. }));
    }

    #[test]
    fn test_parse_in_progress_event_with_nested_response_object() {
        let data = r#"{"type":"response.in_progress","response":{"id":"resp_123","object":"response","created_at":1234567890,"status":"in_progress","model":"gpt-4","output":[],"usage":null}}"#;
        let result = parse_responses_sse_event(data).unwrap().unwrap();

        assert!(matches!(result, ResponseEvent::InProgress { .. }));
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

        let input = "data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"delta\":\"Hi\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_123\",\"object\":\"response\",\"created_at\":1234567890,\"status\":\"completed\",\"model\":\"gpt-4\",\"output\":[],\"usage\":null}}\n\n";

        let events = parser.parse(input);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_parse_completed_event_without_token_usage() {
        // Test parsing response.completed without token_usage field
        let data = r#"{"type":"response.completed","response_id":"resp_123"}"#;
        let result = parse_responses_sse_event(data).unwrap().unwrap();

        match result {
            ResponseEvent::Completed { response } => {
                assert_eq!(response.id, "resp_123");
                assert!(response.usage.is_none());
            }
            _ => panic!("Expected Completed event"),
        }
    }

    #[test]
    fn test_parse_completed_event_with_partial_token_usage() {
        // Test parsing response.completed with partial token_usage (missing input_tokens)
        let data = r#"{"type":"response.completed","response_id":"resp_456","token_usage":{"output_tokens":10}}"#;
        let result = parse_responses_sse_event(data).unwrap().unwrap();

        match result {
            ResponseEvent::Completed { response } => {
                assert_eq!(response.id, "resp_456");
                assert!(response.usage.is_some());
                let usage = response.usage.unwrap();
                assert_eq!(usage.input_tokens, 0); // Should default to 0
                assert_eq!(usage.output_tokens, 10);
                assert_eq!(usage.total_tokens, 0); // Should default to 0
            }
            _ => panic!("Expected Completed event"),
        }
    }

    #[test]
    fn test_parse_completed_event_with_nested_response_object() {
        let data = r#"{"type":"response.completed","response":{"id":"resp_789","object":"response","created_at":1234567890,"status":"completed","model":"gpt-4","output":[],"usage":{"output_tokens":10}}}"#;
        let result = parse_responses_sse_event(data).unwrap().unwrap();

        assert!(matches!(result, ResponseEvent::Completed { .. }));
    }
}
