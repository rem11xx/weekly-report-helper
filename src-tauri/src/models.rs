use serde::{Deserialize, Serialize};

/// 周记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Week {
    pub id: i64,
    pub week_start: String, // YYYY-MM-DD（周二）
    pub week_end: String,   // YYYY-MM-DD（下周一）
    pub plan_raw: String,
    pub created_at: String,
}

/// 计划内任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTask {
    pub id: i64,
    pub week_id: i64,
    pub project: String,
    pub title: String,
    pub sort_order: i64,
    pub estimate_d: f64,
    pub carried_from: Option<i64>,
    pub done: bool,
}

/// 计划外任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdhocTask {
    pub id: i64,
    pub week_id: i64,
    pub project: String,
    pub title: String,
    pub sort_order: i64,
    pub done: bool,
    pub created_at: String,
}

/// 番茄钟记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroSession {
    pub id: i64,
    pub week_id: i64,
    pub task_source: String, // "planned" | "adhoc"
    pub task_id: Option<i64>,
    pub started_at: String,
    pub ended_at: String,
    pub duration_min: i64,
    pub is_break: bool,
}

/// 文本解析出的单个任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTask {
    pub project: String,
    pub title: String,
    pub sort_order: i64,
    pub estimate_d: f64,
    pub raw: String,
}

/// 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlan {
    pub week_start: String,
    pub week_end: String,
    pub tasks: Vec<ParsedTask>,
    pub errors: Vec<String>,
}

/// 番茄钟选任务弹窗用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOption {
    pub source: String, // "planned" | "adhoc"
    pub task_id: i64,
    pub project: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Done,
    InProgress,
    NextMonday,
    NotStarted,
}

/// 周报中的单个任务呈现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTask {
    pub source: String, // "planned" | "adhoc"
    pub task_id: i64,
    pub project: String,
    pub title: String,
    pub sort_order: i64,
    pub estimate_d: f64,
    pub actual_min: i64,
    pub actual_d: f64,
    pub status: TaskStatus,
    pub plan_next_monday: bool,
    pub carried_from: Option<i64>,
}

/// 周报完整数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub week: Week,
    pub tasks: Vec<ReportTask>,
    pub total_actual_d: f64,
}

/// 当前周概况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentWeek {
    pub week: Option<Week>,
    pub planned: Vec<PlannedTask>,
    pub adhoc: Vec<AdhocTask>,
}

/// 顺延请求
#[derive(Debug, Clone, Deserialize)]
pub struct CarryOverRequest {
    pub week_id: i64,
    pub next_monday_task_ids: Vec<i64>,
}

/// 应用全局设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub always_on_top: bool,
}

/// 数据库存储位置信息（供设置页展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStorageInfo {
    pub path: String,
    pub is_custom: bool,
}
