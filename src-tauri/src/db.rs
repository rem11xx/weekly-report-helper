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
";

/// 初始化数据库：建表 + 注册到 Tauri 状态
pub fn init_db(app: &AppHandle) -> Result<()> {
    let app_data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("weekly.db");
    let conn = Connection::open(db_path)?;
    conn.execute_batch(SCHEMA)?;

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
