use super::server::AiCronMcp;
use crate::commands::{
    runs::{cleanup_runs_core, query_all_runs, query_run, query_runs},
    tasks::{
        create_task_core, delete_task_core, query_all_tasks, query_task, set_task_enabled_core,
        update_task_core,
    },
    tools::{get_settings_core, update_settings_core},
};
use crate::models::run::TriggerSource;
use crate::models::task::{CreateTaskRequest, UpdateTaskRequest, WebhookConfig};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{tool, tool_router, ErrorData as McpError};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;

// ── Tool Parameter Types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskIdParam {
    /// Task ID
    pub task_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunIdParam {
    /// Run ID
    pub run_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateTaskParam {
    /// Task name
    pub name: String,
    /// Cron expression (5-field: minute hour dom month dow)
    pub cron_expression: String,
    /// Human-readable schedule description
    pub cron_human: Option<String>,
    /// Prompt for the AI agent
    pub prompt: String,
    /// Working directory path
    pub working_directory: String,
    /// AI tool: "claude" or "custom"
    pub ai_tool: Option<String>,
    /// Custom command template (for ai_tool="custom")
    pub custom_command: Option<String>,
    /// Whether to enable the task immediately (default: true)
    pub enabled: Option<bool>,
    /// Inject context (last run info) into prompt
    pub inject_context: Option<bool>,
    /// Environment variables
    pub env_vars: Option<HashMap<String, String>>,
    /// Webhook notification config
    pub webhook_config: Option<WebhookConfig>,
    /// Allowed tools for Claude CLI (e.g. ["Bash(gh *)", "Read"])
    pub allowed_tools: Option<Vec<String>>,
    /// Skip permission prompts (--dangerously-skip-permissions)
    pub skip_permissions: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateTaskParam {
    /// Task ID to update
    pub task_id: String,
    /// New task name
    pub name: Option<String>,
    /// New cron expression
    pub cron_expression: Option<String>,
    /// New human-readable schedule
    pub cron_human: Option<String>,
    /// New prompt
    pub prompt: Option<String>,
    /// New working directory
    pub working_directory: Option<String>,
    /// New AI tool
    pub ai_tool: Option<String>,
    /// New custom command
    pub custom_command: Option<String>,
    /// Enable/disable
    pub enabled: Option<bool>,
    /// Inject context toggle
    pub inject_context: Option<bool>,
    /// New environment variables
    pub env_vars: Option<HashMap<String, String>>,
    /// Webhook notification config (set to null to remove)
    pub webhook_config: Option<WebhookConfig>,
    /// Allowed tools for Claude CLI
    pub allowed_tools: Option<Vec<String>>,
    /// Skip permission prompts (--dangerously-skip-permissions)
    pub skip_permissions: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetTaskEnabledParam {
    /// Task ID
    pub task_id: String,
    /// Enable (true) or disable (false)
    pub enabled: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListRunsParam {
    /// Filter by task ID (optional)
    pub task_id: Option<String>,
    /// Max number of runs to return (default: 20)
    pub limit: Option<i64>,
    /// Filter by status: "success", "failed", "running", "killed"
    pub status_filter: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PreviewScheduleParam {
    /// Cron expression to preview (5-field)
    pub cron_expression: String,
    /// Number of upcoming runs to show (default: 5, max: 20)
    pub count: Option<usize>,
    /// Timezone (e.g. "Asia/Shanghai", default: system)
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdatePlanParam {
    /// Task ID
    pub task_id: String,
    /// New execution plan content (Markdown)
    pub plan: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParseNlParam {
    /// Natural language task description (e.g. "Every weekday at 9am, run code review")
    pub input: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateSettingsParam {
    /// Natural language provider
    pub nl_provider: Option<String>,
    /// API key for NL provider
    pub nl_api_key: Option<String>,
    /// Base URL for NL provider
    pub nl_base_url: Option<String>,
    /// Model name
    pub nl_model: Option<String>,
    /// Log retention days
    pub log_retention_days: Option<i64>,
    /// Log retention per task
    pub log_retention_per_task: Option<i64>,
    /// Notify on success
    pub notify_on_success: Option<bool>,
    /// Notify on failure
    pub notify_on_failure: Option<bool>,
    /// Timezone
    pub timezone: Option<String>,
    /// MCP server enabled
    pub mcp_server_enabled: Option<bool>,
    /// MCP server port
    pub mcp_server_port: Option<u16>,
}

// ── Tool Implementations ────────────────────────────────────────────────────

#[tool_router(vis = "pub(crate)")]
impl AiCronMcp {
    #[tool(description = "List all scheduled tasks")]
    async fn list_tasks(&self) -> Result<CallToolResult, McpError> {
        let tasks = query_all_tasks(&self.db).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&tasks)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get details of a specific task")]
    async fn get_task(
        &self,
        params: Parameters<TaskIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let task = query_task(&self.db, &params.0.task_id)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&task)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Create a new scheduled task. Returns the created task.")]
    async fn create_task(
        &self,
        params: Parameters<CreateTaskParam>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let req = CreateTaskRequest {
            name: p.name,
            cron_expression: p.cron_expression,
            cron_human: p.cron_human,
            ai_tool: p.ai_tool,
            custom_command: p.custom_command,
            prompt: p.prompt,
            working_directory: p.working_directory,
            enabled: p.enabled,
            inject_context: p.inject_context,
            restrict_network: Some(false),
            restrict_filesystem: Some(false),
            env_vars: p.env_vars,
            webhook_config: p.webhook_config,
            allowed_tools: p.allowed_tools,
            skip_permissions: p.skip_permissions,
        };

        let task =
            create_task_core(&self.db, &req).map_err(|e| McpError::internal_error(e, None))?;

        if task.enabled {
            self.scheduler
                .add_task(task.clone(), self.db.clone(), self.app_handle.clone())
                .await
                .ok();
        }

        // Async generate execution plan
        {
            let db_clone = self.db.clone();
            let task_clone = task.clone();
            tokio::spawn(async move {
                use crate::commands::plan_gen::{generate_execution_plan, scan_directory_context, update_execution_plan_core};
                let dir_context = scan_directory_context(&task_clone.working_directory);
                if let Ok(settings) = get_settings_core(&db_clone) {
                    if let Ok(plan) = generate_execution_plan(
                        &task_clone.prompt,
                        &task_clone.name,
                        &task_clone.working_directory,
                        dir_context.as_deref(),
                        None,
                        &settings,
                    ).await {
                        update_execution_plan_core(&db_clone, &task_clone.id, &plan).ok();
                        log::info!("MCP: Generated execution plan for task '{}'", task_clone.name);
                    }
                }
            });
        }

        let json = serde_json::to_string_pretty(&task)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update an existing task. Only provided fields will be changed.")]
    async fn update_task(
        &self,
        params: Parameters<UpdateTaskParam>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let task_id = p.task_id.clone();
        let req = UpdateTaskRequest {
            name: p.name,
            cron_expression: p.cron_expression,
            cron_human: p.cron_human,
            ai_tool: p.ai_tool,
            custom_command: p.custom_command,
            prompt: p.prompt,
            working_directory: p.working_directory,
            enabled: p.enabled,
            inject_context: p.inject_context,
            restrict_network: None,
            restrict_filesystem: None,
            env_vars: p.env_vars,
            webhook_config: p.webhook_config.map(Some),
            allowed_tools: p.allowed_tools,
            skip_permissions: p.skip_permissions,
            execution_plan: None,
        };

        let task = update_task_core(&self.db, &task_id, &req)
            .map_err(|e| McpError::internal_error(e, None))?;

        if task.enabled {
            self.scheduler
                .add_task(task.clone(), self.db.clone(), self.app_handle.clone())
                .await
                .ok();
        } else {
            self.scheduler.remove_task(&task.id).await.ok();
        }

        let json = serde_json::to_string_pretty(&task)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Delete a task and remove it from the scheduler")]
    async fn delete_task(
        &self,
        params: Parameters<TaskIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let task_id = &params.0.task_id;
        delete_task_core(&self.db, task_id)
            .map_err(|e| McpError::internal_error(e, None))?;
        self.scheduler.remove_task(task_id).await.ok();
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Task {} deleted",
            task_id
        ))]))
    }

    #[tool(description = "Enable or disable a task")]
    async fn set_task_enabled(
        &self,
        params: Parameters<SetTaskEnabledParam>,
    ) -> Result<CallToolResult, McpError> {
        let p = &params.0;
        let task = set_task_enabled_core(&self.db, &p.task_id, p.enabled)
            .map_err(|e| McpError::internal_error(e, None))?;

        if p.enabled {
            self.scheduler
                .add_task(task, self.db.clone(), self.app_handle.clone())
                .await
                .ok();
        } else {
            self.scheduler.remove_task(&p.task_id).await.ok();
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Task {} {}",
            p.task_id,
            if p.enabled { "enabled" } else { "disabled" }
        ))]))
    }

    #[tool(description = "Manually trigger a task execution. Returns the run ID.")]
    async fn trigger_task(
        &self,
        params: Parameters<TaskIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let task = query_task(&self.db, &params.0.task_id)
            .map_err(|e| McpError::internal_error(e, None))?;
        let run_id = uuid::Uuid::new_v4().to_string();
        let db = self.db.clone();
        let app_handle = self.app_handle.clone();
        let run_id_clone = run_id.clone();

        tokio::spawn(crate::commands::runner::execute_task(
            task,
            TriggerSource::Manual,
            app_handle,
            db,
            run_id_clone,
        ));

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Task triggered. Run ID: {}",
            run_id
        ))]))
    }

    #[tool(description = "Kill a currently running task execution")]
    async fn kill_run(
        &self,
        params: Parameters<RunIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let run_id = &params.0.run_id;
        let pid = {
            let reg = crate::commands::runner::PROCESS_REGISTRY
                .lock()
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            reg.get(run_id).copied()
        };

        if let Some(pid) = pid {
            if let Ok(mut reg) = crate::commands::runner::PROCESS_REGISTRY.lock() {
                reg.remove(run_id);
            }

            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .output()
                    .ok();
            }
            #[cfg(not(target_os = "windows"))]
            {
                std::process::Command::new("kill")
                    .args(["-TERM", &format!("-{}", pid)])
                    .output()
                    .ok();
            }

            let ended_at = chrono::Utc::now();
            {
                let conn = self
                    .db
                    .0
                    .lock()
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                conn.execute(
                    "UPDATE runs SET status='killed', ended_at=?1 WHERE id=?2",
                    rusqlite::params![ended_at.to_rfc3339(), run_id],
                )
                .ok();
            }

            Ok(CallToolResult::success(vec![Content::text(format!(
                "Run {} killed",
                run_id
            ))]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "Run {} not found in active processes",
                run_id
            ))]))
        }
    }

    #[tool(description = "List run history. Can filter by task_id and status.")]
    async fn list_runs(
        &self,
        params: Parameters<ListRunsParam>,
    ) -> Result<CallToolResult, McpError> {
        let p = &params.0;
        let limit = p.limit.unwrap_or(20).min(100);

        let json = if let Some(task_id) = &p.task_id {
            let runs = query_runs(&self.db, task_id, limit)
                .map_err(|e| McpError::internal_error(e, None))?;
            serde_json::to_string_pretty(&runs)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        } else {
            let runs = query_all_runs(&self.db, limit, 0, p.status_filter.as_deref(), None)
                .map_err(|e| McpError::internal_error(e, None))?;
            serde_json::to_string_pretty(&runs)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get details of a specific run including stdout/stderr output")]
    async fn get_run(
        &self,
        params: Parameters<RunIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let run = query_run(&self.db, &params.0.run_id)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&run)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Clean up old run records based on retention policy")]
    async fn cleanup_runs(&self) -> Result<CallToolResult, McpError> {
        let deleted =
            cleanup_runs_core(&self.db).map_err(|e| McpError::internal_error(e, None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Cleaned up {} old run records",
            deleted
        ))]))
    }

    #[tool(description = "Preview upcoming run times for a cron expression. Returns ISO 8601 timestamps.")]
    async fn preview_schedule(
        &self,
        params: Parameters<PreviewScheduleParam>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let result = crate::commands::scheduler::preview_next_runs(
            p.cron_expression,
            p.count,
            p.timezone,
        )
        .await
        .map_err(|e| McpError::internal_error(e, None))?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Parse natural language into a task configuration (name, cron, prompt)")]
    async fn parse_natural_language(
        &self,
        params: Parameters<ParseNlParam>,
    ) -> Result<CallToolResult, McpError> {
        let settings =
            get_settings_core(&self.db).map_err(|e| McpError::internal_error(e, None))?;

        let draft = match settings.nl_provider.as_str() {
            "ollama" => {
                crate::commands::ai_parse::parse_with_ollama(&params.0.input, &settings).await
            }
            "openai" => {
                crate::commands::ai_parse::parse_with_openai(&params.0.input, &settings).await
            }
            "claude_cli" => crate::commands::ai_parse::parse_with_cli(&params.0.input).await,
            _ => crate::commands::ai_parse::parse_with_claude(&params.0.input, &settings).await,
        }
        .map_err(|e| McpError::internal_error(e, None))?;

        let json = serde_json::to_string_pretty(&draft)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get current application settings")]
    async fn get_settings(&self) -> Result<CallToolResult, McpError> {
        let settings =
            get_settings_core(&self.db).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&settings)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Generate or regenerate an execution plan for a task. Scans the working directory for project context.")]
    async fn generate_plan(
        &self,
        params: Parameters<TaskIdParam>,
    ) -> Result<CallToolResult, McpError> {
        use crate::commands::plan_gen::{generate_execution_plan, scan_directory_context, update_execution_plan_core};

        let task = query_task(&self.db, &params.0.task_id)
            .map_err(|e| McpError::internal_error(e, None))?;
        let settings = get_settings_core(&self.db)
            .map_err(|e| McpError::internal_error(e, None))?;
        let dir_context = scan_directory_context(&task.working_directory);

        let plan = generate_execution_plan(
            &task.prompt,
            &task.name,
            &task.working_directory,
            dir_context.as_deref(),
            None,
            &settings,
        )
        .await
        .map_err(|e| McpError::internal_error(e, None))?;

        update_execution_plan_core(&self.db, &params.0.task_id, &plan)
            .map_err(|e| McpError::internal_error(e, None))?;

        Ok(CallToolResult::success(vec![Content::text(plan)]))
    }

    #[tool(description = "Manually update the execution plan for a task")]
    async fn update_plan(
        &self,
        params: Parameters<UpdatePlanParam>,
    ) -> Result<CallToolResult, McpError> {
        use crate::commands::plan_gen::update_execution_plan_core;

        update_execution_plan_core(&self.db, &params.0.task_id, &params.0.plan)
            .map_err(|e| McpError::internal_error(e, None))?;

        Ok(CallToolResult::success(vec![Content::text(
            "Execution plan updated successfully",
        )]))
    }

    #[tool(description = "Update application settings. Only provided fields will be changed.")]
    async fn update_settings(
        &self,
        params: Parameters<UpdateSettingsParam>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let mut settings =
            get_settings_core(&self.db).map_err(|e| McpError::internal_error(e, None))?;

        if let Some(v) = p.nl_provider { settings.nl_provider = v; }
        if let Some(v) = p.nl_api_key { settings.nl_api_key = v; }
        if let Some(v) = p.nl_base_url { settings.nl_base_url = v; }
        if let Some(v) = p.nl_model { settings.nl_model = v; }
        if let Some(v) = p.log_retention_days { settings.log_retention_days = v; }
        if let Some(v) = p.log_retention_per_task { settings.log_retention_per_task = v; }
        if let Some(v) = p.notify_on_success { settings.notify_on_success = v; }
        if let Some(v) = p.notify_on_failure { settings.notify_on_failure = v; }
        if let Some(v) = p.timezone { settings.timezone = v; }
        if let Some(v) = p.mcp_server_enabled { settings.mcp_server_enabled = v; }
        if let Some(v) = p.mcp_server_port { settings.mcp_server_port = v; }

        update_settings_core(&self.db, &settings)
            .map_err(|e| McpError::internal_error(e, None))?;

        Ok(CallToolResult::success(vec![Content::text(
            "Settings updated successfully",
        )]))
    }
}
