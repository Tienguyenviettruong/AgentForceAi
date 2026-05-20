use crate::teambus::routing::TeamMessage;
use std::hash::{DefaultHasher, Hash, Hasher};

pub struct SecurityManager {
    encryption_key: String,
}

impl SecurityManager {
    pub fn new(key: &str) -> Self {
        Self {
            encryption_key: key.to_string(),
        }
    }

    /// Encrypts a message payload (mock implementation)
    pub fn encrypt(&self, payload: &str) -> String {
        // In a real implementation, use AES-GCM or similar.
        // Here we just do a mock "encryption" by reversing the string and appending the key prefix.
        let reversed: String = payload.chars().rev().collect();
        format!(
            "ENC[{}]_{}",
            self.encryption_key.chars().take(4).collect::<String>(),
            reversed
        )
    }

    /// Decrypts a message payload (mock implementation)
    pub fn decrypt(&self, encrypted_payload: &str) -> Result<String, String> {
        if encrypted_payload.starts_with("ENC[") {
            let parts: Vec<&str> = encrypted_payload.splitn(2, "]_").collect();
            if parts.len() == 2 {
                let reversed = parts[1];
                let original: String = reversed.chars().rev().collect();
                return Ok(original);
            }
        }
        Err("Invalid encrypted payload format".to_string())
    }

    /// Generates an integrity hash for a message
    pub fn generate_integrity_hash(&self, message: &TeamMessage) -> String {
        let mut hasher = DefaultHasher::new();
        message.id.hash(&mut hasher);
        message.team_instance_id.hash(&mut hasher);
        message.sender_member_id.hash(&mut hasher);
        if let Some(recipient) = &message.recipient_member_id {
            recipient.hash(&mut hasher);
        }
        message.content.hash(&mut hasher);
        self.encryption_key.hash(&mut hasher); // Salt with our key

        format!("{:x}", hasher.finish())
    }

    /// Verifies the integrity of a message against a provided hash
    pub fn verify_integrity(&self, message: &TeamMessage, provided_hash: &str) -> bool {
        let calculated_hash = self.generate_integrity_hash(message);
        calculated_hash == provided_hash
    }

    /// Secures a message by encrypting its content and adding an integrity hash to metadata
    pub fn secure_message(&self, mut message: TeamMessage) -> TeamMessage {
        // Encrypt content
        let encrypted_content = self.encrypt(&message.content);
        message.content = encrypted_content;

        // Generate hash (after encryption so we verify what's sent)
        let hash = self.generate_integrity_hash(&message);

        // Store hash in metadata
        // In a real system we'd parse JSON, add it, and serialize back
        message.metadata = Some(format!("{{\"integrity_hash\": \"{}\"}}", hash));

        message
    }
}
