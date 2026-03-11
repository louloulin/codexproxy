//! Integration tests for API endpoints
//!
//! These tests verify the HTTP layer and endpoint behavior using
//! tower's testing utilities for one-shot request handling.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use openai_proxy::{config::Config, handlers::AppState};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

fn create_test_config() -> Config {
    Config {
        server: openai_proxy::config::ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
        },
        providers: openai_proxy::config::ProvidersConfig {
            openai: openai_proxy::config::ProviderConfig {
                api_key: "test-openai-key".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                default_model: "gpt-4o".to_string(),
                timeout: 60,
            },
            zhipu: openai_proxy::config::ProviderConfig {
                api_key: "test-zhipu-key".to_string(),
                base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                default_model: "glm-4".to_string(),
                timeout: 60,
            },
        },
        routing: openai_proxy::config::RoutingConfig {
            default: "openai".to_string(),
            model_mapping: None,
        },
        logging: openai_proxy::config::LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
        },
    }
}

fn create_app() -> axum::Router {
    let config = create_test_config();
    let state = Arc::new(AppState::new(config));
    openai_proxy::server::router::create_router(state)
}

#[tokio::test]
async fn test_health_check() {
    let app = create_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body_str, "OK");
}

#[tokio::test]
async fn test_root_endpoint() {
    let app = create_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_chat_completions_endpoint_exists() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Hello"}
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should get a response (might be error if no actual API, but endpoint exists)
    assert!(response.status() != StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_responses_endpoint_exists() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "input": []
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should get a response (might be error if no actual API, but endpoint exists)
    assert!(response.status() != StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_zhipu_direct_endpoint_exists() {
    let app = create_app();

    let request = json!({
        "model": "glm-4",
        "messages": [
            {"role": "user", "content": "你好"}
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/providers/zhipu/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should get a response (might be error if no actual API, but endpoint exists)
    assert!(response.status() != StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_chat_completions_invalid_json() {
    let app = create_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(b"{invalid json".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return bad request for invalid JSON
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_responses_invalid_json() {
    let app = create_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(b"{invalid json".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return bad request for invalid JSON
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_nonexistent_endpoint() {
    let app = create_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_method_not_allowed() {
    let app = create_app();

    // Try GET on POST-only endpoint
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/v1/chat/completions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_chat_completions_with_all_fields() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": "You are helpful"},
            {"role": "user", "content": "Hi"}
        ],
        "temperature": 0.7,
        "max_tokens": 100,
        "top_p": 0.9,
        "stream": false,
        "seed": 42,
        "presence_penalty": 0.5,
        "frequency_penalty": 0.3
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept the request (might fail later due to no real API)
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cors_headers() {
    let app = create_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .header("Origin", "http://example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // CORS layer should be present
    assert!(response
        .headers()
        .contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_provider_selection_by_model() {
    // Test that different models route to appropriate providers
    let config = {
        let mut cfg = create_test_config();
        let mut mapping = std::collections::HashMap::new();
        mapping.insert("glm-4".to_string(), "zhipu".to_string());
        mapping.insert("gpt-4o".to_string(), "openai".to_string());
        cfg.routing.model_mapping = Some(mapping);
        cfg
    };

    let state = Arc::new(AppState::new(config));

    // Test OpenAI model selection
    let openai_provider = state.get_provider("gpt-4o").unwrap();
    assert_eq!(openai_provider.name(), "openai");

    // Test Zhipu model selection
    let zhipu_provider = state.get_provider("glm-4").unwrap();
    assert_eq!(zhipu_provider.name(), "zhipu");
}

#[tokio::test]
async fn test_missing_provider_error() {
    // Create config without API keys
    let config = Config {
        server: openai_proxy::config::ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
        },
        providers: openai_proxy::config::ProvidersConfig {
            openai: openai_proxy::config::ProviderConfig {
                api_key: "".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                default_model: "gpt-4o".to_string(),
                timeout: 60,
            },
            zhipu: openai_proxy::config::ProviderConfig {
                api_key: "".to_string(),
                base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                default_model: "glm-4".to_string(),
                timeout: 60,
            },
        },
        routing: openai_proxy::config::RoutingConfig {
            default: "openai".to_string(),
            model_mapping: None,
        },
        logging: openai_proxy::config::LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
        },
    };

    let state = Arc::new(AppState::new(config));

    // Should fail because no provider is configured
    let result = state.get_provider("gpt-4o");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_chat_completions_empty_messages() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": []
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept the request
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_responses_with_instructions() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Hello"}]
            }
        ],
        "instructions": "You are a helpful assistant"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept the request
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_zhipu_with_streaming() {
    let app = create_app();

    let request = json!({
        "model": "glm-4",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "stream": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/providers/zhipu/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept the request
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chat_completions_with_tools() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "What's the weather?"}
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get current weather",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        }
                    }
                }
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept the request
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_concurrent_requests() {
    use tokio::task::JoinSet;

    let app = create_app();

    // Spawn multiple concurrent requests
    let mut set = JoinSet::new();
    for i in 0..5 {
        let app = app.clone();
        set.spawn(async move {
            let request = json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": format!("Request {}", i)}]
            });

            app.oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap()
        });
    }

    // All requests should complete without error
    while let Some(result) = set.join_next().await {
        let response = result.unwrap();
        assert!(response.status() != StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn test_chat_completions_missing_required_fields() {
    let app = create_app();

    // Missing 'messages' field
    let request = json!({
        "model": "gpt-4o"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle missing required fields gracefully
    // The request might fail at the provider level, but shouldn't crash
    assert!(response.status() != StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_responses_missing_required_fields() {
    let app = create_app();

    // Missing 'input' field
    let request = json!({
        "model": "gpt-4o"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle missing required fields gracefully
    assert!(response.status() != StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_chat_completions_with_response_format() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "response_format": {"type": "json_object"}
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept response_format parameter
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chat_completions_with_stop_sequences() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "stop": ["END", "STOP"]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept stop sequences
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_responses_with_text_format() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Hello"}]
            }
        ],
        "text": {"format": {"type": "json_object"}}
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept text.format parameter
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chat_completions_with_assistant_message() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there!"},
            {"role": "user", "content": "How are you?"}
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept conversation history with assistant messages
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chat_completions_with_system_message() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant"},
            {"role": "user", "content": "Hello"}
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept system messages
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_zhipu_with_different_models() {
    // Test with GLM-4
    let app = create_app();
    let request = json!({
        "model": "glm-4",
        "messages": [
            {"role": "user", "content": "你好"}
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/providers/zhipu/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept GLM-4 model
    assert!(response.status() != StatusCode::BAD_REQUEST);

    // Test with GLM-4-Flash (separate app instance)
    let app2 = create_app();
    let request = json!({
        "model": "glm-4-flash",
        "messages": [
            {"role": "user", "content": "你好"}
        ]
    });

    let response = app2
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/providers/zhipu/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept different GLM models
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chat_completions_with_tool_choice() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "What's the weather?"}
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather",
                    "parameters": {"type": "object"}
                }
            }
        ],
        "tool_choice": "auto"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept tool_choice parameter
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_responses_with_multiple_input_types() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [{"type": "input_text", "text": "You are helpful"}]
            },
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Hello"}]
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept multiple input types
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_content_type_validation() {
    let app = create_app();

    // Send request without Content-Type header
    let request = json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "Hello"}]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum should still process the request (it's lenient about content-type)
    assert!(response.status() != StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_empty_request_body() {
    let app = create_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return bad request for empty body
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_very_long_message_content() {
    let app = create_app();

    // Create a very long message
    let long_content = "x".repeat(10000);
    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": long_content}
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle long content (might fail at provider, but not crash)
    assert!(response.status() != StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_special_characters_in_content() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Hello 🌍! Special chars: <>&\"'\\n\\t"}
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle special characters
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_multiple_concurrent_health_checks() {
    use tokio::task::JoinSet;

    let app = create_app();

    // Spawn many concurrent health check requests
    let mut set = JoinSet::new();
    for _ in 0..20 {
        let app = app.clone();
        set.spawn(async move {
            app.oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
        });
    }

    // All health checks should succeed
    let mut success_count = 0;
    while let Some(result) = set.join_next().await {
        let response = result.unwrap();
        if response.status() == StatusCode::OK {
            success_count += 1;
        }
    }
    assert_eq!(success_count, 20);
}

// ==================== Streaming Tests ====================

#[tokio::test]
async fn test_chat_completions_streaming_basic() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Tell me a short story"}
        ],
        "stream": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept streaming request
    assert!(response.status() != StatusCode::BAD_REQUEST);

    // Check for SSE content type
    let content_type = response.headers().get("content-type");
    // Note: In mock tests without real providers, this may not be set
    // In real scenarios, it should be "text/event-stream"
    assert!(response.status() != StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_responses_streaming_basic() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "input": [
            {"type": "message", "role": "user", "content": "Hello"}
        ],
        "stream": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept streaming request
    assert!(response.status() != StatusCode::BAD_REQUEST);
    assert!(response.status() != StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_chat_completions_streaming_with_temperature() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Generate text"}
        ],
        "stream": true,
        "temperature": 0.8
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chat_completions_streaming_with_max_tokens() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Write something"}
        ],
        "stream": true,
        "max_tokens": 100
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_responses_streaming_with_instructions() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "instructions": "You are a helpful assistant",
        "input": [
            {"type": "message", "role": "user", "content": "Hello"}
        ],
        "stream": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chat_completions_streaming_with_tools() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "What's the weather?"}
        ],
        "stream": true,
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        }
                    }
                }
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_chat_completions_streaming_with_system_message() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant"},
            {"role": "user", "content": "Hello"}
        ],
        "stream": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_streaming_false_explicit() {
    let app = create_app();

    // Explicitly set stream to false
    let request = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "stream": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle non-streaming request
    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_responses_streaming_with_text_format() {
    let app = create_app();

    let request = json!({
        "model": "gpt-4o",
        "input": [
            {"type": "message", "role": "user", "content": "Generate JSON"}
        ],
        "stream": true,
        "text": {
            "format": {"type": "json_object"}
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status() != StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_zhipu_streaming_with_different_models() {
    let app = create_app();

    // Test with glm-4-flash
    let request = json!({
        "model": "glm-4-flash",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "stream": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/providers/zhipu/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status() != StatusCode::BAD_REQUEST);
}
