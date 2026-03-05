#[tauri::command]
pub async fn preview_next_runs(
    cron_expr: String,
    count: Option<usize>,
    timezone: Option<String>,
) -> Result<Vec<String>, String> {
    use chrono::Utc;
    use tokio_cron_scheduler::Job;

    let count = count.unwrap_or(5).min(20);
    let now = Utc::now();

    // Validate cron expression by attempting to create a job
    // tokio-cron-scheduler expects 6-7 fields (with seconds); prepend "0" for standard 5-field cron
    let scheduler_expr = if cron_expr.split_whitespace().count() == 5 {
        format!("0 {}", cron_expr)
    } else {
        cron_expr.clone()
    };
    let valid = Job::new_async(scheduler_expr.as_str(), |_, _| Box::pin(async {})).is_ok();
    if !valid {
        return Err(format!("Invalid cron expression: {}", cron_expr));
    }

    // Resolve timezone
    let tz = resolve_timezone(timezone.as_deref());

    let mut results = Vec::new();
    let mut check_time = now;

    for _ in 0..count {
        check_time += chrono::Duration::minutes(1);
        let mut found = false;
        for _step in 0..(365 * 24 * 60) {
            // Convert UTC time to the user's timezone for cron matching
            let local_dt = check_time.with_timezone(&tz).naive_local();
            if cron_matches(&cron_expr, &local_dt) {
                results.push(check_time.to_rfc3339());
                found = true;
                check_time += chrono::Duration::minutes(1);
                break;
            }
            check_time += chrono::Duration::minutes(1);
        }
        if !found {
            break;
        }
    }

    Ok(results)
}

/// Resolve timezone string to a chrono_tz::Tz
fn resolve_timezone(tz_str: Option<&str>) -> chrono_tz::Tz {
    let tz_name = match tz_str {
        Some(s) if !s.is_empty() && s != "system" => s.to_string(),
        _ => iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string()),
    };
    tz_name.parse::<chrono_tz::Tz>().unwrap_or(chrono_tz::UTC)
}

/// Simple 5-field cron matching (min hour dom month dow) using NaiveDateTime (local time)
fn cron_matches(expr: &str, dt: &chrono::NaiveDateTime) -> bool {
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
                return step > 0 && value >= base && (value - base).is_multiple_of(step);
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
