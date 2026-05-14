use anyhow::Result;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::sync::Mutex;
use once_cell::sync::Lazy;

static GLOBAL_MODEL: Lazy<Mutex<TextEmbedding>> = Lazy::new(|| {
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true)
    )
    .expect("Failed to initialize fastembed model");
    Mutex::new(model)
});

pub struct EmbeddingProvider;

impl Default for EmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingProvider {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let text = text.to_string();
        // ONNX Runtime (used by fastembed) requires a large stack (~8-16 MB).
        // tokio::task::spawn_blocking only provides ~1–2 MB on Windows, causing
        // STATUS_STACK_BUFFER_OVERRUN. We use a dedicated thread with 16 MB stack.
        let (tx, rx) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024) // 16 MB stack for ONNX Runtime
            .spawn(move || {
                let result = GLOBAL_MODEL
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))
                    .and_then(|mut model| {
                        model.embed(vec![text], None)
                            .map_err(|e| anyhow::anyhow!("Embedding failed: {}", e))
                    });
                let _ = tx.send(result);
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn embedding thread: {}", e))?;

        let embeddings = rx.await
            .map_err(|_| anyhow::anyhow!("Embedding thread dropped sender"))??;
        
        if let Some(embedding) = embeddings.into_iter().next() {
            Ok(embedding)
        } else {
            Err(anyhow::anyhow!("Failed to generate embedding"))
        }
    }
}
