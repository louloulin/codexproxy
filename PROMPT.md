# OpenAI Chat Completions to Responses API 转换器开发计划

## 项目概述

本项目旨在基于 Rust 实现一个 OpenAI 兼容的 API 代理服务器，将标准的 `/v1/chat/completions` 请求转换为新的 Responses API 格式，同时支持智谱（Zhipu）AI 的 GLM 模型集成。

### 核心目标
1. 实现 Chat Completions → Responses API 的格式转换
2. 支持智谱 AI GLM 系列模型
3. 提供 Codex 可用的 API 服务

---

## 1. 技术背景分析

### 1.1 OpenAI Responses API 与 Chat Completions API 差异

| 特性 | Chat Completions API | Responses API |
|------|---------------------|---------------|
| 端点 | `POST /v1/chat/completions` | `POST /v1/responses` |
| 输入格式 | `messages` (数组) | `items` (联合类型) |
| 输出格式 | `choices` (数组) | `output` (数组) |
| 响应格式 | `response_format` | `text.format` |
| 函数调用 | `tools` / `tool_calls` | 内联函数定义 |
| 推理内容 | 仅 content | 独立的 reasoning items |

#### 消息结构转换

```json
// Chat Completions 请求
{
  "model": "gpt-4o",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "tools": [...],
  "response_format": {"type": "json_object"}
}

// Responses API 请求
{
  "model": "gpt-4o",
  "input": [
    {"type": "message", "role": "system", "content": "You are a helpful assistant."},
    {"type": "message", "role": "user", "content": "Hello!"}
  ],
  "tools": [...],
  "text": {"format": {"type": "json_object"}}
}
```

#### 响应结构转换

```json
// Chat Completions 响应
{
  "id": "chatcmpl-xxx",
  "object": "chat.completion",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello! How can I help?",
      "tool_calls": [...]
    }
  }]
}

// Responses API 响应
{
  "id": "resp_xxx",
  "object": "response",
  "output": [
    {"type": "reasoning", "summary": [...]},
    {"type": "message", "content": [
      {"type": "output_text", "text": "Hello! How can I help?"}
    ]}
  ]
}
```

### 1.2 智谱 AI (Zhipu) API 规范

#### API 端点
```
https://open.bigmodel.cn/api/paas/v4/chat/completions
```

#### 支持模型
- GLM-5
- GLM-4.7
- GLM-4.6V (视觉版本)
- GLM-Image

#### 请求格式 (OpenAI 兼容)
```json
{
  "model": "glm-4",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "temperature": 0.7,
  "max_tokens": 1024,
  "stream": false
}
```

#### 函数调用支持
智谱支持 function calling，格式与 OpenAI 兼容：
```json
{
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get current weather",
        "parameters": {
          "type": "object",
          "properties": {
            "location": {"type": "string", "description": "City name"}
          },
          "required": ["location"]
        }
      }
    }
  ]
}
```

---

## 2. 系统架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                        客户端请求                            │
│  POST /v1/chat/completions                                  │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    Axum Web Server                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Request Transform Layer                 │   │
│  │  - ChatCompletions → Responses API 格式转换         │   │
│  │  - 请求验证与参数处理                                │   │
│  └────────────────────┬────────────────────────────────┘   │
│                       │                                      │
│  ┌────────────────────▼────────────────────────────────┐   │
│  │              Routing Layer                            │   │
│  │  - /v1/chat/completions (OpenAI 兼容)               │   │
│  │  - /v1/responses (Responses API)                   │   │
│  │  - /v1/providers/zhipu/* (智谱直连)                 │   │
│  └────────────────────┬────────────────────────────────┘   │
│                       │                                      │
│  ┌────────────────────▼────────────────────────────────┐   │
│  │              LLM Provider Adapter                     │   │
│  │  - OpenAI Adapter                                   │   │
│  │  - Zhipu Adapter                                    │   │
│  │  - 抽象 Provider Trait                               │   │
│  └────────────────────┬────────────────────────────────┘   │
│                       │                                      │
└───────────────────────┼────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│   OpenAI      │ │   智谱 AI     │ │   自定义      │
│   API         │ │   API         │ │   LLM         │
└───────────────┘ └───────────────┘ └───────────────┘
```

### 2.2 核心模块设计

#### 模块结构
```
src/
├── main.rs                 # 入口文件
├── lib.rs                  # 库入口
├── config/
│   └── mod.rs             # 配置管理
├── server/
│   ├── mod.rs             # 服务器模块
│   └── router.rs          # 路由定义
├── handlers/
│   ├── mod.rs             # 处理器模块
│   ├── chat_completions.rs    # Chat Completions 处理
│   ├── responses.rs           # Responses API 处理
│   └── zhipu.rs              # 智谱 API 处理
├── transform/
│   ├── mod.rs             # 转换模块
│   ├── request.rs         # 请求转换
│   └── response.rs        # 响应转换
├── providers/
│   ├── mod.rs             # Provider 模块
│   ├── trait.rs           # Provider Trait 定义
│   ├── openai.rs          # OpenAI Provider
│   └── zhipu.rs           # 智谱 Provider
├── models/
│   ├── mod.rs             # 模型模块
│   ├── chat.rs            # Chat Completions 模型
│   └── response.rs        # Responses API 模型
├── error.rs               # 错误处理
└── logging.rs             # 日志配置
```

### 2.3 关键 Trait 定义

```rust
// providers/trait.rs
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// 获取 provider 名称
    fn name(&self) -> &str;

    /// 获取基础 URL
    fn base_url(&self) -> &str;

    /// 获取 API 密钥
    fn api_key(&self) -> &str;

    /// 获取默认模型
    fn default_model(&self) -> &str;

    /// 检查是否支持流式
    fn supports_stream(&self) -> bool;

    /// 发送聊天请求 (非流式)
    async fn chat(
        &self,
        client: &Client,
        request: ChatRequest,
    ) -> Result<ChatResponse, ProviderError>;

    /// 发送聊天请求 (流式)
    async fn chat_streaming(
        &self,
        client: &Client,
        request: ChatRequest,
    ) -> Result<StreamingResponse, ProviderError>;
}
```

---

## 3. API 转换规范

### 3.1 请求转换 (Chat Completions → Responses API)

#### 必填字段映射
| Chat Completions | Responses API | 说明 |
|----------------|--------------|------|
| `model` | `model` | 直接传递 |
| `messages` | `input` | 转换为 Item 数组 |

#### 可选字段映射
| Chat Completions | Responses API | 说明 |
|----------------|--------------|------|
| `temperature` | `temperature` | 直接传递 |
| `max_tokens` | `max_tokens` | 直接传递 |
| `top_p` | `top_p` | 直接传递 |
| `stream` | `stream` | 直接传递 |
| `tools` | `tools` | 直接传递 |
| `tool_choice` | `tool_choice` | 直接传递 |
| `response_format` | `text.format` | 格式转换 |
| `seed` | `seed` | 直接传递 |
| `stop` | `stop` | 直接传递 |
| `presence_penalty` | `presence_penalty` | 直接传递 |
| `frequency_penalty` | `frequency_penalty` | 直接传递 |

#### Messages 到 Items 转换

```rust
fn transform_messages_to_items(messages: Vec<Message>) -> Vec<Item> {
    messages
        .into_iter()
        .map(|msg| {
            match msg.role.as_str() {
                "system" => Item {
                    item_type: ItemType::Message(MessageItem {
                        role: "system".to_string(),
                        content: vec![ContentBlock {
                            content_type: ContentType::InputText(InputText {
                                text: msg.content.clone(),
                            }),
                        }],
                    }),
                },
                "user" => Item {
                    item_type: ItemType::Message(MessageItem {
                        role: "user".to_string(),
                        content: vec![ContentBlock {
                            content_type: ContentType::InputText(InputText {
                                text: msg.content.clone(),
                            }),
                        }),
                    }),
                },
                "assistant" => Item {
                    item_type: ItemType::Message(MessageItem {
                        role: "assistant".to_string(),
                        content: vec![ContentBlock {
                            content_type: ContentType::InputText(InputText {
                                text: msg.content.unwrap_or_default(),
                            }),
                        }],
                        tool_calls: msg.tool_calls,
                    }),
                },
                "tool" => Item {
                    item_type: ItemType::FunctionCallOutput(FunctionCallOutputItem {
                        call_id: msg.tool_call_id.clone(),
                        output: msg.content.clone(),
                    }),
                },
                _ => unreachable!(),
            }
        })
        .collect()
}
```

### 3.2 响应转换 (Responses API → Chat Completions)

#### 响应字段映射
| Responses API | Chat Completions | 说明 |
|--------------|-----------------|------|
| `id` | `id` | 格式转换为 `chatcmpl-xxx` |
| `object` | `object` | 转换为 `chat.completion` |
| `created` | `created` | 直接传递 |
| `model` | `model` | 直接传递 |
| `output[]` | `choices[]` | 转换输出项 |

#### Output Items 到 Choices 转换

```rust
fn transform_output_to_choices(output: Vec<OutputItem>) -> Vec<Choice> {
    output
        .into_iter()
        .filter_map(|item| {
            match item.output_type.as_str() {
                "message" => {
                    let content = extract_text_from_content(&item.content);
                    Some(Choice {
                        index: item.index,
                        message: Message {
                            role: "assistant".to_string(),
                            content: Some(content),
                            tool_calls: item.tool_calls,
                        },
                        finish_reason: item.status,
                    })
                }
                "reasoning" => {
                    // 推理内容不直接显示在 message 中
                    None
                }
                _ => None,
            }
        })
        .collect()
}

fn extract_text_from_content(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| {
            match &block.content_type {
                ContentType::OutputText(text) => Some(text.text.clone()),
                _ => None,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

## 4. 智谱 API 集成

### 4.1 智谱 Provider 实现

```rust
// providers/zhipu.rs
use super::{LLMProvider, ProviderError};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct ZhipuProvider {
    api_key: String,
    base_url: String,
    default_model: String,
}

impl ZhipuProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
            default_model: "glm-4".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for ZhipuProvider {
    fn name(&self) -> &str {
        "zhipu"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn supports_stream(&self) -> bool {
        true
    }

    async fn chat(
        &self,
        client: &Client,
        request: ChatRequest,
    ) -> Result<ChatResponse, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(ProviderError::RequestFailed)?;

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(ProviderError::InvalidResponse)?;

        Ok(chat_response)
    }

    // ... streaming 实现
}
```

### 4.2 智谱特定配置

```rust
#[derive(Clone, Debug, Deserialize)]
pub struct ZhipuConfig {
    /// API 密钥
    pub api_key: String,

    /// 基础 URL (可选)
    #[serde(default = "default_zhipu_url")]
    pub base_url: String,

    /// 默认模型
    #[serde(default = "default_zhipu_model")]
    pub default_model: String,

    /// 请求超时 (秒)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_zhipu_url() -> String {
    "https://open.bigmodel.cn/api/paas/v4".to_string()
}

fn default_zhipu_model() -> String {
    "glm-4".to_string()
}

fn default_timeout() -> u64 {
    60
}
```

---

## 5. 流式响应处理

### 5.1 SSE 事件格式

```rust
// Chat Completions 流式响应格式
struct StreamingChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<StreamingChoice>,
}

struct StreamingChoice {
    index: u32,
    delta: Delta,
    finish_reason: Option<String>,
}

struct Delta {
    role: Option<String>,
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}
```

### 5.2 Responses API 流式转换

Responses API 流式输出也使用 SSE，格式略有不同：

```rust
// Responses API 流式 chunk
struct ResponsesStreamingChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    output: Vec<StreamingOutputItem>,
}
```

转换逻辑需要：
1. 接收 Responses API 流式 chunks
2. 转换为 Chat Completions 格式
3. 通过 SSE 返回给客户端

---

## 6. 配置管理

### 6.1 配置文件格式 (YAML)

```yaml
# config.yaml
server:
  host: "0.0.0.0"
  port: 8080

providers:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com/v1"
    default_model: "gpt-4o"
    timeout: 120

  zhipu:
    api_key: "${ZHIPU_API_KEY}"
    base_url: "https://open.bigmodel.cn/api/paas/v4"
    default_model: "glm-4"
    timeout: 60

routing:
  # 默认 Provider
  default: "openai"

  # 模型到 Provider 的映射
  model_mapping:
    gpt-4*: "openai"
    gpt-3.5*: "openai"
    glm*: "zhipu"
    *: "openai"

logging:
  level: "info"
  format: "json"
```

### 6.2 环境变量支持

```rust
fn load_env_vars(config: &mut Config) {
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        config.providers.openai.api_key = api_key;
    }
    if let Ok(api_key) = std::env::var("ZHIPU_API_KEY") {
        config.providers.zhipu.api_key = api_key;
    }
}
```

---

## 7. 错误处理

### 7.1 错误类型定义

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("请求错误: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Provider 错误: {0}")]
    Provider(String),

    #[error("转换错误: {0}")]
    Transform(String),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("流式错误: {0}")]
    Streaming(String),
}
```

### 7.2 错误响应格式

```rust
impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            Error::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Request(e) => (StatusCode::BAD_REQUEST, &e.to_string()),
            Error::Provider(msg) => (StatusCode::BAD_GATEWAY, msg),
            Error::Transform(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            Error::Streaming(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = serde_json::json!({
            "error": {
                "message": message,
                "type": "invalid_request_error",
                "code": status.as_u16()
            }
        });

        (status, Json(body)).into_response()
    }
}
```

---

## 8. 实现计划

### Phase 1: 基础架构 (第 1-2 天)
- [ ] 项目初始化与依赖配置
- [ ] 配置管理模块
- [ ] 基础错误处理
- [ ] 日志配置
- [ ] 单元测试框架搭建

### Phase 2: 核心模型 (第 3-4 天)
- [ ] Chat Completions 请求/响应模型
- [ ] Responses API 请求/响应模型
- [ ] 智谱 API 模型
- [ ] 模型序列化/反序列化

### Phase 3: Provider 抽象 (第 5-6 天)
- [ ] LLMProvider Trait 定义
- [ ] OpenAI Provider 实现
- [ ] 智谱 Provider 实现
- [ ] Provider 路由逻辑

### Phase 4: 转换层 (第 7-9 天)
- [ ] 请求转换器 (Chat → Responses)
- [ ] 响应转换器 (Responses → Chat)
- [ ] 流式转换支持
- [ ] 函数调用转换

### Phase 5: API 服务 (第 10-12 天)
- [ ] Axum 服务器搭建
- [ ] `/v1/chat/completions` 端点
- [ ] `/v1/responses` 端点
- [ ] 智谱直连端点
- [ ] 中间件 (CORS, 限流, 日志)

### Phase 6: 测试与优化 (第 13-15 天)
- [ ] 集成测试
- [ ] 流式响应测试
- [ ] 错误处理测试
- [ ] 性能优化
- [ ] 文档编写

---

## 9. 依赖清单

### Cargo.toml

```toml
[package]
name = "openai-proxy"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web 框架
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# HTTP 客户端
reqwest = { version = "0.11", features = ["json", "stream"] }

# 异步运行时
tokio = { version = "1", features = ["full"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 配置
config = "0.14"
serde_yaml = "0.9"

# 错误处理
thiserror = "1.0"
anyhow = "1.0"

# 异步 trait
async-trait = "0.1"

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# 时间
chrono = { version = "0.4", features = ["serde"] }

# 唯一 ID
uuid = { version = "1.0", features = ["v4", "serde"] }

[dev-dependencies]
tower = { version = "0.4", features = ["util"] }
http-body-util = "0.1"
```

---

## 10. API 使用示例

### 10.1 转换请求示例

```bash
# 请求 (Chat Completions 格式)
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "What is the weather in Tokyo?"}
    ],
    "temperature": 0.7
  }'

# 内部转换为 Responses API 格式发送到上游
# {
#   "model": "gpt-4o",
#   "input": [
#     {"type": "message", "role": "system", "content": "You are a helpful assistant."},
#     {"type": "message", "role": "user", "content": "What is the weather in Tokyo?"}
#   ],
#   "temperature": 0.7
# }
```

### 10.2 智谱请求示例

```bash
# 使用智谱模型
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZHIPU_API_KEY" \
  -d '{
    "model": "glm-4",
    "messages": [
      {"role": "user", "content": "你好，请介绍一下自己"}
    ]
  }'

# 或直接调用智谱端点
curl -X POST http://localhost:8080/v1/providers/zhipu/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZHIPU_API_KEY" \
  -d '{
    "model": "glm-4",
    "messages": [
      {"role": "user", "content": "你好"}
    ]
  }'
```

### 10.3 流式请求示例

```bash
# 流式请求
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Tell me a story"}],
    "stream": true
  }'
```

---

## 11. 总结

本开发计划提供了完整的 OpenAI Chat Completions to Responses API 转换器实现方案，包括：

1. **完整的技术背景分析** - 详细对比两种 API 格式的差异
2. **系统架构设计** - 模块化、可扩展的架构
3. **API 转换规范** - 完整的请求/响应转换逻辑
4. **智谱 AI 集成** - 智谱 Provider 的具体实现
5. **实现计划** - 15 天的迭代开发计划

该方案可直接用于 Codex 项目开发，实现生产级别的 API 代理服务。
