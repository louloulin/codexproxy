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
