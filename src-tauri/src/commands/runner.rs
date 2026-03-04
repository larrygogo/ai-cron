use crate::db::DbConn;
use crate::models::run::{Run, RunStatus, TriggerSource};
use crate::models::task::{AiTool, Task};
use crate::webhook::WebhookSender;
use chrono::Utc;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

/// Build the command args for a given AI tool
fn build_command(task: &Task) -> (String, Vec<String>) {
    match task.ai_tool {
        AiTool::Claude => (
            "claude".to_string(),
            vec!["-p".to_string(), task.prompt.clone()],
        ),
        AiTool::Opencode => (
            "opencode".to_string(),
            vec![task.prompt.clone()],
        ),
        AiTool::Codex => (
            "codex".to_string(),
            vec![
                "--approval-mode".to_string(),
                "full-auto".to_string(),
                task.prompt.clone(),
            ],
        ),
        AiTool::Custom => {
            // Template: replace {prompt} and {cwd}
            let tmpl = task
                .custom_command
                .clone()
                .unwrap_or_else(|| task.prompt.clone());
            let expanded = tmpl
                .replace("{prompt}", &task.prompt)
                .replace("{cwd}", &task.working_directory)
                .replace(
                    "{timestamp}",
                    &Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                );
            // Split into program + args (simple whitespace split)
            let mut parts = expanded.splitn(2, ' ');
            let prog = parts.next().unwrap_or("echo").to_string();
            let args = parts
                .next()
                .unwrap_or("")
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            (prog, args)
        }
    }
}

/// Build context-injected prompt if enabled
fn build_prompt(task: &Task, last_run: Option<&Run>) -> String {
    if !task.inject_context {
        return task.prompt.clone();
    }

    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let last_run_info = match last_run {
        Some(r) => {
            let status = r.status.as_str();
            let duration = r
                .duration_ms
                .map(|ms| {
                    let s = ms / 1000;
                    if s >= 60 {
                        format!("{}m{}s", s / 60, s % 60)
                    } else {
                        format!("{}s", s)
                    }
                })
                .unwrap_or_else(|| "-".to_string());
            let tail: String = r.stdout.chars().rev().take(500).collect::<String>()
                .chars().rev().collect();
            format!(
                "Last run: {} ({}, {})\nLast output (tail):\n{}",
                r.started_at.format("%Y-%m-%d %H:%M"),
                status,
                duration,
                tail
            )
        }
        None => "Last run: never".to_string(),
    };

    format!(
        "[Context]\nCurrent time: {}\nWorking directory: {}\n{}\n\n[Task]\n{}",
        now, task.working_directory, last_run_info, task.prompt
    )
}

/// Core async run function — called by scheduler and manual trigger
pub async fn execute_task(
    task: Task,
    triggered_by: TriggerSource,
    app_handle: AppHandle,
    db: Arc<DbConn>,
) {
    let run_id = Uuid::new_v4().to_string();
    let started_at = Utc::now();

    // Get last run for context injection
    let last_run: Option<Run> = {
        let conn = db.0.lock().unwrap();
        conn.query_row(
            "SELECT id, task_id, status, exit_code, stdout, started_at, ended_at, duration_ms,
             triggered_by, stderr FROM runs WHERE task_id = ?1 ORDER BY started_at DESC LIMIT 1",
            [&task.id],
            |row| {
                Ok(Run {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    status: RunStatus::from_str(&row.get::<_, String>(2)?),
                    exit_code: row.get(3)?,
                    stdout: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    stderr: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
                    started_at: row
                        .get::<_, String>(5)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    ended_at: row
                        .get::<_, Option<String>>(6)?
                        .and_then(|s| s.parse().ok()),
                    duration_ms: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                    triggered_by: TriggerSource::from_str(&row.get::<_, String>(8)?),
                })
            },
        )
        .ok()
    };

    let final_prompt = build_prompt(&task, last_run.as_ref());

    // Insert run record as "running"
    {
        let conn = db.0.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO runs(id,task_id,status,stdout,stderr,started_at,triggered_by)
             VALUES(?1,?2,'running','','',?3,?4)",
            rusqlite::params![
                run_id,
                task.id,
                started_at.to_rfc3339(),
                triggered_by.as_str()
            ],
        );
        // Update task last_run_at
        let _ = conn.execute(
            "UPDATE tasks SET last_run_at=?1, last_run_status='running' WHERE id=?2",
            rusqlite::params![started_at.to_rfc3339(), task.id],
        );
    }

    // Emit run:started
    let _ = app_handle.emit("run:started", serde_json::json!({ "runId": run_id, "taskId": task.id }));

    // Send webhook on_start
    let webhook_sender = WebhookSender::new();
    if let Some(ref wh) = task.webhook_config {
        webhook_sender
            .send(wh, &task, &RunStatus::Running, None, "", "")
            .await;
    }

    // Build command with context-aware prompt
    let mut cmd_task = task.clone();
    cmd_task.prompt = final_prompt;
    let (program, args) = build_command(&cmd_task);

    // Build tokio Command
    let mut cmd = tokio::process::Command::new(&program);
    cmd.args(&args)
        .current_dir(&task.working_directory)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    // Apply env vars
    for (k, v) in &task.env_vars {
        cmd.env(k, v);
    }

    // Sandbox: restrict network on Linux
    #[cfg(target_os = "linux")]
    if task.restrict_network || task.restrict_filesystem {
        // Wrap with unshare
        let mut unshare_args = vec![];
        if task.restrict_network {
            unshare_args.push("--net");
        }
        if task.restrict_filesystem {
            unshare_args.push("--mount");
        }
        // Rebuild as: unshare <flags> <program> <args>
        // Note: requires root or user namespaces enabled
        let original_program = program.clone();
        let mut full_args: Vec<String> = unshare_args.iter().map(|s| s.to_string()).collect();
        full_args.push(original_program);
        full_args.extend(args.clone());
        cmd = tokio::process::Command::new("unshare");
        cmd.args(&full_args)
            .current_dir(&task.working_directory)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
    }

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();
    let mut exit_code: Option<i32> = None;
    let final_status: RunStatus;

    match cmd.spawn() {
        Ok(mut child) => {
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            let run_id_out = run_id.clone();
            let run_id_err = run_id.clone();
            let app_out = app_handle.clone();
            let app_err = app_handle.clone();

            // Collect stdout
            let mut stdout_lines = BufReader::new(stdout).lines();
            let mut stderr_lines = BufReader::new(stderr).lines();

            // Stream stdout and stderr concurrently
            loop {
                tokio::select! {
                    line = stdout_lines.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                stdout_buf.push_str(&l);
                                stdout_buf.push('\n');
                                let _ = app_out.emit("run:output", serde_json::json!({
                                    "runId": run_id_out,
                                    "chunk": l,
                                    "stream": "stdout"
                                }));
                            }
                            _ => break,
                        }
                    }
                    line = stderr_lines.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                stderr_buf.push_str(&l);
                                stderr_buf.push('\n');
                                let _ = app_err.emit("run:output", serde_json::json!({
                                    "runId": run_id_err,
                                    "chunk": l,
                                    "stream": "stderr"
                                }));
                            }
                            _ => break,
                        }
                    }
                }
            }

            // Drain remaining
            while let Ok(Some(l)) = stdout_lines.next_line().await {
                stdout_buf.push_str(&l);
                stdout_buf.push('\n');
            }
            while let Ok(Some(l)) = stderr_lines.next_line().await {
                stderr_buf.push_str(&l);
                stderr_buf.push('\n');
            }

            match child.wait().await {
                Ok(status) => {
                    exit_code = status.code();
                    final_status = if status.success() {
                        RunStatus::Success
                    } else {
                        RunStatus::Failed
                    };
                }
                Err(_) => {
                    final_status = RunStatus::Failed;
                }
            }
        }
        Err(e) => {
            stderr_buf = format!("Failed to spawn process '{}': {}", program, e);
            final_status = RunStatus::Failed;
        }
    }

    let ended_at = Utc::now();
    let duration_ms = (ended_at - started_at).num_milliseconds().max(0) as u64;

    // Persist final run state
    {
        let conn = db.0.lock().unwrap();
        let _ = conn.execute(
            "UPDATE runs SET status=?1, exit_code=?2, stdout=?3, stderr=?4,
             ended_at=?5, duration_ms=?6 WHERE id=?7",
            rusqlite::params![
                final_status.as_str(),
                exit_code,
                stdout_buf,
                stderr_buf,
                ended_at.to_rfc3339(),
                duration_ms as i64,
                run_id
            ],
        );
        // Update FTS index manually since trigger may not fire on UPDATE
        let _ = conn.execute(
            "INSERT OR REPLACE INTO runs_fts(run_id, task_id, task_name, stdout, stderr)
             SELECT ?1, ?2, t.name, ?3, ?4 FROM tasks t WHERE t.id = ?2",
            rusqlite::params![run_id, task.id, stdout_buf, stderr_buf],
        );
        let _ = conn.execute(
            "UPDATE tasks SET last_run_status=?1 WHERE id=?2",
            rusqlite::params![final_status.as_str(), task.id],
        );
    }

    // Emit run:completed
    let _ = app_handle.emit(
        "run:completed",
        serde_json::json!({
            "runId": run_id,
            "taskId": task.id,
            "status": final_status.as_str(),
            "exitCode": exit_code,
            "durationMs": duration_ms
        }),
    );

    // Send webhook on completion
    if let Some(ref wh) = task.webhook_config {
        webhook_sender
            .send(
                wh,
                &task,
                &final_status,
                Some(duration_ms),
                &stdout_buf,
                &stderr_buf,
            )
            .await;
    }
}

/// Tauri command: manually trigger a task now
#[tauri::command]
pub async fn trigger_task_now(
    task_id: String,
    app_handle: AppHandle,
    db: State<'_, DbConn>,
) -> Result<String, String> {
    let task = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT id, name, cron_expression, cron_human, ai_tool, custom_command, prompt,
             working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
             env_vars, webhook_config, created_at, updated_at, last_run_at, last_run_status
             FROM tasks WHERE id = ?1",
            [&task_id],
            crate::commands::tasks::row_to_task_pub,
        )
        .map_err(|e| e.to_string())?
    };

    let run_id = Uuid::new_v4().to_string();
    // Get DB path to open a second connection for the async task
    let db_path = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        conn.path()
            .map(|p| p.to_string())
            .unwrap_or_else(|| ":memory:".to_string())
    };
    let db_arc = Arc::new(DbConn(std::sync::Mutex::new(
        rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?,
    )));
    db_arc
        .0
        .lock()
        .unwrap()
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .ok();

    tokio::spawn(execute_task(
        task,
        TriggerSource::Manual,
        app_handle,
        db_arc,
    ));

    Ok(run_id)
}

/// Kill a running process (tracked via global run state — simplified version)
#[tauri::command]
pub fn kill_run(_run_id: String) -> Result<(), String> {
    // Full implementation requires a global process registry (Phase 2)
    // Placeholder: mark run as killed in DB
    Ok(())
}
