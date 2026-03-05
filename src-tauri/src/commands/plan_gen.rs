use crate::commands::tools::AppSettings;
use crate::db::DbConn;
use std::time::Duration;

const PLAN_SYSTEM_PROMPT: &str = r#"你是一个 AI 任务执行计划生成器。根据任务提示词和项目上下文，生成一份清晰、可执行的步骤化执行计划。
计划应包含：
1. 任务目标概述
2. 成功标准（明确的、可验证的完成条件，用于在执行后判断任务是否真正达成目标）
3. 具体执行步骤（按顺序）
4. 预期输出格式
5. 注意事项和约束
请用简洁的 Markdown 格式输出，不要包含任何多余解释。"#;

/// Generate an execution plan for a task
pub async fn generate_execution_plan(
    prompt: &str,
    task_name: &str,
    working_dir: &str,
    dir_context: Option<&str>,
    failure_context: Option<&str>,
    settings: &AppSettings,
) -> Result<String, String> {
    let mut system = PLAN_SYSTEM_PROMPT.to_string();

    if let Some(failure) = failure_context {
        system.push_str(&format!(
            "\n\n该任务最近连续失败，以下是最近一次的错误信息：\n{}\n请在生成计划时针对这些问题做出调整，增加错误预防和恢复步骤。",
            failure
        ));
    }

    let mut user_msg = format!("任务名称: {}\n工作目录: {}\n\n任务提示词:\n{}", task_name, working_dir, prompt);

    if let Some(ctx) = dir_context {
        user_msg.push_str(&format!("\n\n项目目录结构:\n{}", ctx));
    }

    call_ai(&system, &user_msg, settings).await
}

/// Generic AI call dispatcher — always uses local Claude CLI
pub(crate) async fn call_ai(system: &str, user_msg: &str, _settings: &AppSettings) -> Result<String, String> {
    plan_with_cli(system, user_msg).await
}

async fn plan_with_cli(system: &str, user_msg: &str) -> Result<String, String> {
    let prompt = format!("{}\n\n{}", system, user_msg);

    let output = tokio::time::timeout(
        Duration::from_secs(60),
        tokio::process::Command::new("claude")
            .args(["-p", &prompt])
            .output(),
    )
    .await
    .map_err(|_| "Claude CLI timed out after 60 seconds".to_string())?
    .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Claude CLI error: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

const PRE_CHECK_PROMPT: &str = r#"你是一个任务可行性分析器。根据执行计划和当前环境信息，判断任务目标是否可达。
请严格按以下 JSON 格式回复，不要添加其他内容：
{"feasible": true/false, "concerns": "简要说明风险或不可达原因（不超过100字）"}"#;

const POST_CHECK_PROMPT: &str = r#"你是一个任务完成验证器。根据执行计划中的成功标准，判断任务输出是否达成了目标。
请严格按以下 JSON 格式回复，不要添加其他内容：
{"passed": true/false, "summary": "简要说明判断理由（不超过100字）"}"#;

/// Pre-execution feasibility check
pub async fn pre_check_feasibility(
    execution_plan: &str,
    dir_context: Option<&str>,
    settings: &AppSettings,
) -> Result<String, String> {
    let mut user_msg = format!("## 执行计划\n{}", execution_plan);
    if let Some(ctx) = dir_context {
        user_msg.push_str(&format!("\n\n## 当前环境\n{}", ctx));
    }
    call_ai(PRE_CHECK_PROMPT, &user_msg, settings).await
}

/// Post-execution goal verification
pub async fn post_check_goal(
    execution_plan: &str,
    stdout: &str,
    settings: &AppSettings,
) -> Result<String, String> {
    let user_msg = format!(
        "## 执行计划\n{}\n\n## 任务输出（截取最后2000字符）\n{}",
        execution_plan,
        &stdout[stdout.len().saturating_sub(2000)..]
    );
    call_ai(POST_CHECK_PROMPT, &user_msg, settings).await
}

/// Scan a working directory for project context
pub fn scan_directory_context(working_dir: &str) -> Option<String> {
    let path = std::path::Path::new(working_dir);
    if !path.is_dir() {
        return None;
    }

    let mut output = String::new();
    let ignore_dirs = ["node_modules", ".git", "target", "dist", "build", "__pycache__", ".next", "vendor"];

    // List top-level entries
    let entries: Vec<_> = match std::fs::read_dir(path) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return None,
    };

    output.push_str("文件列表:\n");
    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if ignore_dirs.contains(&name.as_str()) {
            continue;
        }
        let file_type = if entry.path().is_dir() { "📁" } else { "📄" };
        output.push_str(&format!("  {} {}\n", file_type, name));
    }

    // Read key files (first 20 lines each)
    let key_files = ["package.json", "Cargo.toml", "README.md", "pyproject.toml", "go.mod"];
    for key_file in &key_files {
        let file_path = path.join(key_file);
        if file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let preview: String = content.lines().take(20).collect::<Vec<_>>().join("\n");
                output.push_str(&format!("\n{}:\n{}\n", key_file, preview));
            }
        }

        if output.len() > 2000 {
            output.truncate(2000);
            output.push_str("\n...(已截断)");
            break;
        }
    }

    if output.is_empty() {
        None
    } else {
        Some(output)
    }
}

/// Auto-refine execution plan after consecutive failures
pub async fn auto_refine_plan(
    db: std::sync::Arc<DbConn>,
    task_id: String,
    failure_info: String,
    app_handle: tauri::AppHandle,
) {
    use tauri::Emitter;

    let (task, settings) = {
        let result = (|| -> Result<_, String> {
            let task = crate::commands::tasks::query_task(&db, &task_id)?;
            let settings = crate::commands::tools::get_settings_core(&db)?;
            Ok((task, settings))
        })();
        match result {
            Ok(v) => v,
            Err(e) => {
                log::error!("auto_refine_plan: failed to load task/settings: {}", e);
                return;
            }
        }
    };

    let dir_context = scan_directory_context(&task.working_directory);

    match generate_execution_plan(
        &task.prompt,
        &task.name,
        &task.working_directory,
        dir_context.as_deref(),
        Some(&failure_info),
        &settings,
    )
    .await
    {
        Ok(plan) => {
            if let Ok(conn) = db.0.lock() {
                conn.execute(
                    "UPDATE tasks SET execution_plan = ?1 WHERE id = ?2",
                    rusqlite::params![plan, task_id],
                )
                .ok();
            }
            let _ = app_handle.emit("task:plan_generated", &task_id);
            log::info!("Auto-refined execution plan for task '{}'", task.name);
        }
        Err(e) => {
            log::error!("auto_refine_plan failed for '{}': {}", task.name, e);
        }
    }
}

/// Update execution plan in DB
pub fn update_execution_plan_core(db: &DbConn, task_id: &str, plan: &str) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tasks SET execution_plan = ?1 WHERE id = ?2",
        rusqlite::params![plan, task_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_directory_context_with_temp_dir() {
        let tmp = std::env::temp_dir().join("ai_cron_test_scan");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::write(tmp.join("src").join("lib.rs"), "// lib").unwrap();

        let result = scan_directory_context(tmp.to_str().unwrap());
        assert!(result.is_some());
        let ctx = result.unwrap();
        assert!(ctx.contains("文件列表:"));
        assert!(ctx.contains("main.rs"));
        assert!(ctx.contains("Cargo.toml"));
        assert!(ctx.contains("src")); // directory listed
        assert!(ctx.contains("[package]")); // Cargo.toml preview

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_directory_context_ignores_special_dirs() {
        let tmp = std::env::temp_dir().join("ai_cron_test_scan_ignore");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(tmp.join("node_modules")).unwrap();
        fs::create_dir_all(tmp.join(".git")).unwrap();
        fs::create_dir_all(tmp.join("target")).unwrap();
        fs::write(tmp.join("index.ts"), "console.log('hi')").unwrap();

        let result = scan_directory_context(tmp.to_str().unwrap());
        assert!(result.is_some());
        let ctx = result.unwrap();
        assert!(ctx.contains("index.ts"));
        assert!(!ctx.contains("node_modules"));
        assert!(!ctx.contains(".git"));
        assert!(!ctx.contains("target"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_directory_context_nonexistent() {
        let result = scan_directory_context("/nonexistent/path/ai_cron_test_xyz");
        assert!(result.is_none());
    }

    #[test]
    fn scan_directory_context_truncates_long_output() {
        let tmp = std::env::temp_dir().join("ai_cron_test_scan_truncate");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Create a big package.json to trigger truncation
        let big_content = "x".repeat(3000);
        fs::write(tmp.join("package.json"), &big_content).unwrap();

        let result = scan_directory_context(tmp.to_str().unwrap());
        assert!(result.is_some());
        let ctx = result.unwrap();
        assert!(ctx.len() <= 2100); // 2000 + "...(已截断)"

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn update_execution_plan_core_roundtrip() {
        let db = crate::db::init_db_memory();

        // Create a task first
        let req = crate::models::task::CreateTaskRequest {
            name: "Plan Test".to_string(),
            cron_expression: "0 9 * * *".to_string(),
            cron_human: None,
            ai_tool: None,
            custom_command: None,
            prompt: "test".to_string(),
            working_directory: "/tmp".to_string(),
            enabled: Some(true),
            inject_context: None,
            restrict_network: None,
            restrict_filesystem: None,
            env_vars: None,
            webhook_config: None,
            allowed_tools: None,
            skip_permissions: None,
        };
        let task = crate::commands::tasks::create_task_core(&db, &req).unwrap();
        assert!(task.execution_plan.is_empty());

        // Update the plan
        let plan = "## Plan\n1. Step A\n2. Step B";
        update_execution_plan_core(&db, &task.id, plan).unwrap();

        // Read back
        let updated = crate::commands::tasks::query_task(&db, &task.id).unwrap();
        assert_eq!(updated.execution_plan, plan);
        assert_eq!(updated.consecutive_failures, 0);
    }

    #[test]
    fn db_new_columns_exist() {
        let db = crate::db::init_db_memory();
        let conn = db.0.lock().unwrap();

        // Verify execution_plan and consecutive_failures columns exist
        let result = conn.execute(
            "INSERT INTO tasks (id, name, cron_expression, prompt, working_directory, created_at, updated_at, execution_plan, consecutive_failures)
             VALUES ('t1', 'test', '* * * * *', 'p', '/tmp', '2025-01-01', '2025-01-01', 'my plan', 5)",
            [],
        );
        assert!(result.is_ok(), "Should be able to insert with new columns");

        let (plan, failures): (String, i32) = conn.query_row(
            "SELECT execution_plan, consecutive_failures FROM tasks WHERE id = 't1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(plan, "my plan");
        assert_eq!(failures, 5);
    }

    #[test]
    fn consecutive_failures_defaults_to_zero() {
        let db = crate::db::init_db_memory();
        let conn = db.0.lock().unwrap();

        conn.execute(
            "INSERT INTO tasks (id, name, cron_expression, prompt, working_directory, created_at, updated_at)
             VALUES ('t2', 'test', '* * * * *', 'p', '/tmp', '2025-01-01', '2025-01-01')",
            [],
        ).unwrap();

        let (plan, failures): (String, i32) = conn.query_row(
            "SELECT execution_plan, consecutive_failures FROM tasks WHERE id = 't2'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(plan, ""); // DEFAULT ''
        assert_eq!(failures, 0); // DEFAULT 0
    }

    #[test]
    fn create_task_preserves_new_fields() {
        let db = crate::db::init_db_memory();

        let req = crate::models::task::CreateTaskRequest {
            name: "New Task".to_string(),
            cron_expression: "0 12 * * *".to_string(),
            cron_human: Some("每天中午".to_string()),
            ai_tool: None,
            custom_command: None,
            prompt: "do work".to_string(),
            working_directory: "/home/user".to_string(),
            enabled: Some(true),
            inject_context: Some(true),
            restrict_network: None,
            restrict_filesystem: None,
            env_vars: None,
            webhook_config: None,
            allowed_tools: None,
            skip_permissions: None,
        };
        let task = crate::commands::tasks::create_task_core(&db, &req).unwrap();

        // New task should have empty plan and 0 failures
        assert_eq!(task.execution_plan, "");
        assert_eq!(task.consecutive_failures, 0);

        // Update plan manually
        update_execution_plan_core(&db, &task.id, "Step 1: Do stuff").unwrap();

        // Verify via query_task
        let queried = crate::commands::tasks::query_task(&db, &task.id).unwrap();
        assert_eq!(queried.execution_plan, "Step 1: Do stuff");
        assert_eq!(queried.consecutive_failures, 0);
        assert_eq!(queried.name, "New Task");
        assert!(queried.inject_context);
    }

    #[test]
    fn update_task_with_execution_plan() {
        let db = crate::db::init_db_memory();

        let req = crate::models::task::CreateTaskRequest {
            name: "Update Plan Test".to_string(),
            cron_expression: "0 9 * * *".to_string(),
            cron_human: None,
            ai_tool: None,
            custom_command: None,
            prompt: "original".to_string(),
            working_directory: "/tmp".to_string(),
            enabled: Some(true),
            inject_context: None,
            restrict_network: None,
            restrict_filesystem: None,
            env_vars: None,
            webhook_config: None,
            allowed_tools: None,
            skip_permissions: None,
        };
        let task = crate::commands::tasks::create_task_core(&db, &req).unwrap();

        // Update via UpdateTaskRequest with execution_plan
        let update_req = crate::models::task::UpdateTaskRequest {
            name: Some("Updated Name".to_string()),
            cron_expression: None,
            cron_human: None,
            ai_tool: None,
            custom_command: None,
            prompt: None,
            working_directory: None,
            enabled: None,
            inject_context: None,
            restrict_network: None,
            restrict_filesystem: None,
            env_vars: None,
            webhook_config: None,
            allowed_tools: None,
            skip_permissions: None,
            execution_plan: Some("New plan content".to_string()),
        };
        let updated = crate::commands::tasks::update_task_core(&db, &task.id, &update_req).unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.execution_plan, "New plan content");
    }

    #[test]
    fn consecutive_failures_increment_in_db() {
        let db = crate::db::init_db_memory();
        let conn = db.0.lock().unwrap();

        conn.execute(
            "INSERT INTO tasks (id, name, cron_expression, prompt, working_directory, created_at, updated_at, consecutive_failures)
             VALUES ('t3', 'test', '* * * * *', 'p', '/tmp', '2025-01-01', '2025-01-01', 2)",
            [],
        ).unwrap();

        // Increment
        conn.execute(
            "UPDATE tasks SET consecutive_failures = consecutive_failures + 1 WHERE id = 't3'",
            [],
        ).unwrap();

        let count: i32 = conn.query_row(
            "SELECT consecutive_failures FROM tasks WHERE id = 't3'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 3);

        // Reset
        conn.execute(
            "UPDATE tasks SET consecutive_failures = 0 WHERE id = 't3'",
            [],
        ).unwrap();

        let count: i32 = conn.query_row(
            "SELECT consecutive_failures FROM tasks WHERE id = 't3'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }
}
