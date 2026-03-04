use crate::db::DbConn;
use crate::models::run::{Run, RunStatus, RunWithTaskName, TriggerSource};
use chrono::Utc;
use tauri::State;

fn row_to_run(row: &rusqlite::Row) -> rusqlite::Result<Run> {
    let status_str: String = row.get(2)?;
    let started_at_str: String = row.get(5)?;
    let ended_at_str: Option<String> = row.get(6)?;
    let triggered_str: String = row.get(8)?;

    Ok(Run {
        id: row.get(0)?,
        task_id: row.get(1)?,
        status: RunStatus::from_str(&status_str),
        exit_code: row.get(3)?,
        stdout: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        stderr: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
        started_at: started_at_str.parse().unwrap_or_else(|_| Utc::now()),
        ended_at: ended_at_str.and_then(|s| s.parse().ok()),
        duration_ms: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
        triggered_by: TriggerSource::from_str(&triggered_str),
    })
}

#[tauri::command]
pub fn get_runs(
    task_id: String,
    limit: Option<i64>,
    db: State<'_, DbConn>,
) -> Result<Vec<Run>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(50);

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, status, exit_code, stdout, started_at, ended_at, duration_ms,
             triggered_by, stderr
             FROM runs WHERE task_id = ?1 ORDER BY started_at DESC LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;

    let runs: rusqlite::Result<Vec<Run>> = stmt
        .query_map(rusqlite::params![task_id, limit], row_to_run)
        .map_err(|e| e.to_string())?
        .collect();

    runs.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_runs(
    limit: Option<i64>,
    offset: Option<i64>,
    status_filter: Option<String>,
    search_query: Option<String>,
    db: State<'_, DbConn>,
) -> Result<Vec<RunWithTaskName>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);

    // FTS search if query provided
    if let Some(ref q) = search_query {
        if !q.is_empty() {
            let mut stmt = conn
                .prepare(
                    "SELECT r.id, r.task_id, r.status, r.exit_code, r.stdout, r.started_at,
                     r.ended_at, r.duration_ms, r.triggered_by, r.stderr, t.name
                     FROM runs_fts fts
                     JOIN runs r ON r.id = fts.run_id
                     JOIN tasks t ON t.id = r.task_id
                     WHERE runs_fts MATCH ?1
                     ORDER BY r.started_at DESC LIMIT ?2 OFFSET ?3",
                )
                .map_err(|e| e.to_string())?;

            let rows: rusqlite::Result<Vec<RunWithTaskName>> = stmt
                .query_map(rusqlite::params![q, limit, offset], |row| {
                    let run = row_to_run(row)?;
                    let task_name: String = row.get(10)?;
                    Ok(RunWithTaskName { run, task_name })
                })
                .map_err(|e| e.to_string())?
                .collect();

            return rows.map_err(|e| e.to_string());
        }
    }

    let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match &status_filter {
        Some(s) if !s.is_empty() => (
            "SELECT r.id, r.task_id, r.status, r.exit_code, r.stdout, r.started_at,
             r.ended_at, r.duration_ms, r.triggered_by, r.stderr, t.name
             FROM runs r JOIN tasks t ON t.id = r.task_id
             WHERE r.status = ?1 ORDER BY r.started_at DESC LIMIT ?2 OFFSET ?3"
                .to_string(),
            vec![Box::new(s.clone()), Box::new(limit), Box::new(offset)],
        ),
        _ => (
            "SELECT r.id, r.task_id, r.status, r.exit_code, r.stdout, r.started_at,
             r.ended_at, r.duration_ms, r.triggered_by, r.stderr, t.name
             FROM runs r JOIN tasks t ON t.id = r.task_id
             ORDER BY r.started_at DESC LIMIT ?1 OFFSET ?2"
                .to_string(),
            vec![Box::new(limit), Box::new(offset)],
        ),
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: rusqlite::Result<Vec<RunWithTaskName>> = stmt
        .query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| {
                let run = row_to_run(row)?;
                let task_name: String = row.get(10)?;
                Ok(RunWithTaskName { run, task_name })
            },
        )
        .map_err(|e| e.to_string())?
        .collect();

    rows.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_run(id: String, db: State<'_, DbConn>) -> Result<Run, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, task_id, status, exit_code, stdout, started_at, ended_at, duration_ms,
         triggered_by, stderr FROM runs WHERE id = ?1",
        [&id],
        row_to_run,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_runs_for_task(task_id: String, db: State<'_, DbConn>) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM runs WHERE task_id = ?1", [&task_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn cleanup_old_runs(db: State<'_, DbConn>) -> Result<u64, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // Retention by days
    let days: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'log_retention_days'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(30);

    let deleted = conn
        .execute(
            "DELETE FROM runs WHERE started_at < datetime('now', ?1)",
            [format!("-{} days", days)],
        )
        .map_err(|e| e.to_string())?;

    // Retention per task (keep latest N runs per task)
    let per_task: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'log_retention_per_task'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(100);

    conn.execute_batch(&format!(
        "DELETE FROM runs WHERE id NOT IN (
            SELECT id FROM runs r2
            WHERE r2.task_id = runs.task_id
            ORDER BY started_at DESC LIMIT {}
        )",
        per_task
    ))
    .map_err(|e| e.to_string())?;

    Ok(deleted as u64)
}
