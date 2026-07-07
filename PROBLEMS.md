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
| P005 | 已关闭 | 低 | Timer 页 | 去掉最上面的应用名字「工作周报助手」标题 | |
| P008 | 已关闭 | 低 | Timer 页 | Timer 页内容上方与下方的空白高度不一致，应改为上下留白保持一致 | |
| P009 | 已关闭 | 高 | 周计划输入页 | 输入成段文本后解析预览提示「未解析到任务，请检查格式」，但文本格式无误 | `parser::parse_plan` 导入别名 `parse_plan_text` 避同名冲突 |
| P010 | 已关闭 | 高 | 周报页 | 周报页加载统计数据失败：`get_report_data` 命令名不匹配（与 P009 同源） | 随 P009 一并修复 |
| P012 | 已关闭 | 低 | 后端 / carry_over_tasks | 启动时 cargo 编译警告：unused variable `next_end`（commands.rs），该下周 end 推算后未被使用 | |
| P013 | 已关闭 | 中 | 周计划弹窗 / PlanTaskTable | 本周任务清单中可拖动行，但松开后拖动结果在前端不生效（顺序与序号均不变化），无法保存排序 | `tauri.conf.json` 设 `dragDropEnabled:false` 把拖放控制权还给 HTML5（本 app 无文件拖放需求） |
| P014 | 已关闭 | 高 | 后端 / carry_over_tasks | 周五「确认顺延 + 生成周报」链路静默失败：前端 `carryOverTasks` 用 `invoke("carry_over_tasks", { ...req })` 摊开参数，但后端签名是单结构体 `req: CarryOverRequest`，Tauri v2 按参数名 camelCase 即 `req` 取 key → `body.get("req")` 缺失 → invoke reject 被 `report.ts` catch 吞掉 → `plan_next_monday` 永不置位、下周任务不插入、点「确认」不复制不弹 toast 且弹窗不关 | `invoke` 需整体传结构体 `{ req }`，勿摊开参数（Tauri v2 按参数名取 key） |
| P015 | 已关闭 | 中 | 后端 / record_session | 番茄钟 session 历史记录为零：本 diff 前 `recordSession` 用 snake_case 调用 `record_session`，Tauri v2 按 camelCase 取 key 全部 miss → 每次 invoke reject → session 从未落库 → `minutes_map` 恒空、任务状态恒 not_started。历史数据无法恢复，需告知用户此为预期（非数据损坏） | 修复前历史 session 未落库、无法恢复，属预期非数据损坏 |
| P016 | 已关闭 | 中 | 周报页 / report.rs | 生成周报后 id 为 38 的任务时间统计输出为「[已完成] 0d」 | 保持现状不改（0d 如实反映无番茄钟），不引入 spec 强制覆盖 |

| P017 | 已关闭 | 高 | 周计划弹窗 / carry_over_tasks | 周二（2026-06-30）打开周计划仍显示上周内容、无本周输入入口 | carry_over 仅标记 `plan_next_monday=1`，不再预填下周 planned_tasks；周报「下周计划」由本周任务状态推导 |

| P018 | 已关闭 | 高 | 周计划弹窗 / Timer 页 / ensure_week_id | 周二下午打开应用无「本周计划未录入」提醒；打开周计划仍显示上周 carry-over 残留任务（表格态），无文本输入入口 | `ensure_week_id` 的 week_end 改用 `week_start+6` 推导（根除幽灵行）；保存/清空计划后联动 `checkReminder()` 刷新 banner |
| P019 | 已关闭 |  | Timer 页 | 专注倒计时中可收起为「浮球」：仅显示半径 200px 倒计时圆环、无标题栏、默认置顶、可拖动；专注结束自动恢复主界面 | 透明窗口需 tauri.conf 声明 `transparent` + `macOSPrivateApi`（Cargo `macos-private-api` feature），且 macOS 12+ 还须显式 `backgroundColor:[0,0,0,0]` 才能让 WKWebView 的 `underPageBackgroundColor` 透明（仅 `transparent:true` 只关 `drawsBackground`，底色仍不透明）；浮球交互用左键按下 `startDragging` 拖动、双击恢复，不用 `data-tauri-drag-region`（会劫持双击做最大化）；常态保留标题栏，浮球态运行时 `setDecorations(false)` + 强制置顶（不写库，退出按用户偏好恢复） |
| P020 | 已关闭 | 低 | 番茄钟选任务弹窗 / TaskPickerModal | 专注结束选任务弹窗中任务已按项目分组,但每组前未展示项目名;需要在每组选项前显示所属项目 | 分组已存在,缺项目标题头 |
| P021 | 已关闭 | 中 | 番茄钟选任务弹窗 / TaskPickerModal | 需要在已有项目的任务列表最下方直接填写计划外任务(项目+标题),而非走单独的新建入口 | 每个项目组下空行回车即加 adhoc 到该项目；底部表单用于未列出项目,项目必填、回车提交(无按钮)；计划内/计划外均支持回车与「确认」按钮提交(按最近交互处决定) |
| P022 | 已关闭 | 中 | 番茄钟选任务弹窗 / TaskPickerModal | 选任务弹窗目前展示上周(历史)创建的计划外任务,应仅展示本周任务,去掉历史 adhoc | adhoc 查询补 week_id 过滤;历史 adhoc 不再出现于弹窗(预期) |
| P023 | 待处理 | 低 | Timer 页 / 浮球 | 浮球功能当前仅 macOS 验证通过，Windows 下需单独调试：①透明窗口背景（`transparent`+`macOSPrivateApi` 为 macOS 调校，Windows 透明机制不同，常态带标题栏的透明窗口可能有渲染瑕疵）；②浮球拖动用 mousedown 触发 `startDragging`（避开 `data-tauri-drag-region` 劫持双击），Windows 行为待验证；③圆环 `drop-shadow` 在 Windows WebView2 下渲染待验证；④跨显示器不同 DPI 下位置恢复准确性 | 实现按 macOS 调校，Windows 透明/拖动机制不同 |

<!-- 示例（保留作格式参考，可删除）：
| P001 | 待处理 |  | Timer 页 | 倒计时圆环在 0% 时颜色异常 | 仅深色模式复现 |
-->
