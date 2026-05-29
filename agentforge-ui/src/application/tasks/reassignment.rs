use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub struct ReassignmentManager<'a> {
    conn: &'a Connection,
}

impl<'a> ReassignmentManager<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Task 2.19: Build task reassignment and escalation workflow
    pub fn reassign_task(&self, task_id: &str, new_agent_id: &str, _reason: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        // Log the reassignment reason somewhere if history table exists,
        // here we just update the task assignment
        self.conn.execute(
            "UPDATE tasks
             SET assignee_id = ?1, updated_at = ?2
             WHERE id = ?3",
            params![new_agent_id, now, task_id],
        )?;

        // Ideally, we log this reason to task_history.
        Ok(())
    }

    /// Escalate task priority (e.g. from medium to high)
    pub fn escalate_task(&self, task_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE tasks
             SET priority = CASE
                 WHEN priority = 'low' THEN 'medium'
                 WHEN priority = 'medium' THEN 'high'
                 ELSE 'high'
             END,
             updated_at = ?1
             WHERE id = ?2",
            params![now, task_id],
        )?;
        Ok(())
    }
}
