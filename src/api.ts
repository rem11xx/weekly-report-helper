import { invoke } from "@tauri-apps/api/core";
import type {
  ParsedPlan,
  CurrentWeek,
  TaskOption,
  ReportData,
  CarryOverRequest,
  CarryOverResult,
  Week,
  PomodoroSession,
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

/** 获取番茄钟选任务弹窗所需的任务列表（本周 planned + 历史 adhoc） */
export function getTaskOptions(): Promise<TaskOption[]> {
  return invoke<TaskOption[]>("get_task_options");
}

/** 获取所有历史项目名（供下拉） */
export function listProjects(): Promise<string[]> {
  return invoke<string[]>("list_projects");
}

/** 新建计划外任务（番茄钟期间） */
export function createAdhocTask(
  project: string,
  title: string
): Promise<number> {
  return invoke<number>("create_adhoc_task", { project, title });
}

/** 记录一条番茄钟 session */
export function recordSession(params: {
  task_source: "planned" | "adhoc";
  task_id: number | null;
  started_at: string;
  ended_at: string;
  duration_min: number;
  is_break: boolean;
}): Promise<number> {
  return invoke<number>("record_session", params);
}

/** 获取本周全部番茄钟记录 */
export function listSessions(weekId: number): Promise<PomodoroSession[]> {
  return invoke<PomodoroSession[]>("list_sessions", { weekId });
}

/** 获取周报统计原始数据（未应用周五勾选） */
export function getReportData(weekId: number): Promise<ReportData> {
  return invoke<ReportData>("get_report_data", { weekId });
}

/** 周五勾选顺延：将勾选的标记为 next_monday，其余进行中/未开始自动进下周计划 */
export function carryOverTasks(req: CarryOverRequest): Promise<CarryOverResult> {
  return invoke<CarryOverResult>("carry_over_tasks", { ...req });
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

/** 【dev】清除模拟时间，恢复真实系统时间 */
export function clearMockNow(): Promise<void> {
  return invoke<void>("clear_mock_now");
}
