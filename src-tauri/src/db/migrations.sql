-- AI Cron Database Schema
-- SQLite with FTS5 for log search

PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

-- Tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    cron_expression TEXT NOT NULL,
    cron_human TEXT NOT NULL DEFAULT '',
    ai_tool TEXT NOT NULL DEFAULT 'claude',
    custom_command TEXT,
    prompt TEXT NOT NULL DEFAULT '',
    working_directory TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,
    inject_context INTEGER NOT NULL DEFAULT 0,
    restrict_network INTEGER NOT NULL DEFAULT 0,
    restrict_filesystem INTEGER NOT NULL DEFAULT 0,
    env_vars TEXT NOT NULL DEFAULT '{}',
    -- Webhook config (stored as JSON)
    webhook_config TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_run_at TEXT,
    last_run_status TEXT
);

-- Runs table
CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'queued',
    exit_code INTEGER,
    stdout TEXT NOT NULL DEFAULT '',
    stderr TEXT NOT NULL DEFAULT '',
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration_ms INTEGER,
    triggered_by TEXT NOT NULL DEFAULT 'scheduler',
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- FTS5 virtual table for full-text search on run output
CREATE VIRTUAL TABLE IF NOT EXISTS runs_fts USING fts5(
    run_id UNINDEXED,
    task_id UNINDEXED,
    task_name,
    stdout,
    stderr,
    content=''
);

-- Trigger to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS runs_ai AFTER INSERT ON runs BEGIN
    INSERT INTO runs_fts(run_id, task_id, task_name, stdout, stderr)
    SELECT NEW.id, NEW.task_id, t.name, NEW.stdout, NEW.stderr
    FROM tasks t WHERE t.id = NEW.task_id;
END;

CREATE TRIGGER IF NOT EXISTS runs_au AFTER UPDATE ON runs BEGIN
    UPDATE runs_fts SET stdout = NEW.stdout, stderr = NEW.stderr
    WHERE run_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS runs_ad AFTER DELETE ON runs BEGIN
    DELETE FROM runs_fts WHERE run_id = OLD.id;
END;

-- Settings table (key-value)
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Default settings
INSERT OR IGNORE INTO settings(key, value) VALUES
    ('nl_provider', '"claude"'),
    ('nl_api_key', '""'),
    ('nl_base_url', '""'),
    ('nl_model', '""'),
    ('log_retention_days', '30'),
    ('log_retention_per_task', '100'),
    ('notify_on_success', 'false'),
    ('notify_on_failure', 'true');

-- Index for common queries
CREATE INDEX IF NOT EXISTS idx_runs_task_id ON runs(task_id);
CREATE INDEX IF NOT EXISTS idx_runs_started_at ON runs(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status);
