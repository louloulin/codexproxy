//! Tests for Codex CLI event types and parsing
//!
//! These tests verify that Codex CLI events can be correctly parsed
//! and that the stream state machine works properly.

use openai_proxy::models::streaming::{
    CodexCliEvent, CodexCliStreamState, try_parse_codex_cli_event, is_codex_cli_event,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_thread_started_event() {
        let event_json = r#"{"type":"thread.started","thread_id":"thread_123","session_id":"session_456"}"#;
        let event = serde_json::from_str::<CodexCliEvent>(event_json);
        assert!(event.is_ok());

        if let CodexCliEvent::ThreadStarted { thread_id, session_id } = event.unwrap() {
            assert_eq!(thread_id, "thread_123");
            assert_eq!(session_id, Some("session_456".to_string()));
        }
    }

    #[test]
    fn test_parse_thread_started_without_session() {
        let event_json = r#"{"type":"thread.started","thread_id":"thread_123"}"#;
        let event = serde_json::from_str::<CodexCliEvent>(event_json);
        assert!(event.is_ok());

        if let CodexCliEvent::ThreadStarted { thread_id, session_id } = event.unwrap() {
            assert_eq!(thread_id, "thread_123");
            assert_eq!(session_id, None);
        }
    }

    #[test]
    fn test_parse_turn_started_event() {
        let event_json = r#"{"type":"turn.started","turn_id":"turn_456","thread_id":"thread_123"}"#;
        let event = serde_json::from_str::<CodexCliEvent>(event_json);
        assert!(event.is_ok());

        if let CodexCliEvent::TurnStarted { turn_id, thread_id, parent_turn_id } = event.unwrap() {
            assert_eq!(turn_id, "turn_456");
            assert_eq!(thread_id, Some("thread_123".to_string()));
            assert_eq!(parent_turn_id, None);
        }
    }

    #[test]
    fn test_parse_request_submitted_event() {
        let event_json = r#"{"type":"request.submitted","request_id":"req_789","thread_id":"thread_123","turn_id":"turn_456"}"#;
        let event = serde_json::from_str::<CodexCliEvent>(event_json);
        assert!(event.is_ok());

        if let CodexCliEvent::RequestSubmitted { request_id, thread_id, turn_id } = event.unwrap() {
            assert_eq!(request_id, "req_789");
            assert_eq!(thread_id, "thread_123");
            assert_eq!(turn_id, "turn_456");
        }
    }

    #[test]
    fn test_parse_response_generation_started() {
        let event_json = r#"{"type":"response.generation_started","response_id":"resp_abc","turn_id":"turn_456"}"#;
        let event = serde_json::from_str::<CodexCliEvent>(event_json);
        assert!(event.is_ok());

        if let CodexCliEvent::ResponseGenerationStarted { response_id, turn_id } = event.unwrap() {
            assert_eq!(response_id, "resp_abc");
            assert_eq!(turn_id, "turn_456");
        }
    }

    #[test]
    fn test_parse_turn_completed_event() {
        let event_json = r#"{"type":"turn.completed","turn_id":"turn_456","thread_id":"thread_123"}"#;
        let event = serde_json::from_str::<CodexCliEvent>(event_json);
        assert!(event.is_ok());

        if let CodexCliEvent::TurnCompleted { turn_id, thread_id } = event.unwrap() {
            assert_eq!(turn_id, "turn_456");
            assert_eq!(thread_id, "thread_123");
        }
    }

    #[test]
    fn test_parse_turn_failed_event() {
        let event_json = r#"{"type":"turn.failed","turn_id":"turn_456","thread_id":"thread_123","error":"Something went wrong"}"#;
        let event = serde_json::from_str::<CodexCliEvent>(event_json);
        assert!(event.is_ok());

        if let CodexCliEvent::TurnFailed { turn_id, thread_id, error } = event.unwrap() {
            assert_eq!(turn_id, "turn_456");
            assert_eq!(thread_id, "thread_123");
            assert_eq!(error, "Something went wrong");
        }
    }

    #[test]
    fn test_try_parse_codex_cli_event_with_prefix() {
        let line = "data: {\"type\":\"thread.started\",\"thread_id\":\"thread_123\"}";
        let event = try_parse_codex_cli_event(line);
        assert!(event.is_some());

        if let CodexCliEvent::ThreadStarted { thread_id, .. } = event.unwrap() {
            assert_eq!(thread_id, "thread_123");
        }
    }

    #[test]
    fn test_try_parse_codex_cli_event_without_prefix() {
        let line = "{\"type\":\"turn.started\",\"turn_id\":\"turn_456\"}";
        let event = try_parse_codex_cli_event(line);
        assert!(event.is_some());
    }

    #[test]
    fn test_try_parse_codex_cli_event_with_standard_request() {
        // Standard JSON request should not parse as CodexCliEvent
        let line = r#"{"model":"gpt-4o","input":[{"role":"user","content":"hi"}]}"#;
        let event = try_parse_codex_cli_event(line);
        assert!(event.is_none());
    }

    #[test]
    fn test_is_codex_cli_event() {
        assert!(is_codex_cli_event(r#"{"type":"thread.started","thread_id":"t1"}"#));
        assert!(is_codex_cli_event(r#"{"type":"turn.started","turn_id":"t1"}"#));
        assert!(!is_codex_cli_event(r#"{"model":"gpt-4o"}"#));
        assert!(!is_codex_cli_event(""));
    }

    #[test]
    fn test_codex_cli_stream_state_update() {
        let mut state = CodexCliStreamState::new();

        // Initially not ready
        assert!(!state.is_ready());

        // Update with thread started
        let thread_event = CodexCliEvent::ThreadStarted {
            thread_id: "thread_123".to_string(),
            session_id: Some("session_456".to_string()),
        };
        state.update(&thread_event);

        assert_eq!(state.thread_id, Some("thread_123".to_string()));
        assert_eq!(state.session_id, Some("session_456".to_string()));
        assert!(!state.request_received);

        // Update with turn started
        let turn_event = CodexCliEvent::TurnStarted {
            turn_id: "turn_789".to_string(),
            thread_id: None,
            parent_turn_id: None,
        };
        state.update(&turn_event);

        assert_eq!(state.turn_id, Some("turn_789".to_string()));
        assert!(state.is_ready());

        // Update with request submitted
        let request_event = CodexCliEvent::RequestSubmitted {
            request_id: "req_abc".to_string(),
            thread_id: "thread_123".to_string(),
            turn_id: "turn_789".to_string(),
        };
        state.update(&request_event);

        assert!(state.request_received);
    }

    #[test]
    fn test_codex_cli_stream_state_new() {
        let state = CodexCliStreamState::new();
        assert!(state.thread_id.is_none());
        assert!(state.turn_id.is_none());
        assert!(state.session_id.is_none());
        assert!(!state.request_received);
        assert!(!state.is_ready());
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = CodexCliEvent::ThreadStarted {
            thread_id: "thread_123".to_string(),
            session_id: Some("session_456".to_string()),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: CodexCliEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            CodexCliEvent::ThreadStarted { thread_id, session_id } => {
                assert_eq!(thread_id, "thread_123");
                assert_eq!(session_id, Some("session_456".to_string()));
            }
            _ => panic!("Wrong event type after roundtrip"),
        }
    }
}
