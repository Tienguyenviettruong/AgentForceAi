use std::collections::VecDeque;
use std::panic;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct CrashReport {
    pub timestamp: SystemTime,
    pub message: String,
    pub backtrace: String,
}

#[derive(Clone)]
pub struct CrashHandler {
    reports: Arc<Mutex<VecDeque<CrashReport>>>,
}

const MAX_CRASH_REPORTS: usize = 32;

impl CrashHandler {
    pub fn new() -> Self {
        Self {
            reports: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn install_panic_hook(&self) {
        let reports = Arc::clone(&self.reports);

        panic::set_hook(Box::new(move |panic_info| {
            let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            let location = if let Some(loc) = panic_info.location() {
                format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
            } else {
                "Unknown location".to_string()
            };

            let full_message = format!("Panic occurred at {}: {}", location, message);

            // Note: In a real app we'd capture backtrace using backtrace crate or std::backtrace.
            let backtrace = "Backtrace captured (placeholder)".to_string();

            if let Ok(mut r) = reports.lock() {
                if r.len() == MAX_CRASH_REPORTS {
                    r.pop_front();
                }
                r.push_back(CrashReport {
                    timestamp: SystemTime::now(),
                    message: full_message,
                    backtrace,
                });
            }
        }));
    }

    pub async fn get_reports(&self) -> Vec<CrashReport> {
        if let Ok(reports) = self.reports.lock() {
            reports.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub async fn clear_reports(&self) {
        if let Ok(mut reports) = self.reports.lock() {
            reports.clear();
        }
    }

    pub async fn simulate_crash(&self) {
        panic!("Simulated crash for testing crash handler");
    }
}

impl Default for CrashHandler {
    fn default() -> Self {
        Self::new()
    }
}
