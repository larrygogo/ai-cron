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
    pub timezone: String,
    pub mcp_server_enabled: bool,
    pub mcp_server_port: u16,
}

/// Core: get settings (no Tauri dependency)
pub fn get_settings_core(db: &DbConn) -> Result<AppSettings, String> {
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
        timezone: {
            let raw = get_val("timezone");
            let tz: String = serde_json::from_str(&raw).unwrap_or_else(|_| "system".to_string());
            if tz.is_empty() { "system".to_string() } else { tz }
        },
        mcp_server_enabled: parse_bool(get_val("mcp_server_enabled")),
        mcp_server_port: get_val("mcp_server_port").parse::<u16>().unwrap_or(23987),
    })
}

#[tauri::command]
pub fn get_settings(db: State<'_, DbConn>) -> Result<AppSettings, String> {
    get_settings_core(&db)
}

/// Core: update settings (no Tauri dependency)
pub fn update_settings_core(db: &DbConn, settings: &AppSettings) -> Result<(), String> {
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
    upsert(
        "timezone",
        &serde_json::to_string(&settings.timezone).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    upsert(
        "mcp_server_enabled",
        if settings.mcp_server_enabled {
            "true"
        } else {
            "false"
        },
    )
    .map_err(|e| e.to_string())?;
    upsert(
        "mcp_server_port",
        &settings.mcp_server_port.to_string(),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn update_settings(settings: AppSettings, db: State<'_, DbConn>) -> Result<(), String> {
    update_settings_core(&db, &settings)
}

#[tauri::command]
pub fn get_system_timezone() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string())
}

// ── MCP Status ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStatus {
    pub running: bool,
    pub port: u16,
}

#[tauri::command]
pub fn get_mcp_status(
    app_handle: tauri::AppHandle,
) -> McpStatus {
    use tauri::Manager;
    match app_handle.try_state::<crate::mcp::McpState>() {
        Some(state) => McpStatus {
            running: true,
            port: state.port,
        },
        None => McpStatus {
            running: false,
            port: 0,
        },
    }
}

#[tauri::command]
pub fn repair_mcp_config(
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    use tauri::Manager;
    let port = match app_handle.try_state::<crate::mcp::McpState>() {
        Some(state) => state.port,
        None => return Err("MCP 服务未运行".to_string()),
    };
    auto_configure_claude_mcp(port)
}

/// Auto-configure ~/.claude.json with MCP bridge
pub fn auto_configure_claude_mcp(port: u16) -> Result<String, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "无法获取 HOME 目录".to_string())?;
    let home_path = std::path::PathBuf::from(&home);
    let claude_config = home_path.join(".claude.json");

    // Find bridge script path relative to executable
    let bridge_path = find_bridge_path();

    let mut config: serde_json::Value = if claude_config.exists() {
        let content = std::fs::read_to_string(&claude_config)
            .map_err(|e| format!("读取 ~/.claude.json 失败: {}", e))?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Ensure mcpServers exists
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = serde_json::json!({});
    }

    // Set ai-cron entry
    config["mcpServers"]["ai-cron"] = serde_json::json!({
        "type": "stdio",
        "command": "node",
        "args": [bridge_path]
    });

    let output = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&claude_config, &output)
        .map_err(|e| format!("写入 ~/.claude.json 失败: {}", e))?;

    log::info!("Auto-configured ~/.claude.json for MCP (port {})", port);
    Ok(format!("已配置到 {}", claude_config.display()))
}

fn find_bridge_path() -> String {
    // Try to find mcp-bridge.mjs relative to current executable
    if let Ok(exe) = std::env::current_exe() {
        // In dev: src-tauri/target/debug/ai-cron.exe -> src-tauri/mcp-bridge.mjs
        // In prod: installation_dir/ai-cron.exe -> installation_dir/mcp-bridge.mjs
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("mcp-bridge.mjs");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
            // Dev mode: go up from target/debug/
            let dev_candidate = dir
                .parent() // target
                .and_then(|p| p.parent()) // src-tauri
                .map(|p| p.join("mcp-bridge.mjs"));
            if let Some(c) = dev_candidate {
                if c.exists() {
                    return c.to_string_lossy().to_string();
                }
            }
        }
    }
    // Fallback: use a placeholder path
    "<安装路径>/mcp-bridge.mjs".to_string()
}
