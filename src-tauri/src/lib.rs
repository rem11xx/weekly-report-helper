//! 工作周报助手 - Tauri 后端入口

pub mod commands;
pub mod db;
pub mod models;
pub mod parser;
pub mod report;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            db::init_db(app.handle())?;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
