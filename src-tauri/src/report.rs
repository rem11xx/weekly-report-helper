//! 周报生成：状态判定、分钟→d 换算、Markdown 渲染

use rusqlite::{params, Connection};

use crate::models::{ReportData, ReportTask, TaskStatus, Week};

/// 番茄钟分钟 → d 换算
/// 规则：1d = 480min；小数部分向上取到 [0.1, 0.2, 0.25, 0.5, 0.75]；整数部分按 1d 累加
pub fn minutes_to_days(minutes: i64) -> f64 {
    if minutes <= 0 {
        return 0.0;
    }
    let full_days = minutes / 480;
    let remainder_min = minutes % 480;
    let remainder_d = remainder_min as f64 / 480.0;
    let rounded_frac = round_fraction_up(remainder_d);
    full_days as f64 + rounded_frac
}

/// 小数部分向上取到最近档位 [0.1, 0.2, 0.25, 0.5, 0.75]
/// 整数分钟余数正好为 0 时返回 0.0
fn round_fraction_up(frac_d: f64) -> f64 {
    if frac_d <= 0.0 {
        return 0.0;
    }
    const TIERS: [f64; 5] = [0.1, 0.2, 0.25, 0.5, 0.75];
    // 若超过最大档位（0.75），向上进位到 1.0
    for tier in TIERS.iter() {
        if frac_d <= *tier + 1e-9 {
            return *tier;
        }
    }
    1.0 // 余数超过 0.75d（如 0.8d）进位为完整 1d
}

/// 格式化 d 值为字符串：小数部分为 0 显示整数，否则保留 2 位（去尾零）
/// 1.0 -> "1"；1.25 -> "1.25"；0.5 -> "0.5"
pub fn format_days(d: f64) -> String {
    if (d - d.round()).abs() < 1e-9 {
        format!("{}d", d.round() as i64)
    } else {
        // 保留最多 2 位小数，去掉尾部 0
        let s = format!("{:.2}", d);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        format!("{}d", s)
    }
}

/// 判定单个任务的状态
/// - done: 番茄钟累计 >= 预估（预估<=0 时走过番茄钟即完成）
/// - 周五勾选 plan_next_monday: next_monday
/// - 进行中: 累计>0 且 <预估 且未勾选
/// - 未开始: 累计=0 且未勾选
pub fn determine_status(
    actual_min: i64,
    estimate_d: f64,
    plan_next_monday: bool,
) -> TaskStatus {
    let estimate_min = (estimate_d * 480.0).round() as i64;

    // 已完成判定
    let is_done = if estimate_min <= 0 {
        actual_min > 0 // 预估=0 走过番茄钟即完成
    } else {
        actual_min >= estimate_min
    };

    if is_done {
        TaskStatus::Done
    } else if plan_next_monday {
        TaskStatus::NextMonday
    } else if actual_min > 0 {
        TaskStatus::InProgress
    } else {
        TaskStatus::NotStarted
    }
}

/// 聚合本周所有任务 + 番茄钟统计，生成 ReportData
pub fn build_report_data(conn: &Connection, week_id: i64) -> anyhow::Result<ReportData> {
    // week
    let week: Week = query_week(conn, week_id)?;

    // 番茄钟累计分钟，按 (source, task_id) 聚合
    let mut minutes_map: std::collections::HashMap<(String, i64), i64> =
        std::collections::HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT task_source, task_id, SUM(duration_min) FROM pomodoro_sessions
             WHERE week_id = ?1 AND is_break = 0
             GROUP BY task_source, task_id",
        )?;
        let rows = stmt.query_map(params![week_id], |row| {
            let source: String = row.get(0)?;
            let task_id: i64 = row.get(1)?;
            let total: i64 = row.get(2)?;
            Ok((source, task_id, total))
        })?;
        for r in rows {
            let (s, t, m) = r?;
            // task_id 为 NULL 的（休息）跳过；理论上 is_break=0 已过滤
            minutes_map.insert((s, t), m);
        }
    }

    // 计划内任务
    let mut tasks: Vec<ReportTask> = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT id, project, title, sort_order, estimate_d, plan_next_monday, carried_from, done
             FROM planned_tasks WHERE week_id = ?1",
        )?;
        let rows = stmt.query_map(params![week_id], |row| {
            let task_id: i64 = row.get(0)?;
            let estimate_d: f64 = row.get(4)?;
            let plan_next_monday: bool = row.get::<_, i64>(5)? != 0;
            let done: bool = row.get::<_, i64>(7)? != 0;
            let actual_min = *minutes_map
                .get(&("planned".to_string(), task_id))
                .unwrap_or(&0);
            let status = if done {
                TaskStatus::Done
            } else {
                determine_status(actual_min, estimate_d, plan_next_monday)
            };
            Ok(ReportTask {
                source: "planned".to_string(),
                task_id,
                project: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                title: row.get(2)?,
                sort_order: row.get(3)?,
                estimate_d,
                actual_min,
                actual_d: minutes_to_days(actual_min),
                status,
                plan_next_monday,
                carried_from: row.get(6)?,
            })
        })?;
        for r in rows {
            tasks.push(r?);
        }
    }

    // 计划外任务（adhoc）
    {
        let mut stmt = conn.prepare(
            "SELECT id, project, title, sort_order, done, created_at FROM adhoc_tasks WHERE week_id = ?1",
        )?;
        let rows = stmt.query_map(params![week_id], |row| {
            let task_id: i64 = row.get(0)?;
            let sort_order: i64 = row.get(3)?;
            let done: bool = row.get::<_, i64>(4)? != 0;
            let actual_min = *minutes_map
                .get(&("adhoc".to_string(), task_id))
                .unwrap_or(&0);
            // adhoc 任务预估=0；手动勾选完成或走过番茄钟即完成
            let status = if done {
                TaskStatus::Done
            } else {
                determine_status(actual_min, 0.0, false)
            };
            Ok(ReportTask {
                source: "adhoc".to_string(),
                task_id,
                project: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                title: row.get(2)?,
                sort_order,
                estimate_d: 0.0,
                actual_min,
                actual_d: minutes_to_days(actual_min),
                status,
                plan_next_monday: false,
                carried_from: None,
            })
        })?;
        for r in rows {
            tasks.push(r?);
        }
    }

    // 总实际用时（不含休息）
    let total_actual_d: f64 = tasks
        .iter()
        .map(|t| {
            // 计划下周一完成的按预估算；其余按实际
            match t.status {
                TaskStatus::NextMonday => t.estimate_d,
                _ => t.actual_d,
            }
        })
        .sum();

    Ok(ReportData {
        week,
        tasks,
        total_actual_d,
    })
}

fn query_week(conn: &Connection, week_id: i64) -> anyhow::Result<Week> {
    let w = conn.query_row(
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
    )?;
    Ok(w)
}

/// 渲染 Markdown 周报
pub fn render_markdown(data: &ReportData) -> String {
    let mut out = String::new();

    // 标题
    let title = format!(
        "# 工作周报（{} ~ {}）\n\n",
        data.week.week_start, data.week.week_end
    );
    out.push_str(&title);

    // 按项目分组，保留项目出现顺序（按 sort_order 最小者确定）
    // 项目顺序：以该项目第一个任务（sort_order 最小）在全局的次序为准
    let mut project_order: Vec<String> = Vec::new();
    let mut project_first_sort: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    // 先收集所有任务并按 sort_order 升序（稳定）
    let mut sorted_tasks = data.tasks.clone();
    sorted_tasks.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));

    for t in &sorted_tasks {
        let p = t.project.clone();
        let entry = project_first_sort.entry(p.clone()).or_insert(i64::MAX);
        if t.sort_order < *entry {
            *entry = t.sort_order;
        }
        if !project_order.contains(&p) {
            project_order.push(p);
        }
    }
    // 按首次出现的 sort_order 排序项目
    project_order.sort_by_key(|p| *project_first_sort.get(p).unwrap_or(&i64::MAX));

    for project in &project_order {
        let proj_tasks: Vec<&ReportTask> = sorted_tasks
            .iter()
            .filter(|t| &t.project == project)
            .collect();

        // 项目总用时：所有任务用时之和（计划下周一=预估；其余=实际；未开始=0）
        let proj_total: f64 = proj_tasks
            .iter()
            .map(|t| match t.status {
                TaskStatus::NextMonday => t.estimate_d,
                _ => t.actual_d,
            })
            .sum();

        out.push_str(&format!(
            "## {}（总用时 {}）\n",
            project,
            format_days(proj_total)
        ));

        for t in proj_tasks {
            out.push_str(&format_task_line(t));
            out.push('\n');
        }
        out.push('\n');
    }

    // 下周计划：进行中 + 未开始
    let next_week_tasks: Vec<&ReportTask> = sorted_tasks
        .iter()
        .filter(|t| matches!(t.status, TaskStatus::InProgress | TaskStatus::NotStarted))
        .collect();

    if !next_week_tasks.is_empty() {
        out.push_str("## 下周计划\n");
        for t in next_week_tasks {
            out.push_str(&format_next_week_line(t));
            out.push('\n');
        }
    }

    out.trim_end().to_string() + "\n"
}

/// 格式化本周任务行：序号.内容 [状态] 用时d
fn format_task_line(t: &ReportTask) -> String {
    let status_label = status_label(&t.status);
    let mut line = format!("{}.{} {}", t.sort_order, t.title, status_label);
    // 未开始不写用时
    if t.status != TaskStatus::NotStarted {
        let d = match t.status {
            TaskStatus::NextMonday => t.estimate_d,
            _ => t.actual_d,
        };
        line.push(' ');
        line.push_str(&format_days(d));
    }
    line
}

/// 格式化下周计划行：序号.内容 [预估]
/// - 进行中：剩余预估（estimate - actual_d）
/// - 未开始：原预估
/// - 未解析预估（estimate<=0）：不标时间
fn format_next_week_line(t: &ReportTask) -> String {
    let mut line = format!("{}.{}", t.sort_order, t.title);
    let est = match t.status {
        TaskStatus::InProgress => (t.estimate_d - t.actual_d).max(0.0),
        TaskStatus::NotStarted => t.estimate_d,
        _ => 0.0,
    };
    if est > 0.0 {
        line.push(' ');
        line.push_str(&format_days(est));
    }
    line
}

fn status_label(s: &TaskStatus) -> &'static str {
    match s {
        TaskStatus::Done => "[已完成]",
        TaskStatus::InProgress => "[进行中]",
        TaskStatus::NextMonday => "[计划下周一完成]",
        TaskStatus::NotStarted => "[未开始]",
    }
}

// ============ 单元测试 ============
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutes_to_days_basic() {
        // 480min = 1d
        assert!((minutes_to_days(480) - 1.0).abs() < 1e-9);
        // 600min = 1.25d
        assert!((minutes_to_days(600) - 1.25).abs() < 1e-9);
        // 180min = 0.375d -> 向上取 0.5d
        assert!((minutes_to_days(180) - 0.5).abs() < 1e-9);
        // 30min = 0.0625d -> 向上取 0.1d
        assert!((minutes_to_days(30) - 0.1).abs() < 1e-9);
        // 0
        assert!((minutes_to_days(0) - 0.0).abs() < 1e-9);
        // 960min = 2d
        assert!((minutes_to_days(960) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn format_days_integer_and_decimal() {
        assert_eq!(format_days(1.0), "1d");
        assert_eq!(format_days(2.0), "2d");
        assert_eq!(format_days(1.25), "1.25d");
        assert_eq!(format_days(0.5), "0.5d");
        assert_eq!(format_days(0.1), "0.1d");
        assert_eq!(format_days(1.5), "1.5d");
    }

    #[test]
    fn status_done_when_meets_estimate() {
        // 预估 0.5d = 240min；累计 240min -> done
        assert_eq!(
            determine_status(240, 0.5, false),
            TaskStatus::Done
        );
        // 累计 250min > 240 -> done
        assert_eq!(determine_status(250, 0.5, false), TaskStatus::Done);
    }

    #[test]
    fn status_in_progress_when_partial() {
        // 预估 1d=480min；累计 100min -> in_progress
        assert_eq!(
            determine_status(100, 1.0, false),
            TaskStatus::InProgress
        );
    }

    #[test]
    fn status_next_monday_when_checked_and_incomplete() {
        // 预估 1d；累计 100min；勾选 -> next_monday
        assert_eq!(
            determine_status(100, 1.0, true),
            TaskStatus::NextMonday
        );
    }

    #[test]
    fn status_not_started_when_zero() {
        assert_eq!(
            determine_status(0, 1.0, false),
            TaskStatus::NotStarted
        );
    }

    #[test]
    fn status_done_when_estimate_zero_and_has_sessions() {
        // 预估=0，走过番茄钟 -> done
        assert_eq!(determine_status(25, 0.0, false), TaskStatus::Done);
        // 预估=0，没走过 -> not_started
        assert_eq!(determine_status(0, 0.0, false), TaskStatus::NotStarted);
    }

    #[test]
    fn task_line_format_done() {
        let t = ReportTask {
            source: "planned".into(),
            task_id: 1,
            project: "P".into(),
            title: "任务A".into(),
            sort_order: 0,
            estimate_d: 0.5,
            actual_min: 240,
            actual_d: 0.5,
            status: TaskStatus::Done,
            plan_next_monday: false,
            carried_from: None,
        };
        assert_eq!(format_task_line(&t), "0.任务A [已完成] 0.5d");
    }

    #[test]
    fn task_line_format_not_started_no_duration() {
        let t = ReportTask {
            source: "planned".into(),
            task_id: 2,
            project: "P".into(),
            title: "任务B".into(),
            sort_order: 1,
            estimate_d: 0.5,
            actual_min: 0,
            actual_d: 0.0,
            status: TaskStatus::NotStarted,
            plan_next_monday: false,
            carried_from: None,
        };
        // 未开始不写用时
        assert_eq!(format_task_line(&t), "1.任务B [未开始]");
    }

    #[test]
    fn next_week_line_in_progress_shows_remaining() {
        let t = ReportTask {
            source: "planned".into(),
            task_id: 1,
            project: "P".into(),
            title: "任务A".into(),
            sort_order: 1,
            estimate_d: 1.25,
            actual_min: 480, // 1d
            actual_d: 1.0,
            status: TaskStatus::InProgress,
            plan_next_monday: false,
            carried_from: None,
        };
        // 剩余 1.25 - 1.0 = 0.25d
        assert_eq!(format_next_week_line(&t), "1.任务A 0.25d");
    }

    #[test]
    fn next_week_line_not_started_shows_estimate() {
        let t = ReportTask {
            source: "planned".into(),
            task_id: 2,
            project: "P".into(),
            title: "任务B".into(),
            sort_order: 2,
            estimate_d: 0.5,
            actual_min: 0,
            actual_d: 0.0,
            status: TaskStatus::NotStarted,
            plan_next_monday: false,
            carried_from: None,
        };
        assert_eq!(format_next_week_line(&t), "2.任务B 0.5d");
    }

    #[test]
    fn next_week_line_no_estimate_omits_time() {
        let t = ReportTask {
            source: "planned".into(),
            task_id: 3,
            project: "P".into(),
            title: "任务C".into(),
            sort_order: 3,
            estimate_d: 0.0,
            actual_min: 0,
            actual_d: 0.0,
            status: TaskStatus::NotStarted,
            plan_next_monday: false,
            carried_from: None,
        };
        assert_eq!(format_next_week_line(&t), "3.任务C");
    }

    #[test]
    fn render_markdown_full_example() {
        let week = Week {
            id: 1,
            week_start: "2024-06-11".into(),
            week_end: "2024-06-17".into(),
            plan_raw: "".into(),
            created_at: "".into(),
        };
        let tasks = vec![
            ReportTask {
                source: "planned".into(),
                task_id: 1,
                project: "玉环反走私".into(),
                title: "警综对接".into(),
                sort_order: 0,
                estimate_d: 0.5,
                actual_min: 240,
                actual_d: 0.5,
                status: TaskStatus::Done,
                plan_next_monday: false,
                carried_from: None,
            },
            ReportTask {
                source: "planned".into(),
                task_id: 2,
                project: "玉环反走私".into(),
                title: "公安网对接".into(),
                sort_order: 1,
                estimate_d: 1.25,
                actual_min: 480,
                actual_d: 1.0,
                status: TaskStatus::InProgress,
                plan_next_monday: false,
                carried_from: None,
            },
            ReportTask {
                source: "planned".into(),
                task_id: 3,
                project: "玉环反走私".into(),
                title: "人员抓拍".into(),
                sort_order: 2,
                estimate_d: 0.5,
                actual_min: 0,
                actual_d: 0.0,
                status: TaskStatus::NotStarted,
                plan_next_monday: false,
                carried_from: None,
            },
        ];
        let data = ReportData {
            week,
            tasks,
            total_actual_d: 1.5,
        };

        let md = render_markdown(&data);
        assert!(md.contains("# 工作周报（2024-06-11 ~ 2024-06-17）"));
        assert!(md.contains("## 玉环反走私（总用时 1.5d）"));
        assert!(md.contains("0.警综对接 [已完成] 0.5d"));
        assert!(md.contains("1.公安网对接 [进行中] 1d"));
        assert!(md.contains("2.人员抓拍 [未开始]"));
        assert!(md.contains("## 下周计划"));
        assert!(md.contains("1.公安网对接 0.25d"));
        assert!(md.contains("2.人员抓拍 0.5d"));
    }
}
