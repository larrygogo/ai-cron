use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AiTool {
    Claude,
    Custom,
}

impl AiTool {
    pub fn as_str(&self) -> &str {
        match self {
            AiTool::Claude => "claude",
            AiTool::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "custom" => AiTool::Custom,
            _ => AiTool::Claude,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WebhookConfig {
    pub url: String,
    pub platform: String, // "feishu" | "generic"
    pub on_start: bool,
    pub on_success: bool,
    pub on_failure: bool,
    pub on_killed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub cron_expression: String,
    pub cron_human: String,
    pub ai_tool: AiTool,
    pub custom_command: Option<String>,
    pub prompt: String,
    pub working_directory: String,
    pub enabled: bool,
    pub inject_context: bool,
    pub restrict_network: bool,
    pub restrict_filesystem: bool,
    pub env_vars: HashMap<String, String>,
    pub webhook_config: Option<WebhookConfig>,
    pub allowed_tools: Vec<String>,
    pub skip_permissions: bool,
    pub execution_plan: String,
    pub consecutive_failures: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_run_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub cron_expression: String,
    pub cron_human: Option<String>,
    pub ai_tool: Option<String>,
    pub custom_command: Option<String>,
    pub prompt: String,
    pub working_directory: String,
    pub enabled: Option<bool>,
    pub inject_context: Option<bool>,
    pub restrict_network: Option<bool>,
    pub restrict_filesystem: Option<bool>,
    pub env_vars: Option<HashMap<String, String>>,
    pub webhook_config: Option<WebhookConfig>,
    pub allowed_tools: Option<Vec<String>>,
    pub skip_permissions: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub cron_expression: Option<String>,
    pub cron_human: Option<String>,
    pub ai_tool: Option<String>,
    pub custom_command: Option<String>,
    pub prompt: Option<String>,
    pub working_directory: Option<String>,
    pub enabled: Option<bool>,
    pub inject_context: Option<bool>,
    pub restrict_network: Option<bool>,
    pub restrict_filesystem: Option<bool>,
    pub env_vars: Option<HashMap<String, String>>,
    pub webhook_config: Option<Option<WebhookConfig>>,
    pub allowed_tools: Option<Vec<String>>,
    pub skip_permissions: Option<bool>,
    pub execution_plan: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_tool_from_str_as_str_roundtrip() {
        let cases = vec![
            ("claude", AiTool::Claude),
            ("custom", AiTool::Custom),
        ];
        for (s, expected) in &cases {
            let tool = AiTool::from_str(s);
            assert_eq!(&tool, expected);
            assert_eq!(tool.as_str(), *s);
        }
    }

    #[test]
    fn ai_tool_from_str_unknown_defaults_to_claude() {
        assert_eq!(AiTool::from_str("unknown"), AiTool::Claude);
        assert_eq!(AiTool::from_str(""), AiTool::Claude);
        assert_eq!(AiTool::from_str("opencode"), AiTool::Claude);
        assert_eq!(AiTool::from_str("codex"), AiTool::Claude);
    }

    #[test]
    fn ai_tool_serde_roundtrip() {
        let tool = AiTool::Custom;
        let json = serde_json::to_string(&tool).unwrap();
        assert_eq!(json, "\"custom\"");
        let back: AiTool = serde_json::from_str(&json).unwrap();
        assert_eq!(back, AiTool::Custom);
    }

    #[test]
    fn ai_tool_serde_all_variants() {
        for variant in &[AiTool::Claude, AiTool::Custom] {
            let json = serde_json::to_string(variant).unwrap();
            let back: AiTool = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, variant);
        }
    }
}
