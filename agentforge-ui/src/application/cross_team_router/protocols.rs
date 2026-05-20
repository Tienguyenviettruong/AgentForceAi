use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolState {
    Initiated,
    Negotiating,
    Active,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct CollaborationProtocol {
    pub protocol_id: String,
    pub teams: Vec<String>,
    pub terms: String,
    pub state: ProtocolState,
}

pub struct ProtocolManager {
    protocols: HashMap<String, CollaborationProtocol>,
}

impl Default for ProtocolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolManager {
    pub fn new() -> Self {
        Self {
            protocols: HashMap::new(),
        }
    }

    pub fn initiate_protocol(
        &mut self,
        protocol_id: &str,
        teams: Vec<String>,
        terms: &str,
    ) -> Result<()> {
        let protocol = CollaborationProtocol {
            protocol_id: protocol_id.to_string(),
            teams,
            terms: terms.to_string(),
            state: ProtocolState::Initiated,
        };
        self.protocols.insert(protocol_id.to_string(), protocol);
        Ok(())
    }

    pub fn update_state(&mut self, protocol_id: &str, new_state: ProtocolState) -> Result<()> {
        if let Some(protocol) = self.protocols.get_mut(protocol_id) {
            protocol.state = new_state;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Protocol not found"))
        }
    }

    pub fn get_protocol(&self, protocol_id: &str) -> Option<CollaborationProtocol> {
        self.protocols.get(protocol_id).cloned()
    }
}
