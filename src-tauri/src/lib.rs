//! 工作周报助手 - Tauri 后端入口

pub mod commands;
pub mod config;
pub mod db;
pub mod models;
pub mod parser;
pub mod report;

use tauri::Manager;

use crate::db::DbState;

/// 逻辑坐标 (x, y) 是否落在某个可用显示器内。
/// 跨重启恢复常态窗口位置时调用，防多显示器拔除外接屏后窗口落到屏外不可见。
/// 读不到显示器信息则信任，交给 OS 钳制。
fn position_on_screen(window: &tauri::WebviewWindow, x: f64, y: f64) -> bool {
    let factor = window.scale_factor().unwrap_or(1.0);
    let (px, py) = (x * factor, y * factor);
    match window.available_monitors() {
        Ok(mons) if !mons.is_empty() => mons.iter().any(|m| {
            let p = m.position();
            let s = m.size();
            let (mx, my) = (p.x as f64, p.y as f64);
            let (mw, mh) = (s.width as f64, s.height as f64);
            px >= mx && px < mx + mw && py >= my && py < my + mh
        }),
        _ => true,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            db::init_db(app.handle())?;

            // 启动时按持久化设置应用窗口置顶 + 恢复上次常态窗口位置（首帧前生效，无闪烁）
            let state = app.state::<DbState>();
            let (on_top, positions_json): (i64, Option<String>) = {
                let conn = state.0.lock().unwrap();
                conn.query_row(
                    "SELECT always_on_top, window_positions FROM app_settings WHERE id = 1",
                    [],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)),
                )
                .unwrap_or((0, None))
            };
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(on_top != 0);
                // 跨重启保留常态窗口位置：解析上次位置，校验仍落在某屏内再恢复，防 off-screen
                if let Some(json) = positions_json {
                    if let Ok(positions) =
                        serde_json::from_str::<crate::models::WindowPositions>(&json)
                    {
                        if let Some(normal) = positions.normal {
                            if position_on_screen(&window, normal.x, normal.y) {
                                let _ =
                                    window.set_position(tauri::LogicalPosition::new(normal.x, normal.y));
                            }
                        }
                    }
                }
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
            commands::get_window_positions,
            commands::set_window_positions,
            commands::get_db_storage_path,
            commands::set_db_storage_path,
            commands::restore_default_db_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
