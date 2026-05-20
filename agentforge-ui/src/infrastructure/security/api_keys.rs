use super::keychain::Keychain;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub provider: String,
    pub key: String,
}

pub struct ApiKeyManager {
    keychain: Keychain,
}

impl ApiKeyManager {
    /// Initialize a new API Key manager asynchronously.
    pub async fn new() -> Result<Self> {
        Ok(Self {
            keychain: Keychain::new().await?,
        })
    }

    /// Retrieve an API key securely for a given provider.
    pub async fn get_key(&self, provider: &str) -> Result<Option<String>> {
        self.keychain.get_secret("api_keys", provider).await
    }

    /// Save an API key securely for a given provider.
    pub async fn set_key(&self, provider: &str, key: &str) -> Result<()> {
        self.keychain.set_secret("api_keys", provider, key).await
    }

    /// Delete an API key securely for a given provider.
    pub async fn delete_key(&self, provider: &str) -> Result<()> {
        self.keychain.delete_secret("api_keys", provider).await
    }

    /// Validate an API key structure (basic validation).
    pub async fn validate_key_format(key: &str) -> bool {
        !key.trim().is_empty() && key.len() > 10
    }
}
