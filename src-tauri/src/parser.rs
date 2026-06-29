//! 周计划文本解析器
//!
//! 解析规则（详见计划第三节）：
//! 1. 空行分块，每块首行=项目名
//! 2. 块内其余每行=一个任务
//! 3. 行首数字序号 -> sort_order；无序号的全局最大序号+1 续编
//! 4. 行尾预估时长：支持 0.5d / -0.5d / 0.25 / 1,25d（逗号小数点）
//! 5. 1d = 8h = 480min；解析失败 estimate_d=0

use regex::Regex;

use crate::models::{ParsedPlan, ParsedTask};

/// 解析过程中的中间任务结构（行级解析结果，序号待续编）
struct PartialTask {
    project: String,
    title: String,
    estimate_d: f64,
    sort_order: Option<i64>, // None=无序号，待续编
    raw: String,
}

/// 行首序号正则：^\s*(\d+)\s*[.、)]?\s*
fn leading_index_re() -> Regex {
    Regex::new(r"^\s*(\d+)\s*[.、)]?\s*").unwrap()
}

/// 行尾预估时长正则：
/// 允许前导分隔符 [-—/空格]，数字支持整数或逗号/点小数，可选 d 后缀
fn trailing_estimate_re() -> Regex {
    Regex::new(r"[\s\-—]*([0-9]+(?:[.,][0-9]+)?)\s*[dD]?\s*$").unwrap()
}

/// 解析主入口
pub fn parse_plan(raw: &str) -> ParsedPlan {
    let (week_start, week_end) = current_week_range();

    let mut tasks: Vec<ParsedTask> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // 1. 按空行分块
    let blocks = split_blocks_owned(raw);

    // 2. 第一遍：解析所有任务，记录已出现的最大序号
    let mut max_sort: i64 = -1;

    let mut partials: Vec<PartialTask> = Vec::new();

    for block in &blocks {
        let trimmed = block.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut lines = trimmed.lines().filter(|l| !l.trim().is_empty());
        let project = match lines.next() {
            Some(p) => p.trim().to_string(),
            None => continue,
        };

        for line in lines {
            let raw_line = line.trim().to_string();
            if raw_line.is_empty() {
                continue;
            }
            match parse_one_line(&raw_line, &project) {
                Ok(p) => {
                    if let Some(s) = p.sort_order {
                        if s > max_sort {
                            max_sort = s;
                        }
                    }
                    partials.push(p);
                }
                Err(e) => errors.push(format!("「{}」: {}", raw_line, e)),
            }
        }
    }

    // 3. 第二遍：无序号任务接 max_sort+1 续编
    let mut next_seq = max_sort + 1;
    for p in partials {
        let sort_order = match p.sort_order {
            Some(s) => s,
            None => {
                let s = next_seq;
                next_seq += 1;
                s
            }
        };
        tasks.push(ParsedTask {
            project: p.project,
            title: p.title,
            sort_order,
            estimate_d: p.estimate_d,
            raw: p.raw,
        });
    }

    // 4. 按 sort_order 升序稳定排序（序号相同则保持输入顺序）
    tasks.sort_by_key(|t| t.sort_order);

    ParsedPlan {
        week_start,
        week_end,
        tasks,
        errors,
    }
}

/// 解析单行任务
fn parse_one_line(line: &str, project: &str) -> Result<PartialTask, String> {
    let mut work = line.to_string();

    // 提取序号
    let leading = leading_index_re();
    let sort_order = if let Some(caps) = leading.captures(&work) {
        let n: i64 = caps
            .get(1)
            .unwrap()
            .as_str()
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        // 去掉序号前缀部分
        let mat = caps.get(0).unwrap();
        work = work[mat.end()..].to_string();
        Some(n)
    } else {
        None
    };

    // 提取预估时长
    let trailing = trailing_estimate_re();
    let estimate_d = if let Some(caps) = trailing.captures(&work) {
        let num_str = caps.get(1).unwrap().as_str().replace(',', ".");
        let days: f64 = num_str
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let mat = caps.get(0).unwrap();
        work = work[..work.len() - mat.as_str().len()].trim_end().to_string();
        days
    } else {
        0.0
    };

    let title = work.trim().to_string();
    if title.is_empty() {
        return Err("任务内容为空".to_string());
    }

    Ok(PartialTask {
        project: project.to_string(),
        title,
        estimate_d: round_estimate(estimate_d),
        sort_order,
        raw: line.to_string(),
    })
}

/// 预估时长规范化：保留两位小数，避免浮点误差（如 1.25d）
fn round_estimate(d: f64) -> f64 {
    (d * 100.0).round() / 100.0
}

/// 按空行分块（拥有 String 版本，避免生命周期问题）
pub fn split_blocks_owned(raw: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in raw.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

/// 模拟当前时间（dev 测试用）。
///
/// 进程内全局可覆盖的"当前时间"。默认为 None → 返回真实的 `Local::now()`；
/// 由 `set_mock_now` / `clear_mock_now` 命令控制，供前端 dev 面板模拟
/// 周二提醒窗口 / 周五报告 / 周一归属上周等时间点，无需改动系统时钟。
static MOCK_NOW: std::sync::Mutex<Option<chrono::DateTime<chrono::Local>>> =
    std::sync::Mutex::new(None);

/// 取"当前时间"：若已注入 mock 则返回 mock，否则返回真实本地时间。
pub fn mock_now() -> chrono::DateTime<chrono::Local> {
    let guard = MOCK_NOW.lock().unwrap();
    guard.unwrap_or_else(|| chrono::Local::now())
}

/// 注入模拟时间（前端 dev 面板调用）。`iso` 形如 `2026-06-17T13:00`。
pub fn set_mock_now(iso: &str) -> Result<(), String> {
    use chrono::TimeZone;
    let ndt = chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M")
        .map_err(|e| format!("时间格式错误（应为 YYYY-MM-DDTHH:MM）: {e}"))?;
    let dt = chrono::Local
        .from_local_datetime(&ndt)
        .single()
        .ok_or_else(|| "模拟时间在本地时区存在歧义".to_string())?;
    *MOCK_NOW.lock().unwrap() = Some(dt);
    Ok(())
}

/// 清除模拟时间，恢复真实系统时间。
pub fn clear_mock_now() {
    *MOCK_NOW.lock().unwrap() = None;
}

/// 计算当前工作周的起止（周二 ~ 下周一）
/// 逻辑：今天若为周一，则本周=本周二(明天)~下周一(7天后)？不对。
/// 工作周定义为 周二开始 到 下周一结束。
/// 约定：若今天是周一，归属于"上周"（即上个周二开始的周），因为周一还在收尾上周任务。
pub fn current_week_range() -> (String, String) {
    use chrono::{Datelike, Duration, NaiveDate};

    let today: NaiveDate = mock_now().date_naive();
    let weekday = today.weekday().num_days_from_monday() as i64; // 周一=0, 周日=6

    // 周二开始的工作周
    // 如果今天是周一(0)：归属于上个周二开始的周 -> start = today - 6
    // 如果今天是周二(1)：start = today
    // ...
    // 如果今天是周日(6)：start = today - 5
    let start_offset = if weekday == 0 {
        -6 // 周一归上周
    } else {
        -(weekday - 1) // 周二=0, 周三=-1...
    };
    let week_start = today + Duration::days(start_offset);
    let week_end = week_start + Duration::days(6); // 下周一

    (
        week_start.format("%Y-%m-%d").to_string(),
        week_end.format("%Y-%m-%d").to_string(),
    )
}

// ============ 单元测试 ============
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "玉环反走私
0、警综对接-0.5d
1. 公安网对接国标平台线上验证、实时画面，回放预览开发及联调 1,25d
2、人员抓拍轨迹查询（基于台州市的接口，罗喜写过接口）-0.5d
3、大陈票务功能：单表展示-0.25
4、船舶修造数线上验证 0.5d
5、涉海案件及涉海警情模块：数据对接和功能模块-前端吉宇森 1.5d

省反走私
经费管理调整 0.5d

数据协同
线上问题处理 0.25d";

    #[test]
    fn parses_all_three_projects() {
        let plan = parse_plan(SAMPLE);
        let projects: Vec<&String> = plan.tasks.iter().map(|t| &t.project).collect();
        assert!(plan.errors.is_empty(), "errors: {:?}", plan.errors);
        assert_eq!(plan.tasks.len(), 8);
        // 项目分布：玉环反走私×6, 省反走私×1, 数据协同×1
        assert_eq!(projects.iter().filter(|p| ***p == "玉环反走私").count(), 6);
        assert_eq!(projects.iter().filter(|p| ***p == "省反走私").count(), 1);
        assert_eq!(projects.iter().filter(|p| ***p == "数据协同").count(), 1);
    }

    #[test]
    fn parses_estimates_with_various_formats() {
        let plan = parse_plan(SAMPLE);
        let by_title: std::collections::HashMap<&String, &ParsedTask> =
            plan.tasks.iter().map(|t| (&t.title, t)).collect();

        assert_eq!(by_title.get(&"警综对接".to_string()).unwrap().estimate_d, 0.5);
        // 1,25d -> 1.25
        assert_eq!(
            by_title
                .get(&"公安网对接国标平台线上验证、实时画面，回放预览开发及联调".to_string())
                .unwrap()
                .estimate_d,
            1.25
        );
        assert_eq!(
            by_title
                .get(&"人员抓拍轨迹查询（基于台州市的接口，罗喜写过接口）".to_string())
                .unwrap()
                .estimate_d,
            0.5
        );
        // -0.25 无 d 后缀
        assert_eq!(
            by_title.get(&"大陈票务功能：单表展示".to_string()).unwrap().estimate_d,
            0.25
        );
        assert_eq!(
            by_title.get(&"船舶修造数线上验证".to_string()).unwrap().estimate_d,
            0.5
        );
        assert_eq!(
            by_title
                .get(&"涉海案件及涉海警情模块：数据对接和功能模块-前端吉宇森".to_string())
                .unwrap()
                .estimate_d,
            1.5
        );
        assert_eq!(
            by_title.get(&"经费管理调整".to_string()).unwrap().estimate_d,
            0.5
        );
        assert_eq!(
            by_title.get(&"线上问题处理".to_string()).unwrap().estimate_d,
            0.25
        );
    }

    #[test]
    fn indexes_assigned_for_numbered_lines() {
        let plan = parse_plan(SAMPLE);
        let by_title: std::collections::HashMap<&String, &ParsedTask> =
            plan.tasks.iter().map(|t| (&t.title, t)).collect();

        assert_eq!(by_title.get(&"警综对接".to_string()).unwrap().sort_order, 0);
        assert_eq!(
            by_title
                .get(&"公安网对接国标平台线上验证、实时画面，回放预览开发及联调".to_string())
                .unwrap()
                .sort_order,
            1
        );
    }

    #[test]
    fn unnumbered_tasks_get_continued_global_sequence() {
        let plan = parse_plan(SAMPLE);
        // 已有最大序号=5（玉环反走私最后一条），无序号任务应续编为 6, 7
        let seqs: Vec<i64> = plan.tasks.iter().map(|t| t.sort_order).collect();
        assert!(seqs.contains(&6), "经费管理调整应为 6, got {:?}", seqs);
        assert!(seqs.contains(&7), "线上问题处理应为 7, got {:?}", seqs);

        // 经费管理调整 -> 6
        let jf = plan
            .tasks
            .iter()
            .find(|t| t.title == "经费管理调整")
            .unwrap();
        assert_eq!(jf.sort_order, 6);
        // 线上问题处理 -> 7
        let xs = plan
            .tasks
            .iter()
            .find(|t| t.title == "线上问题处理")
            .unwrap();
        assert_eq!(xs.sort_order, 7);
    }

    #[test]
    fn empty_input_produces_empty_tasks() {
        let plan = parse_plan("");
        assert!(plan.tasks.is_empty());
        assert!(plan.errors.is_empty());
    }

    #[test]
    fn handles_no_estimate_line() {
        let plan = parse_plan("项目A\n纯任务无时长\n");
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].estimate_d, 0.0);
        assert_eq!(plan.tasks[0].sort_order, 0); // 无序号 -> 续编从 0 开始
    }

    #[test]
    fn splits_blocks_correctly() {
        let blocks = split_blocks_owned("a\nb\n\nc\n\n\nd");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0], "a\nb");
        assert_eq!(blocks[1], "c");
        assert_eq!(blocks[2], "d");
    }

    #[test]
    fn week_range_tuesday_starts_week() {
        use chrono::{Datelike, Duration, NaiveDate};
        // 模拟周三 2024-06-12 -> 本周二 06-11 ~ 下周一 06-17
        let wed = NaiveDate::from_ymd_opt(2024, 6, 12).unwrap();
        // 直接测试核心计算
        let weekday = wed.weekday().num_days_from_monday() as i64; // 2
        let start_offset = -(weekday - 1); // -1
        let start = wed + Duration::days(start_offset);
        let end = start + Duration::days(6);
        assert_eq!(start.format("%Y-%m-%d").to_string(), "2024-06-11");
        assert_eq!(end.format("%Y-%m-%d").to_string(), "2024-06-17");
    }

    #[test]
    fn estimate_eaten_when_adjacent_to_cjk() {
        // 行尾紧贴中文的「1d」应被当作预估时长吃掉：标题截断为「基层治理对接-本周暂排」、estimate=1d。
        // 「1」前是汉字「排」、无分隔符，原正则用 *（允许零分隔符）故仍匹配。
        let plan = parse_plan("项目A\n5、基层治理对接-本周暂排1d");
        assert_eq!(plan.tasks.len(), 1);
        let t = &plan.tasks[0];
        assert_eq!(t.title, "基层治理对接-本周暂排");
        assert_eq!(t.estimate_d, 1.0);
    }

    #[test]
    fn full_real_user_input_parses_cleanly() {
        // 用户实际粘贴的完整文本（含两个项目、带预估/不带预估/紧贴中文 1d 的混合行）。
        let raw = "省反走私
1、省反走私系统初始化缉私局的用户体系（美亚线下提供），完成和美亚的单点登录（从美亚系统跳转过来）及剩余警综平台的对接工作

玉环反走私
1、初始化有问题的区域管控围栏和初始化玉环船舶港口数据（用于船舶画像和大屏上的停留）-0.75d
2、配合凌琦测试和优化船舶图标时效性问题-0.25d
3、大华光电设备对接，207光电持续追踪效果优化（基于相同AIS和雷达tarid，位置变化后光电引导调整位置）-1.5d
4、涉海警情和涉海行政案件模块模块联调并完成上线-0.5d
5、基层治理对接-本周暂排1d";
        let plan = parse_plan(raw);
        assert!(plan.errors.is_empty(), "errors: {:?}", plan.errors);
        assert_eq!(plan.tasks.len(), 6);
        // 紧贴中文的 1d 应被当预估吃掉：标题截断、estimate=1d（与既有 0.75d/0.25d 等行同规则）
        let line5 = plan
            .tasks
            .iter()
            .find(|t| t.title.contains("基层治理"))
            .unwrap();
        assert_eq!(line5.title, "基层治理对接-本周暂排");
        assert_eq!(line5.estimate_d, 1.0);
    }
}
