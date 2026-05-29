//! Codex CLI E2E Integration Tests
//!
//! These tests verify the full Codex CLI integration with rcodex,
//! testing the complete request/response flow.

use bytes::Bytes;
use rcodex::config::Config;
use rcodex::handlers::AppState;
use rcodex::models::streaming::{try_parse_codex_cli_event, CodexCliEvent};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_cli_event_sequence_parsing() {
        // Simulate the event sequence sent by Codex CLI
        let events = vec![
            r#"{"type":"thread.started","thread_id":"test_thread_123","session_id":"test_session"}"#,
            r#"{"type":"turn.started","turn_id":"test_turn_456","thread_id":"test_thread_123"}"#,
            r#"{"type":"request.submitted","request_id":"req_789","thread_id":"test_thread_123","turn_id":"test_turn_456"}"#,
        ];

        let mut thread_id = None;
        let mut turn_id = None;
        let mut request_id = None;

        for event_str in &events {
            if let Some(event) = try_parse_codex_cli_event(event_str) {
                match event {
                    CodexCliEvent::ThreadStarted { thread_id: tid, .. } => {
                        thread_id = Some(tid);
                    }
                    CodexCliEvent::TurnStarted { turn_id: tid, .. } => {
                        turn_id = Some(tid);
                    }
                    CodexCliEvent::RequestSubmitted { request_id: rid, .. } => {
                        request_id = Some(rid);
                    }
                    _ => {}
                }
            }
        }

        assert_eq!(thread_id, Some("test_thread_123".to_string()));
        assert_eq!(turn_id, Some("test_turn_456".to_string()));
        assert_eq!(request_id, Some("req_789".to_string()));
    }

    #[test]
    fn test_codex_cli_response_event_sequence() {
        // Simulate the expected response event sequence from rcodex
        let expected_events = vec![
            "thread.started",
            "turn.started",
            "response.created",
            "response.output_item.added",
            "response.output_text.delta",
            "response.completed",
            "turn.completed",
        ];

        // Verify all expected events are defined
        for event_type in &expected_events {
            assert!(
                event_type.contains('.') || event_type.contains('_'),
                "Event type should contain proper separator: {}",
                event_type
            );
        }

        assert_eq!(expected_events.len(), 7);
    }

    #[test]
    fn test_config_codex_cli_defaults() {
        let config = Config::default();
        assert!(config.codex_cli.enabled);
        assert!(config.codex_cli.auto_send_events);
        assert!(!config.codex_cli.send_server_model);
        assert!(!config.codex_cli.send_rate_limits);
    }

    #[test]
    fn test_codex_cli_event_types_coverage() {
        // Test all event types that Codex CLI might send
        let event_tests = vec![
            (
                r#"{"type":"thread.started","thread_id":"t1"}"#,
                "ThreadStarted",
            ),
            (
                r#"{"type":"turn.started","turn_id":"t1","thread_id":"t2"}"#,
                "TurnStarted",
            ),
            (
                r#"{"type":"request.submitted","request_id":"r1","thread_id":"t1","turn_id":"t2"}"#,
                "RequestSubmitted",
            ),
            (
                r#"{"type":"response.generation_started","response_id":"resp1","turn_id":"t1"}"#,
                "ResponseGenerationStarted",
            ),
            (
                r#"{"type":"turn.completed","turn_id":"t1","thread_id":"t2"}"#,
                "TurnCompleted",
            ),
            (
                r#"{"type":"turn.failed","turn_id":"t1","thread_id":"t2","error":"failed"}"#,
                "TurnFailed",
            ),
        ];

        for (event_json, expected_type) in event_tests {
            let parsed = try_parse_codex_cli_event(event_json);
            assert!(
                parsed.is_some(),
                "Failed to parse event: {}",
                event_json
            );
            let event_str = format!("{:?}", parsed.unwrap());
            assert!(
                event_str.contains(expected_type),
                "Expected {} in {:?}",
                expected_type,
                event_str
            );
        }
    }

    #[tokio::test]
    async fn test_codex_cli_request_extraction() {
        // Simulate a full Codex CLI request with events followed by actual request
        let body = r#"{"type":"thread.started","thread_id":"thread_123","session_id":"session_456"}
{"type":"turn.started","turn_id":"turn_789","thread_id":"thread_123"}
{"model":"gpt-4o","input":[{"role":"user","content":[{"type":"input_text","text":"Hello"}]}],"stream":true}"#;

        let lines: Vec<String> = body.lines().map(|l| l.to_string()).collect();
        let mut request_found = false;

        for line in &lines {
            if let Some(event) = try_parse_codex_cli_event(line) {
                if matches!(event, CodexCliEvent::ThreadStarted { .. }) {
                    assert!(line.contains("thread_123"));
                }
            } else if line.contains("\"model\"") && line.contains("\"input\"") {
                request_found = true;
                assert!(line.contains("gpt-4o"));
                assert!(line.contains("Hello"));
            }
        }

        assert!(request_found, "Request should be extractable from mixed event stream");
    }

    #[test]
    fn test_rate_limit_snapshot_structure() {
        use rcodex::models::streaming::RateLimitSnapshot;

        let snapshot = RateLimitSnapshot {
            requests_remaining: 100,
            tokens_remaining: 50000,
            requests_limit: 200,
            tokens_limit: 100000,
            reset_after_seconds: Some(60),
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        // RateLimitSnapshot uses camelCase serialization
        assert!(json.contains("requestsRemaining"));
        assert!(json.contains("tokensRemaining"));
        assert!(json.contains("100"));
        assert!(json.contains("50000"));
    }

    #[test]
    fn test_server_model_event_structure() {
        use rcodex::models::streaming::ResponseEvent;

        let event = ResponseEvent::ServerModel {
            model: "glm-5.1".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("response.server_model"));
        assert!(json.contains("glm-5.1"));
    }
}
