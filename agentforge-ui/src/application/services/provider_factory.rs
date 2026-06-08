use crate::providers::BaseProviderAdapter;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// Determine provider kind from provider config
pub fn provider_kind(p: &crate::db::Provider) -> &str {
    match p.provider_name.as_str() {
        "openrouter" | "claude" | "gemini" | "codex" | "opencode" | "custom" => {
            p.provider_name.as_str()
        }
        _ => match p.adapter_type.as_str() {
            "AnthropicAdapter" => "claude",
            "OpenAIAdapter" => "codex",
            "GeminiAdapter" => "gemini",
            "OpenCodeAdapter" => "opencode",
            "CustomAdapter" => "custom",
            _ => p.provider_name.as_str(),
        },
    }
}

/// Create a provider adapter based on the provider config, initialize it, and return as Arc<dyn BaseProviderAdapter>.
/// Returns None if initialization fails.
pub fn create_adapter(
    provider_config: &crate::db::Provider,
) -> Option<Arc<dyn BaseProviderAdapter>> {
    match provider_kind(provider_config) {
        "openrouter" => {
            let mut adapter = crate::providers::openrouter::OpenRouterAdapter::new();
            if adapter.initialize(provider_config).is_ok() {
                Some(Arc::new(adapter) as Arc<dyn BaseProviderAdapter>)
            } else {
                None
            }
        }
        "claude" => {
            let mut adapter = crate::providers::claude::ClaudeAdapter::new();
            if adapter.initialize(provider_config).is_ok() {
                Some(Arc::new(adapter) as Arc<dyn BaseProviderAdapter>)
            } else {
                None
            }
        }
        "gemini" => {
            let mut adapter = crate::infrastructure::llm_providers::gemini::GeminiAdapter::new();
            if adapter.initialize(provider_config).is_ok() {
                Some(Arc::new(adapter) as Arc<dyn BaseProviderAdapter>)
            } else {
                None
            }
        }
        "codex" => {
            let mut adapter = crate::providers::codex::CodexAdapter::new();
            if adapter.initialize(provider_config).is_ok() {
                Some(Arc::new(adapter) as Arc<dyn BaseProviderAdapter>)
            } else {
                None
            }
        }
        "opencode" => {
            let mut adapter = crate::providers::opencode::OpenCodeAdapter::new();
            if adapter.initialize(provider_config).is_ok() {
                Some(Arc::new(adapter) as Arc<dyn BaseProviderAdapter>)
            } else {
                None
            }
        }
        "custom" => {
            let mut adapter = crate::infrastructure::llm_providers::custom::CustomAdapterSDK::new(
                "CustomAdapter",
            );
            if adapter.initialize(provider_config).is_ok() {
                Some(Arc::new(adapter) as Arc<dyn BaseProviderAdapter>)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Shared provider adapter cache for long-running orchestration workers.
///
/// Provider adapters are initialized from immutable provider rows, so workers can reuse the
/// initialized adapter instead of paying setup cost on every task execution. The cache key includes
/// all fields that change the connection/model identity.
#[derive(Default)]
pub struct ProviderAdapterCache {
    adapters: Mutex<HashMap<String, Arc<dyn BaseProviderAdapter>>>,
}

impl ProviderAdapterCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_create(
        &self,
        provider_config: &crate::db::Provider,
    ) -> Option<Arc<dyn BaseProviderAdapter>> {
        let key = Self::cache_key(provider_config);
        if let Some(adapter) = self.adapters.lock().ok()?.get(&key).cloned() {
            return Some(adapter);
        }

        let adapter = create_adapter(provider_config)?;
        self.adapters.lock().ok()?.insert(key, adapter.clone());
        Some(adapter)
    }

    pub fn invalidate(&self, provider_id: &str) {
        if let Ok(mut adapters) = self.adapters.lock() {
            adapters.retain(|key, _| !key.starts_with(provider_id));
        }
    }

    fn cache_key(provider_config: &crate::db::Provider) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            provider_config.id,
            provider_config.provider_name,
            provider_config.model,
            provider_config.adapter_type,
            provider_config.command.as_deref().unwrap_or_default(),
            provider_config.api_key_ref.as_deref().unwrap_or_default()
        )
    }
}

/// Resolve provider config for an agent from the database.
/// Tries by provider name first, then falls back to matching by adapter type.
pub fn resolve_provider_config(
    db: &dyn crate::core::traits::ProviderConfigPort,
    agent: &crate::db::Agent,
) -> Option<crate::db::Provider> {
    db.get_provider_by_name(&agent.provider)
        .ok()
        .flatten()
        .or_else(|| {
            db.list_providers().ok().and_then(|providers| {
                providers
                    .into_iter()
                    .find(|p| provider_kind(p) == agent.provider.as_str())
            })
        })
}

#[cfg(test)]
mod tests {
    fn provider(adapter_type: &str, provider_name: &str) -> crate::db::Provider {
        crate::db::Provider {
            id: "provider-test".to_string(),
            provider_name: provider_name.to_string(),
            model: "test-model".to_string(),
            adapter_type: adapter_type.to_string(),
            command: Some("https://example.test".to_string()),
            api_key_ref: Some("env:AGENTFORGE_TEST_KEY".to_string()),
            status: "active".to_string(),
            capabilities: None,
        }
    }

    #[test]
    fn custom_adapter_maps_to_custom_provider() {
        let provider = provider("CustomAdapter", "ClaudePro");
        assert_eq!(super::provider_kind(&provider), "custom");
        let adapter = super::create_adapter(&provider).expect("custom adapter should initialize");
        assert_eq!(adapter.provider_id(), "custom");
    }

    #[test]
    fn unknown_adapter_fails_closed() {
        let provider = provider("UnknownAdapter", "unknown-provider");
        assert!(super::create_adapter(&provider).is_none());
    }
}
