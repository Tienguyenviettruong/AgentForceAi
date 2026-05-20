use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct DatabaseOptimizer {
    connection: Arc<Mutex<Connection>>,
}

impl DatabaseOptimizer {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    pub async fn vacuum(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute("VACUUM", [])?;
        Ok(())
    }

    pub async fn analyze(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute("ANALYZE", [])?;
        Ok(())
    }

    pub async fn optimize_pragmas(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 30000000000;
            ",
        )?;
        Ok(())
    }

    pub async fn auto_optimize(&self) -> Result<std::time::Duration> {
        let start = Instant::now();
        self.optimize_pragmas().await?;
        self.vacuum().await?;
        self.analyze().await?;
        Ok(start.elapsed())
    }

    pub async fn create_index(&self, table: &str, column: &str) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        let index_name = format!("idx_{}_{}", table, column);
        let query = format!(
            "CREATE INDEX IF NOT EXISTS {} ON {} ({})",
            index_name, table, column
        );
        conn.execute(&query, [])?;
        Ok(())
    }
}
