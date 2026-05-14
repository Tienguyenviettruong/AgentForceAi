use crate::providers::BaseProviderAdapter;
use std::sync::Arc;

/// Determine provider kind from provider config
pub fn provider_kind(p: &crate::db::Provider) -> &str {
    match p.provider_name.as_str() {
        "openrouter" | "claude" | "gemini" | "codex" | "opencode" => p.provider_name.as_str(),
        _ => match p.adapter_type.as_str() {
            "AnthropicAdapter" => "claude",
            "OpenAIAdapter" => "codex",
            "GeminiAdapter" => "gemini",
            "OpenCodeAdapter" => "opencode",
            _ => p.provider_name.as_str(),
        },
    }
}

/// Create a provider adapter based on the provider config, initialize it, and return as Arc<dyn BaseProviderAdapter>.
/// Returns None if initialization fails.
pub fn create_adapter(provider_config: &crate::db::Provider) -> Option<Arc<dyn BaseProviderAdapter>> {
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
            let mut adapter = crate::providers::openrouter::OpenRouterAdapter::new();
            if adapter.initialize(provider_config).is_ok() {
                Some(Arc::new(adapter) as Arc<dyn BaseProviderAdapter>)
            } else {
                None
            }
        }
        "opencode" => {
            let mut adapter = crate::providers::openrouter::OpenRouterAdapter::new();
            if adapter.initialize(provider_config).is_ok() {
                Some(Arc::new(adapter) as Arc<dyn BaseProviderAdapter>)
            } else {
                None
            }
        }
        _ => {
            // Default fallback to openrouter
            let mut adapter = crate::providers::openrouter::OpenRouterAdapter::new();
            if adapter.initialize(provider_config).is_ok() {
                Some(Arc::new(adapter) as Arc<dyn BaseProviderAdapter>)
            } else {
                None
            }
        }
    }
}

/// Resolve provider config for an agent from the database.
/// Tries by provider name first, then falls back to matching by adapter type.
pub fn resolve_provider_config(
    db: &dyn crate::core::traits::database::DatabasePort,
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
