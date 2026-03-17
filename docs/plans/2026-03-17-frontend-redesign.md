# Frontend Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 建立前端重设计基线，并以共享视觉系统驱动后续 admin / tenant 页面 rollout。

**Architecture:** 先抽纯配置的设计语言与动效 preset，再把它们落到全局 token、基础控件、导航壳层和 archetype primitive 上，最后用样板页验证新语言是否真正改善质感与效率。每一批都以最小可验证改动推进，避免一次性大换血。

**Tech Stack:** React 19、TypeScript、Tailwind v4、Framer Motion、shadcn/ui、Node `--test`、Vite

---

### Task 1: 定义设计语言配置与纯函数测试

**Files:**
- Create: `frontend/src/lib/design-system.ts`
- Test: `frontend/src/lib/design-system.test.ts`

**Step 1: Write the failing test**

在 `frontend/src/lib/design-system.test.ts` 中为以下行为写测试：

- `resolveDesignLanguage()` 返回浅色/深色共用的中性色、强调色与 surface token
- `resolveDesignLanguage()` 返回统一的圆角、阴影和密度配置
- 未知 mode 不抛错，并返回安全的默认设计语言

**Step 2: Run test to verify it fails**

Run:
```bash
shnote --what "运行设计语言配置测试" --why "先固定重设计基线的纯配置契约" run "cd frontend && node --test src/lib/design-system.test.ts"
```

Expected: FAIL，因为 `design-system.ts` 尚不存在。

**Step 3: Write minimal implementation**

在 `frontend/src/lib/design-system.ts` 中实现最小配置层：

- `type DesignMode = 'light' | 'dark'`
- `resolveDesignLanguage(mode)` 返回颜色、材质、圆角、阴影、排版节奏与控件密度
- `resolveSurfaceRecipe(kind)` 返回 `panel / panel-muted / stage / sidebar` 的共享表面配置

**Step 4: Run test to verify it passes**

Run:
```bash
shnote --what "验证设计语言配置" --why "确认前端重设计基线配置通过测试" run "cd frontend && node --test src/lib/design-system.test.ts"
```

Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/lib/design-system.ts frontend/src/lib/design-system.test.ts
git commit -m "feat(frontend): add redesign design language config" -m "Define reusable visual tokens and surface recipes for the frontend redesign."
```

### Task 2: 将设计语言落到全局 token 与基础材质

**Files:**
- Modify: `frontend/src/index.css`
- Reference: `frontend/src/lib/design-system.ts`
- Test: `frontend/src/lib/design-system.test.ts`

**Step 1: Extend the failing test**

在 `frontend/src/lib/design-system.test.ts` 中增加断言：

- `stage` 表面比 `panel` 更有材质层次
- `panel-muted` 的边界与阴影弱于 `panel`
- `sidebar` 具备独立于主面板的色温和背景规则

**Step 2: Run test to verify it fails**

Run:
```bash
shnote --what "运行设计材质扩展测试" --why "先让全局表面规则以失败形式固定下来" run "cd frontend && node --test src/lib/design-system.test.ts"
```

Expected: FAIL，新增断言尚未满足。

**Step 3: Write minimal implementation**

在 `frontend/src/index.css` 中：

- 用新的中性色与强调色替换当前更通用的默认 token
- 收敛圆角尺度与阴影层级
- 重写 `page-stage-surface / page-panel-surface / page-panel-surface-muted`
- 为导航和工作区增加更克制的背景与材质层

**Step 4: Run checks**

Run:
```bash
shnote --what "验证全局视觉 token" --why "确认新的材质与全局 token 不破坏前端构建" run "cd frontend && node --test src/lib/design-system.test.ts && npm run lint && npm run build"
```

Expected: all PASS

**Step 5: Commit**

```bash
git add frontend/src/index.css frontend/src/lib/design-system.ts frontend/src/lib/design-system.test.ts
git commit -m "feat(frontend): refresh redesign global tokens" -m "Apply the new visual baseline to global theme and shared surface materials."
```

### Task 3: 升级核心控件的视觉基线

**Files:**
- Modify: `frontend/src/components/ui/button.tsx`
- Modify: `frontend/src/components/ui/input.tsx`
- Modify: `frontend/src/components/ui/textarea.tsx`
- Modify: `frontend/src/components/ui/select.tsx`
- Modify: `frontend/src/components/ui/card.tsx`
- Modify: `frontend/src/components/ui/standard-data-table.tsx`

**Step 1: Write the failing test**

在 `frontend/src/lib/design-system.test.ts` 中补充对控件语义映射的纯函数断言：

- `default` button 应映射到强调色但不使用饱和默认蓝
- `outline` / `ghost` button 应体现层次差异
- `table` 应返回独立的 header / toolbar / row surface recipe

**Step 2: Run test to verify it fails**

Run:
```bash
shnote --what "运行控件视觉映射测试" --why "先固定按钮和表格的新控件语言" run "cd frontend && node --test src/lib/design-system.test.ts"
```

Expected: FAIL

**Step 3: Write minimal implementation**

按最小范围改造：

- `button`：更明确的主次层级、hover/focus/disabled 细节
- `input / textarea / select`：统一边界、背景、焦点 ring 与占位文字层级
- `card`：与新的面板材质对齐
- `standard-data-table`：让 toolbar、header、row hover 与空状态进入同一语言

**Step 4: Run checks**

Run:
```bash
shnote --what "验证核心控件基线" --why "确认基础控件升级后 lint 和 build 仍然通过" run "cd frontend && node --test src/lib/design-system.test.ts && npm run lint && npm run build"
```

Expected: all PASS

**Step 5: Commit**

```bash
git add frontend/src/components/ui/button.tsx frontend/src/components/ui/input.tsx frontend/src/components/ui/textarea.tsx frontend/src/components/ui/select.tsx frontend/src/components/ui/card.tsx frontend/src/components/ui/standard-data-table.tsx frontend/src/lib/design-system.ts frontend/src/lib/design-system.test.ts
git commit -m "feat(frontend): redesign core control surfaces" -m "Refresh buttons, fields, cards, and data tables with the new visual baseline."
```

### Task 4: 抽取统一 microinteraction preset

**Files:**
- Create: `frontend/src/lib/motion-presets.ts`
- Test: `frontend/src/lib/motion-presets.test.ts`
- Modify: `frontend/src/components/ui/loading-overlay.tsx`
- Modify: `frontend/src/components/auth/auth-shell.tsx`
- Modify: `frontend/src/components/layout/AppLayout.tsx`

**Step 1: Write the failing test**

在 `frontend/src/lib/motion-presets.test.ts` 中写测试：

- `pageEnter` 返回短促、非 bounce 的进入动效
- `panelReveal` 返回适合列表与面板的 reveal 动效
- reduced motion 下返回静态或弱化版本

**Step 2: Run test to verify it fails**

Run:
```bash
shnote --what "运行动效 preset 测试" --why "先固定统一微交互的纯函数契约" run "cd frontend && node --test src/lib/motion-presets.test.ts"
```

Expected: FAIL，因为 `motion-presets.ts` 尚不存在。

**Step 3: Write minimal implementation**

在 `frontend/src/lib/motion-presets.ts` 中新增：

- `resolvePageEnterMotion`
- `resolvePanelRevealMotion`
- `resolveFeedbackMotion`

然后把这些 preset 接入：

- `loading-overlay`
- `auth-shell`
- `AppLayout`

要求：

- 不新增高噪声装饰动效
- reduced motion 路径清晰

**Step 4: Run checks**

Run:
```bash
shnote --what "验证统一微交互预设" --why "确认动效预设和接入不破坏前端构建" run "cd frontend && node --test src/lib/motion-presets.test.ts && npm run lint && npm run build"
```

Expected: all PASS

**Step 5: Commit**

```bash
git add frontend/src/lib/motion-presets.ts frontend/src/lib/motion-presets.test.ts frontend/src/components/ui/loading-overlay.tsx frontend/src/components/auth/auth-shell.tsx frontend/src/components/layout/AppLayout.tsx
git commit -m "feat(frontend): add redesign motion presets" -m "Unify page, panel, and feedback microinteractions across shared frontend shells."
```

### Task 5: 重做导航壳层与共享页面表面

**Files:**
- Modify: `frontend/src/components/layout/AppLayout.tsx`
- Modify: `frontend/src/components/layout/page-archetypes.tsx`
- Modify: `frontend/src/components/ui/parallax-background.tsx`
- Modify: `frontend/src/lib/page-archetypes.ts`
- Test: `frontend/src/lib/page-archetypes.test.ts`

**Step 1: Extend the failing test**

在 `frontend/src/lib/page-archetypes.test.ts` 中新增断言：

- `dashboard` 和 `workspace` 的 header / panel 表面强度不同
- `settings` 保持最安静的 surface tone
- `auth` 与 `workspace` 在 stage emphasis 上仍有清楚差异

**Step 2: Run test to verify it fails**

Run:
```bash
shnote --what "运行页面表面差异测试" --why "先固定不同 archetype 的表面强度分层" run "cd frontend && node --test src/lib/page-archetypes.test.ts"
```

Expected: FAIL

**Step 3: Write minimal implementation**

最小实现包括：

- 导航壳层改为更精致、更克制的材质
- `PageIntro / PagePanel / BrandStage` 对应新 surface recipe
- 背景层弱化为长期高频使用友好的氛围层，而非明显装饰层

**Step 4: Run checks**

Run:
```bash
shnote --what "验证导航壳层与页面表面" --why "确认共享页面表面升级后测试、lint 和 build 仍然通过" run "cd frontend && node --test src/lib/page-archetypes.test.ts && npm run lint && npm run build"
```

Expected: all PASS

**Step 5: Commit**

```bash
git add frontend/src/components/layout/AppLayout.tsx frontend/src/components/layout/page-archetypes.tsx frontend/src/components/ui/parallax-background.tsx frontend/src/lib/page-archetypes.ts frontend/src/lib/page-archetypes.test.ts
git commit -m "feat(frontend): redesign shell surfaces" -m "Refresh the shared navigation shell and page surfaces with the new material system."
```

### Task 6: 用样板页验证重设计基线

**Files:**
- Modify: `frontend/src/pages/Login.tsx`
- Modify: `frontend/src/pages/Dashboard.tsx`
- Modify: `frontend/src/pages/ImportJobs.tsx`
- Modify: `frontend/src/pages/Models.tsx`
- Modify: `frontend/src/pages/Config.tsx`
- Modify: `frontend/src/tenant/pages/DashboardPage.tsx`
- Reference: `frontend/src/components/layout/page-archetypes.tsx`
- Reference: `frontend/src/lib/motion-presets.ts`

**Step 1: Write the failing test**

在 `frontend/src/lib/page-archetypes.test.ts` 中补充样板页约束：

- `auth` 移动端品牌区必须压缩
- `dashboard` intro 必须短于 `auth`
- `workspace` 主任务区必须早于次级摘要
- `settings` 的操作区必须收敛到分段面板流

**Step 2: Run test to verify it fails**

Run:
```bash
shnote --what "运行样板页节奏测试" --why "先固定首批样板页的页面节奏约束" run "cd frontend && node --test src/lib/page-archetypes.test.ts"
```

Expected: FAIL

**Step 3: Write minimal implementation**

在不改业务逻辑前提下：

- `Login`：强化入口质感与表单交互细节
- `Dashboard / tenant Dashboard`：统一 KPI、section、rail 的视觉秩序
- `ImportJobs`：强化主任务区与状态反馈
- `Models`：让控制区、状态区、数据表面更一致
- `Config`：让设置页更安静、更稳定

**Step 4: Run checks and manual verification**

Run:
```bash
shnote --what "验证样板页重设计基线" --why "确认首批页面在新设计语言下通过测试与构建" run "cd frontend && node --test src/lib/page-archetypes.test.ts src/lib/design-system.test.ts src/lib/motion-presets.test.ts src/components/ui/trend-chart-core.test.ts src/features/api-keys/admin-capabilities.test.ts src/lib/edition-shell-routing.test.ts && npm run i18n:check && npm run i18n:hardcode -- --no-baseline && node scripts/i18n/check-missing-runtime-keys.mjs && npm run lint && npm run build"
```

人工检查：

- `http://127.0.0.1:5174/login`
- `http://127.0.0.1:5174/dashboard`
- `http://127.0.0.1:5174/imports`
- `http://127.0.0.1:5174/models`
- `http://127.0.0.1:5174/config`

**Step 5: Commit**

```bash
git add frontend/src/pages/Login.tsx frontend/src/pages/Dashboard.tsx frontend/src/pages/ImportJobs.tsx frontend/src/pages/Models.tsx frontend/src/pages/Config.tsx frontend/src/tenant/pages/DashboardPage.tsx frontend/src/lib/page-archetypes.ts frontend/src/lib/page-archetypes.test.ts frontend/src/lib/design-system.ts frontend/src/lib/design-system.test.ts frontend/src/lib/motion-presets.ts frontend/src/lib/motion-presets.test.ts frontend/src/components/layout/page-archetypes.tsx frontend/src/components/auth/auth-shell.tsx frontend/src/components/layout/AppLayout.tsx frontend/src/index.css
git commit -m "feat(frontend): apply redesign baseline to sample pages" -m "Validate the new frontend visual system across auth, dashboard, workspace, and settings samples."
```

### Task 7: 收口剩余复杂页面并回填文档

**Files:**
- Modify: `frontend/src/pages/System.tsx`
- Modify: `frontend/src/pages/Tenants.tsx`
- Modify: `frontend/src/tenant/pages/ApiKeysPage.tsx`
- Modify: `docs/plans/2026-03-17-frontend-redesign-design.md`
- Modify: `docs/plans/2026-03-17-frontend-redesign.md`
- Modify: `docs/plans/2026-03-17-frontend-page-archetypes.md`

**Step 1: Write the failing test**

在 `frontend/src/lib/page-archetypes.test.ts` 中增加：

- `detail` / `settings` / `workspace` 复杂页仍必须保持主次秩序
- 复杂管理页不能回退到 marketing-style hero

**Step 2: Run test to verify it fails**

Run:
```bash
shnote --what "运行复杂页收口测试" --why "先固定剩余复杂页面不能回退到旧表达" run "cd frontend && node --test src/lib/page-archetypes.test.ts"
```

Expected: FAIL

**Step 3: Write minimal implementation**

把剩余三类旧风格重页面收进统一设计语言：

- `System`
- `Tenants`
- tenant `ApiKeys`

并同步回填设计文档与实施计划中的完成状态。

**Step 4: Run checks**

Run:
```bash
shnote --what "验证复杂页收口" --why "确认剩余复杂页面完成重设计后前端基线仍然稳定" run "cd frontend && node --test src/lib/page-archetypes.test.ts src/lib/design-system.test.ts src/lib/motion-presets.test.ts src/components/ui/trend-chart-core.test.ts src/features/api-keys/admin-capabilities.test.ts src/lib/edition-shell-routing.test.ts && npm run i18n:check && npm run i18n:hardcode -- --no-baseline && node scripts/i18n/check-missing-runtime-keys.mjs && npm run lint && npm run build"
```

Expected: all PASS

**Step 5: Commit**

```bash
git add frontend/src/pages/System.tsx frontend/src/pages/Tenants.tsx frontend/src/tenant/pages/ApiKeysPage.tsx docs/plans/2026-03-17-frontend-redesign-design.md docs/plans/2026-03-17-frontend-redesign.md docs/plans/2026-03-17-frontend-page-archetypes.md
git commit -m "docs(frontend): record redesign rollout progress" -m "Track the redesigned complex pages and update the frontend redesign documents."
```
