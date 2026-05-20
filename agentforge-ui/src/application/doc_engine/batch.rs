use anyhow::Result;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::docs::engine::{DocumentEngine, DocumentRequest, DocumentResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub job_id: String,
    pub requests: Vec<DocumentRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub job_id: String,
    pub results: Vec<Result<DocumentResult, String>>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

pub struct BatchProcessor {
    engine: Arc<DocumentEngine>,
}

impl BatchProcessor {
    pub fn new(engine: Arc<DocumentEngine>) -> Self {
        Self { engine }
    }

    pub async fn process_batch(&self, batch_request: BatchRequest) -> Result<BatchResult> {
        let mut futures: Vec<tokio::task::JoinHandle<Result<DocumentResult, anyhow::Error>>> =
            Vec::new();

        for req in batch_request.requests {
            let engine_clone = self.engine.clone();
            futures.push(tokio::spawn(async move {
                engine_clone.generate_document(req).await
            }));
        }

        let completed_jobs = join_all(futures).await;

        let mut results = Vec::new();
        for job_result in completed_jobs {
            match job_result {
                Ok(Ok(doc_result)) => results.push(Ok(doc_result)),
                Ok(Err(e)) => results.push(Err(e.to_string())),
                Err(e) => results.push(Err(format!("Task panic: {}", e))),
            }
        }

        Ok(BatchResult {
            job_id: batch_request.job_id,
            results,
            completed_at: chrono::Utc::now(),
        })
    }
}
