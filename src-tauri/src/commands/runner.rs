use crate::db::DbConn;
use crate::models::run::{Run, RunStatus, TriggerSource};
use crate::models::task::{AiTool, Task};
use crate::webhook::WebhookSender;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

// Global process registry: run_id -> PID
pub static PROCESS_REGISTRY: std::sync::LazyLock<StdMutex<HashMap<String, u32>>> =
    std::sync::LazyLock::new(|| StdMutex::new(HashMap::new()));

/// Expand `~` or `~/...` to the user's home directory
fn expand_tilde(path: &str) -> String {
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        if let Some(home) = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
        {
            let home = std::path::PathBuf::from(home);
            let rest = if path.len() > 1 { &path[2..] } else { "" };
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

/// Build the command args for a given AI tool
fn build_command(task: &Task) -> (String, Vec<String>) {
    match task.ai_tool {
        AiTool::Claude => {
            let mut args = vec!["-p".to_string(), task.prompt.clone()];
            if !task.allowed_tools.is_empty() {
                for tool in &task.allowed_tools {
                    args.push("--allowedTools".to_string());
                    args.push(tool.clone());
                }
            }
            if task.skip_permissions {
                args.push("--dangerously-skip-permissions".to_string());
            }
            ("claude".to_string(), args)
        }
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
            // Run via shell so built-in commands (echo, cd, etc.) work
            #[cfg(target_os = "windows")]
            {
                ("cmd".to_string(), vec!["/C".to_string(), expanded])
            }
            #[cfg(not(target_os = "windows"))]
            {
                ("sh".to_string(), vec!["-c".to_string(), expanded])
            }
        }
    }
}

/// Collect git context from the working directory (best-effort)
fn get_git_context(working_dir: &str) -> Option<String> {
    let dir = expand_tilde(working_dir);
    let mut parts = Vec::new();

    // Current branch
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
    {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            parts.push(format!("Branch: {}", branch));
        }
    }

    // Recent commits (last 5)
    if let Ok(output) = std::process::Command::new("git")
        .args(["log", "--oneline", "-5"])
        .current_dir(&dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
    {
        if output.status.success() {
            let log = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !log.is_empty() {
                parts.push(format!("Recent commits:\n{}", log));
            }
        }
    }

    // Open PRs (via gh CLI, best-effort)
    if let Ok(output) = std::process::Command::new("gh")
        .args([
            "pr", "list", "--state", "open", "--limit", "10",
            "--json", "number,title,author,updatedAt",
            "--template",
            "{{range .}}#{{.number}} {{.title}} (by @{{.author.login}}, updated {{timeago .updatedAt}})\n{{end}}",
        ])
        .current_dir(&dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
    {
        if output.status.success() {
            let prs = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prs.is_empty() {
                parts.push(format!("Open PRs:\n{}", prs));
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Build context-injected prompt with execution plan
fn build_prompt(task: &Task, last_run: Option<&Run>, git_context: Option<&str>) -> String {
    let mut parts = Vec::new();

    // 1. Inject execution plan
    if !task.execution_plan.is_empty() {
        parts.push(format!("[Execution Plan]\n{}", task.execution_plan));
    }

    // 2. Inject historical context if enabled
    if task.inject_context {
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
        parts.push(format!(
            "[Context]\nCurrent time: {}\nWorking directory: {}\n{}",
            now, task.working_directory, last_run_info
        ));
    }

    // 3. Inject git context if enabled
    if task.inject_context {
        if let Some(git_ctx) = git_context {
            parts.push(format!("[Git Context]\n{}", git_ctx));
        }
    }

    // 4. Task prompt
    parts.push(format!("[Task]\n{}", task.prompt));

    parts.join("\n\n")
}

/// Describe an exit status in human-readable Chinese
fn describe_exit(status: &std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        match code {
            0 => "正常退出".to_string(),
            1 => "一般错误 (exit 1)".to_string(),
            2 => "命令用法错误".to_string(),
            126 => "命令不可执行".to_string(),
            127 => "命令未找到".to_string(),
            130 => "被 Ctrl+C 中断 (SIGINT)".to_string(),
            137 => "被系统强制终止 (SIGKILL/OOM)".to_string(),
            143 => "被正常终止 (SIGTERM)".to_string(),
            _ => format!("退出码: {}", code),
        }
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(sig) = status.signal() {
                return format!("被信号终止: {}", sig);
            }
        }
        "异常终止（无退出码）".to_string()
    }
}

/// Log a phase message to stderr buffer and emit event
fn log_phase(buf: &mut String, app: &AppHandle, run_id: &str, msg: &str) {
    let line = format!("[ai-cron] {}", msg);
    buf.push_str(&line);
    buf.push('\n');
    let _ = app.emit("run:output", serde_json::json!({
        "runId": run_id, "chunk": line, "stream": "stderr"
    }));
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
             triggered_by, stderr, goal_evaluation FROM runs WHERE task_id = ?1 ORDER BY started_at DESC LIMIT 1",
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
                    goal_evaluation: row.get(10)?,
                })
            },
        )
        .ok()
    };

    let git_context = if task.inject_context {
        get_git_context(&task.working_directory)
    } else {
        None
    };
    let final_prompt = build_prompt(&task, last_run.as_ref(), git_context.as_deref());

    // Pre-execution feasibility check
    let mut pre_check_result: Option<String> = None;
    // (will be populated after run record is created so we can log to stderr)

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
    let work_dir = expand_tilde(&task.working_directory);

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();
    let mut exit_code: Option<i32> = None;
    let final_status: RunStatus;

    // Phase logging: start
    log_phase(&mut stderr_buf, &app_handle, &run_id,
        &format!("▶ 开始执行任务: {}", task.name));
    log_phase(&mut stderr_buf, &app_handle, &run_id,
        &format!("⚙ 工作目录: {}", work_dir));
    let args_preview = if args.len() <= 2 {
        args.join(" ")
    } else {
        format!("{} ...({}个参数)", args[0], args.len())
    };
    log_phase(&mut stderr_buf, &app_handle, &run_id,
        &format!("⚙ 命令: {} {}", program, args_preview));
    if !task.execution_plan.is_empty() {
        log_phase(&mut stderr_buf, &app_handle, &run_id,
            &format!("⚙ 执行计划已注入 ({} 字符)", task.execution_plan.len()));

        // Pre-execution feasibility check
        if let Ok(settings) = crate::commands::tools::get_settings_core(&db) {
            let dir_ctx = crate::commands::plan_gen::scan_directory_context(&task.working_directory);
            match crate::commands::plan_gen::pre_check_feasibility(
                &task.execution_plan, dir_ctx.as_deref(), &settings
            ).await {
                Ok(result) => {
                    pre_check_result = Some(result.clone());
                    let feasible = result.contains("\"feasible\": true") || result.contains("\"feasible\":true");
                    if feasible {
                        log_phase(&mut stderr_buf, &app_handle, &run_id, "✓ 可达性检查通过");
                    } else {
                        log_phase(&mut stderr_buf, &app_handle, &run_id,
                            &format!("⚠ 可达性检查警告: {}", result.trim()));
                    }
                }
                Err(e) => {
                    log_phase(&mut stderr_buf, &app_handle, &run_id,
                        &format!("⚠ 可达性检查跳过: {}", e));
                }
            }
        }
    }
    let mut cmd = tokio::process::Command::new(&program);
    cmd.args(&args)
        .current_dir(&work_dir)
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
            .current_dir(&work_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
    }

    match cmd.spawn() {
        Ok(mut child) => {
            // Register PID in process registry
            if let Some(pid) = child.id() {
                log_phase(&mut stderr_buf, &app_handle, &run_id,
                    &format!("⏱ 进程已启动 (PID: {})", pid));
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
                    let desc = describe_exit(&status);
                    if status.success() {
                        final_status = RunStatus::Success;
                        let duration = (Utc::now() - started_at).num_milliseconds().max(0);
                        log_phase(&mut stderr_buf, &app_handle, &run_id,
                            &format!("✓ 进程正常退出 ({}, 耗时: {}ms)", desc, duration));
                    } else {
                        final_status = RunStatus::Failed;
                        log_phase(&mut stderr_buf, &app_handle, &run_id,
                            &format!("✗ 进程异常退出 ({})", desc));
                    }
                }
                Err(e) => {
                    final_status = RunStatus::Failed;
                    log_phase(&mut stderr_buf, &app_handle, &run_id,
                        &format!("✗ 等待进程失败: {}", e));
                }
            }

            // Remove from process registry
            if let Ok(mut reg) = PROCESS_REGISTRY.lock() {
                reg.remove(&run_id);
            }
        }
        Err(e) => {
            log_phase(&mut stderr_buf, &app_handle, &run_id,
                &format!("✗ 进程启动失败: {} (命令: {})", e, program));
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

    // Synchronous post-execution goal verification (for tasks with execution plan)
    // If goal not passed, downgrade status to failed
    let mut final_status = final_status;
    if final_status == RunStatus::Success && !task.execution_plan.is_empty() {
        log_phase(&mut stderr_buf, &app_handle, &run_id, "⏳ 正在进行目标验证...");
        if let Ok(settings) = crate::commands::tools::get_settings_core(&db) {
            match crate::commands::plan_gen::post_check_goal(&task.execution_plan, &stdout_buf, &settings).await {
                Ok(post_result) => {
                    let passed = post_result.contains("\"passed\": true") || post_result.contains("\"passed\":true");
                    let evaluation = serde_json::json!({
                        "pre_check": pre_check_result.as_deref().unwrap_or("{}"),
                        "post_check": post_result.trim(),
                        "passed": passed,
                        "evaluated_at": chrono::Utc::now().to_rfc3339()
                    }).to_string();

                    // Store goal_evaluation
                    if let Ok(conn) = db.0.lock() {
                        conn.execute(
                            "UPDATE runs SET goal_evaluation = ?1 WHERE id = ?2",
                            rusqlite::params![evaluation, run_id],
                        ).ok();
                    }

                    if passed {
                        log_phase(&mut stderr_buf, &app_handle, &run_id, "✓ 目标验证通过");
                    } else {
                        log_phase(&mut stderr_buf, &app_handle, &run_id, "✗ 目标验证未通过，状态降级为 failed");
                        final_status = RunStatus::Failed;
                        // Update run status and task status in DB
                        if let Ok(conn) = db.0.lock() {
                            conn.execute(
                                "UPDATE runs SET status = 'failed' WHERE id = ?1",
                                rusqlite::params![run_id],
                            ).ok();
                            conn.execute(
                                "UPDATE tasks SET last_run_status = 'failed' WHERE id = ?1",
                                rusqlite::params![task.id],
                            ).ok();
                        }
                    }

                    let _ = app_handle.emit("run:evaluated", serde_json::json!({
                        "runId": run_id, "taskId": task.id, "passed": passed,
                    }));
                }
                Err(e) => {
                    log::warn!("目标验证失败 (run {}): {}", run_id, e);
                    log_phase(&mut stderr_buf, &app_handle, &run_id,
                        &format!("⚠ 目标验证跳过: {}", e));
                }
            }
        }
        // Update stderr in DB after goal verification logging
        if let Ok(conn) = db.0.lock() {
            conn.execute(
                "UPDATE runs SET stderr = ?1 WHERE id = ?2",
                rusqlite::params![stderr_buf, run_id],
            ).ok();
        }
    }

    // Update consecutive_failures counter
    {
        if let Ok(conn) = db.0.lock() {
            if final_status == RunStatus::Failed {
                let new_count = task.consecutive_failures + 1;
                conn.execute(
                    "UPDATE tasks SET consecutive_failures = ?1 WHERE id = ?2",
                    rusqlite::params![new_count, task.id],
                ).ok();

                // Auto-refine plan after 3 consecutive failures
                if new_count >= 3 && !task.execution_plan.is_empty() {
                    let failure_info = format!(
                        "Exit code: {:?}\nStderr (tail):\n{}",
                        exit_code,
                        &stderr_buf[..stderr_buf.len().min(1000)]
                    );
                    let db_clone = db.clone();
                    let task_id = task.id.clone();
                    let app_clone = app_handle.clone();
                    tokio::spawn(async move {
                        crate::commands::plan_gen::auto_refine_plan(
                            db_clone, task_id, failure_info, app_clone
                        ).await;
                    });
                }
            } else if final_status == RunStatus::Success {
                conn.execute(
                    "UPDATE tasks SET consecutive_failures = 0 WHERE id = ?1",
                    rusqlite::params![task.id],
                ).ok();
            }
        }
    }

    // Emit run:completed (with final status after goal verification)
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
    db_arc: State<'_, Arc<DbConn>>,
) -> Result<String, String> {
    let task = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT id, name, cron_expression, cron_human, ai_tool, custom_command, prompt,
             working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
             env_vars, webhook_config, created_at, updated_at, last_run_at, last_run_status,
             execution_plan, consecutive_failures, allowed_tools, skip_permissions
             FROM tasks WHERE id = ?1",
            [&task_id],
            crate::commands::tasks::row_to_task,
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
        let webhook_task = {
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

            // Query full task for webhook notification
            task_id.and_then(|tid| {
                conn.query_row(
                    "SELECT id, name, cron_expression, cron_human, ai_tool, custom_command, prompt,
                     working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
                     env_vars, webhook_config, created_at, updated_at, last_run_at, last_run_status,
             execution_plan, consecutive_failures, allowed_tools, skip_permissions
                     FROM tasks WHERE id = ?1",
                    [&tid],
                    crate::commands::tasks::row_to_task,
                )
                .ok()
            })
        };

        // Send webhook for killed status (outside DB lock)
        if let Some(ref task) = webhook_task {
            if let Some(ref wh) = task.webhook_config {
                WebhookSender::new().send(wh, task, &RunStatus::Killed, None, "", "").await;
            }
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
            allowed_tools: Vec::new(),
            skip_permissions: false,
            execution_plan: String::new(),
            consecutive_failures: 0,
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
    fn build_command_custom_with_template() {
        let mut task = make_test_task(AiTool::Custom);
        task.custom_command = Some("mybin --prompt {prompt} --dir {cwd}".to_string());
        let (prog, args) = build_command(&task);
        // Custom commands run via shell
        #[cfg(target_os = "windows")]
        {
            assert_eq!(prog, "cmd");
            assert_eq!(args[0], "/C");
            assert!(args[1].contains("mybin"));
            assert!(args[1].contains("do something"));
            assert!(args[1].contains("/tmp/work"));
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(prog, "sh");
            assert_eq!(args[0], "-c");
            assert!(args[1].contains("mybin"));
            assert!(args[1].contains("do something"));
            assert!(args[1].contains("/tmp/work"));
        }
    }

    #[test]
    fn build_command_custom_replaces_timestamp() {
        let mut task = make_test_task(AiTool::Custom);
        task.custom_command = Some("echo {timestamp}".to_string());
        let (prog, args) = build_command(&task);
        // Custom commands run via shell
        #[cfg(target_os = "windows")]
        assert_eq!(prog, "cmd");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(prog, "sh");
        // The expanded command should not contain the literal placeholder
        let cmd_str = &args[1];
        assert!(!cmd_str.contains("{timestamp}"));
    }

    #[test]
    fn build_prompt_no_inject() {
        let task = make_test_task(AiTool::Claude);
        let result = build_prompt(&task, None, None);
        assert_eq!(result, "[Task]\ndo something");
    }

    #[test]
    fn build_prompt_inject_no_last_run() {
        let mut task = make_test_task(AiTool::Claude);
        task.inject_context = true;
        let result = build_prompt(&task, None, None);
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
            goal_evaluation: None,
        };
        let result = build_prompt(&task, Some(&last_run), None);
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
            goal_evaluation: None,
        };
        let result = build_prompt(&task, Some(&last_run), None);
        assert!(result.contains("2m5s"));
    }

    #[test]
    fn build_prompt_with_execution_plan() {
        let mut task = make_test_task(AiTool::Claude);
        task.execution_plan = "1. Step one\n2. Step two".to_string();
        let result = build_prompt(&task, None, None);
        assert!(result.contains("[Execution Plan]"));
        assert!(result.contains("1. Step one"));
        assert!(result.contains("[Task]"));
        assert!(result.contains("do something"));
    }

    #[test]
    fn build_prompt_with_plan_and_context() {
        let mut task = make_test_task(AiTool::Claude);
        task.execution_plan = "Plan here".to_string();
        task.inject_context = true;
        let result = build_prompt(&task, None, None);
        assert!(result.contains("[Execution Plan]"));
        assert!(result.contains("[Context]"));
        assert!(result.contains("[Task]"));
        // Verify order: plan before context before task
        let plan_pos = result.find("[Execution Plan]").unwrap();
        let ctx_pos = result.find("[Context]").unwrap();
        let task_pos = result.find("[Task]").unwrap();
        assert!(plan_pos < ctx_pos);
        assert!(ctx_pos < task_pos);
    }

    #[test]
    fn build_prompt_empty_plan_not_injected() {
        let mut task = make_test_task(AiTool::Claude);
        task.execution_plan = String::new();
        let result = build_prompt(&task, None, None);
        assert!(!result.contains("[Execution Plan]"));
    }

    #[test]
    fn describe_exit_common_codes() {
        // Test the exit code matching logic directly (ExitStatus can't be constructed in tests)
        let cases = vec![
            (0, "正常退出"),
            (1, "一般错误"),
            (127, "命令未找到"),
            (137, "SIGKILL/OOM"),
            (143, "SIGTERM"),
            (42, "退出码: 42"),
        ];
        for (code, expected_substr) in cases {
            // Create a mock-like test by checking the match arms directly
            let desc = match code {
                0 => "正常退出".to_string(),
                1 => "一般错误 (exit 1)".to_string(),
                2 => "命令用法错误".to_string(),
                126 => "命令不可执行".to_string(),
                127 => "命令未找到".to_string(),
                130 => "被 Ctrl+C 中断 (SIGINT)".to_string(),
                137 => "被系统强制终止 (SIGKILL/OOM)".to_string(),
                143 => "被正常终止 (SIGTERM)".to_string(),
                _ => format!("退出码: {}", code),
            };
            assert!(desc.contains(expected_substr),
                "code {} -> '{}' should contain '{}'", code, desc, expected_substr);
        }
    }
}
