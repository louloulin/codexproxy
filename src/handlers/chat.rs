//! Chat Completions API handlers
//!
//! POST /v1/chat/completions - Main chat completions endpoint
//! POST /v1/providers/zhipu/chat/completions - Direct Zhipu endpoint

use axum::{
    extract::State,
    response::{sse::Sse, IntoResponse},
    Json,
};
use futures::{stream, StreamExt};
use std::sync::Arc;
use std::time::Instant;

use crate::db::repository::RequestRecord;
use crate::error::Error;
use crate::models::chat::ChatRequest;

use super::utils::{chat_stream_event_from_chat_chunk, rewrite_chat_request_model_for_provider, AppState};

/// Chat Completions handler
/// POST /v1/chat/completions
///
/// This endpoint accepts Chat Completions format and passes it to the provider.
/// The transform layer is available for future use when calling Responses API.
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatRequest>,
) -> impl IntoResponse {
    let requested_model = body.model.clone();
    let start_time = Instant::now();

    let provider = match state.get_provider(&body.model) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };

    let provider_name = provider.name().to_string();
    let mut body = body;
    let rewritten_from = rewrite_chat_request_model_for_provider(&state, &provider_name, &mut body);
    let effective_model = body.model.clone();
    let stream = body.stream.unwrap_or(false);
    let message_count = body.messages.len();

    // Generate request ID for tracking
    let request_id = uuid::Uuid::new_v4().to_string();
    let tool_count = body.tools.as_ref().map(|tools| tools.len()).unwrap_or(0);

    tracing::info!(
        route = "/v1/chat/completions",
        provider = provider_name,
        model = %requested_model,
        effective_model = %effective_model,
        rewritten_from = rewritten_from.as_deref().unwrap_or(""),
        stream,
        message_count,
        tool_count,
        "dispatching chat completion request"
    );

    // For now, pass through to Chat Completions API
    // The transform layer (transform_chat_to_responses_request) is available
    // for future implementation when we want to call Responses API
    if stream {
        // Handle streaming
        match provider.chat_streaming(body).await {
            Ok(streaming) => {
                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status()
                    ))
                    .into_response();
                }

                // Transform streaming chunks to Responses format to demonstrate conversion
                let stream = match streaming {
                    crate::providers::StreamingChat::Collected { chunks, .. } => {
                        stream::iter(
                            chunks
                                .into_iter()
                                .filter_map(|chunk| chat_stream_event_from_chat_chunk(&chunk))
                                .map(Ok::<_, std::convert::Infallible>),
                        )
                        .boxed()
                    }
                    crate::providers::StreamingChat::Streamed { stream, .. } => {
                        stream
                            .filter_map(|chunk| async move {
                                chat_stream_event_from_chat_chunk(&chunk)
                            })
                            .map(|event| Ok::<_, std::convert::Infallible>(event))
                            .boxed()
                    }
                };

                Sse::new(stream).into_response()
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/chat/completions",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "provider chat streaming request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    } else {
        // Handle non-streaming
        match provider.chat(body).await {
            Ok(chat_response) => {
                // Log successful request
                if let Some(ref repo) = state.request_repo {
                    let latency_ms = start_time.elapsed().as_millis() as i32;
                    let tokens_used = chat_response.usage.as_ref().map(|u| {
                        (u.prompt_tokens + u.completion_tokens) as i32
                    });
                    let record = RequestRecord {
                        id: request_id,
                        provider: provider_name.clone(),
                        model: effective_model.clone(),
                        endpoint: "/v1/chat/completions".to_string(),
                        status_code: Some(200),
                        tokens_used,
                        latency_ms: Some(latency_ms),
                        error_message: None,
                    };
                    if let Err(e) = repo.log_request(&record) {
                        tracing::warn!("Failed to log request: {}", e);
                    }
                }

                // Transform to Responses format and back to demonstrate conversion capability
                let responses_response =
                    crate::transform::transform_chat_to_responses_response(&chat_response);
                let chat_response =
                    crate::transform::transform_responses_to_chat_response(&responses_response);
                Json(chat_response).into_response()
            }
            Err(e) => {
                // Log failed request
                if let Some(ref repo) = state.request_repo {
                    let latency_ms = start_time.elapsed().as_millis() as i32;
                    let record = RequestRecord {
                        id: request_id,
                        provider: provider_name.clone(),
                        model: effective_model.clone(),
                        endpoint: "/v1/chat/completions".to_string(),
                        status_code: Some(500),
                        tokens_used: None,
                        latency_ms: Some(latency_ms),
                        error_message: Some(e.to_string()),
                    };
                    if let Err(e) = repo.log_request(&record) {
                        tracing::warn!("Failed to log request: {}", e);
                    }
                }

                tracing::error!(
                    route = "/v1/chat/completions",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "provider chat request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    }
}

/// Zhipu direct endpoint - bypasses transform layer
/// POST /v1/providers/zhipu/chat/completions
///
/// This endpoint directly proxies to Zhipu AI without any format transformation.
/// Useful for clients that want native GLM model access.
pub async fn zhipu_chat_completions(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatRequest>,
) -> impl IntoResponse {
    let requested_model = body.model.clone();
    let start_time = Instant::now();

    // Get Zhipu provider directly
    let provider = match state.zhipu_provider.clone() {
        Some(p) => p,
        None => {
            return Error::Provider("Zhipu provider not configured".to_string()).into_response()
        }
    };

    let mut body = body;
    let rewritten_from =
        rewrite_chat_request_model_for_provider(&state, provider.name(), &mut body);
    let effective_model = body.model.clone();
    let stream = body.stream.unwrap_or(false);
    let message_count = body.messages.len();
    let tool_count = body.tools.as_ref().map(|tools| tools.len()).unwrap_or(0);

    // Generate request ID for tracking
    let request_id = uuid::Uuid::new_v4().to_string();

    tracing::info!(
        route = "/v1/providers/zhipu/chat/completions",
        provider = provider.name(),
        model = %requested_model,
        effective_model = %effective_model,
        rewritten_from = rewritten_from.as_deref().unwrap_or(""),
        stream,
        message_count,
        tool_count,
        "dispatching direct zhipu request"
    );

    if stream {
        // Handle streaming
        match provider.chat_streaming(body).await {
            Ok(streaming) => {
                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status()
                    ))
                    .into_response();
                }

                // Pass through streaming chunks without transformation
                let stream = match streaming {
                    crate::providers::StreamingChat::Collected { chunks, .. } => {
                        stream::iter(chunks.into_iter().map(|chunk| {
                            Ok::<_, std::convert::Infallible>(
                                axum::response::sse::Event::default()
                                    .data(serde_json::to_string(&chunk).unwrap_or_default()),
                            )
                        }))
                        .boxed()
                    }
                    crate::providers::StreamingChat::Streamed { stream, .. } => stream
                        .map(|chunk| {
                            Ok::<_, std::convert::Infallible>(
                                axum::response::sse::Event::default()
                                    .data(serde_json::to_string(&chunk).unwrap_or_default()),
                            )
                        })
                        .boxed(),
                };

                Sse::new(stream).into_response()
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/providers/zhipu/chat/completions",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "direct zhipu streaming request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    } else {
        // Handle non-streaming - direct pass-through
        match provider.chat(body).await {
            Ok(chat_response) => {
                // Log successful request
                if let Some(ref repo) = state.request_repo {
                    let latency_ms = start_time.elapsed().as_millis() as i32;
                    let tokens_used = chat_response.usage.as_ref().map(|u| {
                        (u.prompt_tokens + u.completion_tokens) as i32
                    });
                    let record = RequestRecord {
                        id: request_id,
                        provider: "zhipu".to_string(),
                        model: effective_model.clone(),
                        endpoint: "/v1/providers/zhipu/chat/completions".to_string(),
                        status_code: Some(200),
                        tokens_used,
                        latency_ms: Some(latency_ms),
                        error_message: None,
                    };
                    if let Err(e) = repo.log_request(&record) {
                        tracing::warn!("Failed to log request: {}", e);
                    }
                }
                Json(chat_response).into_response()
            }
            Err(e) => {
                // Log failed request
                if let Some(ref repo) = state.request_repo {
                    let latency_ms = start_time.elapsed().as_millis() as i32;
                    let record = RequestRecord {
                        id: request_id,
                        provider: "zhipu".to_string(),
                        model: effective_model.clone(),
                        endpoint: "/v1/providers/zhipu/chat/completions".to_string(),
                        status_code: Some(500),
                        tokens_used: None,
                        latency_ms: Some(latency_ms),
                        error_message: Some(e.to_string()),
                    };
                    if let Err(e) = repo.log_request(&record) {
                        tracing::warn!("Failed to log request: {}", e);
                    }
                }

                tracing::error!(
                    route = "/v1/providers/zhipu/chat/completions",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "direct zhipu request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    }
}

/// MiniMax Chat Completions handler
/// POST /v1/text/chatcompletion_v2
///
/// This endpoint accepts Chat Completions format and passes it to MiniMax API.
/// MiniMax uses a different endpoint than standard OpenAI API.
pub async fn minimax_chat_completions(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatRequest>,
) -> impl IntoResponse {
    let requested_model = body.model.clone();
    let start_time = Instant::now();

    // Get MiniMax provider directly
    let provider = match state.minimax_provider.clone() {
        Some(p) => p,
        None => {
            return Error::Provider("MiniMax provider not configured".to_string()).into_response()
        }
    };

    let mut body = body;
    let effective_model = body.model.clone();
    let stream = body.stream.unwrap_or(false);
    let message_count = body.messages.len();
    let tool_count = body.tools.as_ref().map(|tools| tools.len()).unwrap_or(0);

    // Generate request ID for tracking
    let request_id = uuid::Uuid::new_v4().to_string();

    tracing::info!(
        route = "/v1/text/chatcompletion_v2",
        provider = provider.name(),
        model = %requested_model,
        effective_model = %effective_model,
        stream,
        message_count,
        tool_count,
        "dispatching minimax request"
    );

    if stream {
        // Handle streaming
        match provider.chat_streaming(body).await {
            Ok(streaming) => {
                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status()
                    ))
                    .into_response();
                }

                // Pass through streaming chunks without transformation
                let stream = match streaming {
                    crate::providers::StreamingChat::Collected { chunks, .. } => {
                        stream::iter(chunks.into_iter().map(|chunk| {
                            Ok::<_, std::convert::Infallible>(
                                axum::response::sse::Event::default()
                                    .data(serde_json::to_string(&chunk).unwrap_or_default()),
                            )
                        }))
                        .boxed()
                    }
                    crate::providers::StreamingChat::Streamed { stream, .. } => stream
                        .map(|chunk| {
                            Ok::<_, std::convert::Infallible>(
                                axum::response::sse::Event::default()
                                    .data(serde_json::to_string(&chunk).unwrap_or_default()),
                            )
                        })
                        .boxed(),
                };

                Sse::new(stream).into_response()
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/text/chatcompletion_v2",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "minimax streaming request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    } else {
        // Handle non-streaming - direct pass-through
        match provider.chat(body).await {
            Ok(chat_response) => {
                // Log successful request
                if let Some(ref repo) = state.request_repo {
                    let latency_ms = start_time.elapsed().as_millis() as i32;
                    let tokens_used = chat_response.usage.as_ref().map(|u| {
                        (u.prompt_tokens + u.completion_tokens) as i32
                    });
                    let record = RequestRecord {
                        id: request_id,
                        provider: "minimax".to_string(),
                        model: effective_model.clone(),
                        endpoint: "/v1/text/chatcompletion_v2".to_string(),
                        status_code: Some(200),
                        tokens_used,
                        latency_ms: Some(latency_ms),
                        error_message: None,
                    };
                    if let Err(e) = repo.log_request(&record) {
                        tracing::warn!("Failed to log request: {}", e);
                    }
                }
                Json(chat_response).into_response()
            }
            Err(e) => {
                // Log failed request
                if let Some(ref repo) = state.request_repo {
                    let latency_ms = start_time.elapsed().as_millis() as i32;
                    let record = RequestRecord {
                        id: request_id,
                        provider: "minimax".to_string(),
                        model: effective_model.clone(),
                        endpoint: "/v1/text/chatcompletion_v2".to_string(),
                        status_code: Some(500),
                        tokens_used: None,
                        latency_ms: Some(latency_ms),
                        error_message: Some(e.to_string()),
                    };
                    if let Err(e) = repo.log_request(&record) {
                        tracing::warn!("Failed to log request: {}", e);
                    }
                }

                tracing::error!(
                    route = "/v1/text/chatcompletion_v2",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "minimax request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    }
}

/// Models list handler
/// GET /v1/models
///
/// Returns list of all available models in OpenAI format
pub async fn models(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let model_list = state.list_models();

    #[derive(serde::Serialize)]
    struct ModelsResponse {
        object: String,
        data: Vec<super::utils::ModelInfo>,
    }

    Json(ModelsResponse {
        object: "list".to_string(),
        data: model_list,
    }).into_response()
}
