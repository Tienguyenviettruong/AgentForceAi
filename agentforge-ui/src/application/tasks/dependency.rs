use rusqlite::{params, Connection, Result};

pub struct DependencyManager<'a> {
    conn: &'a Connection,
}

impl<'a> DependencyManager<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize the task_dependencies table
    pub fn init(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS task_dependencies (
                task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                depends_on_task_id TEXT NOT NULL,
                PRIMARY KEY (task_id, depends_on_task_id)
            )",
            [],
        )?;
        Ok(())
    }

    /// Add a dependency: task_id depends on depends_on_task_id
    pub fn add_dependency(&self, task_id: &str, depends_on_task_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO task_dependencies (task_id, depends_on_task_id) VALUES (?1, ?2)",
            params![task_id, depends_on_task_id],
        )?;
        Ok(())
    }

    /// Remove a dependency
    pub fn remove_dependency(&self, task_id: &str, depends_on_task_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM task_dependencies WHERE task_id = ?1 AND depends_on_task_id = ?2",
            params![task_id, depends_on_task_id],
        )?;
        Ok(())
    }

    /// Task 2.18: Implement task dependency tracking and blocking resolution
    /// Checks if all dependencies for a task are completed
    pub fn is_task_unblocked(&self, task_id: &str) -> Result<bool> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM task_dependencies td
             JOIN tasks t ON td.depends_on_task_id = t.id
             WHERE td.task_id = ?1 AND t.status != 'completed'",
        )?;

        let pending_deps: i64 = stmt.query_row(params![task_id], |row| row.get(0))?;
        Ok(pending_deps == 0)
    }

    /// Get all tasks that depend on the given task
    pub fn get_dependent_tasks(&self, task_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT task_id FROM task_dependencies WHERE depends_on_task_id = ?1")?;
        let iter = stmt.query_map(params![task_id], |row| row.get(0))?;

        let mut tasks = Vec::new();
        for t in iter {
            tasks.push(t?);
        }
        Ok(tasks)
    }
}
