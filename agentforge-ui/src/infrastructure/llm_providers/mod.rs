pub mod claude;
pub mod codex;
pub mod custom;
pub mod embeddings;
pub mod gemini;
pub mod health;
pub mod iflow;
pub mod opencode;
pub mod openrouter;
pub mod plugin;
pub mod registry;
mod runtime;

use std::future::Future;
use std::pin::Pin;

/// Abstract interface standardizing interactions with all AI providers
/// (Task 1.11: Define BaseProviderAdapter trait/interface)
/// (Task 1.14: Implement streaming response handling interface)
pub trait BaseProviderAdapter: Send + Sync {
    /// Returns the provider's identifier (e.g., "claude", "openai", "gemini")
    fn provider_id(&self) -> &'static str;

    /// Returns the capability profile of this model/provider.
    /// Used by CapabilityRouter to decide how to handle file attachments.
    /// Default: text-only (safe fallback for local/unknown models).
    fn capabilities(&self) -> crate::core::models::ModelCapability {
        crate::core::models::ModelCapability::text_only()
    }

    /// Initialize the provider with API keys, endpoints, and other configuration
    fn initialize(&mut self, config: &crate::db::Provider) -> Result<(), anyhow::Error>;

    /// Send a chat message and receive a single complete response
    fn send_message(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, anyhow::Error>> + Send>>;

    /// Send a chat message and receive a streaming response
    fn send_message_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Box<
                            dyn futures::Stream<
                                    Item = Result<crate::providers::StreamChunk, anyhow::Error>,
                                > + Send
                                + Unpin,
                        >,
                        anyhow::Error,
                    >,
                > + Send,
        >,
    >;

    /// Perform a health check on the provider connection
    /// (Task 1.15: Build provider health check and auto-reconnection logic)
    fn check_health(&self) -> Pin<Box<dyn Future<Output = Result<bool, anyhow::Error>> + Send>>;
}

pub use crate::core::models::{ChatMessage, ChatResponse, StreamChunk, TokenUsage};
