use std::sync::atomic::{AtomicUsize, Ordering};

pub struct MemoryMonitor {
    allocated_bytes: AtomicUsize,
    freed_bytes: AtomicUsize,
    peak_bytes: AtomicUsize,
}

impl MemoryMonitor {
    pub fn new() -> Self {
        Self {
            allocated_bytes: AtomicUsize::new(0),
            freed_bytes: AtomicUsize::new(0),
            peak_bytes: AtomicUsize::new(0),
        }
    }

    pub async fn record_allocation(&self, size: usize) {
        let current = self.allocated_bytes.fetch_add(size, Ordering::SeqCst) + size;

        let mut peak = self.peak_bytes.load(Ordering::SeqCst);
        while current > peak {
            match self.peak_bytes.compare_exchange_weak(
                peak,
                current,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }
    }

    pub async fn record_deallocation(&self, size: usize) {
        self.freed_bytes.fetch_add(size, Ordering::SeqCst);
    }

    pub async fn current_usage(&self) -> usize {
        let allocated = self.allocated_bytes.load(Ordering::SeqCst);
        let freed = self.freed_bytes.load(Ordering::SeqCst);

        allocated.saturating_sub(freed)
    }

    pub async fn peak_usage(&self) -> usize {
        self.peak_bytes.load(Ordering::SeqCst)
    }

    pub async fn reset(&self) {
        self.allocated_bytes.store(0, Ordering::SeqCst);
        self.freed_bytes.store(0, Ordering::SeqCst);
        self.peak_bytes.store(0, Ordering::SeqCst);
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}
