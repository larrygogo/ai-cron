use crate::db::DbConn;
use crate::commands::tools::AppSettings;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDraft {
    pub name: String,
    pub cron_expression: String,
    pub cron_human: String,
    pub prompt: String,
    pub ai_tool: String,
    pub suggested_directory: String,
}

const SYSTEM_PROMPT: &str = r#"You are a task scheduling assistant. The user will describe a task they want to automate.
Your job is to extract:
1. A short name for the task (2-5 words)
2. A cron expression (standard 5-field: minute hour dom month dow)
3. A human-readable schedule description in Chinese (e.g., "每周工作日 09:00")
4. A clear, concise prompt to pass to an AI coding agent (Claude Code / OpenCode / Codex)
5. The best AI tool to use: "claude", "opencode", "codex", or "custom"
6. A suggested working directory (use "~/" as default if not mentioned)

Return ONLY valid JSON in this exact format, no markdown, no explanation:
{
  "name": "task name",
  "cron_expression": "0 9 * * 1-5",
  "cron_human": "每周工作日 09:00",
  "prompt": "detailed prompt for the AI agent",
  "ai_tool": "claude",
  "suggested_directory": "~/"
}

Rules:
- cron_expression must be valid 5-field cron (no seconds field)
- prompt should be in English, clear and actionable
- ai_tool defaults to "claude" unless user specifies otherwise
- Be specific with the cron expression based on the user's description"#;

#[tauri::command]
pub async fn parse_nl_to_task(
    input: String,
    db: State<'_, DbConn>,
) -> Result<TaskDraft, String> {
    // Load settings
    let settings = crate::commands::tools::get_settings(db)?;

    match settings.nl_provider.as_str() {
        "ollama" => parse_with_ollama(&input, &settings).await,
        "openai" => parse_with_openai(&input, &settings).await,
        _ => parse_with_claude(&input, &settings).await,
    }
}

async fn parse_with_claude(input: &str, settings: &AppSettings) -> Result<TaskDraft, String> {
    if settings.nl_api_key.is_empty() {
        return Err("Claude API key not configured. Please set it in Settings.".to_string());
    }

    let model = if settings.nl_model.is_empty() {
        "claude-3-5-haiku-20241022".to_string()
    } else {
        settings.nl_model.clone()
    };

    let client = Client::new();
    let payload = json!({
        "model": model,
        "max_tokens": 512,
        "system": SYSTEM_PROMPT,
        "messages": [{ "role": "user", "content": input }]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &settings.nl_api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let text = json["content"][0]["text"]
        .as_str()
        .ok_or("No text in response")?;

    serde_json::from_str::<TaskDraft>(text.trim())
        .map_err(|e| format!("Failed to parse AI response as JSON: {}\nRaw: {}", e, text))
}

async fn parse_with_openai(input: &str, settings: &AppSettings) -> Result<TaskDraft, String> {
    if settings.nl_api_key.is_empty() {
        return Err("OpenAI API key not configured. Please set it in Settings.".to_string());
    }

    let model = if settings.nl_model.is_empty() {
        "gpt-4o-mini".to_string()
    } else {
        settings.nl_model.clone()
    };

    let base_url = if settings.nl_base_url.is_empty() {
        "https://api.openai.com".to_string()
    } else {
        settings.nl_base_url.trim_end_matches('/').to_string()
    };

    let client = Client::new();
    let payload = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": input }
        ],
        "response_format": { "type": "json_object" }
    });

    let resp = client
        .post(format!("{}/v1/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", settings.nl_api_key))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let text = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?;

    serde_json::from_str::<TaskDraft>(text.trim())
        .map_err(|e| format!("Failed to parse AI response: {}\nRaw: {}", e, text))
}

async fn parse_with_ollama(input: &str, settings: &AppSettings) -> Result<TaskDraft, String> {
    let base_url = if settings.nl_base_url.is_empty() {
        "http://localhost:11434".to_string()
    } else {
        settings.nl_base_url.trim_end_matches('/').to_string()
    };

    let model = if settings.nl_model.is_empty() {
        "llama3.2".to_string()
    } else {
        settings.nl_model.clone()
    };

    let client = Client::new();
    let payload = json!({
        "model": model,
        "prompt": format!("{}\n\nUser request: {}", SYSTEM_PROMPT, input),
        "stream": false,
        "format": "json"
    });

    let resp = client
        .post(format!("{}/api/generate", base_url))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Request failed (is Ollama running?): {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let text = json["response"]
        .as_str()
        .ok_or("No response from Ollama")?;

    serde_json::from_str::<TaskDraft>(text.trim())
        .map_err(|e| format!("Failed to parse AI response: {}\nRaw: {}", e, text))
}
