# plan8.md - rcodex 与 mimo2codex 差距分析与下一步计划

## 一、现状总结

### plan7.md 完成情况: **109% ✅ 已完成**

所有 Phase 1-9 已完成，108 个测试通过。

---

## 二、差距分析

### 2.1 Provider 架构差距

| 功能 | mimo2codex | rcodex | 差距 |
|-----|-------------|--------|------|
| **Generic Provider** | ✅ 完整的 `GenericProviderSpec` JSON 配置 | ❌ 缺少通用 Provider 加载器 | **P0** |
| **Provider 预设** | ✅ `enhanceErrorPreset` 预设系统 | ❌ 缺少预设系统 | **P1** |
| **Provider 发现** | ✅ 从 `providers.json` 动态加载 | ❌ 硬编码 Provider | **P0** |
| **Multi-Provider 路由** | ✅ 完整的 `byClientModel()` 路由 | ✅ `ProviderRegistry` 存在但功能不完整 | P1 |

### 2.2 Model 能力差距

| 功能 | mimo2codex | rcodex | 差距 |
|-----|-------------|--------|------|
| **模型别名** | ✅ `aliases: string[]` | ❌ 缺少别名系统 | P1 |
| **模型能力检测** | ✅ `detectFlags()` 返回 `isTokenPlan` | ✅ 有基础实现 | P0 |
| **Thinking 模式** | ✅ `thinking: {type: "enabled"}` | ❌ 缺少完整支持 | P1 |
| **图片支持检测** | ✅ `supportsImages` 按模型 | ❌ 缺少模型级图片支持 | P0 |
| **WebSearch 检测** | ✅ `supportsWebSearch` | ❌ 缺少完整支持 | P1 |

### 2.3 Transform 层差距

| 功能 | mimo2codex | rcodex | 差距 |
|-----|-------------|--------|------|
| **inline think 提取** | ✅ `extractThinkTags` 处理 `<think>` | ❌ 缺少实现 | **P0** |
| **图片 materialization** | ✅ `materializeStrippedImage()` | ❌ 缺少实现 | **P0** |
| **thinking 默认值** | ✅ 按模型注入 `thinking: enabled` | ❌ 缺少按模型处理 | **P0** |
| **temperature 修正** | ✅ thinking 模式下删除 temperature | ❌ 缺少实现 | P1 |
| **tool_choice 修正** | ✅ 非 "auto" 时删除 | ✅ 有基础实现 | P0 |

### 2.4 流式处理差距

| 功能 | mimo2codex | rcodex | 差距 |
|-----|-------------|--------|------|
| **reasoning_summary_text** | ✅ 完整支持 | ⚠️ 有基础实现 | P1 |
| **output_item.added 事件** | ✅ 完整 | ✅ 已实现 | - |
| **annotation 处理** | ✅ 完整 | ⚠️ 有基础 | P2 |
| **Tool call ID/Name 分离** | ✅ 完整 | ⚠️ 有部分实现 | P1 |
| **StreamState 状态机** | ✅ 完整的状态跟踪 | ⚠️ 有简化实现 | P1 |

### 2.5 错误处理差距

| 功能 | mimo2codex | rcodex | 差距 |
|-----|-------------|--------|------|
| **ContextOverflow 检测** | ✅ 友好提示 | ⚠️ 有基础 | P1 |
| **WebSearch 未激活提示** | ✅ 特定错误提示 | ❌ 缺少 | **P0** |
| **400 错误增强** | ✅ `enhanceErrorPreset` | ❌ 缺少预设系统 | **P0** |
| **Token Plan 检测** | ✅ `isTokenPlan` flag | ⚠️ 有基础 | P1 |

### 2.6 配置与部署差距

| 功能 | mimo2codex | rcodex | 差距 |
|-----|-------------|--------|------|
| **providers.json** | ✅ JSON 配置文件 | ❌ 缺少 | **P0** |
| **环境变量配置** | ✅ `GENERIC_*` 支持 | ⚠️ 有基础 | P1 |
| **Admin UI** | ✅ Web 管理界面 | ❌ 缺少 | P2 |
| **数据库迁移** | ✅ SQLite schema | ❌ 缺少 | P2 |

---

## 三、代码量差距

| 组件 | mimo2codex | rcodex | 差距 |
|-----|-----------|--------|------|
| providers/ | ~1300 行 | ~400 行 | **-900 行** |
| translate/ | ~2100 行 | ~570 行 | **-1530 行** |
| config/ | ~300 行 | ~300 行 | - |
| auth/ | ~500 行 | ❌ 缺少 | **-500 行** |
| **总计** | ~4200 行 | ~1270 行 | **-2930 行** |

---

## 四、优先级排序

### P0 - 必须实现（阻塞功能）

1. **Generic Provider Loader**
   - 从 JSON 文件加载 Provider 配置
   - 支持 `wireApi: "chat" | "responses"`
   - 支持 `features: { webSearch, forceParallelToolCalls }`

2. **inline think 提取**
   - 实现 `extractThinkTags` 处理 `<think>`...`</think>` 标签
   - 正确分离 reasoning content

3. **图片 materialization**
   - 实现 `materializeStrippedImage()` 
   - 支持 data: URL 和 http(s): URL

4. **thinking 默认值注入**
   - 按模型注入 `thinking: {type: "enabled"}`
   - 处理 `mimo-v2-flash` 等禁用模型

5. **WebSearch 未激活错误提示**
   - 检测 `webSearchEnabled is false` 错误
   - 提供友好的激活提示

6. **Provider 预设系统**
   - 实现 `enhanceErrorPreset` 
   - 支持 sensenova, minimax 等预设

### P1 - 重要功能

7. **模型别名系统**
   - 实现 `aliases: Vec<String>`
   - 在 `resolveModel()` 中支持

8. **模型级图片支持**
   - 实现 `supportsImages` 检测
   - 按模型过滤图片

9. **ContextOverflow 友好提示**
   - 检测 context overflow 错误
   - 提供 /compact 提示

10. **temperature 修正**
    - thinking 模式下删除 temperature

11. **Tool call 完整处理**
    - 分离 `function_call.id` 和 `function_call.name` delta
    - 完整的 function_call 状态机

### P2 - 优化功能

12. **Admin UI**
13. **数据库 schema**
14. **日志系统完善**

---

## 五、Phase 10: Generic Provider 系统

### 目标
实现通用的 Provider 加载器，支持从 JSON 配置文件动态加载 Provider。

### 实现方案

```rust
// src/providers/generic_provider.rs
pub struct GenericProviderSpec {
    pub id: String,
    pub shortcut: Option<String>,
    pub display_name: Option<String>,
    pub base_url: String,
    pub env_key: String,
    pub default_model: String,
    pub wire_api: WireApi,
    pub models: Vec<ProviderModel>,
    pub features: GenericFeatures,
    pub force_default_model: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenericFeatures {
    pub web_search: Option<bool>,
    pub force_parallel_tool_calls: Option<bool>,
    pub enhance_error_preset: Option<String>,
    pub // minimax compat features...
}
```

### 关键实现

1. **ProviderSpec 解析**
   - 从 `~/.config/rcodex/providers.json` 加载
   - 环境变量覆盖支持

2. **GenericProvider trait**
   - 实现 `preprocess_chat()`
   - 实现 `preprocess_responses()`

3. **Registry 扩展**
   - `register_generic(spec: GenericProviderSpec)`
   - `load_from_file(path: &Path)`

---

## 六、Phase 11: Thinking/Reasoning 支持

### 目标
完善 thinking/reasoning 模式的处理。

### 实现方案

```rust
// src/transform/thinking.rs
pub struct ThinkTagSplitter {
    // 处理 <think>...</think> 标签
}

pub fn extract_inline_think(content: &str) -> (String, Option<String>) {
    // 返回 (纯消息内容, reasoning内容)
}
```

### 关键实现

1. **inline think 提取**
   - 正则匹配 `<think>`...`</think>`
   - 分离 reasoning content

2. **thinking 默认值注入**
   - 按模型决定是否注入
   - 禁用模型: `mimo-v2-flash`, `mimo-mini`

3. **temperature 修正**
   - thinking 模式下删除 temperature

---

## 七、Phase 12: 错误增强系统

### 目标
实现完整的错误增强系统，提供友好的错误提示。

### 实现方案

```rust
// src/error/enhancer.rs
pub struct ErrorPreset {
    pub id: String,
    pub patterns: Vec<ErrorPattern>,
}

pub struct ErrorPattern {
    pub status: u16,
    pub message_contains: String,
    pub hint: String,
    pub code: String,
}
```

### 预设实现

```rust
// Sensenova preset
const SENSENOVA_ERRORS: &[(&str, &str)] = &[
    ("message queue response", "会话上下文已失效，请重新开始对话"),
    ("max_tokens", "输出长度超限，请减少对话历史或降低 max_tokens"),
];

// MiMo WebSearch preset
const MIMO_WEBSREARCH_ERRORS: &[(&str, &str)] = &[
    ("webSearchEnabled is false", "Web Search 插件未激活"),
];
```

---

## 八、预估时间

| Phase | 工作内容 | 优先级 | 时间 |
|-------|---------|--------|------|
| Phase 10 | Generic Provider 系统 | P0 | 1-2 天 |
| Phase 11 | Thinking/Reasoning 支持 | P0 | 1 天 |
| Phase 12 | 错误增强系统 | P0 | 1 天 |
| Phase 13 | 模型能力完善 | P1 | 1 天 |
| Phase 14 | 流式处理完善 | P1 | 1 天 |
| **总计** | | | **5-7 天** |

---

## 九、风险与缓解

| 风险 | 可能性 | 影响 | 缓解 |
|-----|-------|------|------|
| Generic Provider JSON Schema 不兼容 | 中 | 中 | 先实现核心字段，逐步扩展 |
| Thinking 模式与现有代码冲突 | 高 | 中 | 添加 feature flag，默认关闭 |
| 错误增强影响正常流程 | 低 | 中 | 添加 dry_run 测试模式 |

---

## 十、下一步行动

### Immediate (本周)

1. 实现 GenericProviderSpec 和加载器
2. 实现 inline think 提取
3. 实现图片 materialization

### Short-term (两周内)

4. 完成 thinking 默认值注入
5. 实现错误增强预设
6. 完善模型能力检测

### Medium-term (一个月内)

7. 实现 Admin UI
8. 完善测试覆盖
9. 性能优化

---

## 十一、总结

rcodex 与 mimo2codex 的主要差距在于:

1. **Generic Provider 系统**: 最关键的缺失，需要动态加载 Provider
2. **Thinking/Reasoning 支持**:thinking 模式是现代 LLM 的核心能力
3. **错误增强**: 好的错误提示对用户体验至关重要
4. **模型能力检测**: 需要更细粒度的模型能力支持

预计需要 5-7 天完成 P0 功能，2-4 周完成所有核心功能。
