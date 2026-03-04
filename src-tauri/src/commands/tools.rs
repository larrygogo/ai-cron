use crate::db::DbConn;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub label: String,
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub install_url: String,
    pub install_cmd: Option<String>,
}

fn detect_tool(cmd: &str) -> (bool, Option<String>, Option<String>) {
    // Try `which` / `where` to find the path
    #[cfg(target_os = "windows")]
    let which_cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let which_cmd = "which";

    let path = std::process::Command::new(which_cmd)
        .arg(cmd)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    if path.is_none() {
        return (false, None, None);
    }

    // Try to get version
    let version = std::process::Command::new(cmd)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let combined = if out.is_empty() { err } else { out };
            if combined.is_empty() {
                None
            } else {
                Some(combined.lines().next().unwrap_or("").to_string())
            }
        });

    (true, version, path)
}

#[tauri::command]
pub fn detect_tools() -> Vec<ToolInfo> {
    let tools = vec![
        (
            "claude",
            "Claude Code",
            "https://docs.anthropic.com/en/docs/claude-code",
            None::<&str>,
        ),
        (
            "opencode",
            "OpenCode",
            "https://opencode.ai",
            Some("npm install -g opencode-ai"),
        ),
        (
            "codex",
            "Codex CLI",
            "https://github.com/openai/codex",
            Some("npm install -g @openai/codex"),
        ),
    ];

    tools
        .into_iter()
        .map(|(cmd, label, install_url, install_cmd)| {
            let (available, version, path) = detect_tool(cmd);
            ToolInfo {
                name: cmd.to_string(),
                label: label.to_string(),
                available,
                version,
                path,
                install_url: install_url.to_string(),
                install_cmd: install_cmd.map(|s| s.to_string()),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub nl_provider: String,
    pub nl_api_key: String,
    pub nl_base_url: String,
    pub nl_model: String,
    pub log_retention_days: i64,
    pub log_retention_per_task: i64,
    pub notify_on_success: bool,
    pub notify_on_failure: bool,
}

#[tauri::command]
pub fn get_settings(db: State<'_, DbConn>) -> Result<AppSettings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let get_val = |key: &str| -> String {
        conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get::<_, String>(0)
        })
        .unwrap_or_default()
    };

    let parse_bool = |s: String| s == "true";
    let parse_i64 = |s: String| s.parse::<i64>().unwrap_or(0);

    Ok(AppSettings {
        nl_provider: serde_json::from_str(&get_val("nl_provider"))
            .unwrap_or_else(|_| "claude".to_string()),
        nl_api_key: serde_json::from_str(&get_val("nl_api_key")).unwrap_or_default(),
        nl_base_url: serde_json::from_str(&get_val("nl_base_url")).unwrap_or_default(),
        nl_model: serde_json::from_str(&get_val("nl_model")).unwrap_or_default(),
        log_retention_days: parse_i64(get_val("log_retention_days")),
        log_retention_per_task: parse_i64(get_val("log_retention_per_task")),
        notify_on_success: parse_bool(get_val("notify_on_success")),
        notify_on_failure: parse_bool(get_val("notify_on_failure")),
    })
}

#[tauri::command]
pub fn update_settings(settings: AppSettings, db: State<'_, DbConn>) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let upsert = |key: &str, value: &str| -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    };

    upsert(
        "nl_provider",
        &serde_json::to_string(&settings.nl_provider).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    upsert(
        "nl_api_key",
        &serde_json::to_string(&settings.nl_api_key).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    upsert(
        "nl_base_url",
        &serde_json::to_string(&settings.nl_base_url).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    upsert(
        "nl_model",
        &serde_json::to_string(&settings.nl_model).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    upsert(
        "log_retention_days",
        &settings.log_retention_days.to_string(),
    )
    .map_err(|e| e.to_string())?;
    upsert(
        "log_retention_per_task",
        &settings.log_retention_per_task.to_string(),
    )
    .map_err(|e| e.to_string())?;
    upsert(
        "notify_on_success",
        if settings.notify_on_success {
            "true"
        } else {
            "false"
        },
    )
    .map_err(|e| e.to_string())?;
    upsert(
        "notify_on_failure",
        if settings.notify_on_failure {
            "true"
        } else {
            "false"
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
