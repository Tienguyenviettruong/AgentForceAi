use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub use crate::core::models::Task;

pub struct SharedTaskList<'a> {
    conn: &'a Connection,
}

impl<'a> SharedTaskList<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create_task(&self, task: &Task) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tasks (id, team_id, instance_id, run_id, assignee_id, status, priority, payload, claimed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                task.id,
                task.team_id,
                task.instance_id,
                task.run_id,
                task.assignee_id,
                task.status,
                task.priority,
                task.payload,
                task.claimed_at,
                task.created_at,
                task.updated_at
            ],
        )?;
        Ok(())
    }

    /// Task 2.16: Atomic task claiming (WHERE status='pending')
    pub fn claim_task(&self, task_id: &str, agent_id: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows_affected = self.conn.execute(
            "UPDATE tasks 
             SET assignee_id = ?1, status = 'in_progress', claimed_at = ?2, updated_at = ?3 
             WHERE id = ?4 AND status = 'pending'",
            params![agent_id, now, now, task_id],
        )?;
        Ok(rows_affected > 0)
    }

    pub fn claim_task_for_instance(
        &self,
        task_id: &str,
        agent_id: &str,
        instance_id: &str,
    ) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows_affected = self.conn.execute(
            "UPDATE tasks
             SET assignee_id = ?1, status = 'in_progress', claimed_at = ?2, updated_at = ?3
             WHERE id = ?4 AND status = 'pending' AND instance_id = ?5",
            params![agent_id, now, now, task_id, instance_id],
        )?;
        Ok(rows_affected > 0)
    }

    pub fn mark_completed(&self, task_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE tasks SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
        )?;
        Ok(())
    }

    pub fn mark_failed(&self, task_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE tasks SET status = 'failed', updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
        )?;
        Ok(())
    }
}
