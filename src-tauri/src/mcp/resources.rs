use crate::commands::{
    runs::{query_run, query_runs},
    tasks::{query_all_tasks, query_task},
    tools::get_settings_core,
};
use crate::db::DbConn;
use rmcp::model::*;
use rmcp::ErrorData as McpError;

pub async fn list_resources(_db: &DbConn) -> Result<ListResourcesResult, McpError> {
    let resources = vec![
        RawResource::new("aicron://tasks", "All Tasks")
            .with_description("List of all scheduled tasks")
            .with_mime_type("application/json")
            .no_annotation(),
        RawResource::new("aicron://settings", "App Settings")
            .with_description("Current application settings")
            .with_mime_type("application/json")
            .no_annotation(),
    ];

    Ok(ListResourcesResult {
        resources,
        ..Default::default()
    })
}

pub fn list_resource_templates() -> Result<ListResourceTemplatesResult, McpError> {
    let resource_templates = vec![
        RawResourceTemplate::new("aicron://tasks/{task_id}", "Task Details")
            .with_description("Details of a specific task")
            .with_mime_type("application/json")
            .no_annotation(),
        RawResourceTemplate::new("aicron://tasks/{task_id}/runs", "Task Runs")
            .with_description("Recent runs for a specific task (last 20)")
            .with_mime_type("application/json")
            .no_annotation(),
        RawResourceTemplate::new("aicron://runs/{run_id}", "Run Details")
            .with_description("Details of a specific run including full output")
            .with_mime_type("application/json")
            .no_annotation(),
    ];

    Ok(ListResourceTemplatesResult {
        resource_templates,
        ..Default::default()
    })
}

pub async fn read_resource(db: &DbConn, uri: &str) -> Result<ReadResourceResult, McpError> {
    let json = match uri {
        "aicron://tasks" => {
            let tasks =
                query_all_tasks(db).map_err(|e| McpError::internal_error(e, None))?;
            serde_json::to_string_pretty(&tasks)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        "aicron://settings" => {
            let settings =
                get_settings_core(db).map_err(|e| McpError::internal_error(e, None))?;
            serde_json::to_string_pretty(&settings)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        _ if uri.starts_with("aicron://tasks/") && uri.ends_with("/runs") => {
            let task_id = uri
                .strip_prefix("aicron://tasks/")
                .and_then(|s| s.strip_suffix("/runs"))
                .ok_or_else(|| McpError::invalid_params("Invalid URI format", None))?;
            let runs =
                query_runs(db, task_id, 20).map_err(|e| McpError::internal_error(e, None))?;
            serde_json::to_string_pretty(&runs)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        _ if uri.starts_with("aicron://tasks/") => {
            let task_id = uri
                .strip_prefix("aicron://tasks/")
                .ok_or_else(|| McpError::invalid_params("Invalid URI format", None))?;
            let task =
                query_task(db, task_id).map_err(|e| McpError::internal_error(e, None))?;
            serde_json::to_string_pretty(&task)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        _ if uri.starts_with("aicron://runs/") => {
            let run_id = uri
                .strip_prefix("aicron://runs/")
                .ok_or_else(|| McpError::invalid_params("Invalid URI format", None))?;
            let run =
                query_run(db, run_id).map_err(|e| McpError::internal_error(e, None))?;
            serde_json::to_string_pretty(&run)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        _ => {
            return Err(McpError::resource_not_found(
                format!("Unknown resource URI: {}", uri),
                None,
            ));
        }
    };

    Ok(ReadResourceResult::new(vec![ResourceContents::text(json, uri)]))
}
