use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ProfilerRecord {
    pub name: String,
    pub start_time: Instant,
    pub duration: Option<Duration>,
}

#[derive(Clone)]
pub struct Profiler {
    records: Arc<Mutex<HashMap<String, VecDeque<ProfilerRecord>>>>,
}

const MAX_RECORDS_PER_TRACE: usize = 1_024;

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_trace(&self, name: &str) -> ProfilerRecord {
        ProfilerRecord {
            name: name.to_string(),
            start_time: Instant::now(),
            duration: None,
        }
    }

    pub async fn end_trace(&self, mut record: ProfilerRecord) {
        record.duration = Some(record.start_time.elapsed());
        if let Ok(mut records) = self.records.lock() {
            let traces = records.entry(record.name.clone()).or_default();
            if traces.len() == MAX_RECORDS_PER_TRACE {
                traces.pop_front();
            }
            traces.push_back(record);
        }
    }

    pub async fn get_average_duration(&self, name: &str) -> Option<Duration> {
        let records = self.records.lock().ok()?;
        let traces = records.get(name)?;

        let (total, count) = traces
            .iter()
            .filter_map(|record| record.duration)
            .fold((Duration::ZERO, 0_u32), |(total, count), duration| {
                (total + duration, count + 1)
            });
        if count == 0 {
            return None;
        }
        Some(total / count)
    }

    pub async fn clear(&self) {
        if let Ok(mut records) = self.records.lock() {
            records.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn trace_history_is_bounded() {
        let profiler = Profiler::new();
        for _ in 0..MAX_RECORDS_PER_TRACE + 10 {
            let trace = profiler.start_trace("render").await;
            profiler.end_trace(trace).await;
        }

        let records = profiler.records.lock().unwrap();
        assert_eq!(records["render"].len(), MAX_RECORDS_PER_TRACE);
    }
}
