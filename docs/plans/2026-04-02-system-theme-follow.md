# System Theme Follow Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让应用只跟随操作系统明暗主题，并修复深色模式下关键交互控件的文字可读性问题。

**Architecture:** 保留现有浅色/深色 design tokens，但移除用户手动主题切换入口，运行时统一由系统 `prefers-color-scheme` 驱动根节点的 `dark` class。样式修复优先复用现有 token 与 `jelly-*` 类，对写死前景色/背景色的交互控件做定点收敛而非整站 UI 重构。

**Tech Stack:**
- Frontend: React 19 + TypeScript + Zustand
- Styling: Tailwind CSS v4 + `src/styles/globals.css`
- Tests: Vitest + Testing Library

---

### Task 1: 锁定主题逻辑与失败测试

**Files:**
- Modify: `src/hooks/useTheme.ts`
- Test: `src/stores/__tests__/appStore.test.ts`
- Test: `src/components/Settings/__tests__/Settings.test.tsx`

**Step 1: 写失败测试，表达“运行时只跟随系统主题”**

在 `src/components/Settings/__tests__/Settings.test.tsx` 增加一个最小测试，断言设置页面不再出现主题切换入口或与主题切换相关的可交互控件。

示例：
```tsx
it('does not render a manual theme selector', () => {
  renderSettings()
  expect(screen.queryByText(/theme/i)).toBeNull()
})
```

**Step 2: 运行测试，确认先失败**

Run: `npm test -- src/components/Settings/__tests__/Settings.test.tsx`

Expected: FAIL，因为当前设置页仍可能渲染主题相关内容或测试基线尚未收敛。

**Step 3: 为 store 默认行为补一个最小失败测试**

在 `src/stores/__tests__/appStore.test.ts` 增加断言，强调默认配置依旧兼容 `theme` 字段，但应用实际行为将不再依赖 `light` / `dark` 的手动切换。

示例：
```ts
it('keeps theme config for compatibility', () => {
  expect(getState().config.theme).toBe('system')
})
```

**Step 4: 运行 store 测试**

Run: `npm test -- src/stores/__tests__/appStore.test.ts`

Expected: PASS 或按需调整；如果已有错误，先确保测试表达的是兼容性而不是旧 UI 行为。

---

### Task 2: 收敛运行时主题来源到系统主题

**Files:**
- Modify: `src/hooks/useTheme.ts`

**Step 1: 写最小实现**

将 `useTheme()` 改为仅监听系统 `prefers-color-scheme`，忽略 `config.theme` 的 `light` / `dark` 人工分支。

目标代码形态：
```ts
useEffect(() => {
  const root = document.documentElement
  const mq = window.matchMedia('(prefers-color-scheme: dark)')

  const applyTheme = (isDark: boolean) => {
    root.classList.toggle('dark', isDark)
  }

  applyTheme(mq.matches)

  const handler = (e: MediaQueryListEvent) => applyTheme(e.matches)
  mq.addEventListener('change', handler)
  return () => mq.removeEventListener('change', handler)
}, [])
```

**Step 2: 运行受影响测试**

Run: `npm test -- src/stores/__tests__/appStore.test.ts src/components/Settings/__tests__/Settings.test.tsx`

Expected: 主题运行时逻辑相关测试通过，若出现旧行为断言，改成兼容当前设计。

**Step 3: 检查是否需要保留 `Theme` 类型**

如果 `Theme` 类型与 `config.theme` 字段仍被存储层或其他组件引用，则保留，但不要再让运行时逻辑依赖它。

---

### Task 3: 删除设置页中的手动主题入口

**Files:**
- Modify: `src/components/Settings/GeneralPane.tsx`
- Modify: `src/i18n/locales/zh.json`
- Modify: `src/i18n/locales/en.json`

**Step 1: 写失败测试，确认主题入口已被移除**

在 `src/components/Settings/__tests__/Settings.test.tsx` 中为设置页增加断言，确保手动主题相关文案、下拉框或 section 不再存在。

示例：
```tsx
it('hides theme controls in settings', () => {
  renderSettings()
  expect(screen.queryByText('settings.theme')).toBeNull()
})
```

**Step 2: 运行测试，确认失败**

Run: `npm test -- src/components/Settings/__tests__/Settings.test.tsx`

Expected: FAIL，如果页面仍渲染主题入口。

**Step 3: 修改 `GeneralPane.tsx`**

- 删除主题选择 UI
- 删除与主题 UI 直接相关的说明文案引用
- 保留其他设置项不受影响

**Step 4: 更新 i18n**

如果删除后有翻译键完全失效，可先保留现有键值不动，避免扩大范围；只在确有必要时清理明显无用的主题文案。

**Step 5: 再跑设置页测试**

Run: `npm test -- src/components/Settings/__tests__/Settings.test.tsx`

Expected: PASS

---

### Task 4: 先修 CSS token 与通用按钮类

**Files:**
- Modify: `src/styles/globals.css`

**Step 1: 写失败测试或确定最小可验证目标**

这里更适合通过现有组件测试与类型检查验证，而不是为纯 CSS 写低价值测试。先明确目标：

- `jelly-btn-accent` 在浅色和深色下都能提供足够前景/背景对比
- `jelly-btn` 的 hover / active 不依赖硬编码浅色文本
- 透明按钮 hover 态的前景色能在深色背景上清晰可见

**Step 2: 修改 `globals.css`**

优先做这些收敛：
- 让主 CTA 类显式承担前景/背景对比责任
- 必要时补充适用于深色主题的次按钮或 ghost 按钮样式类
- 避免 disabled 态仅依赖过低透明度

**Step 3: 运行相关测试**

Run: `npm test -- src/components/History/__tests__/History.test.tsx src/components/Settings/__tests__/Settings.test.tsx`

Expected: PASS，且不会因为类名收敛打破现有按钮查询行为。

---

### Task 5: 修复 Onboarding 的按钮和导航可读性

**Files:**
- Modify: `src/components/Onboarding/OnboardingLayout.tsx`
- Modify: `src/components/Onboarding/SttSetupStep.tsx`
- Modify: `src/components/Onboarding/LlmSetupStep.tsx`
- Modify: `src/components/Onboarding/AccountStep.tsx`
- Modify: `src/components/Onboarding/DoneStep.tsx`

**Step 1: 搜索并锁定写死颜色**

重点查找：
```bash
rg "text-white|bg-accent|hover:bg-accent-hover" src/components/Onboarding
```

**Step 2: 逐个收敛**

- 主按钮尽量改用 `jelly-btn-accent`
- 次按钮或返回按钮使用 token 化文字颜色
- 输入框与状态提示遵循 `text-text-*` / `bg-bg-*` / `border-border`

**Step 3: 运行定向测试**

如无现成 Onboarding 测试，可先运行设置和历史测试，随后执行一次 TypeScript 检查避免 JSX/className 回归。

Run: `npx tsc --noEmit`

Expected: PASS

---

### Task 6: 修复 Settings 相关按钮、输入框和操作栏

**Files:**
- Modify: `src/components/Settings/GeneralPane.tsx`
- Modify: `src/components/Settings/SttPane.tsx`
- Modify: `src/components/Settings/LlmPane.tsx`
- Modify: `src/components/Settings/DictionaryPane.tsx`
- Modify: `src/components/Settings/ScenesPane.tsx`
- Modify: `src/components/Settings/shared/DirtyBar.tsx`
- Modify: `src/components/Settings/shared/Toggle.tsx`

**Step 1: 写失败测试，覆盖一个明显的深色可读性场景**

在 `Settings.test.tsx` 中增加一个轻量断言，确认主操作按钮不再依赖固定 `text-white` 类名，或者确认某个按钮具备统一类名如 `jelly-btn-accent`。

示例：
```tsx
it('uses shared accent button styling for primary actions', () => {
  renderSettings()
  const save = screen.queryByText('Save')
  expect(save?.className).toContain('jelly-btn-accent')
})
```

**Step 2: 跑测试，确认先失败**

Run: `npm test -- src/components/Settings/__tests__/Settings.test.tsx`

Expected: FAIL

**Step 3: 修改相关组件**

- 将主按钮从 `bg-accent text-white` 收敛到统一主按钮样式
- 检查输入框、下拉框、热键按钮、权限按钮、保存栏按钮
- 让 toggle 滑块与轨道在深色背景上仍清晰

**Step 4: 再跑设置页测试**

Run: `npm test -- src/components/Settings/__tests__/Settings.test.tsx`

Expected: PASS

---

### Task 7: 修复 History 与弹窗类交互

**Files:**
- Modify: `src/components/History/index.tsx`
- Test: `src/components/History/__tests__/History.test.tsx`
- Modify: `src/components/CopyDialog/index.tsx`

**Step 1: 写失败测试，表达清空按钮的共享样式预期**

在 `History.test.tsx` 增加断言，确认清空按钮或复制按钮使用主题化样式类，而不是依赖不可控的默认颜色。

示例：
```tsx
it('renders clear button with shared button styling', async () => {
  render(<History />)
  expect(screen.getByRole('button', { name: /clear all history/i }).className).toContain('jelly-btn')
})
```

**Step 2: 运行测试，确认先失败**

Run: `npm test -- src/components/History/__tests__/History.test.tsx`

Expected: FAIL

**Step 3: 修改 `History` 与 `CopyDialog`**

- 搜索框保持 token 化输入样式
- 清空按钮使用共享按钮样式
- 复制按钮和关闭按钮在深色背景上前景清晰
- `CopyDialog` 中按钮与文字避免固定浅色写法只适配一套背景

**Step 4: 重新运行历史测试**

Run: `npm test -- src/components/History/__tests__/History.test.tsx`

Expected: PASS

---

### Task 8: 修复其他页面上的主次按钮

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/AccountPage/index.tsx`
- Modify: `src/components/UpgradePage/index.tsx`

**Step 1: 搜索剩余硬编码颜色**

Run:
```bash
rg "text-white|bg-accent|bg-white|text-black" src/App.tsx src/components/AccountPage src/components/UpgradePage
```

**Step 2: 逐个替换为共享样式**

- 主 CTA -> `jelly-btn-accent`
- 次要按钮 -> token 化边框/文本/背景
- 保留 disabled 态，但保证至少能看清按钮文本

**Step 3: 运行 TypeScript 检查**

Run: `npx tsc --noEmit`

Expected: PASS

---

### Task 9: 收尾验证

**Files:**
- Read: `src/hooks/useTheme.ts`
- Read: `src/styles/globals.css`
- Read: `src/components/**`

**Step 1: 运行定向测试**

Run:
```bash
npm test -- src/components/Settings/__tests__/Settings.test.tsx src/components/History/__tests__/History.test.tsx src/stores/__tests__/appStore.test.ts
```

Expected: PASS

**Step 2: 运行 TypeScript 检查**

Run: `npx tsc --noEmit`

Expected: PASS

**Step 3: 运行最近修改文件的 lint 检查**

Run: `npx eslint src/hooks/useTheme.ts src/components/Settings src/components/History src/components/Onboarding src/components/AccountPage src/components/UpgradePage`

Expected: PASS

**Step 4: 人工验收**

- [ ] macOS 系统切到浅色，应用同步浅色
- [ ] macOS 系统切到深色，应用同步深色
- [ ] Windows 系统浅色 / 深色均能同步
- [ ] 深色下 Onboarding / Settings / History / Account / Upgrade 的主按钮、次按钮、输入框文案可读
- [ ] 删除主题设置入口后，页面结构没有明显断裂

---

## 备注

- 本计划刻意避免一次性删除 `config.theme` 字段，先保证兼容与行为收敛
- 若执行过程中发现还有大量零散按钮未纳入范围，再单独补一轮清点，不在当前步骤内扩展为完整组件体系重构
