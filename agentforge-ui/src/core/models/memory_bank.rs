use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryBankCategory {
    ProjectBrief,
    SystemArchitecture,
    ActiveContext,
    Progress,
    LessonsLearned,
    DecisionLog,
}

impl MemoryBankCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProjectBrief => "project_brief",
            Self::SystemArchitecture => "system_architecture",
            Self::ActiveContext => "active_context",
            Self::Progress => "progress",
            Self::LessonsLearned => "lessons_learned",
            Self::DecisionLog => "decision_log",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "project_brief" => Self::ProjectBrief,
            "system_architecture" => Self::SystemArchitecture,
            "active_context" => Self::ActiveContext,
            "progress" => Self::Progress,
            "lessons_learned" => Self::LessonsLearned,
            "decision_log" => Self::DecisionLog,
            _ => Self::ActiveContext,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ProjectBrief => "Project Brief",
            Self::SystemArchitecture => "System Architecture",
            Self::ActiveContext => "Active Context",
            Self::Progress => "Progress",
            Self::LessonsLearned => "Lessons Learned",
            Self::DecisionLog => "Decision Log",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::ProjectBrief,
            Self::SystemArchitecture,
            Self::ActiveContext,
            Self::Progress,
            Self::LessonsLearned,
            Self::DecisionLog,
        ]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryBankStatus {
    Active,
    Archived,
    Stale,
}

impl MemoryBankStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
            Self::Stale => "stale",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "archived" => Self::Archived,
            "stale" => Self::Stale,
            _ => Self::Active,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl ContextPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "critical" => Self::Critical,
            "high" => Self::High,
            "medium" => Self::Medium,
            "low" => Self::Low,
            _ => Self::Medium,
        }
    }

    pub fn weight(&self) -> u32 {
        match self {
            Self::Critical => 100,
            Self::High => 75,
            Self::Medium => 50,
            Self::Low => 25,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBankItem {
    pub id: Uuid,
    pub instance_id: String,
    pub category: MemoryBankCategory,
    pub title: String,
    pub content: String,
    pub status: MemoryBankStatus,
    pub context_priority: ContextPriority,
    pub version: i64,
    pub parent_id: Option<Uuid>,
    pub position_x: f32,
    pub position_y: f32,
    pub color_hex: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub content_hash: String,
    pub token_count: usize,
}

impl MemoryBankItem {
    pub fn new(
        instance_id: &str,
        category: MemoryBankCategory,
        title: &str,
        content: &str,
        created_by: &str,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            instance_id: instance_id.to_string(),
            category,
            title: title.to_string(),
            content: content.to_string(),
            status: MemoryBankStatus::Active,
            context_priority: Self::default_priority(category),
            version: 1,
            parent_id: None,
            position_x: 0.0,
            position_y: 0.0,
            color_hex: None,
            created_at: now,
            updated_at: now,
            created_by: created_by.to_string(),
            content_hash: Self::compute_hash(content),
            token_count: Self::estimate_tokens(content),
        }
    }

    fn default_priority(category: MemoryBankCategory) -> ContextPriority {
        match category {
            MemoryBankCategory::ProjectBrief => ContextPriority::Critical,
            MemoryBankCategory::ActiveContext => ContextPriority::Critical,
            MemoryBankCategory::SystemArchitecture => ContextPriority::High,
            MemoryBankCategory::Progress => ContextPriority::High,
            MemoryBankCategory::LessonsLearned => ContextPriority::Medium,
            MemoryBankCategory::DecisionLog => ContextPriority::Medium,
        }
    }

    pub fn compute_hash(content: &str) -> String {
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in content.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("fnv1a64:{hash:016x}")
    }

    pub fn estimate_tokens(content: &str) -> usize {
        if content.is_empty() {
            return 0;
        }
        // ~4 chars per token heuristic
        content.len().div_ceil(4).max(1)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryBankLinkType {
    DependsOn,
    RelatesTo,
    Supersedes,
    DerivedFrom,
}

impl MemoryBankLinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DependsOn => "depends_on",
            Self::RelatesTo => "relates_to",
            Self::Supersedes => "supersedes",
            Self::DerivedFrom => "derived_from",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "depends_on" => Self::DependsOn,
            "relates_to" => Self::RelatesTo,
            "supersedes" => Self::Supersedes,
            "derived_from" => Self::DerivedFrom,
            _ => Self::RelatesTo,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBankLink {
    pub id: Uuid,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub link_type: MemoryBankLinkType,
    pub label: Option<String>,
    pub instance_id: String,
}

impl MemoryBankLink {
    pub fn new(
        source_id: Uuid,
        target_id: Uuid,
        link_type: MemoryBankLinkType,
        label: Option<String>,
        instance_id: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_id,
            target_id,
            link_type,
            label,
            instance_id: instance_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBankSnapshot {
    pub id: Uuid,
    pub instance_id: String,
    pub summary: String,
    pub items_json: String,
    pub created_at: DateTime<Utc>,
    pub consolidated_to_knowledge_id: Option<Uuid>,
}
