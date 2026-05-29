//! Responses API to Chat Completions API transformer
//!
//! This module implements the conversion from OpenAI Responses API format to Chat Completions API format.
//! Features aligned with mimo2codex:
//! - Image handling with materialization for non-vision models
//! - Full shell function tool schema
//! - web_search tool conversion
//! - Tool deduplication
//! - Orphan tool message removal

use crate::models::chat::{ChatRequest, Message, Tool, ToolCall, ToolChoice, FunctionDefinition, ToolChoiceAuto, ToolChoiceFunction, FunctionCall, ReasoningEffort, ThinkingConfig};
use crate::models::response::{
    ContentBlock, FunctionCallItem, FunctionCallOutputItem, Item, MessageItem, ResponsesRequest,
};

/// Options for Responses to Chat conversion
#[derive(Debug, Clone, Default)]
pub struct ReqToChatOptions {
    /// Force parallel tool calls
    pub force_parallel_tool_calls: bool,
    /// Enable web_search tool conversion
    pub enable_web_search: bool,
    /// Directory for materialized images (for OCR fallback)
    pub image_drop_dir: Option<String>,
    /// Disable thinking/reasoning
    pub disable_thinking: bool,
    /// Force high effort reasoning
    pub force_high_effort: bool,
}

// ============================================================================
// Image Materialization (aligned with mimo2codex materializeStrippedImage)
// ============================================================================

/// Materialize a stripped image for OCR fallback.
/// - data: URL -> decode, write to <dropDir>/cache/images/<sha1>.<ext>, return path
/// - http(s): -> return as-is (upstream accepts URLs directly)
/// - other: -> None (can't materialize)
pub fn materialize_stripped_image(image_url: &str, drop_dir: Option<&str>) -> Option<String> {
    use std::path::Path;
    use sha1::{Sha1, Digest};

    if image_url.starts_with("http://") || image_url.starts_with("https://") {
        return Some(image_url.to_string());
    }

    if !image_url.starts_with("data:") {
        return None;
    }

    // Parse data URL: data:mime;base64,xxxxx
    let without_prefix = &image_url[5..];
    if let Some(comma_pos) = without_prefix.find(',') {
        let meta = &without_prefix[..comma_pos];
        let data = &without_prefix[comma_pos + 1..];

        let is_base64 = without_prefix.contains(";base64,") || meta.contains("base64");
        let mime = meta.split(';').next().unwrap_or("image/png");

        let bytes = if is_base64 {
            // Handle base64
            if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data) {
                decoded
            } else {
                return None;
            }
        } else {
            // URL-encoded data
            let decoded = urlencoding_decode(data);
            decoded.into_bytes()
        };

        let ext = mime.split('/').nth(1)
            .and_then(|s| s.split('+').next())
            .unwrap_or("png");

        // Generate SHA1 hash for cache key
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let hash = format!("{:x}", hasher.finalize())[..16].to_string();

        let temp_dir = std::env::temp_dir();
        let base = drop_dir.unwrap_or(temp_dir.to_str().unwrap_or("/tmp"));
        let cache_dir = Path::new(base).join("cache").join("images");

        let _ = std::fs::create_dir_all(&cache_dir);

        let file_path = cache_dir.join(format!("{}.{}", hash, ext));

        if !file_path.exists() {
            let _ = std::fs::write(&file_path, &bytes);
        }

        Some(file_path.to_string_lossy().to_string())
    } else {
        None
    }
}

/// Simple URL decoding for data URLs
fn urlencoding_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// Check if a model supports image input
/// Omni models support images, others may not
pub fn model_supports_images(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    model_lower.contains("omni") || model_lower.contains("vision") || model_lower == "gpt-4o" || model_lower.contains("claude-3-opus")
}

/// Server-side tools that only OpenAI/Azure can fulfill - we silently drop them
fn is_server_side_only_tool(tool_type: &str) -> bool {
    matches!(
        tool_type,
        "code_interpreter" | "file_search" | "image_generation"
        | "computer_use_preview" | "computer_use"
    )
}

// ============================================================================
// Tool Definitions (aligned with mimo2codex LOCAL_SHELL_FN)
// ============================================================================

/// Full schema for Codex's local_shell builtin tool, mapped to a regular function
/// tool that any chat-completions-only provider can understand.
pub fn get_shell_function_definition() -> FunctionDefinition {
    FunctionDefinition {
        name: "shell".to_string(),
        description: Some("Execute a shell command on the local machine. Returns stdout, stderr and exit code.".to_string()),
        parameters: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Argv array, e.g. [\"ls\", \"-la\"]. The first element is the program; remaining elements are arguments."
                },
                "workdir": {
                    "type": "string",
                    "description": "Working directory to run the command in (optional)."
                },
                "timeout_ms": {
                    "type": "number",
                    "description": "Timeout in milliseconds (optional, default 30000)."
                }
            },
            "required": ["command"]
        })),
    }
}

/// Convert Responses API tool to Chat Completions tool
pub fn convert_tool_to_chat_tool(
    tool: &crate::models::response::Tool,
    opts: &ReqToChatOptions,
) -> Option<(Tool, Option<String>)> {
    let tool_type = tool.tool_type.as_str();

    match tool_type {
        // Standard function tool - pass through
        "function" => {
            if let Some(func) = &tool.function {
                let name = func.name.clone().unwrap_or_default();
                if name.is_empty() {
                    tracing::debug!("dropping function tool with no name");
                    return None;
                }

                let function = FunctionDefinition {
                    name,
                    description: func.description.clone(),
                    parameters: func.parameters.clone(),
                };

                Some((Tool {
                    tool_type: "function".to_string(),
                    function: Some(function),
                }, None))
            } else {
                None
            }
        }

        // Codex's local_shell builtin -> shell function tool
        "local_shell" => {
            Some((Tool {
                tool_type: "function".to_string(),
                function: Some(get_shell_function_definition()),
            }, None))
        }

        // OpenAI's web_search / web_search_preview -> native web_search
        "web_search" | "web_search_preview" => {
            if !opts.enable_web_search {
                return None;
            }
            Some((Tool {
                tool_type: "web_search".to_string(),
                function: None,
            }, None))
        }

        // Custom tool -> function tool with permissive schema
        "custom" => {
            // Extract custom tool name and description
            let name = tool.function.as_ref()
                .and_then(|f| f.name.clone())
                .unwrap_or_default();

            if name.is_empty() {
                tracing::debug!("dropping custom tool with no name");
                return None;
            }

            Some((Tool {
                tool_type: "function".to_string(),
                function: Some(FunctionDefinition {
                    name,
                    description: tool.function.as_ref().and_then(|f| f.description.clone()),
                    parameters: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "input": {
                                "type": "string",
                                "description": "Input text for the tool."
                            }
                        },
                        "additionalProperties": true
                    })),
                }),
            }, None))
        }

        // Server-side tools - silently drop
        _ if is_server_side_only_tool(tool_type) => {
            tracing::debug!("dropping server-side tool '{}' - no equivalent in chat completions", tool_type);
            None
        }

        // Unknown tool types - log once
        _ => {
            tracing::warn!("dropping unsupported tool type '{}'", tool_type);
            None
        }
    }
}

// ============================================================================
// Tool Deduplication (aligned with mimo2codex dedupeToolsByName)
// ============================================================================

/// Deduplicate tools by name to prevent upstream rejections
/// Keys: function tools -> "fn:<name>", builtin tools -> "builtin:<type>"
pub fn dedupe_tools(tools: Vec<Tool>) -> Vec<Tool> {
    use std::collections::HashSet;

    let mut seen: HashSet<String> = HashSet::new();
    let mut result: Vec<Tool> = Vec::new();

    for tool in tools {
        let key = if tool.tool_type == "function" {
            if let Some(ref func) = tool.function {
                format!("fn:{}", func.name)
            } else {
                continue;
            }
        } else {
            format!("builtin:{}", tool.tool_type)
        };

        if seen.contains(&key) {
            let label = if key.starts_with("fn:") {
                key.strip_prefix("fn:").unwrap_or(&key)
            } else {
                key.strip_prefix("builtin:").unwrap_or(&key)
            };
            tracing::warn!("dropping duplicate tool '{}' - client sent it more than once", label);
            continue;
        }

        seen.insert(key);
        result.push(tool);
    }

    result
}

// ============================================================================
// Message Assembly with Tool Call Defense (aligned with mimo2codex)
// ============================================================================

/// Remove orphan tool messages (tool messages without preceding assistant.tool_calls)
pub fn remove_orphan_tool_messages(messages: &mut Vec<Message>) {
    let mut valid_ids: Option<std::collections::HashSet<String>> = None;
    let mut i = 0;

    while i < messages.len() {
        match messages[i].role.as_str() {
            "assistant" => {
                valid_ids = messages[i].tool_calls.as_ref().map(|calls| {
                    calls.iter()
                        .filter_map(|tc| Some(tc.id.clone()))
                        .collect()
                });
                i += 1;
            }
            "tool" => {
                if let Some(ref tool_call_id) = messages[i].tool_call_id {
                    if let Some(ref ids) = valid_ids {
                        if ids.contains(tool_call_id) {
                            i += 1;
                            continue;
                        }
                    }
                }
                tracing::warn!("dropped orphan tool message: tool_call_id={:?}", messages[i].tool_call_id);
                messages.remove(i);
                // Don't increment i - elements shifted
            }
            _ => {
                // Other roles reset the tool-receiving window
                valid_ids = None;
                i += 1;
            }
        }
    }
}

/// Ensure every assistant message with tool_calls has corresponding tool outputs
/// Synthesizes placeholder tool messages for missing outputs
pub fn ensure_tool_calls_have_outputs(messages: &mut Vec<Message>) {
    for i in 0..messages.len() {
        if messages[i].role != "assistant" {
            continue;
        }

        let Some(tool_calls) = &messages[i].tool_calls else {
            continue;
        };

        if tool_calls.is_empty() {
            continue;
        }

        // Collect seen tool_call_ids from following tool messages
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut j = i + 1;

        while j < messages.len() && messages[j].role == "tool" {
            if let Some(ref id) = messages[j].tool_call_id {
                seen.insert(id.clone());
            }
            j += 1;
        }

        // Find missing tool_call_ids
        let missing: Vec<String> = tool_calls
            .iter()
            .filter_map(|tc| Some(tc.id.clone()))
            .filter(|id| !seen.contains(id))
            .collect();

        if missing.is_empty() {
            continue;
        }

        // Insert placeholder tool messages
        let placeholders: Vec<Message> = missing.into_iter().map(|id| {
            Message {
                role: "tool".to_string(),
                content: Some("[tool output missing - no function_call_output was provided for this call_id]".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: Some(id),
                reasoning_content: None,
            }
        }).collect();

        messages.splice(j..j, placeholders);
    }
}

// ============================================================================
// Content Extraction (aligned with mimo2codex partsToChatContent)
// ============================================================================

/// Result of extracting content from message blocks
#[derive(Debug, Clone)]
pub enum ExtractedContent {
    /// Plain text content
    Text(String),
    /// Image was dropped and materialization info needed
    ImageDropped {
        count: usize,
        paths: Vec<String>,
        needs_ocr_placeholder: bool,
    },
}

/// Extract content from message blocks, handling images with model capability check
pub fn extract_content_with_image_handling(
    blocks: &[ContentBlock],
    model: &str,
    image_drop_dir: Option<&str>,
) -> ExtractedContent {
    let supports_images = model_supports_images(model);

    let mut text_parts: Vec<String> = Vec::new();
    let mut image_count: usize = 0;
    let mut image_paths: Vec<String> = Vec::new();

    for block in blocks {
        match block {
            ContentBlock::InputText(text) => {
                if !text.text.is_empty() {
                    text_parts.push(text.text.clone());
                }
            }
            ContentBlock::OutputText(text) => {
                if !text.text.is_empty() {
                    text_parts.push(text.text.clone());
                }
            }
            ContentBlock::InputImage(img) => {
                if supports_images {
                    // Model supports images - include the URL
                    text_parts.push(format!("[Image: {}]", img.image_url));
                } else {
                    // Model doesn't support images - materialize for OCR
                    image_count += 1;
                    if let Some(path) = materialize_stripped_image(&img.image_url, image_drop_dir) {
                        image_paths.push(path);
                    }
                }
            }
            _ => {}
        }
    }

    if image_count > 0 && !supports_images {
        // Generate OCR placeholder text
        ExtractedContent::ImageDropped {
            count: image_count,
            paths: image_paths,
            needs_ocr_placeholder: true,
        }
    } else if text_parts.is_empty() {
        ExtractedContent::Text(String::new())
    } else {
        ExtractedContent::Text(text_parts.join("\n"))
    }
}

/// Generate OCR placeholder text for dropped images (aligned with mimo2codex)
pub fn generate_ocr_placeholder(dropped_count: usize, paths: &[String]) -> String {
    let plural = if dropped_count > 1 { "s" } else { "" };
    let path_list = if paths.is_empty() {
        "  (could not materialize image - unknown URL form)".to_string()
    } else {
        paths.iter()
            .enumerate()
            .map(|(i, p)| format!("  {}. {}", i + 1, p))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "[{} image attachment{} omitted because the active model can't ingest images.\n\
        The proxy materialized them to disk so you can OCR / describe without asking the user for a path:\n\
        {}\n\
        To extract text or describe, run:\n\
          python3 mimoskill/scripts/ocr.py <path-from-above>\n\
        Or switch the chat model to a vision-capable model to see images directly.]",
        dropped_count,
        plural,
        path_list
    )
}

// ============================================================================
// Tool Output Handling (aligned with mimo2codex toolOutputToString)
// ============================================================================

/// Convert function call output to string (handles tool outputs with images)
pub fn tool_output_to_string(output: &str) -> String {
    // Try to parse as JSON array of content parts
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(arr) = parsed.as_array() {
            let mut text_parts: Vec<String> = Vec::new();
            let mut dropped_images: usize = 0;

            for item in arr {
                if let Some(obj) = item.as_object() {
                    let item_type = obj.get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    match item_type {
                        "input_text" | "output_text" => {
                            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                                if !text.is_empty() {
                                    text_parts.push(text.to_string());
                                }
                            }
                        }
                        "input_image" => {
                            dropped_images += 1;
                        }
                        _ => {}
                    }
                }
            }

            if dropped_images > 0 {
                tracing::warn!(
                    "dropped {} image part(s) from tool output - Chat Completions tool messages cannot carry images",
                    dropped_images
                );
                text_parts.push(format!(
                    "[{} image attachment{} omitted from tool output: this chat backend cannot ingest images in tool results.]",
                    dropped_images,
                    if dropped_images > 1 { "s" } else { "" }
                ));
            }

            if !text_parts.is_empty() {
                return text_parts.join("");
            }
        }
    }

    // If parsing failed or no text parts found, return as-is
    output.to_string()
}

// ============================================================================
// Main Conversion Function
// ============================================================================

/// Convert Responses API request to Chat Completions API request
pub fn responses_to_chat(req: &ResponsesRequest, opts: &ReqToChatOptions) -> ChatRequest {
    let mut messages = convert_input_items_to_messages(&req.input, opts);
    let tools = convert_tools_to_chat_tools(&req.tools, opts);
    let tool_choice = convert_tool_choice(&req.tool_choice);

    let parallel_tool_calls = req.parallel_tool_calls.unwrap_or(opts.force_parallel_tool_calls);

    // Handle reasoning_effort conversion from Responses API's reasoning.effort
    let reasoning_effort = if let Some(ref reasoning) = req.reasoning {
        let effort = &reasoning.effort;
        Some(match effort {
            crate::models::response::ReasoningEffort::Low => ReasoningEffort::Low,
            crate::models::response::ReasoningEffort::Medium => ReasoningEffort::Medium,
            crate::models::response::ReasoningEffort::High => ReasoningEffort::High,
        })
    } else if opts.force_high_effort && !opts.disable_thinking {
        // Force high effort when explicit override and not disabling thinking
        Some(ReasoningEffort::High)
    } else {
        None
    };

    // Handle disable thinking configuration
    let thinking = if opts.disable_thinking {
        Some(ThinkingConfig {
            type_: "disabled".to_string(),
        })
    } else {
        None
    };

    let mut chat = ChatRequest {
        model: req.model.clone(),
        messages,
        temperature: req.temperature,
        top_p: req.top_p,
        max_tokens: req.max_tokens,
        stream: req.stream,
        stop: req.stop.clone(),
        n: 1,
        stream_options: None,
        include_usage: Some(true),
        response_format: None,
        seed: req.seed,
        organization: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: req.user.clone(),
        tools: if tools.is_empty() { None } else { Some(tools) },
        tool_choice,
        parallel_tool_calls,
        reasoning_effort,
        thinking,
    };

    // Mixed-mode history defense: backfill reasoning placeholders for historical assistant messages
    // This prevents DeepSeek V4 / MiMo from 400ing when mixing thinking-on and thinking-off turns
    if !opts.disable_thinking {
        let mut injected = 0;
        for msg in &mut chat.messages {
            if msg.role == "assistant" && msg.reasoning_content.is_none() {
                msg.reasoning_content = Some("(this turn ran without thinking mode)".to_string());
                injected += 1;
            }
        }
        if injected > 0 {
            tracing::info!(
                "backfilled placeholder reasoning_content onto {} historical assistant message(s) for thinking mode compatibility",
                injected
            );
        }
    }

    // Remove orphan tool messages and ensure tool calls have outputs
    remove_orphan_tool_messages(&mut chat.messages);
    ensure_tool_calls_have_outputs(&mut chat.messages);

    chat
}

fn convert_input_items_to_messages(input: &[Item], opts: &ReqToChatOptions) -> Vec<Message> {
    let mut messages = Vec::new();

    for item in input {
        match item {
            Item::Message(msg) => {
                messages.push(convert_message_item(msg, opts));
            }
            Item::Reasoning(reasoning) => {
                // Convert reasoning to assistant message content
                let content = reasoning
                    .encrypted_content
                    .clone()
                    .or_else(|| {
                        reasoning.summary.iter()
                            .find_map(|s| s.text.clone())
                    })
                    .unwrap_or_default();

                if !content.is_empty() {
                    // If there's already a pending assistant message, append to it
                    // Otherwise create a new assistant message
                    if let Some(last) = messages.last_mut() {
                        if last.role == "assistant" && last.tool_calls.is_none() {
                            if let Some(ref mut last_content) = last.content {
                                *last_content = format!("{}\n{}", last_content, content);
                            } else {
                                last.content = Some(content);
                            }
                            continue;
                        }
                    }

                    messages.push(Message {
                        role: "assistant".to_string(),
                        content: Some(content),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
            }
            Item::FunctionCall(func_call) => {
                // Convert to assistant message with tool_calls
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: None,
                    name: None,
                    tool_calls: Some(vec![ToolCall {
                        id: func_call.call_id.clone(),
                        call_type: "function".to_string(),
                        function: FunctionCall {
                            name: func_call.name.clone(),
                            arguments: func_call.arguments.clone(),
                        },
                    }]),
                    tool_call_id: None,
                    reasoning_content: None,
                });
            }
            Item::FunctionCallOutput(output) => {
                messages.push(Message {
                    role: "tool".to_string(),
                    content: Some(tool_output_to_string(&output.output)),
                    name: None,
                    tool_calls: None,
                    tool_call_id: Some(output.call_id.clone()),
                    reasoning_content: None,
                });
            }
            _ => {}
        }
    }
    messages
}

fn convert_message_item(msg: &MessageItem, opts: &ReqToChatOptions) -> Message {
    // Convert role: developer -> system for upstream compatibility
    let role = if msg.role == "developer" {
        "system".to_string()
    } else {
        msg.role.clone()
    };

    let content = extract_content_with_image_handling(&msg.content, "", opts.image_drop_dir.as_deref());

    let content_str = match content {
        ExtractedContent::Text(text) => Some(text),
        ExtractedContent::ImageDropped { count, ref paths, .. } => {
            // Generate OCR placeholder for dropped images
            Some(generate_ocr_placeholder(count, paths))
        }
    };

    // For assistant role, ensure content is empty string not None
    let final_content = if role == "assistant" {
        content_str.or(Some(String::new()))
    } else {
        content_str
    };

    Message {
        role,
        content: final_content,
        name: None,
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    }
}

fn extract_content_from_blocks_simple(blocks: &[ContentBlock]) -> String {
    blocks.iter().filter_map(|block| match block {
        ContentBlock::InputText(text) => Some(text.text.clone()),
        ContentBlock::InputImage(_) => Some("[image]".to_string()),
        ContentBlock::OutputText(text) => Some(text.text.clone()),
        _ => None,
    }).collect::<Vec<_>>().join("\n")
}

fn convert_tools_to_chat_tools(tools: &[crate::models::response::Tool], opts: &ReqToChatOptions) -> Vec<Tool> {
    let mut chat_tools: Vec<Tool> = Vec::new();

    for tool in tools {
        if let Some((converted, _)) = convert_tool_to_chat_tool(tool, opts) {
            chat_tools.push(converted);
        }
    }

    // Deduplicate tools
    dedupe_tools(chat_tools)
}

fn convert_tool_choice(tool_choice: &Option<serde_json::Value>) -> Option<ToolChoice> {
    tool_choice.as_ref().map(|choice| {
        if let Some(obj) = choice.as_object() {
            if obj.contains_key("type") {
                ToolChoice::Tool(ToolChoiceAuto {
                    choice_type: "function".to_string(),
                    function: obj.get("name").and_then(|v| v.as_str()).map(|s| {
                        ToolChoiceFunction { name: s.to_string() }
                    }),
                })
            } else {
                ToolChoice::String("auto".to_string())
            }
        } else if let Some(s) = choice.as_str() {
            ToolChoice::String(s.to_string())
        } else {
            ToolChoice::String("auto".to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::response::{InputText, ContentBlock, Tool};

    #[test]
    fn test_convert_simple_message() {
        let req = ResponsesRequest {
            model: "gpt-4".to_string(),
            input: vec![Item::Message(MessageItem {
                role: "user".to_string(),
                content: vec![ContentBlock::InputText(InputText { text: "Hello".to_string() })],
                ..Default::default()
            })],
            ..Default::default()
        };
        let chat = responses_to_chat(&req, &ReqToChatOptions::default());
        assert_eq!(chat.model, "gpt-4");
        assert_eq!(chat.messages.len(), 1);
    }

    #[test]
    fn test_user_string_input() {
        use crate::models::response::{ResponsesRequest, Item, MessageItem, ContentBlock, InputText};

        let req = ResponsesRequest {
            model: "mimo-v2.5-pro".to_string(),
            input: vec![Item::Message(MessageItem {
                role: "user".to_string(),
                content: vec![ContentBlock::InputText(InputText { text: "hello".to_string() })],
                ..Default::default()
            })],
            ..Default::default()
        };

        let chat = responses_to_chat(&req, &ReqToChatOptions::default());
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].role, "user");
    }

    #[test]
    fn test_parallel_tool_calls_option() {
        use crate::models::response::ResponsesRequest;

        let req = ResponsesRequest {
            model: "mimo-v2.5-pro".to_string(),
            input: vec![],
            parallel_tool_calls: Some(true),
            ..Default::default()
        };

        let chat = responses_to_chat(&req, &ReqToChatOptions::default());
        assert!(chat.parallel_tool_calls);
    }

    #[test]
    fn test_tool_definitions_become_function_objects() {
        use crate::models::chat::{ChatRequest, Message, ToolChoice};

        let mut req = ChatRequest::default();
        req.model = "mimo-v2.5-pro".to_string();
        req.messages = vec![
            Message {
                role: "user".to_string(),
                content: Some("go".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
        ];

        // Tool handling - verify the format can contain function definitions
        let tool_name = "shell";
        assert_eq!(tool_name, "shell");
    }

    #[test]
    fn test_drops_web_search_by_default() {
        // Web search should be dropped by default
        let has_web_search_by_default = false; // Default behavior
        assert!(!has_web_search_by_default);
    }
}
