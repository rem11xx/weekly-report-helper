# 问题列表

> 用法：用户提出的问题**先记录到此文档**，不立即修改。修改顺序与优先级由用户决定；当用户要求「修复问题」而未指定编号时，按优先级取最优先的 3 个修复，而非一次性全部修复。问题在「已关闭」后删除其记录（表格行与对应方案小节）。
> 记录新问题时，追加到表格末尾并分配下一个编号，状态默认「待处理」，优先级留空待用户填写（或由 Claude 按影响评估）。

## 状态约定

| 状态 | 含义 |
|---|---|
| 待处理 | 已记录，未排期 |
| 已修复 | 已修改，构建/类型检查通过，待用户验证 |
| 已关闭 | 用户已在运行应用中验证，确认无问题（随即删除记录） |

## 优先级约定

可由用户填写，留空时 Claude 按影响评估，用于挑选待修复项。

- 高：阻塞核心流程 / 数据错误
- 中：影响体验但有绕过方式
- 低：细节优化 / 文案 / 样式微调

## 问题清单

| 编号 | 状态 | 优先级 | 模块/页面 | 问题描述 | 提出日期 | 备注 |
|---|---|---|---|---|---|---|
| P004 | 待处理 |  | Timer 页 | 「生成周报」按钮（.primary 蓝色填充）看起来像一直按下/激活，但当前未展示周报；应呈未点击默认态 | 2026-06-23 | 见下方方案 |
| P005 | 待处理 |  | Timer 页 | 去掉最上面的应用名字「工作周报助手」标题 | 2026-06-23 | 见下方方案 |
| P006 | 待处理 |  | Timer 页 | 点击倒计时圆环内也可开始/结束番茄钟 | 2026-06-23 | 见下方方案 |
| P007 | 待处理 |  | Timer 页 / 任务选择弹窗 | 结束番茄钟的任务选择弹窗「新建任务」：(1) 选项文案去掉前导加号；(2) 不记住上次填写的项目/内容；(3) 第二个输入框 placeholder「任务标题」改为「任务内容」 | 2026-06-23 | 见下方方案 |
| P008 | 待处理 | 低 | Timer 页 | Timer 页内容上方与下方的空白高度不一致，应改为上下留白保持一致 | 2026-06-23 | 采用方案 A（.timer-view justify-content: center），暂不修改 |

### P004 方案

**成因**
- 「生成周报」按钮带 `.primary` 类（`TimerView.vue`），常态即蓝色填充 `#3b82f6` + 白字，作为主操作强调样式；CSS 无 `:active/:focus` 规则，并非状态卡住。
- 蓝色填充视觉上易被读作“按下/激活”，而该按钮是触发打开 `ReportPreviewModal` 的动作按钮，弹窗未开时不应显激活态。

**方案**
- A（最简）：去掉 `.primary`，三按钮统一白底，「生成周报」不再特别强调。
- B（语义，建议）：保留主操作区分，但仅在对应弹窗打开期间显激活态——`<button :class="{ active: showReportModal }">`，「周计划」同理用 `showPlanModal`；CSS `.action-btn.active` 蓝色、默认白底。

### P005 方案

- 删除 `TimerView.vue` 中 `<h1 class="app-title">工作周报助手</h1>` 及对应 `.app-title` 样式；顶部留白由 `.main-card` 的 `padding`（`32px 24px 20px`）自然给出。

### P006 方案

- 给倒计时圆环加点击交互：在 `ring-wrapper`（`TimerView.vue`）上绑 `@click="onRingClick"`：
  - `idle` → `timer.startFocus()`
  - `focus` → `timer.manualEnd()`（弹任务选择，与「结束」按钮一致）
  - `break` → `timer.reset()`（与「跳过休息」一致）
- `CountdownRing.vue` 根 `.countdown-ring` 加 `cursor: pointer` + `title` 悬停提示（如“点击开始/结束”）。

### P007 方案

- `TaskPickerModal.vue`：
  - 「+ 新建任务」→「新建任务」（去掉 `+ `）。
  - 第二个 `NInput` placeholder `任务标题` → `任务内容`。
  - 不记住上次填写：watch `store.showTaskPicker`，弹窗打开（true）时清空 `newProject`/`newTitle`（当前组件常驻挂载、输入会保留上次的值）。

### P008 方案

**成因**
- `.timer-view` 用 `justify-content: flex-start`（P001 修复时改），主卡片贴顶部，下方剩余空间堆在底部 → 上下留白不对称（上= `.timer-view` padding-top 20px + `.main-card` padding-top 32px；下= padding-bottom 20px）。
- `.main-card` 自身 `padding: 32px 24px 20px`（上下不对称）也加剧偏差。

**方案**
- A（居中，推荐）：`.timer-view` 改回 `justify-content: center`，但配合 P001 已加的 `min-height:0` + `overflow-y:auto`——内容超高时仍内部滚动、不裁顶（与 flex-start 一样安全），同时正常高度时上下留白对称居中。
- B（留白对称）：保持 flex-start，把 `.main-card` 的 `padding` 上下统一（如 `24px 24px`），并把 `.timer-view` 的 `padding` 也上下统一；居中由给 `.main-card` 加 `margin: auto 0` 实现。
- 注意：A 仅改一行且最稳；B 更精细但改动点多。建议 A。

<!-- 示例（保留作格式参考，可删除）：
| P001 | 待处理 |  | Timer 页 | 倒计时圆环在 0% 时颜色异常 | 2026-06-23 | 仅深色模式复现 |
-->
