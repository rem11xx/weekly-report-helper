use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

/// 全局数据库连接（Mutex 保证线程安全）
pub struct DbState(pub Mutex<Connection>);

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS weeks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_start DATE NOT NULL,
    week_end   DATE NOT NULL,
    plan_raw   TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS planned_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_id INTEGER NOT NULL,
    project    TEXT,
    title      TEXT NOT NULL,
    sort_order INTEGER DEFAULT 9999,
    estimate_d REAL DEFAULT 0,
    carried_from INTEGER,
    plan_next_monday INTEGER DEFAULT 0,
    done INTEGER DEFAULT 0,
    FOREIGN KEY (week_id) REFERENCES weeks(id)
);

CREATE TABLE IF NOT EXISTS adhoc_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_id INTEGER NOT NULL,
    project TEXT,
    title   TEXT NOT NULL,
    sort_order INTEGER DEFAULT 9999,
    done INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (week_id) REFERENCES weeks(id)
);

CREATE TABLE IF NOT EXISTS pomodoro_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_id     INTEGER NOT NULL,
    task_source TEXT NOT NULL,
    task_id     INTEGER,
    started_at  TIMESTAMP NOT NULL,
    ended_at    TIMESTAMP NOT NULL,
    duration_min INTEGER NOT NULL,
    is_break    BOOLEAN DEFAULT 0,
    FOREIGN KEY (week_id) REFERENCES weeks(id)
);

CREATE TABLE IF NOT EXISTS app_settings (
    -- 单行配置表：CHECK 约束保证仅一行（id 恒为 1）
    id INTEGER PRIMARY KEY CHECK (id = 1),
    always_on_top INTEGER DEFAULT 0 NOT NULL
);
";

/// 初始化数据库：建表 + 注册到 Tauri 状态
pub fn init_db(app: &AppHandle) -> Result<()> {
    // 默认 app_data_dir 始终创建：config.json 与（未自定义时的）默认 weekly.db 都住这里
    let app_data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&app_data_dir)?;

    // 解析生效的 DB 路径：自定义目录（来自 config.json）或默认 app_data_dir。
    // 未配置 / 配置非法时回落默认 —— 与引入本功能前行为一致。
    let (db_path, _is_custom) = crate::config::effective_db_path(app)?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?; // 防御：自定义目录理论上已存在（读时校验过）
    }

    let conn = Connection::open(db_path)?;
    conn.execute_batch(SCHEMA)?;

    // 应用全局设置：保证 app_settings 有且仅有一行默认值（旧库/新库幂等）
    conn.execute(
        "INSERT OR IGNORE INTO app_settings (id, always_on_top) VALUES (1, 0)",
        [],
    )?;

    // 幂等迁移：为旧库补列（全新库已由 SCHEMA 建好，这里跳过）
    add_column_if_missing(&conn, "planned_tasks", "done", "INTEGER DEFAULT 0")?;
    add_column_if_missing(&conn, "adhoc_tasks", "sort_order", "INTEGER DEFAULT 9999")?;
    add_column_if_missing(&conn, "adhoc_tasks", "done", "INTEGER DEFAULT 0")?;

    app.manage(DbState(Mutex::new(conn)));
    Ok(())
}

/// 幂等加列：若 `table` 中不存在 `column` 则 ALTER ADD，已存在则跳过。
fn add_column_if_missing(conn: &Connection, table: &str, column: &str, def: &str) -> Result<()> {
    let pragma = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&pragma)?;
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    if !cols.iter().any(|c| c == column) {
        let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, def);
        conn.execute(&sql, [])?;
    }
    Ok(())
}
