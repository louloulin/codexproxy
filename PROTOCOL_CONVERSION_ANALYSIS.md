# Responses API ↔ Chat API 协议转换深度分析

## 📋 目录
1. [协议概述](#协议概述)
2. [问题根源](#问题根源)
3. [修复方案](#修复方案)
4. [协议转换逻辑](#协议转换逻辑)
5. [测试验证](#测试验证)

---

## 协议概述

### Chat Completions API (OpenAI Legacy)

**请求格式**:
```json
{
  "model": "gpt-4",
  "messages": [
    {"role": "system", "content": "You are helpful"},
    {"role": "user", "content": "Hello"}
  ],
  "temperature": 0.7,
  "stream": true,
  "stream_options": {"include_usage": true}
}
```

**流式响应格式** (SSE):
```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}],"usage":null}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}],"usage":null}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}

data: [DONE]
```

### Responses API (OpenAI New)

**请求格式**:
```json
{
  "model": "gpt-4",
  "input": [
    {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "Hello"}]}
  ],
  "instructions": "You are helpful",
  "temperature": 0.7,
  "stream": true
}
```

**流式响应格式** (SSE Semantic Events):
```
data: {"type":"response.created","response":{"id":"resp_123","object":"response","created_at":1234567890,"model":"gpt-4","status":"in_progress"}}

data: {"type":"response.in_progress","response":{"id":"resp_123",...}}

data: {"type":"response.output_item.added","output_index":0,"item":{"type":"message","role":"assistant",...}}

data: {"type":"response.content_part.added","item_id":"msg_123","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}

data: {"type":"response.output_text.delta","item_id":"msg_123","output_index":0,"content_index":0,"delta":"Hello"}

data: {"type":"response.output_text.done","item_id":"msg_123","output_index":0,"content_index":0,"text":"Hello!"}

data: {"type":"response.content_part.done",...}

data: {"type":"response.output_item.done","output_index":0,"item":{...}}

data: {"type":"response.completed","response_id":"resp_123","token_usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}

data: [DONE]
```

---

## 问题根源

### 🔴 问题 1: OpenAI Provider 返回空 chunks

**位置**: `src/providers/openai.rs:189-190` (已修复)

**原始代码**:
```rust
// Return streaming response
Ok(StreamingChat::new(status.as_u16(), vec![]))  // ❌ 空数组！
```

**问题**:
- OpenAI provider 根本没有处理 SSE 流
- 直接返回空 chunks 数组
- 导致 `responses_protocol_payloads_from_chat_chunks` 提前返回，不发送 `response.completed`

**修复后**:
```rust
// Process SSE stream and collect chunks
let mut chunks = Vec::new();
let body = response.bytes().await
    .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

let body_str = String::from_utf8_lossy(&body);

// Parse SSE format
for line in body_str.lines() {
    let line = line.trim();
    if line.starts_with("data: ") {
        let data = &line[6..];
        if data == "[DONE]" {
            break;
        }
        if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
            chunks.push(chunk);
        }
    }
}

Ok(StreamingChat::new(status.as_u16(), chunks))
```

### 🔴 问题 2: 空 chunks 导致 `response.completed` 未发送

**位置**: `src/handlers/mod.rs:123-126` (已修复)

**原始代码**:
```rust
fn responses_protocol_payloads_from_chat_chunks(chunks: &[ChatCompletionChunk]) -> Vec<String> {
    if chunks.is_empty() {
        return Vec::new();  // ❌ 直接返回，不发送 response.completed
    }
    ...
}
```

**问题**:
- 如果 chunks 为空，直接返回空数组
- 不会发送 `response.completed` 事件
- 客户端等待超时，报错 "stream closed before response.completed"

**修复后**:
```rust
fn responses_protocol_payloads_from_chat_chunks(chunks: &[ChatCompletionChunk]) -> Vec<String> {
    if chunks.is_empty() {
        tracing::warn!("responses_protocol_payloads_from_chat_chunks called with empty chunks");
        // Still send response.completed event even with empty chunks
        return vec![
            serde_json::to_string(&json!({
                "type": "response.completed",
                "response_id": "resp_empty",
                "token_usage": null
            }))
            .unwrap_or_default(),
            "[DONE]".to_string(),
        ];
    }
    ...
}
```

### 🟡 问题 3: Usage 字段解析失败

**位置**: `src/models/response.rs:1311`, `src/models/chat.rs:316` (已修复)

**原始代码**:
```rust
pub struct Usage {
    pub input_tokens: u64,  // ❌ 必填字段，解析失败
    pub output_tokens: u64,
    pub total_tokens: u64,
}
```

**修复后**:
```rust
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,  // ✅ 可选，默认为 0

    #[serde(default)]
    pub output_tokens: u64,

    #[serde(default)]
    pub total_tokens: u64,
}
```

---

## 协议转换逻辑

### 转换流程图

```
┌─────────────────────────────────────────────────────────────────┐
│                      客户端请求 (Responses API)                  │
│  POST /v1/responses                                              │
│  {                                                               │
│    "model": "gpt-4",                                             │
│    "input": [...],                                               │
│    "stream": true                                                │
│  }                                                               │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│            1. Responses → Chat 请求转换                          │
│  transform_responses_to_chat_request()                           │
│  • input → messages                                              │
│  • instructions → system message                                 │
│  • tools → tools (with type mapping)                             │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│            2. 调用 Provider (Chat API)                           │
│  provider.chat_streaming(chat_request)                           │
│  • OpenAI Provider: 处理 SSE 流，收集 chunks                     │
│  • Zhipu Provider: 处理 SSE 流，收集 chunks                      │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│            3. Chat → Responses 流式转换                          │
│  responses_protocol_payloads_from_chat_chunks()                  │
│  • 生成 response.created 事件                                    │
│  • 生成 response.in_progress 事件                                │
│  • 为每个 delta 生成 response.output_text.delta                  │
│  • 为每个 finish_reason 生成 response.output_item.done           │
│  • 生成 response.completed 事件 (包含 token_usage)               │
│  • 添加 [DONE] 标记                                              │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                返回给客户端 (Responses API SSE)                  │
│  data: {"type":"response.created",...}                           │
│  data: {"type":"response.output_text.delta",...}                 │
│  data: {"type":"response.completed",...}                         │
│  data: [DONE]                                                    │
└─────────────────────────────────────────────────────────────────┘
```

### 关键转换函数

#### 1. `transform_responses_to_chat_request`
**文件**: `src/transform/mod.rs:163-431`

**功能**: Responses API 请求 → Chat API 请求

**转换规则**:
- `instructions` → `messages[role="system"]`
- `input[type="message"]` → `messages[role=user/assistant]`
- `input[type="function_call"]` → `messages[tool_calls]`
- `input[type="function_call_output"]` → `messages[role="tool"]`
- `tools[type="function"]` → `tools[function={...}]`

#### 2. `responses_protocol_payloads_from_chat_chunks`
**文件**: `src/handlers/mod.rs:123-324`

**功能**: Chat chunks → Responses API 语义事件

**生成的事件序列**:
1. `response.created` - 响应开始
2. `response.in_progress` - 响应进行中
3. `response.output_item.added` - 输出项添加
4. `response.content_part.added` - 内容部分添加
5. `response.output_text.delta` - 文本增量 (每个 chunk)
6. `response.output_text.done` - 文本完成
7. `response.content_part.done` - 内容部分完成
8. `response.output_item.done` - 输出项完成
9. `response.completed` - 响应完成 (包含 token_usage)
10. `[DONE]` - 流结束标记

#### 3. `transform_chat_stream_to_responses_stream`
**文件**: `src/transform/mod.rs:557-636`

**功能**: Chat streaming chunk → Responses streaming chunk

**转换规则**:
- `delta.content` → `output[type="message"].content[type="output_text"]`
- `delta.tool_calls` → `output[type="message"].tool_calls`
- `usage` → `usage{input_tokens, output_tokens, total_tokens}`

---

## 修复方案

### ✅ 已完成的修复

#### 1. 修复 OpenAI Provider 流式处理
**文件**: `src/providers/openai.rs:189-215`

```rust
// Process SSE stream and collect chunks
let mut chunks = Vec::new();
let body = response.bytes().await
    .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

let body_str = String::from_utf8_lossy(&body);

// Parse SSE format: "data: {...}\n\n" or "data: [DONE]\n\n"
for line in body_str.lines() {
    let line = line.trim();
    if line.starts_with("data: ") {
        let data = &line[6..];
        if data == "[DONE]" {
            break;
        }
        if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
            chunks.push(chunk);
        }
    }
}

Ok(StreamingChat::new(status.as_u16(), chunks))
```

**效果**:
- ✅ 正确处理 SSE 流
- ✅ 收集所有 chunks
- ✅ 解析 `[DONE]` 标记

#### 2. 确保总是发送 `response.completed`
**文件**: `src/handlers/mod.rs:123-136`

```rust
if chunks.is_empty() {
    tracing::warn!("responses_protocol_payloads_from_chat_chunks called with empty chunks");
    return vec![
        serde_json::to_string(&json!({
            "type": "response.completed",
            "response_id": "resp_empty",
            "token_usage": null
        }))
        .unwrap_or_default(),
        "[DONE]".to_string(),
    ];
}
```

**效果**:
- ✅ 即使 chunks 为空也发送 `response.completed`
- ✅ 防止客户端超时等待
- ✅ 添加警告日志

#### 3. 修复 Usage 字段解析
**文件**: `src/models/response.rs:1309-1329`, `src/models/chat.rs:313-325`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,

    #[serde(default)]
    pub output_tokens: u64,

    #[serde(default)]
    pub total_tokens: u64,
}
```

**效果**:
- ✅ 允许部分 token 字段缺失
- ✅ 默认值为 0
- ✅ 避免 "missing field input_tokens" 错误

#### 4. 添加测试用例
**文件**: `src/sse/responses.rs:483-528`

```rust
#[test]
fn test_parse_completed_event_without_token_usage() {
    let data = r#"{"type":"response.completed","response_id":"resp_123"}"#;
    let result = parse_responses_sse_event(data).unwrap().unwrap();
    // ✅ 验证可以解析没有 token_usage 的事件
}

#[test]
fn test_parse_completed_event_with_partial_token_usage() {
    let data = r#"{"type":"response.completed","response_id":"resp_456","token_usage":{"output_tokens":10}}"#;
    let result = parse_responses_sse_event(data).unwrap().unwrap();
    // ✅ 验证可以解析部分 token 字段
}
```

---

## 测试验证

### 测试场景

#### 场景 1: 正常流式响应
```bash
curl -X POST http://localhost:8080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "input": [{"type": "message", "role": "user", "content": "Hello"}],
    "stream": true
  }'
```

**预期输出**:
```
data: {"type":"response.created",...}
data: {"type":"response.in_progress",...}
data: {"type":"response.output_text.delta","delta":"Hello",...}
data: {"type":"response.completed","response_id":"resp_123","token_usage":{...}}
data: [DONE]
```

#### 场景 2: 空响应
```bash
# 如果 provider 返回空 chunks
```

**预期输出**:
```
data: {"type":"response.completed","response_id":"resp_empty","token_usage":null}
data: [DONE]
```

#### 场景 3: 部分使用统计
```bash
# 如果 usage 只包含部分字段
```

**预期输出**:
```
data: {"type":"response.completed","token_usage":{"input_tokens":0,"output_tokens":10,"total_tokens":0}}
data: [DONE]
```

---

## 下一步优化建议

### 🔴 高优先级

1. **实现真正的流式处理**
   - 当前: 等待整个响应完成 (`response.bytes()`)
   - 建议: 使用 `response.bytes_stream()` 实时处理
   - 效果: 减少内存占用，降低延迟

2. **添加超时处理**
   - 为 provider 请求添加超时
   - 为 SSE 流添加心跳检测
   - 防止无限等待

### 🟡 中优先级

3. **改进错误处理**
   - 在 `response.completed` 中包含错误信息
   - 添加更详细的日志
   - 实现 SSE 重连机制

4. **性能监控**
   - 记录 chunk 处理时间
   - 监控内存使用
   - 统计 token 使用量

### 🟢 低优先级

5. **缓存优化**
   - 缓存重复的 JSON 序列化
   - 使用对象池减少分配
   - 优化字符串克隆

---

## 参考资料

### 官方文档
- [Responses Streaming Events](https://developers.openai.com/api/reference/resources/responses/streaming-events/)
- [Streaming API Responses Guide](https://developers.openai.com/api/docs/guides/streaming-responses/)
- [Migrate to Responses API](https://developers.openai.com/api/docs/guides/migrate-to-responses/)

### 社区资源
- [Responses API streaming - the simple guide to "events"](https://community.openai.com/t/responses-api-streaming-the-simple-guide-to-events/1363122)
- [Token usage calculation with streaming responses](https://community.openai.com/t/token-usage-calculation-with-streaming-responses-is-this-not-supported/1298080)
- [OpenAI Response API to Chat Completion API Stream Converter](https://gist.github.com/jlia0/b30fd55b7dcf36ca6623bc84f96c289c)

---

**生成时间**: 2026-03-23
**版本**: v1.0
**状态**: ✅ 修复完成，待测试验证
