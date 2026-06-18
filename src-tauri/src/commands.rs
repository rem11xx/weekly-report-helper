//! Tauri 命令层：暴露给前端的所有 invoke 命令
//!
//! 注意：Tauri v2 的 #[command] 要求返回的 Err 类型实现 serde::Serialize。
//! anyhow::Error 未实现，因此所有命令统一返回 Result<T, String>，
//! 内部 anyhow 错误通过 map_err 转字符串。

use rusqlite::params;
use tauri::State;

use crate::db::DbState;
use crate::models::*;
use crate::parser::{current_week_range, mock_now, parse_plan};
use crate::report::{build_report_data, render_markdown};

/// 统一把 rusqlite::Error / anyhow::Error 转 String（Tauri v2 要求 Err 实现 Serialize）
fn s<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

/// 获取或自动创建当前周 ID（消除 or_else 泛型推断问题）
fn ensure_week_id(conn: &rusqlite::Connection, week_start: &str) -> Result<i64, String> {
    let (_, week_end) = current_week_range();
    conn.query_row(
        "SELECT id FROM weeks WHERE week_start = ?1",
        params![week_start],
        |row| row.get(0),
    )
    .or_else(|_| -> Result<i64, rusqlite::Error> {
        conn.execute(
            "INSERT INTO weeks (week_start, week_end) VALUES (?1, ?2)",
            params![week_start, week_end],
        )?;
        Ok(conn.last_insert_rowid())
    })
    .map_err(s)
}

// ============ 解析 / 计划 ============

/// 仅解析文本，不落库（供输入页实时预览）
#[tauri::command]
pub fn parse_plan_cmd(raw: String) -> ParsedPlan {
    parse_plan(&raw)
}

/// 保存周计划：解析 + 落库（upsert week + 重建 planned_tasks）
#[tauri::command]
pub fn save_week_plan(state: State<'_, DbState>, raw: String) -> Result<Week, String> {
    let parsed = parse_plan(&raw);
    let conn = state.0.lock().unwrap();

    let (week_start, week_end) = (parsed.week_start.clone(), parsed.week_end.clone());

    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM weeks WHERE week_start = ?1",
            params![week_start],
            |row| row.get(0),
        )
        .ok();

    let week_id = match existing_id {
        Some(id) => {
            conn.execute(
                "UPDATE weeks SET plan_raw = ?1, week_end = ?2 WHERE id = ?3",
                params![raw, week_end, id],
            )
            .map_err(s)?;
            conn.execute(
                "DELETE FROM planned_tasks WHERE week_id = ?1",
                params![id],
            )
            .map_err(s)?;
            id
        }
        None => {
            conn.execute(
                "INSERT INTO weeks (week_start, week_end, plan_raw) VALUES (?1, ?2, ?3)",
                params![week_start, week_end, raw],
            )
            .map_err(s)?;
            conn.last_insert_rowid()
        }
    };

    for t in &parsed.tasks {
        conn.execute(
            "INSERT INTO planned_tasks (week_id, project, title, sort_order, estimate_d)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![week_id, t.project, t.title, t.sort_order, t.estimate_d],
        )
        .map_err(s)?;
    }

    let week = conn
        .query_row(
            "SELECT id, week_start, week_end, COALESCE(plan_raw,''), created_at FROM weeks WHERE id = ?1",
            params![week_id],
            |row| {
                Ok(Week {
                    id: row.get(0)?,
                    week_start: row.get(1)?,
                    week_end: row.get(2)?,
                    plan_raw: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .map_err(s)?;
    Ok(week)
}

/// 获取/创建当前周
#[tauri::command]
pub fn get_current_week(state: State<'_, DbState>) -> Result<CurrentWeek, String> {
    let conn = state.0.lock().unwrap();
    let (week_start, _week_end) = current_week_range();

    let week: Option<Week> = conn
        .query_row(
            "SELECT id, week_start, week_end, COALESCE(plan_raw,''), created_at FROM weeks
             WHERE week_start = ?1",
            params![week_start],
            |row| {
                Ok(Week {
                    id: row.get(0)?,
                    week_start: row.get(1)?,
                    week_end: row.get(2)?,
                    plan_raw: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .ok();

    let (week, week_id_opt) = match week {
        Some(w) => (Some(w.clone()), Some(w.id)),
        None => (None, None),
    };

    let planned = match week_id_opt {
        Some(wid) => {
            let mut stmt = conn
                .prepare(
                    "SELECT id, week_id, COALESCE(project,''), title, sort_order, estimate_d, carried_from
                     FROM planned_tasks WHERE week_id = ?1 ORDER BY sort_order",
                )
                .map_err(s)?;
            let rows = stmt.query_map(params![wid], |row| {
                Ok(PlannedTask {
                    id: row.get(0)?,
                    week_id: row.get(1)?,
                    project: row.get(2)?,
                    title: row.get(3)?,
                    sort_order: row.get(4)?,
                    estimate_d: row.get(5)?,
                    carried_from: row.get(6)?,
                })
            }).map_err(s)?;
            rows.filter_map(|r| r.ok()).collect()
        }
        None => vec![],
    };

    let adhoc = match week_id_opt {
        Some(wid) => {
            let mut stmt = conn
                .prepare(
                    "SELECT id, week_id, COALESCE(project,''), title, created_at
                     FROM adhoc_tasks WHERE week_id = ?1 ORDER BY created_at",
                )
                .map_err(s)?;
            let rows = stmt.query_map(params![wid], |row| {
                Ok(AdhocTask {
                    id: row.get(0)?,
                    week_id: row.get(1)?,
                    project: row.get(2)?,
                    title: row.get(3)?,
                    created_at: row.get(4)?,
                })
            }).map_err(s)?;
            rows.filter_map(|r| r.ok()).collect()
        }
        None => vec![],
    };

    Ok(CurrentWeek { week, planned, adhoc })
}

// ============ 番茄钟 ============

/// 获取番茄钟选任务弹窗所需的任务列表（本周 planned + 历史 adhoc 去重）
#[tauri::command]
pub fn get_task_options(state: State<'_, DbState>) -> Result<Vec<TaskOption>, String> {
    let conn = state.0.lock().unwrap();
    let (week_start, _) = current_week_range();
    let week_id_opt: Option<i64> = conn
        .query_row(
            "SELECT id FROM weeks WHERE week_start = ?1",
            params![week_start],
            |row| row.get(0),
        )
        .ok();

    let mut options: Vec<TaskOption> = Vec::new();

    if let Some(wid) = week_id_opt {
        let mut stmt = conn
            .prepare(
                "SELECT id, COALESCE(project,''), title FROM planned_tasks WHERE week_id = ?1 ORDER BY sort_order",
            )
            .map_err(s)?;
        let rows = stmt.query_map(params![wid], |row| {
            Ok(TaskOption {
                source: "planned".into(),
                task_id: row.get(0)?,
                project: row.get(1)?,
                title: row.get(2)?,
            })
        }).map_err(s)?;
        for r in rows {
            options.push(r.map_err(s)?);
        }

        let mut stmt2 = conn
            .prepare("SELECT id, COALESCE(project,''), title FROM adhoc_tasks ORDER BY created_at DESC")
            .map_err(s)?;
        let rows2 = stmt2.query_map([], |row| {
            Ok(TaskOption {
                source: "adhoc".into(),
                task_id: row.get(0)?,
                project: row.get(1)?,
                title: row.get(2)?,
            })
        }).map_err(s)?;
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        for r in rows2 {
            let opt = r.map_err(s)?;
            let key = (opt.project.clone(), opt.title.clone());
            if seen.insert(key) {
                options.push(opt);
            }
        }
    }

    Ok(options)
}

/// 获取所有历史项目名（下拉用）
#[tauri::command]
pub fn list_projects(state: State<'_, DbState>) -> Result<Vec<String>, String> {
    let conn = state.0.lock().unwrap();
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    let mut stmt = conn
        .prepare("SELECT DISTINCT COALESCE(project,'') FROM planned_tasks WHERE COALESCE(project,'') <> ''")
        .map_err(s)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(s)?;
    for r in rows {
        set.insert(r.map_err(s)?);
    }

    let mut stmt2 = conn
        .prepare("SELECT DISTINCT COALESCE(project,'') FROM adhoc_tasks WHERE COALESCE(project,'') <> ''")
        .map_err(s)?;
    let rows2 = stmt2.query_map([], |row| row.get::<_, String>(0)).map_err(s)?;
    for r in rows2 {
        set.insert(r.map_err(s)?);
    }

    Ok(set.into_iter().collect())
}

/// 新建计划外任务
#[tauri::command]
pub fn create_adhoc_task(
    state: State<'_, DbState>,
    project: String,
    title: String,
) -> Result<i64, String> {
    let conn = state.0.lock().unwrap();
    let (week_start, _) = current_week_range();
    let week_id = ensure_week_id(&conn, &week_start)?;

    conn.execute(
        "INSERT INTO adhoc_tasks (week_id, project, title) VALUES (?1, ?2, ?3)",
        params![week_id, project, title],
    )
    .map_err(s)?;
    Ok(conn.last_insert_rowid())
}

/// 记录一条番茄钟 session
#[tauri::command]
pub fn record_session(
    state: State<'_, DbState>,
    task_source: String,
    task_id: Option<i64>,
    started_at: String,
    ended_at: String,
    duration_min: i64,
    is_break: bool,
) -> Result<i64, String> {
    let conn = state.0.lock().unwrap();
    let (week_start, _) = current_week_range();
    let week_id = ensure_week_id(&conn, &week_start)?;

    conn.execute(
        "INSERT INTO pomodoro_sessions (week_id, task_source, task_id, started_at, ended_at, duration_min, is_break)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![week_id, task_source, task_id, started_at, ended_at, duration_min, is_break],
    )
    .map_err(s)?;
    Ok(conn.last_insert_rowid())
}

/// 获取本周全部 session
#[tauri::command]
pub fn list_sessions(
    state: State<'_, DbState>,
    week_id: i64,
) -> Result<Vec<PomodoroSession>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, week_id, task_source, task_id, started_at, ended_at, duration_min, is_break
             FROM pomodoro_sessions WHERE week_id = ?1 ORDER BY started_at",
        )
        .map_err(s)?;
    let rows = stmt.query_map(params![week_id], |row| {
        Ok(PomodoroSession {
            id: row.get(0)?,
            week_id: row.get(1)?,
            task_source: row.get(2)?,
            task_id: row.get(3)?,
            started_at: row.get(4)?,
            ended_at: row.get(5)?,
            duration_min: row.get(6)?,
            is_break: row.get::<_, i64>(7)? != 0,
        })
    }).map_err(s)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ============ 周报 ============

/// 获取周报统计原始数据
#[tauri::command]
pub fn get_report_data_cmd(
    state: State<'_, DbState>,
    week_id: i64,
) -> Result<ReportData, String> {
    let conn = state.0.lock().unwrap();
    build_report_data(&conn, week_id).map_err(s)
}

/// 周五勾选顺延：
/// - 勾选的标记 plan_next_monday=1
/// - 进行中 + 未开始自动进下周计划（carried_from 记录本周 task_id）
#[tauri::command]
pub fn carry_over_tasks(
    state: State<'_, DbState>,
    req: CarryOverRequest,
) -> Result<CarryOverResult, String> {
    let conn = state.0.lock().unwrap();

    // 1. 标记 plan_next_monday
    for id in &req.next_monday_task_ids {
        conn.execute(
            "UPDATE planned_tasks SET plan_next_monday = 1 WHERE id = ?1 AND week_id = ?2",
            params![id, req.week_id],
        )
        .map_err(s)?;
    }

    // 2. 推算下周 week_start/end
    let (this_start, this_end): (String, String) = conn
        .query_row(
            "SELECT week_start, week_end FROM weeks WHERE id = ?1",
            params![req.week_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(s)?;

    use chrono::NaiveDate;
    let next_start_date =
        NaiveDate::parse_from_str(&this_start, "%Y-%m-%d").map_err(s)? + chrono::Duration::days(7);
    let next_end_date =
        NaiveDate::parse_from_str(&this_end, "%Y-%m-%d").map_err(s)? + chrono::Duration::days(7);
    let next_start = next_start_date.format("%Y-%m-%d").to_string();
    let next_end = next_end_date.format("%Y-%m-%d").to_string();

    let next_week_id = ensure_week_id(&conn, &next_start)?;

    // 3. 进行中 + 未开始 的 planned 任务写入下周
    let data = build_report_data(&conn, req.week_id).map_err(s)?;
    let mut carried = 0i64;
    for t in &data.tasks {
        if t.source != "planned" {
            continue;
        }
        if matches!(t.status, TaskStatus::InProgress | TaskStatus::NotStarted) {
            conn.execute(
                "INSERT INTO planned_tasks (week_id, project, title, sort_order, estimate_d, carried_from)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    next_week_id,
                    t.project,
                    t.title,
                    t.sort_order,
                    t.estimate_d,
                    t.task_id
                ],
            )
            .map_err(s)?;
            carried += 1;
        }
    }

    Ok(CarryOverResult {
        next_week_id,
        carried_count: carried,
    })
}

/// 渲染 Markdown 周报
#[tauri::command]
pub fn render_report_markdown(
    state: State<'_, DbState>,
    week_id: i64,
) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    let data = build_report_data(&conn, week_id).map_err(s)?;
    Ok(render_markdown(&data))
}

/// 保存周报到文件
#[tauri::command]
pub fn save_report_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(s)
}

/// 是否需要提醒填本周计划（周二 12:00 后且本周无 planned_tasks）
#[tauri::command]
pub fn needs_plan_reminder(state: State<'_, DbState>) -> Result<bool, String> {
    use chrono::{Datelike, Timelike};
    let now = mock_now();
    let weekday = now.weekday().num_days_from_monday() as i64;
    let hour = now.hour() as i64;

    let in_reminder_window = weekday == 1 && hour >= 12;
    if !in_reminder_window {
        return Ok(false);
    }

    let conn = state.0.lock().unwrap();
    let (week_start, _) = current_week_range();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM planned_tasks pt
             JOIN weeks w ON pt.week_id = w.id
             WHERE w.week_start = ?1",
            params![week_start],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(count == 0)
}

/// 【dev】注入模拟当前时间，用于在界面上测试不同时间点（周二提醒/周五报告/周一归属）。
/// `iso` 形如 `2026-06-17T13:00`。
#[tauri::command]
pub fn set_mock_now(iso: String) -> Result<(), String> {
    crate::parser::set_mock_now(&iso)
}

/// 【dev】清除模拟时间，恢复真实系统时间。
#[tauri::command]
pub fn clear_mock_now() -> Result<(), String> {
    crate::parser::clear_mock_now();
    Ok(())
}
