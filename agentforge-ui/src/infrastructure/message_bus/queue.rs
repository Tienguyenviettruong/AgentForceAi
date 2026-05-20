use crate::teambus::routing::TeamMessage;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub max_size: usize,
    pub retry_limit: u32,
    pub retry_delay_ms: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_size: 1000,
            retry_limit: 3,
            retry_delay_ms: 100,
        }
    }
}

pub struct MessageQueue {
    sender: mpsc::Sender<TeamMessage>,
    receiver: Arc<Mutex<mpsc::Receiver<TeamMessage>>>,
    config: QueueConfig,
    current_size: Arc<Mutex<usize>>,
}

impl MessageQueue {
    pub fn new(config: QueueConfig) -> Self {
        let (sender, receiver) = mpsc::channel(config.max_size);
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            config,
            current_size: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn enqueue(&self, message: TeamMessage) -> Result<(), String> {
        let mut size = self.current_size.lock().await;
        if *size >= self.config.max_size {
            return Err("Queue is full. Backpressure applied.".to_string());
        }

        match self.sender.send(message).await {
            Ok(_) => {
                *size += 1;
                Ok(())
            }
            Err(e) => Err(format!("Failed to enqueue message: {}", e)),
        }
    }

    pub async fn dequeue(&self) -> Option<TeamMessage> {
        let mut rx = self.receiver.lock().await;
        if let Some(msg) = rx.recv().await {
            let mut size = self.current_size.lock().await;
            if *size > 0 {
                *size -= 1;
            }
            Some(msg)
        } else {
            None
        }
    }

    pub async fn process_with_retry<F, Fut>(&self, mut processor: F)
    where
        F: FnMut(TeamMessage) -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        while let Some(msg) = self.dequeue().await {
            let mut attempt = 0;
            let mut success = false;

            while attempt < self.config.retry_limit {
                match processor(msg.clone()).await {
                    Ok(_) => {
                        success = true;
                        break;
                    }
                    Err(e) => {
                        eprintln!("Failed to process message {}: {}. Retrying...", msg.id, e);
                        attempt += 1;
                        sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                    }
                }
            }

            if !success {
                eprintln!(
                    "Message {} failed after {} attempts.",
                    msg.id, self.config.retry_limit
                );
                // In a real system, we'd send this to a dead letter queue (DLQ)
            }
        }
    }
}
