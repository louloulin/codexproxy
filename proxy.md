# rcodex Proxy / Codex CLI 接入问题分析

> 更新时间：2026-04-22
> 目标：对照 `/Users/louloulin/Documents/linchong/code/codex-glm-proxy`，分析 `rcodex` 当前接入 Codex CLI 时为什么不稳定/不兼容，并给出后续 TODO。

## 结论

`codex-glm-proxy` 能跑通的核心原因，不是它“功能更多”，而是它把问题收敛成了一个非常明确的代理职责：

1. 只做一条稳定的协议桥接链路：`Responses API -> Chat Completions -> Responses API`。
2. 明确把历史 `function_call` / `function_call_output` 转成 chat message 序列，而不是依赖上游 provider 真正理解 `previous_response_id`。
3. 流式结束以 **上游 `[DONE]`** 为准，主动补齐 `response.completed`，而不是把完成态绑定到 `usage` 是否出现。
4. 工具兼容策略简单直接：不支持的工具显式过滤，支持的工具明确转换。

`rcodex` 当前的问题，根本上不是某一个字段漏了，而是：

1. 协议边界被拆散到了 `transform`、`handlers`、`protocol/events`、`websocket` 四处，HTTP/SSE/WS 三条路径的语义不一致。
2. 对 Codex CLI 的“支持面”声明得比真实实现更乐观，导致 planner 接受了请求，但 fallback 路径实际保不住这些语义。
3. 对流式终止条件和 WebSocket 协议的理解偏离了 Codex 真实客户端实现。

---

## 一、从 codex-glm-proxy 学到什么

### 1. 参考实现的关键取舍

`codex-glm-proxy` 的 README 明确把自己定义为一个“Responses <-> Chat Completions”的本地桥接器，而不是一个同时模拟多套上游能力的复杂网关。

证据：

- `README_CN.md:44-73`：架构上只有 Codex CLI、代理、GLM 三层。
- `README_CN.md:97-130`：关键转换点就是“请求转 chat / 响应转 responses events”。
- `proxy.py:32-181`：`convert_responses_to_chat()` 明确负责模型映射、input/history/tool 处理。
- `proxy.py:444-718`：流式转换逻辑直接围绕 `delta`、`tool_calls`、`[DONE]` 生成 Responses 事件。

### 2. 它为什么对 Codex 更友好

`codex-glm-proxy` 的几个设计点非常关键：

1. `proxy.py:66-120` 把 `message`、`function_call`、`function_call_output` 都转换进 chat 历史，避免多轮时只剩文本上下文。
2. `proxy.py:140-171` 对不支持的工具显式过滤，而不是宣称“支持”后在后面静默丢字段。
3. `proxy.py:455-501` 在读到上游 `[DONE]` 时，无条件收口并发出 `response.completed`。
4. `proxy.py:517-718` 明确维护事件顺序：`response.created -> output_item.added -> delta -> done -> completed -> [DONE]`。

这套思路不一定等于“官方 Responses WebSocket v2”，但它在代理场景下非常稳定，因为它只守住一个最小兼容闭环。

---

## 二、rcodex 当前最关键的问题

## P0-1：WebSocket 协议实现与 Codex CLI 实际协议不一致

这是目前最致命的问题。

`rcodex` 当前 WebSocket 处理器把客户端第一条消息当成裸 `ResponsesRequest`：

- `src/handlers/websocket.rs:38-59`

但官方 Codex 客户端发送的是带 `type` 的 WebSocket 请求：

- `codex/codex-rs/codex-api/src/common.rs:244-249`
- `codex/codex-rs/codex-api/src/common.rs:197-219`
- `codex/codex-rs/core/src/client.rs:1202-1208`

也就是说，Codex 真实发的是：

```json
{
  "type": "response.create",
  ...
}
```

而不是：

```json
{
  "model": "...",
  "input": [...]
}
```

另外，`rcodex` WebSocket 返回的是“把 SSE 再包进 WS 文本帧”：

- `src/handlers/websocket.rs:135-166`

它发送的是：

```text
data: {...}\n\n
```

但官方客户端在 WebSocket 模式下期待的是 **纯 JSON 事件帧**，不是 SSE 包装：

- `codex/codex-rs/codex-api/src/endpoint/responses_websocket.rs:343-410`
- `codex/codex-rs/codex-api/src/endpoint/responses_websocket.rs:541-641`

同时，官方握手还依赖：

- `OpenAI-Beta: responses_websockets=2026-02-06`
- `x-client-request-id`
- `session_id`
- `x-codex-turn-state`

证据：

- `codex/codex-rs/core/src/client.rs:701-728`
- `codex/codex-rs/codex-api/src/requests/headers.rs:4-11`
- `codex/codex-rs/codex-api/src/endpoint/responses_websocket.rs:386-404`

而 `rcodex` 的 WS handler 完全没有处理这套 header/state 语义。

结论：

`rcodex` 当前的 WS 路径不是“官方 WebSocket Responses 协议实现”，而是“把现有 SSE 逻辑塞进 WebSocket”。如果 Codex CLI 开启 websocket transport，这条路基本天然不兼容。

## P0-2：流式完成条件错误，`response.completed` 被错误绑定到 `usage`

`rcodex` 在 streamed fallback 路径里，只有当 `chunk.usage.is_some()` 时才把流标记为 final：

- `src/protocol/events.rs:427-433`
- `src/protocol/events.rs:583-618`

这意味着如果上游 provider：

1. 正常流式输出了所有 delta
2. 最后发了 `[DONE]`
3. 但没有带 `usage`

那么 `rcodex` 就不会发 `response.completed`，也不会发最终 `[DONE]`。

而官方 Codex SSE/WS 解析器在流关闭但没有看到 `response.completed` 时，会报：

- `stream closed before response.completed`

证据：

- `codex/codex-rs/codex-api/src/sse/responses.rs:317-354`
- `codex/codex-rs/codex-api/src/endpoint/responses_websocket.rs:541-583`

对照参考实现：

- `codex-glm-proxy/proxy.py:455-501`

`codex-glm-proxy` 是在读到 `[DONE]` 时主动补 `response.completed`，这个策略对代理更稳。

结论：

`rcodex` 的 streamed fallback 目前把“有 usage”误当成“流结束”的唯一判据，这会直接导致 Codex CLI 在真实 provider 上卡死或报协议错误。

## P0-3：多轮上下文策略不对，`previous_response_id` 在 fallback 场景直接被拒

`rcodex` 的 capability planner 对 chat fallback 明确把 `previous_response_id` 视为不支持：

- `src/protocol/capabilities.rs:55-69`
- `src/protocol/capabilities.rs:157-160`

这会让类似 Zhipu 的 fallback 路径在多轮时直接 reject。

但参考实现的思路不是“要求 provider 原生支持 `previous_response_id`”，而是：

1. 把 Responses input 里的历史消息、历史 tool call、tool output 全量展开。
2. 转成 chat history 后继续调用 chat completions。

证据：

- `codex-glm-proxy/proxy.py:59-120`

`rcodex` 自己其实也已经有这类转换基础：

- `src/transform/mod.rs:189-320`

但 planner 先把请求拒掉了，导致 fallback 的价值被提前抹掉。

结论：

对代理来说，`previous_response_id` 不应该简单等价于“provider 必须原生支持 continuation”。否则 Codex CLI 的多轮体验会被直接打断。

## P1-1：能力声明过于乐观，真实 fallback 根本保不住这些字段

`ZhipuProvider` 使用的是 `chat_fallback_with_full_support()` 这一套能力声明，宣称支持：

- `reasoning`
- `store`
- `model_settings`
- `prompt_cache_key`
- `namespace`

证据：

- `src/protocol/capabilities.rs:92-108`

但真正的 transform/fallback 路径里，这些字段大多被直接丢掉了：

- `transform_chat_to_responses_request()` 把 `store / previous_response_id / metadata / model_settings / reasoning / service_tier / prompt_cache_key / namespace` 统统写成 `None`
  - `src/transform/mod.rs:157-168`

同时 `transform_responses_to_chat_request()` 并没有把 `model_settings / prompt_cache_key / namespace / store` 映射到 `ChatRequest`，因为 `ChatRequest` 自身也没有这些字段：

- `src/models/chat.rs:12-89`
- `src/transform/mod.rs:173-320`

结论：

当前 planner 是“先说能保住，再在转换时丢字段”。这会让 Codex CLI 觉得服务端支持某能力，但运行时行为又不一致，属于非常典型的兼容性陷阱。

## P1-2：消息内容转换会丢信息，不适合 Codex 的复杂 input item

`transform_responses_to_chat_request()` 对 `Message.content` 的处理是 `find_map()`：

- `src/transform/mod.rs:193-206`

这表示它只拿 **第一个可识别的 content block**，后面的 block 会被丢掉。结果包括：

1. 多个 text block 不能正确拼接。
2. multimodal / mixed content 只能退化成单一字符串。
3. reasoning / refusal / output_text 混合结构很容易失真。

对照参考实现：

- `codex-glm-proxy/proxy.py:74-87`

它至少会遍历 content blocks，把多个 `input_text` 拼起来，而不是只取第一个。

结论：

这会让 Codex CLI 一些更复杂的输入 item 在 fallback 时被静默降级，属于“能跑但语义慢慢坏掉”的问题。

## P1-3：响应头和元数据没有透传，破坏官方客户端的粘性路由/模型识别

官方客户端会从 Responses HTTP/WS 响应头里读取：

- `x-codex-turn-state`
- `X-Models-Etag`
- `openai-model`
- `x-reasoning-included`

证据：

- `codex/codex-rs/codex-api/src/sse/responses.rs:57-89`
- `codex/codex-rs/codex-api/src/endpoint/responses_websocket.rs:386-404`

但 `rcodex` 当前无论 native responses 还是 chat fallback，基本都只返回一个裸 `Sse` 响应，没有把这些 header 透回给客户端：

- `src/handlers/mod.rs:795-846`
- `src/handlers/mod.rs:870-982`

这意味着：

1. turn-state 无法建立或回放。
2. server model 无法可靠透传。
3. 某些客户端优化路径会失效。

这未必每次都立刻报错，但会让接入表现明显比官方接口脆弱。

## P1-4：协议实现没有单一事实来源，三条路径语义分叉

现在 `rcodex` 至少存在三套不同语义：

1. HTTP collected path：`responses_protocol_payloads_from_chat_chunks()`
2. HTTP streamed path：`ResponsesChatStreamState`
3. WebSocket path：`handlers/websocket.rs` 里的 SSE-over-WS

证据：

- `src/protocol/events.rs:624-913`
- `src/protocol/events.rs:410-622`
- `src/handlers/websocket.rs:118-174`

这三套逻辑对以下问题的处理都不完全一致：

1. 何时发 `response.completed`
2. 是否依赖 `usage`
3. tool_call 增量与完成态
4. 输出帧到底是 JSON 还是 SSE 包装

这也是为什么仓库里已经出现了多份 “response.completed / stream closed” 相关诊断文档，但问题仍然容易反复出现。

---

## 三、为什么 codex-glm-proxy 更容易接通，而 rcodex 更容易出问题

一句话总结：

`codex-glm-proxy` 走的是“把 Codex 当成 Responses-over-HTTP/SSE 客户端来满足”的最小闭环；`rcodex` 走的是“同时兼容 native responses / chat fallback / websocket / codex 特性”的大而全路线，但协议抽象还没有收敛成单一真相，所以一旦进入真实 Codex CLI 运行路径，最先暴露的就是状态机和边界协议不一致。

更具体地说：

1. `codex-glm-proxy` 的成功来自“少承诺、强收口”。
2. `rcodex` 的问题来自“多路径、多能力声明，但没有统一协议内核”。
3. 目前真正阻塞 Codex CLI 的，不是 JSON 字段多一两个，而是 WebSocket contract 和 streamed completion contract。

---

## 四、建议的修复优先级

### 第一优先级

- 修正 WebSocket 协议，按官方 `response.create` / JSON event frame / handshake headers 实现。
- 让 streamed fallback 在收到上游结束信号时总能产出 `response.completed`，不能再依赖 `usage`。
- 重新定义 fallback 下的多轮策略，不要把 `previous_response_id` 直接等价成“必须 reject”。

### 第二优先级

- 收紧 capability planner，只声明 fallback 真正能保住的字段。
- 把 Responses -> Chat 的 content block 转换改成“遍历并合并”，不要 `find_map()`。
- 把 `x-codex-turn-state`、`openai-model`、`x-reasoning-included`、`X-Models-Etag` 纳入统一响应头透传策略。

### 第三优先级

- 合并 HTTP collected / HTTP streamed / WS 三套 event builder，建立单一协议状态机。
- 针对 Codex CLI 建立真实兼容性回归测试，不再只做结构级测试。

---

## TODO List

- [ ] 按官方 `ResponsesWsRequest::ResponseCreate` 重新实现 `/v1/responses` 的 WebSocket 入站协议。
- [ ] WebSocket 出站改为纯 JSON event frame，移除当前 SSE-over-WS 包装。
- [ ] 在 WebSocket 握手和 HTTP Responses 响应中补齐 `x-codex-turn-state`、`openai-model`、`x-reasoning-included`、`X-Models-Etag`。
- [ ] 为 streamed chat fallback 增加“基于上游 `[DONE]` 的完成收口逻辑”，确保始终发出 `response.completed`。
- [ ] 重构 `ResponsesChatStreamState`，不要再把 `usage` 当作唯一 final signal。
- [ ] 调整 fallback planner：对 `previous_response_id` 优先尝试“历史展开到 input/messages”，只有真正无法保真时再 reject。
- [ ] 收紧 `chat_fallback_with_full_support()` 的能力声明，改成“实现了什么就声明什么”。
- [ ] 显式梳理 `reasoning / store / model_settings / prompt_cache_key / namespace / service_tier` 在 fallback 中的保真策略。
- [ ] 修复 `transform_responses_to_chat_request()` 的 content block 合并逻辑，避免 `find_map()` 丢块。
- [ ] 为复杂 input item 增加测试：多 text blocks、tool history、function_call_output、reasoning、multimodal。
- [ ] 建一个统一的 Responses protocol builder，替代当前 HTTP collected / streamed / websocket 三套分叉逻辑。
- [ ] 增加真实 Codex CLI 兼容测试，覆盖 HTTP SSE、WebSocket、multi-turn、tool call、no-usage-stream ending。

