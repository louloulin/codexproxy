# plan13.md - rcodex-admin UI 细节完善

> **目标**: 完成 UI 细节，追平 mimo2codex
> **基于**: plan11.md (100%) + plan12.md (97%)
> **创建日期**: 2026-05-26
> **更新日期**: 2026-05-26 18:40

---

## 一、UI 细节分析

### 1.1 mimo2codex UI 结构

```
CodexEnable
├── Title + Intro
├── Alert (modesInfo)
├── Error/Success Alerts
├── CurrentStateCard
│   ├── CodexDirRow
│   ├── Auth status (ownerTag)
│   ├── ConfigToml status
│   ├── Export/Import buttons
│   └── PageTour (FloatButton)
├── Collapse (Prereq/SetupSnippets)
└── Tabs
    ├── targets tab
    │   ├── Test All Button
    │   ├── External Warning
    │   └── ProviderBlock[]
    ├── thinking tab
    │   ├── Thinking Switch
    │   └── ForceHighEffort Switch
    ├── backsups tab
    │   └── BackupCard
    ├── history tab
    │   └── HistoryPanel
    └── override tab
        └── RuntimeOverrideCard
```

### 1.2 rcodex-admin UI 结构

```
CodexPage
├── Header (title, language, import/export buttons)
├── Tabs
│   ├── config tab
│   │   ├── CodexStateCard
│   │   └── ProviderSelector
│   ├── setup tab
│   │   └── SetupSnippets
│   ├── thinking tab
│   │   └── ThinkingPanel
│   ├── backups tab
│   │   └── BackupList
│   ├── history tab
│   │   └── HistoryPanel
│   └── override tab
│       └── OverridePanel
└── ImportModal / ExportModal
```

---

## 二、UI 差距详细对比

### 2.1 CodexEnable 布局

| 元素 | mimo2codex | rcodex-admin | 差距 |
|------|------------|--------------|------|
| Title | `<Typography.Title>` | `<h1>` | ✅ 完成 |
| Intro | `<Typography.Paragraph>` | subtitle p | ✅ 完成 |
| Alert (modesInfo) | ✅ Info Alert | ❌ 缺失 | **需添加** |
| Error/Success Alerts | ✅ 独立 Alert | ✅ 内嵌 | ✅ 完成 |
| CurrentStateCard | ✅ Descriptions | ✅ Card | ✅ 完成 |
| Collapse (Prereq) | ✅ Collapse | ❌ 无 | **需添加** |
| Tabs | ✅ Ant Tabs | ✅ Radix Tabs | ✅ 完成 |

### 2.2 CurrentStateCard 组件

| 元素 | mimo2codex | rcodex-admin | 差距 |
|------|------------|--------------|------|
| CodexDirRow | ✅ 独立组件 | ✅ 内嵌 | ✅ 完成 |
| Auth Owner | ✅ Tag + path | ✅ Badge + path | ✅ 完成 |
| ConfigToml | ✅ Tags (provider/model) | ✅ Badge (provider) | ⚠️ 可改进 |
| Export/Import | ✅ 根据 authMode | ✅ 始终显示 | ⚠️ 可改进 |
| PageTour | ✅ FloatButton | ❌ 无 | **需实现** |

### 2.3 ProviderBlock Table

| 元素 | mimo2codex | rcodex-admin | 差距 |
|------|------------|--------------|------|
| Test All Button | ✅ Tabs 外部 | ✅ ProviderSelector 内 | ✅ 完成 |
| External Warning | ✅ Alert | ❌ 无 | **需添加** |
| Table 列 | Model/Source/Context/Ops | Model/Source/Context/Status/Actions | ✅ 完成 |
| Override Button | ✅ 每行 | ✅ 每行 | ✅ 完成 |

### 2.4 Thinking Panel

| 元素 | mimo2codex | rcodex-admin | 差距 |
|------|------------|--------------|------|
| Thinking Switch | ✅ Switch | ✅ Switch | ✅ 完成 |
| Status Text | ✅ "On/Off" | ❌ 无 | **需添加** |
| ForceHighEffort | ✅ Switch | ✅ Switch | ✅ 完成 |
| CLI Override | ✅ Alert + disabled | ✅ Alert + disabled | ✅ 完成 |
| Saving Loading | ✅ loading prop | ⚠️ isPending | ⚠️ 可改进 |

---

## 三、UI 细节 TODO

### 🔲 3.1 Alert (modesInfo)

**mimo2codex**:
```tsx
<Alert
  type="info"
  description={
    <Space direction="vertical" size={6}>
      <Trans i18nKey="modesInfo.applyFile" />
      <Trans i18nKey="modesInfo.runtimeOverride" />
    </Space>
  }
/>
```

**rcodex-admin**: 缺失

**实现**: 在 CodexPage 顶部添加介绍 Alert

---

### 🔲 3.2 Collapse (Prereq/SetupSnippets)

**mimo2codex**:
```tsx
<Collapse
  items={[{
    key: "prereq",
    label: (
      <span>
        <strong>{t("prereq.title")}</strong>
        <Typography.Text type="secondary">{t("prereq.subtitle")}</Typography.Text>
      </span>
    ),
    children: <SetupSnippets />,
  }]}
/>
```

**rcodex-admin**: 在 setup tab 中显示

**评估**: 功能等价，可保持现状

---

### 🔲 3.3 Thinking Status Text

**mimo2codex**:
```tsx
<span>
  {thinkingDisabled ? t("thinking.statusOff") : t("thinking.statusOn")}
</span>
```

**rcodex-admin**: 缺失状态文字

**实现**: 在 ThinkingPanel 添加状态文字

---

### 🔲 3.4 PageTour FloatButton

**mimo2codex**:
```tsx
<FloatButton
  icon={<QuestionCircleOutlined />}
  tooltip={t("openHelper")}
  style={{ right: 24, top: 60 }}
  onClick={() => setOpen(true)}
/>
<Tour
  open={open}
  steps={toTourSteps(steps)}
/>
```

**rcodex-admin**: 缺失

**实现**:
1. 创建 PageTour 组件
2. 添加 FloatButton
3. 定义 Tour steps
4. 使用 localStorage 记住完成状态

---

## 四、i18n 翻译对比

### 4.1 mimo2codex 翻译命名

```json
{
  "codexEnable": {
    "title": "Codex Enable",
    "intro": "...",
    "modesInfo": {
      "applyFile": "Apply changes to config.toml",
      "runtimeOverride": "Runtime override"
    },
    "prereq": {
      "title": "Prerequisites",
      "subtitle": "..."
    },
    "state": {
      "codexDir": "Codex Directory",
      "authJson": "Auth",
      "configToml": "Config"
    }
  },
  "tour": {
    "openHelper": "Help",
    "steps": {...}
  }
}
```

### 4.2 rcodex-admin 翻译命名

```json
{
  "app": {
    "title": "Codex Admin",
    "subtitle": "..."
  },
  "codexState": {...},
  "provider": {...},
  "backup": {...}
}
```

**差距**:
- mimo2codex 使用 `codexEnable` 命名空间
- rcodex-admin 使用分散的命名空间

**评估**: 功能等价，可保持现状

---

## 五、实现优先级

### P1 - 高价值 UI 细节

| 功能 | 工时 | 价值 | 状态 |
|------|------|------|------|
| PageTour FloatButton | 2h | 高 | 🔲 |
| modesInfo Alert | 0.5h | 中 | 🔲 |
| Thinking Status Text | 0.25h | 低 | 🔲 |

### P2 - 可改进项

| 功能 | 工时 | 价值 | 状态 |
|------|------|------|------|
| Collapse Prereq | 1h | 中 | 🔲 |
| External Warning | 0.25h | 低 | 🔲 |

---

## 六、进度追踪

### 6.1 当前进度

| 类别 | 总计 | 已完成 | 进度 |
|------|------|--------|------|
| UI 细节 | 10 | 7 | **70%** |

### 6.2 更新日志

| 日期 | 版本 | 更新内容 |
|------|------|----------|
| 2026-05-26 18:40 | 3.0 | UI 细节分析版本 |

---

## 七、验证清单

```bash
# PageTour
- [ ] FloatButton 显示在右上角
- [ ] 点击打开 Tour
- [ ] 可以跳过
- [ ] 完成状态保存到 localStorage

# Alert
- [ ] 顶部显示 modesInfo
- [ ] 国际化翻译正确

# Thinking Status
- [ ] 显示 "On/Off" 状态文字
```

---

**文档版本**: 3.0
**更新日期**: 2026-05-26 18:40
**状态**: UI 细节分析完成
**预计工时**: 3-4h

---

## 八、完整功能清单

### 8.1 plan11 + plan12 + plan13

| 阶段 | 功能数 | 已完成 | 进度 |
|------|--------|--------|------|
| plan11 核心 | 14 | 14 | 100% |
| plan11 P1-P3 | 10 | 10 | 100% |
| plan12 API/UI | 5 | 4 | 80% |
| plan13 UI 细节 | 5 | 2 | 40% |
| **总计** | **34** | **30** | **88%** |

### 8.2 剩余功能

| 功能 | 优先级 | 工时 |
|------|--------|------|
| PageTour | P1 | 2h |
| modesInfo Alert | P1 | 0.5h |
| Thinking Status | P1 | 0.25h |
| Collapse Prereq | P2 | 1h |
| External Warning | P2 | 0.25h |
| **总计** | | **4h** |
