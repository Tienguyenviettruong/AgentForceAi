use std::future::Future;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u32,
    pub total_time: Duration,
    pub avg_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
}

pub struct BenchmarkRunner;

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self
    }

    pub async fn run_async<F, Fut>(
        &self,
        name: &str,
        iterations: u32,
        mut task: F,
    ) -> BenchmarkResult
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut total_time = Duration::ZERO;
        let mut min_time = Duration::MAX;
        let mut max_time = Duration::ZERO;

        for _ in 0..iterations {
            let start = Instant::now();
            task().await;
            let elapsed = start.elapsed();

            total_time += elapsed;
            if elapsed < min_time {
                min_time = elapsed;
            }
            if elapsed > max_time {
                max_time = elapsed;
            }
        }

        let avg_time = if iterations > 0 {
            total_time / iterations
        } else {
            Duration::ZERO
        };

        BenchmarkResult {
            name: name.to_string(),
            iterations,
            total_time,
            avg_time,
            min_time,
            max_time,
        }
    }

    pub async fn run_sync<F>(&self, name: &str, iterations: u32, mut task: F) -> BenchmarkResult
    where
        F: FnMut(),
    {
        let mut total_time = Duration::ZERO;
        let mut min_time = Duration::MAX;
        let mut max_time = Duration::ZERO;

        for _ in 0..iterations {
            let start = Instant::now();
            task();
            let elapsed = start.elapsed();

            total_time += elapsed;
            if elapsed < min_time {
                min_time = elapsed;
            }
            if elapsed > max_time {
                max_time = elapsed;
            }
        }

        let avg_time = if iterations > 0 {
            total_time / iterations
        } else {
            Duration::ZERO
        };

        BenchmarkResult {
            name: name.to_string(),
            iterations,
            total_time,
            avg_time,
            min_time,
            max_time,
        }
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}
