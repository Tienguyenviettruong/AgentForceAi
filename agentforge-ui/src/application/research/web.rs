use crate::application::file_intelligence::{self, AnalyzeOptions};
use crate::core::models::{KnowledgeItem, RetentionPolicy, Tag};
use crate::core::traits::database::DatabasePort;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchQuery {
    pub id: Uuid,
    pub keywords: Vec<String>,
    pub max_results: usize,
    pub search_engine: String,
    #[serde(default)]
    pub recency_days: Option<u32>,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default = "default_fetch_pages")]
    pub fetch_pages: bool,
}

impl WebSearchQuery {
    pub fn new(query: &str, max_results: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            keywords: vec![query.to_string()],
            max_results,
            search_engine: "auto".to_string(),
            recency_days: None,
            domains: Vec::new(),
            fetch_pages: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub fetched_at: DateTime<Utc>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub content_summary: String,
}

#[derive(Debug, Clone, Default)]
pub struct ResearchNotebookSaveContext {
    pub source_kind: Option<String>,
    pub source_uri_normalized: Option<String>,
    pub origin_run_id: Option<String>,
    pub origin_session_id: Option<String>,
    pub origin_instance_id: Option<String>,
    pub origin_agent_id: Option<String>,
}

pub struct WebSearchEngine;

impl WebSearchEngine {
    pub async fn execute_search(query: &WebSearchQuery) -> Result<Vec<WebSearchResult>, String> {
        let max_results = query.max_results.clamp(1, 20);
        let raw_query = query.keywords.join(" ");
        if is_http_url(raw_query.trim()) {
            let result = fetch_url_as_result(raw_query.trim()).await?;
            return Ok(vec![result]);
        }

        let search_query = build_search_query(&raw_query, &query.domains, query.recency_days);
        let engine = query.search_engine.to_ascii_lowercase();
        let mut results = if engine == "bing"
            || has_env("BING_SEARCH_API_KEY")
            || has_env("AGENTFORGE_BING_SEARCH_API_KEY")
        {
            bing_search(&search_query, max_results, query.recency_days).await
        } else if engine == "serpapi" || has_env("SERPAPI_API_KEY") {
            serpapi_search(&search_query, max_results, query.recency_days).await
        } else {
            ddg_search(&search_query, max_results).await
        }?;

        if !query.domains.is_empty() {
            results.retain(|r| {
                query.domains.iter().any(|domain| {
                    let domain = domain.trim().trim_start_matches("site:");
                    r.url.contains(domain)
                })
            });
        }

        results.truncate(max_results);
        if query.fetch_pages {
            fetch_result_pages(&mut results).await;
        }
        Ok(results)
    }
}

pub async fn fetch_url_as_result(url: &str) -> Result<WebSearchResult, String> {
    let analysis = file_intelligence::analyze_url(
        url,
        AnalyzeOptions {
            max_text_chars: 18_000,
            ..AnalyzeOptions::default()
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    let title = if analysis.file_name.trim().is_empty() {
        url.to_string()
    } else {
        analysis.file_name.clone()
    };
    let content_summary = summarize_content(&analysis.text, 1400);
    Ok(WebSearchResult {
        title,
        url: analysis.source,
        snippet: analysis
            .notes
            .first()
            .cloned()
            .unwrap_or_else(|| "Fetched URL content.".to_string()),
        fetched_at: Utc::now(),
        content: analysis.text,
        content_summary,
    })
}

pub fn web_search_result_from_analysis(
    analysis: file_intelligence::FileAnalysis,
) -> WebSearchResult {
    let title = if analysis.file_name.trim().is_empty() {
        analysis.source.clone()
    } else {
        analysis.file_name.clone()
    };
    let content_summary = summarize_content(&analysis.text, 1400);
    WebSearchResult {
        title,
        url: analysis.source,
        snippet: analysis
            .notes
            .first()
            .cloned()
            .unwrap_or_else(|| "Fetched URL content.".to_string()),
        fetched_at: Utc::now(),
        content: analysis.text,
        content_summary,
    }
}

pub fn build_research_notebook(query: &str, results: &[WebSearchResult]) -> String {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let mut out = String::new();
    out.push_str(&format!("# Research Notebook: {}\n\n", query.trim()));
    out.push_str(&format!("- Created: {}\n", now));
    out.push_str(&format!("- Sources: {}\n", results.len()));
    out.push_str(
        "- Method: search results were fetched and normalized by AgentForge file intelligence.\n\n",
    );

    out.push_str("## Source Index\n\n");
    for (ix, result) in results.iter().enumerate() {
        out.push_str(&format!(
            "[{}] [{}]({}) - {}\n",
            ix + 1,
            result.title,
            result.url,
            result.snippet
        ));
    }

    out.push_str("\n## Synthesis Seeds\n\n");
    for (ix, result) in results.iter().enumerate() {
        let summary = if result.content_summary.trim().is_empty() {
            summarize_content(&result.snippet, 500)
        } else {
            result.content_summary.clone()
        };
        out.push_str(&format!(
            "### [{}] {}\n\n{}\n\n",
            ix + 1,
            result.title,
            summary
        ));
    }

    out.push_str("## Extracted Source Notes\n\n");
    for (ix, result) in results.iter().enumerate() {
        out.push_str(&format!(
            "### [{}] {}\n\nURL: {}\nFetched: {}\n\n",
            ix + 1,
            result.title,
            result.url,
            result.fetched_at
        ));
        if result.content.trim().is_empty() {
            out.push_str(&format!("Snippet: {}\n\n", result.snippet));
        } else {
            out.push_str("```text\n");
            out.push_str(&file_intelligence::truncate_chars(&result.content, 6000));
            out.push_str("\n```\n\n");
        }
    }
    out
}

pub async fn save_research_notebook(
    db: Arc<dyn DatabasePort>,
    query: &str,
    notebook: &str,
) -> anyhow::Result<String> {
    save_research_notebook_with_context(db, query, notebook, ResearchNotebookSaveContext::default())
        .await
}

pub async fn save_research_notebook_with_context(
    db: Arc<dyn DatabasePort>,
    query: &str,
    notebook: &str,
    context: ResearchNotebookSaveContext,
) -> anyhow::Result<String> {
    let source_kind = context
        .source_kind
        .clone()
        .unwrap_or_else(|| "web_research".to_string());
    let source_uri_normalized = context
        .source_uri_normalized
        .clone()
        .unwrap_or_else(|| research_source_uri(query, &context));

    let vault_path = std::env::var("AGENTFORGE_OBSIDIAN_VAULT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| db.get_setting("obsidian_vault_path").ok().flatten());

    let mut saved_to = "knowledge_db".to_string();
    let mut item = KnowledgeItem::new(
        if query.trim().is_empty() {
            "Research Notebook"
        } else {
            query.trim()
        },
        notebook,
        vec![
            Tag("research".to_string()),
            Tag("web".to_string()),
            Tag("research_notebook".to_string()),
        ],
        RetentionPolicy::KeepForever,
    );
    item.source_kind = source_kind;
    item.source_uri_normalized = Some(source_uri_normalized);
    item.origin_run_id = context.origin_run_id;
    item.origin_session_id = context.origin_session_id;
    item.origin_instance_id = context.origin_instance_id;
    item.origin_agent_id = context.origin_agent_id;

    if let Some(vault) = vault_path {
        let ts = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let path = std::path::PathBuf::from(vault)
            .join("Research")
            .join(format!("research_{}_{}.md", ts, slugify(query)));
        let write_result = (|| -> anyhow::Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, notebook.as_bytes())?;
            Ok(())
        })();
        match write_result {
            Ok(()) => {
                item.vault_path = Some(path.display().to_string());
                crate::infrastructure::fs::obsidian_adapter::sync_obsidian_file(&path, &*db).await;
                saved_to = path.display().to_string();
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "Research notebook Obsidian write failed; keeping knowledge DB copy"
                );
            }
        }
    }

    db.upsert_knowledge_item(&item)?;
    Ok(saved_to)
}

pub fn list_saved_research_notebooks(
    db: Arc<dyn DatabasePort>,
) -> anyhow::Result<Vec<KnowledgeItem>> {
    let mut items: Vec<KnowledgeItem> = db
        .get_all_knowledge_items()?
        .into_iter()
        .filter(is_research_notebook_item)
        .collect();
    items.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(items)
}

pub fn is_research_notebook_item(item: &KnowledgeItem) -> bool {
    item.source_kind == "web_research"
        || item.source_kind == "research_scratchpad"
        || item.tags.iter().any(|tag| tag.0 == "research_notebook")
        || item
            .content
            .trim_start()
            .starts_with("# Research Notebook:")
}

async fn fetch_result_pages(results: &mut [WebSearchResult]) {
    for result in results.iter_mut() {
        match file_intelligence::analyze_url(
            &result.url,
            AnalyzeOptions {
                max_text_chars: 18_000,
                ..AnalyzeOptions::default()
            },
        )
        .await
        {
            Ok(analysis) => {
                result.content = analysis.text;
                result.content_summary = summarize_content(&result.content, 1400);
                if result.title.trim().is_empty() {
                    result.title = analysis.file_name;
                }
            }
            Err(e) => {
                result.content_summary = format!("Fetch failed: {}", e);
            }
        }
    }
}

async fn bing_search(
    query: &str,
    max_results: usize,
    recency_days: Option<u32>,
) -> Result<Vec<WebSearchResult>, String> {
    let key = std::env::var("BING_SEARCH_API_KEY")
        .or_else(|_| std::env::var("AGENTFORGE_BING_SEARCH_API_KEY"))
        .map_err(|_| "Bing search key is not configured.".to_string())?;
    let endpoint = std::env::var("AGENTFORGE_BING_SEARCH_ENDPOINT")
        .unwrap_or_else(|_| "https://api.bing.microsoft.com/v7.0/search".to_string());

    let client = reqwest::Client::new();
    let mut request = client
        .get(endpoint)
        .header("Ocp-Apim-Subscription-Key", key)
        .query(&[
            ("q", query.to_string()),
            ("count", max_results.to_string()),
            ("textDecorations", "false".to_string()),
            ("textFormat", "Raw".to_string()),
        ]);
    if let Some(freshness) = bing_freshness(recency_days) {
        request = request.query(&[("freshness", freshness)]);
    }

    let json: Value = request
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    if let Some(values) = json
        .get("webPages")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_array())
    {
        for value in values {
            let title = value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url = value
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let snippet = value
                .get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if is_http_url(&url) {
                results.push(WebSearchResult {
                    title,
                    url,
                    snippet,
                    fetched_at: Utc::now(),
                    content: String::new(),
                    content_summary: String::new(),
                });
            }
        }
    }
    Ok(results)
}

async fn serpapi_search(
    query: &str,
    max_results: usize,
    recency_days: Option<u32>,
) -> Result<Vec<WebSearchResult>, String> {
    let key = std::env::var("SERPAPI_API_KEY")
        .map_err(|_| "SERPAPI_API_KEY is not configured.".to_string())?;
    let client = reqwest::Client::new();
    let mut params = vec![
        ("engine".to_string(), "google".to_string()),
        ("q".to_string(), query.to_string()),
        ("api_key".to_string(), key),
        ("num".to_string(), max_results.to_string()),
    ];
    if let Some(tbs) = serpapi_recency(recency_days) {
        params.push(("tbs".to_string(), tbs));
    }
    let json: Value = client
        .get("https://serpapi.com/search.json")
        .query(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    if let Some(values) = json.get("organic_results").and_then(|v| v.as_array()) {
        for value in values {
            let title = value
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url = value
                .get("link")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let snippet = value
                .get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if is_http_url(&url) {
                results.push(WebSearchResult {
                    title,
                    url,
                    snippet,
                    fetched_at: Utc::now(),
                    content: String::new(),
                    content_summary: String::new(),
                });
            }
        }
    }
    Ok(results)
}

async fn ddg_search(query: &str, max_results: usize) -> Result<Vec<WebSearchResult>, String> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );
    let html = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    Ok(parse_ddg_html(&html, max_results))
}

fn parse_ddg_html(html: &str, max_results: usize) -> Vec<WebSearchResult> {
    let mut results = Vec::new();
    let mut pos = 0usize;
    while let Some(anchor_rel) = html[pos..].find("result__a") {
        let anchor = pos + anchor_rel;
        let href_ix = match html[anchor..].find("href=\"") {
            Some(v) => anchor + v + 6,
            None => break,
        };
        let href_end = match html[href_ix..].find('"') {
            Some(v) => href_ix + v,
            None => break,
        };
        let url = normalize_ddg_url(&html[href_ix..href_end]);

        let title_start = match html[href_end..].find('>') {
            Some(v) => href_end + v + 1,
            None => break,
        };
        let title_end = match html[title_start..].find("</a>") {
            Some(v) => title_start + v,
            None => break,
        };
        let title = html_text(&html[title_start..title_end]);

        let block_end = html[title_end..]
            .find("result__a")
            .map(|v| title_end + v)
            .unwrap_or_else(|| html.len());
        let block = &html[title_end..block_end];
        let snippet = extract_ddg_snippet(block).unwrap_or_default();

        if is_http_url(&url) && !title.trim().is_empty() {
            results.push(WebSearchResult {
                title,
                url,
                snippet,
                fetched_at: Utc::now(),
                content: String::new(),
                content_summary: String::new(),
            });
        }
        if results.len() >= max_results {
            break;
        }
        pos = title_end;
    }
    results
}

fn extract_ddg_snippet(block: &str) -> Option<String> {
    let sn_ix = block.find("result__snippet")?;
    let gt = sn_ix + block[sn_ix..].find('>')? + 1;
    let end = gt + block[gt..].find("</")?;
    Some(html_text(&block[gt..end]))
}

fn normalize_ddg_url(url: &str) -> String {
    let decoded = html_text(url);
    if let Some(ix) = decoded.find("uddg=") {
        let raw = decoded[ix + 5..].split('&').next().unwrap_or("");
        return urlencoding::decode(raw)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| raw.to_string());
    }
    if decoded.starts_with("//") {
        format!("https:{}", decoded)
    } else {
        decoded
    }
}

fn html_text(input: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_search_query(raw: &str, domains: &[String], recency_days: Option<u32>) -> String {
    let mut q = raw.trim().to_string();
    for domain in domains {
        let domain = domain.trim().trim_start_matches("site:");
        if !domain.is_empty() {
            q.push_str(&format!(" site:{}", domain));
        }
    }
    if let Some(days) = recency_days {
        let after = Utc::now() - ChronoDuration::days(days as i64);
        q.push_str(&format!(" after:{}", after.format("%Y-%m-%d")));
    }
    q
}

fn bing_freshness(days: Option<u32>) -> Option<String> {
    match days {
        Some(0 | 1) => Some("Day".to_string()),
        Some(2..=7) => Some("Week".to_string()),
        Some(8..=31) => Some("Month".to_string()),
        _ => None,
    }
}

fn serpapi_recency(days: Option<u32>) -> Option<String> {
    match days {
        Some(0 | 1) => Some("qdr:d".to_string()),
        Some(2..=7) => Some("qdr:w".to_string()),
        Some(8..=31) => Some("qdr:m".to_string()),
        _ => None,
    }
}

fn summarize_content(content: &str, max_chars: usize) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    file_intelligence::truncate_chars(&normalized, max_chars)
}

fn default_fetch_pages() -> bool {
    true
}

fn has_env(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn research_source_uri(query: &str, context: &ResearchNotebookSaveContext) -> String {
    let slug = slugify(query);
    let ts = Utc::now().format("%Y%m%d%H%M%S%3f");
    if let Some(run_id) = context
        .origin_run_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return format!("research://run/{}/query/{}/{}", run_id, slug, ts);
    }
    if let Some(session_id) = context
        .origin_session_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return format!("research://session/{}/query/{}/{}", session_id, slug, ts);
    }
    format!("research://query/{}/{}", slug, ts)
}

fn slugify(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    let slug = slug.trim_matches('_');
    if slug.is_empty() {
        "research".to_string()
    } else {
        slug.chars().take(80).collect()
    }
}
