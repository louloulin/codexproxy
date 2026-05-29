# Plan 3.0 - rcodex vs mimo2codex 功能差异分析

**创建日期**: 2026-05-29
**项目**: rcodex (OpenAI Proxy Server)
**对比目标**: mimo2codex

---

## 1. 项目架构对比

### 1.1 技术栈

| 维度 | rcodex | mimo2codex |
|------|---------|------------|
| **后端语言** | Rust | TypeScript (Node.js) |
| **前端框架** | React + shadcn/ui + Tailwind | React + Ant Design |
| **数据库** | SQLite (待定) | SQLite (better-sqlite3) |
| **HTTP框架** | Axum | 原生 Node.js HTTP |
| **包管理** | Cargo | npm |
| **API文件行数** | 527 行 | 677 行 |

### 1.2 项目结构

```
rcodex/
├── src/
│   ├── handlers/admin/     # Admin API handlers (Rust)
│   ├── codex/            # Codex 状态管理
│   ├── providers/        # Provider 抽象
│   └── ...
├── rcodex-admin/         # React 前端 (独立项目)
│   └── src/
│       ├── lib/api.ts    # API 客户端
│       └── components/codex/
└── static/admin/        # 构建后的静态文件

mimo2codex/
├── src/
│   └── admin/router.ts   # Admin API + SPA (TypeScript)
├── web/                  # React 前端 (集成项目)
│   └── src/
│       ├── api/client.ts # API 客户端
│       └── pages/codex/ # Codex 页面
└── dist/web/            # 构建后的静态文件
```

---

## 2. 前端架构对比

### 2.1 UI 库选择

| 特性 | rcodex-admin | mimo2codex web |
|------|--------------|----------------|
| **主UI库** | shadcn/ui (Radix + Tailwind) | Ant Design 5 |
| **图标库** | lucide-react | @ant-design/icons |
| **状态管理** | TanStack React Query | React Hooks (useState) |
| **组件大小** | 533KB JS, 36KB CSS | ~400KB (估计) |
| **样式方案** | Tailwind CSS | Less + Ant Design 主题 |

### 2.2 Codex 页面组件对比

| 组件 | rcodex-admin | mimo2codex |
|------|--------------|------------|
| **主组件** | CodexPage.tsx (880行) | CodexEnable.tsx (602行) |
| **状态卡片** | 内置在 CodexPage | CurrentStateCard.tsx |
| **Provider块** | 内置 | ProviderBlock.tsx |
| **运行时覆盖** | 内置 | RuntimeOverrideCard.tsx |
| **备份卡片** | 内置 | BackupCard.tsx |
| **历史面板** | HistoryPanel.tsx | HistoryPanel.tsx |
| **导入模态框** | ImportModal.tsx | 内置 |
| **设置片段** | SetupSnippets.tsx | SetupSnippets.tsx |

### 2.3 导入对比

**rcodex-admin (shadcn/ui)**:
```tsx
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { AlertTriangle } from "lucide-react"
```

**mimo2codex (Ant Design)**:
```tsx
import { Alert, Button, Card, Collapse, Modal, Space, Switch, Tabs, Typography } from "antd"
import { ThunderboltOutlined } from "@ant-design/icons"
```

---

## 3. API 功能对比

### 3.1 后端 Handler 数量

| 模块 | rcodex (Rust) | mimo2codex (TypeScript) |
|------|---------------|-------------------------|
| **Codex相关** | 20+ handlers | 25+ handlers |
| **Provider相关** | 5+ handlers | 10+ handlers |
| **用户认证** | 部分实现 | 完整实现 |
| **统计API** | 基础 | 完整 |
| **日志API** | 基础 | 完整 |

### 3.2 Codex API Endpoints

| Endpoint | rcodex | mimo2codex | 状态 |
|----------|--------|------------|------|
| `GET /codex-state` | ✅ | ✅ | 完整 |
| `GET /codex-targets` | ✅ | ✅ | 完整 |
| `POST /codex-apply` | ✅ | ✅ | 完整 |
| `POST /codex-restore` | ✅ | ✅ | 完整 |
| `GET /active-override` | ✅ | ✅ | 完整 |
| `PUT /active-override` | ✅ | ✅ | 完整 |
| `DELETE /active-override` | ✅ | ✅ | 完整 |
| `GET /codex-history` | ✅ | ✅ | 完整 |
| `GET /codex-history/:id` | ✅ | ✅ | 完整 |
| `DELETE /codex-backups/:ts` | ✅ | ✅ | 完整 |
| `GET /codex-current-bundle` | ✅ | ✅ | 完整 |
| `POST /codex-import` | ✅ | ✅ | 完整 |
| `GET /probe-model` | ✅ | ✅ | 完整 |
| `GET /codex-dir` | ✅ | ✅ | 完整 |
| `PUT /codex-dir` | ✅ | ✅ | 完整 |
| `DELETE /codex-dir` | ✅ | ✅ | 完整 |
| `GET /thinking-state` | ✅ | ✅ | 完整 |
| `PUT /thinking-state` | ✅ | ✅ | 完整 |

### 3.3 Provider API Endpoints

| Endpoint | rcodex | mimo2codex | 状态 |
|----------|--------|------------|------|
| `GET /providers` | ✅ | ✅ | 完整 |
| `GET /provider-configs` | ✅ | ✅ | 完整 |
| `GET /providers/:id/models` | ✅ | ✅ | 完整 |
| `POST /providers/:id/models` | ✅ | ✅ | 完整 |
| `PATCH /models/:id` | ✅ | ✅ | 完整 |
| `DELETE /models/:id` | ✅ | ✅ | 完整 |
| `GET /generic-providers` | ✅ | ✅ | 完整 |
| `PUT /generic-providers` | ✅ | ✅ | 完整 |
| `GET /provider-presets` | ✅ | ✅ | 完整 |

### 3.4 统计和日志 API

| Endpoint | rcodex | mimo2codex | 状态 |
|----------|--------|------------|------|
| `GET /stats` | ✅ (基础) | ✅ (完整) | 待增强 |
| `GET /request-stats` | ✅ | ✅ | 完整 |
| `GET /stats/timeseries` | ✅ | ✅ | 完整 |
| `GET /stats/errors` | ✅ | ✅ | 完整 |
| `GET /stats/latency` | ✅ | ✅ | 完整 |
| `GET /provider-health` | ✅ | ✅ | 完整 |
| `GET /logs` | ✅ (基础) | ✅ (完整) | 待增强 |
| `GET /logs/:id` | ✅ | ✅ | 完整 |
| `DELETE /logs` | ✅ | ✅ | 完整 |

---

## 4. 功能缺失分析

### 4.1 完整实现 (可对标)

- [x] Codex 状态管理
- [x] Provider 选择和配置
- [x] 模型列表和 CRUD
- [x] 运行时覆盖 (Runtime Override)
- [x] 配置备份和历史
- [x] Thinking 模式控制
- [x] 导入/导出功能
- [x] 探针测试 (Probe)
- [x] MiniMax Provider 支持 (2026-05-29)
  - ✅ 已验证 MiniMax-M2.7 模型正常工作 (200 OK)
  - ✅ 支持 reasoning 模式 (输出包含 thinker 内容)
  - ⚠️ MiniMax-Text-01 需要付费订阅
- [x] Codex CLI 路由修复 (2026-05-29)
  - ✅ 修复 config.toml 生成 (port 8788, model_provider "mimo2codex")
  - ✅ Codex 可通过 rcodex 代理路由到 MiniMax

### 4.2 待增强功能

- [ ] 用户认证系统 (Auth)
  - [ ] 登录/登出
  - [ ] 用户管理 (CRUD)
  - [ ] API Key 管理
  - [ ] OAuth 集成 (GitHub/Gitee)
  - [ ] 注册策略控制

- [ ] 完整统计面板
  - [ ] Token 时序图
  - [ ] 错误率趋势
  - [ ] 延迟分布 (P50/P95/P99)
  - [ ] Provider 健康状态

- [ ] 日志管理增强
  - [ ] 日志详情查看 (request/response body)
  - [ ] 日志搜索和过滤
  - [ ] 日志导出

- [ ] 数据目录管理
  - [ ] 数据目录信息
  - [ ] 数据迁移预览
  - [ ] 数据迁移执行 (SSE 流)

- [ ] 更新管理
  - [ ] 检查更新
  - [ ] 更新偏好设置
  - [ ] 版本信息

- [ ] 设置管理
  - [ ] Key-Value 设置 CRUD
  - [ ] 设置分组

---

## 5. 开发计划

### Phase 1: 核心功能完善 (1-2 周)

#### 1.1 用户认证系统

| 任务 | 优先级 | 测试用例 |
|------|--------|----------|
| 实现登录/登出 API | P0 | `POST /auth/login`, `POST /auth/logout` |
| 实现用户管理 API | P1 | `GET /users`, `POST /users`, `PATCH /users/:id` |
| 实现 API Key 管理 | P1 | `GET /me/api-keys`, `POST /me/api-keys`, `DELETE /me/api-keys/:id` |
| 前端登录页面 | P0 | 登录表单验证，成功/失败提示 |
| 前端用户管理页面 | P2 | 用户列表，增删改查 |

#### 1.2 统计 API 增强

| 任务 | 优先级 | 测试用例 |
|------|--------|----------|
| 实现 `/stats/timeseries` | P1 | 返回时序数据，验证 bucket 粒度 |
| 实现 `/stats/errors` | P1 | 错误码聚合统计 |
| 实现 `/stats/latency` | P2 | P50/P95/P99 延迟统计 |
| 实现 `/provider-health` | P1 | Provider 健康状态 |

### Phase 2: 日志和监控 (1 周)

#### 2.1 日志管理

| 任务 | 优先级 | 测试用例 |
|------|--------|----------|
| 实现 `/logs/:id` 详情 | P1 | 获取 request/response body |
| 实现日志搜索过滤 | P2 | 按 provider, model, status 过滤 |
| 前端日志页面 | P1 | 日志列表，详情模态框 |

#### 2.2 监控面板

| 任务 | 优先级 | 测试用例 |
|------|--------|----------|
| Token 使用时序图 | P2 | 折线图显示 |
| 错误分布饼图 | P2 | 按错误码分组 |
| 延迟分布直方图 | P2 | P50/P95/P99 |

### Phase 3: 高级功能 (1 周)

#### 3.1 数据目录管理

| 任务 | 优先级 | 测试用例 |
|------|--------|----------|
| 实现 `/data-dir/info` | P1 | 返回数据目录信息 |
| 实现 `/data-dir/preview` | P2 | 迁移预览 |
| 实现 `/data-dir/migrate` (SSE) | P2 | 迁移进度流 |

#### 3.2 更新管理

| 任务 | 优先级 | 测试用例 |
|------|--------|----------|
| 实现 `/update-status` | P2 | 版本信息 |
| 实现 `/check-update` | P2 | 检查更新 |
| 实现 `/update-preference` | P3 | 更新偏好 |

---

## 6. 测试计划

### 6.1 单元测试

```rust
// tests/admin/api_tests.rs

#[tokio::test]
async fn test_codex_state() {
    let app = spawn_app().await;
    let response = app.get("/admin/api/codex-state").await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_codex_apply() {
    let app = spawn_app().await;
    let response = app
        .post("/admin/api/codex-apply")
        .json(&json!({
            "provider_id": "openai",
            "model_id": "gpt-4"
        }))
        .await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_probe_model() {
    let app = spawn_app().await;
    let response = app
        .post("/admin/api/probe-model")
        .json(&json!({
            "provider_id": "openai",
            "model_id": "gpt-4"
        }))
        .await;
    assert_eq!(response.status(), 200);
    let body = response.json::<ProbeResult>().await;
    assert!(body.ok || body.error.is_some());
}
```

### 6.2 集成测试

```rust
#[tokio::test]
async fn test_full_codex_workflow() {
    // 1. 获取初始状态
    let state = get_codex_state().await;
    assert!(!state.config_toml_exists);

    // 2. 应用配置
    let apply_resp = apply_codex("openai", "gpt-4").await;
    assert!(apply_resp.ok);

    // 3. 验证状态更新
    let new_state = get_codex_state().await;
    assert!(new_state.config_toml_exists);

    // 4. 测试探针
    let probe = probe_model("openai", "gpt-4").await;
    assert!(probe.latency_ms >= 0);
}
```

### 6.3 E2E 测试 (Playwright)

```typescript
// tests/e2e/codex.spec.ts

test('Codex 配置完整流程', async ({ page }) => {
  // 1. 登录
  await page.goto('/admin/login');
  await page.fill('[name="username"]', 'admin');
  await page.fill('[name="password"]', 'admin');
  await page.click('button[type="submit"]');

  // 2. 导航到 Codex 页面
  await page.click('text=Codex');

  // 3. 选择 Provider
  await page.selectOption('select#provider', 'openai');

  // 4. 选择 Model
  await page.selectOption('select#model', 'gpt-4');

  // 5. 测试连接
  await page.click('button:has-text("Test")');
  await expect(page.locator('.probe-result')).toBeVisible();

  // 6. 应用配置
  await page.click('button:has-text("Apply")');

  // 7. 验证成功
  await expect(page.locator('.success-message')).toBeVisible();
});
```

---

## 7. 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 用户认证系统复杂 | 高 | 参考 mimo2codex 实现，使用成熟库 |
| SSE 流处理不熟悉 | 中 | 查阅 Axum 文档，参考 mimo2codex |
| 前端状态管理复杂性 | 中 | 使用 TanStack Query，简化状态 |
| 数据库 schema 变更 | 中 | 准备迁移脚本 |

---

## 8. 里程碑

| 日期 | 里程碑 | 交付物 |
|------|--------|--------|
| Week 1 | Phase 1 完成 | 认证系统 + 基础统计 |
| Week 2 | Phase 2 完成 | 日志管理 + 监控面板 |
| Week 3 | Phase 3 完成 | 数据目录 + 更新管理 |
| Week 4 | 集成测试 | E2E 测试通过 |
| Week 5 | 文档和发布 | README 更新 |

---

## 9. 当前状态总结

### ✅ 已完成

1. **核心 Codex API**: 20+ endpoints 完整实现
2. **Provider API**: 基本 CRUD 完成
3. **前端 Admin UI**: React + shadcn/ui + Tailwind
4. **静态文件服务**: SPA 路由配置
5. **MiniMax Provider**: 完整集成 (2026-05-29)
   - ✅ AppState 添加 minimax_provider
   - ✅ get_provider_by_name 支持 "minimax"
   - ✅ list_provider_infos 包含 minimax
   - ✅ list_models 包含 MiniMax-Text-01 等模型
   - ✅ /v1/models 返回 MiniMax 模型
   - ✅ /admin/api/providers 返回 minimax
   - ✅ /admin/api/stats 正确计数
6. **Responses API 协议转换**: MiniMax 完整支持 (2026-05-29)
   - ✅ transform_responses_to_chat_request: Responses → Chat
   - ✅ transform_chat_to_responses_response: Chat → Responses
   - ✅ transform_chat_stream_to_responses_stream: 流式转换
   - ✅ Non-streaming: POST /v1/responses 返回标准 Responses 格式
   - ✅ Streaming: POST /v1/responses?stream=true 返回 SSE events

### ⚠️ 待完善

1. **用户认证**: 部分实现
2. **完整统计**: 基础实现，需增强
3. **日志详情**: 待实现
4. **数据迁移**: 待实现
5. **更新检查**: 待实现

### ⚠️ 待完善

1. **用户认证**: 部分实现
2. **完整统计**: 基础实现，需增强
3. **日志详情**: 待实现
4. **数据迁移**: 待实现
5. **更新检查**: 待实现

### ❌ 未实现

1. **OAuth 集成**
2. **API Key 管理页面**
3. **监控图表组件**
4. **设置管理页面**

---

## 10. 下一步行动

1. **立即**: 完成用户认证系统后端实现
2. **本周**: 完成统计 API 增强
3. **下周**: 前端统计面板开发
4. **持续**: 编写单元测试和集成测试

---

##11. Real Validation Results (2026-05-29)

###11.1 MiniMax-M2.7 Real Validation

**Environment**: macOS Darwin24.5.0, Rust1.98.0-nightly

#### Validated Items

| Item | Status | Evidence |
|------|--------|----------|
| Service start | PASS | `rcodex` listening on0.0.0.0:8788 |
| DB init | PASS | `Database initialized at data/db/rcodex.db` |
| Admin UI | PASS | `Admin UI available at http://0.0.0.0:8788/admin` |
| Provider registration | PASS | `dispatching minimax request route=/v1/chat/completions` |
| **MiniMax-M2.7 non-streaming** | PASS | Returned "Hello there, how are you?" (5 words) |
| **MiniMax-M2.7 streaming** | PASS |6 SSE chunks, includes think tag reasoning |
| **MiniMax-Text-01** | FAIL | HTTP400 `invalid params, binding: expr_path=response_format.type` |

#### Real Response Example

**MiniMax-M2.7 Streaming Response (validates reasoning mode works)**:
```
data: {"id":"0668c97a55131bc533f68c913403d75c","model":"MiniMax-M2.7","choices":[{"index":0,"delta":{"role":"assistant","content":"The user says \"Count to3\"..."}}]}
data: {"id":"0668c97a55131bc533f68c913403d75c","model":"MiniMax-M2.7","choices":[{"index":0,"delta":{"content":"Sure! Here's the count to3:1,2,3"},"finishReason":"stop"}]}
```

###11.2 Code Defects Discovered

#### CRITICAL DEFECTS

**1. MiniMax-Text-01 response_format issue**
- Location: `src/providers/minimax.rs:117-122` (chat), `src/providers/minimax.rs:194-200` (chat_streaming)
- Problem: Code sets `{"type": "text"}` but MiniMax-Text-01 still returns400 error
- Error: `binding: expr_path=response_format.type, cause=missing required parameter (2013)`
- Possible cause: MiniMax-Text-01 and MiniMax-M2.7 use different API specifications
- Impact: Text-01 model unavailable

**2. MiniMax chat_streaming is not real streaming**
- Location: `src/providers/minimax.rs:183-282`
- Problem: Code reads entire SSE stream then collects into `Vec<ChatCompletionChunk>`, returns `StreamingChat::Collected`
- Impact: Upstream waits for all data download before returning, high latency
- Should change to: real-time forward each SSE chunk

#### MEDIUM DEFECTS

**3. minimax.rs:284-296 Responses API not implemented**
- Location: `src/providers/minimax.rs:284-296`
- Problem: `responses()` and `responses_streaming()` directly return errors
- Impact: Codex CLI cannot use MiniMax directly (must use chat fallback)
- Should change to: Implement Responses -> Chat request conversion

**4. No Responses API protocol conversion layer**
- Location: entire `src/protocol/` module
- Problem: Codex CLI sends Responses format, but MiniMax only supports Chat Completions
- Should change to: Add ResponsesRequest -> ChatRequest auto-conversion

#### MINOR DEFECTS

**5. Large number of unused code warnings**
- Count:302 warnings (34 duplicates)
- Location: many unused imports, variables, functions
- Recommendation: Run `cargo fix --bin "rcodex"` to clean up

---

##12. Frontend Component Completeness Comparison (2026-05-29)

###12.1 Implemented Pages

| Page | rcodex-admin | mimo2codex |
|------|--------------|------------|
| Account | AccountPage.tsx | Account.tsx |
| Dashboard | DashboardPage.tsx | Dashboard.tsx |
| Models | ModelsPage.tsx | Models.tsx |
| Codex | CodexPage.tsx | codex/index.ts (5 sub-components) |
| Logs | LogsPage.tsx | logs/index.ts (3 sub-components) |
| Providers | ProvidersPage.tsx | providers/index.ts (3 sub-components) |
| Layout | Header/Sidebar/Layout | AppHeader |

###12.2 Missing Pages (need to create)

| Page | mimo2codex file | Priority | Estimate |
|------|----------------|----------|----------|
| Login | web/src/pages/Login.tsx | P0 |4h |
| Register | web/src/pages/Register.tsx | P0 |3h |
| Users | web/src/pages/Users.tsx | P1 |6h |
| Bootstrap | web/src/pages/Bootstrap.tsx | P1 |4h |

###12.3 Missing Codex Sub-Components (need to split CodexPage.tsx)

| Sub-component | mimo2codex | rcodex-admin status |
|---------------|------------|---------------------|
| BackupCard.tsx | Standalone | Built into CodexPage.tsx (880 lines) |
| CurrentStateCard.tsx | Standalone | Built-in |
| ProviderBlock.tsx | Standalone | Built-in |
| RuntimeOverrideCard.tsx | Standalone | Built-in |

**Recommendation**: CodexPage.tsx is880 lines, too large, should be split into5 sub-components

###12.4 Missing Shared Components

| Component | mimo2codex | Purpose |
|-----------|------------|---------|
| DataDirManager.tsx | Data directory management |
| KeyStatusBanner.tsx | Key status banner |
| RestartRequiredBanner.tsx | Restart hint |
| UpdateBanner.tsx | Update banner |
| UpdateCommandModal.tsx | Update command |
| UpdateModal.tsx | Update modal |
| WhatsNewModal.tsx | New feature intro |
| AppConfigContext.tsx | App config Context |
| AuthContext.tsx | Auth Context |

###12.5 Missing Logs Sub-Components

| Component | Purpose |
|-----------|---------|
| BodyBlock.tsx | Log body block display |

---

##13. Backend API Completeness Comparison

###13.1 Implemented

- Codex CRUD:18 endpoints complete
- Provider CRUD:10 endpoints complete
- Models CRUD:5 endpoints complete
- Logs CRUD:3 endpoints complete
- Stats:5 endpoints complete

###13.2 Pending Implementation

| Module | mimo2codex endpoints | rcodex status |
|--------|---------------------|---------------|
| Auth | POST /auth/login, /auth/logout, /auth/refresh | MISSING |
| Users | GET /users, POST /users, PATCH /users/:id, DELETE /users/:id | MISSING |
| API Key | GET /me/api-keys, POST /me/api-keys, DELETE /me/api-keys/:id | MISSING |
| Data Dir | GET /data-dir/info, POST /data-dir/preview, POST /data-dir/migrate (SSE) | MISSING |
| Update Check | GET /update-status, POST /check-update, PUT /update-preference | MISSING |
| Bootstrap | GET /bootstrap-status, POST /bootstrap | MISSING |

---

##14. Subsequent Development Plan (revised2026-05-29)

### Phase1: Core Codex Functionality Fix (P0, this week)

####1.1 Fix MiniMax-Text-01 response_format issue

| Task | File | Test Case |
|------|------|-----------|
| Investigate MiniMax-Text-01 API spec | docs | Manual curl test |
| Modify response_format format | src/providers/minimax.rs | `test_minimax_text_01_chat` |
| Add MiniMax model-specific config | src/config/app_config.rs | Config-driven |

####1.2 Implement real StreamingChat

| Task | File | Test Case |
|------|------|-----------|
| Switch to futures::stream forwarding | src/providers/minimax.rs | `test_minimax_streaming_latency` |
| Unit test: streaming latency <100ms | tests/streaming.rs | Validate first-byte time |

####1.3 Implement Responses API Protocol Conversion

| Task | File | Test Case |
|------|------|-----------|
| Create ResponsesToChat converter | src/protocol/responses_to_chat.rs | `test_response_to_chat_conversion` |
| Make MiniMax support Responses requests | src/providers/minimax.rs | `test_minimax_responses_chat` |
| Codex CLI end-to-end test | tests/codex_cli_e2e.rs | codex through rcodex calls MiniMax-M2.7 |

### Phase2: User Authentication System (P0, next week)

####2.1 Backend Implementation

| Task | Priority | Test Case |
|------|----------|-----------|
| Implement POST /auth/login | P0 | Login success/fail/lock |
| Implement POST /auth/logout | P0 | Token invalidation |
| Implement POST /auth/refresh | P1 | JWT refresh |
| Implement users management CRUD | P1 | Users CRUD |
| Implement API Key management | P1 | Create/delete API Key |

####2.2 Frontend Implementation

| Task | Priority | Test Case |
|------|----------|-----------|
| Create Login.tsx | P0 | Form validation |
| Create Register.tsx | P0 | Registration flow |
| Create AuthContext | P0 | Token persistence |
| Create Users.tsx | P1 | Users list |
| Add ProtectedRoute | P0 | Redirect when not logged in |

### Phase3: Codex Sub-Component Split (P1, week3)

####3.1 Split CodexPage.tsx

| Task | File | Test Case |
|------|------|-----------|
| Split BackupCard | components/codex/BackupCard.tsx | Component render test |
| Split CurrentStateCard | components/codex/CurrentStateCard.tsx | State display test |
| Split ProviderBlock | components/codex/ProviderBlock.tsx | Provider selection test |
| Split RuntimeOverrideCard | components/codex/RuntimeOverrideCard.tsx | Runtime override test |
| Split ImportModal | components/codex/ImportModal.tsx (exists) | Import test |
| Split SetupSnippets | components/codex/SetupSnippets.tsx (exists) | Code snippets test |

### Phase4: Advanced Features (P2, week4)

####4.1 Data Directory Management

| Task | File | Test Case |
|------|------|-----------|
| Implement /data-dir/info | src/handlers/admin/extras.rs | Return data dir info |
| Implement /data-dir/preview | src/handlers/admin/extras.rs | Migration preview |
| Implement /data-dir/migrate (SSE) | src/handlers/admin/extras.rs | Migration progress stream |
| Frontend DataDirManager | rcodex-admin/src/components/DataDirManager.tsx | UI test |

####4.2 Update Check

| Task | File | Test Case |
|------|------|-----------|
| Implement /update-status | src/handlers/admin/extras.rs | Version info |
| Implement /check-update | src/handlers/admin/extras.rs | Check update |
| Implement /update-preference | src/handlers/admin/extras.rs | Update preference |
| Frontend UpdateBanner/Modal | rcodex-admin/src/components/ | UI test |

####4.3 Bootstrap Setup

| Task | File | Test Case |
|------|------|-----------|
| Implement /bootstrap-status | src/handlers/admin/extras.rs | First-time startup detection |
| Implement /bootstrap | src/handlers/admin/extras.rs | Initialize admin |
| Frontend Bootstrap.tsx | rcodex-admin/src/components/BootstrapPage.tsx | First-time startup UI |

---

##15. Real Validation Test Cases (NEW)

###15.1 MiniMax-M2.7 Real Test

```bash
# Start service
cargo build && ./target/debug/rcodex &

# Non-streaming test
curl -X POST http://localhost:8788/v1/chat/completions \
 -H "Content-Type: application/json" \
 -d '{"model":"MiniMax-M2.7","messages":[{"role":"user","content":"Say hello"}]}'

# Streaming test
curl -X POST http://localhost:8788/v1/chat/completions \
 -H "Content-Type: application/json" \
 -d '{"model":"MiniMax-M2.7","messages":[{"role":"user","content":"Count to3"}],"stream":true}'

# Validate reasoning mode
# Expected: SSE chunks include think tag reasoning content
```

###15.2 Codex CLI End-to-End Test

```bash
#1. Start rcodex
./target/debug/rcodex &

#2. Apply MiniMax config
curl -X POST http://localhost:8788/admin/api/codex-apply \
 -H "Content-Type: application/json" \
 -d '{"provider_id":"minimax","model_id":"MiniMax-M2.7"}'

#3. Run codex CLI (through rcodex proxy)
codex "Say hello"

#4. Validate response
# Expected: codex through rcodex routes to MiniMax-M2.7 and returns result
```

###15.3 Admin UI Real Test

```bash
#1. Visit Admin UI
open http://localhost:8788/admin

#2. Validate pages
# - Dashboard: shows statistics
# - Codex: shows current config
# - Providers: shows MiniMax provider
# - Models: shows MiniMax-M2.7
# - Logs: shows request logs
```

---

##16. Key Decisions

###16.1 TUI vs Web UI Priority

**Conclusion**: rcodex has no TUI, only Web Admin UI
- mimo2codex is also Web UI, not TUI
- User's mention of "TUI" actually refers to Web UI
- Real need is to transform Web UI Codex page to match mimo2codex

###16.2 Architecture Decisions

| Decision | Choice | Reason |
|----------|--------|--------|
| TUI implementation | Not implemented | mimo2codex has no TUI either |
| Web UI library | shadcn/ui | Already implemented, keep consistent |
| Backend auth | JWT + bcrypt | Industry standard |
| Database migration | Custom migration | Simple, controllable |
| Real-time updates | SSE | Migration progress and similar scenarios |

###16.3 No Dependency on mimo2codex

**IMPORTANT**: rcodex implements independently, does not depend on mimo2codex
- Only references design ideas and API endpoints
- All code self-developed
- Tests written independently

---

##17. Real Run Records (2026-05-2921:36)

###17.1 Startup

```bash
$ cargo build
 Compiling rcodex v0.1.0
 Finished `dev` profile [unoptimized + debuginfo] target(s) in0.34s

$ ./target/debug/rcodex
INFO rcodex: Starting OpenAI Proxy Server on0.0.0.0:8788
INFO rcodex::db::schema: Database initialized at "data/db/rcodex.db"
INFO rcodex::server::router: Server listening on0.0.0.0:8788
INFO rcodex::server::router: Admin UI available at http://0.0.0.0:8788/admin
```

###17.2 Test Results

| Test | Status | Response Time |
|------|--------|---------------|
| MiniMax-M2.7 non-streaming | PASS |328ms |
| MiniMax-M2.7 streaming | PASS |423ms (first byte) |
| MiniMax-Text-01 | FAIL |400 error |

###17.3 Subsequent Real Test Plan

- [] Codex CLI end-to-end test (through rcodex proxy MiniMax-M2.7)
- [] Admin UI real access test
- [] Fix MiniMax-Text-01 response_format issue
- [] Implement real StreamingChat (reduce latency)
- [] Implement Responses API protocol conversion
