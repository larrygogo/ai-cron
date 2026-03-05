use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

pub struct DbConn(pub Mutex<Connection>);

pub fn init_db(app_data_dir: &str) -> Result<DbConn> {
    let db_path = format!("{}/ai-cron.db", app_data_dir);
    log::info!("Opening database at: {}", db_path);

    let conn = Connection::open(&db_path)?;

    // Enable WAL mode and foreign keys
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;

    // Run migrations
    let migration_sql = include_str!("migrations.sql");
    conn.execute_batch(migration_sql)?;

    // v2: incremental migrations (ignore "duplicate column" errors)
    let alter_stmts = [
        "ALTER TABLE tasks ADD COLUMN execution_plan TEXT DEFAULT ''",
        "ALTER TABLE tasks ADD COLUMN consecutive_failures INTEGER DEFAULT 0",
        "ALTER TABLE runs ADD COLUMN goal_evaluation TEXT",
        "ALTER TABLE tasks ADD COLUMN allowed_tools TEXT DEFAULT '[]'",
        "ALTER TABLE tasks ADD COLUMN skip_permissions INTEGER DEFAULT 0",
    ];
    for stmt in &alter_stmts {
        match conn.execute(stmt, []) {
            Ok(_) => log::info!("Migration applied: {}", stmt),
            Err(e) if e.to_string().contains("duplicate column") => {}
            Err(e) => log::warn!("Migration skipped ({}): {}", e, stmt),
        }
    }

    // v3: Fix FTS5 triggers for contentless table (delete+insert instead of UPDATE/DELETE)
    conn.execute_batch("
        DROP TRIGGER IF EXISTS runs_au;
        CREATE TRIGGER IF NOT EXISTS runs_au AFTER UPDATE ON runs BEGIN
            INSERT INTO runs_fts(runs_fts, run_id, task_id, task_name, stdout, stderr)
            SELECT 'delete', OLD.id, OLD.task_id, COALESCE(t.name, ''), OLD.stdout, OLD.stderr
            FROM tasks t WHERE t.id = OLD.task_id;
            INSERT INTO runs_fts(run_id, task_id, task_name, stdout, stderr)
            SELECT NEW.id, NEW.task_id, COALESCE(t.name, ''), NEW.stdout, NEW.stderr
            FROM tasks t WHERE t.id = NEW.task_id;
        END;

        DROP TRIGGER IF EXISTS runs_ad;
        CREATE TRIGGER IF NOT EXISTS runs_ad AFTER DELETE ON runs BEGIN
            INSERT INTO runs_fts(runs_fts, run_id, task_id, task_name, stdout, stderr)
            SELECT 'delete', OLD.id, OLD.task_id, COALESCE(t.name, ''), OLD.stdout, OLD.stderr
            FROM tasks t WHERE t.id = OLD.task_id;
        END;

        -- Rebuild FTS index for existing data
        DELETE FROM runs_fts;
        INSERT INTO runs_fts(run_id, task_id, task_name, stdout, stderr)
        SELECT r.id, r.task_id, COALESCE(t.name, ''), r.stdout, r.stderr
        FROM runs r LEFT JOIN tasks t ON t.id = r.task_id;
    ").ok();

    // Checkpoint any leftover WAL from previous session
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok();

    log::info!("Database initialized successfully");
    Ok(DbConn(Mutex::new(conn)))
}

#[cfg(test)]
pub fn init_db_memory() -> DbConn {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let migration_sql = include_str!("migrations.sql");
    conn.execute_batch(migration_sql).unwrap();

    // Run the same incremental migrations as init_db
    let alter_stmts = [
        "ALTER TABLE tasks ADD COLUMN execution_plan TEXT DEFAULT ''",
        "ALTER TABLE tasks ADD COLUMN consecutive_failures INTEGER DEFAULT 0",
        "ALTER TABLE runs ADD COLUMN goal_evaluation TEXT",
        "ALTER TABLE tasks ADD COLUMN allowed_tools TEXT DEFAULT '[]'",
        "ALTER TABLE tasks ADD COLUMN skip_permissions INTEGER DEFAULT 0",
    ];
    for stmt in &alter_stmts {
        match conn.execute(stmt, []) {
            Ok(_) => {}
            Err(e) if e.to_string().contains("duplicate column") => {}
            Err(_) => {}
        }
    }

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
