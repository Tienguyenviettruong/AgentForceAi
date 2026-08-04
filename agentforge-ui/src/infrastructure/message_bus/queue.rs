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
}

impl MessageQueue {
    pub fn new(config: QueueConfig) -> Self {
        let (sender, receiver) = mpsc::channel(config.max_size);
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            config,
        }
    }

    pub async fn enqueue(&self, message: TeamMessage) -> Result<(), String> {
        match self.sender.try_send(message) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                Err("Queue is full. Backpressure applied.".to_string())
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err("Failed to enqueue message: queue is closed".to_string())
            }
        }
    }

    pub async fn dequeue(&self) -> Option<TeamMessage> {
        let mut rx = self.receiver.lock().await;
        rx.recv().await
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

#[cfg(test)]
mod tests {
    use super::*;

    fn message(content: &str) -> TeamMessage {
        TeamMessage::new_broadcast(
            "instance".to_string(),
            "sender".to_string(),
            content.to_string(),
        )
    }

    #[tokio::test]
    async fn bounded_queue_applies_backpressure_without_a_counter_lock() {
        let queue = MessageQueue::new(QueueConfig {
            max_size: 1,
            ..QueueConfig::default()
        });

        queue.enqueue(message("first")).await.unwrap();
        assert!(queue.enqueue(message("second")).await.is_err());
        assert_eq!(queue.dequeue().await.unwrap().content, "first");
        queue.enqueue(message("third")).await.unwrap();
    }
}
