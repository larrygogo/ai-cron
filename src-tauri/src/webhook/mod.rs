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

    /// Determine whether a webhook should be sent for the given status and config.
    pub fn should_send(config: &WebhookConfig, status: &RunStatus) -> bool {
        match status {
            RunStatus::Running => config.on_start,
            RunStatus::Success => config.on_success,
            RunStatus::Failed => config.on_failure,
            RunStatus::Killed => config.on_killed,
            RunStatus::Queued => false,
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
        let should_send = Self::should_send(config, status);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::task::WebhookConfig;

    fn make_webhook_config(on_start: bool, on_success: bool, on_failure: bool, on_killed: bool) -> WebhookConfig {
        WebhookConfig {
            url: "https://example.com/hook".to_string(),
            platform: "generic".to_string(),
            on_start,
            on_success,
            on_failure,
            on_killed,
        }
    }

    #[test]
    fn should_send_running_checks_on_start() {
        let cfg = make_webhook_config(true, false, false, false);
        assert!(WebhookSender::should_send(&cfg, &RunStatus::Running));

        let cfg = make_webhook_config(false, true, true, true);
        assert!(!WebhookSender::should_send(&cfg, &RunStatus::Running));
    }

    #[test]
    fn should_send_success_checks_on_success() {
        let cfg = make_webhook_config(false, true, false, false);
        assert!(WebhookSender::should_send(&cfg, &RunStatus::Success));

        let cfg = make_webhook_config(true, false, true, true);
        assert!(!WebhookSender::should_send(&cfg, &RunStatus::Success));
    }

    #[test]
    fn should_send_failed_checks_on_failure() {
        let cfg = make_webhook_config(false, false, true, false);
        assert!(WebhookSender::should_send(&cfg, &RunStatus::Failed));

        let cfg = make_webhook_config(true, true, false, true);
        assert!(!WebhookSender::should_send(&cfg, &RunStatus::Failed));
    }

    #[test]
    fn should_send_killed_checks_on_killed() {
        let cfg = make_webhook_config(false, false, false, true);
        assert!(WebhookSender::should_send(&cfg, &RunStatus::Killed));

        let cfg = make_webhook_config(true, true, true, false);
        assert!(!WebhookSender::should_send(&cfg, &RunStatus::Killed));
    }

    #[test]
    fn should_send_queued_always_false() {
        let cfg = make_webhook_config(true, true, true, true);
        assert!(!WebhookSender::should_send(&cfg, &RunStatus::Queued));
    }

    #[test]
    fn output_truncation_short_output() {
        let output = "short output";
        assert!(output.len() <= 28000);
        // No truncation needed
        let result = if output.len() > 28000 {
            format!("{}...\n[Output truncated, {} chars total]", &output[..28000], output.len())
        } else {
            output.to_string()
        };
        assert_eq!(result, "short output");
    }

    #[test]
    fn output_truncation_long_output() {
        let output = "a".repeat(30000);
        let result = if output.len() > 28000 {
            format!("{}...\n[Output truncated, {} chars total]", &output[..28000], output.len())
        } else {
            output.clone()
        };
        assert!(result.contains("[Output truncated, 30000 chars total]"));
        assert!(result.starts_with(&"a".repeat(28000)));
    }
}
