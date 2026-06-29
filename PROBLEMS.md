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
| P013 | 已关闭 | 中 | 周计划弹窗 / PlanTaskTable | 本周任务清单中可拖动行，但松开后拖动结果在前端不生效（顺序与序号均不变化），无法保存排序 | 根因（二次排查后修正）：Tauri v2 `app.windows[].dragDropEnabled` 默认 `true`，webview 在 OS 层捕获拖放以接收拖入文件，会**吞掉 HTML5 `dragstart`** → `onDragStart` 从不执行，整条 DnD 事件链不启动。首次「把手方案」把 `draggable` 从 `<tr>` 移到 `<span>` 把手仍不触发，正因如此（与元素类型无关）。修复：`tauri.conf.json` 窗口加 `"dragDropEnabled": false`，把拖放控制权还给 HTML5（本 app 无文件拖放需求，零损失）；保留把手方案（避免与 textarea/checkbox 原生拖拽冲突）。`cargo build` 通过。用户已在运行应用中验证拖拽排序生效 |
| P014 | 已关闭 | 高 | 后端 / carry_over_tasks | 周五「确认顺延 + 生成周报」链路静默失败：前端 `carryOverTasks` 用 `invoke("carry_over_tasks", { ...req })` 摊开参数，但后端签名是单结构体 `req: CarryOverRequest`，Tauri v2 按参数名 camelCase 即 `req` 取 key → `body.get("req")` 缺失 → invoke reject 被 `report.ts` catch 吞掉 → `plan_next_monday` 永不置位、下周任务不插入、点「确认」不复制不弹 toast 且弹窗不关 | 已关闭：`api.ts` 改 `invoke("carry_over_tasks", { req })`（整体传结构体，不再展开参数）；顺带为「确认任务完成情况」弹窗增加勾选逻辑说明区（黑色文字，适配亮色背景）。用户已在运行应用中验证。 |
| P015 | 已修复 | 中 | 后端 / record_session | 番茄钟 session 历史记录为零：本 diff 前 `recordSession` 用 snake_case 调用 `record_session`，Tauri v2 按 camelCase 取 key 全部 miss → 每次 invoke reject → session 从未落库 → `minutes_map` 恒空、任务状态恒 not_started。历史数据无法恢复，需告知用户此为预期（非数据损坏） | camelCase 修复已在代码中（`api.ts` 映射 `taskSource`/`taskId`/`startedAt`/`endedAt`/`durationMin`/`isBreak`），经查库验证生效：本周（week_id=1）已正常落库 17 条 session、643 专注分钟。数据恢复无必要（无法重建未知历史丢失）。按用户决定不加「无番茄钟」UI 提示。待运行应用验证后关闭 |
| P016 | 已关闭 | 中 | 周报页 / report.rs | 生成周报后 id 为 38 的任务时间统计输出为「[已完成] 0d」 | 查库确认：任务 38 为 planned、`done=1`、`estimate_d=0.75`，但 `pomodoro_sessions` 无对应记录——属「手动完成但无番茄钟」，**非** P015 snake_case 数据丢失（本周 17 条 session 正常落库）。`done` 标志强制 `status=Done`、`actual_d=0d` 即输出「[已完成] 0d」。经与用户确认：保持现状不改代码（0d 如实反映无番茄钟），不引入 spec 强制覆盖。 |

<!-- 示例（保留作格式参考，可删除）：
| P001 | 待处理 |  | Timer 页 | 倒计时圆环在 0% 时颜色异常 | 仅深色模式复现 |
-->
