use crate::commands::{
    runs::{query_run, query_runs},
    tasks::{query_all_tasks, query_task},
};
use crate::db::DbConn;
use rmcp::model::*;
use rmcp::ErrorData as McpError;
use std::collections::HashMap;

pub fn list_prompts() -> Result<ListPromptsResult, McpError> {
    let prompts = vec![
        Prompt::new(
            "create_task_guide",
            Some("Guide for creating a new scheduled task. Helps parse cron expressions, generate prompts, and check conflicts."),
            Some(vec![PromptArgument::new("description").with_description("Natural language description of the task to create").with_required(true)]),
        ),
        Prompt::new(
            "diagnose_run",
            Some("Diagnose a failed or problematic run. Pulls full task config and run logs."),
            Some(vec![PromptArgument::new("run_id").with_description("The run ID to diagnose").with_required(true)]),
        ),
        Prompt::new(
            "task_status_report",
            Some("Generate a status report of all tasks: enabled/disabled, success rate, recent failures."),
            None,
        ),
        Prompt::new(
            "optimize_schedule",
            Some("Analyze a task's run history and suggest schedule optimizations."),
            Some(vec![PromptArgument::new("task_id").with_description("The task ID to analyze").with_required(true)]),
        ),
    ];

    Ok(ListPromptsResult {
        prompts,
        ..Default::default()
    })
}

pub async fn get_prompt(
    db: &DbConn,
    name: &str,
    arguments: Option<HashMap<String, String>>,
) -> Result<GetPromptResult, McpError> {
    let args = arguments.unwrap_or_default();

    match name {
        "create_task_guide" => {
            let description = args
                .get("description")
                .ok_or_else(|| McpError::invalid_params("Missing 'description' argument", None))?;

            let tasks =
                query_all_tasks(db).map_err(|e| McpError::internal_error(e, None))?;
            let existing_schedules: Vec<String> = tasks
                .iter()
                .map(|t| {
                    format!(
                        "- {} [{}] ({})",
                        t.name,
                        t.cron_expression,
                        if t.enabled { "enabled" } else { "disabled" }
                    )
                })
                .collect();

            let content = format!(
                "The user wants to create a new scheduled task:\n\n\
                 \"{}\"\n\n\
                 ## Existing Tasks\n{}\n\n\
                 ## Instructions\n\
                 1. Parse the description to determine the cron schedule\n\
                 2. Generate a clear, actionable prompt for the AI agent\n\
                 3. Check for schedule conflicts with existing tasks\n\
                 4. Use the `create_task` tool to create the task\n\n\
                 ## Notes\n\
                 - Use standard 5-field cron (minute hour dom month dow)\n\
                 - Default AI tool is 'claude'\n\
                 - Always set a meaningful working directory",
                description,
                if existing_schedules.is_empty() {
                    "No existing tasks.".to_string()
                } else {
                    existing_schedules.join("\n")
                }
            );

            Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                PromptMessageRole::User,
                content,
            )])
            .with_description("Guide for creating a scheduled task"))
        }

        "diagnose_run" => {
            let run_id = args
                .get("run_id")
                .ok_or_else(|| McpError::invalid_params("Missing 'run_id' argument", None))?;

            let run =
                query_run(db, run_id).map_err(|e| McpError::internal_error(e, None))?;
            let task =
                query_task(db, &run.task_id).map_err(|e| McpError::internal_error(e, None))?;

            let content = format!(
                "## Run Diagnosis\n\n\
                 ### Task Configuration\n\
                 - Name: {}\n\
                 - AI Tool: {}\n\
                 - Cron: {} ({})\n\
                 - Working Directory: {}\n\
                 - Prompt:\n```\n{}\n```\n\n\
                 ### Run Details\n\
                 - Run ID: {}\n\
                 - Status: {}\n\
                 - Exit Code: {}\n\
                 - Started: {}\n\
                 - Ended: {}\n\
                 - Duration: {}ms\n\
                 - Triggered By: {}\n\n\
                 ### stdout\n```\n{}\n```\n\n\
                 ### stderr\n```\n{}\n```\n\n\
                 ## Instructions\n\
                 Analyze the run output and diagnose:\n\
                 1. What went wrong (if failed)\n\
                 2. Root cause analysis\n\
                 3. Suggested fixes (prompt changes, directory issues, tool problems)\n\
                 4. Whether the task configuration needs updating",
                task.name,
                task.ai_tool.as_str(),
                task.cron_expression,
                task.cron_human,
                task.working_directory,
                task.prompt,
                run.id,
                run.status.as_str(),
                run.exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
                run.started_at.to_rfc3339(),
                run.ended_at
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_else(|| "N/A".to_string()),
                run.duration_ms.unwrap_or(0),
                run.triggered_by.as_str(),
                if run.stdout.is_empty() {
                    "(empty)"
                } else {
                    &run.stdout
                },
                if run.stderr.is_empty() {
                    "(empty)"
                } else {
                    &run.stderr
                },
            );

            Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                PromptMessageRole::User,
                content,
            )])
            .with_description(format!("Diagnose run {}", run_id)))
        }

        "task_status_report" => {
            let tasks =
                query_all_tasks(db).map_err(|e| McpError::internal_error(e, None))?;

            let mut report = String::from("## Task Status Report\n\n");

            if tasks.is_empty() {
                report.push_str("No tasks configured.\n");
            } else {
                report.push_str("| Task | Schedule | Status | Last Run | Last Status |\n");
                report.push_str("|------|----------|--------|----------|-------------|\n");

                for task in &tasks {
                    let status = if task.enabled { "Enabled" } else { "Disabled" };
                    let last_run = task
                        .last_run_at
                        .map(|t| t.format("%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "Never".to_string());
                    let last_status = task.last_run_status.as_deref().unwrap_or("N/A");

                    report.push_str(&format!(
                        "| {} | {} | {} | {} | {} |\n",
                        task.name, task.cron_human, status, last_run, last_status
                    ));
                }

                let total = tasks.len();
                let enabled = tasks.iter().filter(|t| t.enabled).count();
                let recent_failures = tasks
                    .iter()
                    .filter(|t| t.last_run_status.as_deref() == Some("failed"))
                    .count();

                report.push_str(&format!(
                    "\n### Summary\n- Total tasks: {}\n- Enabled: {}\n- Disabled: {}\n- Recent failures: {}\n",
                    total, enabled, total - enabled, recent_failures
                ));
            }

            report.push_str(
                "\n## Instructions\n\
                 Review this report and provide:\n\
                 1. Overall health assessment\n\
                 2. Any tasks that need attention\n\
                 3. Recommendations for improvement",
            );

            Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                PromptMessageRole::User,
                report,
            )])
            .with_description("Task status report"))
        }

        "optimize_schedule" => {
            let task_id = args
                .get("task_id")
                .ok_or_else(|| McpError::invalid_params("Missing 'task_id' argument", None))?;

            let task =
                query_task(db, task_id).map_err(|e| McpError::internal_error(e, None))?;
            let runs =
                query_runs(db, task_id, 50).map_err(|e| McpError::internal_error(e, None))?;

            let mut content = format!(
                "## Schedule Optimization Analysis\n\n\
                 ### Task: {}\n\
                 - Current Schedule: {} ({})\n\
                 - Working Directory: {}\n\n\
                 ### Recent Run History ({} runs)\n",
                task.name,
                task.cron_expression,
                task.cron_human,
                task.working_directory,
                runs.len()
            );

            if runs.is_empty() {
                content.push_str("No run history available.\n");
            } else {
                let success_count =
                    runs.iter().filter(|r| r.status.as_str() == "success").count();
                let failed_count =
                    runs.iter().filter(|r| r.status.as_str() == "failed").count();
                let durations: Vec<u64> = runs.iter().filter_map(|r| r.duration_ms).collect();

                let avg_duration = if durations.is_empty() {
                    0
                } else {
                    durations.iter().sum::<u64>() / durations.len() as u64
                };
                let max_duration = durations.iter().max().copied().unwrap_or(0);
                let min_duration = durations.iter().min().copied().unwrap_or(0);

                content.push_str(&format!(
                    "- Success: {} / Failed: {} (Success rate: {:.0}%)\n\
                     - Duration — Avg: {}ms, Min: {}ms, Max: {}ms\n\n",
                    success_count,
                    failed_count,
                    if runs.is_empty() {
                        0.0
                    } else {
                        success_count as f64 / runs.len() as f64 * 100.0
                    },
                    avg_duration,
                    min_duration,
                    max_duration
                ));

                content.push_str(
                    "### Recent Runs\n| Time | Status | Duration |\n|------|--------|----------|\n",
                );
                for run in runs.iter().take(10) {
                    content.push_str(&format!(
                        "| {} | {} | {}ms |\n",
                        run.started_at.format("%m-%d %H:%M"),
                        run.status.as_str(),
                        run.duration_ms.unwrap_or(0)
                    ));
                }
            }

            content.push_str(
                "\n## Instructions\n\
                 Based on the run history, suggest:\n\
                 1. Whether the current schedule frequency is appropriate\n\
                 2. If the task runs too long, suggest less frequent scheduling\n\
                 3. If failures are time-correlated, suggest schedule adjustments\n\
                 4. A new cron expression if changes are warranted",
            );

            Ok(
                GetPromptResult::new(vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    content,
                )])
                .with_description(format!("Schedule optimization for task {}", task.name)),
            )
        }

        _ => Err(McpError::invalid_params(
            format!("Unknown prompt: {}", name),
            None,
        )),
    }
}
