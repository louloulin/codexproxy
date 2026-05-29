# rcodex Codex CLI Proxy Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `rcodex` 从当前的 Chat-first 兼容代理，演进为可稳定支撑 `Codex CLI` 的代理服务，完整承接 `Codex CLI` 的 Responses 协议语义，并提供与 OpenAI 风格接口之间的可靠转换，同时明确 `glm-5` 的支持边界与落地方案。

**Architecture:** 现有系统已经具备多 provider 路由、Responses/Chat 转换和 SSE 基础能力，但协议保真度不足。后续改造的核心不是继续堆补丁，而是引入更清晰的 internal canonical model、provider capability negotiation、Responses-first 执行路径和统一的 streaming event builder，使 `/v1/responses` 成为一等入口，同时保留 `/v1/chat/completions` 的兼容能力。

**Tech Stack:** Rust, Axum, Tokio, Reqwest, Serde, Futures, SSE, OpenAI-compatible API, Zhipu GLM provider, Cargo test

---

## 一、计划背景与现状判断

这份 `plan4.md` 建立在当前整个仓库的真实代码基础上，重点不是复述已有功能，而是把“当前项目能做什么、做不到什么、为什么做不到、后面如何一步步补齐”写成可执行路线。

### 当前项目的真实状态

当前 `rcodex` 已经具备：

- OpenAI provider 与 Zhipu provider 的接入
- `/v1/chat/completions` 和 `/v1/responses` 两类 HTTP 接口
- Responses 与 Chat 间的基础协议转换
- SSE 基础流式处理
- `glm-5` 的基础调用能力

但当前项目仍然不是一个真正的 Codex CLI 协议代理，原因主要有 4 类：

1. `Responses` 不是系统内的一等协议
2. 内部模型无法完整承载 Codex CLI 的工具与事件语义
3. transform 层对字段和工具的保真支持不足
4. 流式链路对文本较友好，对工具和复杂事件支持不完整

### 当前最需要统一的认识

`rcodex` 现在不是“完全不支持 Codex CLI”，而是“已经具备部分基础，但还不能稳定承接 Codex CLI 的核心协议语义”。

因此后续改造目标不是：

- 仅让一个 demo 请求通

而是：

- 让 Codex CLI 的常用请求、工具调用、流式事件和状态续接，在代理层具有明确、可测试、可解释的行为。

### 外部协议调研结论

除了本仓代码分析，后续规划还应以两类外部事实为依据：

1. OpenAI 官方文档对 `Responses API` 与 `Chat Completions API` 的定位差异
2. 本地 Codex 仓库中围绕 `/responses`、SSE 事件、`previous_response_id` 的实际使用线索

#### OpenAI 官方结论

根据 OpenAI 官方文档：

- `Responses API` 被定位为 `Chat Completions API` 的超集
- OpenAI 明确建议新项目优先采用 `Responses API`
- `Responses API` 支持基于 `items` 的输入结构，而不是单一 `messages[]`
- `Responses API` 明确支持 `previous_response_id`
- 当 `previous_response_id` 与 `instructions` 一起使用时，上一轮的 instructions 不会自动继承
- `Chat Completions API` 仍然被支持，但它的工具和流式语义与 Responses 并不等价

这些结论直接意味着：

- 不能把 `Responses` 简化理解成“只是 Chat 的另一种包裹形式”
- `Codex CLI` 如果围绕 `Responses` 工作，那么代理层必须优先保障 Responses 语义，而不是先压平到 Chat 再勉强恢复

#### 本地 Codex 仓库线索

本地 Codex 仓库中可以观察到：

- 测试与工具更偏向 `/responses` 请求和 SSE 事件序列
- `response.output_item.done`
- `response.completed`
- `previous_response_id`

这些点在本地仓库和已有分析文档中被反复提及，说明对 Codex CLI 来说，下面这些能力不是可选增强，而是协议核心：

- item 级事件生命周期
- 多轮状态续接
- 流式完成事件的完整性
- `/responses` 作为主协议入口

因此，`plan4.md` 的后续任务必须围绕这条协议现实展开，而不是继续把重点放在普通 Chat 兼容。

---

## 二、当前代码问题总览

在正式拆任务之前，先把问题总结为可执行改造对象。

### 问题 A：架构仍然是 Chat-first

当前 `LLMProvider` 已有 `responses()` 和 `responses_streaming()` 接口，但系统整体的设计重心仍在 Chat 路径，尤其对 Zhipu 的支持依然主要依赖：

- `Responses -> Chat`
- `provider.chat()`
- `Chat -> Responses`

这意味着所有 Responses 独有语义都只能被压缩进 Chat 能表达的子集。

### 问题 B：工具系统模型过窄

当前 `src/models/chat.rs` 中的 `Tool` 只有：

- `type`
- `function`

这不足以保真承载：

- `web_search`
- `file_search`
- `computer_use`
- `mcp`

的专有字段，因此工具定义会在 transform 中丢失。

### 问题 C：Responses 字段承接不完整

当前 `ResponsesRequest` 虽然已经定义了很多字段，但在 `transform_responses_to_chat_request()` 和 `transform_chat_to_responses_request()` 中，大量字段没有真正透传，例如：

- `previous_response_id`
- `store`
- `reasoning`
- `structured_output`
- `metadata`
- `include`
- `prompt_cache_key`

### 问题 D：streaming 对 tool events 支持不足

当前系统对 `response.output_text.delta` 的支持已经有基础，但 Codex CLI 更依赖的是：

- `response.output_item.added`
- `response.function_call_arguments.delta`
- `response.function_call_arguments.done`
- `response.output_item.done`
- `response.completed`

当前这些事件在不同路径下行为不一致，尤其 streamed path 和 collected path 逻辑分裂明显。

### 问题 E：缺少明确的 provider capability negotiation

当前系统在选择 provider 后，并没有显式决定：

- 该 provider 是否支持原生 Responses
- 是否支持完整工具体系
- 是否支持 reasoning
- 是否支持 structured output
- 不支持时应如何拒绝、降级或转换

这导致系统会出现“看起来成功，实际语义丢失”的问题。

### 问题 F：`glm-5` 还未提升为一等正式支持模型

当前项目里 `glm-5` 主要体现在：

- 路由可配
- provider list 中出现
- 基础文本请求可通

但尚未做到：

- 默认配置层可用
- 文档化支持范围清晰
- 针对 Codex CLI 的回归测试稳定
- tool / reasoning / streaming 行为经过专项验证

### 问题 G：缺少清晰的 OpenAI Responses ↔ Chat 转换原则

当前项目虽然做了转换，但还没有形成一套明确、被文档化的转换规则，例如：

- 哪些字段可以无损映射
- 哪些字段只能部分映射
- 哪些字段必须拒绝
- 哪些事件可以稳定从 Chat 重建
- 哪些事件只有原生 Responses 才能保真

这会导致实现人员在不同模块里各自做判断，久而久之就会形成：

- handler 有一套假设
- transform 有一套假设
- provider 又有一套假设

最终让系统行为不一致。

### 问题 H：当前代码已经存在“目标协议雏形”，但没有被扶正为系统中心

从当前实现看，项目并不是完全缺少 Codex/Responses 所需模型，恰恰相反，已经存在不少“接近目标形态”的代码：

- `src/models/response.rs` 已经定义了较完整的 `ResponsesRequest`
- `src/models/response.rs` 已经包含 `previous_response_id`、`include`、`structured_output`、`reasoning`、`prompt_cache_key`
- `src/models/streaming.rs` 已经定义了大量 Responses/Codex 风格 SSE 事件
- `src/sse/responses.rs` 已经有对应的事件解析能力
- `src/providers/openai.rs` 已经具备原生 `/responses` 与 `/responses` 流式请求能力

真正的问题在于：

- 这些能力没有成为系统内部的“主执行语义”
- `handler` 仍然以 Chat fallback 为主要执行模型
- `transform` 仍在承担过多协议折叠责任
- `chat.rs` 这个更窄的模型仍然在主流程里拥有过高的话语权

这也是为什么“最佳改造方式”不是从零重写，而是重排现有重心。

---

## 三、目标架构

后续目标架构建议收敛为：

`Client Protocol (Responses / Chat)`
→ `Internal Canonical Model`
→ `Provider Capability Planning`
→ `Provider Adapter`
→ `Provider Response`
→ `Internal Canonical Events / Objects`
→ `Target Protocol Serialization`

### 设计原则

- `/v1/responses` 必须是一等链路
- `/v1/chat/completions` 继续保留，但不再主导内部抽象
- 内部建立 canonical protocol model，减少直接做 `Responses <-> Chat` 的脆弱双向硬转换
- 明确 provider capability matrix
- 对不能保真的能力显式报错
- tool / reasoning / streaming 必须优先于普通文本兼容逻辑

### 协议转换原则

后续实现时，必须明确遵守下面的协议转换原则：

1. `Responses -> Responses` 原生路径优先  
   如果 provider 原生支持 Responses，则不应先降级成 Chat。

2. `Responses -> Chat` 只能作为受控 fallback  
   只能在 capability planning 明确允许时使用。

3. 保真优先于表面兼容  
   宁可明确报“不支持”，也不要静默丢字段。

4. item 语义优先于文本拼接  
   能保留 `item` 的场景，不应只保留纯文本。

5. event 生命周期必须完整  
   对 Codex CLI 来说，`response.completed` 和 `response.output_item.done` 不只是日志事件，而是协议闭环的一部分。

### 最佳改造方式

基于当前整个仓库的真实实现，最佳改造方式不是：

- 直接推翻现有代码重写一套新代理
- 继续在现有 transform 上不断叠加补丁

而是下面这条更稳的路线：

1. 以现有 `src/models/response.rs` 为基础，扶正 `Responses` 作为系统主协议
2. 以现有 `src/models/streaming.rs` 和 `src/sse/responses.rs` 为基础，扶正 Responses SSE 事件模型
3. 把 `chat.rs` 明确降级为 provider-specific transport DTO
4. 在 handler 层引入 capability planning，决定：
   - 原生 Responses 直通
   - Responses via canonical transform
   - Chat fallback
   - Reject
5. 把 `transform` 从“协议中心”改造成“适配器中心”

这条路线的优势是：

- 最大化复用当前已存在的 Responses 模型和测试基础
- 最小化无意义重写
- 降低对 `/v1/chat/completions` 既有兼容行为的破坏
- 更容易渐进式落地与回归

它的本质是“重排中心、抽离 fallback”，而不是“全盘重构”。

---

## 四、文件与模块规划

在不做无意义大拆分的前提下，建议后续围绕以下文件组织工作。

### 核心修改文件

**Files:**
- Modify: `src/handlers/mod.rs`
- Modify: `src/providers/trait_.rs`
- Modify: `src/providers/openai.rs`
- Modify: `src/providers/zhipu.rs`
- Modify: `src/models/chat.rs`
- Modify: `src/models/response.rs`
- Modify: `src/models/streaming.rs`
- Modify: `src/transform/mod.rs`
- Modify: `src/config/app_config.rs`
- Modify: `src/server/router.rs`
- Test: `tests/integration_tests.rs`

### 建议新增文件

**Files:**
- Create: `src/protocol/mod.rs`
- Create: `src/protocol/canonical.rs`
- Create: `src/protocol/capabilities.rs`
- Create: `src/protocol/events.rs`
- Create: `tests/codex_cli_protocol_tests.rs`
- Create: `tests/glm5_e2e_tests.rs`

### 模块职责建议

- `src/protocol/canonical.rs`
  - 内部统一请求、输出 item、工具定义、事件定义

- `src/protocol/capabilities.rs`
  - provider capability matrix
  - fallback / reject / transform planning

- `src/protocol/events.rs`
  - Responses SSE 事件统一构建
  - collected / streamed 共用逻辑

### 当前代码与未来架构的映射关系

为了避免后续实现时重复造轮子，建议先明确现有代码中哪些模块应该被保留、哪些应该被降级为适配层：

- 保留并强化：
  - `src/models/response.rs`
  - `src/models/streaming.rs`
  - `src/sse/responses.rs`
  - `src/providers/openai.rs` 中原生 Responses 路径

- 保留但重组：
  - `src/handlers/mod.rs`
  - `src/transform/mod.rs`
  - `src/providers/zhipu.rs`

- 降级为 fallback DTO：
  - `src/models/chat.rs`

这个映射关系很关键，因为它决定了最佳改造方式不是把 `response.rs` 内容再复制一遍到新模块，而是把它提升为更接近 canonical 的入口基础。

---

## 五、实施任务

### Task 1: 建立协议能力矩阵与现状基线

**Files:**
- Modify: `plan4.md`
- Modify: `src/providers/trait_.rs`
- Modify: `src/providers/openai.rs`
- Modify: `src/providers/zhipu.rs`
- Test: `tests/integration_tests.rs`

- [ ] **Step 1: 盘点当前 provider 能力**

检查并记录：

- OpenAI provider 是否支持 native responses
- Zhipu provider 哪些 Responses 能力是 transform 来的
- 两个 provider 对 tool / reasoning / structured output / streaming 的支持边界
- Chat 与 Responses 两类 OpenAI 协议之间哪些字段可无损映射
- 哪些字段必须列入 reject / partial-support 名单

Run: `rtk rg -n "supports_responses_api|responses_streaming|reasoning|structured_output|tool|glm-5" src tests`
Expected: 能定位所有当前能力入口和缺口位置

- [ ] **Step 2: 在 `LLMProvider` 层设计 capability API**

为后续实现设计能力查询接口，至少覆盖：

- native responses support
- tool support by type
- reasoning support
- structured output support
- previous_response_id support
- store / cache support

预期改造方向：

```rust
pub struct ProviderCapabilities {
    pub native_responses: bool,
    pub supports_reasoning: bool,
    pub supports_structured_output: bool,
    pub supports_previous_response_id: bool,
    pub supported_tool_types: Vec<String>,
}
```

同时建议增加：

```rust
pub enum FallbackMode {
    NativeResponses,
    ResponsesViaCanonicalTransform,
    ChatFallback,
}
```

这样后续 handler 能直接依据能力规划主路径，而不是在逻辑里散落 `if supports_responses_api()`

- [ ] **Step 3: 为 OpenAI 和 Zhipu 明确 capability 返回**

实现 `capabilities()` 或等价机制，让 handler 能依据能力做路由与拒绝。

- [ ] **Step 4: 为现状补基础测试**

新增测试验证：

- provider capability 不为空
- OpenAI 与 Zhipu 返回的 capability 有差异
- 不会再隐式假设所有 provider 支持所有 Responses 能力

Run: `rtk cargo test --lib providers`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/providers/trait_.rs src/providers/openai.rs src/providers/zhipu.rs tests/integration_tests.rs plan4.md
git commit -m "refactor: introduce provider capability baseline"
```

### Task 2: 引入 internal canonical protocol model

**Files:**
- Create: `src/protocol/mod.rs`
- Create: `src/protocol/canonical.rs`
- Modify: `src/models/chat.rs`
- Modify: `src/models/response.rs`
- Modify: `src/transform/mod.rs`
- Test: `tests/codex_cli_protocol_tests.rs`

- [ ] **Step 1: 写 failing tests，明确 canonical model 目标**

先写测试覆盖以下场景：

- Responses message item 保真进入 canonical
- function tool 保真
- `web_search/file_search/computer_use/mcp` tool definition 保真
- `previous_response_id` 与 `reasoning` 进入 canonical
- OpenAI Responses item 结构不会被过早压缩成单字符串 message
- Chat fallback 只在 canonical planning 阶段之后发生

示例测试骨架：

```rust
#[test]
fn canonical_request_preserves_non_function_tools() {
    // build ResponsesRequest with file_search/computer_use/mcp
    // convert to canonical
    // assert fields preserved
}
```

- [ ] **Step 2: 定义 canonical request / item / tool / event 基础结构**

至少要能表示：

- system instructions
- multi-block input
- function/tool calls
- tool outputs
- reasoning config
- metadata
- previous_response_id
- structured output settings

注意：

- 不建议完全复制 `src/models/response.rs`
- 更好的做法是以 `response.rs` 为基础，抽出内部 canonical 关注点
- 让 external wire model 与 internal canonical model 在必要处共享结构，而不是重复定义两套近似对象

- [ ] **Step 3: 让 transform 先转到 canonical，再决定如何落到 Chat**

不要求一次性替换所有路径，但要开始减少直接 `Responses <-> Chat` 的硬编码转换。

- [ ] **Step 4: 把非 function 工具字段移出“会丢失”的路径**

至少保证这些字段在 internal canonical model 中不会消失：

- `vector_store_ids`
- `display_width`
- `display_height`
- `environment`
- `server_label`
- `server_url`
- `require_approval`

- [ ] **Step 5: 运行测试**

Run: `rtk cargo test canonical_request_preserves_non_function_tools -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/protocol src/models src/transform tests/codex_cli_protocol_tests.rs
git commit -m "refactor: add canonical protocol model"
```

### Task 3: 扩展工具系统，补齐 Codex CLI 工具定义支持

**Files:**
- Modify: `src/models/chat.rs`
- Modify: `src/models/response.rs`
- Modify: `src/transform/mod.rs`
- Modify: `src/providers/zhipu.rs`
- Test: `tests/codex_cli_protocol_tests.rs`

- [ ] **Step 1: 写 failing tests，锁定工具定义保真**

覆盖：

- `web_search`
- `file_search`
- `computer_use`
- `mcp`
- `function`

并验证经过 `Responses -> canonical -> provider request planning` 后字段不丢。

- [ ] **Step 2: 扩展 chat/internal tool model**

不要再只依赖：

```rust
pub struct Tool {
    pub tool_type: String,
    pub function: Option<FunctionDefinition>,
}
```

要么扩展字段，要么明确 chat model 仅作为 provider-specific DTO，不再承担 internal canonical role。

- [ ] **Step 3: 为 Zhipu 加入工具映射策略**

明确：

- 哪些工具能直接映射到 Zhipu OpenAI-compatible request
- 哪些工具只能降级为 function-like representation
- 哪些工具必须拒绝
- 哪些工具只在 OpenAI native responses path 中正式支持

- [ ] **Step 4: 对不能保真的工具显式报错**

例如 `computer_use` 若当前 provider 不支持，应返回明确错误而不是静默消失。

- [ ] **Step 5: 运行工具相关测试**

Run: `rtk cargo test tool -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/models/chat.rs src/models/response.rs src/transform/mod.rs src/providers/zhipu.rs tests/codex_cli_protocol_tests.rs
git commit -m "feat: preserve codex tool definitions through proxy pipeline"
```

### Task 4: 把 `/v1/responses` 升级为 Responses-first execution path

**Files:**
- Modify: `src/handlers/mod.rs`
- Modify: `src/providers/trait_.rs`
- Modify: `src/transform/mod.rs`
- Create: `src/protocol/capabilities.rs`
- Test: `tests/integration_tests.rs`

- [ ] **Step 1: 写 failing integration tests**

测试以下行为：

- provider 原生支持 Responses 时不走 Chat transform
- provider 不支持某些 Responses 能力时显式返回错误
- provider 支持部分能力时按 capability plan 走 fallback
- OpenAI native responses path 与 Chat fallback path 行为可区分
- `previous_response_id` 不会在 fallback 中悄悄消失

- [ ] **Step 2: 设计 `ResponsesExecutionPlan`**

示例：

```rust
pub enum ResponsesExecutionPlan {
    NativeResponses,
    TransformToChat { unsupported_fields: Vec<String> },
    Reject { reason: String },
}
```

更推荐扩展为：

```rust
pub enum ResponsesExecutionPlan {
    NativeResponses,
    ResponsesViaCanonicalTransform,
    ChatFallback { lossy_fields: Vec<String> },
    Reject { reason: String },
}
```

这样能把“基于 canonical 的 Responses-first 流程”和“真正 Chat fallback”区分开，避免未来又重新混到一起。

- [ ] **Step 3: 在 handler 中先做 capability planning，再执行**

顺序调整为：

1. parse request
2. build canonical request
3. inspect provider capabilities
4. build execution plan
5. execute
6. serialize response

- [ ] **Step 4: 补错误语义**

对于以下情况返回明确错误：

- `previous_response_id` 需要保真但 provider 不支持
- tool type 不支持
- reasoning requested but unsupported
- structured output requested but cannot be preserved

- [ ] **Step 5: 运行集成测试**

Run: `rtk cargo test --test integration_tests`
Expected: PASS 或仅剩与本任务无关的历史失败

- [ ] **Step 6: Commit**

```bash
git add src/handlers/mod.rs src/providers/trait_.rs src/transform/mod.rs src/protocol/capabilities.rs tests/integration_tests.rs
git commit -m "refactor: add responses-first execution planning"
```

### Task 5: 统一 Responses SSE 事件生成逻辑

**Files:**
- Create: `src/protocol/events.rs`
- Modify: `src/handlers/mod.rs`
- Modify: `src/models/streaming.rs`
- Test: `tests/codex_cli_protocol_tests.rs`

- [ ] **Step 1: 写 failing tests 覆盖事件序列**

测试：

- `response.created`
- `response.in_progress`
- `response.output_item.added`
- `response.output_text.delta`
- `response.function_call_arguments.delta`
- `response.function_call_arguments.done`
- `response.output_item.done`
- `response.completed`
- `[DONE]`

同时增加一类协议一致性测试：

- OpenAI native responses stream 原样透传关键事件
- Chat fallback stream 至少保证客户端可消费的最小闭环
- 不允许出现“流关闭但没有 response.completed”

顺序必须稳定且可预期。

- [ ] **Step 2: 从 `handlers/mod.rs` 提取统一 event builder**

把 collected path 和 streamed path 的事件生成逻辑收敛到一个模块，不再双写。

这里的最佳做法不是简单抽函数，而是：

- 让 `src/protocol/events.rs` 接收 canonical output delta / provider event
- 再生成 Responses SSE 事件

不要继续让 `handlers/mod.rs` 直接拼 JSON 字符串，否则后续 reasoning / tool / metadata 事件还会继续散在 handler 里。

- [ ] **Step 3: 补 streamed tool call 增量事件**

确保 streamed path 下也能输出：

- function call arguments delta
- function call done
- output item done

- [ ] **Step 4: 修复 response snapshot 中的硬编码占位**

当前 `response_snapshot_json()` 里许多字段是固定空值，后续要逐步改成来自 canonical/request context 的真实值。

- [ ] **Step 5: 运行 streaming tests**

Run: `rtk cargo test stream -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/protocol/events.rs src/handlers/mod.rs src/models/streaming.rs tests/codex_cli_protocol_tests.rs
git commit -m "feat: unify responses streaming event generation"
```

### Task 6: 补齐 `previous_response_id` / `reasoning` / `structured_output` 语义

**Files:**
- Modify: `src/models/response.rs`
- Modify: `src/transform/mod.rs`
- Modify: `src/handlers/mod.rs`
- Modify: `src/providers/openai.rs`
- Modify: `src/providers/zhipu.rs`
- Test: `tests/codex_cli_protocol_tests.rs`

- [ ] **Step 1: 写 failing tests**

覆盖：

- `previous_response_id` 被接收并进入 internal flow
- `reasoning` 字段不会静默消失
- `structured_output` 至少能做 capability validation

- [ ] **Step 2: 对 OpenAI path 保真透传**

对原生 Responses provider，尽可能直接透传这些字段。

- [ ] **Step 2.1: 明确“当前代码已有但未扶正”的实现**

检查并重用：

- `src/models/response.rs` 中已有字段
- `src/providers/openai.rs` 中已有 responses 请求/流式路径
- `src/sse/responses.rs` 中已有 reasoning 事件解析模型

这一步的目的不是新增类型，而是避免重复建设。

- [ ] **Step 3: 对 Zhipu path 定义明确策略**

分成 3 类：

- preserve
- transform
- reject

不要再采用 silent drop。

- [ ] **Step 3.1: 为 OpenAI 官方语义建立注释与测试依据**

明确写在代码和测试注释中：

- `previous_response_id` 是 Responses 原生语义
- `instructions` 与 `previous_response_id` 同用时不应简单复用旧 instructions
- Chat fallback 无法天然等价实现该语义时必须显式处理

- [ ] **Step 4: 统一错误语义**

例如：

```json
{
  "error": {
    "message": "Provider zhipu does not preserve previous_response_id in chat fallback mode",
    "type": "unsupported_feature"
  }
}
```

- [ ] **Step 5: 运行 targeted tests**

Run: `rtk cargo test previous_response_id -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/models/response.rs src/transform/mod.rs src/handlers/mod.rs src/providers/openai.rs src/providers/zhipu.rs tests/codex_cli_protocol_tests.rs
git commit -m "feat: preserve or explicitly reject advanced responses semantics"
```

### Task 7: 把 `glm-5` 提升为正式支持目标模型

**Files:**
- Modify: `src/config/app_config.rs`
- Modify: `src/providers/zhipu.rs`
- Modify: `README.md`
- Create: `tests/glm5_e2e_tests.rs`
- Test: `tests/glm5_e2e_tests.rs`

- [ ] **Step 1: 写 failing glm-5 smoke tests**

至少覆盖：

- `glm-5` 非流式文本请求
- `glm-5` 流式文本请求
- `glm-5` function calling 基础请求
- `glm-5` 经 `/v1/responses` 的最小可用路径

- [ ] **Step 2: 调整默认配置建议**

视产品策略决定：

- 默认模型是否切到 `glm-5`
- README 是否把 `glm-5` 作为主推荐模型
- 是否增加 `routing.model_mapping` 示例

- [ ] **Step 3: 在 provider 层补 `glm-5` 兼容逻辑**

例如：

- 工具预处理
- endpoint 选择校验
- 特定字段映射
- 明确日志提示

- [ ] **Step 4: 文档化 `glm-5` 支持边界**

明确列出：

- 已支持
- 部分支持
- 不支持

- [ ] **Step 5: 运行 glm-5 tests**

Run: `rtk cargo test glm5 -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/config/app_config.rs src/providers/zhipu.rs README.md tests/glm5_e2e_tests.rs
git commit -m "feat: formalize glm-5 support for codex proxy workflows"
```

### Task 8: 建立 Codex CLI e2e 回归测试与验收基线

**Files:**
- Create: `tests/codex_cli_protocol_tests.rs`
- Create: `tests/glm5_e2e_tests.rs`
- Modify: `tests/integration_tests.rs`

- [ ] **Step 1: 设计 Codex CLI 核心场景回归集**

至少包含：

- 最小 Responses 文本对话
- function call 往返
- tool output 续接
- streamed text
- streamed function call arguments
- response.completed + [DONE]
- `previous_response_id`
- OpenAI Responses native path
- OpenAI Chat fallback path
- Zhipu `glm-5` path

- [ ] **Step 2: 引入 mock upstream fixtures**

通过 mock server 模拟：

- OpenAI native responses upstream
- Zhipu chat upstream
- mixed chunk / tool delta / usage final chunk

- [ ] **Step 3: 建立验收命令**

推荐至少有：

```bash
rtk cargo test
rtk cargo test --test codex_cli_protocol_tests -- --nocapture
rtk cargo test --test glm5_e2e_tests -- --nocapture
```

- [ ] **Step 4: 记录当前已知历史失败并隔离**

例如现有日志文件截断相关历史失败，需要单独标记，避免影响新能力的验收判断。

- [ ] **Step 5: Commit**

```bash
git add tests/integration_tests.rs tests/codex_cli_protocol_tests.rs tests/glm5_e2e_tests.rs
git commit -m "test: add codex cli proxy regression coverage"
```

---

## 六、执行顺序建议

建议严格按下面顺序推进，不要跳：

1. Task 1 能力矩阵
2. Task 2 canonical model
3. Task 3 工具系统
4. Task 4 Responses-first execution planning
5. Task 5 统一 streaming events
6. Task 6 高级 Responses 语义
7. Task 7 `glm-5` 正式支持
8. Task 8 e2e 回归与验收

原因很简单：

- 不先做 capability baseline，就无法知道哪些行为应 preserve，哪些应 reject
- 不先做 canonical model，就会继续陷入 transform 补丁地狱
- 不先统一 execution path，就无法稳定定义 streaming 与高级字段行为

另外还有一个现实原因：

- 当前代码已经在 `response.rs` / `streaming.rs` / `openai.rs` 中积累了不少接近目标能力的实现
- 如果不先把这些“已存在的正确方向”整合成主路径，后面任何新增功能都会继续叠加在旧结构之上，导致债务越来越重

---

## 七、风险与注意事项

### 风险 1：一次性重构过大

应对方式：

- 每个任务单独提交
- 每个任务都以测试为边界
- 先引入新结构，再逐步迁移旧逻辑

### 风险 1.1：重复造 canonical / responses 模型

应对方式：

- 优先审查并复用 `src/models/response.rs`
- 对必须抽离的 internal 结构只抽“额外语义”，不要复制现有字段全集
- 把“复用现有类型”作为 code review 检查项

### 风险 2：Chat 路径被破坏

应对方式：

- 保留现有 `/v1/chat/completions` 回归测试
- 对 chat handler 补最小 smoke tests
- 不在早期任务中同时重写 chat 和 responses 两条链路

### 风险 3：Zhipu 与 OpenAI 行为差异扩大

应对方式：

- 把差异前置到 capability planning
- 不试图在表面 API 上假装两者完全一致
- 明确 provider-specific error semantics

### 风险 5：错误把 Codex CLI 理解成“只需要 /responses 能返回文本”

应对方式：

- 把外部协议调研结论固化到测试和文档里
- 对 `response.output_item.done`、`response.completed`、`previous_response_id` 建立专项回归
- 明确把“可消费的事件闭环”当作验收标准，而不是只看 HTTP 200

### 风险 4：继续 silent drop

应对方式：

- 任何高级字段或工具若无法保真，都必须：
  - preserve
  - transform with explicit note
  - reject

三选一，禁止静默丢失。

---

## 十、rcodex vs mimo2codex 功能对比分析 (v2.8.x)

> **更新时间:** 2026-05-28
> **目标:** 识别当前代码中的 mock 部分，制定完善计划

### 10.1 前端功能对比

| 功能模块 | rcodex (rcodex-admin) | mimo2codex | 状态 | 说明 |
|---------|---------------------|------------|------|------|
| **Providers页面** | ✅ 已实现 | ✅ 已实现 | 完成 | 支持测试连接、可展开行显示模型 |
| **Codex页面** | ⚠️ 部分实现 | ✅ 完整 | 待完成 | Override按钮功能未实现 |
| **Dashboard页面** | ✅ 已实现 | ✅ 已实现 | 完成 | 统计信息、请求趋势 |
| **Logs页面** | ✅ 已实现 | ✅ 已实现 | 完成 | 日志查询、详情 |
| **Models页面** | ✅ 已实现 | ✅ 已实现 | 完成 | 模型管理 |
| **Account页面** | ✅ 已实现 | ✅ 已实现 | 完成 | API keys、OAuth |
| **Thinking控制** | ✅ 已实现 | ✅ 已实现 | 完成 | 运行时控制 |
| **历史记录** | ⚠️ 返回空数组 | ✅ 完整 | 待完成 | `get_codex_history_handler` 返回空 |
| **Active Override** | ⚠️ 未持久化 | ✅ 完整 | 待完成 | 仅返回null，无实际存储 |
| **Provider Health** | ✅ 已实现 | ✅ 已实现 | 完成 | 健康检查 |
| **导入/导出** | ✅ 已实现 | ✅ 已实现 | 完成 | 配置迁移 |

### 10.2 后端API对比

| API端点 | rcodex | mimo2codex | 状态 | 问题 |
|---------|--------|-----------|------|------|
| `GET /admin/api/stats` | ✅ 返回1个provider | ✅ 动态count | 待修复 | total_providers硬编码为1 |
| `GET /admin/api/codex-state` | ✅ 已实现 | ✅ 已实现 | 完成 | |
| `GET /admin/api/codex-targets` | ✅ 已实现 | ✅ 已实现 | 完成 | |
| `POST /admin/api/probe` | ✅ 真实请求 | ✅ 真实请求 | 完成 | MiniMax返回真实错误 |
| `POST /admin/api/codex-apply` | ✅ 已实现 | ✅ 已实现 | 完成 | |
| `POST /admin/api/codex-restore` | ✅ 已实现 | ✅ 已实现 | 完成 | |
| `GET /admin/api/active-override` | ⚠️ 返回null | ✅ 完整 | 待完成 | 未读取实际override |
| `PUT /admin/api/active-override` | ⚠️ 仅返回 | ✅ 持久化 | 待完成 | 未保存到存储 |
| `DELETE /admin/api/active-override` | ⚠️ 仅返回 | ✅ 清除 | 待完成 | 未清除存储 |
| `GET /admin/api/codex-history` | ⚠️ 返回空数组 | ✅ 完整 | 待完成 | 需要实现DB查询 |
| `GET /admin/api/codex-history/:id` | ⚠️ 返回错误 | ✅ 完整 | 待完成 | 需要实现DB查询 |
| `GET /admin/api/thinking` | ✅ 已实现 | ✅ 已实现 | 完成 | |
| `PUT /admin/api/thinking` | ✅ 已实现 | ✅ 已实现 | 完成 | |

### 10.3 Rust后端Mock/待实现功能清单

#### 高优先级 (必须实现)

1. **`get_active_override_handler`** (line 118-120)
   - 当前: 直接返回 `{ override: null }`
   - 需要: 从DB读取当前active override
   - 影响: Override面板无法显示当前状态

2. **`put_active_override_handler`** (line 129-137)
   - 当前: 仅返回成功响应，不保存
   - 需要: 保存override到存储
   - 影响: Runtime override无法持久化

3. **`delete_active_override_handler`** (line 140-147)
   - 当前: 仅返回成功响应，不删除
   - 需要: 从存储删除override
   - 影响: 无法清除runtime override

4. **`get_codex_history_handler`** (line 154-157)
   - 当前: 返回空数组 `Vec::new()`
   - 需要: 从DB查询历史记录
   - 影响: History页面显示为空

5. **`get_codex_history_by_id_handler`** (line 160-164)
   - 当前: 返回错误信息
   - 需要: 从DB查询单条记录
   - 影响: 历史详情无法查看

6. **`total_providers` in `api_stats`** (line 137)
   - 当前: 返回硬编码值1
   - 需要: 动态计算所有provider数量
   - 影响: Dashboard统计数据不准确

#### 中优先级 (应该实现)

7. **前端Override按钮** (CodexPage.tsx line 385-392)
   - 当前: `// TODO: Implement override` + console.log
   - 需要: 调用 `api.codex.setOverride()`
   - 影响: 用户无法设置runtime override

8. **CodexStateCard中的Override状态** (line 181-183)
   - 当前: 硬编码显示 `inactive`
   - 需要: 显示实际override状态
   - 影响: 用户看不到override是否激活

### 10.4 测试用例设计

#### TC1: Providers页面测试模型连接

```typescript
// 目标: 验证Providers页面的模型探测功能
describe('ProvidersPage - Model Probe', () => {
  it('should test MiniMax connection', async () => {
    // 1. 访问 /providers 页面
    // 2. 点击MiniMax模型的测试按钮
    // 3. 验证返回真实的API响应
    // 4. 确认错误信息正确显示（如"余额不足"）
  });

  it('should show all configured providers', async () => {
    // 1. 验证OpenAI、Zhipu、MiniMax都显示
    // 2. 验证每个provider的hasKey状态正确
  });

  it('should expand to show models', async () => {
    // 1. 点击展开按钮
    // 2. 验证模型列表显示
    // 3. 验证context_window信息显示
  });
});
```

#### TC2: Codex Override功能测试

```typescript
// 目标: 验证runtime override功能
describe('CodexPage - Runtime Override', () => {
  it('should set override via button', async () => {
    // 1. 在ProviderSelector中点击Override按钮
    // 2. 验证PUT /admin/api/active-override被调用
    // 3. 验证Override面板显示正确状态
    // 4. 验证CodexStateCard显示override active
  });

  it('should display current override state', async () => {
    // 1. 先设置一个override
    // 2. 刷新页面
    // 3. 验证override状态被正确读取和显示
  });

  it('should clear override', async () => {
    // 1. 已有active override
    // 2. 点击Clear Override按钮
    // 3. 验证DELETE /admin/api/active-override被调用
    // 4. 验证override状态被清除
  });
});
```

#### TC3: Codex History测试

```typescript
// 目标: 验证历史记录功能
describe('CodexPage - History', () => {
  it('should display history entries', async () => {
    // 1. 执行apply操作创建历史
    // 2. 访问History标签页
    // 3. 验证历史记录正确显示
    // 4. 验证时间戳格式正确
  });

  it('should allow restore from history', async () => {
    // 1. 点击历史记录的restore按钮
    // 2. 验证POST /admin/api/codex-restore被调用
    // 3. 验证状态更新成功
  });
});
```

#### TC4: Backend API测试

```typescript
// 目标: 验证后端API正确性
describe('Backend API', () => {
  it('should return correct provider count', async () => {
    const stats = await api.stats.get('24h');
    expect(stats.total_providers).toBeGreaterThan(1);
  });

  it('should persist override', async () => {
    await api.codex.setOverride('minimax', 'MiniMax-M2');
    const override = await api.codex.activeOverride();
    expect(override.data?.providerId).toBe('minimax');
  });

  it('should query history from DB', async () => {
    const history = await api.codex.history();
    expect(Array.isArray(history.data)).toBe(true);
  });
});
```

### 10.5 实施计划

#### Phase 1: 后端修复 (高优先级)

- [ ] **Step 1.1**: 实现 `get_active_override_handler` 从DB读取
- [ ] **Step 1.2**: 实现 `put_active_override_handler` 保存到DB
- [ ] **Step 1.3**: 实现 `delete_active_override_handler` 从DB删除
- [ ] **Step 1.4**: 实现 `get_codex_history_handler` 从DB查询
- [ ] **Step 1.5**: 实现 `get_codex_history_by_id_handler` 从DB查询
- [ ] **Step 1.6**: 修复 `api_stats` 动态计算provider数量

#### Phase 2: 前端完善

- [ ] **Step 2.1**: 实现Override按钮调用API
- [ ] **Step 2.2**: CodexStateCard显示真实override状态
- [ ] **Step 2.3**: History页面正确显示历史记录

#### Phase 3: 验证测试

- [ ] **Step 3.1**: 运行前端测试用例 TC1-TC4
- [ ] **Step 3.2**: 使用curl测试后端API
- [ ] **Step 3.3**: 端到端验证整个流程

---

## 十一、验收标准

完成本计划后，系统至少应达到以下标准：

### 协议层

- `/v1/responses` 成为稳定一等入口
- `/v1/chat/completions` 继续可用
- 能明确区分 preserve / transform / reject
- OpenAI 官方 Responses 关键语义在原生 path 下保真
- 当前已经存在于 `response.rs` / `streaming.rs` / `openai.rs` 的 Responses 能力被真正纳入主路径，而不是停留在边缘实现

### 工具层

- function tool 完整往返
- 非 function tools 不再静默丢失
- streamed tool events 具备一致性

### `glm-5` 层

- `glm-5` 文本请求稳定
- `glm-5` 基础 function calling 稳定
- `glm-5` 经 `/v1/responses` 的最小 Codex CLI 工作流可通过

### 测试层

- provider tests 通过
- integration tests 通过或历史失败被单独隔离
- codex protocol regression tests 通过
- glm-5 e2e tests 通过
- 至少有一组测试明确验证 OpenAI Responses 与 Chat 的协议差异不会被错误抹平

---

## 九、计划完成后的执行选择

`plan4.md` 的定位是“可执行路线图”。完成该计划文档后，下一步建议有两种执行方式：

1. Subagent-Driven（推荐）  
   每个 Task 单独派发、逐任务 review、逐任务落地，风险更可控

2. Inline Execution  
   在当前会话中按 Task 分批执行，适合连续推进但更依赖上下文稳定

从当前项目复杂度看，更推荐第一种，因为它涉及：

- 架构调整
- 模型调整
- 协议调整
- 测试新增

这些变更彼此有关联，但也足够复杂，逐 Task 推进更稳。
