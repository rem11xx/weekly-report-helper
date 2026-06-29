# 问题列表

> 用法：用户提出的问题**先记录到此文档**，不立即修改。修改顺序与优先级由用户决定；当用户要求「修复问题」而未指定编号时，按优先级取最优先的 3 个修复，而非一次性全部修复。问题在「已关闭」后**保留表格行**（仅删除其方案小节），以便保留历史记录。
> 记录新问题时，追加到表格末尾并分配下一个编号，状态默认「待处理」，优先级留空待用户填写（或由 Claude 按影响评估）。

## 状态约定

| 状态 | 含义 |
|---|---|
| 待处理 | 已记录，未排期 |
| 已修复 | 已修改，构建/类型检查通过，待用户验证 |
| 已关闭 | 用户已在运行应用中验证，确认无问题（保留表格行，删除其方案小节） |

## 优先级约定

可由用户填写，留空时 Claude 按影响评估，用于挑选待修复项。

- 高：阻塞核心流程 / 数据错误
- 中：影响体验但有绕过方式
- 低：细节优化 / 文案 / 样式微调

## 问题清单

| 编号 | 状态 | 优先级 | 模块/页面 | 问题描述 | 备注 |
|---|---|---|---|---|---|
| P005 | 已关闭 | 低 | Timer 页 | 去掉最上面的应用名字「工作周报助手」标题 | 删除 `<h1 class="app-title">` 及 `.app-title` 样式，顶部留白由 `.main-card` padding 给出 |
| P008 | 已关闭 | 低 | Timer 页 | Timer 页内容上方与下方的空白高度不一致，应改为上下留白保持一致 | 已关闭：`.timer-view` `padding: 20px` 四边统一、`justify-content: flex-start`；`.main-card` 加 `flex:1` 撑满剩余高度。上下左右空白均=20px，中间内容撑满。用户已验证 |
| P009 | 已关闭 | 高 | 周计划输入页 | 输入成段文本后解析预览提示「未解析到任务，请检查格式」，但文本格式无误 | 后端命令去 `_cmd` 后缀：`parse_plan_cmd→parse_plan`、`get_report_data_cmd→get_report_data`（含 `lib.rs` 注册）；`commands.rs` 内 `parser::parse_plan` 导入改别名 `parse_plan_text` 避同名冲突 |
| P010 | 已关闭 | 高 | 周报页 | 周报页加载统计数据失败：`get_report_data` 命令名不匹配（与 P009 同源） | 随 P009 一并修复（`get_report_data_cmd→get_report_data`，`lib.rs` 注册同步），前端不动 |
| P012 | 已关闭 | 低 | 后端 / carry_over_tasks | 启动时 cargo 编译警告：unused variable `next_end`（commands.rs），该下周 end 推算后未被使用 | 已关闭：删除 `this_end`/`next_end_date`/`next_end` 死链，`cargo build` 无 warning，用户已确认 |
| P013 | 待处理 | 中 | 周计划弹窗 / PlanTaskTable | 本周任务清单中可拖动行，但松开后拖动结果在前端不生效（顺序与序号均不变化），无法保存排序 | 2026-06-24 记录；已尝试按拖动方向修正 `onDrop` 插入位置（向下拖插到 target 之后），问题依旧。审查补充根因：疑为 HTML5 DnD 作用在 `<tr>` 上浏览器兼容性差（Firefox 等 `dragstart` 可能不触发 → drop 永不触发），建议改用拖拽把手或专用库（`vuedraggable`/`useDraggable`）而非继续调 `onDrop` |
| P014 | 已修复 | 高 | 后端 / carry_over_tasks | 周五「确认顺延 + 生成周报」链路静默失败：前端 `carryOverTasks` 用 `invoke("carry_over_tasks", { ...req })` 摊开参数，但后端签名是单结构体 `req: CarryOverRequest`，Tauri v2 按参数名 camelCase 即 `req` 取 key → `body.get("req")` 缺失 → invoke reject 被 `report.ts` catch 吞掉 → `plan_next_monday` 永不置位、下周任务不插入、点「确认」不复制不弹 toast 且弹窗不关 | 2026-06-24 审查发现；根因已对照 tauri 2.11.2 / tauri-macros 2.6.2 源码核实（结构体参数不会自动展开）。建议前端改 `invoke("carry_over_tasks", { req })`（最小改动），或后端拆 `week_id`/`next_monday_task_ids` 两个基本类型参数 + 前端传 camelCase。与 P013 无关；`record_session` 同类 bug 本 diff 已修。2026-06-29 用户复报相同症状（确认任务完成情况弹窗勾选后点「确认」无反应），已核对当前代码 bug 仍在：`api.ts:98` 仍 `invoke("carry_over_tasks", { ...req })` 摊开参数；`ReportView` 的「确认」走 `confirmAndRender`，其 catch 仅 `console.error`、不关弹窗不渲染不复制，故表现为「无反应」。最小修复：前端改 `invoke("carry_over_tasks", { req })`。已修复（2026-06-29）：`api.ts` 改 `invoke("carry_over_tasks", { req })`（不再展开参数），`vue-tsc --noEmit` 通过；待用户在运行应用中验证「确认」会关弹窗、渲染周报。 |
| P015 | 待处理 |  | 后端 / record_session | 番茄钟 session 历史记录为零：本 diff 前 `recordSession` 用 snake_case 调用 `record_session`，Tauri v2 按 camelCase 取 key 全部 miss → 每次 invoke reject → session 从未落库 → `minutes_map` 恒空、任务状态恒 not_started。历史数据无法恢复，需告知用户此为预期（非数据损坏） | 2026-06-24 审查发现；代码已由本 diff 修为 camelCase 映射（`taskSource`/`taskId`/`startedAt`/`endedAt`/`durationMin`/`isBreak`）。后续待定：是否在 UI 提示历史番茄钟为零、是否需数据补救 |

<!-- 示例（保留作格式参考，可删除）：
| P001 | 待处理 |  | Timer 页 | 倒计时圆环在 0% 时颜色异常 | 仅深色模式复现 |
-->
