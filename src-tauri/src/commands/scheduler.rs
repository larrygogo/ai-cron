use crate::db::DbConn;
use crate::scheduler::engine::SchedulerState;
use tauri::State;

#[tauri::command]
pub fn set_task_enabled_and_reschedule(
    _id: String,
    _enabled: bool,
    _db: State<'_, DbConn>,
    _scheduler: State<'_, SchedulerState>,
) -> Result<(), String> {
    // Full implementation: update DB then add/remove from scheduler
    // For Phase 1, DB update is handled by tasks::set_task_enabled
    // Scheduler hot-reload will be wired in Phase 2
    Ok(())
}

#[tauri::command]
pub async fn preview_next_runs(
    cron_expr: String,
    count: Option<usize>,
) -> Result<Vec<String>, String> {
    use chrono::Utc;
    use tokio_cron_scheduler::Job;

    let count = count.unwrap_or(5).min(20);
    let now = Utc::now();

    // Validate cron expression by attempting to create a job
    let valid = Job::new_async(cron_expr.as_str(), |_, _| Box::pin(async {})).is_ok();
    if !valid {
        return Err(format!("Invalid cron expression: {}", cron_expr));
    }

    // Simple next-N calculation using chrono + cron crate
    // tokio-cron-scheduler uses the cron crate internally
    // We'll use a simple approach: advance time and check
    let mut results = Vec::new();
    let mut check_time = now;

    for _ in 0..count {
        check_time = check_time + chrono::Duration::minutes(1);
        // Walk minute by minute to find next match (up to 1 year)
        let mut found = false;
        for _step in 0..(365 * 24 * 60) {
            if cron_matches(&cron_expr, &check_time) {
                results.push(check_time.to_rfc3339());
                found = true;
                check_time = check_time + chrono::Duration::minutes(1);
                break;
            }
            check_time = check_time + chrono::Duration::minutes(1);
        }
        if !found {
            break;
        }
    }

    Ok(results)
}

/// Simple 5-field cron matching (min hour dom month dow)
fn cron_matches(expr: &str, dt: &chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::Datelike;
    use chrono::Timelike;

    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() < 5 {
        return false;
    }

    let minute = dt.minute();
    let hour = dt.hour();
    let dom = dt.day();
    let month = dt.month();
    let dow = dt.weekday().num_days_from_sunday(); // 0=Sun

    field_matches(fields[0], minute)
        && field_matches(fields[1], hour)
        && field_matches(fields[2], dom)
        && field_matches(fields[3], month)
        && field_matches(fields[4], dow)
}

fn field_matches(field: &str, value: u32) -> bool {
    if field == "*" {
        return true;
    }
    // Handle ranges (e.g., 1-5)
    if field.contains('-') {
        let parts: Vec<&str> = field.split('-').collect();
        if parts.len() == 2 {
            if let (Ok(lo), Ok(hi)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                return value >= lo && value <= hi;
            }
        }
    }
    // Handle step (e.g., */5)
    if field.contains('/') {
        let parts: Vec<&str> = field.split('/').collect();
        if parts.len() == 2 {
            if let Ok(step) = parts[1].parse::<u32>() {
                let base: u32 = if parts[0] == "*" {
                    0
                } else {
                    parts[0].parse().unwrap_or(0)
                };
                return step > 0 && value >= base && (value - base) % step == 0;
            }
        }
    }
    // Handle lists (e.g., 1,3,5)
    if field.contains(',') {
        return field.split(',').any(|part| {
            part.trim().parse::<u32>().map(|v| v == value).unwrap_or(false)
        });
    }
    // Exact value
    field.parse::<u32>().map(|v| v == value).unwrap_or(false)
}
