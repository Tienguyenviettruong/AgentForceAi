use super::framework::{Skill, SkillInput, SkillMetadata, SkillOutput};
use async_trait::async_trait;

pub struct WebSearchSkill;

#[async_trait]
impl Skill for WebSearchSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "research.web_search".to_string(),
            name: "Web Search".to_string(),
            description: "Searches the web for up-to-date information using DuckDuckGo.".to_string(),
            version: "2.0".to_string(),
            category: "Research".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let query = input.parameters.get("query").cloned().unwrap_or_default();
        if query.is_empty() {
            return SkillOutput {
                result: String::new(),
                success: false,
                error_message: Some("No query provided".to_string()),
            };
        }

        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(&query));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        match client.get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await {
            Ok(resp) => {
                if let Ok(text) = resp.text().await {
                    let mut results = Vec::new();
                    let mut result_num = 0;

                    for segment in text.split("result__snippet") {
                        if result_num > 0 && result_num <= 5 {
                            let mut in_tag = false;
                            let mut clean = String::new();
                            for c in segment.chars().take(500) {
                                if c == '<' { in_tag = true; continue; }
                                if c == '>' { in_tag = false; continue; }
                                if !in_tag { clean.push(c); }
                            }
                            let clean = clean.trim().to_string();
                            if !clean.is_empty() && clean.len() > 20 {
                                results.push(format!("{}. {}", result_num, clean));
                            }
                        }
                        result_num += 1;
                    }

                    if results.is_empty() {
                        // Fallback: raw strip
                        let mut in_tag = false;
                        let mut stripped = String::new();
                        for c in text.chars() {
                            if c == '<' { in_tag = true; continue; }
                            if c == '>' { in_tag = false; stripped.push(' '); continue; }
                            if !in_tag { stripped.push(c); }
                        }
                        let truncated: String = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
                        let limit = std::cmp::min(3000, truncated.len());
                        return SkillOutput {
                            result: format!("Search results for '{}':\n{}", query, &truncated[..limit]),
                            success: true,
                            error_message: None,
                        };
                    }

                    SkillOutput {
                        result: format!("Search results for '{}':\n{}", query, results.join("\n\n")),
                        success: true,
                        error_message: None,
                    }
                } else {
                    SkillOutput {
                        result: String::new(),
                        success: false,
                        error_message: Some("Failed to read response body".to_string()),
                    }
                }
            }
            Err(e) => {
                SkillOutput {
                    result: String::new(),
                    success: false,
                    error_message: Some(format!("Web search failed: {}", e)),
                }
            }
        }
    }
}

pub struct DocumentAnalysisSkill;

#[async_trait]
impl Skill for DocumentAnalysisSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "research.document_analysis".to_string(),
            name: "Document Analysis".to_string(),
            description: "Analyzes documents and extracts key information including structure, word count, and topics.".to_string(),
            version: "2.0".to_string(),
            category: "Research".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let content = input.parameters.get("content").cloned().unwrap_or_default();
        if content.is_empty() {
            return SkillOutput {
                result: String::new(),
                success: false,
                error_message: Some("No content provided".to_string()),
            };
        }

        let word_count = content.split_whitespace().count();
        let line_count = content.lines().count();
        let char_count = content.chars().count();

        // Extract headings (lines starting with # in markdown)
        let headings: Vec<&str> = content
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .collect();

        // Extract code blocks
        let code_block_count = content.matches("```").count() / 2;

        // Extract links
        let link_count = content.matches("](").count();

        let result = format!(
            "Document Analysis:\n- Words: {}\n- Lines: {}\n- Characters: {}\n- Headings: {}\n- Code blocks: {}\n- Links: {}\n{}",
            word_count,
            line_count,
            char_count,
            headings.len(),
            code_block_count,
            link_count,
            if !headings.is_empty() {
                format!("\nDocument Structure:\n{}", headings.iter().map(|h| format!("  {}", h)).collect::<Vec<_>>().join("\n"))
            } else {
                String::new()
            }
        );

        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}

pub struct DataExtractionSkill;

#[async_trait]
impl Skill for DataExtractionSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "research.data_extraction".to_string(),
            name: "Data Extraction".to_string(),
            description: "Extracts structured data from unstructured text including emails, URLs, dates, and numbers.".to_string(),
            version: "2.0".to_string(),
            category: "Research".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let text = input.parameters.get("text").cloned().unwrap_or_default();
        if text.is_empty() {
            return SkillOutput {
                result: String::new(),
                success: false,
                error_message: Some("No text provided".to_string()),
            };
        }

        // Extract URLs
        let urls: Vec<&str> = text
            .split_whitespace()
            .filter(|w| w.starts_with("http://") || w.starts_with("https://"))
            .collect();

        // Extract potential email patterns (simple heuristic)
        let emails: Vec<&str> = text
            .split_whitespace()
            .filter(|w| w.contains('@') && w.contains('.'))
            .collect();

        // Extract numbers
        let numbers: Vec<&str> = text
            .split_whitespace()
            .filter(|w| w.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ','))
            .filter(|w| !w.is_empty() && w.chars().any(|c| c.is_ascii_digit()))
            .collect();

        let result = format!(
            "Data Extraction Results:\n- URLs found: {} {}\n- Emails found: {} {}\n- Numbers found: {} {}\n- Total text length: {} chars",
            urls.len(),
            if urls.is_empty() { String::new() } else { format!("({})", urls.join(", ")) },
            emails.len(),
            if emails.is_empty() { String::new() } else { format!("({})", emails.join(", ")) },
            numbers.len(),
            if numbers.is_empty() { String::new() } else { format!("(first 10: {})", numbers.iter().take(10).cloned().collect::<Vec<_>>().join(", ")) },
            text.len()
        );

        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}
