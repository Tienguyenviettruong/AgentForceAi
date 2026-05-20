use std::collections::HashMap;
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
    records: Arc<Mutex<HashMap<String, Vec<ProfilerRecord>>>>,
}

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
            records
                .entry(record.name.clone())
                .or_insert_with(Vec::new)
                .push(record);
        }
    }

    pub async fn get_average_duration(&self, name: &str) -> Option<Duration> {
        let records = self.records.lock().ok()?;
        let traces = records.get(name)?;

        let valid_traces: Vec<_> = traces.iter().filter_map(|r| r.duration).collect();
        if valid_traces.is_empty() {
            return None;
        }

        let total: Duration = valid_traces.iter().sum();
        Some(total / valid_traces.len() as u32)
    }

    pub async fn clear(&self) {
        if let Ok(mut records) = self.records.lock() {
            records.clear();
        }
    }
}
