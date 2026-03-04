use crate::db::DbConn;
use crate::models::task::{AiTool, CreateTaskRequest, Task, UpdateTaskRequest, WebhookConfig};
use crate::scheduler::engine::SchedulerState;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

pub fn row_to_task_pub(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    row_to_task(row)
}

fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let ai_tool_str: String = row.get(4)?;
    let env_vars_str: String = row.get(12)?;
    let webhook_str: Option<String> = row.get(13)?;
    let created_at_str: String = row.get(14)?;
    let updated_at_str: String = row.get(15)?;
    let last_run_at_str: Option<String> = row.get(16)?;

    let env_vars: HashMap<String, String> = serde_json::from_str(&env_vars_str).unwrap_or_default();
    let webhook_config: Option<WebhookConfig> = webhook_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(Task {
        id: row.get(0)?,
        name: row.get(1)?,
        cron_expression: row.get(2)?,
        cron_human: row.get(3)?,
        ai_tool: AiTool::from_str(&ai_tool_str),
        custom_command: row.get(5)?,
        prompt: row.get(6)?,
        working_directory: row.get(7)?,
        enabled: row.get::<_, i32>(8)? != 0,
        inject_context: row.get::<_, i32>(9)? != 0,
        restrict_network: row.get::<_, i32>(10)? != 0,
        restrict_filesystem: row.get::<_, i32>(11)? != 0,
        env_vars,
        webhook_config,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
        last_run_at: last_run_at_str.and_then(|s| s.parse().ok()),
        last_run_status: row.get(17)?,
    })
}

#[tauri::command]
pub fn get_tasks(db: State<'_, DbConn>) -> Result<Vec<Task>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, cron_expression, cron_human, ai_tool, custom_command, prompt,
             working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
             env_vars, webhook_config, created_at, updated_at, last_run_at, last_run_status
             FROM tasks ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let tasks: rusqlite::Result<Vec<Task>> = stmt
        .query_map([], row_to_task)
        .map_err(|e| e.to_string())?
        .collect();

    tasks.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_task(id: String, db: State<'_, DbConn>) -> Result<Task, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, name, cron_expression, cron_human, ai_tool, custom_command, prompt,
         working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
         env_vars, webhook_config, created_at, updated_at, last_run_at, last_run_status
         FROM tasks WHERE id = ?1",
        [&id],
        row_to_task,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_task(
    req: CreateTaskRequest,
    db: State<'_, DbConn>,
    db_arc: State<'_, Arc<DbConn>>,
    app_handle: AppHandle,
) -> Result<Task, String> {
    let task = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let ai_tool = AiTool::from_str(&req.ai_tool.unwrap_or_else(|| "claude".to_string()));
        let env_vars_json =
            serde_json::to_string(&req.env_vars.unwrap_or_default()).map_err(|e| e.to_string())?;
        let webhook_json = req
            .webhook_config
            .as_ref()
            .map(|w| serde_json::to_string(w).unwrap_or_default());

        conn.execute(
            "INSERT INTO tasks (id, name, cron_expression, cron_human, ai_tool, custom_command,
             prompt, working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
             env_vars, webhook_config, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            rusqlite::params![
                id,
                req.name,
                req.cron_expression,
                req.cron_human.unwrap_or_default(),
                ai_tool.as_str(),
                req.custom_command,
                req.prompt,
                req.working_directory,
                req.enabled.unwrap_or(true) as i32,
                req.inject_context.unwrap_or(false) as i32,
                req.restrict_network.unwrap_or(false) as i32,
                req.restrict_filesystem.unwrap_or(false) as i32,
                env_vars_json,
                webhook_json,
                now,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;

        conn.query_row(
            "SELECT id, name, cron_expression, cron_human, ai_tool, custom_command, prompt,
             working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
             env_vars, webhook_config, created_at, updated_at, last_run_at, last_run_status
             FROM tasks WHERE id = ?1",
            [&id],
            row_to_task,
        )
        .map_err(|e| e.to_string())?
    };

    // Notify scheduler if enabled
    if task.enabled {
        if let Some(sched) = app_handle.try_state::<Arc<SchedulerState>>() {
            let s: &Arc<SchedulerState> = &sched;
            s.add_task(task.clone(), (*db_arc).clone(), app_handle.clone())
                .await
                .ok();
        }
    }

    Ok(task)
}

#[tauri::command]
pub async fn update_task(
    id: String,
    req: UpdateTaskRequest,
    db: State<'_, DbConn>,
    db_arc: State<'_, Arc<DbConn>>,
    app_handle: AppHandle,
) -> Result<Task, String> {
    let task = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();

        // Build dynamic update
        let mut sets: Vec<String> = vec!["updated_at = ?1".to_string()];
        let mut idx = 2usize;

        macro_rules! push_field {
            ($field:expr, $col:expr) => {
                if $field.is_some() {
                    sets.push(format!("{} = ?{}", $col, idx));
                    idx += 1;
                }
            };
        }

        push_field!(req.name, "name");
        push_field!(req.cron_expression, "cron_expression");
        push_field!(req.cron_human, "cron_human");
        push_field!(req.ai_tool, "ai_tool");
        push_field!(req.custom_command, "custom_command");
        push_field!(req.prompt, "prompt");
        push_field!(req.working_directory, "working_directory");
        push_field!(req.enabled, "enabled");
        push_field!(req.inject_context, "inject_context");
        push_field!(req.restrict_network, "restrict_network");
        push_field!(req.restrict_filesystem, "restrict_filesystem");
        push_field!(req.env_vars, "env_vars");
        push_field!(req.webhook_config, "webhook_config");

        let sql = format!("UPDATE tasks SET {} WHERE id = ?{}", sets.join(", "), idx);

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        let mut bind_idx = 1usize;
        stmt.raw_bind_parameter(bind_idx, &now)
            .map_err(|e| e.to_string())?;
        bind_idx += 1;

        macro_rules! bind_opt {
            ($val:expr) => {
                if let Some(ref v) = $val {
                    stmt.raw_bind_parameter(bind_idx, v)
                        .map_err(|e| e.to_string())?;
                    bind_idx += 1;
                }
            };
            ($val:expr, bool) => {
                if let Some(v) = $val {
                    stmt.raw_bind_parameter(bind_idx, v as i32)
                        .map_err(|e| e.to_string())?;
                    bind_idx += 1;
                }
            };
            ($val:expr, json) => {
                if let Some(ref v) = $val {
                    let json = serde_json::to_string(v).map_err(|e| e.to_string())?;
                    stmt.raw_bind_parameter(bind_idx, json)
                        .map_err(|e| e.to_string())?;
                    bind_idx += 1;
                }
            };
            ($val:expr, opt_json) => {
                if let Some(ref v) = $val {
                    let json = v
                        .as_ref()
                        .map(|w| serde_json::to_string(w).unwrap_or_default());
                    stmt.raw_bind_parameter(bind_idx, json)
                        .map_err(|e| e.to_string())?;
                    bind_idx += 1;
                }
            };
        }

        bind_opt!(req.name);
        bind_opt!(req.cron_expression);
        bind_opt!(req.cron_human);
        bind_opt!(req.ai_tool);
        bind_opt!(req.custom_command);
        bind_opt!(req.prompt);
        bind_opt!(req.working_directory);
        bind_opt!(req.enabled, bool);
        bind_opt!(req.inject_context, bool);
        bind_opt!(req.restrict_network, bool);
        bind_opt!(req.restrict_filesystem, bool);
        bind_opt!(req.env_vars, json);
        bind_opt!(req.webhook_config, opt_json);

        stmt.raw_bind_parameter(bind_idx, &id)
            .map_err(|e| e.to_string())?;
        stmt.raw_execute().map_err(|e| e.to_string())?;

        conn.query_row(
            "SELECT id, name, cron_expression, cron_human, ai_tool, custom_command, prompt,
             working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
             env_vars, webhook_config, created_at, updated_at, last_run_at, last_run_status
             FROM tasks WHERE id = ?1",
            [&id],
            row_to_task,
        )
        .map_err(|e| e.to_string())?
    };

    // Reschedule task
    if let Some(sched) = app_handle.try_state::<Arc<SchedulerState>>() {
        let s: &Arc<SchedulerState> = &sched;
        if task.enabled {
            s.add_task(task.clone(), (*db_arc).clone(), app_handle.clone())
                .await
                .ok();
        } else {
            s.remove_task(&task.id).await.ok();
        }
    }

    Ok(task)
}

#[tauri::command]
pub async fn delete_task(
    id: String,
    db: State<'_, DbConn>,
    app_handle: AppHandle,
) -> Result<(), String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", [&id])
            .map_err(|e| e.to_string())?;
    }

    // Remove from scheduler
    if let Some(sched) = app_handle.try_state::<Arc<SchedulerState>>() {
        let s: &Arc<SchedulerState> = &sched;
        s.remove_task(&id).await.ok();
    }

    Ok(())
}

#[tauri::command]
pub async fn set_task_enabled(
    id: String,
    enabled: bool,
    db: State<'_, DbConn>,
    db_arc: State<'_, Arc<DbConn>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let task = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![enabled as i32, now, id],
        )
        .map_err(|e| e.to_string())?;

        conn.query_row(
            "SELECT id, name, cron_expression, cron_human, ai_tool, custom_command, prompt,
             working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
             env_vars, webhook_config, created_at, updated_at, last_run_at, last_run_status
             FROM tasks WHERE id = ?1",
            [&id],
            row_to_task,
        )
        .map_err(|e| e.to_string())?
    };

    // Update scheduler
    if let Some(sched) = app_handle.try_state::<Arc<SchedulerState>>() {
        let s: &Arc<SchedulerState> = &sched;
        if enabled {
            s.add_task(task, (*db_arc).clone(), app_handle.clone())
                .await
                .ok();
        } else {
            s.remove_task(&id).await.ok();
        }
    }

    Ok(())
}
