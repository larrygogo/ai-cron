use crate::commands::runner::execute_task;
use crate::commands::tasks::row_to_task_pub;
use crate::db::DbConn;
use crate::models::run::TriggerSource;
use std::sync::Arc;
use tauri::AppHandle;
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct SchedulerState {
    pub scheduler: JobScheduler,
}

impl SchedulerState {
    pub async fn new() -> anyhow::Result<Self> {
        let scheduler = JobScheduler::new().await?;
        Ok(Self { scheduler })
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        self.scheduler.start().await?;
        Ok(())
    }

    /// Load all enabled tasks from DB and schedule them
    pub async fn load_tasks(&self, db: Arc<DbConn>, app_handle: AppHandle) {
        let tasks = {
            let conn = db.0.lock().unwrap();
            let mut stmt = match conn.prepare(
                "SELECT id, name, cron_expression, cron_human, ai_tool, custom_command, prompt,
                 working_directory, enabled, inject_context, restrict_network, restrict_filesystem,
                 env_vars, webhook_config, created_at, updated_at, last_run_at, last_run_status
                 FROM tasks WHERE enabled = 1",
            ) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to prepare task query: {}", e);
                    return;
                }
            };

            match stmt
                .query_map([], row_to_task_pub)
                .map(|iter| iter.filter_map(|r| r.ok()).collect::<Vec<_>>())
            {
                Ok(tasks) => tasks,
                Err(e) => {
                    log::error!("Failed to load tasks: {}", e);
                    vec![]
                }
            }
        };

        for task in tasks {
            log::info!("Scheduling task: {} [{}]", task.name, task.cron_expression);
            self.schedule_task(task, db.clone(), app_handle.clone())
                .await;
        }
    }

    /// Schedule a single task (idempotent — caller is responsible for dedup)
    pub async fn schedule_task(
        &self,
        task: crate::models::task::Task,
        db: Arc<DbConn>,
        app_handle: AppHandle,
    ) {
        let cron_expr = task.cron_expression.clone();
        let task_name = task.name.clone();

        let job_result = Job::new_async(cron_expr.as_str(), move |_uuid, _lock| {
            let task = task.clone();
            let db = db.clone();
            let app_handle = app_handle.clone();
            Box::pin(async move {
                log::info!("Executing scheduled task: {}", task.name);
                execute_task(task, TriggerSource::Scheduler, app_handle, db).await;
            })
        });

        match job_result {
            Ok(job) => {
                if let Err(e) = self.scheduler.add(job).await {
                    log::error!("Failed to add job for task '{}': {}", task_name, e);
                }
            }
            Err(e) => {
                log::error!(
                    "Invalid cron expression '{}' for task '{}': {}",
                    cron_expr,
                    task_name,
                    e
                );
            }
        }
    }
}
