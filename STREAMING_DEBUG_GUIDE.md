# 🔍 流式响应问题诊断指南

## 问题描述
**错误信息**: `Stream disconnected before completion: stream closed before response.completed`

## 已添加的诊断日志

### 1. Provider 层日志
**文件**: `src/providers/zhipu.rs`, `src/providers/openai.rs`

```rust
tracing::info!(
    provider = self.name(),
    total_lines = line_count,
    body_length = body_str.len(),
    "processing SSE stream"
);

tracing::info!(
    provider = self.name(),
    data_lines = data_lines,
    parsed_chunks = parsed_chunks,
    parse_errors = errors,
    "SSE stream processing complete"
);
```

**日志示例**:
```
INFO processing SSE stream provider=zhipu total_lines=42 body_length=3847
INFO found [DONE] marker in SSE stream provider=zhipu
INFO SSE stream processing complete provider=zhipu data_lines=40 parsed_chunks=39 parse_errors=0
```

### 2. Handler 层日志
**文件**: `src/handlers/mod.rs`

```rust
tracing::info!(
    route = "/v1/responses",
    provider = provider.name(),
    chunk_count = streaming.chunks.len(),
    status = streaming.status,
    "provider streaming response received"
);

tracing::info!(
    route = "/v1/responses",
    event_count = events.len(),
    "generated SSE events from chunks"
);
```

**日志示例**:
```
INFO provider streaming response received route=/v1/responses provider=zhipu chunk_count=39 status=200
INFO generated SSE events from chunks route=/v1/responses event_count=45
```

### 3. 转换层日志
**文件**: `src/handlers/mod.rs`

```rust
tracing::info!(
    chunk_count = chunks.len(),
    "responses_protocol_payloads_from_chat_chunks called"
);

tracing::info!(
    payload_count = payloads.len(),
    final_usage = ?final_usage,
    "generated payloads, adding response.completed"
);

tracing::info!(
    total_payloads = payloads.len(),
    "responses_protocol_payloads_from_chat_chunks complete"
);
```

**日志示例**:
```
INFO responses_protocol_payloads_from_chat_chunks called chunk_count=39
INFO processing chunks starting from first first_chunk_id=chatcmpl-123 first_chunk_model=gpt-4
INFO generated payloads, adding response.completed payload_count=43 final_usage=Some(...)
INFO responses_protocol_payloads_from_chat_chunks complete total_payloads=45
```

---

## 🔧 诊断步骤

### 步骤 1: 启动服务并查看日志

```bash
# 启动服务，设置日志级别为 debug
RUST_LOG=debug cargo run --release

# 或者只看 info 级别
RUST_LOG=info cargo run --release

# 只看特定模块
RUST_LOG=rcodex=info,rcodex::handlers=debug,rcodex::providers=debug cargo run --release
```

### 步骤 2: 发送测试请求

```bash
# 测试 Responses API 流式请求
curl -X POST http://localhost:8080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "input": [{"type": "message", "role": "user", "content": "Hello"}],
    "stream": true
  }' \
  -v
```

### 步骤 3: 分析日志

**关键日志序列**:

```
1. INFO provider streaming request
   └─ provider 接收到流式请求

2. INFO processing SSE stream
   └─ 开始处理 SSE 流

3. INFO found [DONE] marker in SSE stream
   └─ 找到 [DONE] 标记

4. INFO SSE stream processing complete
   └─ SSE 流处理完成，显示解析的 chunk 数量

5. INFO provider streaming response received
   └─ handler 接收到 provider 返回的 chunks

6. INFO responses_protocol_payloads_from_chat_chunks called
   └─ 开始转换 chunks 为 payloads

7. INFO generated payloads, adding response.completed
   └─ 生成 payloads，添加 response.completed 事件

8. INFO responses_protocol_payloads_from_chat_chunks complete
   └─ 转换完成，显示总 payload 数量
```

---

## 🚨 常见问题诊断

### 问题 1: `parsed_chunks = 0`

**症状**:
```
INFO SSE stream processing complete provider=zhipu data_lines=40 parsed_chunks=0 parse_errors=40
```

**原因**:
- ChatCompletionChunk 解析失败
- JSON 格式不匹配

**诊断**:
```bash
# 查看解析错误详情
RUST_LOG=rcodex::providers=debug cargo run --release
```

**查看错误日志**:
```
WARN failed to parse chunk provider=zhipu error=... data_preview=...
```

**解决方案**:
1. 检查 provider 返回的 JSON 格式
2. 确认 ChatCompletionChunk 结构定义正确
3. 添加字段映射或默认值

### 问题 2: `chunk_count = 0`

**症状**:
```
INFO provider streaming response received route=/v1/responses provider=zhipu chunk_count=0 status=200
WARN responses_protocol_payloads_from_chat_chunks called with empty chunks
```

**原因**:
- Provider 没有返回任何 chunk
- SSE 流解析完全失败

**诊断**:
1. 检查 `total_lines` 和 `body_length`
2. 检查 `data_lines` 数量
3. 查看原始响应内容

**解决方案**:
```rust
// 添加原始响应日志
tracing::debug!(
    provider = self.name(),
    raw_response = %body_str,
    "raw SSE response"
);
```

### 问题 3: `event_count` 不包含 `response.completed`

**症状**:
```
INFO generated SSE events from chunks route=/v1/responses event_count=2
```

但预期应该有更多事件。

**原因**:
- `responses_protocol_payloads_from_chat_chunks` 提前返回
- Chunks 为空触发了快速路径

**诊断**:
检查是否有以下日志:
```
WARN responses_protocol_payloads_from_chat_chunks called with empty chunks
```

### 问题 4: `[DONE]` 标记缺失

**症状**:
- 流正常结束但没有 `[DONE]`
- 客户端等待超时

**原因**:
- Provider 没有发送 `[DONE]`
- `[DONE]` 在循环外部

**诊断**:
查看日志中是否有:
```
INFO found [DONE] marker in SSE stream
```

**解决方案**:
```rust
// 即使没有 [DONE]，也要处理完所有数据
for line in body_str.lines() {
    // ... 处理逻辑
}

// 循环结束后，确保处理完成
tracing::info!("SSE stream ended (no explicit [DONE] marker)");
```

---

## 📊 日志分析工具

### 方法 1: 使用 grep 过滤

```bash
# 只看 provider 相关日志
cargo run --release 2>&1 | grep "provider="

# 只看 chunk 相关日志
cargo run --release 2>&1 | grep "chunk"

# 只看 response.completed 相关日志
cargo run --release 2>&1 | grep "response.completed"
```

### 方法 2: 使用 jq 分析 JSON 日志

```bash
# 如果使用 JSON 格式日志
RUST_LOG=info cargo run --release 2>&1 | jq 'select(.message contains "chunk")'

# 统计 parsed_chunks
cargo run --release 2>&1 | jq 'select(.parsed_chunks != null) | .parsed_chunks'
```

### 方法 3: 保存日志到文件

```bash
# 保存完整日志
RUST_LOG=debug cargo run --release 2>&1 | tee debug.log

# 实时查看和保存
RUST_LOG=info cargo run --release 2>&1 | tee >(grep "ERROR\|WARN" > errors.log)
```

---

## 🎯 快速诊断脚本

创建 `diagnose.sh`:

```bash
#!/bin/bash

echo "=== Starting rcodex with diagnostic logging ==="
echo "Log level: DEBUG"
echo "Focus: streaming, chunks, response.completed"
echo ""

# 启动服务
RUST_LOG=rcodex=debug cargo run --release 2>&1 | while IFS= read -r line; do
    echo "$line"

    # 高亮关键日志
    if echo "$line" | grep -q "parsed_chunks"; then
        echo "  👆 CHUNK PARSE INFO"
    fi

    if echo "$line" | grep -q "response.completed"; then
        echo "  👆 RESPONSE COMPLETED EVENT"
    fi

    if echo "$line" | grep -q "ERROR\|WARN"; then
        echo "  ⚠️  ERROR/WARNING DETECTED"
    fi
done
```

运行:
```bash
chmod +x diagnose.sh
./diagnose.sh
```

---

## 🔬 高级调试

### 1. 打印原始 SSE 响应

在 `src/providers/zhipu.rs` 或 `src/providers/openai.rs` 添加:

```rust
// 在处理 SSE 之前
tracing::debug!(
    provider = self.name(),
    raw_sse = %body_str.lines().take(5).collect::<Vec<_>>().join("\n"),
    "first 5 lines of SSE response"
);
```

### 2. 打印每个 chunk 的详细信息

```rust
for (i, chunk) in chunks.iter().enumerate() {
    tracing::debug!(
        provider = self.name(),
        index = i,
        chunk_id = %chunk.id,
        choices = chunk.choices.len(),
        finish_reason = ?chunk.choices.first().and_then(|c| c.finish_reason.as_ref()),
        has_content = chunk.choices.first().and_then(|c| c.delta.as_ref()).and_then(|d| d.content.as_ref()).is_some(),
        "chunk detail"
    );
}
```

### 3. 打印生成的事件

在 `src/handlers/mod.rs` 的 `responses_protocol_payloads_from_chat_chunks` 函数中:

```rust
for (i, payload) in payloads.iter().enumerate() {
    tracing::debug!(
        index = i,
        payload_preview = &payload[..payload.len().min(100)],
        "generated payload"
    );
}
```

---

## ✅ 预期的完整日志流程

**成功的流式响应应该包含以下日志序列**:

```
1. INFO provider streaming request provider=zhipu model=gpt-4
2. INFO processing SSE stream provider=zhipu total_lines=42 body_length=3847
3. INFO found [DONE] marker in SSE stream provider=zhipu
4. INFO SSE stream processing complete provider=zhipu data_lines=40 parsed_chunks=39 parse_errors=0
5. INFO provider streaming response received route=/v1/responses provider=zhipu chunk_count=39 status=200
6. INFO generated SSE events from chunks route=/v1/responses event_count=45
7. INFO responses_protocol_payloads_from_chat_chunks called chunk_count=39
8. INFO processing chunks starting from first first_chunk_id=chatcmpl-123
9. INFO generated payloads, adding response.completed payload_count=43 final_usage=Some(...)
10. INFO responses_protocol_payloads_from_chat_chunks complete total_payloads=45
```

**如果缺少任何一步，说明该步骤有问题。**

---

## 📝 报告问题

如果问题依然存在，请提供以下信息:

1. **完整的日志输出** (从请求开始到错误)
2. **Provider 类型** (OpenAI / Zhipu)
3. **请求内容** (脱敏后)
4. **错误消息** (完整的错误堆栈)
5. **以下关键指标**:
   - `total_lines`
   - `data_lines`
   - `parsed_chunks`
   - `parse_errors`
   - `chunk_count`
   - `event_count`
   - `total_payloads`

---

**创建时间**: 2026-03-23
**用途**: 诊断 "stream closed before response.completed" 错误
