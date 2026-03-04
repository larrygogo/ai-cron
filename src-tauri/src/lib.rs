mod commands;
mod db;
mod models;
mod scheduler;
mod tray;
mod webhook;

use commands::{
    ai_parse::parse_nl_to_task,
    runs::{cleanup_old_runs, delete_runs_for_task, get_all_runs, get_run, get_runs},
    scheduler::preview_next_runs,
    tasks::{create_task, delete_task, get_task, get_tasks, set_task_enabled, update_task},
    tools::{detect_tools, get_settings, update_settings},
    runner::{trigger_task_now, kill_run},
};
use db::DbConn;
use scheduler::engine::SchedulerState;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize logger
            env_logger::init();

            // Get app data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir")
                .to_string_lossy()
                .to_string();

            // Ensure directory exists
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data dir");

            // Initialize database
            let db_conn = db::init_db(&app_data_dir).expect("Failed to initialize database");
            app.manage(db_conn);

            // Create shared DB connection for scheduler/runner
            let db_arc = Arc::new(DbConn(std::sync::Mutex::new(
                rusqlite::Connection::open(format!("{}/ai-cron.db", app_data_dir))
                    .expect("Second DB connection failed"),
            )));
            db_arc.0.lock().unwrap()
                .execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
                .ok();
            app.manage(db_arc.clone());

            // Initialize and start scheduler
            let app_handle = app.handle().clone();
            let db_for_scheduler = db_arc.clone();

            tauri::async_runtime::spawn(async move {
                match SchedulerState::new().await {
                    Ok(scheduler_state) => {
                        if let Err(e) = scheduler_state.start().await {
                            log::error!("Failed to start scheduler: {}", e);
                            return;
                        }
                        let scheduler_arc = Arc::new(scheduler_state);

                        // Load existing tasks
                        scheduler_arc
                            .load_tasks(db_for_scheduler, app_handle.clone())
                            .await;
                        log::info!("Scheduler started successfully");

                        // Register scheduler as managed state
                        app_handle.manage(scheduler_arc);
                    }
                    Err(e) => {
                        log::error!("Failed to create scheduler: {}", e);
                    }
                }
            });

            // Setup system tray
            tray::setup_tray(app.handle())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Tasks
            get_tasks,
            get_task,
            create_task,
            update_task,
            delete_task,
            set_task_enabled,
            // Runs
            get_runs,
            get_all_runs,
            get_run,
            delete_runs_for_task,
            cleanup_old_runs,
            // Runner
            trigger_task_now,
            kill_run,
            // Scheduler
            preview_next_runs,
            // Tools & Settings
            detect_tools,
            get_settings,
            update_settings,
            // AI Parse
            parse_nl_to_task,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
