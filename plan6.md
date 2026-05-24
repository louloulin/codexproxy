# rcodex Codex CLI 协议实现全面分析

> **分析日期**: 2026-04-20
> **分析来源**: Codex CLI 0.121.0 二进制逆向工程 (`/opt/homebrew/bin/codex`)
> **目标**: 全面分析 rcodex 协议实现问题，识别并修复真实问题

---

## 一、Codex CLI 协议架构

### 1.1 Codex CLI 使用两种协议

```
┌─────────────────────────────────────────────────────────────────┐
│                    Codex CLI 0.121.0                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  配置: wire_api = "responses"  ──────────────────┐              │
│  配置: wire_api = "chat"       ──────────────────┼──────> rcodex│
│                                                     │              │
│  App-Server Protocol (stdio) ◄───────────────────┘              │
│  - JSON-RPC over stdio                                              │
│  - 事件: thread.started, turn.started, agent_message              │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 rcodex 实现的协议

rcodex 实现的是 **OpenAI Responses API** (HTTP SSE)，当 Codex CLI 配置 `wire_api = "responses"` 时使用。

```
Codex CLI (wire_api="responses")  ──HTTP/SSE──>  rcodex  ──> Zhipu/GLM
                                                Responses API
```

---

## 二、从 Codex 二进制提取的协议规范

### 2.1 Codex 期望的 SSE 事件类型

通过逆向 `codex` 二进制提取的事件类型：

| 事件类型 | 状态 | 说明 |
|---------|------|------|
| `response.created` | ✅ | 响应创建 |
| `response.in_progress` | ✅ | 响应进行中 |
| `response.output_item.added` | ✅ | 输出项添加 |
| `response.output_text.delta` | ✅ | 文本增量 |
| `response.reasoning_summary_text.delta` | ⚠️ | 推理摘要文本 |
| `response.reasoning_text.delta` | ⚠️ | 推理文本 |
| `response.reasoning_summary_part.added` | ⚠️ | 推理摘要部分 |
| `response.server_model` | ❓ | 服务器模型 |
| `response.server_reasoning_included` | ❓ | 推理包含 |
| `response.rate_limits` | ❓ | 速率限制 |
| `response.models_etag` | ❓ | 模型 Etag |
| `response.output_item.done` | ✅ | 输出项完成 |
| `response.completed` | ✅ | 响应完成 |
| `response.failed` | ❓ | 响应失败 |

**rcodex 当前发送的事件**:
- ✅ `response.created`
- ✅ `response.in_progress`
- ✅ `response.output_item.added`
- ✅ `response.output_text.delta`
- ✅ `response.output_text.done`
- ❌ `response.content_part.added` (Codex 不使用)
- ❌ `response.content_part.done` (Codex 不使用)
- ✅ `response.output_item.done`
- ✅ `response.completed`

### 2.2 Codex 二进制中的关键错误消息

从 Codex 二进制中提取的错误消息揭示了关键协议要求：

```
"stream closed before response.completed"
"OutputTextDelta without active item"
"ReasoningSummaryDelta without active item"
"ReasoningRawContentDelta without active item"
"ReasoningSummaryPartAdded without active item"
"failed to parse ResponseItem from output_item.done"
"failed to parse ResponseItem from output_item.added"
"idle timeout waiting for SSE"
```

### 2.3 关键发现：item 状态要求

**"OutputTextDelta without active item"** 是关键错误！

这意味着 Codex CLI 在内部维护一个"active item"状态，当收到 `response.output_text.delta` 时，它必须已经收到对应的 `response.output_item.added` 并且该 item 仍然处于 active 状态。

**问题分析**:
1. rcodex 确实在发送 `response.output_item.added` 之后才发送 `response.output_text.delta`
2. 但可能是 `item_id` 或 `output_index` 关联不正确

---

## 三、rcodex 当前实现分析

### 3.1 ResponsesStreamState 状态机

**文件**: `src/handlers/mod.rs:197-476`

```rust
struct ResponsesStreamState {
    initial_events_emitted: bool,
    added_indices: HashMap<u32, bool>,      // 追踪已发送 output_item.added 的 index
    text_by_index: HashMap<u32, String>,   // 累积文本
    final_usage: Option<Usage>,
    is_final: bool,
    text_done_indices: HashMap<u32, bool>,
    content_part_done_indices: HashMap<u32, bool>,
    item_done_indices: HashMap<u32, bool>,
}
```

### 3.2 当前事件生成顺序

```
1. response.created (首次 chunk)
2. response.in_progress (首次 chunk)
3. response.output_item.added (每个 index 首次出现)
4. response.output_text.delta (每个 delta 内容)
5. response.output_text.done (finish_reason 出现)
6. response.content_part.done (Codex 不使用)
7. response.output_item.done (finish_reason 出现)
8. response.completed (最终 chunk)
9. [DONE]
```

### 3.3 发现的问题

#### 问题 1: `response.output_text.delta` 缺少 `item_id` 字段

**当前 rcodex 发送**:
```json
{
  "type": "response.output_text.delta",
  "output_index": 0,
  "content_index": 0,
  "delta": "Hello",
  "total_duration": 1000000,
  "start_index": 0,
  "end_index": 5
}
```

**可能问题**: Codex 可能期望 `item_id` 字段来关联 delta 和 item。

**修复建议**: 添加 `item_id` 到 `response.output_text.delta`:
```json
{
  "type": "response.output_text.delta",
  "item_id": "msg_xxx_0",
  "output_index": 0,
  "content_index": 0,
  "delta": "Hello"
}
```

#### 问题 2: rcodex 发送 Codex 不使用的 events

**Codex 二进制中没有找到**:
- `response.content_part.added`
- `response.content_part.done`

rcodex 当前会发送这些事件，但 Codex CLI 可能忽略或出错。

#### 问题 3: `response.server_model` 和 `response.rate_limits` 缺失

Codex 期望的事件：
- `response.server_model` - 服务器模型信息
- `response.rate_limits` - 速率限制信息

rcodex 当前不发送这些事件。

#### 问题 4: item 结构可能不匹配

从错误消息 "failed to parse ResponseItem from output_item.done" 来看，Codex 期望的 item 结构和 rcodex 发送的可能不同。

**rcodex 当前发送的 output_item**:
```json
{
  "type": "message",
  "id": "msg_xxx_0",
  "role": "assistant",
  "content": []
}
```

**Codex 可能期望的**: 可能需要更多字段或不同结构。

---

## 四、真实问题分析

### 4.1 Codex CLI 不显示响应文本的根本原因

基于二进制分析，最可能的原因是：

1. **item_id 关联问题**: `response.output_text.delta` 事件缺少 `item_id`，导致 Codex 无法将 delta 与正确的 item 关联

2. **事件顺序问题**: 虽然 rcodex 表面上按正确顺序发送事件，但 Codex 内部的状态机可能因为某些字段不匹配而无法正确处理

3. **缺少必要事件**: `response.server_model` 和 `response.rate_limits` 可能是 Codex 期望的必填事件

### 4.2 修复优先级

| 优先级 | 修复项 | 影响 |
|--------|--------|------|
| P0 | 添加 `item_id` 到 `response.output_text.delta` | 可能修复文本显示 |
| P0 | 检查 item 结构是否正确 | 可能修复文本显示 |
| P1 | 移除 Codex 不使用的事件 | 减少协议噪音 |
| P1 | 添加 `response.server_model` 事件 | 完整协议支持 |
| P2 | 添加 `response.rate_limits` 事件 | 完整协议支持 |

---

## 五、修复计划

### Task 1: 添加 `item_id` 到 `response.output_text.delta`

**文件**: `src/handlers/mod.rs`
**位置**: `process_chunk` 函数，约 line 336

**当前代码**:
```rust
events.push(serde_json::to_string(&json!({
    "type": "response.output_text.delta",
    "output_index": index,
    "content_index": 0,
    "delta": content,
    "total_duration": 1000000,
    "start_index": start_index,
    "end_index": accumulated.len() as u32
})).unwrap());
```

**修复代码**:
```rust
let item_id = format!("msg_{}_{}", chunk.id, index);
events.push(serde_json::to_string(&json!({
    "type": "response.output_text.delta",
    "item_id": item_id,
    "output_index": index,
    "content_index": 0,
    "delta": content
})).unwrap());
```

### Task 2: 检查并修正 `response.output_item.added` 的 item 结构

**位置**: `src/handlers/mod.rs`，约 line 307

**当前代码**:
```rust
events.push(serde_json::to_string(&json!({
    "type": "response.output_item.added",
    "output_index": index,
    "item": {
        "type": "message",
        "id": item_id,
        "role": role,
        "content": []
    }
})).unwrap());
```

**可能需要**: 检查 Codex 期望的 item 结构，添加必要字段。

### Task 3: 移除 Codex 不使用的事件

移除 `response.content_part.added` 和 `response.content_part.done` 的发送。

### Task 4: 添加 `response.server_model` 事件

在 `response.created` 或 `response.in_progress` 之后发送：
```json
{
  "type": "response.server_model",
  "model": "glm-5"
}
```

### Task 5: 添加 `response.rate_limits` 事件

```json
{
  "type": "response.rate_limits",
  "rate_limits": [...]
}
```

---

## 六、验证方法

### 6.1 本地测试

```bash
# 启动 rcodex
cargo run &

# 测试 Responses API 流式
curl -N -X POST http://127.0.0.1:9080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{"model": "glm-4-flash", "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "Hello"}]}], "stream": true}'

# 使用 Codex CLI 测试
codex --config model_provider=rcodex --config model=glm-5 exec "What is 2+2?"
```

### 6.2 添加调试日志

在 `process_chunk` 中添加更详细的日志：
```rust
tracing::debug!(
    item_id = %item_id,
    output_index = index,
    delta_len = content.len(),
    "emitting response.output_text.delta with item_id"
);
```

---

## 七、附录：Codex 二进制协议字符串

### A.1 SSE 事件类型字符串

```
response.created
response.in_progress
response.output_item.added
response.output_item.done
response.output_text.delta
response.reasoning_summary_text.delta
response.reasoning_text.delta
response.reasoning_summary_part.added
response.server_model
response.server_reasoning_included
response.rate_limits
response.models_etag
response.completed
response.failed
```

### A.2 关键错误消息

```
stream closed before response.completed
OutputTextDelta without active item
ReasoningSummaryDelta without active item
ReasoningRawContentDelta without active item
ReasoningSummaryPartAdded without active item
failed to parse ResponseItem from output_item.done
failed to parse ResponseItem from output_item.added
idle timeout waiting for SSE
codex responses expects a streaming payload with "stream": true
```

### A.3 源代码路径

```
codex-api/src/sse/responses.rs
codex-rs/core/src/client.rs
codex-rs/core/src/codex.rs
```

---

**文档版本**: 1.0
**更新日期**: 2026-04-20
**状态**: 分析完成 - 待修复
