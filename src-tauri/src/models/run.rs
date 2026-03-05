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
    pub goal_evaluation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunWithTaskName {
    pub run: Run,
    pub task_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_status_from_str_as_str_roundtrip() {
        let cases = vec![
            ("queued", RunStatus::Queued),
            ("running", RunStatus::Running),
            ("success", RunStatus::Success),
            ("failed", RunStatus::Failed),
            ("killed", RunStatus::Killed),
        ];
        for (s, expected) in &cases {
            let status = RunStatus::from_str(s);
            assert_eq!(&status, expected);
            assert_eq!(status.as_str(), *s);
        }
    }

    #[test]
    fn run_status_unknown_defaults_to_queued() {
        assert_eq!(RunStatus::from_str("unknown"), RunStatus::Queued);
        assert_eq!(RunStatus::from_str(""), RunStatus::Queued);
    }

    #[test]
    fn trigger_source_from_str_as_str_roundtrip() {
        let cases = vec![
            ("scheduler", TriggerSource::Scheduler),
            ("manual", TriggerSource::Manual),
        ];
        for (s, expected) in &cases {
            let src = TriggerSource::from_str(s);
            assert_eq!(&src, expected);
            assert_eq!(src.as_str(), *s);
        }
    }

    #[test]
    fn trigger_source_unknown_defaults_to_scheduler() {
        assert_eq!(TriggerSource::from_str("unknown"), TriggerSource::Scheduler);
        assert_eq!(TriggerSource::from_str(""), TriggerSource::Scheduler);
    }
}
