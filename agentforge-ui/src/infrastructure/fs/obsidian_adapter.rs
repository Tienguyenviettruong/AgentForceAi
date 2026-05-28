use crate::knowledge::core::{KnowledgeItem, RetentionPolicy, Tag};
use chrono::Utc;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ObsidianMarkdown {
    pub frontmatter: Option<String>,
    pub content: String,
    pub links: Vec<String>,
    pub tags: Vec<String>,
}

impl ObsidianMarkdown {
    /// Parse raw Obsidian Markdown content
    pub fn parse(raw_content: &str) -> Self {
        let mut frontmatter = None;
        let mut content = raw_content.to_string();

        // Very basic frontmatter parsing
        if raw_content.starts_with("---\n") {
            if let Some(end_idx) = raw_content[4..].find("---\n") {
                frontmatter = Some(raw_content[4..end_idx + 4].to_string());
                content = raw_content[end_idx + 8..].to_string();
            } else if let Some(end_idx) = raw_content[4..].find("---\r\n") {
                frontmatter = Some(raw_content[4..end_idx + 4].to_string());
                content = raw_content[end_idx + 9..].to_string();
            }
        }

        let mut links = Vec::new();
        let mut tags = Vec::new();

        // Extract wikilinks: [[link]]
        let mut current_idx = 0;
        while let Some(start) = content[current_idx..].find("[[") {
            let actual_start = current_idx + start;
            if let Some(end) = content[actual_start..].find("]]") {
                let actual_end = actual_start + end;
                let link_content = &content[actual_start + 2..actual_end];

                // Obsidian links can have aliases [[Link|Alias]]
                let link = if let Some(pipe_idx) = link_content.find('|') {
                    link_content[..pipe_idx].to_string()
                } else {
                    link_content.to_string()
                };

                links.push(link);
                current_idx = actual_end + 2;
            } else {
                break;
            }
        }

        // Extract tags: #tag (simple whitespace-based approach)
        for line in content.lines() {
            for word in line.split_whitespace() {
                if word.starts_with('#') && word.len() > 1 {
                    // Strip potential trailing punctuation
                    let tag = word[1..]
                        .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                        .to_string();
                    if !tag.is_empty() {
                        tags.push(tag);
                    }
                }
            }
        }

        Self {
            frontmatter,
            content,
            links,
            tags,
        }
    }

    /// Convert into a KnowledgeItem for the Brain
    pub async fn into_knowledge_item(
        self,
        title: &str,
        vault_path: Option<String>,
    ) -> Result<KnowledgeItem, String> {
        let tags: Vec<Tag> = self.tags.into_iter().map(Tag).collect();

        Ok(KnowledgeItem {
            id: Uuid::new_v4(),
            record_kind: crate::knowledge::core::KnowledgeRecordKind::Document,
            title: title.to_string(),
            content_hash: Some(KnowledgeItem::content_hash(&self.content)),
            content: self.content,
            tags,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            retention_policy: RetentionPolicy::KeepForever,
            source_kind: if vault_path.is_some() {
                "obsidian".to_string()
            } else {
                "manual".to_string()
            },
            source_uri_normalized: vault_path
                .as_deref()
                .map(KnowledgeItem::normalize_file_source),
            vault_path,
            origin_run_id: None,
            origin_session_id: None,
            origin_instance_id: None,
            origin_agent_id: None,
        })
    }

    /// Read an Obsidian Markdown file from disk
    pub async fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Ok(Self::parse(&content))
    }
}

use notify::{Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct ObsidianWatcher {
    watcher: RecommendedWatcher,
    pub rx: mpsc::Receiver<notify::Result<NotifyEvent>>,
}

impl ObsidianWatcher {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel(100);

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.blocking_send(res);
        })
        .map_err(|e| e.to_string())?;

        watcher
            .watch(path.as_ref(), RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;

        Ok(Self { watcher, rx })
    }
}

use crate::core::traits::database::DatabasePort;
use notify::EventKind;
use std::path::PathBuf;

fn normalized_vault_path(path: &Path) -> String {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let normalized = resolved.to_string_lossy().replace('\\', "/");
    normalized
        .strip_prefix("//?/")
        .unwrap_or(&normalized)
        .to_string()
}

pub async fn sync_obsidian_file(
    path: &std::path::Path,
    db: &dyn crate::core::traits::database::DatabasePort,
) {
    if path.extension().and_then(|s| s.to_str()) == Some("md") {
        if let Ok(markdown) = ObsidianMarkdown::read_from_file(path).await {
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled");
            let vault_path_str = normalized_vault_path(path);
            let source_uri = KnowledgeItem::normalize_file_source(&vault_path_str);

            // Check if an item with this vault_path already exists
            let existing_item = db
                .get_all_knowledge_items()
                .unwrap_or_default()
                .into_iter()
                .find(|i| {
                    i.source_uri_normalized.as_deref() == Some(source_uri.as_str())
                        || i.vault_path
                            .as_ref()
                            .map(|p| normalized_vault_path(Path::new(p)))
                            == Some(vault_path_str.clone())
                });

            let Ok(mut item) = markdown
                .into_knowledge_item(title, Some(vault_path_str))
                .await
            else {
                return;
            };

            if let Some(existing) = existing_item {
                item.id = existing.id; // Preserve ID so it updates instead of duplicating
            }

            if let Err(e) = db.upsert_knowledge_item(&item) {
                eprintln!("Failed to sync Obsidian file to DB: {}", e);
            } else {
                let text_chunks = chunk_text(&item.content, 500);
                let embedding_provider = crate::providers::embeddings::EmbeddingProvider::new();

                let mut chunk_data = Vec::new();
                for (i, chunk_text) in text_chunks.into_iter().enumerate() {
                    let text: String = chunk_text;
                    if let Ok(embedding) = embedding_provider.get_embedding(&text).await {
                        chunk_data.push((i, text, embedding));
                    }
                }

                if !chunk_data.is_empty() {
                    if let Err(e) = db.upsert_knowledge_chunks(&item.id.to_string(), chunk_data) {
                        eprintln!("Failed to sync chunks to DB: {}", e);
                    }
                }
                println!("Successfully synced Obsidian file: {:?}", path);
            }
        }
    }
}
impl ObsidianWatcher {
    /// Starts the background sync loop for a given Obsidian vault path
    pub fn start_sync(
        db: Arc<dyn DatabasePort>,
        vault_path: PathBuf,
        runtime: Arc<tokio::runtime::Runtime>,
    ) -> Result<RecommendedWatcher, String> {
        // Initial scan of the directory
        let scan_path = vault_path.clone();
        let scan_db = db.clone();
        runtime.spawn(async move {
            // Using walkdir to find all .md files
            for entry in walkdir::WalkDir::new(&scan_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    sync_obsidian_file(entry.path(), &*scan_db).await;
                }
            }
        });

        let watcher = Self::new(&vault_path)?;

        let mut rx = watcher.rx;

        // Background task to process events
        runtime.spawn(async move {
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => {
                        // Only care about file creations and modifications
                        if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                            for path in event.paths {
                                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                                    sync_obsidian_file(&path, &*db).await;
                                }
                            }
                        }
                    }
                    Err(e) => println!("Watch error: {:?}", e),
                }
            }
        });

        // The watcher needs to be kept alive, so we return it
        Ok(watcher.watcher)
    }
}

pub fn chunk_text(text: &str, max_tokens: usize) -> Vec<String> {
    // Basic chunker: split by double newlines (paragraphs), then group up to ~max_tokens (assuming ~4 chars/token)
    let max_chars = max_tokens * 4;
    let paragraphs: Vec<&str> = text.split("\n\n").collect();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for p in paragraphs {
        if current_chunk.len() + p.len() > max_chars && !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
            current_chunk.clear();
        }
        current_chunk.push_str(p);
        current_chunk.push_str("\n\n");
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    chunks
}
