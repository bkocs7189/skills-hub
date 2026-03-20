# 需求：设置弹窗改为独立页面

## Context

设置功能原来是一个 560px 宽的模态弹窗（SettingsModal），在小窗口下容易被遮挡，需要最大化窗口才能完整查看。将其改为 `activeView` 视图系统中的独立页面，与 myskills/explore/detail 并列，UX 更自然。

## 实施方案

### 1. 组件迁移：`modals/SettingsModal.tsx` → `SettingsPage.tsx`

- 移动到 `src/components/skills/SettingsPage.tsx`，与 `ExplorePage`、`SkillDetailView` 同级
- 类型名 `SettingsModalProps` → `SettingsPageProps`
  - 删除 `open: boolean`
  - `onRequestClose` → `onBack`
- 去掉 modal 外壳（`modal-backdrop` / `modal` / `modal-header` / `modal-footer`）
- 复用 `detail-header` + `detail-back-btn` 样式，与 SkillDetailView 保持一致
- useEffect 从依赖 `open` 改为挂载/卸载模式
- 删除 `if (!open) return null` 守卫
- 删除底部 "Done" 按钮

### 2. `App.tsx` — 状态和路由

- `activeView` 类型扩展为 `'myskills' | 'explore' | 'detail' | 'settings'`
- 删除 `showSettingsModal` 布尔状态
- `handleOpenSettings` 改为 `setActiveView('settings')`
- `handleCloseSettings` 改为 `setActiveView('myskills')`
- `<main>` 条件渲染新增 `settings` 分支，渲染 `<SettingsPage>`
- 删除底部的 `<SettingsModal>` 渲染块

### 3. `Header.tsx` — 导航状态

- `activeView` 类型加 `| 'settings'`
- 设置齿轮按钮在 `activeView === 'settings'` 时添加 `active` 样式

### 4. `App.css` — 样式调整

- 删除 `.settings-modal` 和 `.settings-body` 规则
- 新增 `.settings-page`（居中布局）和 `.settings-page-body`（560px 最大宽度，居中）
- 复用 `.detail-header` / `.detail-back-btn` / `.detail-skill-name` 样式
- 所有 `.settings-field` 等子样式保持不变

### 不需要改动

- **i18n**：复用已有的 `detail.back`、`settings` key
- **Rust 后端**：纯前端改动
- **types.ts**：无 DTO 变更
