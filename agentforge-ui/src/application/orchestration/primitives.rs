use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Initializing,
    Running,
    Idle,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub sender: String,
    pub payload: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AgentOutput {
    pub agent_id: Uuid,
    pub data: String,
}

pub struct AgentContext {
    pub id: Uuid,
    pub status: AgentStatus,
    pub message_tx: mpsc::Sender<Message>,
    pub cancel_tx: oneshot::Sender<()>,
    pub status_tx: broadcast::Sender<AgentStatus>,
}

pub struct AgentManager {
    agents: Arc<RwLock<HashMap<Uuid, AgentContext>>>,
    outputs: Arc<RwLock<Vec<AgentOutput>>>,
    task_queue: Arc<Mutex<BinaryHeap<TaskItem>>>,
}

#[derive(Debug)]
pub struct TaskItem {
    pub id: Uuid,
    pub priority: TaskPriority,
    pub payload: String,
    pub created_at: DateTime<Utc>,
}

impl PartialEq for TaskItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for TaskItem {}
impl PartialOrd for TaskItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TaskItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Equal => other.created_at.cmp(&self.created_at),
            ord => ord,
        }
    }
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            outputs: Arc::new(RwLock::new(Vec::new())),
            task_queue: Arc::new(Mutex::new(BinaryHeap::new())),
        }
    }

    // 3.19: spawn_agent
    pub async fn spawn_agent(&self, _name: &str) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(100);
        let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
        let (status_tx, _) = broadcast::channel(10);

        let context = AgentContext {
            id,
            status: AgentStatus::Initializing,
            message_tx: msg_tx,
            cancel_tx,
            status_tx: status_tx.clone(),
        };

        self.agents.write().await.insert(id, context);

        let status_tx_clone = status_tx.clone();
        let outputs_clone = self.outputs.clone();

        tokio::spawn(async move {
            let _ = status_tx_clone.send(AgentStatus::Running);

            loop {
                tokio::select! {
                    msg = msg_rx.recv() => {
                        if let Some(m) = msg {
                            outputs_clone.write().await.push(AgentOutput {
                                agent_id: id,
                                data: format!("Processed: {}", m.payload),
                            });
                        }
                    }
                    _ = &mut cancel_rx => {
                        let _ = status_tx_clone.send(AgentStatus::Cancelled);
                        break;
                    }
                }
            }
        });

        Ok(id)
    }

    // 3.20: send_to_agent
    pub async fn send_to_agent(&self, agent_id: Uuid, payload: String) -> Result<()> {
        if let Some(agent) = self.agents.read().await.get(&agent_id) {
            let msg = Message {
                id: Uuid::new_v4(),
                sender: "system".to_string(),
                payload,
                timestamp: Utc::now(),
            };
            agent
                .message_tx
                .send(msg)
                .await
                .map_err(|_| anyhow::anyhow!("Failed to send message"))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Agent not found"))
        }
    }

    // 3.21: wait_agent
    pub async fn wait_agent(&self, agent_id: Uuid) -> Result<AgentStatus> {
        let rx = {
            let agents = self.agents.read().await;
            if let Some(agent) = agents.get(&agent_id) {
                agent.status_tx.subscribe()
            } else {
                return Err(anyhow::anyhow!("Agent not found"));
            }
        };

        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(status @ AgentStatus::Completed)
                | Ok(status @ AgentStatus::Failed(_))
                | Ok(status @ AgentStatus::Cancelled) => {
                    return Ok(status);
                }
                Ok(_) => continue,
                Err(_) => return Err(anyhow::anyhow!("Agent channel closed")),
            }
        }
    }

    // 3.22: cancel_agent
    pub async fn cancel_agent(&self, agent_id: Uuid) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.remove(&agent_id) {
            let _ = agent.cancel_tx.send(());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Agent not found"))
        }
    }

    // 3.23: agent_output_collection
    pub async fn collect_outputs(&self, agent_id: Uuid) -> Vec<AgentOutput> {
        let outputs = self.outputs.read().await;
        outputs
            .iter()
            .filter(|o| o.agent_id == agent_id)
            .cloned()
            .collect()
    }

    // 3.24: agent_status_monitoring
    pub async fn monitor_status(&self, agent_id: Uuid) -> Result<broadcast::Receiver<AgentStatus>> {
        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(&agent_id) {
            Ok(agent.status_tx.subscribe())
        } else {
            Err(anyhow::anyhow!("Agent not found"))
        }
    }

    // 3.25: task_priority_management
    pub async fn enqueue_task(&self, payload: String, priority: TaskPriority) -> Uuid {
        let task = TaskItem {
            id: Uuid::new_v4(),
            priority,
            payload,
            created_at: Utc::now(),
        };
        let id = task.id;
        self.task_queue.lock().await.push(task);
        id
    }

    pub async fn dequeue_highest_priority_task(&self) -> Option<TaskItem> {
        self.task_queue.lock().await.pop()
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}
