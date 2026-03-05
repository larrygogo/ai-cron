mod commands;
mod db;
mod mcp;
mod models;
mod scheduler;
mod tray;
mod webhook;

use commands::{
    ai_parse::parse_nl_to_task,
    runs::{cleanup_old_runs, delete_runs_for_task, get_all_runs, get_run, get_runs},
    scheduler::preview_next_runs,
    tasks::{create_task, delete_task, generate_plan, get_task, get_tasks, set_task_enabled, update_plan, update_task},
    tools::{detect_tools, get_mcp_status, get_settings, get_system_timezone, repair_mcp_config, update_settings},
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
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Initialize logger
            env_logger::init();

            // Windows: remove native decorations at runtime
            // (tauri.conf.json keeps decorations:true for macOS Overlay titlebar)
            #[cfg(target_os = "windows")]
            {
                let window = app.get_webview_window("main").unwrap();
                window.set_decorations(false).unwrap();
            }

            // Get app data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir")
                .to_string_lossy()
                .to_string();

            // Ensure directory exists
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data dir");

            // Initialize database — primary connection (runs migrations)
            let db_conn = db::init_db(&app_data_dir).expect("Failed to initialize database");

            // Clean up zombie runs (status='running' from previous session)
            {
                let conn = db_conn.0.lock().unwrap();
                let now = chrono::Utc::now().to_rfc3339();
                let cleaned = conn.execute(
                    "UPDATE runs SET status='killed', ended_at=?1 WHERE status='running'",
                    rusqlite::params![now],
                ).unwrap_or(0);
                if cleaned > 0 {
                    log::info!("Cleaned up {} zombie runs from previous session", cleaned);
                }
            }

            app.manage(db_conn);

            // Shared DB connection for scheduler/runner (WAL mode allows concurrent readers)
            let db_arc = Arc::new(DbConn(std::sync::Mutex::new({
                let conn = rusqlite::Connection::open(format!("{}/ai-cron.db", app_data_dir))
                    .expect("Shared DB connection failed");
                conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
                    .expect("Failed to set PRAGMAs on shared connection");
                conn
            })));
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
                            .load_tasks(db_for_scheduler.clone(), app_handle.clone())
                            .await;
                        log::info!("Scheduler started successfully");

                        // Register scheduler as managed state
                        app_handle.manage(scheduler_arc.clone());

                        // Always start MCP server with dynamic port
                        let app_data_dir_clone = app_handle
                            .path()
                            .app_data_dir()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        match mcp::start_mcp_server(
                            db_for_scheduler,
                            scheduler_arc,
                            app_handle.clone(),
                            &app_data_dir_clone,
                        )
                        .await
                        {
                            Ok(mcp_state) => {
                                let port = mcp_state.port;
                                app_handle.manage(mcp_state);
                                log::info!("MCP server started on port {}", port);

                                // Auto-configure ~/.claude.json
                                if let Err(e) = commands::tools::auto_configure_claude_mcp(port) {
                                    log::warn!("Auto-configure claude.json failed: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to start MCP server: {}", e);
                            }
                        }
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
            get_system_timezone,
            update_settings,
            // Execution Plan
            generate_plan,
            update_plan,
            // MCP
            get_mcp_status,
            repair_mcp_config,
            // AI Parse
            parse_nl_to_task,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // Cancel MCP server on exit
                if let Some(mcp_state) = app_handle.try_state::<mcp::McpState>() {
                    mcp_state.cancel.cancel();
                }
                // Checkpoint WAL to main database file before exit
                if let Some(db) = app_handle.try_state::<DbConn>() {
                    if let Ok(conn) = db.0.lock() {
                        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok();
                        log::info!("WAL checkpoint completed on primary connection");
                    }
                }
                if let Some(db_arc) = app_handle.try_state::<Arc<DbConn>>() {
                    if let Ok(conn) = db_arc.0.lock() {
                        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok();
                        log::info!("WAL checkpoint completed on shared connection");
                    }
                }
            }
        });
}
