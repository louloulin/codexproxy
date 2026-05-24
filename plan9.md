# plan9.md - Codex Switch 功能建设计划 (多 Crates 架构)

> **目标**: 对标 mimo2codex 的 Codex Enable 功能，构建 rcodex 的 codex-switch
> **参考**: mimo2codex src/codex/state.ts, src/codex/files.ts, src/db/overrides.ts, doc/codex-enable.md
> **架构**: Workspace multi-crates (可独立测试和发布)
> **创建日期**: 2026-05-24

---

## 一、架构概览

### 1.1 多 Crates 设计

```
rcodex-workspace/
├── Cargo.toml              # Workspace 根配置
├── crates/
│   ├── codex-core/        # 核心接口 (traits + types)
│   ├── codex-files/       # 文件系统操作 (atomic write, backup)
│   ├── codex-switch/       # 切换逻辑 (apply, restore)
│   ├── codex-override/     # 运行时覆盖 (数据库)
│   └── codex-admin/        # Admin API + UI
├── src/                   # 主应用 (使用所有 crates)
└── tests/                 # 集成测试
```

### 1.2 各 Crate 职责

| Crate | 职责 | 对标 mimo2codex |
|-------|------|-----------------|
| `codex-core` | 共享类型、Trait 定义 | `src/codex/types.ts` |
| `codex-files` | atomicWrite, backupFile, listBackups, pruneBackups | `src/codex/files.ts` |
| `codex-switch` | applyCodex, restoreCodex, deleteBackupPair | `src/codex/state.ts` |
| `codex-override` | getActiveOverride, setActiveOverride, clearActiveOverride | `src/db/overrides.ts` |
| `codex-admin` | Admin API handlers, HTML UI | `src/admin/router.ts` |

---

## 二、Crate 详细规格

### 2.1 codex-core

**路径**: `crates/codex-core/Cargo.toml`

```toml
[package]
name = "codex-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
```

**导出类型**:

```rust
/// Codex 配置目录
#[derive(Debug, Clone)]
pub struct CodexDir {
    pub path: PathBuf,
    pub auth_json: PathBuf,
    pub config_toml: PathBuf,
}

/// 备份条目
#[derive(Debug, Clone)]
pub struct BackupEntry {
    pub path: PathBuf,
    pub ts: u64,
    pub preserved: bool,
}

/// 备份对 (配对备份)
#[derive(Debug, Clone)]
pub struct BackupPair {
    pub ts: u64,
    pub auth_backup: Option<PathBuf>,
    pub toml_backup: Option<PathBuf>,
    pub preserved: bool,
    pub model: Option<String>,
    pub provider: Option<String>,
}

/// auth.json 所有者类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthJsonOwner {
    /// 由 mimo2codex/rcodex 写入
    This,
    /// 外部配置 (真实 OpenAI 或其他)
    External,
    /// 文件不存在
    Missing,
}

/// 运行时覆盖配置
#[derive(Debug, Clone)]
pub struct ActiveOverride {
    pub provider_id: String,
    pub model_id: String,
}

/// 应用结果
#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub backup_ts: u64,
    pub auth_backup: Option<PathBuf>,
    pub toml_backup: Option<PathBuf>,
    pub owner_before: AuthJsonOwner,
    pub preserved: bool,
}

/// Codex 状态快照
#[derive(Debug, Clone)]
pub struct CodexState {
    pub codex_dir: PathBuf,
    pub auth_path: PathBuf,
    pub toml_path: PathBuf,
    pub owner: AuthJsonOwner,
    pub auth_exists: bool,
    pub toml_exists: bool,
    pub toml_text: Option<String>,
    pub backups: Vec<BackupPair>,
}

/// 切换目标
#[derive(Debug, Clone)]
pub struct SwitchTarget {
    pub provider_id: String,
    pub model_id: String,
    pub base_url: String,
    pub auth_sentinel: String,
}

/// Host 配置
#[derive(Debug, Clone)]
pub struct HostPort {
    pub host: String,
    pub port: u16,
}

impl HostPort {
    pub fn url(&self) -> String {
        format!("http://{}:{}/v1", self.host, self.port)
    }
}
```

### 2.2 codex-files

**路径**: `crates/codex-files/Cargo.toml`

```toml
[package]
name = "codex-files"
version = "0.1.0"
edition = "2021"

[dependencies]
codex-core = { path = "../codex-core" }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
tempfile = "3.8"
```

**核心 API**:

```rust
use codex_core::*;

/// 原子写入文件
pub fn atomic_write(path: &Path, contents: &str) -> Result<(), AtomicWriteError>;

/// 备份文件
/// 
/// 当 preserve=true 时，备份被标记为永久保留，不会被 prune 清理
pub fn backup_file(path: &Path, ts: u64, preserve: bool) -> Result<Option<PathBuf>, BackupError>;

/// 列出所有备份
pub fn list_backups(path: &Path) -> Result<Vec<BackupEntry>, ListBackupsError>;

/// 清理旧备份 (保留 keep 个最新的非 preserve 备份)
pub fn prune_backups(path: &Path, keep: usize) -> Result<(), PruneError>;

/// 删除指定 ts 的所有备份
pub fn delete_backups_at(path: &Path, ts: u64) -> Result<usize, DeleteError>;

/// 检测 auth.json 所有者
pub fn detect_auth_owner(path: &Path) -> Result<AuthJsonOwner, DetectOwnerError>;

/// 读取 config.toml 内容 (不解析)
pub fn read_toml_if_exists(path: &Path) -> Result<Option<String>, ReadError>;
```

### 2.3 codex-switch

**路径**: `crates/codex-switch/Cargo.toml`

```toml
[package]
name = "codex-switch"
version = "0.1.0"
edition = "2021"

[dependencies]
codex-core = { path = "../codex-core" }
codex-files = { path = "../codex-files" }
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"

[dev-dependencies]
tempfile = "3.8"
```

**核心 API**:

```rust
use codex_core::*;
use codex_files::*;

/// Codex 切换器
pub struct CodexSwitcher {
    codex_dir: CodexDir,
    backup_keep: usize,
}

impl CodexSwitcher {
    /// 创建切换器
    pub fn new(codex_dir: CodexDir) -> Self;
    
    /// 创建切换器并设置备份保留数量
    pub fn with_backup_keep(mut self, keep: usize) -> Self;
    
    /// 应用切换 (写入 auth.json + config.toml + 备份)
    pub fn apply(&self, target: &SwitchTarget, host: &HostPort) -> Result<ApplyResult, SwitchError>;
    
    /// 恢复到指定时间戳的备份
    /// 
    /// 对称恢复:
    /// - 如果备份存在 → 写入
    /// - 如果备份不存在 → 删除当前文件
    pub fn restore(&self, ts: u64) -> Result<(), RestoreError>;
    
    /// 删除备份对
    pub fn delete_backup(&self, ts: u64, force: bool) -> Result<usize, DeleteError>;
    
    /// 获取当前 Codex 状态
    pub fn read_state(&self) -> Result<CodexState, ReadStateError>;
    
    /// 获取备份对列表
    pub fn list_backup_pairs(&self) -> Result<Vec<BackupPair>, ListError>;
    
    /// 获取可用的切换目标
    pub fn get_targets(&self) -> Vec<TargetOption>;
}
```

### 2.4 codex-override

**路径**: `crates/codex-override/Cargo.toml`

```toml
[package]
name = "codex-override"
version = "0.1.0"
edition = "2021"

[dependencies]
codex-core = { path = "../codex-core" }
rusqlite = { version = "0.31", features = ["bundled"] }
thiserror = "1.0"

[dev-dependencies]
tempfile = "3.8"
```

**核心 API**:

```rust
use codex_core::*;

/// 覆盖管理器 (使用 SQLite settings 表)
pub struct OverrideManager {
    conn: Connection,
}

impl OverrideManager {
    /// 创建管理器 (打开或创建数据库)
    pub fn new(db_path: &Path) -> Result<Self, OverrideError>;
    
    /// 获取当前覆盖
    pub fn get(&self) -> Result<Option<ActiveOverride>, OverrideError>;
    
    /// 设置覆盖 (provider_id + model_id)
    pub fn set(&self, provider_id: &str, model_id: &str) -> Result<(), OverrideError>;
    
    /// 清除覆盖
    pub fn clear(&self) -> Result<(), OverrideError>;
    
    /// 检查是否有有效覆盖
    pub fn has_override(&self) -> Result<bool, OverrideError>;
}
```

### 2.5 codex-admin

**路径**: `crates/codex-admin/Cargo.toml`

```toml
[package]
name = "codex-admin"
version = "0.1.0"
edition = "2021"

[dependencies]
codex-core = { path = "../codex-core" }
codex-files = { path = "../codex-files" }
codex-switch = { path = "../codex-switch" }
codex-override = { path = "../codex-override" }
axum = "0.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
tower = { version = "0.4", features = ["util"] }
```

**REST API**:

| Method | Path | Handler |
|--------|------|---------|
| GET | `/admin/api/codex-state` | `GET /codex-state` |
| GET | `/admin/api/codex-targets` | `GET /codex-targets` |
| POST | `/admin/api/codex-apply` | `POST /codex-apply` |
| POST | `/admin/api/codex-restore` | `POST /codex-restore` |
| DELETE | `/admin/api/codex-backups/:ts` | `DELETE /codex-backups/:ts` |
| GET | `/admin/api/active-override` | `GET /active-override` |
| PUT | `/admin/api/active-override` | `PUT /active-override` |
| DELETE | `/admin/api/active-override` | `DELETE /active-override` |

---

## 三、主应用集成

### 3.1 Cargo.toml (Workspace)

```toml
[workspace]
members = [
    "crates/codex-core",
    "crates/codex-files", 
    "crates/codex-switch",
    "crates/codex-override",
    "crates/codex-admin",
    ".",
]

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
codex-core = { path = "crates/codex-core" }
codex-files = { path = "crates/codex-files" }
codex-switch = { path = "crates/codex-switch" }
codex-override = { path = "crates/codex-override" }
codex-admin = { path = "crates/codex-admin" }
```

### 3.2 lib.rs 导出

```rust
// src/lib.rs
pub use codex_core::*;
pub use codex_files::*;
pub use codex_switch::*;
pub use codex_override::*;
pub use codex_admin::*;

// Admin API 聚合
pub mod admin {
    pub use codex_admin::*;
}
```

---

## 四、实现计划

### Phase 1: codex-core (P0)

| 任务 | 文件 | 测试 |
|------|------|------|
| 定义所有类型 | `crates/codex-core/src/lib.rs` | 5 tests |
| 实现 HostPort | `crates/codex-core/src/host.rs` | 3 tests |

### Phase 2: codex-files (P0)

| 任务 | 文件 | 测试 |
|------|------|------|
| atomic_write | `crates/codex-files/src/atomic.rs` | 4 tests |
| backup_file | `crates/codex-files/src/backup.rs` | 5 tests |
| list_backups | `crates/codex-files/src/list.rs` | 4 tests |
| prune_backups | `crates/codex-files/src/prune.rs` | 3 tests |
| detect_auth_owner | `crates/codex-files/src/detect.rs` | 4 tests |

### Phase 3: codex-switch (P0)

| 任务 | 文件 | 测试 |
|------|------|------|
| CodexSwitcher 结构 | `crates/codex-switch/src/switcher.rs` | 6 tests |
| apply_codex | `crates/codex-switch/src/apply.rs` | 5 tests |
| restore_codex | `crates/codex-switch/src/restore.rs` | 4 tests |
| list_backup_pairs | `crates/codex-switch/src/pairs.rs` | 4 tests |
| delete_backup_pair | `crates/codex-switch/src/delete.rs` | 3 tests |

### Phase 4: codex-override (P0)

| 任务 | 文件 | 测试 |
|------|------|------|
| OverrideManager 结构 | `crates/codex-override/src/manager.rs` | 5 tests |
| get/set/clear override | `crates/codex-override/src/ops.rs` | 6 tests |

### Phase 5: codex-admin (P1)

| 任务 | 文件 | 测试 |
|------|------|------|
| Admin handlers | `crates/codex-admin/src/handlers.rs` | 10 tests |
| Admin router | `crates/codex-admin/src/router.rs` | 5 tests |
| Admin HTML page | `crates/codex-admin/src/page.rs` | 3 tests |

### Phase 6: 主应用集成 (P1)

| 任务 | 文件 | 测试 |
|------|------|------|
| 更新 lib.rs | `src/lib.rs` | - |
| 更新 handlers/admin | `src/handlers/admin/mod.rs` | 5 tests |
| 更新 router | `src/server/router.rs` | 3 tests |

---

## 五、测试覆盖目标

| Crate | 测试数 | 覆盖率 |
|-------|--------|--------|
| codex-core | 8 | 100% |
| codex-files | 16 | 95% |
| codex-switch | 22 | 90% |
| codex-override | 11 | 95% |
| codex-admin | 18 | 85% |
| **总计** | **75+** | **>90%** |

---

## 六、文件结构

```
crates/
├── codex-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types.rs
│       └── host.rs
├── codex-files/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── atomic.rs
│       ├── backup.rs
│       ├── list.rs
│       ├── prune.rs
│       └── detect.rs
├── codex-switch/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── switcher.rs
│       ├── apply.rs
│       ├── restore.rs
│       ├── pairs.rs
│       └── delete.rs
├── codex-override/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── manager.rs
│       └── ops.rs
└── codex-admin/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── handlers.rs
        ├── router.rs
        └── page.rs

src/
├── codex/           # 保留现有代码 (状态管理)
├── handlers/admin/   # 扩展以使用 codex-admin
└── server/           # 扩展 router

tests/
├── codex_switch_tests.rs
└── admin_tests.rs
```

---

## 七、优先级与时间估算

| Phase | 任务 | 优先级 | 工时 |
|-------|------|--------|------|
| 1 | codex-core 类型定义 | P0 | 1h |
| 2 | codex-files 核心实现 | P0 | 4h |
| 3 | codex-switch 切换逻辑 | P0 | 4h |
| 4 | codex-override 运行时覆盖 | P0 | 2h |
| 5 | codex-admin Admin API | P1 | 3h |
| 6 | 主应用集成 | P1 | 2h |
| 7 | 测试覆盖 | P2 | 3h |

**总计**: ~19h

---

## 八、迁移策略

### 8.1 保持向后兼容

现有 `src/setup/mod.rs` 中的 `build_cc_switch_files` 等函数继续工作:

```rust
// src/setup/mod.rs (保持不变)
pub fn build_cc_switch_files(host: &HostConfig, target: &ProviderTarget) -> CcSwitchFiles {
    let target = SwitchTarget {
        provider_id: target.provider_id(),
        model_id: target.model().to_string(),
        base_url: host.url(),
        auth_sentinel: "rcodex-local".to_string(),
    };
    // 委托给 codex-switch
    crate::codex_switch::build_cc_switch_files(&target, host)
}
```

### 8.2 逐步迁移

1. **Phase 1-4**: 独立开发 crates，不影响现有代码
2. **Phase 5**: 添加 Admin API，同时保留现有 handlers
3. **Phase 6**: 切换到新实现，删除旧代码

---

## 九、风险与缓解

| 风险 | 缓解 |
|------|------|
| 多 crates 依赖循环 | codex-core 无依赖，其他依赖它 |
| 原子写入失败 | 使用 temp 文件 + rename，失败时清理 |
| Windows 路径 | 使用 `Path` 抽象，测试覆盖 |
| 并发备份 | ts.pid 确保唯一性 |

---

## 十、成功标准

1. ✅ mimo2codex Codex Enable 功能完整对标
2. ✅ 所有 75+ 测试通过
3. ✅ Admin UI 可一键切换 provider/model
4. ✅ 备份/恢复/删除功能完整
5. ✅ 运行时覆盖支持 (无需重启 Codex)
6. ✅ `.preserve` 保护外部原始配置
7. ✅ 无编译警告
