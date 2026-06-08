use crate::core::models::Agent;
use std::collections::HashSet;

pub fn normalize_route_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

pub fn is_coordinator_role(role: &str) -> bool {
    normalize_route_key(role) == "coordinator"
}

pub fn tool_allowed_for_role(role: &str, tool_name: &str) -> bool {
    if !is_coordinator_role(role) {
        return true;
    }

    !matches!(
        tool_name,
        "write_file"
            | "edit_file"
            | "delete_file"
            | "run_cli"
            | "render_document"
            | "render_pdf"
            | "generate_image"
            | "generate_video"
    )
}

pub fn tool_denied_message(role: &str, tool_name: &str) -> String {
    format!(
        "Tool denied: role '{}' is read/coordination only for artifact-changing tool '{}'. Assign this work to a non-Coordinator agent with matching competency.",
        role, tool_name
    )
}

pub fn filter_tools_json_for_role(tools_json: &mut serde_json::Value, role: &str) {
    let Some(tools) = tools_json
        .get_mut("tools")
        .and_then(|value| value.as_array_mut())
    else {
        return;
    };
    tools.retain(|tool| {
        tool.get("name")
            .and_then(|value| value.as_str())
            .map(|name| tool_allowed_for_role(role, name))
            .unwrap_or(true)
    });
}

pub fn assignment_keys_for_agent(agent: &Agent) -> Vec<String> {
    let mut keys = Vec::new();
    for key in [
        agent.name.clone(),
        agent.routing_role(),
        agent.profile_position(),
    ] {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !keys.iter().any(|existing| existing == trimmed) {
            keys.push(trimmed.to_string());
        }
    }
    keys
}

pub fn canonical_task_type(task_type: &str, title: &str, description: &str) -> String {
    let explicit = normalize_route_key(task_type);
    if let Some(kind) = canonical_from_key(&explicit) {
        return kind.to_string();
    }

    let text = normalize_route_key(&format!("{} {}", title, description));
    for kind in TASK_KINDS {
        if aliases_for(kind)
            .iter()
            .any(|alias| !alias.is_empty() && text.contains(alias))
        {
            return (*kind).to_string();
        }
    }
    if explicit.is_empty() {
        "general".to_string()
    } else {
        explicit
    }
}

pub fn task_requires_artifact_change(task_type: &str, title: &str, description: &str) -> bool {
    let kind = canonical_task_type(task_type, title, description);
    if matches!(
        kind.as_str(),
        "implementation"
            | "testing"
            | "build"
            | "documentation"
            | "content"
            | "design"
            | "marketing"
            | "operations"
    ) {
        return true;
    }

    let text = format!("{} {}", title, description).to_lowercase();
    [
        "write_file",
        "edit_file",
        "delete_file",
        "run_cli",
        "create file",
        "write file",
        "edit file",
        "modify file",
        "delete file",
        "create directory",
        "mkdir",
        "tao file",
        "ghi file",
        "sua file",
        "xoa file",
        "tao thu muc",
        "create doc",
        "write doc",
        "write spec",
        "save draft",
        "render",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

pub fn validate_task_assignment(
    role: &str,
    task_type: &str,
    title: &str,
    description: &str,
) -> Result<(), String> {
    if is_coordinator_role(role) && task_requires_artifact_change(task_type, title, description) {
        return Err(format!(
            "Role '{}' cannot be assigned artifact-changing work. Keep Coordinator for planning/delegation/review/read-only work and assign file creation or modification to another agent with matching competency.",
            role
        ));
    }
    Ok(())
}

pub fn validate_agent_task_assignment(
    agent: &Agent,
    task_type: &str,
    title: &str,
    description: &str,
) -> Result<(), String> {
    let role = agent.routing_role();
    validate_task_assignment(&role, task_type, title, description)?;

    let kind = canonical_task_type(task_type, title, description);
    if kind == "general" {
        return Ok(());
    }

    let disallowed = normalized_set(agent.disallowed_task_types());
    if disallowed.contains("all") || set_matches_kind(&disallowed, &kind) {
        return Err(format!(
            "Agent '{}' ({}) explicitly disallows task type '{}'. Use another agent or hand off to a team with matching competency.",
            agent.name, role, kind
        ));
    }

    let allowed = normalized_set(agent.allowed_task_types());
    if !allowed.is_empty() && !allowed.contains("all") && !set_matches_kind(&allowed, &kind) {
        return Err(format!(
            "Agent '{}' ({}) is not allowed to take task type '{}'. Allowed task types: {}.",
            agent.name,
            role,
            kind,
            display_set(&allowed)
        ));
    }

    let profile = inferred_agent_competencies(agent);
    if !allowed.is_empty() || !profile.is_empty() {
        if !set_matches_kind(&allowed, &kind) && !set_matches_kind(&profile, &kind) {
            return Err(format!(
                "Agent '{}' ({}) does not have competency for task type '{}'. Competencies inferred/configured: {}. Hand off to a team/agent with matching competency if this instance lacks one.",
                agent.name,
                role,
                kind,
                display_set(&profile)
            ));
        }
    }

    Ok(())
}

pub fn capable_agent_labels_for_task(
    agents: &[Agent],
    task_type: &str,
    title: &str,
    description: &str,
) -> Vec<String> {
    agents
        .iter()
        .filter(|agent| {
            validate_agent_task_assignment(agent, task_type, title, description).is_ok()
        })
        .map(|agent| format!("{} ({})", agent.name, agent.routing_role()))
        .collect()
}

pub fn agent_profile_for_prompt(agent: &Agent) -> String {
    let inferred = inferred_agent_competencies(agent);
    let inferred = if inferred.is_empty() {
        "not inferred".to_string()
    } else {
        display_set(&inferred)
    };
    format!(
        "{} | inferred_task_competencies={}",
        agent.profile_for_prompt(),
        inferred
    )
}

fn canonical_from_key(key: &str) -> Option<&'static str> {
    TASK_KINDS
        .iter()
        .copied()
        .find(|kind| aliases_for(kind).contains(&key))
}

const TASK_KINDS: &[&str] = &[
    "planning",
    "product",
    "analysis",
    "research",
    "documentation",
    "implementation",
    "testing",
    "build",
    "architecture",
    "design",
    "content",
    "marketing",
    "operations",
    "review",
    "handoff",
];

fn aliases_for(kind: &str) -> &'static [&'static str] {
    match kind {
        "planning" => &[
            "planning",
            "plan",
            "roadmap",
            "coordination",
            "coordinate",
            "strategy",
        ],
        "product" => &[
            "product",
            "productmanagement",
            "pm",
            "productowner",
            "prd",
            "scope",
        ],
        "analysis" => &[
            "analysis",
            "analyze",
            "analyst",
            "ba",
            "businessanalysis",
            "requirement",
            "requirements",
            "domain",
            "metric",
            "metrics",
        ],
        "research" => &[
            "research",
            "websearch",
            "benchmark",
            "survey",
            "marketresearch",
        ],
        "documentation" => &[
            "documentation",
            "document",
            "docs",
            "doc",
            "spec",
            "readme",
            "guide",
            "manual",
            "apireference",
        ],
        "implementation" => &[
            "implementation",
            "implement",
            "coding",
            "code",
            "sourcecode",
            "development",
            "develop",
            "developer",
            "dev",
            "engineer",
            "backend",
            "frontend",
            "fullstack",
            "programmer",
            "handler",
            "repository",
            "service",
            "model",
            "golang",
            "go",
            "python",
            "typescript",
            "javascript",
            "java",
            "rust",
        ],
        "testing" => &[
            "testing",
            "test",
            "tests",
            "qa",
            "qualityassurance",
            "unittest",
            "integrationtest",
            "e2e",
        ],
        "build" => &[
            "build", "runcli", "cli", "compile", "package", "deploy", "cicd", "devops",
        ],
        "architecture" => &[
            "architecture",
            "architect",
            "systemdesign",
            "technicaldesign",
        ],
        "design" => &[
            "design",
            "designer",
            "ux",
            "ui",
            "visual",
            "prototype",
            "wireframe",
            "mockup",
            "brand",
            "creative",
        ],
        "content" => &[
            "content",
            "contentcreator",
            "creator",
            "copywriting",
            "copywriter",
            "writer",
            "editorial",
            "script",
            "post",
            "socialmedia",
        ],
        "marketing" => &[
            "marketing",
            "marketer",
            "campaign",
            "growth",
            "seo",
            "ads",
            "advertising",
            "positioning",
            "funnel",
        ],
        "operations" => &[
            "operations",
            "ops",
            "sre",
            "infra",
            "infrastructure",
            "monitoring",
            "support",
            "admin",
        ],
        "review" => &[
            "review", "critic", "audit", "approve", "approval", "feedback", "qa",
        ],
        "handoff" => &[
            "handoff",
            "delegate",
            "delegation",
            "escalation",
            "transfer",
        ],
        _ => &[],
    }
}

fn normalized_set(values: Vec<String>) -> HashSet<String> {
    values
        .into_iter()
        .map(|value| normalize_route_key(&value))
        .filter(|value| !value.is_empty())
        .map(|value| {
            canonical_from_key(&value)
                .or_else(|| {
                    TASK_KINDS.iter().copied().find(|kind| {
                        aliases_for(kind)
                            .iter()
                            .any(|alias| !alias.is_empty() && value.contains(alias))
                    })
                })
                .unwrap_or(value.as_str())
                .to_string()
        })
        .collect()
}

fn inferred_agent_competencies(agent: &Agent) -> HashSet<String> {
    let mut set = HashSet::new();
    let explicit = normalized_set(
        agent
            .competencies()
            .into_iter()
            .chain(agent.responsibilities())
            .collect(),
    );
    set.extend(explicit);

    let profile_text = normalize_route_key(&format!(
        "{} {} {} {}",
        agent.name,
        agent.routing_role(),
        agent.profile_position(),
        agent.profile_details().unwrap_or_default()
    ));

    for kind in TASK_KINDS {
        if aliases_for(kind)
            .iter()
            .any(|alias| !alias.is_empty() && profile_text.contains(alias))
        {
            set.insert((*kind).to_string());
        }
    }

    if is_coordinator_role(&agent.routing_role()) {
        set.insert("planning".to_string());
        set.insert("review".to_string());
        set.insert("handoff".to_string());
        set.remove("implementation");
        set.remove("testing");
        set.remove("build");
    }

    set
}

fn set_matches_kind(set: &HashSet<String>, kind: &str) -> bool {
    set.contains(kind) || aliases_for(kind).iter().any(|alias| set.contains(*alias))
}

fn display_set(set: &HashSet<String>) -> String {
    if set.is_empty() {
        return "none".to_string();
    }
    let mut values: Vec<String> = set.iter().cloned().collect();
    values.sort();
    values.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str, config: Option<&str>) -> Agent {
        Agent {
            id: name.to_string(),
            name: name.to_string(),
            provider: "test".to_string(),
            system_prompt: None,
            config: config.map(str::to_string),
            status: "online".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn coordinator_cannot_use_artifact_changing_tools() {
        assert!(!tool_allowed_for_role("Coordinator", "write_file"));
        assert!(!tool_allowed_for_role("Coordinator", "edit_file"));
        assert!(!tool_allowed_for_role("Coordinator", "run_cli"));
        assert!(tool_allowed_for_role("Coordinator", "read_file"));
        assert!(tool_allowed_for_role("Coordinator", "create_subtasks"));
    }

    #[test]
    fn non_coordinator_agents_keep_artifact_tools_but_not_all_task_types() {
        for role in ["PM", "BA", "Developer", "Designer", "Content Creator"] {
            assert!(tool_allowed_for_role(role, "write_file"));
            assert!(tool_allowed_for_role(role, "edit_file"));
        }

        assert!(validate_agent_task_assignment(
            &agent("BA", Some(r#"{"role":"BA"}"#)),
            "implementation",
            "Implement handlers",
            "Write backend handlers"
        )
        .is_err());
        assert!(validate_agent_task_assignment(
            &agent("Content Creator", Some(r#"{"role":"Content Creator"}"#)),
            "content",
            "Write launch post",
            "Create social media copy"
        )
        .is_ok());
    }

    #[test]
    fn config_allowed_task_types_override_generic_role() {
        let custom = agent(
            "Custom Agent",
            Some(
                r#"{"role":"Custom","allowed_task_types":["design"],"competencies":["brand design"]}"#,
            ),
        );
        assert!(validate_agent_task_assignment(
            &custom,
            "design",
            "Create brand mockup",
            "Design hero visual"
        )
        .is_ok());
        assert!(validate_agent_task_assignment(
            &custom,
            "marketing",
            "Create campaign",
            "Prepare ad funnel"
        )
        .is_err());
    }

    #[test]
    fn coordinator_cannot_receive_document_creation_task() {
        assert!(validate_task_assignment(
            "Coordinator",
            "documentation",
            "Create SPEC.md",
            "Write a project specification"
        )
        .is_err());

        assert!(validate_task_assignment(
            "Coordinator",
            "planning",
            "Plan backend work",
            "Read the request and create subtasks"
        )
        .is_ok());
    }
}
