//! Tauri 命令层：暴露给前端的所有 invoke 命令
//!
//! 注意：Tauri v2 的 #[command] 要求返回的 Err 类型实现 serde::Serialize。
//! anyhow::Error 未实现，因此所有命令统一返回 Result<T, String>，
//! 内部 anyhow 错误通过 map_err 转字符串。

use rusqlite::{params, DatabaseName};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager, State};

use crate::config;
use crate::db::DbState;
use crate::models::*;
use crate::parser::{current_week_range, mock_now, parse_plan as parse_plan_text};
use crate::report::{build_report_data, render_markdown};

/// 统一把 rusqlite::Error / anyhow::Error 转 String（Tauri v2 要求 Err 实现 Serialize）
fn s<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

/// 获取或自动创建指定周 ID（消除 or_else 泛型推断问题）
///
/// week_end 由传入的 week_start 推导（+6 天），**不**重新调 `current_week_range()`：
/// 调用方可能传入非当前周的 week_start（如未来某周），若 week_end 取当前周会得到
/// `week_end < week_start` 的幽灵行——P018 根因即此（id=3 出现 07-07~07-06）。
fn ensure_week_id(conn: &rusqlite::Connection, week_start: &str) -> Result<i64, String> {
    use chrono::NaiveDate;
    let start_date = NaiveDate::parse_from_str(week_start, "%Y-%m-%d").map_err(s)?;
    let week_end = (start_date + chrono::Duration::days(6))
        .format("%Y-%m-%d")
        .to_string();

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

// ============ 设置 ============

/// 读取窗口置顶设置
#[tauri::command]
pub fn get_always_on_top(state: State<'_, DbState>) -> Result<bool, String> {
    let conn = state.0.lock().unwrap();
    let flag: i64 = conn
        .query_row(
            "SELECT always_on_top FROM app_settings WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(flag != 0)
}

/// 设置窗口置顶（写库 + 应用到主窗口）
#[tauri::command]
pub fn set_always_on_top(
    state: State<'_, DbState>,
    app: AppHandle,
    always_on_top: bool,
) -> Result<(), String> {
    let flag: i64 = if always_on_top { 1 } else { 0 };
    {
        let conn = state.0.lock().unwrap();
        conn.execute(
            "UPDATE app_settings SET always_on_top = ?1 WHERE id = 1",
            params![flag],
        )
        .map_err(s)?;
    } // 释放锁后再调用窗口 API
    if let Some(window) = app.get_webview_window("main") {
        window.set_always_on_top(always_on_top).map_err(s)?;
    }
    Ok(())
}

/// 读取「开始专注即进入浮球」设置（默认开）
#[tauri::command]
pub fn get_focus_enters_mini(state: State<'_, DbState>) -> Result<bool, String> {
    let conn = state.0.lock().unwrap();
    let flag: i64 = conn
        .query_row(
            "SELECT focus_enters_mini FROM app_settings WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(1);
    Ok(flag != 0)
}

/// 设置「开始专注即进入浮球」（仅写库；专注开始时由前端读取生效）
#[tauri::command]
pub fn set_focus_enters_mini(
    state: State<'_, DbState>,
    focus_enters_mini: bool,
) -> Result<(), String> {
    let flag: i64 = if focus_enters_mini { 1 } else { 0 };
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE app_settings SET focus_enters_mini = ?1 WHERE id = 1",
        params![flag],
    )
    .map_err(s)?;
    Ok(())
}

/// 读取浮球/常态各自记忆的窗口位置（JSON 列；空/损坏回落空）
#[tauri::command]
pub fn get_window_positions(state: State<'_, DbState>) -> Result<WindowPositions, String> {
    let conn = state.0.lock().unwrap();
    let json: Option<String> = conn
        .query_row(
            "SELECT window_positions FROM app_settings WHERE id = 1",
            [],
            |r| r.get::<_, Option<String>>(0),
        )
        .unwrap_or(None);
    let positions = match json {
        Some(s) if !s.trim().is_empty() => serde_json::from_str(&s).unwrap_or_default(),
        _ => WindowPositions::default(),
    };
    Ok(positions)
}

/// 写入浮球/常态窗口位置（整体覆盖；前端在捕获位置时调用）
#[tauri::command]
pub fn set_window_positions(
    state: State<'_, DbState>,
    positions: WindowPositions,
) -> Result<(), String> {
    let json = serde_json::to_string(&positions).map_err(s)?;
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE app_settings SET window_positions = ?1 WHERE id = 1",
        params![json],
    )
    .map_err(s)?;
    Ok(())
}

// ============ 数据库存储位置 ============

/// 读取当前数据库文件路径 + 是否自定义位置（供设置页展示）
#[tauri::command]
pub fn get_db_storage_path(app: AppHandle) -> Result<DbStorageInfo, String> {
    let (path, is_custom) = config::effective_db_path(&app).map_err(s)?;
    Ok(DbStorageInfo {
        path: path.to_string_lossy().into_owned(),
        is_custom,
    })
}

/// 切换数据库存储文件夹：校验 →（按需）在线备份迁移 → 写 config.json → 触发重启。
/// 数据策略（用户确认）：目标文件夹已存在 weekly.db 时直接使用该文件（不复制）；
/// 否则把当前库整库备份迁移到新位置。
#[tauri::command]
pub fn set_db_storage_path(
    state: State<'_, DbState>,
    app: AppHandle,
    new_dir: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&new_dir);
    if !dir.is_absolute() {
        return Err("请选择一个绝对路径的文件夹".into());
    }
    if !dir.is_dir() {
        return Err("文件夹不存在或不是目录".into());
    }
    // 可写探测：写一个临时文件再删
    let probe = dir.join(".weekly_write_probe");
    fs::write(&probe, b"").map_err(|e| format!("文件夹不可写: {}", e))?;
    let _ = fs::remove_file(&probe);

    let target_db = dir.join("weekly.db");
    if !target_db.exists() {
        // 目标不存在 → 把当前库整库迁移过去（SQLite 在线备份，事务一致快照）
        let guard = state.0.lock().unwrap();
        // 当前为 rollback 模式为 no-op；未来启用 WAL 时保证日志已落盘
        let _ = guard.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        guard
            .backup(DatabaseName::Main, &target_db, None)
            .map_err(s)?;
        drop(guard);
    }
    // 目标已存在 → 直接使用该文件（不复制）

    config::write_db_path_config(&app, Some(&dir)).map_err(s)?;
    // 用 request_restart（非 restart）：后者 -> ! 会让本命令永不返回，invoke 卡死
    app.request_restart();
    Ok(())
}

/// 恢复默认存储位置：把当前（自定义位置的）库迁回 app_data_dir/weekly.db
/// （覆盖陈旧默认文件，把数据带回家）→ 清空 db_path → 重启。
#[tauri::command]
pub fn restore_default_db_path(
    state: State<'_, DbState>,
    app: AppHandle,
) -> Result<(), String> {
    let (_, is_custom) = config::effective_db_path(&app).map_err(s)?;
    if !is_custom {
        return Err("当前已是默认位置".into()); // 同时也是自拷贝护栏
    }
    let default_db = config::default_db_path(&app).map_err(s)?;
    {
        let guard = state.0.lock().unwrap();
        let _ = guard.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        guard
            .backup(DatabaseName::Main, &default_db, None)
            .map_err(s)?;
    }
    config::write_db_path_config(&app, None).map_err(s)?;
    app.request_restart();
    Ok(())
}

// ============ 解析 / 计划 ============

/// 仅解析文本，不落库（供输入页实时预览）
#[tauri::command]
pub fn parse_plan(raw: String) -> ParsedPlan {
    parse_plan_text(&raw)
}

/// 保存周计划：解析 + 落库（upsert week + 重建 planned_tasks）
#[tauri::command]
pub fn save_week_plan(state: State<'_, DbState>, raw: String) -> Result<Week, String> {
    let parsed = parse_plan_text(&raw);
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
                    "SELECT id, week_id, COALESCE(project,''), title, sort_order, estimate_d, carried_from, done
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
                    done: row.get::<_, i64>(7)? != 0,
                })
            }).map_err(s)?;
            rows.filter_map(|r| r.ok()).collect()
        }
        None => vec![],
    };

    let mut adhoc = match week_id_opt {
        Some(wid) => {
            let mut stmt = conn
                .prepare(
                    "SELECT id, week_id, COALESCE(project,''), title, sort_order, done, created_at
                     FROM adhoc_tasks WHERE week_id = ?1 ORDER BY sort_order, created_at",
                )
                .map_err(s)?;
            let rows = stmt.query_map(params![wid], |row| {
                Ok(AdhocTask {
                    id: row.get(0)?,
                    week_id: row.get(1)?,
                    project: row.get(2)?,
                    title: row.get(3)?,
                    sort_order: row.get(4)?,
                    done: row.get::<_, i64>(5)? != 0,
                    created_at: row.get(6)?,
                })
            }).map_err(s)?;
            rows.filter_map(|r| r.ok()).collect()
        }
        None => vec![],
    };

    // 归一化历史 adhoc 任务序号：旧库迁移后仍为默认 9999，这里补上真实序号并落库（幂等）
    let legacy: Vec<usize> = adhoc
        .iter()
        .enumerate()
        .filter(|(_, t)| t.sort_order >= 9999)
        .map(|(i, _)| i)
        .collect();
    if !legacy.is_empty() {
        let max_sort = planned
            .iter()
            .map(|t| t.sort_order)
            .chain(adhoc.iter().map(|t| t.sort_order))
            .filter(|&s| s < 9999)
            .max()
            .unwrap_or(0);
        let mut next = max_sort + 1;
        for i in legacy {
            adhoc[i].sort_order = next;
            let _ = conn.execute(
                "UPDATE adhoc_tasks SET sort_order = ?1 WHERE id = ?2",
                params![next, adhoc[i].id],
            );
            next += 1;
        }
    }

    Ok(CurrentWeek { week, planned, adhoc })
}

// ============ 番茄钟 ============

/// 获取番茄钟选任务弹窗所需的任务列表（本周 planned + 本周 adhoc 去重）
///
/// P022：adhoc 仅取本周（`week_id = wid`），不再混入历史周的计划外任务——
/// 弹窗只服务「本次番茄钟选哪个任务」，历史 adhoc 与本周无关。
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
                "SELECT id, COALESCE(project,''), title FROM planned_tasks WHERE week_id = ?1 AND done = 0 ORDER BY sort_order",
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
            .prepare("SELECT id, COALESCE(project,''), title FROM adhoc_tasks WHERE week_id = ?1 AND done = 0 ORDER BY created_at DESC")
            .map_err(s)?;
        let rows2 = stmt2.query_map(params![wid], |row| {
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

/// 新建计划外任务（序号自动续编 = 本周 planned+adhoc 最大序号 + 1）
#[tauri::command]
pub fn create_adhoc_task(
    state: State<'_, DbState>,
    project: String,
    title: String,
) -> Result<AdhocTask, String> {
    let conn = state.0.lock().unwrap();
    let (week_start, _) = current_week_range();
    let week_id = ensure_week_id(&conn, &week_start)?;

    let max_sort: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM (
                SELECT sort_order FROM planned_tasks WHERE week_id = ?1
                UNION ALL
                SELECT sort_order FROM adhoc_tasks WHERE week_id = ?1
            )",
            params![week_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let sort_order = max_sort + 1;

    conn.execute(
        "INSERT INTO adhoc_tasks (week_id, project, title, sort_order, done) VALUES (?1, ?2, ?3, ?4, 0)",
        params![week_id, project, title, sort_order],
    )
    .map_err(s)?;
    let id = conn.last_insert_rowid();

    let task = conn
        .query_row(
            "SELECT id, week_id, COALESCE(project,''), title, sort_order, done, created_at
             FROM adhoc_tasks WHERE id = ?1",
            params![id],
            |row| {
                Ok(AdhocTask {
                    id: row.get(0)?,
                    week_id: row.get(1)?,
                    project: row.get(2)?,
                    title: row.get(3)?,
                    sort_order: row.get(4)?,
                    done: row.get::<_, i64>(5)? != 0,
                    created_at: row.get(6)?,
                })
            },
        )
        .map_err(s)?;
    Ok(task)
}

/// 更新单个任务的内容 / 序号 / 完成态（计划内或计划外通用）
#[tauri::command]
pub fn update_task(
    state: State<'_, DbState>,
    source: String,
    id: i64,
    title: String,
    sort_order: i64,
    done: bool,
) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let done_i: i64 = if done { 1 } else { 0 };
    match source.as_str() {
        "planned" => conn
            .execute(
                "UPDATE planned_tasks SET title = ?1, sort_order = ?2, done = ?3 WHERE id = ?4",
                params![title, sort_order, done_i, id],
            )
            .map_err(s)?,
        "adhoc" => conn
            .execute(
                "UPDATE adhoc_tasks SET title = ?1, sort_order = ?2, done = ?3 WHERE id = ?4",
                params![title, sort_order, done_i, id],
            )
            .map_err(s)?,
        other => return Err(format!("未知任务来源: {}", other)),
    };
    Ok(())
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
pub fn get_report_data(
    state: State<'_, DbState>,
    week_id: i64,
) -> Result<ReportData, String> {
    let conn = state.0.lock().unwrap();
    build_report_data(&conn, week_id).map_err(s)
}

/// 周五勾选顺延：
/// - 勾选的任务标记 plan_next_monday=1（决定其在周报中显示为「计划下周一完成」）
///
/// 设计（P017）：顺延任务**仅作为本周周报的「下周计划」项呈现**——由 render_markdown
/// 从本周 in_progress/not_started 任务状态推导，不读下周 planned_tasks。此处**不再**把
/// 未完成任务预填进下周 planned_tasks：下周二由用户用新文本解析强制生成本周任务，
/// 避免上周未完成项把新一周的输入入口顶成「已有计划」表格态而无法重录。
#[tauri::command]
pub fn carry_over_tasks(
    state: State<'_, DbState>,
    req: CarryOverRequest,
) -> Result<(), String> {
    let conn = state.0.lock().unwrap();

    // 勾选的任务标记 plan_next_monday=1（用于周报状态判定）
    for id in &req.next_monday_task_ids {
        conn.execute(
            "UPDATE planned_tasks SET plan_next_monday = 1 WHERE id = ?1 AND week_id = ?2",
            params![id, req.week_id],
        )
        .map_err(s)?;
    }

    Ok(())
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

/// 【dev】清空本周全部数据：删除本周 planned_tasks / adhoc_tasks / pomodoro_sessions，并置空 plan_raw。
/// 回到无计划输入态，便于反复调试。番茄钟记录一并清掉，得到干净起点。
#[tauri::command]
pub fn clear_week_data(state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let (week_start, _) = current_week_range();
    let week_id_opt: Option<i64> = conn
        .query_row(
            "SELECT id FROM weeks WHERE week_start = ?1",
            params![week_start],
            |row| row.get(0),
        )
        .ok();
    let Some(wid) = week_id_opt else {
        return Ok(());
    };
    conn.execute("DELETE FROM pomodoro_sessions WHERE week_id = ?1", params![wid])
        .map_err(s)?;
    conn.execute("DELETE FROM adhoc_tasks WHERE week_id = ?1", params![wid])
        .map_err(s)?;
    conn.execute("DELETE FROM planned_tasks WHERE week_id = ?1", params![wid])
        .map_err(s)?;
    conn.execute("UPDATE weeks SET plan_raw = NULL WHERE id = ?1", params![wid])
        .map_err(s)?;
    Ok(())
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
