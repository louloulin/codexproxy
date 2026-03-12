# OpenAI Responses API 使用示例

本文档展示如何使用 `/v1/responses` 端点调用 OpenAI Responses API。

## 前提条件

1. 服务已启动在 `http://localhost:8080`
2. 配置文件中已设置有效的 OpenAI API Key

## API 端点

- **Chat Completions API**: `POST /v1/chat/completions`
- **Responses API**: `POST /v1/responses`

## 关键格式差异

### Responses API 格式
- 使用 `input` 字段（数组）
- `content` 是 `ContentBlock` 数组
- 文本内容使用 `{ "type": "input_text", "text": "..." }`
- 系统指令使用 `instructions` 字段

### Chat Completions API 格式
- 使用 `messages` 字段（数组）
- `content` 是字符串
- 系统消息使用 `{ "role": "system", "content": "..." }`

## 示例 1: 非流式 Responses API 调用

```bash
curl -X POST "http://localhost:8080/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "Hello! What is 2+2?"
          }
        ]
      }
    ],
    "stream": false
  }'
```

**响应示例:**
```json
{
  "id": "resp-abc123",
  "object": "response",
  "created": 1234567890,
  "model": "gpt-4o",
  "output": [
    {
      "type": "message",
      "role": "assistant",
      "content": [
        {
          "type": "output_text",
          "text": "2+2 equals 4."
        }
      ]
    }
  ],
  "usage": {
    "input_tokens": 15,
    "output_tokens": 10,
    "total_tokens": 25
  }
}
```

## 示例 2: 流式 Responses API 调用

```bash
curl -X POST "http://localhost:8080/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "Count from 1 to 5"
          }
        ]
      }
    ],
    "stream": true
  }'
```

**流式响应示例:**
```
data: {"id":"resp-abc123","object":"response.chunk","created":1234567890,"model":"gpt-4o","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"1"}]}]}

data: {"id":"resp-abc123","object":"response.chunk","created":1234567890,"model":"gpt-4o","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"\n2"}]}]}

data: [DONE]
```

## 示例 3: 带系统指令的请求

```bash
curl -X POST "http://localhost:8080/v1/responses" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "instructions": "You are a helpful assistant that responds in haiku format.",
    "input": [
      {
        "type": "message",
        "role": "user",
        "content": [
          {
            "type": "input_text",
            "text": "What is programming?"
          }
        ]
      }
    ],
    "stream": false
  }'
```

## 示例 4: Chat Completions API 对比

传统的 Chat Completions API:

```bash
curl -X POST "http://localhost:8080/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {
        "role": "user",
        "content": "Hello!"
      }
    ],
    "stream": false
  }'
```

## Transform 层说明

本服务实现了 Chat Completions API 和 Responses API 之间的双向转换:

- `/v1/responses` 接收 Responses 格式 → 转换为 Chat 格式 → 调用 OpenAI → 转换回 Responses 格式
- `/v1/chat/completions` 接收 Chat 格式 → 调用 OpenAI → 通过 Responses 转换后返回

### 转换函数

1. `transform_responses_to_chat_request` - Responses 请求 → Chat 请求
2. `transform_chat_to_responses_response` - Chat 响应 → Responses 响应
3. `transform_chat_to_responses_request` - Chat 请求 → Responses 请求
4. `transform_responses_to_chat_response` - Responses 响应 → Chat 响应
5. `transform_chat_stream_to_responses_stream` - Chat 流 → Responses 流
6. `transform_responses_stream_to_chat_stream` - Responses 流 → Chat 流

## 运行测试

验证 Transform 层功能:

```bash
# 运行所有 transform 相关测试
cargo test transform

# 运行完整测试套件
cargo test
```

## 启动服务

```bash
# 使用默认配置
./target/release/openai-proxy

# 使用自定义配置
./target/release/openai-proxy --config config.test.yaml
```

## 注意事项

1. Responses API 是 OpenAI 的新 API 格式，用于替代已弃用的 Assistants API
2. `input` 字段是 Item 数组，支持 `message` 和 `reasoning` 等类型
3. `content` 是 ContentBlock 数组，支持 `input_text`、`output_text`、`image` 等类型
4. 系统指令使用 `instructions` 字段，而不是单独的消息类型
5. 流式响应使用 Server-Sent Events (SSE) 格式
6. 所有请求都会经过 Transform 层进行格式转换