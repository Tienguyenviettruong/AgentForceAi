use std::future::Future;
use std::sync::Mutex;
use std::time::Instant;

pub struct StartupOptimizer {
    start_time: Instant,
    milestones: Mutex<Vec<(String, std::time::Duration)>>,
}

impl StartupOptimizer {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            milestones: Mutex::new(Vec::new()),
        }
    }

    pub async fn record_milestone(&self, name: &str) {
        let elapsed = self.start_time.elapsed();
        if let Ok(mut milestones) = self.milestones.lock() {
            milestones.push((name.to_string(), elapsed));
        }
    }

    pub async fn get_milestones(&self) -> Vec<(String, std::time::Duration)> {
        if let Ok(milestones) = self.milestones.lock() {
            milestones.clone()
        } else {
            Vec::new()
        }
    }

    pub async fn execute_task<F, Fut, T>(&self, name: &str, task: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        let result = task().await;
        self.record_milestone(name).await;
        result
    }

    pub async fn lazy_load<F, Fut, T>(&self, init: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        init().await
    }
}

impl Default for StartupOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
