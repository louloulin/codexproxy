//! MiniMax/MiMo compatibility layer for Chat Completions requests

use crate::models::chat::{ChatRequest, Message};

/// MiniMax compatibility features
#[derive(Debug, Clone, Default)]
pub struct CompatOptions {
    /// Enable all default compatibility features
    pub minimax_compat: bool,
    /// Remove null content from assistant messages
    pub drop_null_content: bool,
    /// Remove tool_choice: "auto"
    pub drop_tool_choice_auto: bool,
    /// Remove stream_options field
    pub drop_stream_options: bool,
    /// Remove parallel_tool_calls field (bool type)
    pub drop_parallel_tool_calls: bool,
    /// Merge multiple system messages into one
    pub merge_system_messages: bool,
    /// Remove response_format field
    pub drop_response_format: bool,
    /// Remove non-function/custom tools
    pub drop_non_function_tools: bool,
}

impl CompatOptions {
    /// Create options with minimax_compat enabled (applies default features)
    pub fn minimax_compat() -> Self {
        Self {
            minimax_compat: true,
            drop_null_content: true,
            drop_tool_choice_auto: true,
            merge_system_messages: true,
            ..Default::default()
        }
    }

    fn is_on(&self, feature: &str) -> bool {
        match feature {
            "drop_null_content" => self.drop_null_content || self.minimax_compat,
            "drop_tool_choice_auto" => self.drop_tool_choice_auto || self.minimax_compat,
            "drop_stream_options" => self.drop_stream_options,
            "drop_parallel_tool_calls" => self.drop_parallel_tool_calls,
            "merge_system_messages" => self.merge_system_messages || self.minimax_compat,
            "drop_response_format" => self.drop_response_format,
            "drop_non_function_tools" => self.drop_non_function_tools,
            _ => false,
        }
    }

    fn is_any_on(&self) -> bool {
        self.drop_null_content
            || self.drop_tool_choice_auto
            || self.drop_stream_options
            || self.drop_parallel_tool_calls
            || self.merge_system_messages
            || self.drop_response_format
            || self.drop_non_function_tools
    }
}

/// Apply MiniMax compatibility transformations to a ChatRequest
pub fn apply_compat(chat: &mut ChatRequest, opts: &CompatOptions) {
    if !opts.minimax_compat && !opts.is_any_on() {
        return;
    }

    if opts.is_on("drop_null_content") {
        drop_null_content(chat);
    }

    if opts.is_on("drop_tool_choice_auto") {
        drop_tool_choice_auto(chat);
    }

    if opts.is_on("drop_stream_options") {
        chat.stream_options = None;
    }

    if opts.is_on("drop_parallel_tool_calls") {
        chat.parallel_tool_calls = false;
    }

    if opts.is_on("merge_system_messages") {
        merge_system_messages(&mut chat.messages);
    }

    if opts.is_on("drop_response_format") {
        chat.response_format = None;
    }

    if opts.is_on("drop_non_function_tools") {
        drop_non_function_tools(chat);
    }
}

fn drop_null_content(_chat: &mut ChatRequest) {}

fn drop_tool_choice_auto(chat: &mut ChatRequest) {
    if let Some(ref choice) = chat.tool_choice {
        if choice.is_auto() {
            chat.tool_choice = None;
        }
    }
}

fn merge_system_messages(messages: &mut Vec<Message>) {
    if messages.is_empty() {
        return;
    }

    let mut system_contents: Vec<String> = Vec::new();
    let mut non_system_messages: Vec<Message> = Vec::new();

    for msg in messages.drain(..) {
        if msg.role == "system" {
            if let Some(ref content) = msg.content {
                if !content.is_empty() {
                    system_contents.push(content.clone());
                }
            }
        } else {
            non_system_messages.push(msg);
        }
    }

    if !system_contents.is_empty() {
        let merged_content = system_contents.join("\n\n");
        let system_msg = Message {
            role: "system".to_string(),
            content: Some(merged_content),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        };

        let mut final_messages = vec![system_msg];
        final_messages.extend(non_system_messages);
        *messages = final_messages;
    } else {
        *messages = non_system_messages;
    }
}

fn drop_non_function_tools(chat: &mut ChatRequest) {
    if let Some(ref mut tools) = chat.tools {
        tools.retain(|tool| {
            tool.tool_type == "function" || tool.tool_type == "custom"
        });

        if tools.is_empty() {
            chat.tools = None;
        }
    }
}

trait IsAuto {
    fn is_auto(&self) -> bool;
}

impl IsAuto for crate::models::chat::ToolChoice {
    fn is_auto(&self) -> bool {
        match self {
            crate::models::chat::ToolChoice::String(s) => s == "auto",
            crate::models::chat::ToolChoice::Tool(tc) => tc.choice_type == "auto",
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_chat(tool_choice: Option<crate::models::chat::ToolChoice>) -> ChatRequest {
        ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            tool_choice,
            ..Default::default()
        }
    }

    impl Default for ChatRequest {
        fn default() -> Self {
            ChatRequest {
                model: "gpt-4".to_string(),
                messages: vec![],
                temperature: None,
                top_p: None,
                max_tokens: None,
                stream: None,
                stop: None,
                n: 1,
                stream_options: None,
                include_usage: None,
                response_format: None,
                seed: None,
                organization: None,
                presence_penalty: None,
                frequency_penalty: None,
                logit_bias: None,
                user: None,
                tools: None,
                tool_choice: None,
                parallel_tool_calls: true,
                reasoning_effort: None,
                thinking: None,
            }
        }
    }

    #[test]
    fn test_drop_tool_choice_auto() {
        let mut chat = create_test_chat(Some(crate::models::chat::ToolChoice::String("auto".to_string())));

        let opts = CompatOptions {
            drop_tool_choice_auto: true,
            ..Default::default()
        };

        apply_compat(&mut chat, &opts);
        assert!(chat.tool_choice.is_none());
    }

    #[test]
    fn test_merge_system_messages() {
        let mut messages = vec![
            Message {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            Message {
                role: "system".to_string(),
                content: Some("System 1".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            Message {
                role: "system".to_string(),
                content: Some("System 2".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
        ];

        merge_system_messages(&mut messages);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.as_ref().unwrap().contains("System 1"));
        assert!(messages[0].content.as_ref().unwrap().contains("System 2"));
    }

    #[test]
    fn test_minimax_compat_options() {
        let opts = CompatOptions::minimax_compat();
        assert!(opts.drop_null_content);
        assert!(opts.drop_tool_choice_auto);
        assert!(opts.merge_system_messages);
    }
}

// Additional tests matching mimo2codex/test/minimaxCompat.test.ts
#[cfg(test)]
mod minimax_compat_tests {
    use super::*;

    #[test]
    fn test_minimax_compat_merges_system_messages() {
        use crate::models::chat::{ChatRequest, Message};

        let mut chat = ChatRequest::default();
        chat.messages = vec![
            Message {
                role: "system".to_string(),
                content: Some("System 1".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            Message {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            Message {
                role: "system".to_string(),
                content: Some("System 2".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
        ];

        let opts = CompatOptions::minimax_compat();
        apply_compat(&mut chat, &opts);

        // Should have 2 messages: one merged system and one user
        let system_count = chat.messages.iter().filter(|m| m.role == "system").count();
        assert_eq!(system_count, 1);

        // System message should contain both contents
        let system_msg = chat.messages.iter().find(|m| m.role == "system").unwrap();
        assert!(system_msg.content.as_ref().unwrap().contains("System 1"));
        assert!(system_msg.content.as_ref().unwrap().contains("System 2"));
    }

    #[test]
    fn test_minimax_compat_drops_tool_choice_auto() {
        use crate::models::chat::{ChatRequest, Message, ToolChoice};

        let mut chat = ChatRequest::default();
        chat.messages = vec![Message {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }];
        chat.tool_choice = Some(ToolChoice::String("auto".to_string()));

        let opts = CompatOptions::minimax_compat();
        apply_compat(&mut chat, &opts);

        // tool_choice should be removed when set to "auto"
        assert!(chat.tool_choice.is_none());
    }

    #[test]
    fn test_minimax_compat_preserves_tool_choice_named() {
        use crate::models::chat::{ChatRequest, Message, ToolChoice};

        let mut chat = ChatRequest::default();
        chat.messages = vec![Message {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }];
        chat.tool_choice = Some(ToolChoice::String("my_function".to_string()));

        let opts = CompatOptions::minimax_compat();
        apply_compat(&mut chat, &opts);

        // Named tool_choice should be preserved
        assert!(chat.tool_choice.is_some());
    }
}
