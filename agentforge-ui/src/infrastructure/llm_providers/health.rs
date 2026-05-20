use super::BaseProviderAdapter;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Provider health check and auto-reconnection logic
/// (Task 1.15)
pub struct HealthMonitor {
    adapter: Arc<dyn BaseProviderAdapter>,
    reconnect_attempts: u32,
    max_retries: u32,
    base_delay_ms: u64,
}

impl HealthMonitor {
    pub fn new(adapter: Arc<dyn BaseProviderAdapter>) -> Self {
        Self {
            adapter,
            reconnect_attempts: 0,
            max_retries: 5,
            base_delay_ms: 1000,
        }
    }

    pub async fn monitor(&mut self) -> Result<()> {
        loop {
            match self.adapter.check_health().await {
                Ok(true) => {
                    self.reconnect_attempts = 0; // reset on success
                    sleep(Duration::from_secs(30)).await;
                }
                _ => {
                    self.reconnect_attempts += 1;
                    if self.reconnect_attempts > self.max_retries {
                        return Err(anyhow::anyhow!("Max reconnection attempts reached"));
                    }

                    // Exponential backoff
                    let delay = self.base_delay_ms * (2_u64.pow(self.reconnect_attempts - 1));
                    sleep(Duration::from_millis(delay)).await;

                    // Attempt reconnect logic...
                    // In a real system, we might re-initialize the adapter here.
                }
            }
        }
    }
}
