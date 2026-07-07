import { invoke } from "@tauri-apps/api/core";
import type {
  ParsedPlan,
  CurrentWeek,
  TaskOption,
  ReportData,
  CarryOverRequest,
  Week,
  PomodoroSession,
  AdhocTask,
  DbStorageInfo,
  WindowPositions,
} from "@/types";

/** 解析周计划文本（仅预览，不落库） */
export function parsePlan(raw: string): Promise<ParsedPlan> {
  return invoke<ParsedPlan>("parse_plan", { raw });
}

/** 保存周计划（解析 + 落库 week + planned_tasks） */
export function saveWeekPlan(raw: string): Promise<Week> {
  return invoke<Week>("save_week_plan", { raw });
}

/** 获取/创建当前周，返回本周计划与计划外任务 */
export function getCurrentWeek(): Promise<CurrentWeek> {
  return invoke<CurrentWeek>("get_current_week");
}

/** 获取番茄钟选任务弹窗所需的任务列表（本周 planned + 本周 adhoc） */
export function getTaskOptions(): Promise<TaskOption[]> {
  return invoke<TaskOption[]>("get_task_options");
}

/** 获取所有历史项目名（供下拉） */
export function listProjects(): Promise<string[]> {
  return invoke<string[]>("list_projects");
}

/** 新建计划外任务（番茄钟期间或周计划表格空行）；返回新建行（含序号） */
export function createAdhocTask(
  project: string,
  title: string
): Promise<AdhocTask> {
  return invoke<AdhocTask>("create_adhoc_task", { project, title });
}

/** 更新单个任务的内容 / 序号 / 完成态（计划内或计划外通用）。
 *  类型保留 snake_case 与后端模型一致；invoke 时顶层参数转 camelCase
 *  （Tauri v2 默认把 Rust snake_case 参数名映射为 JS camelCase）。 */
export function updateTask(params: {
  source: "planned" | "adhoc";
  id: number;
  title: string;
  sort_order: number;
  done: boolean;
}): Promise<void> {
  return invoke<void>("update_task", {
    source: params.source,
    id: params.id,
    title: params.title,
    sortOrder: params.sort_order,
    done: params.done,
  });
}

/** 记录一条番茄钟 session。
 *  类型保留 snake_case 与后端模型一致；invoke 时顶层参数转 camelCase。 */
export function recordSession(params: {
  task_source: "planned" | "adhoc";
  task_id: number | null;
  started_at: string;
  ended_at: string;
  duration_min: number;
  is_break: boolean;
}): Promise<number> {
  return invoke<number>("record_session", {
    taskSource: params.task_source,
    taskId: params.task_id,
    startedAt: params.started_at,
    endedAt: params.ended_at,
    durationMin: params.duration_min,
    isBreak: params.is_break,
  });
}

/** 获取本周全部番茄钟记录 */
export function listSessions(weekId: number): Promise<PomodoroSession[]> {
  return invoke<PomodoroSession[]>("list_sessions", { weekId });
}

/** 获取周报统计原始数据（未应用周五勾选） */
export function getReportData(weekId: number): Promise<ReportData> {
  return invoke<ReportData>("get_report_data", { weekId });
}

/** 周五勾选顺延：将勾选的标记为 next_monday，其余进行中/未开始自动进下周计划。
 *  后端签名为单结构体参数 `req: CarryOverRequest`，Tauri v2 按参数名（camelCase 即 `req`）
 *  取 key，故必须整体包成 `{ req }` 传递；不能展开（展开后 `body.get("req")` 缺失会 reject）。
 *  `CarryOverRequest` 字段保持 snake_case 与后端模型一致，由结构体自身 Deserialize 按字段名解析。 */
export function carryOverTasks(req: CarryOverRequest): Promise<void> {
  return invoke<void>("carry_over_tasks", { req });
}

/** 渲染 Markdown 周报文本 */
export function renderReportMarkdown(weekId: number): Promise<string> {
  return invoke<string>("render_report_markdown", { weekId });
}

/** 把周报保存为 .md 文件（路径由前端对话框决定，后端写入） */
export function saveReportFile(path: string, content: string): Promise<void> {
  return invoke<void>("save_report_file", { path, content });
}

/** 工具：判断是否需要提醒填本周计划（周二 12:00 之后且本周无计划） */
export function needsPlanReminder(): Promise<boolean> {
  return invoke<boolean>("needs_plan_reminder");
}

/** 【dev】注入模拟当前时间（ISO 形如 2026-06-17T13:00） */
export function setMockNow(iso: string): Promise<void> {
  return invoke<void>("set_mock_now", { iso });
}

/** 【dev】清空本周全部数据（计划内/计划外任务 + 番茄钟记录 + plan_raw），回到无计划输入态 */
export function clearWeekData(): Promise<void> {
  return invoke<void>("clear_week_data");
}

/** 【dev】清除模拟时间，恢复真实系统时间 */
export function clearMockNow(): Promise<void> {
  return invoke<void>("clear_mock_now");
}

/** 读取窗口置顶设置 */
export function getAlwaysOnTop(): Promise<boolean> {
  return invoke<boolean>("get_always_on_top");
}

/** 设置窗口置顶（后端写库 + 应用到主窗口） */
export function setAlwaysOnTop(alwaysOnTop: boolean): Promise<void> {
  return invoke<void>("set_always_on_top", { alwaysOnTop });
}

/** 读取「开始专注即进入浮球」设置（默认开） */
export function getFocusEntersMini(): Promise<boolean> {
  return invoke<boolean>("get_focus_enters_mini");
}

/** 设置「开始专注即进入浮球」（仅写库；专注开始时由前端读取生效） */
export function setFocusEntersMini(focusEntersMini: boolean): Promise<void> {
  return invoke<void>("set_focus_enters_mini", { focusEntersMini });
}

/** 读取浮球/常态各自记忆的窗口位置（逻辑坐标） */
export function getWindowPositions(): Promise<WindowPositions> {
  return invoke<WindowPositions>("get_window_positions");
}

/** 写入浮球/常态窗口位置（整体覆盖；前端在切换捕获位置时调用） */
export function setWindowPositions(positions: WindowPositions): Promise<void> {
  return invoke<void>("set_window_positions", { positions });
}

/** 读取当前数据库存储位置（文件路径 + 是否自定义） */
export function getDbStoragePath(): Promise<DbStorageInfo> {
  return invoke<DbStorageInfo>("get_db_storage_path");
}

/** 切换数据库存储文件夹（后端：校验→按需迁移→写配置→重启）。
 *  new_dir -> newDir（Tauri v2 把 Rust snake_case 参数名映射为 JS camelCase）。 */
export function setDbStoragePath(newDir: string): Promise<void> {
  return invoke<void>("set_db_storage_path", { newDir });
}

/** 恢复默认存储位置（后端：迁回默认→清空配置→重启） */
export function restoreDefaultDbPath(): Promise<void> {
  return invoke<void>("restore_default_db_path");
}
