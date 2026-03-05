use crate::db::DbConn;
use crate::models::run::{Run, RunStatus, TriggerSource};
use crate::models::task::{AiTool, Task};
use crate::webhook::WebhookSender;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

// Global process registry: run_id -> PID
static PROCESS_REGISTRY: std::sync::LazyLock<StdMutex<HashMap<String, u32>>> =
    std::sync::LazyLock::new(|| StdMutex::new(HashMap::new()));

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
    run_id: String,
) {
    let started_at = Utc::now();

    // Get last run for context injection
    let last_run: Option<Run> = {
        let conn = match db.0.lock() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to lock DB for last run query: {}", e);
                return;
            }
        };
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
        let conn = match db.0.lock() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to lock DB for run insert: {}", e);
                return;
            }
        };
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
        let mut unshare_args = vec![];
        if task.restrict_network {
            unshare_args.push("--net");
        }
        if task.restrict_filesystem {
            unshare_args.push("--mount");
        }
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
            // Register PID in process registry
            if let Some(pid) = child.id() {
                if let Ok(mut reg) = PROCESS_REGISTRY.lock() {
                    reg.insert(run_id.clone(), pid);
                }
            }

            // Safety: stdout/stderr are guaranteed to exist because we set Stdio::piped() above
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            let run_id_out = run_id.clone();
            let run_id_err = run_id.clone();
            let app_out = app_handle.clone();
            let app_err = app_handle.clone();

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

            // Remove from process registry
            if let Ok(mut reg) = PROCESS_REGISTRY.lock() {
                reg.remove(&run_id);
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
        let conn = match db.0.lock() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to lock DB for run persist: {}", e);
                return;
            }
        };
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
        // Contentless FTS5 does not support REPLACE; delete first then insert
        let _ = conn.execute(
            "DELETE FROM runs_fts WHERE run_id = ?1",
            rusqlite::params![run_id],
        );
        let _ = conn.execute(
            "INSERT INTO runs_fts(run_id, task_id, task_name, stdout, stderr)
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

    // Send desktop notification
    send_notification(&app_handle, &task, &final_status);

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

/// Send desktop notification based on settings
fn send_notification(app_handle: &AppHandle, task: &Task, status: &RunStatus) {
    use tauri_plugin_notification::NotificationExt;

    // Read settings to check notification preferences
    if let Some(db) = app_handle.try_state::<DbConn>() {
        let (notify_success, notify_failure) = {
            let conn = match db.0.lock() {
                Ok(c) => c,
                Err(_) => return,
            };
            let success: bool = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'notify_on_success'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false);
            let failure: bool = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'notify_on_failure'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true);
            (success, failure)
        };

        let should_notify = match status {
            RunStatus::Success => notify_success,
            RunStatus::Failed => notify_failure,
            RunStatus::Killed => notify_failure,
            _ => false,
        };

        if should_notify {
            let icon = match status {
                RunStatus::Success => "✓",
                RunStatus::Failed => "✗",
                RunStatus::Killed => "⊘",
                _ => "",
            };
            app_handle
                .notification()
                .builder()
                .title("AI Cron")
                .body(format!("{} {} — {}", icon, task.name, status.as_str()))
                .show()
                .ok();
        }
    }
}

/// Tauri command: manually trigger a task now
#[tauri::command]
pub async fn trigger_task_now(
    task_id: String,
    app_handle: AppHandle,
    db: State<'_, DbConn>,
    db_arc: State<'_, Arc<DbConn>>,
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
    let shared_db = db_arc.inner().clone();
    let run_id_clone = run_id.clone();

    tokio::spawn(execute_task(
        task,
        TriggerSource::Manual,
        app_handle,
        shared_db,
        run_id_clone,
    ));

    Ok(run_id)
}

/// Kill a running process by run_id
#[tauri::command]
pub async fn kill_run(
    run_id: String,
    db: State<'_, DbConn>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let pid = PROCESS_REGISTRY
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&run_id);

    if let Some(pid) = pid {
        // Kill the process tree
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .output()
                .ok();
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Use kill command to terminate the process group
            std::process::Command::new("kill")
                .args(["-TERM", &format!("-{}", pid)])
                .output()
                .ok();
        }

        // Update DB
        let ended_at = Utc::now();
        {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            let task_id: Option<String> = conn
                .query_row(
                    "SELECT task_id FROM runs WHERE id = ?1",
                    [&run_id],
                    |r| r.get(0),
                )
                .ok();

            conn.execute(
                "UPDATE runs SET status='killed', ended_at=?1 WHERE id=?2",
                rusqlite::params![ended_at.to_rfc3339(), run_id],
            )
            .ok();

            if let Some(ref tid) = task_id {
                conn.execute(
                    "UPDATE tasks SET last_run_status='killed' WHERE id=?1",
                    [tid],
                )
                .ok();
            }

            // Emit run:completed event
            let _ = app_handle.emit(
                "run:completed",
                serde_json::json!({
                    "runId": run_id,
                    "taskId": task_id,
                    "status": "killed",
                    "exitCode": null,
                    "durationMs": 0
                }),
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::run::{Run, RunStatus, TriggerSource};
    use crate::models::task::{AiTool, Task};
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_test_task(ai_tool: AiTool) -> Task {
        Task {
            id: "test-id".to_string(),
            name: "Test Task".to_string(),
            cron_expression: "* * * * *".to_string(),
            cron_human: "every minute".to_string(),
            ai_tool,
            custom_command: None,
            prompt: "do something".to_string(),
            working_directory: "/tmp/work".to_string(),
            enabled: true,
            inject_context: false,
            restrict_network: false,
            restrict_filesystem: false,
            env_vars: HashMap::new(),
            webhook_config: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run_at: None,
            last_run_status: None,
        }
    }

    #[test]
    fn build_command_claude() {
        let task = make_test_task(AiTool::Claude);
        let (prog, args) = build_command(&task);
        assert_eq!(prog, "claude");
        assert_eq!(args, vec!["-p", "do something"]);
    }

    #[test]
    fn build_command_opencode() {
        let task = make_test_task(AiTool::Opencode);
        let (prog, args) = build_command(&task);
        assert_eq!(prog, "opencode");
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn build_command_codex() {
        let task = make_test_task(AiTool::Codex);
        let (prog, args) = build_command(&task);
        assert_eq!(prog, "codex");
        assert_eq!(args, vec!["--approval-mode", "full-auto", "do something"]);
    }

    #[test]
    fn build_command_custom_with_template() {
        let mut task = make_test_task(AiTool::Custom);
        task.custom_command = Some("mybin --prompt {prompt} --dir {cwd}".to_string());
        let (prog, args) = build_command(&task);
        assert_eq!(prog, "mybin");
        // Note: split_whitespace splits "do something" into ["do", "something"]
        assert!(args.contains(&"--prompt".to_string()));
        assert!(args.contains(&"do".to_string()));
        assert!(args.contains(&"something".to_string()));
        assert!(args.contains(&"/tmp/work".to_string()));
        assert!(args.contains(&"--dir".to_string()));
    }

    #[test]
    fn build_command_custom_replaces_timestamp() {
        let mut task = make_test_task(AiTool::Custom);
        task.custom_command = Some("echo {timestamp}".to_string());
        let (prog, args) = build_command(&task);
        assert_eq!(prog, "echo");
        // timestamp is dynamic, just check it's not the literal placeholder
        assert!(!args[0].contains("{timestamp}"));
    }

    #[test]
    fn build_prompt_no_inject() {
        let task = make_test_task(AiTool::Claude);
        let result = build_prompt(&task, None);
        assert_eq!(result, "do something");
    }

    #[test]
    fn build_prompt_inject_no_last_run() {
        let mut task = make_test_task(AiTool::Claude);
        task.inject_context = true;
        let result = build_prompt(&task, None);
        assert!(result.contains("[Context]"));
        assert!(result.contains("Last run: never"));
        assert!(result.contains("[Task]"));
        assert!(result.contains("do something"));
    }

    #[test]
    fn build_prompt_inject_with_last_run() {
        let mut task = make_test_task(AiTool::Claude);
        task.inject_context = true;
        let last_run = Run {
            id: "run-1".to_string(),
            task_id: "test-id".to_string(),
            status: RunStatus::Success,
            exit_code: Some(0),
            stdout: "hello output".to_string(),
            stderr: String::new(),
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            duration_ms: Some(5000),
            triggered_by: TriggerSource::Scheduler,
        };
        let result = build_prompt(&task, Some(&last_run));
        assert!(result.contains("[Context]"));
        assert!(result.contains("success"));
        assert!(result.contains("5s"));
        assert!(result.contains("hello output"));
        assert!(result.contains("[Task]"));
        assert!(result.contains("do something"));
    }

    #[test]
    fn build_prompt_inject_duration_formatting() {
        let mut task = make_test_task(AiTool::Claude);
        task.inject_context = true;
        let last_run = Run {
            id: "run-1".to_string(),
            task_id: "test-id".to_string(),
            status: RunStatus::Failed,
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error".to_string(),
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            duration_ms: Some(125000), // 2m5s
            triggered_by: TriggerSource::Manual,
        };
        let result = build_prompt(&task, Some(&last_run));
        assert!(result.contains("2m5s"));
    }
}
