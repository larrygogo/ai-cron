use crate::db::DbConn;
use crate::models::run::{Run, RunStatus, RunWithTaskName, TriggerSource};
use chrono::Utc;
use tauri::State;

pub fn row_to_run(row: &rusqlite::Row) -> rusqlite::Result<Run> {
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
        goal_evaluation: row.get(10)?,
    })
}

/// Core: query runs for a task
pub fn query_runs(db: &DbConn, task_id: &str, limit: i64) -> Result<Vec<Run>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, status, exit_code, stdout, started_at, ended_at, duration_ms,
             triggered_by, stderr, goal_evaluation
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
pub fn get_runs(
    task_id: String,
    limit: Option<i64>,
    db: State<'_, DbConn>,
) -> Result<Vec<Run>, String> {
    query_runs(&db, &task_id, limit.unwrap_or(50))
}

/// Core: query all runs with optional filters
pub fn query_all_runs(
    db: &DbConn,
    limit: i64,
    offset: i64,
    status_filter: Option<&str>,
    search_query: Option<&str>,
) -> Result<Vec<RunWithTaskName>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // FTS search if query provided
    if let Some(q) = search_query {
        if !q.is_empty() {
            let mut stmt = conn
                .prepare(
                    "SELECT r.id, r.task_id, r.status, r.exit_code, r.stdout, r.started_at,
                     r.ended_at, r.duration_ms, r.triggered_by, r.stderr, r.goal_evaluation, t.name
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
                    let task_name: String = row.get(11)?;
                    Ok(RunWithTaskName { run, task_name })
                })
                .map_err(|e| e.to_string())?
                .collect();

            return rows.map_err(|e| e.to_string());
        }
    }

    let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match status_filter {
        Some(s) if !s.is_empty() => (
            "SELECT r.id, r.task_id, r.status, r.exit_code, r.stdout, r.started_at,
             r.ended_at, r.duration_ms, r.triggered_by, r.stderr, r.goal_evaluation, t.name
             FROM runs r JOIN tasks t ON t.id = r.task_id
             WHERE r.status = ?1 ORDER BY r.started_at DESC LIMIT ?2 OFFSET ?3"
                .to_string(),
            vec![Box::new(s.to_string()), Box::new(limit), Box::new(offset)],
        ),
        _ => (
            "SELECT r.id, r.task_id, r.status, r.exit_code, r.stdout, r.started_at,
             r.ended_at, r.duration_ms, r.triggered_by, r.stderr, r.goal_evaluation, t.name
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
pub fn get_all_runs(
    limit: Option<i64>,
    offset: Option<i64>,
    status_filter: Option<String>,
    search_query: Option<String>,
    db: State<'_, DbConn>,
) -> Result<Vec<RunWithTaskName>, String> {
    query_all_runs(
        &db,
        limit.unwrap_or(50),
        offset.unwrap_or(0),
        status_filter.as_deref(),
        search_query.as_deref(),
    )
}

/// Core: query single run
pub fn query_run(db: &DbConn, id: &str) -> Result<Run, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, task_id, status, exit_code, stdout, started_at, ended_at, duration_ms,
         triggered_by, stderr, goal_evaluation FROM runs WHERE id = ?1",
        [id],
        row_to_run,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_run(id: String, db: State<'_, DbConn>) -> Result<Run, String> {
    query_run(&db, &id)
}

/// Core: delete runs for a task
pub fn delete_runs_core(db: &DbConn, task_id: &str) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM runs WHERE task_id = ?1", [task_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_runs_for_task(task_id: String, db: State<'_, DbConn>) -> Result<(), String> {
    delete_runs_core(&db, &task_id)
}

/// Core: cleanup old runs by retention policy
pub fn cleanup_runs_core(db: &DbConn) -> Result<u64, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // Retention by days
    let days: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'log_retention_days'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(30);

    let deleted_by_days = conn
        .execute(
            "DELETE FROM runs WHERE started_at < datetime('now', ?1)",
            [format!("-{} days", days)],
        )
        .map_err(|e| e.to_string())? as u64;

    // Retention per task (keep latest N runs per task)
    let per_task: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'log_retention_per_task'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(100);
    let per_task = per_task.clamp(1, 10000);

    let deleted_per_task = conn
        .execute(
            "DELETE FROM runs WHERE id IN (
                SELECT id FROM (
                    SELECT id, ROW_NUMBER() OVER (PARTITION BY task_id ORDER BY started_at DESC) AS rn
                    FROM runs
                ) WHERE rn > ?1
            )",
            rusqlite::params![per_task],
        )
        .map_err(|e| e.to_string())? as u64;

    Ok(deleted_by_days + deleted_per_task)
}

#[tauri::command]
pub fn cleanup_old_runs(db: State<'_, DbConn>) -> Result<u64, String> {
    cleanup_runs_core(&db)
}
