//! 引导配置：存放「数据库存储位置」等需要在打开 DB 之前就读到的设置。
//!
//! 为何独立于 `app_settings` 表：`init_db` 要在打开 SQLite 之前就知道 `weekly.db`
//! 放在哪，而 `app_settings` 本身就在该库里 —— 鸡生蛋。故把这类引导配置放进
//! 一个位于默认 `app_data_dir` 下的 `config.json`（该目录始终可达），DB 路径字段
//! 为绝对目录；`None`/空/非法值时回落到默认 `app_data_dir/weekly.db`。

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const CONFIG_FILE: &str = "config.json";
const DB_FILENAME: &str = "weekly.db";

#[derive(Default, Serialize, Deserialize)]
struct BootstrapConfig {
    /// 自定义 DB 所在目录（绝对路径）；None / 空 => 默认位置
    #[serde(default)]
    db_path: Option<String>,
}

fn config_file_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join(CONFIG_FILE))
}

/// 默认位置的 DB 文件路径：`app_data_dir/weekly.db`
pub fn default_db_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join(DB_FILENAME))
}

/// 读取自定义 DB 目录。以下情况一律返回 `None`（回落默认，启动永不崩）：
/// config.json 缺失 / JSON 损坏 / 字段空 / 值非绝对路径或目录不存在。
/// 纯读，不会修复或写入。
pub fn read_db_path_config(app: &AppHandle) -> Option<PathBuf> {
    let path = config_file_path(app).ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let cfg: BootstrapConfig = serde_json::from_str(&content).ok()?;
    cfg.db_path
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && p.is_dir())
}

/// 写入（或清空）自定义 DB 目录。懒创建 `config.json`。
/// 传 `None` 写 `{"db_path": null}`（始终可解析）。
pub fn write_db_path_config(app: &AppHandle, custom_dir: Option<&Path>) -> Result<()> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;

    let cfg = BootstrapConfig {
        db_path: custom_dir.map(|p| p.to_string_lossy().into_owned()),
    };
    let json = serde_json::to_string_pretty(&cfg)?;
    std::fs::write(app_data_dir.join(CONFIG_FILE), json)?;
    Ok(())
}

/// 解析当前生效的 DB 文件路径 + 是否自定义位置。
/// - 自定义目录存在 => `(dir/weekly.db, true)`
/// - 否则 => `(app_data_dir/weekly.db, false)`（默认行为，与未引入本功能前一致）
pub fn effective_db_path(app: &AppHandle) -> Result<(PathBuf, bool)> {
    match read_db_path_config(app) {
        Some(dir) => Ok((dir.join(DB_FILENAME), true)),
        None => Ok((default_db_path(app)?, false)),
    }
}
