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
