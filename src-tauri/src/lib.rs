//! 工作周报助手 - Tauri 后端入口

pub mod commands;
pub mod config;
pub mod db;
pub mod models;
pub mod parser;
pub mod report;

use tauri::Manager;

use crate::db::DbState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            db::init_db(app.handle())?;

            // 启动时按持久化设置应用窗口置顶（首帧前生效，无闪烁）
            let state = app.state::<DbState>();
            let on_top: i64 = {
                let conn = state.0.lock().unwrap();
                conn.query_row(
                    "SELECT always_on_top FROM app_settings WHERE id = 1",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0)
            };
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(on_top != 0);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::parse_plan,
            commands::save_week_plan,
            commands::get_current_week,
            commands::get_task_options,
            commands::list_projects,
            commands::create_adhoc_task,
            commands::update_task,
            commands::record_session,
            commands::list_sessions,
            commands::get_report_data,
            commands::carry_over_tasks,
            commands::render_report_markdown,
            commands::save_report_file,
            commands::needs_plan_reminder,
            commands::clear_week_data,
            commands::set_mock_now,
            commands::clear_mock_now,
            commands::get_always_on_top,
            commands::set_always_on_top,
            commands::get_focus_enters_mini,
            commands::set_focus_enters_mini,
            commands::get_db_storage_path,
            commands::set_db_storage_path,
            commands::restore_default_db_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
