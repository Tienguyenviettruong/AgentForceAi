use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::oneshot;

/// Holds the model once it finishes loading.
/// None  → still loading (background thread not done yet)
/// Some(Ok(model)) → ready
/// Some(Err(msg))  → failed to load
static GLOBAL_MODEL: OnceLock<Arc<Mutex<Result<TextEmbedding, String>>>> = OnceLock::new();

/// Kick off the model load in a background thread. Safe to call multiple times — only the first
/// call does anything. Should be called at app startup so the model is likely ready by the time
/// the first chat message arrives.
pub fn prewarm_embedding_model() {
    let cell = GLOBAL_MODEL.get_or_init(|| {
        let slot: Arc<Mutex<Result<TextEmbedding, String>>> =
            Arc::new(Mutex::new(Err("loading".to_string())));
        let slot_clone = slot.clone();
        std::thread::Builder::new()
            .name("embedding-model-init".to_string())
            .stack_size(16 * 1024 * 1024) // 16 MB for ONNX Runtime
            .spawn(move || {
                tracing::info!("Loading ONNX embedding model in background");
                let result = TextEmbedding::try_new(
                    InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                        .with_show_download_progress(false),
                )
                .map_err(|e| format!("Embedding model load failed: {e}"));
                match &result {
                    Ok(_) => tracing::info!("Embedding model loaded"),
                    Err(e) => tracing::warn!("Embedding model load failed: {}", e),
                }
                if let Ok(mut guard) = slot_clone.lock() {
                    *guard = result;
                }
            })
            .ok();
        slot
    });
    // Ensure OnceLock is populated even if prewarm is called multiple times.
    let _ = cell;
}

pub struct EmbeddingProvider;

impl Default for EmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingProvider {
    pub fn new() -> Self {
        // Trigger background load if not already started.
        prewarm_embedding_model();
        Self
    }

    /// Returns Err immediately if the model is still loading — never blocks.
    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let text = text.to_string();
        // Check if the model slot exists and is ready WITHOUT blocking.
        let slot = match GLOBAL_MODEL.get() {
            Some(s) => s.clone(),
            None => {
                // prewarm not triggered yet (shouldn't happen since new() calls it)
                prewarm_embedding_model();
                return Err(anyhow::anyhow!("Embedding model not ready yet"));
            }
        };

        // Try to acquire the lock non-blockingly to check model status.
        let is_ready = slot.try_lock().map(|guard| guard.is_ok()).unwrap_or(false); // locked = still loading

        if !is_ready {
            return Err(anyhow::anyhow!("Embedding model not ready yet"));
        }

        // Model is ready — run embedding on a dedicated thread (ONNX needs large stack).
        let (tx, rx) = oneshot::channel();
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let result = slot
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))
                    .and_then(|mut guard| {
                        let model = guard.as_mut().map_err(|e| anyhow::anyhow!(e.clone()))?;
                        model
                            .embed(vec![text], None)
                            .map_err(|e| anyhow::anyhow!("Embedding generation failed: {}", e))
                    });
                let _ = tx.send(result);
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn embedding thread: {}", e))?;

        let embeddings = rx
            .await
            .map_err(|_| anyhow::anyhow!("Embedding thread dropped sender"))??;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty embedding result"))
    }
}
