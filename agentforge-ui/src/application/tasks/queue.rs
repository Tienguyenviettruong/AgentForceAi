use super::shared_task_list::Task;
use rusqlite::{params, Connection, Result};

pub struct TaskQueue<'a> {
    conn: &'a Connection,
}

impl<'a> TaskQueue<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Task 2.17: Build task priority queue and ordering system
    /// Returns available tasks ordered by priority (high > medium > low) and creation time.
    pub fn get_next_tasks(&self, team_id: &str, limit: u32) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, team_id, instance_id, run_id, assignee_id, status, priority, payload, claimed_at, created_at, updated_at
             FROM tasks
             WHERE team_id = ?1 AND status = 'pending'
             ORDER BY
                 CASE priority
                     WHEN 'high' THEN 1
                     WHEN 'medium' THEN 2
                     WHEN 'low' THEN 3
                     ELSE 4
                 END ASC,
                 created_at ASC
             LIMIT ?2"
        )?;

        let iter = stmt.query_map(params![team_id, limit], |row| {
            Ok(Task {
                id: row.get(0)?,
                team_id: row.get(1)?,
                instance_id: row.get(2)?,
                run_id: row.get(3)?,
                assignee_id: row.get(4)?,
                status: row.get(5)?,
                priority: row.get(6)?,
                payload: row.get(7)?,
                claimed_at: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        let mut tasks = Vec::new();
        for t in iter {
            tasks.push(t?);
        }
        Ok(tasks)
    }
}
