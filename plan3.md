# rcodex 后续规划：构建面向 Codex CLI 的 Responses/OpenAI 协议代理

> 更新时间：2026-04-17  
> 文档性质：现状分析 + 问题归纳 + 后续规划  
> 目标：基于当前 `rcodex` 代码真实状态，制定把项目演进为“可稳定服务 Codex CLI 的代理”的后续路线图，而不是停留在简单的 Chat Completions 兼容层。

---

## 一、文档目标

本文不是实现细节清单，也不是单次修复说明，而是一份面向后续迭代的规划文档。它要回答 5 个问题：

1. 当前 `rcodex` 实际是什么
2. 当前代码已经实现了什么
3. 当前代码还缺什么，尤其是对 `Codex CLI + glm-5` 的支持缺什么
4. 这些问题的根因是什么
5. 后续应该按什么顺序推进，才能把项目从“基础兼容代理”演进为“可靠的 Codex CLI 协议代理”

---

## 二、项目当前真实定位

### 2.1 当前项目不是原生 Codex CLI Proxy

从当前代码结构看，`rcodex` 目前是一个：

- 对外暴露 OpenAI 风格接口
- 内部支持多个 provider
- 以 `Chat Completions` 为基础抽象
- 对 `Responses API` 做协议转换和事件包装

它已经具备“把部分 Responses 请求转换成 Chat 请求，再转回 Responses 格式”的能力，但还不是原生意义上的 “Responses-first proxy”。

### 2.2 当前更像两层代理的混合体

仓库现有实现同时承担了两类职责：

1. 普通 LLM 网关  
   根据 `model -> provider` 路由，把请求转发到 OpenAI 或 Zhipu

2. 协议转换层  
   在 `Chat Completions API` 与 `Responses API` 之间双向转换

这两个职责现在是耦合在一起的，而且底层 provider 抽象是以 Chat 为中心设计的，这会让 Codex CLI 相关能力天然处于“降级映射”状态。

---

## 三、当前代码结构分析

### 3.1 核心模块

当前项目的关键代码可以分成 5 层：

1. 配置层  
   `src/config/app_config.rs`

2. HTTP 路由和 handler 层  
   `src/server/router.rs`  
   `src/handlers/mod.rs`

3. provider 抽象层  
   `src/providers/trait_.rs`  
   `src/providers/openai.rs`  
   `src/providers/zhipu.rs`

4. 数据模型层  
   `src/models/chat.rs`  
   `src/models/response.rs`  
   `src/models/streaming.rs`

5. 转换层  
   `src/transform/mod.rs`

### 3.2 当前请求执行链路

#### Chat 路径

`/v1/chat/completions`

执行逻辑：

1. 根据模型名选择 provider
2. 必要时把非 `glm-*` 模型重写成 Zhipu 默认模型
3. 调用 provider 的 `chat()` 或 `chat_streaming()`
4. 对部分返回值再做一次 Chat/Responses 往返转换

#### Responses 路径

`/v1/responses`

执行逻辑：

1. 接收 `ResponsesRequest`
2. 选择 provider
3. 如果 provider 原生支持 Responses，则直接走 provider responses path
4. 如果 provider 不原生支持，则先 `Responses -> Chat`
5. 再调用 provider 的 chat 接口
6. 最后把结果包装回 Responses 或 Responses SSE 事件

### 3.3 当前 provider 架构的真实特点

`LLMProvider` trait 虽然已经声明了：

- `chat`
- `chat_streaming`
- `responses`
- `responses_streaming`

但从项目整体设计上看，真正成熟稳定的是 Chat 路径。  
对于 Zhipu，Responses 支持本质上仍然依赖 Chat 转换，而不是完整的原生 Responses 协议承接。

---

## 四、当前已经实现的能力

### 4.1 已经具备的基础能力

当前项目已经完成了以下基础能力：

- OpenAI provider 接入
- Zhipu provider 接入
- 基础路由能力
- OpenAI 风格 `/v1/chat/completions`
- OpenAI 风格 `/v1/responses`
- SSE 基础流式响应
- Chat 与 Responses 的基础对象转换
- 基础 function calling 数据结构
- 基础日志、限流、代理配置

### 4.2 对 `glm-5` 已有的实际支持

基于代码现状，可以认为当前已经“部分支持” `glm-5`：

- 可以通过 Zhipu provider 调用 `glm-5`
- 可以在非复杂场景下完成普通文本生成
- 可以返回基础 Chat 响应
- 可以把部分 Responses 请求压缩成 Chat 请求后发给 `glm-5`
- 可以做基础流式文本输出

### 4.3 目前“看起来支持、实际并不完整”的能力

以下能力在模型层或类型层已经出现，但工程上还没有形成完整可依赖支持：

- `reasoning`
- `structured_output`
- `previous_response_id`
- `store`
- `metadata`
- `prompt_cache_key`
- `web_search`
- `file_search`
- `computer_use`
- `mcp`
- 流式 tool call 事件

这类能力当前主要停留在：

- 数据模型声明了
- 局部转换写了
- 某些测试覆盖了静态结构

但未形成端到端、保真、可稳定消费的完整链路。

---

## 五、当前存在的核心问题

下面的问题不是零散 bug，而是影响项目后续演进方向的结构性问题。

### 5.1 根问题一：项目目前仍是 Chat-first，不是 Responses-first

这会带来两个直接后果：

1. 所有 Responses 独有语义都只能“映射”到 Chat 的表达能力里
2. Codex CLI 依赖的协议能力会被迫降级、丢失或伪造

这也是为什么当前项目虽然已经有 `/v1/responses` 路由，但还不能等价看成“Codex CLI 协议代理”。

### 5.2 根问题二：工具系统模型过窄

`src/models/chat.rs` 中的 `Tool` 结构只有：

- `type`
- `function`

但 Codex CLI / Responses API 里的工具体系并不只有 function：

- `web_search`
- `file_search`
- `computer_use`
- `mcp`

这些工具往往还需要特定字段，例如：

- `vector_store_ids`
- `display_width`
- `display_height`
- `environment`
- `server_label`
- `server_url`
- `require_approval`

当前内部模型无法承载这些字段，所以一旦 `ResponsesRequest.tools` 经过转换，很多工具定义会在中途丢失。

### 5.3 根问题三：Responses 语义在 transform 层保真度不够

`src/transform/mod.rs` 是当前最关键、也是最脆弱的模块之一。

存在的问题包括：

- `Responses -> Chat` 时大量字段被忽略
- `Chat -> Responses` 时只恢复出较浅层的 message 语义
- 多内容块被压缩成字符串
- 非文本内容被弱化
- tool 定义和 tool 输出并未完全双向保真
- reasoning / store / previous_response_id / metadata 等字段没有被完整承接

这意味着当前 transform 更像“兼容性折中层”，不是“协议等价转换层”。

### 5.4 根问题四：流式链路对文本友好，对工具和复杂事件不友好

当前 Responses SSE 事件生成分为两种思路：

1. collected chunks 批量生成事件
2. streamed chunks 逐步生成事件

但对 Zhipu 真实流式路径来说，Codex 更在意的不是单纯文本 delta，而是：

- output item 生命周期
- function call 参数增量
- function call 完成事件
- reasoning 相关事件
- completion / done 的时序语义

现在这些能力只覆盖了一部分，且不同路径行为不一致。

### 5.5 根问题五：支持能力缺少“显式失败”

当前代码对不少不支持的能力采用“静默丢弃”或“弱降级”策略：

- 接了字段，但没发给上游
- 接了工具，但没真正保留
- 接了 Responses 语义，但压平后再也恢复不了

这会导致最糟糕的结果：

不是请求直接报错，而是“看起来成功了，但行为不对”。

对于 Codex CLI 这种 agent 场景，这类问题比显式失败更危险，因为它会造成：

- 工具无法调用但用户无感知
- 上下文丢失但请求仍返回 200
- reasoning 没生效但代理仍伪造正常输出

---

## 六、针对 Codex CLI 的真实差距

### 6.1 当前项目与 Codex CLI 的契合点

当前项目已经具备以下对接基础：

- 提供 `/v1/responses`
- 定义了较丰富的 Responses 模型
- 已经开始做 Responses SSE 事件
- 已有部分 Codex 风格 item 类型建模
- 已经考虑到 tool calls 和流式事件

这说明项目方向并没有错，当前问题主要是：

能力保真不足，协议承接不完整，缺少一层“把 Responses 当成一等协议”的架构升级。

### 6.2 当前还不够支撑 Codex CLI 的能力

对 Codex CLI 来说，当前项目还缺以下关键能力：

1. 真正可靠的 Responses-first 请求处理
2. `previous_response_id` 语义保留
3. `store` / `metadata` / `include` / `prompt_cache_key` 等字段透传策略
4. 完整的工具定义透传
5. 完整的工具输出 item 透传
6. streamed tool call 增量事件
7. reasoning 相关字段和事件支持
8. 多内容块和富输入格式的保真支持
9. provider 能力协商与能力降级策略
10. 对“不支持能力”的显式拒绝和错误信息

### 6.3 对 `glm-5` 的特殊差距

如果目标是“让 Codex CLI 能稳定使用 `glm-5`”，当前项目至少还缺：

- `glm-5` 默认路由和默认模型设置
- `glm-5` 在 Zhipu OpenAI 兼容接口下的工具能力适配
- `glm-5` 的 function calling 流式兼容增强
- `glm-5` 结构化输出兼容策略
- `glm-5` thinking/reasoning 支持方案
- `glm-5` 与 Codex Responses 语义之间的规范映射

---

## 七、后续规划总目标

### 7.1 目标定义

后续 `rcodex` 的目标不应只是“支持更多模型”，而应升级为：

一个对外提供 OpenAI 风格 API、对内可路由到多 provider、并且能稳定服务 Codex CLI 的协议代理。

### 7.2 目标能力边界

规划完成后的目标系统，至少应满足以下要求：

- 对外稳定提供 `/v1/responses`
- 对外稳定提供 `/v1/chat/completions`
- 明确区分 Responses-first 与 Chat-first 两类链路
- 能稳定代理 Codex CLI 常见工作流
- 对 `glm-5` 提供明确、受测、文档化的支持范围
- 在不支持的能力上明确报错，而不是静默降级

---

## 八、分阶段演进规划

后续建议分 5 个阶段推进，而不是继续在现有转换层上零碎打补丁。

### 阶段一：协议收敛与能力边界梳理

目标：

- 明确 `rcodex` 支持哪些 OpenAI/Codex Responses 能力
- 明确哪些能力通过原生 provider 支持
- 明确哪些能力只能降级
- 明确哪些能力直接不支持

这一阶段需要完成：

1. 梳理当前 `ResponsesRequest` 全字段支持矩阵
2. 梳理 `ChatRequest` 可承载字段矩阵
3. 梳理 OpenAI provider 与 Zhipu provider 的能力差异
4. 给每个字段和工具定义“保真 / 降级 / 不支持”标签
5. 建立文档化能力矩阵

输出物：

- 协议能力矩阵
- provider 能力矩阵
- `glm-5` 支持边界文档

### 阶段二：内部模型升级

目标：

- 把内部数据模型从“只够用”升级成“足以承载 Codex 协议”

这一阶段需要完成：

1. 扩展 chat/internal tool 模型
2. 为 `web_search`、`file_search`、`computer_use`、`mcp` 增加专用字段
3. 为多内容块输入输出建立可保真的内部表示
4. 区分“外部 API 模型”和“内部 canonical 模型”
5. 减少在 transform 层里临时拼装 JSON 的做法

输出物：

- 新的内部 canonical protocol model
- 清晰的 tool schema
- 更稳定的 message/content 表示

### 阶段三：Responses-first 执行链路重构

目标：

- 让 `/v1/responses` 成为一等链路，而不是 Chat 降级链路

这一阶段需要完成：

1. 重新定义 provider capability abstraction
2. 区分 provider 的三类能力：
   - 原生 Responses
   - 原生 Chat
   - Responses-via-transform
3. 在 handler 层引入能力协商
4. 建立 `ResponsesExecutionPlan`
5. 对不支持能力做显式拒绝

输出物：

- Responses-first execution path
- 明确的 fallback policy
- 错误信息和降级说明机制

### 阶段四：流式事件与工具链路补完

目标：

- 让 Codex CLI 能正确消费事件流，而不只是收到文本

这一阶段需要完成：

1. 统一 collected 和 streamed 两套事件生成逻辑
2. 补全 tool call 增量事件
3. 补全 tool call done 事件
4. 规范 output_item.added / output_item.done 生命周期
5. 加入 reasoning 相关事件支持
6. 确保 `[DONE]`、`response.completed`、usage 的时序一致

输出物：

- 统一的 Responses SSE event builder
- 完整的 tool streaming support
- 更接近 Codex CLI 期望的事件序列

### 阶段五：面向 `glm-5` 的专门支持与验收

目标：

- 把 `glm-5` 从“可以试”提升为“正式支持”

这一阶段需要完成：

1. 默认配置支持 `glm-5`
2. 文档中明确 `glm-5` 推荐配置
3. 为 `glm-5` 建立端到端 smoke tests
4. 为 `glm-5 + Codex CLI` 建立回归测试
5. 明确 `glm-5` 的能力限制与推荐场景

输出物：

- `glm-5` 支持说明
- `glm-5` e2e regression tests
- 可复现的验收脚本

---

## 九、需要优先解决的问题清单

按优先级看，后续最重要的不是“多加几个字段”，而是先解决下面这些问题。

### P0

- 把 `/v1/responses` 从 Chat-first 改造成 Responses-first
- 建立 canonical internal protocol model
- 明确 provider capability negotiation
- 修复工具定义在 transform 中丢失的问题
- 修复 streamed tool events 不完整的问题

### P1

- 完善 `previous_response_id` / `store` / `metadata` / `reasoning` 等字段策略
- 完善 structured output 支持
- 完善多内容块与多模态输入表示
- 建立不支持能力的显式错误体系

### P2

- provider 能力探测自动化
- 更精细的 telemetry / diagnostics
- 更完整的模型路由策略
- OpenAI 与 Zhipu 特性差异文档化

---

## 十、测试与验证规划

后续演进不能只靠单元测试，需要建立分层验证。

### 10.1 当前测试现状

当前仓库已经有：

- provider 级测试
- transform 级测试
- integration tests

但目前测试更多覆盖：

- 类型转换
- 基础 endpoint 是否存在
- 某些静态格式兼容

而缺少：

- Codex CLI 真实工作流级别的回归测试
- `glm-5` 专项测试
- tool streaming e2e tests
- reasoning / structured output / previous_response_id 链路测试

### 10.2 后续必须补齐的测试层次

建议建立 4 层验证：

1. 模型层测试  
   验证序列化/反序列化是否保真

2. transform 层测试  
   验证 `Responses <-> internal <-> Chat` 的字段映射

3. handler/proxy 层测试  
   验证路由、fallback、错误处理、能力协商

4. e2e 场景测试  
   验证 Codex CLI 核心场景：
   - 普通文本
   - function calling
   - tool call output
   - streaming
   - `previous_response_id`
   - `glm-5` 路由

---

## 十一、配置与文档规划

除了代码改造，后续还必须同步做两类非代码工作。

### 11.1 配置层规划

需要完成：

- 明确 `glm-5` 路由示例
- 明确 `zhipu.default_model` 推荐值
- 明确 OpenAI/Zhipu 双 provider 配置模板
- 增加 Codex CLI 使用示例配置

### 11.2 文档层规划

需要完成：

- 对外说明哪些 Codex 能力已支持
- 对外说明哪些能力仍不支持
- 提供 `glm-5` 推荐配置
- 提供故障排查指南
- 提供 `Responses API` 与 `Chat Completions API` 的支持矩阵

---

## 十二、推荐的架构演进方向

### 12.1 不建议继续单纯堆补丁

如果继续在当前结构上只做局部加字段、局部补 SSE 事件，短期能改善兼容性，但中期会越来越复杂：

- transform 逻辑继续膨胀
- provider 行为继续分叉
- Chat 与 Responses 语义继续纠缠
- 维护者越来越难判断哪条链路是真正权威实现

### 12.2 建议引入 canonical internal protocol

更可持续的方向是：

1. 外部输入先进入统一 internal protocol
2. internal protocol 再映射到 provider capability
3. provider 返回结果再映射回 internal protocol
4. 最后 internal protocol 再序列化成目标 API 输出

这样可以把问题从：

`Responses <-> Chat <-> Provider`

升级成：

`Responses/Chat <-> Internal Canonical Model <-> Provider Adapter`

这是后续让项目同时稳定支持：

- OpenAI
- Zhipu
- Codex CLI
- 未来更多 provider

的关键。

---

## 十三、里程碑建议

建议按下面的里程碑推进，而不是一次性做“大重构”。

### 里程碑 M1：能力边界明确

验收标准：

- 有清晰的支持矩阵
- 有 `glm-5` 当前状态说明
- 有 provider capability 文档

### 里程碑 M2：模型层升级完成

验收标准：

- internal canonical model 落地
- tool schema 完整
- transform 行为受测

### 里程碑 M3：Responses-first 链路稳定

验收标准：

- `/v1/responses` 不再依赖脆弱的临时降级逻辑
- 对不支持能力会明确报错
- `previous_response_id` 等语义有清晰处理策略

### 里程碑 M4：Codex CLI 核心链路通过

验收标准：

- 文本场景通过
- function calling 场景通过
- streaming 场景通过
- tool events 场景通过

### 里程碑 M5：`glm-5` 正式支持

验收标准：

- `glm-5` 配置简单可用
- `glm-5` 回归测试稳定
- 文档清楚说明限制与推荐用法

---

## 十四、最终结论

当前 `rcodex` 已经具备继续演进为 Codex CLI 协议代理的基础，但还没有到“完整支持 Codex CLI + glm-5”的阶段。

项目当前最大的问题不是“缺少某一个字段”，而是：

- 架构仍以 Chat-first 为主
- 内部协议模型不足以承载 Codex 的完整语义
- tool / reasoning / streaming 的保真支持不完整

因此，后续规划不应只是继续补兼容逻辑，而应该围绕以下主线展开：

1. 明确能力边界
2. 升级内部模型
3. 重构 Responses-first 执行链路
4. 补齐流式和工具事件
5. 以 `glm-5` 为目标模型完成端到端验收

只有完成这条路线，`rcodex` 才能从“支持一部分 OpenAI/GLM 接口的代理”演进成“可稳定服务 Codex CLI 的协议代理”。

