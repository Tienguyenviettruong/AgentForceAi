use anyhow::Result;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct InterTeamMessage {
    pub message_id: String,
    pub source_team: String,
    pub target_team: String,
    pub payload: String,
    pub priority: u8,
}

pub struct MessageRouter {
    queues: Arc<Mutex<std::collections::HashMap<String, VecDeque<InterTeamMessage>>>>,
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            queues: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn route_message(&self, message: InterTeamMessage) -> Result<()> {
        let mut queues = self.queues.lock().unwrap();
        let queue = queues.entry(message.target_team.clone()).or_default();

        // Simple priority queueing: insert high priority at front, low at back
        if message.priority > 5 {
            queue.push_front(message);
        } else {
            queue.push_back(message);
        }
        Ok(())
    }

    pub fn receive_messages(&self, team_id: &str) -> Result<Vec<InterTeamMessage>> {
        let mut queues = self.queues.lock().unwrap();
        if let Some(queue) = queues.get_mut(team_id) {
            let messages: Vec<_> = queue.drain(..).collect();
            Ok(messages)
        } else {
            Ok(Vec::new())
        }
    }
}
