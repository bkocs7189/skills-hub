# Bugfix：切换语言时闪现 "Installing Skills..." 弹窗

## 问题描述

在 Explore 页面点击语言切换按钮（EN/中）时，会短暂弹出 "Installing Skills..." 加载遮罩，一两秒后自动消失。

## 根因分析

`invokeTauri` 函数的 `useCallback` 依赖数组包含了 `t`（i18next 翻译函数）：

```ts
const invokeTauri = useCallback(async (...) => {
  if (!isTauri) throw new Error(t('errors.notTauri'))
  ...
}, [isTauri, t])  // ← t 导致级联重建
```

切换语言时的级联链：

1. `i18n.changeLanguage()` → `t` 引用变化
2. `t` 变化 → `invokeTauri` 重新创建（依赖 `t`）
3. `invokeTauri` 变化 → `loadPlan` 重新创建（依赖 `invokeTauri`）
4. `loadPlan` 变化 → `useEffect([isTauri, loadPlan])` 重新触发
5. `loadPlan()` 执行 → `setLoading(true)` → 显示加载弹窗
6. 请求完成 → `setLoading(false)` → 弹窗消失

## 修复方案

将 `invokeTauri` 中的 `t('errors.notTauri')` 替换为硬编码字符串，从依赖数组移除 `t`：

```ts
const invokeTauri = useCallback(async (...) => {
  if (!isTauri) throw new Error('Tauri API is not available')
  ...
}, [isTauri])  // ← 不再依赖 t
```

该错误消息仅在非 Tauri 环境下触发（实际不可能发生），无需翻译。

## 修改文件

- `src/App.tsx`：`invokeTauri` useCallback 依赖数组移除 `t`
