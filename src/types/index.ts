// 前后端共享的数据类型（与 src-tauri/src/models.rs 保持一致）

/** 周记录 */
export interface Week {
  id: number;
  week_start: string; // YYYY-MM-DD（周二）
  week_end: string; // YYYY-MM-DD（下周一）
  plan_raw: string;
  created_at: string;
}

/** 计划内任务（周初解析得到） */
export interface PlannedTask {
  id: number;
  week_id: number;
  project: string;
  title: string;
  sort_order: number;
  estimate_d: number;
  carried_from: number | null;
  done: boolean;
}

/** 计划外任务（番茄钟期间新建） */
export interface AdhocTask {
  id: number;
  week_id: number;
  project: string;
  title: string;
  sort_order: number;
  done: boolean;
  created_at: string;
}

/** 番茄钟记录 */
export interface PomodoroSession {
  id: number;
  week_id: number;
  task_source: "planned" | "adhoc";
  task_id: number | null;
  started_at: string;
  ended_at: string;
  duration_min: number;
  is_break: boolean;
}

/** 文本解析结果（供输入页预览） */
export interface ParsedPlan {
  week_start: string;
  week_end: string;
  tasks: ParsedTask[];
  errors: string[];
}

export interface ParsedTask {
  project: string;
  title: string;
  sort_order: number;
  estimate_d: number;
  raw: string;
}

/** 当前周概况 */
export interface CurrentWeek {
  week: Week | null;
  planned: PlannedTask[];
  adhoc: AdhocTask[];
}

/** 番茄钟选任务弹窗用：按项目分组的任务列表 */
export interface TaskOption {
  source: "planned" | "adhoc";
  task_id: number;
  project: string;
  title: string;
}

/** 周报统计中单个任务的呈现 */
export interface ReportTask {
  source: "planned" | "adhoc";
  task_id: number;
  project: string;
  title: string;
  sort_order: number;
  estimate_d: number;
  actual_min: number; // 番茄钟累计分钟
  actual_d: number; // 换算后的 d
  status: TaskStatus;
  plan_next_monday: boolean;
  carried_from: number | null;
}

export type TaskStatus =
  | "done" // 已完成
  | "in_progress" // 进行中
  | "next_monday" // 计划下周一完成
  | "not_started"; // 未开始

/** 周报完整数据（供前端渲染 Markdown 之外也可用于弹窗勾选） */
export interface ReportData {
  week: Week;
  tasks: ReportTask[];
  total_actual_d: number;
}

/** 周五勾选顺延任务的请求体 */
export interface CarryOverRequest {
  week_id: number;
  // 状态为 next_monday 的任务（番茄钟未达预估，由用户勾选的）
  next_monday_task_ids: number[];
  // 顺延任务仅作为本周周报「下周计划」项呈现（后端从本周任务状态推导），
  // 不再预填进下周 planned_tasks；下周二由新文本解析生成本周任务（P017）。
}

/** 应用全局设置 */
export interface AppSettings {
  always_on_top: boolean;
  /** 开始专注即进入浮球（默认开） */
  focus_enters_mini: boolean;
}

/** 数据库存储位置信息（与后端 DbStorageInfo 对应） */
export interface DbStorageInfo {
  /** 当前生效的 weekly.db 绝对路径 */
  path: string;
  /** false => 默认 app_data_dir 位置；true => 用户自定义目录 */
  is_custom: boolean;
}
