//! Responses API to Chat Completions API transformer

use crate::models::chat::{ChatRequest, Message, Tool, ToolCall, ToolChoice, FunctionDefinition, ToolChoiceAuto, ToolChoiceFunction};
use crate::models::response::{
    ContentBlock, FunctionCallItem, FunctionCallOutputItem, Item, MessageItem, ResponsesRequest,
};

/// Options for Responses to Chat conversion
#[derive(Debug, Clone, Default)]
pub struct ReqToChatOptions {
    pub force_parallel_tool_calls: bool,
    pub enable_web_search: bool,
    pub image_drop_dir: Option<String>,
    pub disable_thinking: bool,
    pub force_high_effort: bool,
}

/// Convert Responses API request to Chat Completions API request
pub fn responses_to_chat(req: &ResponsesRequest, opts: &ReqToChatOptions) -> ChatRequest {
    let messages = convert_input_items_to_messages(&req.input);
    let tools = convert_tools_to_chat_tools(&req.tools, opts);
    let tool_choice = convert_tool_choice(&req.tool_choice);
    
    let parallel_tool_calls = req.parallel_tool_calls.unwrap_or(opts.force_parallel_tool_calls);

    ChatRequest {
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
    }
}

fn convert_input_items_to_messages(input: &[Item]) -> Vec<Message> {
    let mut messages = Vec::new();
    
    for item in input {
        match item {
            Item::Message(msg) => {
                messages.push(convert_message_item(msg));
            }
            Item::FunctionCall(func_call) => {
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: None,
                    name: None,
                    tool_calls: Some(vec![ToolCall {
                        id: func_call.call_id.clone(),
                        call_type: "function".to_string(),
                        function: crate::models::chat::FunctionCall {
                            name: func_call.name.clone(),
                            arguments: func_call.arguments.clone(),
                        },
                    }]),
                    tool_call_id: None,
                });
            }
            Item::FunctionCallOutput(output) => {
                messages.push(Message {
                    role: "tool".to_string(),
                    content: Some(output.output.clone()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: Some(output.call_id.clone()),
                });
            }
            _ => {}
        }
    }
    messages
}

fn convert_message_item(msg: &MessageItem) -> Message {
    let content = extract_content_from_blocks(&msg.content);
    Message {
        role: msg.role.clone(),
        content: Some(content),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }
}

fn extract_content_from_blocks(blocks: &[ContentBlock]) -> String {
    blocks.iter().filter_map(|block| match block {
        ContentBlock::InputText(text) => Some(text.text.clone()),
        ContentBlock::InputImage(_) => Some("[image]".to_string()),
        _ => None,
    }).collect::<Vec<_>>().join("\n")
}

fn convert_tools_to_chat_tools(tools: &[crate::models::response::Tool], opts: &ReqToChatOptions) -> Vec<Tool> {
    tools.iter().filter_map(|tool| {
        match tool.tool_type.as_str() {
            "function" => {
                tool.function.as_ref().map(|func| {
                    Tool {
                        tool_type: "function".to_string(),
                        function: Some(FunctionDefinition {
                            name: func.name.clone().unwrap_or_default(),
                            description: func.description.clone(),
                            parameters: func.parameters.clone(),
                        }),
                    }
                })
            }
            "web_search" | "web_search_preview" => {
                if opts.enable_web_search {
                    Some(Tool { tool_type: tool.tool_type.clone(), function: None })
                } else { None }
            }
            "local_shell" => {
                Some(Tool {
                    tool_type: "function".to_string(),
                    function: Some(FunctionDefinition {
                        name: "shell".to_string(),
                        description: Some("Execute shell command".to_string()),
                        parameters: Some(serde_json::json!({
                            "type": "object",
                            "properties": { "command": { "type": "string", "description": "The shell command to execute" } },
                            "required": ["command"]
                        })),
                    }),
                })
            }
            _ => Some(Tool { tool_type: tool.tool_type.clone(), function: None }),
        }
    }).collect()
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
}
