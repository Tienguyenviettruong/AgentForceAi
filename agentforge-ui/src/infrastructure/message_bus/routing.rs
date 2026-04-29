use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    Direct,
    Broadcast,
    RoleGroup,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    pub id: String,
    pub team_instance_id: String,
    pub sender_member_id: String,
    pub recipient_member_id: Option<String>,
    pub recipient_role: Option<String>,
    pub message_type: MessageType,
    pub content: String,
    pub metadata: Option<String>,
    pub delivery_status: String,
    pub created_at: String,
}

impl TeamMessage {
    pub fn new_direct(
        team_instance_id: String,
        sender: String,
        recipient: String,
        content: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            team_instance_id,
            sender_member_id: sender,
            recipient_member_id: Some(recipient),
            recipient_role: None,
            message_type: MessageType::Direct,
            content,
            metadata: None,
            delivery_status: "delivered".to_string(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn new_broadcast(
        team_instance_id: String,
        sender: String,
        content: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            team_instance_id,
            sender_member_id: sender,
            recipient_member_id: None,
            recipient_role: None,
            message_type: MessageType::Broadcast,
            content,
            metadata: None,
            delivery_status: "delivered".to_string(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn new_role_group(
        team_instance_id: String,
        sender: String,
        role: String,
        content: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            team_instance_id,
            sender_member_id: sender,
            recipient_member_id: None,
            recipient_role: Some(role),
            message_type: MessageType::RoleGroup,
            content,
            metadata: None,
            delivery_status: "delivered".to_string(),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

pub struct TeamBusRouter {
    // member_id -> [(team_instance_id, sender channel)]
    direct_channels: Arc<RwLock<HashMap<String, Vec<(String, mpsc::Sender<TeamMessage>)>>>>,
    // team_instance_id -> broadcast channel
    broadcast_channels: Arc<RwLock<HashMap<String, broadcast::Sender<TeamMessage>>>>,
    // team_instance_id -> role -> set of member_ids
    role_members: Arc<RwLock<HashMap<String, HashMap<String, HashSet<String>>>>>,
}

impl Default for TeamBusRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl TeamBusRouter {
    pub fn new() -> Self {
        Self {
            direct_channels: Arc::new(RwLock::new(HashMap::new())),
            broadcast_channels: Arc::new(RwLock::new(HashMap::new())),
            role_members: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_member(&self, team_instance_id: &str, member_id: &str, role: &str) -> mpsc::Receiver<TeamMessage> {
        let (tx, rx) = mpsc::channel(100);
        let mut direct_guard = self.direct_channels.write().await;
        direct_guard
            .entry(member_id.to_string())
            .or_default()
            .push((team_instance_id.to_string(), tx));

        let mut roles_guard = self.role_members.write().await;
        let team_roles = roles_guard.entry(team_instance_id.to_string()).or_default();
        let members = team_roles.entry(role.to_string()).or_default();
        members.insert(member_id.to_string());

        let mut bc_guard = self.broadcast_channels.write().await;
        if !bc_guard.contains_key(team_instance_id) {
            let (bc_tx, _) = broadcast::channel(1000);
            bc_guard.insert(team_instance_id.to_string(), bc_tx);
        }

        rx
    }

    pub async fn unregister_member(&self, team_instance_id: &str, member_id: &str, role: &str) {
        let mut direct_guard = self.direct_channels.write().await;
        if let Some(list) = direct_guard.get_mut(member_id) {
            list.retain(|(iid, _)| iid != team_instance_id);
            if list.is_empty() {
                direct_guard.remove(member_id);
            }
        }

        if let Some(team_roles) = self.role_members.write().await.get_mut(team_instance_id) {
            if let Some(members) = team_roles.get_mut(role) {
                members.remove(member_id);
            }
        }
    }

    pub async fn subscribe_broadcast(&self, team_instance_id: &str) -> broadcast::Receiver<TeamMessage> {
        let mut bc_guard = self.broadcast_channels.write().await;
        if !bc_guard.contains_key(team_instance_id) {
            let (bc_tx, _) = broadcast::channel(1000);
            bc_guard.insert(team_instance_id.to_string(), bc_tx);
        }
        bc_guard
            .get(team_instance_id)
            .expect("broadcast channel must exist")
            .subscribe()
    }

    pub async fn route_message(&self, message: TeamMessage) -> Result<(), String> {
        match message.message_type {
            MessageType::Direct => {
                if let Some(recipient) = &message.recipient_member_id {
                    let channels = self.direct_channels.read().await;
                    if let Some(txs) = channels.get(recipient) {
                        let mut delivered = false;
                        for (iid, tx) in txs {
                            if iid == &message.team_instance_id {
                                tx.send(message.clone()).await.map_err(|e| e.to_string())?;
                                delivered = true;
                            }
                        }
                        if delivered {
                            return Ok(());
                        }
                    }
                    return Err("Recipient not found".to_string());
                }
                Err("Missing recipient for direct message".to_string())
            }
            MessageType::Broadcast => {
                let tx = {
                    let mut channels = self.broadcast_channels.write().await;
                    if !channels.contains_key(&message.team_instance_id) {
                        let (bc_tx, _) = broadcast::channel(1000);
                        channels.insert(message.team_instance_id.clone(), bc_tx);
                    }
                    channels
                        .get(&message.team_instance_id)
                        .cloned()
                        .ok_or_else(|| "Team instance not found".to_string())?
                };
                let _ = tx.send(message);
                Ok(())
            }
            MessageType::RoleGroup => {
                if let Some(role) = &message.recipient_role {
                    let mut recipients = Vec::new();
                    {
                        let roles_guard = self.role_members.read().await;
                        if let Some(team_roles) = roles_guard.get(&message.team_instance_id) {
                            if let Some(members) = team_roles.get(role) {
                                recipients.extend(members.iter().cloned());
                            }
                        }
                    }
                    
                    let channels = self.direct_channels.read().await;
                    for recipient in recipients {
                        if let Some(txs) = channels.get(&recipient) {
                            for (iid, tx) in txs {
                                if iid == &message.team_instance_id {
                                    let _ = tx.send(message.clone()).await;
                                }
                            }
                        }
                    }
                    return Ok(());
                }
                Err("Missing role for role group message".to_string())
            }
            MessageType::System => {
                let tx = {
                    let mut channels = self.broadcast_channels.write().await;
                    if !channels.contains_key(&message.team_instance_id) {
                        let (bc_tx, _) = broadcast::channel(1000);
                        channels.insert(message.team_instance_id.clone(), bc_tx);
                    }
                    channels.get(&message.team_instance_id).cloned()
                };
                if let Some(tx) = tx {
                    let _ = tx.send(message);
                }
                Ok(())
            }
        }
    }
}
