use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub struct TaskHistoryEntry {
    pub id: String,
    pub task_id: String,
    pub changed_by: Option<String>,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
    pub details: Option<String>,
    pub changed_at: String,
}

pub struct HistoryManager<'a> {
    conn: &'a Connection,
}

impl<'a> HistoryManager<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Task 2.20: Implement task history and audit trail
    pub fn init(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS task_history (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                changed_by TEXT,
                old_status TEXT,
                new_status TEXT,
                details TEXT,
                changed_at TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn record_history(&self, entry: &TaskHistoryEntry) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO task_history (id, task_id, changed_by, old_status, new_status, details, changed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id,
                entry.task_id,
                entry.changed_by,
                entry.old_status,
                entry.new_status,
                entry.details,
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_task_history(&self, task_id: &str) -> Result<Vec<TaskHistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, changed_by, old_status, new_status, details, changed_at
             FROM task_history
             WHERE task_id = ?1
             ORDER BY changed_at ASC",
        )?;

        let iter = stmt.query_map(params![task_id], |row| {
            Ok(TaskHistoryEntry {
                id: row.get(0)?,
                task_id: row.get(1)?,
                changed_by: row.get(2)?,
                old_status: row.get(3)?,
                new_status: row.get(4)?,
                details: row.get(5)?,
                changed_at: row.get(6)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in iter {
            entries.push(entry?);
        }
        Ok(entries)
    }
}
