//! Responses API handler
//!
//! POST /v1/responses - Responses API endpoint with fallback support

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use futures::{stream, StreamExt};
use std::sync::Arc;

use crate::error::Error;
use crate::models::response::ResponsesRequest;
use crate::protocol::capabilities::ResponsesExecutionPlan;
use crate::providers::format_request_body_for_log;

use super::utils::{
    build_responses_execution_plan, rewrite_chat_request_model_for_provider,
    responses_stream_events_from_chat_chunks, unsupported_responses_features_message, AppState,
};

/// Responses API handler
/// POST /v1/responses
pub async fn responses(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ResponsesRequest>,
) -> impl IntoResponse {
    // 请求体调试日志：记录模型与序列化后大小
    if let Ok(body_str) = serde_json::to_string(&body) {
        tracing::debug!(
            model = %body.model,
            stream = %body.stream.unwrap_or(false),
            input_count = body.input.len(),
            body_len = body_str.len(),
            request_body = %format_request_body_for_log(&body_str),
            "responses request body"
        );
        // Log each input item's type for debugging
        for (i, item) in body.input.iter().enumerate() {
            tracing::debug!(index = i, item = ?item, "input item");
        }
    } else {
        tracing::debug!(
            model = %body.model,
            stream = %body.stream.unwrap_or(false),
            "responses request body (serialize failed)"
        );
    }

    let provider = match state.get_provider(&body.model) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let provider_name = provider.name().to_string();

    tracing::info!(
        route = "/v1/responses",
        provider = provider_name,
        model = %body.model,
        stream = body.stream.unwrap_or(false),
        input_count = body.input.len(),
        tool_count = body.tools.len(),
        instructions = body.instructions.is_some(),
        "dispatching responses request"
    );

    let stream = body.stream.unwrap_or(false);
    let requested_model_for_response = body.model.clone();

    let execution_plan = build_responses_execution_plan(provider.as_ref(), &body);
    tracing::info!(
        route = "/v1/responses",
        provider = provider.name(),
        model = %requested_model_for_response,
        stream,
        execution_plan = ?execution_plan,
        "planned responses execution"
    );

    match &execution_plan {
        ResponsesExecutionPlan::NativeResponses => {
            tracing::info!(
                route = "/v1/responses",
                provider = provider.name(),
                model = %requested_model_for_response,
                stream,
                include_count = body.include.len(),
                has_previous_response_id = body.previous_response_id.is_some(),
                "using native responses provider path"
            );

            if stream {
                match provider.responses_streaming(body).await {
                    Ok(streaming) => {
                        if !streaming.is_success() {
                            return Error::Provider(format!(
                                "Streaming responses request failed with status: {}",
                                streaming.status
                            ))
                            .into_response();
                        }

                        let stream = streaming.events.map(|payload| {
                            Ok::<_, std::convert::Infallible>(Event::default().data(payload))
                        });

                        return Sse::new(stream)
                            .keep_alive(KeepAlive::new())
                            .into_response();
                    }
                    Err(e) => {
                        tracing::error!(
                            route = "/v1/responses",
                            provider = provider.name(),
                            model = %requested_model_for_response,
                            error = %e,
                            "provider native responses streaming request failed"
                        );
                        return Error::Provider(e.to_string()).into_response();
                    }
                }
            }

            match provider.responses(body).await {
                Ok(responses_response) => return Json(responses_response).into_response(),
                Err(e) => {
                    tracing::error!(
                        route = "/v1/responses",
                        provider = provider.name(),
                        model = %requested_model_for_response,
                        error = %e,
                        "provider native responses request failed"
                    );
                    return Error::Provider(e.to_string()).into_response();
                }
            }
        }
        ResponsesExecutionPlan::Reject {
            unsupported_features,
        } => {
            let message =
                unsupported_responses_features_message(provider.name(), unsupported_features);
            tracing::warn!(
                route = "/v1/responses",
                provider = provider.name(),
                model = %requested_model_for_response,
                unsupported_features = ?unsupported_features,
                "rejecting responses request before lossy chat fallback"
            );
            return Error::Provider(message).into_response();
        }
        ResponsesExecutionPlan::ChatFallback => {}
    }

    // Chat fallback mode - convert Responses request to Chat request
    if stream {
        // Handle streaming - convert to chat format, call provider, convert back
        let mut chat_request = crate::transform::transform_responses_to_chat_request(&body);
        let rewritten_from =
            rewrite_chat_request_model_for_provider(&state, provider.name(), &mut chat_request);
        let effective_model = chat_request.model.clone();
        let transformed_body = serde_json::to_string(&chat_request).ok();
        tracing::debug!(
            route = "/v1/responses",
            provider = provider.name(),
            transformed_model = %effective_model,
            rewritten_from = rewritten_from.as_deref().unwrap_or(""),
            transformed_message_count = chat_request.messages.len(),
            transformed_tool_count = chat_request.tools.as_ref().map(|tools| tools.len()).unwrap_or(0),
            transformed_request_body = transformed_body
                .as_deref()
                .map(format_request_body_for_log)
                .unwrap_or_else(|| "<serialize failed>".to_string()),
            "responses request transformed to chat request"
        );

        match provider.chat_streaming(chat_request).await {
            Ok(streaming) => {
                tracing::info!(
                    route = "/v1/responses",
                    status = streaming.status(),
                    is_streamed = streaming.is_streamed(),
                    is_collected = streaming.is_collected(),
                    "provider streaming response received"
                );

                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status()
                    ))
                    .into_response();
                }

                // Handle based on streaming type
                match streaming {
                    crate::providers::StreamingChat::Collected { chunks, .. } => {
                        // Log chunk details
                        for (i, chunk) in chunks.iter().enumerate() {
                            tracing::debug!(
                                route = "/v1/responses",
                                chunk_index = i,
                                chunk_id = %chunk.id,
                                choices_count = chunk.choices.len(),
                                has_usage = chunk.usage.is_some(),
                                "streaming chunk detail"
                            );
                        }

                        let events = responses_stream_events_from_chat_chunks(&chunks);
                        tracing::info!(
                            route = "/v1/responses",
                            event_count = events.len(),
                            chunk_count = chunks.len(),
                            "generated SSE events from chunks"
                        );

                        // 保险检查：确保至少有 [DONE]
                        if events.is_empty() {
                            tracing::error!(
                                route = "/v1/responses",
                                chunk_count = chunks.len(),
                                "CRITICAL: No events generated from chunks, using fallback"
                            );
                            let fallback_events = responses_stream_events_from_chat_chunks(&[]);

                            let stream = stream::iter(
                                fallback_events.into_iter().map(Ok::<_, std::convert::Infallible>),
                            );

                            // 使用 KeepAlive 防止连接提前关闭
                            return Sse::new(stream)
                                .keep_alive(KeepAlive::new())
                                .into_response();
                        }

                        // 记录每个事件的内容（前100字符）
                        for (i, event) in events.iter().enumerate() {
                            tracing::debug!(
                                route = "/v1/responses",
                                event_index = i,
                                event_preview = ?event,
                                "SSE event detail"
                            );
                        }

                        let stream = stream::iter(
                            events.into_iter().map(Ok::<_, std::convert::Infallible>),
                        );

                        // 添加 KeepAlive 以防止连接提前关闭
                        Sse::new(stream)
                            .keep_alive(KeepAlive::new())
                            .into_response()
                    }
                    crate::providers::StreamingChat::Streamed { stream, .. } => {
                        let chunks: Vec<_> = stream.collect().await;
                        let events = responses_stream_events_from_chat_chunks(&chunks);
                        tracing::info!(
                            route = "/v1/responses",
                            event_count = events.len(),
                            chunk_count = chunks.len(),
                            "generated SSE events from streamed chunks"
                        );

                        let stream =
                            stream::iter(events.into_iter().map(Ok::<_, std::convert::Infallible>));

                        Sse::new(stream).keep_alive(KeepAlive::new()).into_response()
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/responses",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "provider responses streaming request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    } else {
        // Handle non-streaming
        let mut chat_request = crate::transform::transform_responses_to_chat_request(&body);
        let rewritten_from =
            rewrite_chat_request_model_for_provider(&state, provider.name(), &mut chat_request);
        let effective_model = chat_request.model.clone();
        let transformed_body = serde_json::to_string(&chat_request).ok();
        tracing::debug!(
            route = "/v1/responses",
            provider = provider.name(),
            transformed_model = %effective_model,
            rewritten_from = rewritten_from.as_deref().unwrap_or(""),
            transformed_message_count = chat_request.messages.len(),
            transformed_tool_count = chat_request.tools.as_ref().map(|tools| tools.len()).unwrap_or(0),
            transformed_request_body = transformed_body
                .as_deref()
                .map(format_request_body_for_log)
                .unwrap_or_else(|| "<serialize failed>".to_string()),
            "responses request transformed to chat request"
        );

        match provider.chat(chat_request).await {
            Ok(chat_response) => {
                let responses_response =
                    crate::transform::transform_chat_to_responses_response(&chat_response);
                // 响应体调试日志：记录序列化后大小
                if let Ok(resp_str) = serde_json::to_string(&responses_response) {
                    tracing::debug!(
                        status = %"200",
                        resp_len = resp_str.len(),
                        "responses non-streaming response body"
                    );
                } else {
                    tracing::debug!(
                        status = %"200",
                        "responses non-streaming response body (serialize failed)"
                    );
                }
                Json(responses_response).into_response()
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/responses",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "provider responses request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    }
}
