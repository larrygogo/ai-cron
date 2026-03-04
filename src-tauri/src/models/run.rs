use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    Success,
    Failed,
    Killed,
}

impl RunStatus {
    pub fn as_str(&self) -> &str {
        match self {
            RunStatus::Queued => "queued",
            RunStatus::Running => "running",
            RunStatus::Success => "success",
            RunStatus::Failed => "failed",
            RunStatus::Killed => "killed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => RunStatus::Running,
            "success" => RunStatus::Success,
            "failed" => RunStatus::Failed,
            "killed" => RunStatus::Killed,
            _ => RunStatus::Queued,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    Scheduler,
    Manual,
}

impl TriggerSource {
    pub fn as_str(&self) -> &str {
        match self {
            TriggerSource::Scheduler => "scheduler",
            TriggerSource::Manual => "manual",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "manual" => TriggerSource::Manual,
            _ => TriggerSource::Scheduler,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub task_id: String,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub triggered_by: TriggerSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunWithTaskName {
    pub run: Run,
    pub task_name: String,
}
