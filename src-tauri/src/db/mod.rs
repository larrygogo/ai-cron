use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

pub struct DbConn(pub Mutex<Connection>);

pub fn init_db(app_data_dir: &str) -> Result<DbConn> {
    let db_path = format!("{}/ai-cron.db", app_data_dir);
    log::info!("Opening database at: {}", db_path);

    let conn = Connection::open(&db_path)?;

    // Enable WAL mode and foreign keys
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    // Run migrations
    let migration_sql = include_str!("migrations.sql");
    conn.execute_batch(migration_sql)?;

    log::info!("Database initialized successfully");
    Ok(DbConn(Mutex::new(conn)))
}

#[cfg(test)]
pub fn init_db_memory() -> DbConn {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let migration_sql = include_str!("migrations.sql");
    conn.execute_batch(migration_sql).unwrap();
    DbConn(Mutex::new(conn))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_db_memory_creates_tables() {
        let db = init_db_memory();
        let conn = db.0.lock().unwrap();

        // Verify tasks table exists
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        // Verify runs table exists
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM runs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        // Verify settings table has default values
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))
            .unwrap();
        assert!(count > 0, "settings table should have default rows");
    }

    #[test]
    fn init_db_memory_fts5_available() {
        let db = init_db_memory();
        let conn = db.0.lock().unwrap();

        // runs_fts virtual table should exist
        let result = conn.execute(
            "INSERT INTO runs_fts(run_id, task_id, task_name, stdout, stderr) VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params!["r1", "t1", "test", "out", "err"],
        );
        assert!(result.is_ok(), "FTS5 table should be writable");
    }
}
