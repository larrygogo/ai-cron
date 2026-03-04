use crate::models::run::RunStatus;
use crate::models::task::{Task, WebhookConfig};
use reqwest::Client;
use serde_json::json;

pub struct WebhookSender {
    client: Client,
}

impl WebhookSender {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn send(
        &self,
        config: &WebhookConfig,
        task: &Task,
        status: &RunStatus,
        duration_ms: Option<u64>,
        stdout: &str,
        stderr: &str,
    ) {
        let should_send = match status {
            RunStatus::Running => config.on_start,
            RunStatus::Success => config.on_success,
            RunStatus::Failed => config.on_failure,
            RunStatus::Killed => config.on_killed,
            RunStatus::Queued => false,
        };

        if !should_send {
            return;
        }

        let output = format!("{}{}", stdout, stderr);
        // Truncate to avoid massive payloads (Feishu limit ~30KB)
        let output_truncated = if output.len() > 28000 {
            format!(
                "{}...\n[Output truncated, {} chars total]",
                &output[..28000],
                output.len()
            )
        } else {
            output
        };

        let result = match config.platform.as_str() {
            "feishu" => {
                self.send_feishu(config, task, status, duration_ms, &output_truncated)
                    .await
            }
            _ => {
                self.send_generic(config, task, status, duration_ms, &output_truncated)
                    .await
            }
        };

        if let Err(e) = result {
            log::error!("Webhook send failed for task {}: {}", task.name, e);
        }
    }

    async fn send_feishu(
        &self,
        config: &WebhookConfig,
        task: &Task,
        status: &RunStatus,
        duration_ms: Option<u64>,
        output: &str,
    ) -> anyhow::Result<()> {
        let (emoji, color, status_text) = match status {
            RunStatus::Running => ("🚀", "blue", "开始执行"),
            RunStatus::Success => ("✅", "green", "执行成功"),
            RunStatus::Failed => ("❌", "red", "执行失败"),
            RunStatus::Killed => ("⛔", "orange", "已手动终止"),
            RunStatus::Queued => ("⏳", "grey", "排队中"),
        };

        let duration_str = duration_ms
            .map(|ms| {
                let secs = ms / 1000;
                if secs >= 60 {
                    format!("{}m{}s", secs / 60, secs % 60)
                } else {
                    format!("{}s", secs)
                }
            })
            .unwrap_or_else(|| "-".to_string());

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

        let mut elements = vec![
            json!({
                "tag": "div",
                "text": {
                    "tag": "lark_md",
                    "content": format!(
                        "**任务**: {}\n**状态**: {}\n**耗时**: {}\n**工具**: {}\n**触发**: 定时\n**时间**: {}",
                        task.name, status_text, duration_str,
                        format!("{:?}", task.ai_tool).to_lowercase(), now
                    )
                }
            }),
            json!({ "tag": "hr" }),
        ];

        if !output.is_empty() {
            elements.push(json!({
                "tag": "div",
                "text": {
                    "tag": "lark_md",
                    "content": format!("**输出**:\n```\n{}\n```", output)
                }
            }));
        }

        let payload = json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": format!("{} {}", emoji, task.name)
                    },
                    "template": color
                },
                "elements": elements
            }
        });

        self.client
            .post(&config.url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn send_generic(
        &self,
        config: &WebhookConfig,
        task: &Task,
        status: &RunStatus,
        duration_ms: Option<u64>,
        output: &str,
    ) -> anyhow::Result<()> {
        let payload = json!({
            "task_id": task.id,
            "task_name": task.name,
            "status": status.as_str(),
            "ai_tool": task.ai_tool.as_str(),
            "duration_ms": duration_ms,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "output": output,
        });

        self.client
            .post(&config.url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
