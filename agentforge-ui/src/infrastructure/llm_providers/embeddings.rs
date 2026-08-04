use anyhow::{anyhow, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::OnceLock;
use tokio::sync::oneshot;

const EMBEDDING_QUEUE_CAPACITY: usize = 32;
const EMBEDDING_WORKER_STACK_BYTES: usize = 16 * 1024 * 1024;

struct EmbeddingRequest {
    text: String,
    response: oneshot::Sender<std::result::Result<Vec<f32>, String>>,
}

static EMBEDDING_WORKER: OnceLock<async_channel::Sender<EmbeddingRequest>> = OnceLock::new();

fn embedding_worker() -> &'static async_channel::Sender<EmbeddingRequest> {
    EMBEDDING_WORKER.get_or_init(|| {
        let (tx, rx) = async_channel::bounded::<EmbeddingRequest>(EMBEDDING_QUEUE_CAPACITY);
        std::thread::Builder::new()
            .name("embedding-worker".to_string())
            .stack_size(EMBEDDING_WORKER_STACK_BYTES)
            .spawn(move || {
                tracing::info!("Loading ONNX embedding model on dedicated worker");
                let mut model = TextEmbedding::try_new(
                    InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                        .with_show_download_progress(false),
                )
                .map_err(|error| format!("Embedding model load failed: {error}"));

                match &model {
                    Ok(_) => tracing::info!("Embedding model loaded"),
                    Err(error) => tracing::warn!("{}", error),
                }

                while let Ok(request) = rx.recv_blocking() {
                    let result = match &mut model {
                        Ok(model) => model
                            .embed(vec![request.text], None)
                            .map_err(|error| format!("Embedding generation failed: {error}"))
                            .and_then(|mut embeddings| {
                                embeddings
                                    .drain(..)
                                    .next()
                                    .ok_or_else(|| "Empty embedding result".to_string())
                            }),
                        Err(error) => Err(error.clone()),
                    };
                    let _ = request.response.send(result);
                }
            })
            .expect("failed to spawn embedding worker");
        tx
    })
}

/// Starts the shared worker early when an installation explicitly prefers
/// warm semantic search. Normal application startup leaves it lazy.
pub fn prewarm_embedding_model() {
    let _ = embedding_worker();
}

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
        let (response_tx, response_rx) = oneshot::channel();
        embedding_worker()
            .send(EmbeddingRequest {
                text: text.to_string(),
                response: response_tx,
            })
            .await
            .map_err(|_| anyhow!("Embedding worker is unavailable"))?;

        response_rx
            .await
            .map_err(|_| anyhow!("Embedding worker stopped before returning a result"))?
            .map_err(anyhow::Error::msg)
    }
}
