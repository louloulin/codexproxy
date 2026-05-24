# rcodex 5.0 全面分析与后续完善计划

> 日期：2026-04-21
> 范围：`rcodex/` 当前代理实现 + `codex/` 上游 Codex CLI 源码关键链路
> 优先级结论：先把 `Responses SSE 完整兼容` 做到稳定可用，再做 `provider/config 架构升级`，最后补 `能力对齐`

---

## 1. 这次我实际分析了什么

本次不是只看 `rcodex`，而是沿着真实调用链把上游 Codex CLI 一起对照了：

- `codex/codex-rs/model-provider-info/src/lib.rs`
  - 确认 Codex 0.121.0 只接受 `wire_api = "responses"`，`chat` 已被移除。
- `codex/codex-rs/codex-api/src/sse/responses.rs`
  - 确认 Codex 对 Responses SSE 的解析重点是：
    - `response.output_item.added`
    - `response.output_text.delta`
    - `response.output_item.done`
    - `response.completed`
- `codex/codex-rs/core/src/codex.rs`
  - 确认 `exec` 路径里真正驱动最终 assistant 输出的是 `OutputItemDone` 和流式 active item 状态，不是只有 `response.completed` 就够。
- `src/providers/zhipu.rs`
  - 确认当前实现本质仍是 `Chat Completions -> Responses SSE` 的 fallback，而非真正 native `/responses` provider。
- `src/handlers/mod.rs`
  - 确认 rcodex 的 Responses SSE 事件组装都在这里，是最关键的兼容层。
- `src/models/chat.rs`
  - 确认上游 Zhipu 流式 chunk 的字段风格与本地反序列化定义存在兼容缺口。
- `~/.codex/config.toml`
  - 确认本机 `model_provider = "rcodex"` 的 provider 配置实际指向 `http://127.0.0.1:9080`。

---

## 2. 本次复现到的真实问题

我执行了用户指定命令：

```bash
codex --config model_provider=rcodex --config model=glm-4-flash exec "What is 2+2?"
```

### 修复前现象

- 命令退出码是 `0`
- 代理日志显示上游 GLM 已经正常流式返回并结束
- 但终端只有：
  - `user What is 2+2?`
- 没有 assistant 最终输出

### 根因

核心根因不是“模型没回答”，而是“Codex 没收到它认可的终止语义”：

1. Zhipu 原始最后一帧是 snake_case：
   - `"finish_reason":"stop"`
   - `"tool_calls": [...]`
2. `src/models/chat.rs` 里对应结构用了 `rename_all = "camelCase"`，但没有为 snake_case 做 `alias`
3. 结果最后一帧在 rcodex 内部被解析成：
   - `finish_reason = None`
   - `tool_calls = None`
4. `src/handlers/mod.rs` 的 Responses 事件状态机因此无法稳定产出：
   - `response.output_text.done`
   - `response.output_item.done`
5. Codex CLI 的 `exec` 流程最终没有拿到可完成的 assistant message，于是看起来像“代理没回”

### 本次已完成修复

- 在 `src/models/chat.rs` 为以下字段补了 snake_case 兼容：
  - `finish_reason`
  - `tool_calls`
- 在 `src/handlers/mod.rs` 中把 `response.output_item.added` 的 message 内容改成带空 `output_text` 占位，而不是空数组
- 新增并跑通了两类测试：
  - chunk 反序列化兼容测试
  - Responses payload 终止事件生成测试

### 修复后验证结果

同一条命令现在能输出 assistant 文本：

```text
The calculation of 2+2 is straightforward. It equals 4.
```

并且完整测试通过：

```bash
cargo test
```

结果：

- `232 passed`
- `4 ignored`

---

## 3. 当前 rcodex 的真实定位

现在的 rcodex 不是“原生 Codex provider”，而是：

> `Codex CLI Responses 请求`
> -> `rcodex Responses handler`
> -> `transform_responses_to_chat_request`
> -> `Zhipu chat/completions`
> -> `ChatCompletionChunk`
> -> `rcodex 重新拼装 Responses SSE`

这条链路现在已经“能跑”，但还没有“完全对齐”。

换句话说：

- 现在是 `usable`
- 还不是 `fully compatible`
- 更不是 `architecturally complete`

---

## 4. 剩余核心问题分层

### P0：必须优先解决

#### P0-1. 仍然是 chat fallback，不是 native responses provider

表现：

- `src/providers/zhipu.rs` 的核心还是 `/chat/completions`
- `previous_response_id`、原生 response item 生命周期、部分 tool 语义都只能模拟

影响：

- 多轮状态连续性不可靠
- 工具调用语义容易丢字段
- 以后 Codex CLI 再升级协议时，兼容成本持续升高

#### P0-2. Responses SSE 兼容目前还是“事件拼装驱动”，不是“协议模型驱动”

表现：

- `src/handlers/mod.rs` 既做 handler，又做协议状态机，又做兼容补丁
- 这个文件已经非常大，后续继续加特性容易再出隐式回归

影响：

- 难测
- 难扩展
- 难复用到 websocket / future provider

#### P0-3. provider/config 架构还是硬编码双 provider

表现：

- `src/config/app_config.rs` 里 `ProvidersConfig` 仍然写死 `openai` / `zhipu`
- `AppState::get_provider_by_name()` 也是硬编码分支

影响：

- 无法像上游 Codex 一样用 `[model_providers.<id>]` 方式自然扩展
- 后续接入更多 provider 时要继续堆硬编码

#### P0-4. model metadata 没有对齐 Codex

现象：

- `codex exec` 明确报：
  - `Unknown model glm-4-flash is used. This will use fallback model metadata.`

影响：

- personality / effort / token window 等行为退化为 fallback
- 体验不稳定

#### P0-5. 当前 Zhipu base_url 与模型族语义仍有风险

现象：

- 日志里一直有：
  - `glm model is being sent to Zhipu coding endpoint; verify the configured base_url`

影响：

- 现在碰巧可用，不代表长期稳定
- 一旦智谱侧网关策略变化，`glm-4-flash` 可能再次出现兼容问题

---

### P1：第二阶段必须收敛

#### P1-1. Codex tool 语义仍不完整

当前虽然通过 capability 声明放行了：

- `function`
- `web_search`
- `namespace`
- `custom`

但本质上：

- 不是所有 tool type 都有一等映射
- 很多 tool 还在“尽量别拒绝”的兼容层面

#### P1-2. `previous_response_id` 仍然被 chat fallback 限制

上游 Codex 的连续上下文能力很依赖 response identity。
只要 rcodex 继续走 chat fallback，这个能力就只能部分模拟。

#### P1-3. WebSocket / Realtime 还没有真正进入设计主路径

当前重点还在 HTTP SSE。
但如果目标是“后续完善整个 proxy 功能”，那就不能只停在 SSE。

#### P1-4. 认证模型混乱

当前本机配置里 `rcodex` 同时用了：

- `requires_openai_auth = true`
- `experimental_bearer_token = ...`

对于一个本地代理 provider，这个组合语义不清晰，也不利于长期维护。

---

### P2：第三阶段优化项

#### P2-1. `src/handlers/mod.rs` 需要拆分

建议至少拆成：

- `responses_handler.rs`
- `responses_stream_state.rs`
- `responses_event_builder.rs`
- `chat_handler.rs`

#### P2-2. provider 能力声明需要系统化

现在 capability 有了基础，但还不够细。
后面应该把“支持什么 Responses 特性、支持什么 tool、哪些字段需要 lossy fallback”做成可测试声明。

#### P2-3. 建立针对上游 Codex fixture 的协议回归测试

最理想的方式不是只写本地单测，而是引入一批“以 Codex upstream fixture 为准”的回归样本。

---

## 5. 5.0 版本建议目标

我建议把 `codex5.0` 定义成下面这个目标，而不是泛泛“继续修”：

> 让 rcodex 从“能跑的 chat fallback 代理”升级为“协议边界清晰、可验证、接近原生 Codex provider 的 Responses 代理”

判断是否达标，至少看这 5 条：

1. `codex exec` 稳定输出 assistant 内容
2. `response.output_item.done` / `response.completed` 生成逻辑有完整回归测试
3. provider/config 不再硬编码双 provider
4. `glm-4-flash` 不再触发 unknown model fallback 警告
5. `previous_response_id` / tool 调用 / 多轮连续性有明确支持边界

---

## 6. 协议稳定化专项设计

### 6.1 目标

第一阶段只处理：

- `HTTP POST /v1/responses`
- `Codex CLI exec` 依赖的 Responses SSE 输出

目标不是把 rcodex 变成完整 native provider，而是把当前最关键的协议边界稳定下来：

- 只要上游 `chat/completions` 正常流式结束，rcodex 就必须稳定产出可被 Codex 消费的终止事件序列
- 协议兼容逻辑必须能在单测和黑盒测试里被反复验证
- 后续重构不能再依赖“手工执行一次 `codex exec` 看起来没坏”来判断安全性

### 6.2 本阶段范围

纳入范围：

- `chat.completion.chunk` 到 Responses SSE 的生命周期映射
- snake_case / camelCase 流式字段兼容
- 文本输出 item 的 added / delta / done / completed 收尾逻辑
- tool call 参数增量拼接与 done 事件收尾
- 最终 `[DONE]` 与 terminal usage 的稳定输出
- `/v1/responses` handler 的黑盒回归验证

明确不纳入范围：

- WebSocket / realtime
- provider registry / config map 重构
- native `/responses` provider 接入
- 大规模认证体系改造
- 全量 Codex feature parity

### 6.3 设计原则

第一原则是“协议状态机与 HTTP handler 解耦”。

当前问题不在于 axum 路由本身，而在于 [src/handlers/mod.rs](/Users/louloulin/Documents/linchong/claude/rcodex/src/handlers/mod.rs) 同时承担：

- 请求分发
- provider fallback 决策
- chat chunk 状态累积
- Responses SSE 事件生成
- 协议兼容补丁

这会让协议稳定化越来越依赖局部经验，而不是清晰边界。第一阶段应该把“协议如何生成事件”从“HTTP 如何返回事件”里拆开。

### 6.4 建议结构

建议把当前实现整理成下面的职责边界：

- `src/handlers/mod.rs`
  - 保留 HTTP 入口、provider 调用、SSE event 封装
- `src/handlers/responses_protocol.rs`
  - 暴露 `chat chunk -> responses payloads/events` 的高层接口
- `src/handlers/responses_stream_state.rs`
  - 保存流式状态机，例如：
    - 初始事件是否已发送
    - 当前 message item 的累计文本
    - tool call 参数累计状态
    - terminal usage / completed 触发条件
- `src/models/chat.rs`
  - 只负责 provider chunk 兼容反序列化

如果为了减少改动，也可以先不立即新建 2 个文件，而是先在现有文件内部按模块拆函数；但最终边界必须清晰到可以独立测试“协议状态推进”。

### 6.5 数据流设计

第一阶段固定走这条链路：

1. Codex CLI 发起 `/v1/responses`
2. rcodex 将 `ResponsesRequest` 转成 `ChatRequest`
3. provider 返回 `ChatCompletionChunk` 流
4. 协议状态机逐 chunk 推进
5. 状态机产出一组 Responses SSE payload
6. handler 仅负责把 payload 包成 SSE `Event`

关键约束：

- terminal chunk 的判定必须同时考虑：
  - `finish_reason`
  - usage 是否出现
- `response.output_item.done` 不能依赖 handler 猜测，而必须由协议状态机根据完成条件稳定生成
- `response.completed` 必须只在终局生成一次
- `[DONE]` 必须总是跟在最后

### 6.6 错误处理设计

本阶段优先保证“协议层不静默丢终止语义”。

要求：

- 如果 chunk 解析成功但终止字段缺失，应能从测试直接暴露回归
- 如果 provider 流结束但没有形成 completed 条件，协议层必须进入显式 fallback，而不是悄悄结束
- 如果生成的 payload 为空，必须记录错误并触发最小 completed fallback

这部分的核心不是“优雅”，而是“不可悄悄失败”。

### 6.7 测试设计

本阶段测试分 4 层。

#### A. 模型兼容反序列化测试

验证 [src/models/chat.rs](/Users/louloulin/Documents/linchong/claude/rcodex/src/models/chat.rs) 对以下字段的兼容：

- `finish_reason` / `finishReason`
- `tool_calls` / `toolCalls`
- terminal usage 字段

#### B. 协议状态机测试

直接测试 `chunk -> payloads/events`：

- 只有 delta 的普通文本流
- 最后一帧空文本但有 `finish_reason`
- tool call arguments 分段输入
- usage 只在最后一帧出现
- provider 提前结束时的 fallback completed

#### C. 黑盒 handler 测试

直接验证 `/v1/responses` 返回体包含完整事件：

- `response.output_item.added`
- `response.output_text.delta`
- `response.output_item.done`
- `response.completed`
- `[DONE]`

#### D. 真命令回归验证

固定保留这条验收命令：

```bash
codex --config model_provider=rcodex --config model=glm-4-flash exec "What is 2+2?"
```

通过标准：

- 有 assistant 最终输出
- 没有“stream closed before response.completed”类回归

### 6.8 验收标准

协议稳定化完成的标志不是“修过一个 bug”，而是同时满足：

1. `cargo test` 中存在固定的协议回归测试集合
2. 文本流和 tool call 流都能产出 `response.output_item.done`
3. `/v1/responses` 的 handler 黑盒测试覆盖 completed 终局
4. 真命令 `codex exec` 可重复通过
5. 协议逻辑的改动不再必须在巨型 handler 中到处找副作用

### 6.9 推荐实施方式

第一阶段推荐采用“协议核心抽离型”而不是“继续打补丁”。

原因：

- 最小补丁虽然快，但会继续放大 [src/handlers/mod.rs](/Users/louloulin/Documents/linchong/claude/rcodex/src/handlers/mod.rs) 的耦合
- 一步到位重构又超出当前边界
- 协议核心抽离正好适合“先稳定，再扩展”

所以协议稳定化的推荐顺序是：

1. 固化现有 bug 的测试样本
2. 抽离协议状态推进逻辑
3. 让 handler 只负责 HTTP 与 SSE 封装
4. 用黑盒测试和 `codex exec` 进行最终验收

---

## 7. 建议实施路线

### 阶段 A：协议稳定化

目标：

- 把“现在能跑”的兼容层变成“不会轻易回归”的兼容层

任务：

1. 系统补齐 snake_case / camelCase 兼容矩阵
2. 固化 Responses SSE 生命周期测试
3. 增加真实上游 chunk fixture 测试
4. 增加 `codex exec` 黑盒回归脚本

完成标志：

- 任意一次修改流式协议代码，都能通过固定回归集

### 阶段 B：架构重构

目标：

- 把 handler 中的协议状态机抽离出来

任务：

1. 从 `src/handlers/mod.rs` 拆出 `responses streaming state machine`
2. 把 event builder 与 HTTP handler 解耦
3. 把 provider capability / fallback plan 做成单独模块

完成标志：

- 协议逻辑可以单测，不依赖 axum handler 才能验证

### 阶段 C：provider/config 升级

目标：

- 对齐 Codex 上游的 provider 思路

任务：

1. 把 `ProvidersConfig` 改成 map 结构
2. 引入 provider registry
3. 让模型路由、header、重试、能力声明都配置化
4. 为 `rcodex` provider 增加明确的 model metadata 配置

完成标志：

- 新增 provider 不需要再改硬编码分支

### 阶段 D：能力对齐

目标：

- 让 rcodex 更接近“原生 Codex provider”

任务：

1. 明确 `previous_response_id` 的支持策略
2. 补齐 tool 调用 item 级语义
3. 评估并设计 websocket/realtime 支持
4. 清理认证策略与 provider 定义语义

完成标志：

- 不再只是“能把纯文本答复转出来”
- 而是能稳定承接更真实的 Codex 工作流

---

## 8. 我建议的优先执行顺序

### 第一优先级

先做：

- 阶段 A：协议稳定化

原因：

- 这是所有后续工作的地基
- 不先把 SSE 与终止语义锁死，后面的重构都容易反复回归

### 第二优先级

再做：

- 阶段 B：架构重构

原因：

- 当前最大维护成本在 `src/handlers/mod.rs`
- 不拆这个文件，后续每次补协议都会越来越危险

### 第三优先级

然后做：

- 阶段 C：provider/config 升级

原因：

- 这是把 rcodex 从“项目级 hack”提升到“可扩展代理框架”的关键一步

### 第四优先级

最后做：

- 阶段 D：能力对齐

原因：

- 这是 5.0 真正走向完整代理的阶段
- 但前提必须是协议层和架构层已经稳住

---

## 9. 阶段性结论

到 2026-04-21 这一步，rcodex 的判断应该是：

- 不是“完全不可用”
- 也不是“已经完整兼容 Codex”
- 而是“刚刚跨过最关键的协议终止问题，进入可以系统收敛的阶段”

最核心的事实有两个：

1. 本次 `codex --config model_provider=rcodex --config model=glm-4-flash exec "What is 2+2?"` 已经修复成功
2. rcodex 还没有完成从 `chat fallback proxy` 到 `Codex-native-style responses proxy` 的架构升级

所以后续最该做的，不是继续零散补洞，而是围绕这份 5.0 路线图分阶段推进。

---

## 10. 上游 Codex 代码与协议架构总览

虽然用户要求“全面分析整个 codex 目录整个代码”，但真正决定 `rcodex` 是否能与 Codex CLI 稳定协作的，不是所有 crate 都同等重要，而是下面这条分层链路。

### 10.1 最关键的 crate 分层

#### A. 入口层

- `codex/codex-rs/exec`
  - CLI 入口，负责读取配置、启动会话、把最终输出打印到终端
- `codex/codex-rs/tui`
  - 交互式 UI，消费更完整的 turn / item / delta 事件
- `codex/codex-rs/app-server*`
  - 桌面端与线程化交互协议层，和本次 `exec` 问题不是一条主链，但共享同一套核心 turn 协议

#### B. 核心调度层

- `codex/codex-rs/core`
  - 真正的 Codex 引擎
  - 负责 turn 生命周期、工具执行、流式事件消费、conversation state、response bookmark 续接

#### C. 模型传输层

- `codex/codex-rs/codex-api`
  - 负责把 `/v1/responses` 的 HTTP / SSE / WebSocket 细节封装成统一的 `ResponseEvent` 流

#### D. 协议模型层

- `codex/codex-rs/protocol`
  - 定义 `ResponseItem`、`ResponseInputItem`、turn event、工具输出等核心模型
- `codex/codex-rs/model-provider-info`
  - 定义 provider 元信息与 `wire_api`
  - 现在只接受 `responses`

#### E. 元数据与辅助层

- `codex/codex-rs/models-manager`
  - 负责模型元数据、fallback model metadata 警告
- `codex/codex-rs/otel`
  - 记录响应事件、completed、异常关闭等 telemetry
- `codex/codex-rs/responses-api-proxy`
  - 这是一个“严格只转发 `POST /v1/responses`”的参考实现，对 rcodex 的边界设计很有启发

### 10.2 上游真正的职责分离

上游 Codex 并不是“CLI 直接处理 provider 原始 JSON”，而是明确分层：

1. `exec` 负责用户入口与输出模式
2. `core` 负责 turn 和 item 生命周期
3. `codex-api` 负责把 SSE / WS 转成强类型 `ResponseEvent`
4. `protocol` 负责定义 item 的合法形状
5. provider config 只声明“这个 provider 说的是什么 wire protocol”

这也是 rcodex 当前最需要借鉴的地方：

- 上游把“HTTP 传输”与“协议语义”分开了
- rcodex 目前还把“handler、兼容补丁、状态机、事件生成”揉在一起

### 10.3 对 rcodex 最关键的上游结论

从上游代码看，当前 Codex 的基本前提已经非常明确：

- provider 侧必须说 `Responses` 语言，而不是 `Chat` 语言
- CLI / core 不关心你后端是不是 OpenAI，只关心你是否稳定输出符合 `ResponseEvent` 预期的事件序列
- 只要事件序列缺终局、缺 item 生命周期、或者 item 结构不合法，Codex 就会把它视为“流没完成”或“没有可展示的 assistant 输出”

换句话说：

> `rcodex` 的本质任务不是“把别家模型接进来”，而是“把别家模型稳定翻译成上游 Codex 认可的 Responses 协议”

---

## 11. 上游 Codex 核心协议不变量

这一节是本次最关键的分析结果。它不是宽泛的“兼容 OpenAI 风格”，而是上游代码里已经写死的行为约束。

### 11.1 请求侧不变量

从 [codex/codex-rs/codex-api/src/common.rs](/Users/louloulin/Documents/linchong/claude/rcodex/codex/codex-rs/codex-api/src/common.rs) 可以看到，`ResponsesApiRequest` 的稳定核心字段是：

- `model`
- `instructions`
- `input: Vec<ResponseItem>`
- `tools`
- `tool_choice`
- `parallel_tool_calls`
- `reasoning`
- `store`
- `stream`
- `include`
- `text`

而从 [codex/codex-rs/core/src/client.rs](/Users/louloulin/Documents/linchong/claude/rcodex/codex/codex-rs/core/src/client.rs) 可以看到，`previous_response_id` 是续接语义的一部分，不是“锦上添花”的可选字段。

结论：

- 即使一期只做 `exec`，也不能把 `/v1/responses` 看成“只有 prompt 和 stream=true 的简化接口”
- 只要后续要支持更真实的多轮或 websocket 续接，`response_id` / `previous_response_id` 语义迟早要补正

### 11.2 事件侧不变量

从 [codex/codex-rs/codex-api/src/sse/responses.rs](/Users/louloulin/Documents/linchong/claude/rcodex/codex/codex-rs/codex-api/src/sse/responses.rs) 与 [codex/codex-rs/core/src/codex.rs](/Users/louloulin/Documents/linchong/claude/rcodex/codex/codex-rs/core/src/codex.rs) 可以提炼出下面这组硬约束。

#### 不变量 1：`response.completed` 是流的法定终局

如果流结束前没有 `response.completed`，上游会明确视为错误：

- `stream closed before response.completed`

这说明：

- 不能只发 `[DONE]`
- 也不能只发 `response.output_item.done`
- `response.completed` 必须出现，而且只能在终局出现

#### 不变量 2：assistant 文本是 item 生命周期，不只是 delta 文本

上游真正消费的是：

1. `response.output_item.added`
2. `response.output_text.delta`
3. `response.output_item.done`
4. `response.completed`

其中：

- `delta` 负责流式体验
- `done` 负责把最终 item 固化进 turn state
- `completed` 负责关闭整个 response

所以“有 delta 没 done”是不够的，“有 done 没 completed”也不够。

#### 不变量 3：`OutputTextDelta` 必须挂在一个 active item 上

上游在处理 `ResponseEvent::OutputTextDelta` 时，要求当前必须已经有 active item。
也就是说，先有 `response.output_item.added`，后有 `response.output_text.delta`，这个顺序不能乱。

如果只推 delta，不先推 added，上游逻辑会进入异常分支：

- `OutputTextDelta without active item`

#### 不变量 4：message item 的内容结构必须合法

上游测试 fixture 和协议模型都表明，assistant message 的 content 不是任意数组，而是合法 `ResponseItem::Message` 内容。
对普通文本来说，最稳妥的最小形状就是：

```json
{
  "type": "message",
  "role": "assistant",
  "content": [
    { "type": "output_text", "text": "" }
  ]
}
```

这也解释了为什么本次把 `response.output_item.added` 从空 content 数组改为带空 `output_text` 占位后，Codex CLI 的表现更稳定。

#### 不变量 5：`response_id` 来源于 `response.completed`

根据 [codex/codex-rs/docs/protocol_v1.md](/Users/louloulin/Documents/linchong/claude/rcodex/codex/codex-rs/docs/protocol_v1.md)，turn 完成后，Codex 会把最终 `response.completed.response.id` 保存起来，用于后续 thread 续接。

所以 `response.completed` 不是单纯收尾事件，它还承担：

- turn 成功 bookmark
- conversation 续接锚点
- `previous_response_id` 的来源

### 11.3 一期最该锁死的最小事件序列

对 `codex exec` 纯文本问答来说，最小稳定序列应该固定为：

```text
response.created                       (可选但建议保留)
response.output_item.added            (assistant message with empty output_text)
response.output_text.delta            (0..n)
response.output_item.done             (assistant final message)
response.completed                    (contains response.id, optional usage)
[DONE]
```

只要 rcodex 的一期协议测试不能把这条序列锁死，后续每次改动都有较高回归概率。

---

## 12. rcodex 当前协议偏差图谱

基于上游代码与 rcodex 当前实现对照，当前偏差不是一个点，而是四层问题叠在一起。

### 12.1 请求语义偏差

当前 rcodex 是：

- 接收 `ResponsesRequest`
- 再转为 `ChatRequest`
- 再发给 Zhipu `/chat/completions`

这意味着它天生存在 lossy mapping：

- `previous_response_id` 没有原生落点
- response item 级语义要靠本地重建
- tool / reasoning / continuation 只能做近似映射

这不是 bug，而是当前架构决定的上限。

### 12.2 事件生命周期偏差

当前 rcodex 的协议关键点都挤在 [src/handlers/mod.rs](/Users/louloulin/Documents/linchong/claude/rcodex/src/handlers/mod.rs)：

- handler 做请求入口
- handler 兼做流式状态机
- handler 兼做 Responses 事件 builder
- handler 兼做 provider 兼容补丁

这会导致一个实际后果：

- 任何局部字段兼容问题，最终都会在巨型 handler 里表现为“终止事件没发对”

本次修复的 snake_case / camelCase 问题，本质上就是这个架构耦合的典型副作用。

### 12.3 状态续接偏差

上游 Codex 里：

- `response.completed.response.id` 会被保存
- 后续请求会在适合的条件下携带 `previous_response_id`

当前 rcodex 一期即使先不做 websocket，也必须承认一个事实：

- 只要继续走 `chat/completions` fallback，就无法天然拥有与上游等价的 response identity 语义

因此必须在计划里显式区分：

- 一期：保证单轮 `exec` 文本流稳定
- 二期：明确 `response_id` / `previous_response_id` 的模拟或替代策略

### 12.4 模型与 provider 元信息偏差

当前还存在两个明显的元信息问题：

#### A. `glm-4-flash` 在上游 Codex 里是 unknown model

从 [codex/codex-rs/models-manager/src/model_info.rs](/Users/louloulin/Documents/linchong/claude/rcodex/codex/codex-rs/models-manager/src/model_info.rs) 可以看到，未知模型会退回 fallback metadata。

影响：

- reasoning / verbosity / tool behavior 退化
- 用户体验受 fallback 模型策略影响

#### B. Zhipu base_url 与模型族仍有不确定性

本地实现里明确有警告：

- `glm model is being sent to Zhipu coding endpoint; verify the configured base_url`

这意味着：

- 当前“跑通”并不等于“配置已正确”
- proxy 的协议层稳定之后，provider 元信息与 endpoint 语义也要收敛

### 12.5 测试边界偏差

上游 Codex 的测试思路非常清晰：

- 有 SSE fixture
- 有 `/v1/responses` mock server
- 有历史续接与 `previous_response_id` 断言
- 有“缺少 completed 就失败”的负例测试

而 rcodex 当前虽然已经补上关键兼容测试，但整体回归网还不够厚，尤其缺：

- fixture 化的 provider chunk 样本库
- 完整的最小事件序列黑盒测试
- “流关闭但未 completed”类负例测试

---

## 13. 协议架构图（ANSI 文本）

### 13.1 上游 Codex 的核心协议链路

```text
+------------------+        +-------------------+        +----------------------+
| codex exec / tui | -----> | codex-core        | -----> | codex-api client     |
| user entrypoint  |        | turn state        |        | /v1/responses client |
+------------------+        +-------------------+        +----------------------+
         ^                             |                              |
         |                             v                              v
         |                  consumes ResponseEvent            HTTP POST /v1/responses
         |                  - output_item.added                        |
         |                  - output_text.delta                        |
         |                  - output_item.done                         |
         |                  - response.completed                       |
         |                             ^                               |
         |                             |                               v
         +-----------------------------+----------------------- SSE response stream
```

### 13.2 上游 `exec` 视角下的最小稳定事件序列

```text
request start
   |
   v
response.created                   optional but recommended
   |
   v
response.output_item.added         create active assistant item
   |
   v
response.output_text.delta*        stream visible text
   |
   v
response.output_item.done          finalize assistant message item
   |
   v
response.completed                 finalize response + expose response_id
   |
   v
[DONE]
```

### 13.3 当前 rcodex 的实际代理链路

```text
+------------------+       ResponsesRequest        +-------------------------+
| Codex CLI exec   | ----------------------------> | rcodex /v1/responses    |
+------------------+                               +-------------------------+
                                                              |
                                                              v
                                           transform_responses_to_chat_request
                                                              |
                                                              v
                                                Zhipu /chat/completions stream
                                                              |
                                                              v
                                                ChatCompletionChunk sequence
                                                              |
                                                              v
                                  rcodex handler-local state machine + SSE builder
                                                              |
                                                              v
                          response.output_item.added / delta / done / completed / [DONE]
```

### 13.4 当前 rcodex 的核心薄弱点

```text
provider chunk field mismatch
   |
   +--> finish_reason missing
   |
   +--> tool_calls missing
   |
   v
handler state machine cannot infer terminal state reliably
   |
   +--> output_item.done may be skipped
   |
   +--> completed may become fragile fallback
   |
   v
Codex CLI sees "no finalized assistant item"
   |
   v
exec output disappears even though upstream model answered
```

### 13.5 一期目标架构：协议核心抽离型

```text
+------------------------+        +----------------------------+        +------------------+
| HTTP handler layer     | -----> | Responses protocol core    | <----- | provider chunks  |
| - parse request        |        | - stream state             |        | ChatCompletion   |
| - call provider        |        | - event sequencing         |        | chunk stream     |
| - wrap SSE event       |        | - done/completed rules     |        +------------------+
+------------------------+        | - fallback invariants      |
                                  +----------------------------+
                                                |
                                                v
                               deterministic SSE payload sequence for Codex
```

### 13.6 二期目标架构：从 chat fallback 走向 provider-native thinking

```text
Phase 1:
  Responses request ----> chat fallback ----> protocol core ----> stable SSE

Phase 2:
  Responses request ----> provider registry ----> native responses or explicit adapter
                                            |
                                            +--> response_id / previous_response_id policy
                                            +--> capability declaration
                                            +--> model metadata alignment
```

---

## 14. 第一阶段优先计划：只做 `/v1/responses` 协议稳定化

这一阶段不求“大而全”，只求把 `Codex CLI exec` 所依赖的协议边界做成稳定地基。

### 14.1 目标

固定目标只有一个：

> 只要上游 provider 正常结束流式输出，rcodex 就必须稳定地产出上游 Codex 可消费的完整 Responses SSE 终局序列。

### 14.2 必做任务

#### 任务 1：固化协议不变量测试

- 固化普通文本流的：
  - `added -> delta -> done -> completed -> [DONE]`
- 固化 tool call 流的 done 终局
- 固化“缺少 completed 必须失败”的负例

#### 任务 2：抽离协议核心

- 从 `src/handlers/mod.rs` 抽出：
  - stream state
  - terminal detection
  - event builder
- handler 只保留：
  - request orchestration
  - provider invocation
  - SSE 包装

#### 任务 3：建立 provider chunk 兼容矩阵

- 明确兼容字段：
  - `finish_reason` / `finishReason`
  - `tool_calls` / `toolCalls`
  - terminal usage 形状
- 为常见 provider 方言建立 fixture

#### 任务 4：建立黑盒验收

至少固定这条命令为验收样例：

```bash
codex --config model_provider=rcodex --config model=glm-4-flash exec "What is 2+2?"
```

通过标准：

- 有 assistant 最终输出
- 无 `stream closed before response.completed`
- 无“只有 user 没有 assistant”的回归

### 14.3 明确暂缓项

一期明确不做：

- websocket / realtime
- provider registry 重构
- native responses provider 接入
- 全量 tool parity
- 全量模型元数据系统化

这些都重要，但不应该和一期的协议地基抢优先级。

### 14.4 一期完成标志

只要下面 5 条同时满足，一期就可以判定完成：

1. `codex exec` 单轮文本问答稳定
2. `response.output_item.done` 与 `response.completed` 有固定回归测试
3. handler 与协议状态机职责分离
4. provider 字段方言兼容可测试
5. 文档中明确记录当前支持边界与未支持边界

### 14.5 当前已实现状态（2026-04-21）

本轮按“最佳最小方式”优先完成了一期里最关键的协议收敛，而不是继续扩散到 provider registry 或 websocket。

已完成：

- [x] 将 `chat chunk -> Responses SSE payload` 的协议核心抽离到 [src/protocol/events.rs](/Users/louloulin/Documents/linchong/claude/rcodex/src/protocol/events.rs)
- [x] 新增 `ResponsesChatStreamState`，让流式状态推进不再只存在于 [src/handlers/mod.rs](/Users/louloulin/Documents/linchong/claude/rcodex/src/handlers/mod.rs) 的 handler 内部
- [x] 让 `/v1/responses` 主路径实际改为委托 `src/protocol/events.rs` 生成 chat-fallback 的 Responses 协议 payload
- [x] 为协议核心新增独立测试：
  - `test_chat_chunk_protocol_core_emits_minimal_exec_lifecycle`
  - `test_chat_chunk_protocol_core_tracks_stream_state_across_chunks`
- [x] 保持既有 handler / protocol / e2e 测试通过
- [x] 真实执行黑盒命令验证通过：

```bash
codex --config model_provider=rcodex --config model=glm-4-flash exec "What is 2+2?"
```

本轮黑盒结果：

- 有 assistant 最终输出
- 命令退出码为 `0`
- 输出文本为：

```text
The calculation of 2+2 is straightforward. It equals 4.
```

本轮验证命令：

```bash
cargo test --quiet
cargo build --quiet
codex --config model_provider=rcodex --config model=glm-4-flash exec "What is 2+2?"
```

验证结果：

- `cargo test --quiet` 通过
- `cargo build --quiet` 通过
- `codex exec` 真实链路通过

仍未完成，保留到后续阶段：

- [ ] 为其余 `glm-*` 模型补齐 `model_messages` / personality 元数据
- [ ] `previous_response_id` 的真实连续性策略
- [ ] provider registry / config map 化
- [ ] websocket / realtime 路径

结论：

当前一期可以认定“协议核心抽离”已经开始落地，并且 `Codex CLI exec` 的最关键单轮文本链路在真实命令下保持通过；但 5.0 还没有进入 provider-native 或多轮连续性阶段。

### 14.6 当前已实现状态补充（glm-5 / model metadata，2026-04-21）

本轮继续按“最佳最小方式”推进，没有改动 rcodex 的协议处理代码，而是先把 `Codex -> model catalog -> rcodex` 这一层跑通并验证清楚。

已完成：

- [x] 确认 rcodex 已有可用模型目录文件：[codex/rcodex/models.json](/Users/louloulin/Documents/linchong/claude/rcodex/codex/rcodex/models.json)
- [x] 确认 `codex` 原生支持 `model_catalog_json`
- [x] 定位并修正全局配置未生效的真实根因：
  - 不是 `model_catalog_json` 字段不支持
  - 而是 `~/.codex/config.toml` 里该字段之前被错误放进了 `[model_providers.yunyi]` 表作用域
  - 修正为顶层字段后，`codex exec` 已能自动加载 rcodex 模型目录
- [x] 完成 `glm-5` 真实黑盒验证：

```bash
codex --config model_provider=rcodex --config model=glm-5 exec "What is 2+2?"
```

本轮 `glm-5` 黑盒结果：

- 命令退出码为 `0`
- 有 assistant 最终输出
- 输出文本包含：

```text
Two plus two equals **4**.
```

- `Unknown model glm-5 is used. This will use fallback model metadata.` 不再出现

本轮补充验证命令：

```bash
cargo test --quiet
cargo build --quiet
curl -s http://127.0.0.1:9080/health
codex debug prompt-input -c model_provider=rcodex -c model=glm-5 "What is 2+2?"
codex --config model_provider=rcodex --config model=glm-5 exec "What is 2+2?"
```

补充验证结果：

- `cargo test --quiet` 通过：
  - `236 passed, 4 ignored`
- `cargo build --quiet` 通过
- 代理健康检查返回 `OK`
- `codex debug prompt-input` 在不显式传 `model_catalog_json` 的情况下可直接返回，说明全局 model catalog 已被加载
- `codex exec` 真实链路通过

当前边界：

- `glm-5` 的 unknown-model fallback 已解决
- `glm-5` 仍有一条剩余告警：
  - `Model personality requested but model_messages is missing, falling back to base instructions.`
- 这说明后续待做项已经从“模型不可识别”收敛为“personality 模板元数据未补齐”

结论：

`P0-4 model metadata` 的“模型可识别 + 不再走 unknown fallback”这一层，已经用最小方式落地并完成 `glm-5` 真实验证；后续应把重点收缩到 `model_messages` / personality 元数据，而不是继续排查 `model_catalog_json` 是否生效。

### 14.7 当前已实现状态补充（glm-5 / personality metadata，2026-04-21）

本轮继续按“最佳最小方式”推进，只对 `glm-5` 增加最小可用的 personality 元数据，不扩散修改其它模型目录项，也没有改动代理协议逻辑。

已完成：

- [x] 在 [tests/glm5_e2e_tests.rs](/Users/louloulin/Documents/linchong/claude/rcodex/tests/glm5_e2e_tests.rs) 新增回归测试：
  - `test_glm5_model_catalog_includes_personality_messages`
- [x] 先跑出失败用例，再最小修改 [codex/rcodex/models.json](/Users/louloulin/Documents/linchong/claude/rcodex/codex/rcodex/models.json)
- [x] 为 `glm-5` 模型项补齐：
  - `model_messages.instructions_template`
  - `instructions_variables.personality_default`
  - `instructions_variables.personality_friendly`
  - `instructions_variables.personality_pragmatic`
- [x] 重新执行真实 `codex` 验证：

```bash
codex --config model_provider=rcodex --config model=glm-5 exec "What is 2+2?"
```

本轮 `glm-5` 黑盒结果：

- 命令退出码为 `0`
- assistant 正常输出 `4`
- 本轮日志中不再出现：
  - `Model personality requested but model_messages is missing, falling back to base instructions.`

本轮补充验证命令：

```bash
cargo test --quiet test_glm5_model_catalog_includes_personality_messages --test glm5_e2e_tests
cargo build --quiet
curl -s http://127.0.0.1:9080/health
codex debug prompt-input -c model_provider=rcodex -c model=glm-5 "What is 2+2?"
codex --config model_provider=rcodex --config model=glm-5 exec "What is 2+2?"
```

补充验证结果：

- 目标回归测试通过：
  - `1 passed`
- `cargo build --quiet` 通过
- 代理健康检查返回 `OK`
- `codex debug prompt-input` 可正常返回
- `codex exec` 真实链路通过

验证边界：

- 本轮独立执行 `cargo test --quiet` 时，仓库里仍有一个与本次改动无直接关联的失败：
  - `tests/integration_tests.rs` 中的 `test_binary_creates_and_truncates_log_file_on_startup`
- 该失败发生在日志文件启动截断路径，和本轮 `glm-5 model_messages` 修改不是同一条功能链
- 因此本轮可确认：
  - `glm-5 personality metadata` 已最小落地并通过真实 `codex exec`
  - 仓库全量测试当前仍存在一个独立待处理项

结论：

`glm-5` 的 model metadata 现在已经同时覆盖了“模型可识别”和“personality 模板存在”两层最小能力；后续下一步应转向：

- 处理其余 `glm-*` 模型的 personality 元数据一致性
- 单独排查 `test_binary_creates_and_truncates_log_file_on_startup` 的稳定性问题

### 14.8 当前已实现状态补充（启动日志测试稳定化，2026-04-21）

本轮继续按“最佳最小方式”推进，没有改动生产日志逻辑，而是把上一轮留下的验证阻塞收敛到测试层。

根因定位：

- [x] 确认 [src/logging.rs](/Users/louloulin/Documents/linchong/claude/rcodex/src/logging.rs) 中的 `prepare_log_file()` 本身已经会在启动时截断旧文件
- [x] 通过真实二进制最小复现确认：
  - `logs/server.log` 中的 stale 内容会被清掉
  - 启动日志也会正常写入
- [x] 因此锁定根因是 [tests/integration_tests.rs](/Users/louloulin/Documents/linchong/claude/rcodex/tests/integration_tests.rs) 中
  - `test_binary_creates_and_truncates_log_file_on_startup`
  - 依赖固定 `sleep(600ms)`，存在启动时序脆弱性

已完成：

- [x] 将该集成测试改为“有上限的轮询等待”
- [x] 等待条件改为同时满足：
  - stale 日志内容已被移除
  - `Starting OpenAI Proxy Server` 已实际写入日志
- [x] 清理本轮为了测试修复引入的局部 warning

本轮补充验证命令：

```bash
cargo test --quiet test_binary_creates_and_truncates_log_file_on_startup --test integration_tests
cargo test --quiet
cargo build --quiet
curl -s http://127.0.0.1:9080/health
codex --config model_provider=rcodex --config model=glm-5 exec "What is 2+2?"
```

补充验证结果：

- 启动日志集成测试通过：
  - `1 passed`
- 全量测试通过：
  - `237 passed, 4 ignored`
- `cargo build --quiet` 通过
- 代理健康检查返回 `OK`
- `glm-5` 真实 `codex exec` 继续通过，并正常输出 `4`

当前结论：

- 上一轮留下的“仓库全量测试当前仍存在一个独立待处理项”已收敛完成
- 当前最小已验证状态变为：
  - `glm-5` 的 unknown-model fallback 已解决
  - `glm-5` 的 personality fallback 已解决
  - 全量 `cargo test --quiet` 已恢复通过

后续下一步应继续聚焦：

- 为其余 `glm-*` 模型补齐 personality 元数据一致性
- 再进入更大的 `previous_response_id` / provider registry 阶段

### 14.9 当前已实现状态补充（其余 `glm-*` personality 元数据一致性，2026-04-21）

本轮继续严格按“最佳最小方式”推进，没有扩展 provider 架构，也没有改动上游 Codex 源码，只补齐了模型 catalog 中剩余 `glm-*` 条目的 personality 元数据一致性，并先用回归测试锁定行为。

根因定位：

- [x] 确认 [codex/rcodex/models.json](/Users/louloulin/Documents/linchong/claude/rcodex/codex/rcodex/models.json) 中：
  - `glm-5` 已有 `model_messages`
  - `glm-5.1` / `glm-4-flash` / `glm-4` 仍为 `null`
- [x] 确认这会导致不同 `glm-*` 模型在 Codex CLI 中的 personality 行为不一致
- [x] 先在 [tests/glm5_e2e_tests.rs](/Users/louloulin/Documents/linchong/claude/rcodex/tests/glm5_e2e_tests.rs) 新增“所有 `glm-*` 条目都必须具备可用 personality 元数据”的回归测试
- [x] 红灯验证命中根因：
  - 新测试首次失败于 `glm-5.1 should provide a personality-aware instructions template`

已完成：

- [x] 为 `glm-5.1` 补齐与 `glm-5` 一致的最小 `model_messages`
- [x] 为 `glm-4-flash` 补齐与 `glm-5` 一致的最小 `model_messages`
- [x] 为 `glm-4` 补齐与 `glm-5` 一致的最小 `model_messages`
- [x] 保持改动边界最小：
  - 只修改 model catalog
  - 不改 provider 路由
  - 不改协议状态机

本轮补充验证命令：

```bash
rtk cargo test --quiet test_all_glm_models_include_personality_messages
rtk cargo test --quiet
rtk cargo build --quiet
rtk curl -s http://127.0.0.1:9080/health
rtk codex --config model_provider=rcodex --config model=glm-5 exec "What is 2+2?"
```

补充验证结果：

- 新增 `glm-*` personality 一致性回归测试通过：
  - `1 passed`
- 全量测试通过：
  - `238 passed, 4 ignored`
- `cargo build --quiet` 通过
- 代理健康检查返回 `OK`
- `glm-5` 真实 `codex exec` 通过，并正常输出 `4`

当前结论：

- 现有 model catalog 中全部 `glm-*` 条目已经具备一致的 personality 元数据基础
- `glm-5` 的真实执行链路在本轮最小实现后仍保持可用
- 第一阶段里“model metadata / personality metadata”这条最小收敛线已经基本补齐

后续下一步应继续聚焦：

- 进入 `previous_response_id` 能力边界的真实支持或显式约束
- 推进 provider registry / config 从硬编码向可扩展映射演进
