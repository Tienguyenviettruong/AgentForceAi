use crate::core::models::{
    Agent, Instance, Provider, ProviderTemplate, SessionRecord, Team, WorkflowRecord,
};
use crate::core::traits::database::DatabasePort;
use crate::knowledge::core::KnowledgeItem;
use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

fn normalize_permission_list(value: Option<&str>) -> Option<String> {
    value.map(|raw| {
        if raw.trim() == "all" {
            "[\"all\"]".to_string()
        } else if serde_json::from_str::<Vec<String>>(raw).is_ok() {
            raw.to_string()
        } else {
            "[]".to_string()
        }
    })
}

fn normalize_route_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn routing_role_from_agent_row(name: &str, config: Option<&str>) -> String {
    config
        .and_then(|config| serde_json::from_str::<serde_json::Value>(config).ok())
        .and_then(|value| {
            value
                .get("role")
                .and_then(|role| role.as_str())
                .map(str::trim)
                .filter(|role| !role.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| name.to_string())
}

fn agent_routing_role(conn: &Connection, agent_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT name, config FROM agents WHERE id = ?1",
            params![agent_id],
            |row| {
                let name: String = row.get(0)?;
                let config: Option<String> = row.get(1)?;
                Ok(routing_role_from_agent_row(&name, config.as_deref()))
            },
        )
        .optional()?)
}

fn sort_agent_ids_by_routing_role(conn: &Connection, agents: &mut Vec<String>) {
    agents.sort_by_key(|agent_id| {
        let role = agent_routing_role(conn, agent_id)
            .ok()
            .flatten()
            .unwrap_or_default();
        (
            normalize_route_key(&role) != "coordinator",
            normalize_route_key(&role),
            agent_id.clone(),
        )
    });
}

fn has_coordinator_agent(conn: &Connection, agents: &[String]) -> bool {
    agents.iter().any(|agent_id| {
        agent_routing_role(conn, agent_id)
            .ok()
            .flatten()
            .map(|role| normalize_route_key(&role) == "coordinator")
            .unwrap_or(false)
    })
}

fn find_team_coordinator_agent(conn: &Connection, team_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.name, a.config
         FROM members m
         JOIN agents a ON a.id = m.agent_id
         WHERE m.team_id = ?1
         ORDER BY m.joined_at ASC",
    )?;
    let rows = stmt.query_map(params![team_id], |row| {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let config: Option<String> = row.get(2)?;
        Ok((id, routing_role_from_agent_row(&name, config.as_deref())))
    })?;

    for row in rows {
        let (id, role) = row?;
        if normalize_route_key(&role) == "coordinator" {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

fn record_task_run_event(
    conn: &Connection,
    task_id: &str,
    event_type: &str,
    actor_id: Option<&str>,
    payload: Option<&str>,
) -> Result<()> {
    let run_id: Option<String> = conn
        .query_row(
            "SELECT run_id FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    if let Some(run_id) = run_id {
        conn.execute(
            "INSERT INTO run_events (id, run_id, event_type, actor_type, actor_id, task_id, payload, created_at)
             VALUES (?1, ?2, ?3, 'agent', ?4, ?5, ?6, ?7)",
            params![
                uuid::Uuid::new_v4().to_string(),
                run_id,
                event_type,
                actor_id,
                task_id,
                payload,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
    }
    Ok(())
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path =
            std::env::var("AGENTFORGE_DB_PATH").unwrap_or_else(|_| "agentforge.db".to_string());
        let mut conn = Connection::open(&db_path)?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA foreign_keys=ON;

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL
            );

            -- 1. Provider Configs
            CREATE TABLE IF NOT EXISTS provider_configs (
                id TEXT PRIMARY KEY,
                provider_name TEXT NOT NULL,
                model TEXT NOT NULL,
                adapter_type TEXT NOT NULL,
                command TEXT,
                node_version TEXT,
                config TEXT,
                api_key_ref TEXT,
                status TEXT NOT NULL DEFAULT 'available',
                is_builtin INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- 2. Teams
            CREATE TABLE IF NOT EXISTS teams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                objectives TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- 3. Roles
            CREATE TABLE IF NOT EXISTS roles (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                permissions TEXT,
                capabilities TEXT
            );

            -- Phase 4: security principals are separate from agent routing roles
            CREATE TABLE IF NOT EXISTS security_actors (
                id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                actor_kind TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS security_roles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS security_permissions (
                id TEXT PRIMARY KEY,
                code TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS security_actor_roles (
                actor_id TEXT NOT NULL REFERENCES security_actors(id) ON DELETE CASCADE,
                role_id TEXT NOT NULL REFERENCES security_roles(id) ON DELETE CASCADE,
                PRIMARY KEY(actor_id, role_id)
            );

            CREATE TABLE IF NOT EXISTS security_role_permissions (
                role_id TEXT NOT NULL REFERENCES security_roles(id) ON DELETE CASCADE,
                permission_id TEXT NOT NULL REFERENCES security_permissions(id) ON DELETE CASCADE,
                PRIMARY KEY(role_id, permission_id)
            );

            -- 4. Instances
            CREATE TABLE IF NOT EXISTS instances (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL DEFAULT 'Untitled',
                team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                config TEXT,
                state TEXT,
                created_at TEXT NOT NULL
            );

            -- 5. Agents
            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                provider TEXT NOT NULL,
                system_prompt TEXT,
                config TEXT,
                status TEXT NOT NULL DEFAULT 'offline',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- 6. Members
            CREATE TABLE IF NOT EXISTS members (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                instance_id TEXT REFERENCES instances(id) ON DELETE SET NULL,
                agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
                role_id TEXT REFERENCES roles(id) ON DELETE SET NULL,
                joined_at TEXT NOT NULL
            );

            -- 7. Tasks
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE,
                assignee_id TEXT REFERENCES agents(id) ON DELETE SET NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                priority TEXT NOT NULL DEFAULT 'medium',
                payload TEXT,
                claimed_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- 8. Messages
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE,
                sender_id TEXT NOT NULL,
                recipient_id TEXT,
                type TEXT NOT NULL,
                content TEXT NOT NULL,
                sent_at TEXT NOT NULL
            );

            -- 9. Team Messages
            CREATE TABLE IF NOT EXISTS team_messages (
                id TEXT PRIMARY KEY,
                team_instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
                sender_member_id TEXT NOT NULL,
                recipient_member_id TEXT,
                recipient_role TEXT,
                message_type TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                delivery_status TEXT NOT NULL DEFAULT 'delivered',
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_instance_time ON team_messages(team_instance_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_messages_sender ON team_messages(sender_member_id);
            CREATE INDEX IF NOT EXISTS idx_messages_recipient ON team_messages(recipient_member_id);

            -- 10. Sessions
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
                team_instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE,
                user_id TEXT,
                context TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- 11. Conversations
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_conversations_session_time ON conversations(session_id, created_at);

            -- 12. Workflows (iFlows)
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                definition TEXT NOT NULL,
                version TEXT NOT NULL,
                origin_kind TEXT NOT NULL DEFAULT 'iflow',
                activation_status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Workflow States
            CREATE TABLE IF NOT EXISTS workflow_states (
                execution_id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                state_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- 13. Knowledge
            CREATE TABLE IF NOT EXISTS knowledge (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT,
                category TEXT,
                vault_path TEXT,
                source_kind TEXT NOT NULL DEFAULT 'manual',
                source_uri_normalized TEXT,
                content_hash TEXT,
                origin_run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL,
                origin_session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
                id UNINDEXED,
                title,
                content,
                tags
            );

            -- Knowledge Chunks for Vector Embeddings
            CREATE TABLE IF NOT EXISTS knowledge_chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                embedding TEXT,
                FOREIGN KEY(document_id) REFERENCES knowledge(id) ON DELETE CASCADE
            );

            -- 14. App Settings
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- 15. Audit Log
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                user_id TEXT,
                action TEXT NOT NULL,
                resource TEXT NOT NULL,
                details TEXT
            );

            -- 16. Provider Templates
            CREATE TABLE IF NOT EXISTS provider_templates (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL UNIQUE,
                protocol TEXT NOT NULL,
                adapter TEXT NOT NULL,
                models TEXT NOT NULL,
                default_base_url TEXT NOT NULL
            );

            -- 17. Token Usage
            CREATE TABLE IF NOT EXISTS token_usage (
                id TEXT PRIMARY KEY,
                instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE,
                agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS mcp_tools (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL,
                version TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT NOT NULL,
                input_schema TEXT NOT NULL,
                is_active BOOLEAN DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- Phase 5: installed MCP server lifecycle and scoped model context
            CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                transport TEXT NOT NULL,
                command TEXT,
                args TEXT NOT NULL DEFAULT '[]',
                endpoint TEXT,
                env_secret_refs TEXT NOT NULL DEFAULT '{}',
                header_secret_refs TEXT NOT NULL DEFAULT '{}',
                source_kind TEXT NOT NULL DEFAULT 'manual',
                is_enabled BOOLEAN NOT NULL DEFAULT 1,
                health_status TEXT NOT NULL DEFAULT 'not_checked',
                last_error TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS capability_selections (
                id TEXT PRIMARY KEY,
                scope_kind TEXT NOT NULL,
                scope_id TEXT NOT NULL DEFAULT '',
                capability_kind TEXT NOT NULL,
                capability_id TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                selected_by TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(scope_kind, scope_id, capability_kind, capability_id)
            );

            CREATE TABLE IF NOT EXISTS llm_context_snapshots (
                id TEXT PRIMARY KEY,
                run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL,
                session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
                instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
                agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
                mode TEXT,
                selected_capabilities_json TEXT NOT NULL,
                context_hash TEXT NOT NULL,
                character_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS llm_context_sources (
                id TEXT PRIMARY KEY,
                snapshot_id TEXT NOT NULL REFERENCES llm_context_snapshots(id) ON DELETE CASCADE,
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                rank INTEGER,
                character_count INTEGER NOT NULL DEFAULT 0,
                trust_level TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS task_dependencies (
                task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                depends_on_task_id TEXT NOT NULL,
                PRIMARY KEY (task_id, depends_on_task_id)
            );

            CREATE TABLE IF NOT EXISTS artifacts (
                id TEXT PRIMARY KEY,
                run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL,
                session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
                instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
                agent_id TEXT REFERENCES agents(id) ON DELETE SET NULL,
                invocation_id TEXT,
                artifact_kind TEXT NOT NULL,
                path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS knowledge_entries (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                session_id TEXT,
                instance_id TEXT REFERENCES instances(id) ON DELETE SET NULL,
                run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(agent_id) REFERENCES agents(id)
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_entries_fts
            USING fts5(id UNINDEXED, title, content, tags);

            CREATE TABLE IF NOT EXISTS knowledge_migration_audit (
                id TEXT PRIMARY KEY,
                migration_version INTEGER NOT NULL,
                source_uri_normalized TEXT NOT NULL,
                retained_id TEXT NOT NULL,
                removed_id TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cross_team_cases (
                correlation_id TEXT PRIMARY KEY,
                owner_instance_id TEXT NOT NULL,
                target_instance_id TEXT NOT NULL,
                latest_event_type TEXT NOT NULL,
                summary TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cross_team_case_events (
                id TEXT PRIMARY KEY,
                correlation_id TEXT NOT NULL REFERENCES cross_team_cases(correlation_id) ON DELETE CASCADE,
                from_instance_id TEXT NOT NULL,
                reply_to_instance_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                summary TEXT NOT NULL,
                payload TEXT,
                created_at TEXT NOT NULL
            );

            -- Phase 7: governed human-like collaboration contract
            CREATE TABLE IF NOT EXISTS collaboration_cases (
                id TEXT PRIMARY KEY,
                legacy_correlation_id TEXT UNIQUE,
                origin_run_id TEXT,
                parent_case_id TEXT REFERENCES collaboration_cases(id) ON DELETE SET NULL,
                owner_instance_id TEXT NOT NULL,
                target_instance_id TEXT NOT NULL,
                owner_agent_id TEXT,
                state TEXT NOT NULL DEFAULT 'submitted',
                priority TEXT NOT NULL DEFAULT 'medium',
                risk_level TEXT NOT NULL DEFAULT 'medium',
                objective TEXT NOT NULL,
                acceptance_json TEXT NOT NULL DEFAULT '[]',
                constraints_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                resolved_at TEXT
            );

            CREATE TABLE IF NOT EXISTS handoff_packages (
                id TEXT PRIMARY KEY,
                case_id TEXT NOT NULL REFERENCES collaboration_cases(id) ON DELETE CASCADE,
                run_id TEXT,
                from_instance_id TEXT NOT NULL,
                to_instance_id TEXT NOT NULL,
                from_agent_id TEXT,
                to_agent_id TEXT,
                objective TEXT NOT NULL,
                acceptance_json TEXT NOT NULL DEFAULT '[]',
                constraints_json TEXT NOT NULL DEFAULT '{}',
                context_refs_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'submitted',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS case_readbacks (
                id TEXT PRIMARY KEY,
                case_id TEXT NOT NULL REFERENCES collaboration_cases(id) ON DELETE CASCADE,
                handoff_id TEXT NOT NULL REFERENCES handoff_packages(id) ON DELETE CASCADE,
                agent_id TEXT NOT NULL,
                understanding TEXT NOT NULL,
                assumptions_json TEXT NOT NULL DEFAULT '[]',
                questions_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'submitted',
                accepted_by TEXT,
                created_at TEXT NOT NULL,
                resolved_at TEXT
            );

            CREATE TABLE IF NOT EXISTS case_decisions (
                id TEXT PRIMARY KEY,
                case_id TEXT NOT NULL REFERENCES collaboration_cases(id) ON DELETE CASCADE,
                run_id TEXT,
                author_agent_id TEXT NOT NULL,
                decision TEXT NOT NULL,
                rationale TEXT NOT NULL,
                alternatives_json TEXT NOT NULL DEFAULT '[]',
                evidence_refs_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS case_deliverables (
                id TEXT PRIMARY KEY,
                case_id TEXT NOT NULL REFERENCES collaboration_cases(id) ON DELETE CASCADE,
                run_id TEXT,
                agent_id TEXT NOT NULL,
                title TEXT NOT NULL,
                artifact_refs_json TEXT NOT NULL DEFAULT '[]',
                acceptance_evidence_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'submitted',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS case_reviews (
                id TEXT PRIMARY KEY,
                case_id TEXT NOT NULL REFERENCES collaboration_cases(id) ON DELETE CASCADE,
                deliverable_id TEXT NOT NULL REFERENCES case_deliverables(id) ON DELETE CASCADE,
                reviewer_agent_id TEXT NOT NULL,
                verdict TEXT NOT NULL,
                findings_json TEXT NOT NULL DEFAULT '[]',
                required_actions_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS case_consensus_records (
                id TEXT PRIMARY KEY,
                case_id TEXT NOT NULL REFERENCES collaboration_cases(id) ON DELETE CASCADE,
                proposal TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'proposed',
                quorum_rule_json TEXT NOT NULL DEFAULT '{}',
                resolution TEXT,
                created_at TEXT NOT NULL,
                resolved_at TEXT
            );

            CREATE TABLE IF NOT EXISTS case_consensus_votes (
                id TEXT PRIMARY KEY,
                consensus_id TEXT NOT NULL REFERENCES case_consensus_records(id) ON DELETE CASCADE,
                voter_id TEXT NOT NULL,
                vote TEXT NOT NULL,
                rationale TEXT,
                created_at TEXT NOT NULL,
                UNIQUE(consensus_id, voter_id)
            );

            CREATE TABLE IF NOT EXISTS case_escalations (
                id TEXT PRIMARY KEY,
                case_id TEXT NOT NULL REFERENCES collaboration_cases(id) ON DELETE CASCADE,
                raised_by TEXT NOT NULL,
                reason TEXT NOT NULL,
                severity TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'open',
                resolved_by TEXT,
                resolution TEXT,
                created_at TEXT NOT NULL,
                resolved_at TEXT
            );

            CREATE TABLE IF NOT EXISTS delegated_grants (
                id TEXT PRIMARY KEY,
                case_id TEXT NOT NULL REFERENCES collaboration_cases(id) ON DELETE CASCADE,
                run_id TEXT,
                grantor_actor_id TEXT NOT NULL,
                grantee_agent_id TEXT NOT NULL,
                allowed_tools_json TEXT NOT NULL DEFAULT '[]',
                allowed_mcp_json TEXT NOT NULL DEFAULT '[]',
                workspace_scope_json TEXT NOT NULL DEFAULT '[]',
                token_limit INTEGER,
                cost_limit REAL,
                expires_at TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS agent_competencies (
                agent_id TEXT NOT NULL,
                competency_key TEXT NOT NULL,
                score REAL NOT NULL DEFAULT 0,
                evidence_count INTEGER NOT NULL DEFAULT 0,
                confidence REAL NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (agent_id, competency_key)
            );

            CREATE TABLE IF NOT EXISTS routing_decisions (
                id TEXT PRIMARY KEY,
                case_id TEXT NOT NULL REFERENCES collaboration_cases(id) ON DELETE CASCADE,
                selected_agent_id TEXT NOT NULL,
                competency_key TEXT NOT NULL,
                score_snapshot_json TEXT NOT NULL DEFAULT '{}',
                rationale TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            -- Phase 8: governed learning and evolution lifecycle
            CREATE TABLE IF NOT EXISTS evaluation_rubrics (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version INTEGER NOT NULL,
                criteria_json TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                UNIQUE(name, version)
            );

            CREATE TABLE IF NOT EXISTS run_evaluations (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                rubric_id TEXT REFERENCES evaluation_rubrics(id) ON DELETE SET NULL,
                evaluator_id TEXT NOT NULL,
                score REAL NOT NULL,
                verdict TEXT NOT NULL,
                evidence_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS feedback_records (
                id TEXT PRIMARY KEY,
                run_id TEXT,
                case_id TEXT REFERENCES collaboration_cases(id) ON DELETE SET NULL,
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                subject_kind TEXT NOT NULL,
                subject_id TEXT NOT NULL,
                content TEXT NOT NULL,
                validation_status TEXT NOT NULL DEFAULT 'quarantined',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS lessons (
                id TEXT PRIMARY KEY,
                source_evaluation_id TEXT REFERENCES run_evaluations(id) ON DELETE SET NULL,
                source_feedback_id TEXT REFERENCES feedback_records(id) ON DELETE SET NULL,
                scope_kind TEXT NOT NULL,
                scope_id TEXT NOT NULL DEFAULT '',
                instruction TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'candidate',
                validated_by TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS learning_candidates (
                id TEXT PRIMARY KEY,
                candidate_kind TEXT NOT NULL,
                source_lesson_id TEXT NOT NULL REFERENCES lessons(id) ON DELETE RESTRICT,
                target_id TEXT NOT NULL,
                baseline_version_id TEXT,
                proposed_definition_json TEXT NOT NULL,
                risk_level TEXT NOT NULL DEFAULT 'medium',
                status TEXT NOT NULL DEFAULT 'draft',
                created_by TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS skill_versions (
                id TEXT PRIMARY KEY,
                skill_id TEXT NOT NULL,
                candidate_id TEXT REFERENCES learning_candidates(id) ON DELETE SET NULL,
                version INTEGER NOT NULL,
                instructions TEXT NOT NULL,
                activation_status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL,
                UNIQUE(skill_id, version)
            );

            CREATE TABLE IF NOT EXISTS benchmark_suites (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                UNIQUE(name, version)
            );

            CREATE TABLE IF NOT EXISTS benchmark_cases (
                id TEXT PRIMARY KEY,
                suite_id TEXT NOT NULL REFERENCES benchmark_suites(id) ON DELETE CASCADE,
                input_json TEXT NOT NULL,
                expectation_json TEXT NOT NULL,
                risk_level TEXT NOT NULL DEFAULT 'medium',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS benchmark_runs (
                id TEXT PRIMARY KEY,
                candidate_id TEXT NOT NULL REFERENCES learning_candidates(id) ON DELETE CASCADE,
                suite_id TEXT NOT NULL,
                status TEXT NOT NULL,
                aggregate_score REAL,
                regression_count INTEGER NOT NULL DEFAULT 0,
                result_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS benchmark_results (
                id TEXT PRIMARY KEY,
                benchmark_run_id TEXT NOT NULL REFERENCES benchmark_runs(id) ON DELETE CASCADE,
                benchmark_case_id TEXT NOT NULL,
                score REAL NOT NULL,
                verdict TEXT NOT NULL,
                evidence_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS benchmark_runner_jobs (
                id TEXT PRIMARY KEY,
                candidate_id TEXT NOT NULL REFERENCES learning_candidates(id) ON DELETE CASCADE,
                suite_id TEXT NOT NULL,
                status TEXT NOT NULL,
                requested_by TEXT NOT NULL,
                benchmark_run_id TEXT REFERENCES benchmark_runs(id) ON DELETE SET NULL,
                error TEXT,
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS canary_deployments (
                id TEXT PRIMARY KEY,
                candidate_id TEXT NOT NULL REFERENCES learning_candidates(id) ON DELETE CASCADE,
                scope_json TEXT NOT NULL DEFAULT '{}',
                traffic_percent REAL NOT NULL,
                status TEXT NOT NULL DEFAULT 'running',
                started_at TEXT NOT NULL,
                ended_at TEXT
            );

            CREATE TABLE IF NOT EXISTS canary_observations (
                id TEXT PRIMARY KEY,
                deployment_id TEXT NOT NULL REFERENCES canary_deployments(id) ON DELETE CASCADE,
                run_id TEXT,
                metric_json TEXT NOT NULL,
                verdict TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS promotion_decisions (
                id TEXT PRIMARY KEY,
                candidate_id TEXT NOT NULL REFERENCES learning_candidates(id) ON DELETE CASCADE,
                decided_by TEXT NOT NULL,
                decision TEXT NOT NULL,
                rationale TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS rollback_records (
                id TEXT PRIMARY KEY,
                candidate_id TEXT NOT NULL REFERENCES learning_candidates(id) ON DELETE CASCADE,
                deployment_id TEXT REFERENCES canary_deployments(id) ON DELETE SET NULL,
                initiated_by TEXT NOT NULL,
                reason TEXT NOT NULL,
                restored_version_id TEXT,
                created_at TEXT NOT NULL
            );

            -- Phase 1: persistent orchestration execution spine
            CREATE TABLE IF NOT EXISTS orchestration_runs (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
                initiated_by TEXT,
                goal TEXT NOT NULL,
                mode TEXT NOT NULL,
                status TEXT NOT NULL,
                workflow_id TEXT REFERENCES workflows(id) ON DELETE SET NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS run_events (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL REFERENCES orchestration_runs(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                actor_type TEXT NOT NULL,
                actor_id TEXT,
                task_id TEXT,
                payload TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS approval_requests (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL REFERENCES orchestration_runs(id) ON DELETE CASCADE,
                operation TEXT NOT NULL,
                requested_by TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                resolved_by TEXT,
                decision_reason TEXT,
                created_at TEXT NOT NULL,
                resolved_at TEXT
            );

            CREATE TABLE IF NOT EXISTS tool_invocations (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL REFERENCES orchestration_runs(id) ON DELETE CASCADE,
                tool_name TEXT NOT NULL,
                sealed_payload_json TEXT NOT NULL,
                payload_hash TEXT NOT NULL,
                mode TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'sealed',
                approval_request_id TEXT REFERENCES approval_requests(id) ON DELETE SET NULL,
                result TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS mode_transitions (
                id TEXT PRIMARY KEY,
                instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE,
                run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL,
                actor_id TEXT,
                from_mode TEXT NOT NULL,
                to_mode TEXT NOT NULL,
                reason TEXT,
                policy_version TEXT,
                created_at TEXT NOT NULL
            );

            -- Phase 2: iFlow version and durable execution lifecycle
            CREATE TABLE IF NOT EXISTS workflow_versions (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
                run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL,
                instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
                version INTEGER NOT NULL,
                definition_json TEXT NOT NULL,
                validation_status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(workflow_id, version)
            );

            CREATE TABLE IF NOT EXISTS workflow_executions (
                id TEXT PRIMARY KEY,
                workflow_version_id TEXT NOT NULL REFERENCES workflow_versions(id) ON DELETE CASCADE,
                run_id TEXT NOT NULL REFERENCES orchestration_runs(id) ON DELETE CASCADE,
                status TEXT NOT NULL,
                state_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_runs_instance_time
                ON orchestration_runs(instance_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_runs_session_time
                ON orchestration_runs(session_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_run_events_run_time
                ON run_events(run_id, created_at ASC);
            CREATE INDEX IF NOT EXISTS idx_approvals_status_time
                ON approval_requests(status, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_tool_invocations_run_status
                ON tool_invocations(run_id, status, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_mode_transitions_time
                ON mode_transitions(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_workflow_versions_run_time
                ON workflow_versions(run_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_workflow_executions_version_time
                ON workflow_executions(workflow_version_id, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_mcp_server_enabled
                ON mcp_servers(is_enabled, name);
            CREATE INDEX IF NOT EXISTS idx_capability_selection_scope
                ON capability_selections(scope_kind, scope_id, capability_kind, enabled);
            CREATE INDEX IF NOT EXISTS idx_context_snapshots_run_time
                ON llm_context_snapshots(run_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_context_sources_snapshot
                ON llm_context_sources(snapshot_id);
            CREATE INDEX IF NOT EXISTS idx_task_dependencies_dependency
                ON task_dependencies(depends_on_task_id);
            CREATE INDEX IF NOT EXISTS idx_artifacts_run_time
                ON artifacts(run_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_collaboration_cases_state_time
                ON collaboration_cases(state, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_collaboration_cases_run
                ON collaboration_cases(origin_run_id);
            CREATE INDEX IF NOT EXISTS idx_case_escalations_status_time
                ON case_escalations(status, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_delegated_grants_agent_run
                ON delegated_grants(grantee_agent_id, run_id, status);
            CREATE INDEX IF NOT EXISTS idx_run_evaluations_run_time
                ON run_evaluations(run_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_lessons_status_scope
                ON lessons(status, scope_kind, scope_id);
            CREATE INDEX IF NOT EXISTS idx_learning_candidates_status_time
                ON learning_candidates(status, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_benchmark_cases_suite
                ON benchmark_cases(suite_id, created_at ASC);
            CREATE INDEX IF NOT EXISTS idx_benchmark_runner_jobs_candidate
                ON benchmark_runner_jobs(candidate_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_canary_candidate_time
                ON canary_deployments(candidate_id, started_at DESC);

            ",
        )?;

        conn.execute(
            "ALTER TABLE tasks ADD COLUMN instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE sessions ADD COLUMN team_instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE instances ADD COLUMN name TEXT NOT NULL DEFAULT 'Untitled'",
            [],
        )
        .ok();

        // Migration: thêm cột capabilities cho model capability routing
        conn.execute(
            "ALTER TABLE provider_configs ADD COLUMN capabilities TEXT",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE tasks ADD COLUMN run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE token_usage ADD COLUMN run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE workflows ADD COLUMN run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE workflows ADD COLUMN origin_kind TEXT NOT NULL DEFAULT 'iflow'",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE workflows ADD COLUMN activation_status TEXT NOT NULL DEFAULT 'draft'",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE knowledge ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'manual'",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE knowledge ADD COLUMN source_uri_normalized TEXT",
            [],
        )
        .ok();

        conn.execute("ALTER TABLE knowledge ADD COLUMN content_hash TEXT", [])
            .ok();

        conn.execute(
            "ALTER TABLE knowledge ADD COLUMN origin_run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE knowledge ADD COLUMN origin_session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE knowledge_entries ADD COLUMN instance_id TEXT REFERENCES instances(id) ON DELETE SET NULL",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE knowledge_entries ADD COLUMN run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL",
            [],
        )
        .ok();

        conn.execute(
            "ALTER TABLE mcp_tools ADD COLUMN server_id TEXT REFERENCES mcp_servers(id) ON DELETE SET NULL",
            [],
        )
        .ok();

        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, ?3)",
            params![
                2026052501_i64,
                "execution_spine",
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, ?3)",
            params![
                2026052701_i64,
                "artifact_and_dependency_provenance",
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, ?3)",
            params![
                2026052601_i64,
                "context_and_mcp_server_lifecycle",
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, ?3)",
            params![
                2026052502_i64,
                "iflow_lifecycle",
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        let phase3_applied: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![2026052503_i64],
            |row| row.get(0),
        )?;
        if phase3_applied == 0 {
            Self::migrate_knowledge_provenance(&mut conn)?;
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
                 VALUES (?1, ?2, ?3)",
                params![
                    2026052503_i64,
                    "knowledge_provenance",
                    chrono::Utc::now().to_rfc3339()
                ],
            )?;
        }

        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, ?3)",
            params![
                2026052504_i64,
                "governed_tool_gateway",
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, ?3)",
            params![
                2026052702_i64,
                "humanlike_collaboration_contract",
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, ?3)",
            params![
                2026052703_i64,
                "governed_learning_evolution",
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        let credential_migration_applied: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![2026052704_i64],
            |row| row.get(0),
        )?;
        if credential_migration_applied == 0 {
            Self::migrate_legacy_credentials(&mut conn)?;
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
                 VALUES (?1, ?2, ?3)",
                params![
                    2026052704_i64,
                    "legacy_credential_scrub",
                    chrono::Utc::now().to_rfc3339()
                ],
            )?;
        }

        let role_typo_migration_applied: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![2026060301_i64],
            |row| row.get(0),
        )?;
        if role_typo_migration_applied == 0 {
            Self::migrate_agent_routing_role_typos(&mut conn)?;
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
                 VALUES (?1, ?2, ?3)",
                params![
                    2026060301_i64,
                    "agent_routing_role_typo_normalization",
                    chrono::Utc::now().to_rfc3339()
                ],
            )?;
        }

        let provider_identity_migration_applied: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![2026060501_i64],
            |row| row.get(0),
        )?;
        if provider_identity_migration_applied == 0 {
            Self::migrate_provider_configs_to_id_identity(&mut conn)?;
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
                 VALUES (?1, ?2, ?3)",
                params![
                    2026060501_i64,
                    "provider_configs_id_identity",
                    chrono::Utc::now().to_rfc3339()
                ],
            )?;
        }

        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at)
             VALUES ('governance_max_tokens_per_run', '1000000', ?1)
             ON CONFLICT(key) DO UPDATE SET
                value = CASE
                    WHEN CAST(app_settings.value AS INTEGER) < 1000000 THEN '1000000'
                    ELSE app_settings.value
                END,
                updated_at = CASE
                    WHEN CAST(app_settings.value AS INTEGER) < 1000000 THEN excluded.updated_at
                    ELSE app_settings.updated_at
                END",
            params![now],
        )?;

        Self::backfill_task_dependencies(&conn)?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.seed_provider_templates().ok();
        db.ensure_knowledge_entries_fts().ok();
        Ok(db)
    }

    fn migrate_agent_routing_role_typos(conn: &mut Connection) -> Result<()> {
        let records: Vec<(String, String)> = {
            let mut stmt =
                conn.prepare("SELECT id, config FROM agents WHERE config IS NOT NULL")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };

        for (agent_id, config) in records {
            let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&config) else {
                continue;
            };
            let role = value
                .get("role")
                .and_then(|role| role.as_str())
                .map(str::trim)
                .unwrap_or("");
            if normalize_route_key(role) != "achitecture" {
                continue;
            }
            value["role"] = serde_json::Value::String("Architecture".to_string());
            conn.execute(
                "UPDATE agents SET config = ?1, updated_at = ?2 WHERE id = ?3",
                params![
                    serde_json::to_string(&value)?,
                    chrono::Utc::now().to_rfc3339(),
                    agent_id
                ],
            )?;
        }

        Ok(())
    }

    fn migrate_knowledge_provenance(conn: &mut Connection) -> Result<()> {
        let tx = conn.transaction()?;
        let records: Vec<(String, String, Option<String>, String)> = {
            let mut stmt =
                tx.prepare("SELECT id, content, vault_path, updated_at FROM knowledge")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };

        let mut external_sources =
            std::collections::HashMap::<String, Vec<(String, String)>>::new();
        for (id, content, vault_path, updated_at) in records {
            let content_hash = KnowledgeItem::content_hash(&content);
            if let Some(path) = vault_path.filter(|path| !path.trim().is_empty()) {
                let source_uri = KnowledgeItem::normalize_file_source(&path);
                tx.execute(
                    "UPDATE knowledge
                     SET source_kind = 'obsidian', source_uri_normalized = ?1, content_hash = ?2
                     WHERE id = ?3",
                    params![source_uri, content_hash, id],
                )?;
                external_sources
                    .entry(source_uri)
                    .or_default()
                    .push((id, updated_at));
            } else {
                tx.execute(
                    "UPDATE knowledge SET content_hash = COALESCE(content_hash, ?1) WHERE id = ?2",
                    params![content_hash, id],
                )?;
            }
        }

        for (source_uri, mut versions) in external_sources {
            if versions.len() <= 1 {
                continue;
            }
            versions.sort_by(|left, right| right.1.cmp(&left.1));
            let retained_id = versions[0].0.clone();
            for (removed_id, _) in versions.into_iter().skip(1) {
                tx.execute(
                    "INSERT INTO knowledge_migration_audit (
                        id, migration_version, source_uri_normalized, retained_id, removed_id, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        uuid::Uuid::new_v4().to_string(),
                        2026052503_i64,
                        source_uri,
                        retained_id,
                        removed_id,
                        chrono::Utc::now().to_rfc3339(),
                    ],
                )?;
                tx.execute(
                    "DELETE FROM knowledge_fts WHERE id = ?1",
                    params![removed_id],
                )?;
                tx.execute("DELETE FROM knowledge WHERE id = ?1", params![removed_id])?;
            }
        }

        tx.execute("DELETE FROM knowledge_fts", [])?;
        let retained_documents: Vec<(String, String, String, Option<String>)> = {
            let mut stmt = tx.prepare("SELECT id, title, content, tags FROM knowledge")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };
        for (id, title, content, tags) in retained_documents {
            tx.execute(
                "INSERT INTO knowledge_fts (id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
                params![id, title, content, tags.unwrap_or_else(|| "[]".to_string())],
            )?;
        }
        tx.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS ux_knowledge_external_source
             ON knowledge(source_kind, source_uri_normalized)
             WHERE source_uri_normalized IS NOT NULL;",
        )?;
        tx.commit()?;
        Ok(())
    }

    fn migrate_legacy_credentials(conn: &mut Connection) -> Result<()> {
        let tx = conn.transaction()?;
        let removed_settings = tx.execute(
            "DELETE FROM app_settings
             WHERE key IN ('output_image_api_key', 'output_pdf_api_key', 'output_video_api_key')",
            [],
        )?;
        let cleared_providers = tx.execute(
            "UPDATE provider_configs SET api_key_ref = NULL
             WHERE api_key_ref IS NOT NULL
               AND TRIM(api_key_ref) <> ''
               AND api_key_ref NOT LIKE 'secret://%'
               AND api_key_ref NOT LIKE 'env:%'",
            [],
        )?;
        let cleared_mcp_env = tx.execute(
            "UPDATE mcp_servers SET env_secret_refs = '{}', updated_at = ?1
             WHERE env_secret_refs IS NOT NULL
               AND env_secret_refs <> '{}'
               AND env_secret_refs NOT LIKE '%secret://%'",
            params![chrono::Utc::now().to_rfc3339()],
        )?;
        tx.execute(
            "INSERT INTO audit_log (id, timestamp, user_id, action, resource, details)
             VALUES (?1, ?2, NULL, 'credential_legacy_scrubbed', 'secure_configuration', ?3)",
            params![
                uuid::Uuid::new_v4().to_string(),
                chrono::Utc::now().to_rfc3339(),
                format!(
                    "Removed {} raw output setting(s), cleared {} raw provider reference(s), cleared {} unsafe MCP environment configuration(s). Reconfiguration requires secret:// or env: references.",
                    removed_settings, cleared_providers, cleared_mcp_env
                )
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn migrate_provider_configs_to_id_identity(conn: &mut Connection) -> Result<()> {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS provider_configs_v2 (
                id TEXT PRIMARY KEY,
                provider_name TEXT NOT NULL,
                model TEXT NOT NULL,
                adapter_type TEXT NOT NULL,
                command TEXT,
                node_version TEXT,
                config TEXT,
                api_key_ref TEXT,
                status TEXT NOT NULL DEFAULT 'available',
                is_builtin INTEGER DEFAULT 0,
                capabilities TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            INSERT OR REPLACE INTO provider_configs_v2 (
                id, provider_name, model, adapter_type, command, node_version, config,
                api_key_ref, status, is_builtin, capabilities, created_at, updated_at
            )
            SELECT
                id, provider_name, model, adapter_type, command, node_version, config,
                api_key_ref, status, COALESCE(is_builtin, 0), capabilities, created_at, updated_at
            FROM provider_configs;

            DROP TABLE provider_configs;
            ALTER TABLE provider_configs_v2 RENAME TO provider_configs;",
        )?;
        tx.commit()?;
        Ok(())
    }

    fn sync_task_dependencies(
        conn: &Connection,
        task_id: &str,
        instance_id: Option<&str>,
        payload: Option<&str>,
    ) -> Result<()> {
        conn.execute(
            "DELETE FROM task_dependencies WHERE task_id = ?1",
            params![task_id],
        )?;
        let (Some(instance_id), Some(payload)) = (instance_id, payload) else {
            return Ok(());
        };
        let Ok(task) =
            serde_json::from_str::<crate::application::orchestration::core::DagTask>(payload)
        else {
            return Ok(());
        };
        for dependency in task.dependencies {
            let dependency_id = if dependency.starts_with(&format!("{}:", instance_id)) {
                dependency
            } else {
                format!("{}:{}", instance_id, dependency)
            };
            conn.execute(
                "INSERT OR IGNORE INTO task_dependencies (task_id, depends_on_task_id)
                 VALUES (?1, ?2)",
                params![task_id, dependency_id],
            )?;
        }
        Ok(())
    }

    fn backfill_task_dependencies(conn: &Connection) -> Result<()> {
        let tasks: Vec<(String, Option<String>, Option<String>)> = {
            let mut stmt = conn.prepare("SELECT id, instance_id, payload FROM tasks")?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };
        for (task_id, instance_id, payload) in tasks {
            Self::sync_task_dependencies(
                conn,
                &task_id,
                instance_id.as_deref(),
                payload.as_deref(),
            )?;
        }
        Ok(())
    }

    fn ensure_knowledge_entries_fts(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let sql: Option<String> = conn
            .prepare("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'knowledge_entries_fts'")?
            .query_row([], |row| row.get(0))
            .optional()?;

        let needs_rebuild = match sql {
            None => true,
            Some(s) => {
                let s = s.to_lowercase();
                s.contains("content='knowledge_entries'")
                    || s.contains("content_rowid")
                    || !s.contains("id unindexed")
            }
        };

        if needs_rebuild {
            conn.execute("DROP TABLE IF EXISTS knowledge_entries_fts", [])?;
            conn.execute(
                "CREATE VIRTUAL TABLE knowledge_entries_fts USING fts5(id UNINDEXED, title, content, tags)",
                [],
            )?;
            let mut stmt =
                conn.prepare("SELECT id, title, content, tags FROM knowledge_entries")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?
                        .unwrap_or_else(|| "[]".to_string()),
                ))
            })?;
            for r in rows {
                let (id, title, content, tags) = r?;
                let _ = conn.execute(
                    "INSERT INTO knowledge_entries_fts (id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
                    params![id, title, content, tags],
                )?;
            }
        }
        Ok(())
    }
}
impl crate::core::traits::database::DatabasePort for Database {
    fn seed_provider_templates(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT count(*) FROM provider_templates")?;
        let count: i64 = stmt.query_row(rusqlite::params![], |row: &rusqlite::Row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        let templates = [
            (
                "Claude Code (Anthropic)",
                "REST API + Streaming",
                "AnthropicAdapter",
                vec![
                    "claude-opus-4-5",
                    "claude-sonnet-4-5",
                    "claude-haiku-3-5",
                    "claude-3-opus-20240229",
                    "claude-3-5-sonnet-20241022",
                ],
                "https://api.anthropic.com/v1",
            ),
            (
                "Codex (OpenAI)",
                "REST API + Streaming",
                "OpenAIAdapter",
                vec![
                    "gpt-4o",
                    "gpt-4o-mini",
                    "gpt-4-turbo",
                    "o1-preview",
                    "o1-mini",
                    "gpt-3.5-turbo",
                ],
                "https://api.openai.com/v1",
            ),
            (
                "Gemini (Google)",
                "REST API + Streaming",
                "GeminiAdapter",
                vec![
                    "gemini-2.0-flash",
                    "gemini-2.0-flash-thinking-exp",
                    "gemini-1.5-pro",
                    "gemini-1.5-flash",
                    "gemini-1.5-flash-8b",
                ],
                "https://generativelanguage.googleapis.com/v1beta",
            ),
            (
                "iFlow",
                "Custom Protocol",
                "IFlowAdapter",
                vec!["iflow-agent-v1", "iflow-agent-v2", "iflow-orchestrator"],
                "http://localhost:8080/api",
            ),
            (
                "OpenCode",
                "REST API",
                "OpenCodeAdapter",
                vec!["opencode-base", "opencode-pro", "opencode-mini"],
                "https://api.opencode.ai/v1",
            ),
            (
                "Custom Provider",
                "BaseProviderAdapter",
                "CustomAdapter",
                vec!["custom-model"],
                "",
            ),
        ];

        for (label, protocol, adapter, models, url) in templates {
            let id = uuid::Uuid::new_v4().to_string();
            let models_json = serde_json::to_string(&models).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "INSERT INTO provider_templates (id, label, protocol, adapter, models, default_base_url) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, label, protocol, adapter, models_json, url],
            )?;
        }

        Ok(())
    }

    fn list_provider_templates(&self) -> Result<Vec<ProviderTemplate>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, label, protocol, adapter, models, default_base_url FROM provider_templates ORDER BY label ASC")?;
        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            let models_json: String = row.get(4)?;
            let models = serde_json::from_str(&models_json).unwrap_or_default();
            Ok(ProviderTemplate {
                id: row.get(0)?,
                label: row.get(1)?,
                protocol: row.get(2)?,
                adapter: row.get(3)?,
                models,
                default_base_url: row.get(5)?,
            })
        })?;

        let mut templates = Vec::new();
        for t in iter {
            templates.push(t?);
        }
        Ok(templates)
    }

    fn insert_provider(&self, p: &Provider) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let cap_json = p.capabilities.as_ref().map(|c| c.to_json());
        conn.execute(
            "INSERT OR REPLACE INTO provider_configs (id, provider_name, model, adapter_type, command, api_key_ref, status, capabilities, is_builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10)",
            params![p.id, p.provider_name, p.model, p.adapter_type, p.command, p.api_key_ref, p.status, cap_json, now, now],
        )?;
        Ok(())
    }

    fn list_providers(&self) -> Result<Vec<Provider>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, provider_name, model, adapter_type, command, api_key_ref, status, capabilities
             FROM provider_configs
             WHERE is_builtin = 0
             ORDER BY datetime(updated_at) DESC, provider_name ASC",
        )?;
        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            let cap_json: Option<String> = row.get(7)?;
            let capabilities = cap_json
                .as_deref()
                .and_then(crate::core::models::ModelCapability::from_json);
            Ok(Provider {
                id: row.get(0)?,
                provider_name: row.get(1)?,
                model: row.get(2)?,
                adapter_type: row.get(3)?,
                command: row.get(4)?,
                api_key_ref: row.get(5)?,
                status: row.get(6)?,
                capabilities,
            })
        })?;

        let mut providers = Vec::new();
        for p in iter {
            providers.push(p?);
        }
        Ok(providers)
    }

    fn get_provider_by_name(&self, provider_name: &str) -> Result<Option<Provider>> {
        let conn = self.conn.lock().unwrap();
        if let Some((name, model)) = provider_name.split_once(" / ") {
            let mut stmt = conn.prepare(
                "SELECT id, provider_name, model, adapter_type, command, api_key_ref, status, capabilities
                 FROM provider_configs
                 WHERE provider_name = ?1 AND model = ?2
                 ORDER BY datetime(updated_at) DESC
                 LIMIT 1",
            )?;
            let mut rows = stmt.query(rusqlite::params![name.trim(), model.trim()])?;

            if let Some(row) = rows.next()? {
                let cap_json: Option<String> = row.get(7)?;
                let capabilities = cap_json
                    .as_deref()
                    .and_then(crate::core::models::ModelCapability::from_json);
                return Ok(Some(Provider {
                    id: row.get(0)?,
                    provider_name: row.get(1)?,
                    model: row.get(2)?,
                    adapter_type: row.get(3)?,
                    command: row.get(4)?,
                    api_key_ref: row.get(5)?,
                    status: row.get(6)?,
                    capabilities,
                }));
            }
        }

        let normalized = provider_name
            .split(" / ")
            .next()
            .unwrap_or(provider_name)
            .trim()
            .to_string();
        let mut stmt = conn.prepare(
            "SELECT id, provider_name, model, adapter_type, command, api_key_ref, status, capabilities
             FROM provider_configs
             WHERE provider_name = ?1
             ORDER BY datetime(updated_at) DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(rusqlite::params![normalized])?;

        if let Some(row) = rows.next()? {
            let cap_json: Option<String> = row.get(7)?;
            let capabilities = cap_json
                .as_deref()
                .and_then(crate::core::models::ModelCapability::from_json);
            Ok(Some(Provider {
                id: row.get(0)?,
                provider_name: row.get(1)?,
                model: row.get(2)?,
                adapter_type: row.get(3)?,
                command: row.get(4)?,
                api_key_ref: row.get(5)?,
                status: row.get(6)?,
                capabilities,
            }))
        } else {
            Ok(None)
        }
    }

    fn insert_team(&self, team: &Team) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO teams (id, name, description, objectives, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                team.id,
                team.name,
                team.description,
                team.objectives,
                team.created_at,
                team.updated_at
            ],
        )?;
        Ok(())
    }

    fn create_instance(
        &self,
        id: &str,
        name: &str,
        team_id: &str,
        config: Option<&str>,
        state: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO instances (id, name, team_id, config, state, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, name, team_id, config, state, now],
        )?;
        Ok(())
    }

    fn update_instance_name(&self, instance_id: &str, name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE instances SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, instance_id],
        )?;
        Ok(())
    }

    fn update_instance_state(&self, instance_id: &str, state: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE instances SET state = ?1 WHERE id = ?2",
            rusqlite::params![state, instance_id],
        )?;
        Ok(())
    }

    fn list_instances(&self) -> Result<Vec<Instance>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, team_id, config, state, created_at
             FROM instances
             ORDER BY created_at DESC",
        )?;
        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            Ok(Instance {
                id: row.get(0)?,
                name: row.get(1)?,
                team_id: row.get(2)?,
                config: row.get(3)?,
                state: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut instances = Vec::new();
        for r in iter {
            instances.push(r?);
        }
        Ok(instances)
    }

    fn list_teams(&self) -> Result<Vec<Team>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, objectives, created_at, updated_at
             FROM teams
             ORDER BY updated_at DESC",
        )?;
        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            Ok(Team {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                objectives: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        let mut teams = Vec::new();
        for t in iter {
            teams.push(t?);
        }
        Ok(teams)
    }

    fn insert_agent(&self, agent: &Agent) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO agents (id, name, provider, system_prompt, config, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                agent.id,
                agent.name,
                agent.provider,
                agent.system_prompt,
                agent.config,
                agent.status,
                agent.created_at,
                agent.updated_at
            ],
        )?;
        Ok(())
    }

    fn delete_agent(&self, agent_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Remove relationships if any (foreign keys with ON DELETE CASCADE normally handle this, but let's be explicit for team_agents)
        conn.execute(
            "DELETE FROM team_agents WHERE agent_id = ?1",
            params![agent_id],
        )?;
        conn.execute("DELETE FROM agents WHERE id = ?1", params![agent_id])?;
        Ok(())
    }

    fn list_agents(&self) -> Result<Vec<Agent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, provider, system_prompt, config, status, created_at, updated_at
             FROM agents
             ORDER BY updated_at DESC",
        )?;
        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            Ok(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                system_prompt: row.get(3)?,
                config: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        let mut agents = Vec::new();
        for a in iter {
            agents.push(a?);
        }
        Ok(agents)
    }
    fn assign_agent_to_team(&self, team_id: &str, agent_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO members (id, team_id, agent_id, joined_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, team_id, agent_id, now],
        )?;
        Ok(())
    }

    fn remove_agent_from_team(&self, team_id: &str, agent_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM members WHERE team_id = ?1 AND agent_id = ?2",
            params![team_id, agent_id],
        )?;
        Ok(())
    }

    fn get_team_agents(&self, team_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT agent_id FROM members WHERE team_id = ?1")?;
        let iter = stmt.query_map(params![team_id], |row: &rusqlite::Row| row.get(0))?;

        let mut agents = Vec::new();
        for a in iter {
            agents.push(a?);
        }
        agents.sort();
        agents.dedup();
        sort_agent_ids_by_routing_role(&conn, &mut agents);
        Ok(agents)
    }

    fn get_instance_agents(&self, instance_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "
            SELECT m.agent_id
            FROM members m
            WHERE m.instance_id = ?1
            ORDER BY m.joined_at ASC
        ",
        )?;
        let iter = stmt.query_map(params![instance_id], |row: &rusqlite::Row| row.get(0))?;
        let mut agents = Vec::new();
        for a in iter {
            agents.push(a?);
        }

        let mut stmt = conn.prepare("SELECT team_id FROM instances WHERE id = ?1")?;
        let team_id: String =
            stmt.query_row(params![instance_id], |row: &rusqlite::Row| row.get(0))?;
        drop(stmt);

        if agents.is_empty() {
            let mut stmt = conn.prepare(
                "
                SELECT m.agent_id
                FROM members m
                WHERE m.team_id = ?1
                  AND (m.instance_id IS NULL OR m.instance_id = '')
                ORDER BY m.joined_at ASC
            ",
            )?;
            let iter = stmt.query_map(params![team_id], |row: &rusqlite::Row| row.get(0))?;
            for a in iter {
                agents.push(a?);
            }
        } else if !has_coordinator_agent(&conn, &agents) {
            if let Some(coordinator_id) = find_team_coordinator_agent(&conn, &team_id)? {
                agents.push(coordinator_id);
            }
        }

        agents.sort();
        agents.dedup();
        sort_agent_ids_by_routing_role(&conn, &mut agents);
        Ok(agents)
    }

    fn get_instance_agent_name_mapping(
        &self,
        instance_id: &str,
    ) -> Result<std::collections::HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut map = std::collections::HashMap::new();

        let mut stmt = conn.prepare(
            "SELECT a.name, a.config, a.status, m.agent_id
             FROM members m
             JOIN agents a ON m.agent_id = a.id
             WHERE m.instance_id = ?1",
        )?;

        let iter = stmt.query_map(rusqlite::params![instance_id], |row: &rusqlite::Row| {
            let agent_name: String = row.get(0)?;
            let config: Option<String> = row.get(1)?;
            let status: String = row.get(2)?;
            let agent_id: String = row.get(3)?;
            let config_json = config
                .as_deref()
                .and_then(|config| serde_json::from_str::<serde_json::Value>(config).ok());
            let role = config_json
                .as_ref()
                .and_then(|value| {
                    value
                        .get("role")
                        .and_then(|role| role.as_str())
                        .map(str::trim)
                        .filter(|role| !role.is_empty())
                        .map(ToOwned::to_owned)
                })
                .unwrap_or_else(|| agent_name.clone());
            let position = config_json.as_ref().and_then(|value| {
                value
                    .get("position")
                    .and_then(|position| position.as_str())
                    .map(str::trim)
                    .filter(|position| !position.is_empty())
                    .map(ToOwned::to_owned)
            });
            Ok((agent_name, role, position, agent_id, status))
        })?;

        for (agent_name, role, position, agent_id, status) in iter.flatten() {
            if status.to_lowercase() != "offline" {
                for key in [Some(agent_name), Some(role), position] {
                    if let Some(key) = key.filter(|key| !key.trim().is_empty()) {
                        map.insert(key.clone(), agent_id.clone());
                        let normalized = normalize_route_key(&key);
                        if !normalized.is_empty() {
                            map.insert(normalized, agent_id.clone());
                        }
                    }
                }
            }
        }

        if !map.is_empty() {
            if !map
                .keys()
                .any(|key| normalize_route_key(key) == "coordinator")
            {
                let team_id = conn
                    .query_row(
                        "SELECT team_id FROM instances WHERE id = ?1",
                        rusqlite::params![instance_id],
                        |row: &rusqlite::Row| row.get::<_, String>(0),
                    )
                    .optional()?;
                if let Some(team_id) = team_id {
                    if let Some(coordinator_id) = find_team_coordinator_agent(&conn, &team_id)? {
                        if let Some((agent_name, role, position, agent_id, status)) = conn
                            .query_row(
                                "SELECT a.name, a.config, a.status, a.id
                                 FROM agents a
                                 WHERE a.id = ?1",
                                rusqlite::params![coordinator_id],
                                |row: &rusqlite::Row| {
                                    let agent_name: String = row.get(0)?;
                                    let config: Option<String> = row.get(1)?;
                                    let status: String = row.get(2)?;
                                    let agent_id: String = row.get(3)?;
                                    let config_json = config.as_deref().and_then(|config| {
                                        serde_json::from_str::<serde_json::Value>(config).ok()
                                    });
                                    let role = config_json
                                        .as_ref()
                                        .and_then(|value| {
                                            value
                                                .get("role")
                                                .and_then(|role| role.as_str())
                                                .map(str::trim)
                                                .filter(|role| !role.is_empty())
                                                .map(ToOwned::to_owned)
                                        })
                                        .unwrap_or_else(|| agent_name.clone());
                                    let position = config_json.as_ref().and_then(|value| {
                                        value
                                            .get("position")
                                            .and_then(|position| position.as_str())
                                            .map(str::trim)
                                            .filter(|position| !position.is_empty())
                                            .map(ToOwned::to_owned)
                                    });
                                    Ok((agent_name, role, position, agent_id, status))
                                },
                            )
                            .optional()?
                        {
                            if status.to_lowercase() != "offline" {
                                for key in [Some(agent_name), Some(role), position] {
                                    if let Some(key) = key.filter(|key| !key.trim().is_empty()) {
                                        map.insert(key.clone(), agent_id.clone());
                                        let normalized = normalize_route_key(&key);
                                        if !normalized.is_empty() {
                                            map.insert(normalized, agent_id.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            return Ok(map);
        }

        // Fallback to team_id
        let mut stmt_team = conn.prepare("SELECT team_id FROM instances WHERE id = ?1")?;
        let team_id_result: Result<String, _> = stmt_team
            .query_row(rusqlite::params![instance_id], |row: &rusqlite::Row| {
                row.get(0)
            });
        drop(stmt_team);

        if let Ok(team_id) = team_id_result {
            let mut stmt = conn.prepare(
                "SELECT a.name, a.config, a.status, m.agent_id
                 FROM members m
                 JOIN agents a ON m.agent_id = a.id
                 WHERE m.team_id = ?1",
            )?;

            let iter = stmt.query_map(rusqlite::params![team_id], |row: &rusqlite::Row| {
                let agent_name: String = row.get(0)?;
                let config: Option<String> = row.get(1)?;
                let status: String = row.get(2)?;
                let agent_id: String = row.get(3)?;
                let config_json = config
                    .as_deref()
                    .and_then(|config| serde_json::from_str::<serde_json::Value>(config).ok());
                let role = config_json
                    .as_ref()
                    .and_then(|value| {
                        value
                            .get("role")
                            .and_then(|role| role.as_str())
                            .map(str::trim)
                            .filter(|role| !role.is_empty())
                            .map(ToOwned::to_owned)
                    })
                    .unwrap_or_else(|| agent_name.clone());
                let position = config_json.as_ref().and_then(|value| {
                    value
                        .get("position")
                        .and_then(|position| position.as_str())
                        .map(str::trim)
                        .filter(|position| !position.is_empty())
                        .map(ToOwned::to_owned)
                });
                Ok((agent_name, role, position, agent_id, status))
            })?;

            for (agent_name, role, position, agent_id, status) in iter.flatten() {
                if status.to_lowercase() != "offline" {
                    for key in [Some(agent_name), Some(role), position] {
                        if let Some(key) = key.filter(|key| !key.trim().is_empty()) {
                            map.insert(key.clone(), agent_id.clone());
                            let normalized = normalize_route_key(&key);
                            if !normalized.is_empty() {
                                map.insert(normalized, agent_id.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(map)
    }

    fn upsert_task(
        &self,
        id: &str,
        team_id: &str,
        instance_id: Option<&str>,
        run_id: Option<&str>,
        assignee_id: Option<&str>,
        status: &str,
        priority: &str,
        payload: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO tasks
                (id, team_id, instance_id, run_id, assignee_id, status, priority, payload, claimed_at, created_at, updated_at)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                team_id = excluded.team_id,
                instance_id = excluded.instance_id,
                run_id = COALESCE(excluded.run_id, tasks.run_id),
                assignee_id = excluded.assignee_id,
                status = excluded.status,
                priority = excluded.priority,
                payload = excluded.payload,
                updated_at = excluded.updated_at",
            params![
                id,
                team_id,
                instance_id,
                run_id,
                assignee_id,
                status,
                priority,
                payload,
                now,
                now
            ],
        )?;
        Self::sync_task_dependencies(&conn, id, instance_id, payload)?;
        if run_id.is_some() {
            record_task_run_event(&conn, id, "task_created", assignee_id, payload)?;
        }
        Ok(())
    }

    fn get_total_tokens_per_agent(&self) -> Result<Vec<(String, usize)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.name, SUM(t.total_tokens)
             FROM token_usage t
             JOIN agents a ON t.agent_id = a.id
             GROUP BY t.agent_id
             ORDER BY SUM(t.total_tokens) DESC",
        )?;

        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            let name: String = row.get(0)?;
            let tokens: usize = row.get(1)?;
            Ok((name, tokens))
        })?;

        let mut result = Vec::new();
        for item in iter {
            result.push(item?);
        }
        Ok(result)
    }

    fn get_total_tokens_per_instance(&self) -> Result<Vec<(String, usize)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT i.id, SUM(t.total_tokens)
             FROM token_usage t
             JOIN instances i ON t.instance_id = i.id
             GROUP BY t.instance_id
             ORDER BY SUM(t.total_tokens) DESC",
        )?;

        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            let id: String = row.get(0)?;
            let tokens: usize = row.get(1)?;
            Ok((id, tokens))
        })?;

        let mut result = Vec::new();
        for item in iter {
            result.push(item?);
        }
        Ok(result)
    }

    fn get_agent_instance_count(&self) -> Result<Vec<(String, usize)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.name, COUNT(DISTINCT m.instance_id)
             FROM members m
             JOIN agents a ON m.agent_id = a.id
             WHERE m.instance_id IS NOT NULL
             GROUP BY m.agent_id
             ORDER BY COUNT(DISTINCT m.instance_id) DESC",
        )?;

        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            let name: String = row.get(0)?;
            let count: usize = row.get(1)?;
            Ok((name, count))
        })?;

        let mut result = Vec::new();
        for item in iter {
            result.push(item?);
        }
        Ok(result)
    }

    fn get_total_daily_tokens(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().naive_utc().date().to_string();
        let mut stmt =
            conn.prepare("SELECT SUM(total_tokens) FROM token_usage WHERE date(created_at) = ?1")?;
        let count: Option<usize> = stmt
            .query_row(rusqlite::params![now], |row: &rusqlite::Row| row.get(0))
            .unwrap_or(Some(0));
        Ok(count.unwrap_or(0))
    }

    fn get_total_tasks_completed(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM tasks WHERE status = 'completed'")?;
        let count: usize = stmt
            .query_row(rusqlite::params![], |row: &rusqlite::Row| row.get(0))
            .unwrap_or(0);
        Ok(count)
    }

    fn get_total_tasks_count(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM tasks")?;
        let count: usize = stmt
            .query_row(rusqlite::params![], |row: &rusqlite::Row| row.get(0))
            .unwrap_or(0);
        Ok(count)
    }

    fn get_active_agents_count(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COUNT(*) FROM agents WHERE status IN ('active', 'online', 'running')",
        )?;
        let count: usize = stmt
            .query_row(rusqlite::params![], |row: &rusqlite::Row| row.get(0))
            .unwrap_or(0);
        Ok(count)
    }

    fn insert_token_usage(
        &self,
        instance_id: Option<&str>,
        run_id: Option<&str>,
        agent_id: &str,
        input_tokens: usize,
        output_tokens: usize,
        total_tokens: usize,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO token_usage (id, instance_id, run_id, agent_id, input_tokens, output_tokens, total_tokens, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id,
                instance_id,
                run_id,
                agent_id,
                input_tokens,
                output_tokens,
                total_tokens,
                now
            ],
        )?;
        Ok(())
    }

    fn get_total_tokens_for_run(&self, run_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let total: Option<i64> = conn.query_row(
            "SELECT SUM(total_tokens) FROM token_usage WHERE run_id = ?1",
            params![run_id],
            |row| row.get(0),
        )?;
        Ok(total.unwrap_or(0).max(0) as usize)
    }

    fn assign_task_to_agent(&self, task_id: &str, agent_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks
             SET assignee_id = ?1, updated_at = ?2
             WHERE id = ?3",
            rusqlite::params![agent_id, now, task_id],
        )?;
        Ok(())
    }

    fn list_tasks_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<Vec<crate::tasks::shared_task_list::Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, team_id, instance_id, run_id, assignee_id, status, priority, payload, claimed_at, created_at, updated_at
             FROM tasks
             WHERE instance_id = ?1
             ORDER BY created_at DESC",
        )?;

        let iter = stmt.query_map(params![instance_id], |row: &rusqlite::Row| {
            Ok(crate::tasks::shared_task_list::Task {
                id: row.get(0)?,
                team_id: row.get(1)?,
                instance_id: row.get(2)?,
                run_id: row.get(3)?,
                assignee_id: row.get(4)?,
                status: row.get(5)?,
                priority: row.get(6)?,
                payload: row.get(7)?,
                claimed_at: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        let mut tasks = Vec::new();
        for r in iter {
            tasks.push(r?);
        }
        Ok(tasks)
    }

    fn list_recent_tasks(&self, limit: u32) -> Result<Vec<crate::tasks::shared_task_list::Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, team_id, instance_id, run_id, assignee_id, status, priority, payload, claimed_at, created_at, updated_at
             FROM tasks
             ORDER BY updated_at DESC
             LIMIT ?1",
        )?;

        let iter = stmt.query_map(params![limit], |row: &rusqlite::Row| {
            Ok(crate::tasks::shared_task_list::Task {
                id: row.get(0)?,
                team_id: row.get(1)?,
                instance_id: row.get(2)?,
                run_id: row.get(3)?,
                assignee_id: row.get(4)?,
                status: row.get(5)?,
                priority: row.get(6)?,
                payload: row.get(7)?,
                claimed_at: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in iter {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    fn list_pending_tasks_for_instance(
        &self,
        instance_id: &str,
        limit: u32,
    ) -> Result<Vec<crate::tasks::shared_task_list::Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, team_id, instance_id, run_id, assignee_id, status, priority, payload, claimed_at, created_at, updated_at
             FROM tasks pending
             WHERE pending.instance_id = ?1 AND pending.status = 'pending'
               AND NOT EXISTS (
                    SELECT 1
                    FROM task_dependencies dependency_link
                    LEFT JOIN tasks dependency ON dependency.id = dependency_link.depends_on_task_id
                    WHERE dependency_link.task_id = pending.id
                      AND (dependency.id IS NULL OR dependency.status != 'completed')
               )
             ORDER BY
                CASE priority
                    WHEN 'high' THEN 1
                    WHEN 'medium' THEN 2
                    WHEN 'low' THEN 3
                    ELSE 4
                END ASC,
                created_at ASC
             LIMIT ?2",
        )?;

        let iter = stmt.query_map(params![instance_id, limit], |row: &rusqlite::Row| {
            Ok(crate::tasks::shared_task_list::Task {
                id: row.get(0)?,
                team_id: row.get(1)?,
                instance_id: row.get(2)?,
                run_id: row.get(3)?,
                assignee_id: row.get(4)?,
                status: row.get(5)?,
                priority: row.get(6)?,
                payload: row.get(7)?,
                claimed_at: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        let mut tasks = Vec::new();
        for t in iter {
            tasks.push(t?);
        }
        Ok(tasks)
    }

    fn is_task_unblocked(&self, task_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let blocked: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM task_dependencies dependency_link
             LEFT JOIN tasks dependency ON dependency.id = dependency_link.depends_on_task_id
             WHERE dependency_link.task_id = ?1
               AND (dependency.id IS NULL OR dependency.status != 'completed')",
            params![task_id],
            |row| row.get(0),
        )?;
        Ok(blocked == 0)
    }

    fn claim_task_for_instance(
        &self,
        task_id: &str,
        agent_id: &str,
        instance_id: &str,
    ) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let rows_affected = conn.execute(
            "UPDATE tasks
             SET assignee_id = ?1, status = 'in_progress', claimed_at = ?2, updated_at = ?3
             WHERE id = ?4 AND status = 'pending' AND instance_id = ?5
               AND NOT EXISTS (
                    SELECT 1
                    FROM task_dependencies dependency_link
                    LEFT JOIN tasks dependency ON dependency.id = dependency_link.depends_on_task_id
                    WHERE dependency_link.task_id = tasks.id
                      AND (dependency.id IS NULL OR dependency.status != 'completed')
               )",
            params![agent_id, now, now, task_id, instance_id],
        )?;
        if rows_affected > 0 {
            record_task_run_event(
                &conn,
                task_id,
                "task_claimed",
                Some(agent_id),
                Some("Task claimed for execution"),
            )?;
        }
        Ok(rows_affected > 0)
    }

    fn recover_stale_in_progress_tasks(&self, max_age_seconds: u64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let max_age_seconds = max_age_seconds.clamp(60, 86_400) as i64;
        let cutoff = (Utc::now() - chrono::Duration::seconds(max_age_seconds)).to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id
             FROM tasks
             WHERE status = 'in_progress'
               AND (updated_at IS NULL OR updated_at < ?1)",
        )?;
        let task_ids = stmt
            .query_map(params![cutoff], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);
        if task_ids.is_empty() {
            return Ok(0);
        }

        let now = Utc::now().to_rfc3339();
        let rows = conn.execute(
            "UPDATE tasks
             SET status = 'pending', claimed_at = NULL, updated_at = ?1
             WHERE status = 'in_progress'
               AND (updated_at IS NULL OR updated_at < ?2)",
            params![now, cutoff],
        )?;
        for task_id in task_ids.iter().take(rows) {
            record_task_run_event(
                &conn,
                task_id,
                "task_recovered_after_restart",
                None,
                Some("Stale in-progress task was returned to pending after startup recovery."),
            )?;
        }
        Ok(rows)
    }

    fn mark_task_completed(&self, task_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
        )?;
        record_task_run_event(&conn, task_id, "task_completed", None, None)?;
        Ok(())
    }

    fn mark_task_failed(&self, task_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = 'failed', updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
        )?;
        record_task_run_event(&conn, task_id, "task_failed", None, None)?;
        Ok(())
    }

    fn mark_task_waiting_approval(&self, task_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = 'waiting_approval', updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
        )?;
        record_task_run_event(&conn, task_id, "task_waiting_approval", None, None)?;
        Ok(())
    }

    fn resolve_waiting_tasks_for_run(&self, run_id: &str, status: &str) -> Result<()> {
        if !matches!(status, "pending" | "failed") {
            anyhow::bail!("Invalid task status after approval decision: {}", status);
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2
             WHERE run_id = ?3 AND status = 'waiting_approval'",
            params![status, Utc::now().to_rfc3339(), run_id],
        )?;
        Ok(())
    }

    fn seed_sdg_team(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT count(*) FROM teams WHERE name = 'SDG'")?;
        let count: i64 = stmt.query_row(rusqlite::params![], |row: &rusqlite::Row| row.get(0))?;
        if count > 0 {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT OR IGNORE INTO instances (id, team_id, config, state, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    "sdg-instance-123",
                    "sdg-team-123",
                    None::<String>,
                    None::<String>,
                    now
                ],
            )?;
            return Ok(());
        }

        let now = chrono::Utc::now().to_rfc3339();
        let team_id = "sdg-team-123".to_string();

        // conn.execute(
        //     "INSERT INTO teams (id, name, description, objectives, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        //     rusqlite::params![team_id, "SDG", "SDG Team", "Test real data with OpenRouter", now, now],
        // )?;

        // conn.execute(
        //     "INSERT OR IGNORE INTO instances (id, team_id, config, state, created_at)
        //      VALUES (?1, ?2, ?3, ?4, ?5)",
        //     rusqlite::params![
        //         "sdg-instance-123",
        //         team_id,
        //         None::<String>,
        //         None::<String>,
        //         now
        //     ],
        // )?;

        conn.execute(
            "INSERT OR IGNORE INTO provider_configs (id, provider_name, model, adapter_type, command, api_key_ref, status, is_builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9)",
            rusqlite::params![
                "openrouter-gemma-config",
                "openrouter",
                "google/gemma-4-31b-it",
                "openai_compatible",
                None::<String>,
                None::<String>,
                "available",
                now,
                now
            ],
        )?;

        let agents = [
            (
                "Coordinator",
                "sdg-coord-123",
                "You are the Coordinator/Leader of the SDG team. You are responsible for breaking down goals, assigning tasks to other agents, and orchestrating the workflow.",
                r#"{"role":"Coordinator","position":"Coordinator","responsibilities":["coordination","planning","delegation","review","handoff"],"competencies":["planning","review","handoff"],"allowed_task_types":["planning","review","handoff","research","analysis"],"disallowed_task_types":["implementation","testing","build","documentation","content","design","marketing","operations"]}"#,
            ),
            (
                "PM",
                "sdg-pm-123",
                "You are the PM of the SDG team. Please provide short, direct responses about product management.",
                r#"{"role":"PM","position":"Project Manager","responsibilities":["scope","roadmap","prioritization","acceptance criteria"],"competencies":["planning","product","review","documentation"],"allowed_task_types":["planning","product","documentation","review"],"disallowed_task_types":["implementation","testing","build"]}"#,
            ),
            (
                "DEV",
                "sdg-dev-123",
                "You are the DEV of the SDG team. You write code and solve technical issues.",
                r#"{"role":"DEV","position":"Software Engineer","responsibilities":["implementation","testing","build","technical troubleshooting"],"competencies":["implementation","testing","build","operations"],"allowed_task_types":["implementation","testing","build","operations","review"],"disallowed_task_types":[]}"#,
            ),
            (
                "BA",
                "sdg-ba-123",
                "You are the BA of the SDG team. You analyze business requirements and metrics.",
                r#"{"role":"BA","position":"Business Analyst","responsibilities":["requirements","business analysis","domain analysis","metrics"],"competencies":["analysis","documentation","research","review"],"allowed_task_types":["analysis","documentation","research","review"],"disallowed_task_types":["implementation","testing","build"]}"#,
            ),
        ];
        for (name, agent_id, prompt, config) in agents {
            conn.execute(
                "INSERT INTO agents (id, name, provider, system_prompt, config, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![agent_id, name, "openrouter", prompt, Some(config), "online", now, now],
            )?;
            conn.execute(
                "INSERT INTO members (id, team_id, agent_id, joined_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), team_id, agent_id, now],
            )?;
        }

        Ok(())
    }
    fn get_agent(&self, agent_id: &str) -> Result<Option<Agent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, provider, system_prompt, config, status, created_at, updated_at FROM agents WHERE id = ?1")?;
        let mut rows = stmt.query(params![agent_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                system_prompt: row.get(3)?,
                config: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn insert_team_message(
        &self,
        msg: &crate::infrastructure::message_bus::routing::TeamMessage,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let msg_type = match msg.message_type {
            crate::infrastructure::message_bus::routing::MessageType::Direct => "direct",
            crate::infrastructure::message_bus::routing::MessageType::Broadcast => "broadcast",
            crate::infrastructure::message_bus::routing::MessageType::RoleGroup => "role_group",
            crate::infrastructure::message_bus::routing::MessageType::System => "system",
        };

        conn.execute(
            "INSERT INTO team_messages (
                id, team_instance_id, sender_member_id, recipient_member_id,
                recipient_role, message_type, content, metadata, delivery_status, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                msg.id,
                msg.team_instance_id,
                msg.sender_member_id,
                msg.recipient_member_id,
                msg.recipient_role,
                msg_type,
                msg.content,
                msg.metadata,
                msg.delivery_status,
                msg.created_at,
            ],
        )?;
        Ok(())
    }

    fn get_team_messages_for_instance(
        &self,
        team_instance_id: &str,
        limit: u32,
    ) -> Result<Vec<crate::infrastructure::message_bus::routing::TeamMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, team_instance_id, sender_member_id, recipient_member_id,
                    recipient_role, message_type, content, metadata, delivery_status, created_at
             FROM team_messages
             WHERE team_instance_id = ?1
             ORDER BY created_at ASC
             LIMIT ?2",
        )?;

        let iter = stmt.query_map(params![team_instance_id, limit], |row: &rusqlite::Row| {
            let msg_type_str: String = row.get(5)?;
            let message_type = match msg_type_str.as_str() {
                "direct" => crate::infrastructure::message_bus::routing::MessageType::Direct,
                "broadcast" => crate::infrastructure::message_bus::routing::MessageType::Broadcast,
                "role_group" => crate::infrastructure::message_bus::routing::MessageType::RoleGroup,
                _ => crate::infrastructure::message_bus::routing::MessageType::System,
            };

            Ok(crate::infrastructure::message_bus::routing::TeamMessage {
                id: row.get(0)?,
                team_instance_id: row.get(1)?,
                sender_member_id: row.get(2)?,
                recipient_member_id: row.get(3)?,
                recipient_role: row.get(4)?,
                message_type,
                content: row.get(6)?,
                metadata: row.get(7)?,
                delivery_status: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;

        let mut messages = Vec::new();
        for m in iter {
            messages.push(m?);
        }
        Ok(messages)
    }

    fn get_team_messages_for_instance_by_type(
        &self,
        team_instance_id: &str,
        message_type: crate::infrastructure::message_bus::routing::MessageType,
        limit: u32,
    ) -> Result<Vec<crate::infrastructure::message_bus::routing::TeamMessage>> {
        let msg_type = match message_type {
            crate::infrastructure::message_bus::routing::MessageType::Direct => "direct",
            crate::infrastructure::message_bus::routing::MessageType::Broadcast => "broadcast",
            crate::infrastructure::message_bus::routing::MessageType::RoleGroup => "role_group",
            crate::infrastructure::message_bus::routing::MessageType::System => "system",
        };

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, team_instance_id, sender_member_id, recipient_member_id,
                    recipient_role, message_type, content, metadata, delivery_status, created_at
             FROM team_messages
             WHERE team_instance_id = ?1 AND message_type = ?2
             ORDER BY created_at ASC
             LIMIT ?3",
        )?;

        let iter = stmt.query_map(
            params![team_instance_id, msg_type, limit],
            |row: &rusqlite::Row| {
                let msg_type_str: String = row.get(5)?;
                let message_type = match msg_type_str.as_str() {
                    "direct" => crate::infrastructure::message_bus::routing::MessageType::Direct,
                    "broadcast" => {
                        crate::infrastructure::message_bus::routing::MessageType::Broadcast
                    }
                    "role_group" => {
                        crate::infrastructure::message_bus::routing::MessageType::RoleGroup
                    }
                    _ => crate::infrastructure::message_bus::routing::MessageType::System,
                };

                Ok(crate::infrastructure::message_bus::routing::TeamMessage {
                    id: row.get(0)?,
                    team_instance_id: row.get(1)?,
                    sender_member_id: row.get(2)?,
                    recipient_member_id: row.get(3)?,
                    recipient_role: row.get(4)?,
                    message_type,
                    content: row.get(6)?,
                    metadata: row.get(7)?,
                    delivery_status: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )?;

        let mut messages = Vec::new();
        for m in iter {
            messages.push(m?);
        }
        Ok(messages)
    }

    fn update_team_message_delivery_status(&self, message_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE team_messages SET delivery_status = ?1 WHERE id = ?2",
            params![status, message_id],
        )?;
        Ok(())
    }

    fn update_team_message_content(&self, message_id: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE team_messages SET content = ?1 WHERE id = ?2",
            params![content, message_id],
        )?;
        Ok(())
    }

    fn append_conversation_turn(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        metadata: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO conversations (id, session_id, role, content, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, session_id, role, content, metadata, now],
        )?;
        Ok(())
    }

    fn ensure_session(
        &self,
        session_id: &str,
        agent_id: &str,
        team_instance_id: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO sessions (id, agent_id, team_instance_id, user_id, context, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?5)",
            params![session_id, agent_id, team_instance_id, now, now],
        )?;
        Ok(())
    }

    fn create_session_for_instance(&self, instance_id: &str, agent_id: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO sessions (id, agent_id, team_instance_id, user_id, context, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?5)",
            params![session_id, agent_id, instance_id, now, now],
        )?;
        Ok(session_id)
    }

    fn list_sessions_for_instance(&self, instance_id: &str) -> Result<Vec<SessionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, team_instance_id, created_at, updated_at
             FROM sessions
             WHERE team_instance_id = ?1
             ORDER BY updated_at DESC",
        )?;
        let iter = stmt.query_map(params![instance_id], |row: &rusqlite::Row| {
            Ok(SessionRecord {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                team_instance_id: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in iter {
            out.push(r?);
        }
        Ok(out)
    }

    fn get_latest_session_for_instance(&self, instance_id: &str) -> Result<Option<SessionRecord>> {
        let sessions = self.list_sessions_for_instance(instance_id)?;
        Ok(sessions.into_iter().next())
    }

    fn touch_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            params![now, session_id],
        )?;
        Ok(())
    }

    fn get_conversation_turns(
        &self,
        session_id: &str,
    ) -> Result<Vec<crate::core::models::ChatMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT role, content, metadata
             FROM conversations
             WHERE session_id = ?1
             ORDER BY created_at ASC",
        )?;
        let mut rows = stmt.query(rusqlite::params![session_id])?;

        let mut msgs = Vec::new();
        while let Some(row) = rows.next()? {
            let role = row.get::<usize, String>(0)?;
            let content = row.get::<usize, String>(1)?;
            let metadata = row.get::<usize, Option<String>>(2).unwrap_or(None);

            let mut agent_name = None;
            let mut thought_duration_secs = None;
            if let Some(meta_str) = metadata {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                    if let Some(name) = json.get("agent_name").and_then(|n| n.as_str()) {
                        agent_name = Some(name.to_string().into());
                    }
                    thought_duration_secs = json.get("thought_duration_secs").and_then(|n| {
                        n.as_f64()
                            .or_else(|| n.as_str().and_then(|s| s.parse::<f64>().ok()))
                    });
                }
            }

            msgs.push(crate::core::models::ChatMessage {
                role: role.into(),
                content: content.clone().into(),
                parts: vec![crate::core::models::ContentPart::Text(content)],
                agent_name,
                thought_duration_secs,
            });
        }
        Ok(msgs)
    }

    fn save_message(
        &self,
        team_id: &str,
        instance_id: Option<&str>,
        role: &str,
        content: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO messages (id, team_id, instance_id, sender_id, recipient_id, type, content, sent_at)
             VALUES (?1, ?2, ?3, ?4, NULL, 'direct', ?5, ?6)",
            params![id, team_id, instance_id, role, content, now],
        )?;
        Ok(())
    }

    fn get_messages(
        &self,
        team_id: &str,
        instance_id: Option<&str>,
    ) -> Result<Vec<crate::core::models::ChatMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt;
        let mut rows = if let Some(iid) = instance_id {
            stmt = conn.prepare("SELECT sender_id, content FROM messages WHERE team_id = ?1 AND instance_id = ?2 ORDER BY sent_at ASC")?;
            stmt.query(params![team_id, iid])?
        } else {
            stmt = conn.prepare("SELECT sender_id, content FROM messages WHERE team_id = ?1 AND instance_id IS NULL ORDER BY sent_at ASC")?;
            stmt.query(params![team_id])?
        };

        let mut msgs = Vec::new();
        while let Some(row) = rows.next()? {
            let role = row.get::<usize, String>(0)?;
            let content = row.get::<usize, String>(1)?;
            msgs.push(crate::core::models::ChatMessage {
                role: role.into(),
                content: content.clone().into(),
                parts: vec![crate::core::models::ContentPart::Text(content)],
                agent_name: None,
                thought_duration_secs: None,
            });
        }
        Ok(msgs)
    }

    fn upsert_knowledge_item(&self, item: &KnowledgeItem) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let tags_json = serde_json::to_string(&item.tags).unwrap_or_default();
        let persisted_id = match item.source_uri_normalized.as_deref() {
            Some(source_uri) => tx
                .query_row(
                    "SELECT id FROM knowledge
                     WHERE source_kind = ?1 AND source_uri_normalized = ?2
                     LIMIT 1",
                    params![item.source_kind, source_uri],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .unwrap_or_else(|| item.id.to_string()),
            None => item.id.to_string(),
        };

        tx.execute(
            "INSERT INTO knowledge (
                id, title, content, tags, created_at, updated_at, vault_path, source_kind,
                source_uri_normalized, content_hash, origin_run_id, origin_session_id
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                title=excluded.title,
                content=excluded.content,
                tags=excluded.tags,
                updated_at=excluded.updated_at,
                vault_path=excluded.vault_path,
                source_kind=excluded.source_kind,
                source_uri_normalized=excluded.source_uri_normalized,
                content_hash=excluded.content_hash,
                origin_run_id=excluded.origin_run_id,
                origin_session_id=excluded.origin_session_id",
            params![
                persisted_id,
                item.title,
                item.content,
                tags_json,
                item.created_at.to_rfc3339(),
                item.updated_at.to_rfc3339(),
                item.vault_path,
                item.source_kind,
                item.source_uri_normalized,
                item.content_hash,
                item.origin_run_id,
                item.origin_session_id,
            ],
        )?;

        tx.execute(
            "DELETE FROM knowledge_fts WHERE id = ?1",
            params![persisted_id],
        )?;
        tx.execute(
            "INSERT INTO knowledge_fts (id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
            params![persisted_id, item.title, item.content, tags_json],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn get_all_knowledge_items(&self) -> Result<Vec<KnowledgeItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, content, tags, created_at, updated_at, vault_path,
                    source_kind, source_uri_normalized, content_hash, origin_run_id, origin_session_id
             FROM knowledge ORDER BY updated_at DESC",
        )?;

        let mut rows = stmt.query(params![])?;
        let mut items = Vec::new();

        while let Some(row) = rows.next()? {
            let row: &rusqlite::Row = row;
            let id_str: String = row.get(0)?;
            let id = uuid::Uuid::parse_str(&id_str).unwrap_or_default();
            let title = row.get::<usize, String>(1)?;
            let content = row.get::<usize, String>(2)?;
            let tags_str = row.get::<usize, String>(3)?;
            let tags: Vec<crate::knowledge::core::Tag> =
                serde_json::from_str(&tags_str).unwrap_or_default();
            let created_at_str = row.get::<usize, String>(4)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let updated_at_str = row.get::<usize, String>(5)?;
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let vault_path = row
                .get::<usize, Option<String>>(6)?
                .filter(|path| !path.is_empty());

            items.push(KnowledgeItem {
                id,
                record_kind: crate::knowledge::core::KnowledgeRecordKind::Document,
                title,
                content,
                tags,
                retention_policy: crate::knowledge::core::RetentionPolicy::KeepForever,
                created_at,
                updated_at,
                vault_path,
                source_kind: row.get(7)?,
                source_uri_normalized: row.get(8)?,
                content_hash: row.get(9)?,
                origin_run_id: row.get(10)?,
                origin_session_id: row.get(11)?,
                origin_instance_id: None,
                origin_agent_id: None,
            });
        }

        Ok(items)
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )?;
        Ok(())
    }

    fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1 LIMIT 1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let row: &rusqlite::Row = row;
            Ok(Some(row.get::<usize, String>(0)?))
        } else {
            Ok(None)
        }
    }

    fn get_recent_workspaces(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key LIKE 'workspace_%' ORDER BY updated_at DESC LIMIT 5")?;
        let iter = stmt.query_map([], |row: &rusqlite::Row| row.get(0))?;

        let mut workspaces = Vec::new();
        for w in iter.flatten() {
            if !workspaces.contains(&w) {
                workspaces.push(w);
            }
        }
        Ok(workspaces)
    }

    fn search_knowledge(&self, query: &str) -> Result<Vec<KnowledgeItem>> {
        let conn = self.conn.lock().unwrap();
        // Fallback FTS implementation for basic term searching (mocking Vector search for now)
        let mut stmt = conn.prepare(
            "SELECT id, title, content, tags, created_at, updated_at, vault_path,
                    source_kind, source_uri_normalized, content_hash, origin_run_id, origin_session_id
             FROM knowledge
             WHERE content LIKE ?1 OR title LIKE ?1
             LIMIT 5",
        )?;

        let search_pattern = format!("%{}%", query);
        let rows = stmt.query_map(params![search_pattern], |row: &rusqlite::Row| {
            let tags_str = row.get::<usize, String>(3)?;
            let tags = serde_json::from_str(&tags_str).unwrap_or_default();

            Ok(KnowledgeItem {
                id: uuid::Uuid::parse_str(&row.get::<usize, String>(0)?).unwrap_or_default(),
                record_kind: crate::knowledge::core::KnowledgeRecordKind::Document,
                title: row.get::<usize, String>(1)?,
                content: row.get::<usize, String>(2)?,
                tags,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<usize, String>(4)?)
                    .unwrap()
                    .into(),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<usize, String>(5)?)
                    .unwrap()
                    .into(),
                retention_policy: crate::knowledge::core::RetentionPolicy::KeepForever,
                vault_path: row.get(6)?,
                source_kind: row.get(7)?,
                source_uri_normalized: row.get(8)?,
                content_hash: row.get(9)?,
                origin_run_id: row.get(10)?,
                origin_session_id: row.get(11)?,
                origin_instance_id: None,
                origin_agent_id: None,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    fn search_knowledge_fts(&self, query: &str, limit: u32) -> Result<Vec<KnowledgeItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT k.id, k.title, k.content, k.tags, k.created_at, k.updated_at, k.vault_path,
                    k.source_kind, k.source_uri_normalized, k.content_hash,
                    k.origin_run_id, k.origin_session_id
             FROM knowledge k
             JOIN knowledge_fts ON knowledge_fts.id = k.id
             WHERE knowledge_fts MATCH ?1
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, limit], |row: &rusqlite::Row| {
            let id_str = row.get::<usize, String>(0)?;
            let tags_str = row.get::<usize, String>(3)?;
            let tags = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(KnowledgeItem {
                id: uuid::Uuid::parse_str(&id_str).unwrap_or_default(),
                record_kind: crate::knowledge::core::KnowledgeRecordKind::Document,
                title: row.get::<usize, String>(1)?,
                content: row.get::<usize, String>(2)?,
                tags,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<usize, String>(4)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<usize, String>(5)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                retention_policy: crate::knowledge::core::RetentionPolicy::KeepForever,
                vault_path: row.get(6)?,
                source_kind: row.get(7)?,
                source_uri_normalized: row.get(8)?,
                content_hash: row.get(9)?,
                origin_run_id: row.get(10)?,
                origin_session_id: row.get(11)?,
                origin_instance_id: None,
                origin_agent_id: None,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    fn upsert_knowledge_chunks(
        &self,
        document_id: &str,
        chunks: Vec<(usize, String, Vec<f32>)>,
    ) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        tx.execute(
            "DELETE FROM knowledge_chunks WHERE document_id = ?1",
            rusqlite::params![document_id],
        )?;

        let mut stmt = tx.prepare(
            "INSERT INTO knowledge_chunks (id, document_id, chunk_index, content, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;

        for (index, content, embedding) in chunks {
            let chunk_id = uuid::Uuid::new_v4().to_string();
            let embedding_json = serde_json::to_string(&embedding).unwrap_or_default();
            stmt.execute(rusqlite::params![
                chunk_id,
                document_id,
                index,
                content,
                embedding_json,
            ])?;
        }

        drop(stmt);
        tx.commit()?;

        Ok(())
    }

    fn search_similar_chunks(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, String, f32)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT k.title, c.content, c.embedding
             FROM knowledge_chunks c
             JOIN knowledge k ON c.document_id = k.id",
        )?;

        let mut rows = stmt.query(params![])?;
        let mut results = Vec::new();

        while let Some(row) = rows.next()? {
            let row: &rusqlite::Row = row;
            let title = row.get::<usize, String>(0)?;
            let content = row.get::<usize, String>(1)?;
            let embedding_json = row.get::<usize, String>(2)?;
            let doc_embedding: Vec<f32> = serde_json::from_str(&embedding_json).unwrap_or_default();

            if doc_embedding.len() == query_embedding.len() {
                let mut dot_product = 0.0;
                let mut norm_a = 0.0;
                let mut norm_b = 0.0;
                for i in 0..query_embedding.len() {
                    dot_product += query_embedding[i] * doc_embedding[i];
                    norm_a += query_embedding[i] * query_embedding[i];
                    norm_b += doc_embedding[i] * doc_embedding[i];
                }
                let similarity = if norm_a > 0.0 && norm_b > 0.0 {
                    dot_product / (norm_a.sqrt() * norm_b.sqrt())
                } else {
                    0.0
                };
                results.push((title, content, similarity));
            }
        }

        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    fn upsert_workflow(&self, wf: &WorkflowRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO workflows (
                id, run_id, origin_kind, activation_status, name, definition, version, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(id) DO UPDATE SET
                run_id = COALESCE(excluded.run_id, workflows.run_id),
                origin_kind = excluded.origin_kind,
                activation_status = excluded.activation_status,
                name = excluded.name,
                definition = excluded.definition,
                version = excluded.version,
                updated_at = excluded.updated_at",
            params![
                wf.id,
                wf.run_id,
                wf.origin_kind,
                wf.activation_status,
                wf.name,
                wf.definition,
                wf.version,
                now
            ],
        )?;
        Ok(())
    }

    fn list_workflows(&self) -> Result<Vec<WorkflowRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, origin_kind, activation_status, name, definition, version, created_at, updated_at
             FROM workflows
             ORDER BY updated_at DESC",
        )?;
        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            Ok(WorkflowRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                origin_kind: row.get(2)?,
                activation_status: row.get(3)?,
                name: row.get(4)?,
                definition: row.get(5)?,
                version: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let mut items = Vec::new();
        for it in iter {
            items.push(it?);
        }
        Ok(items)
    }

    fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, origin_kind, activation_status, name, definition, version, created_at, updated_at
             FROM workflows
             WHERE id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![workflow_id])?;
        if let Some(row) = rows.next()? {
            let row: &rusqlite::Row = row;
            Ok(Some(WorkflowRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                origin_kind: row.get(2)?,
                activation_status: row.get(3)?,
                name: row.get(4)?,
                definition: row.get(5)?,
                version: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn delete_workflow(&self, workflow_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM workflows WHERE id = ?1", params![workflow_id])?;
        // Also clean up workflow states
        conn.execute(
            "DELETE FROM workflow_states WHERE workflow_id = ?1",
            params![workflow_id],
        )?;
        Ok(())
    }

    fn next_workflow_version_number(&self, workflow_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let next = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM workflow_versions WHERE workflow_id = ?1",
            params![workflow_id],
            |row| row.get(0),
        )?;
        Ok(next)
    }

    fn save_workflow_version(
        &self,
        version: &crate::core::models::WorkflowVersionRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workflow_versions (
                id, workflow_id, run_id, instance_id, version, definition_json,
                validation_status, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                version.id,
                version.workflow_id,
                version.run_id,
                version.instance_id,
                version.version,
                version.definition_json,
                version.validation_status,
                version.created_at,
            ],
        )?;
        Ok(())
    }

    fn get_latest_workflow_version_for_run(
        &self,
        run_id: &str,
    ) -> Result<Option<crate::core::models::WorkflowVersionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, run_id, instance_id, version, definition_json,
                    validation_status, created_at
             FROM workflow_versions
             WHERE run_id = ?1
             ORDER BY created_at DESC, version DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![run_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(crate::core::models::WorkflowVersionRecord {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                run_id: row.get(2)?,
                instance_id: row.get(3)?,
                version: row.get(4)?,
                definition_json: row.get(5)?,
                validation_status: row.get(6)?,
                created_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn get_latest_workflow_version_for_workflow(
        &self,
        workflow_id: &str,
    ) -> Result<Option<crate::core::models::WorkflowVersionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, run_id, instance_id, version, definition_json,
                    validation_status, created_at
             FROM workflow_versions
             WHERE workflow_id = ?1
             ORDER BY version DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![workflow_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(crate::core::models::WorkflowVersionRecord {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                run_id: row.get(2)?,
                instance_id: row.get(3)?,
                version: row.get(4)?,
                definition_json: row.get(5)?,
                validation_status: row.get(6)?,
                created_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn get_workflow_version(
        &self,
        version_id: &str,
    ) -> Result<Option<crate::core::models::WorkflowVersionRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, workflow_id, run_id, instance_id, version, definition_json,
                    validation_status, created_at
             FROM workflow_versions WHERE id = ?1",
            params![version_id],
            |row| {
                Ok(crate::core::models::WorkflowVersionRecord {
                    id: row.get(0)?,
                    workflow_id: row.get(1)?,
                    run_id: row.get(2)?,
                    instance_id: row.get(3)?,
                    version: row.get(4)?,
                    definition_json: row.get(5)?,
                    validation_status: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn save_workflow_execution(
        &self,
        execution: &crate::core::models::WorkflowExecutionRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workflow_executions (
                id, workflow_version_id, run_id, status, state_json, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                workflow_version_id = excluded.workflow_version_id,
                run_id = excluded.run_id,
                status = excluded.status,
                state_json = excluded.state_json,
                updated_at = excluded.updated_at",
            params![
                execution.id,
                execution.workflow_version_id,
                execution.run_id,
                execution.status,
                execution.state_json,
                execution.updated_at,
            ],
        )?;
        Ok(())
    }

    fn get_latest_workflow_execution_for_version(
        &self,
        workflow_version_id: &str,
    ) -> Result<Option<crate::core::models::WorkflowExecutionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workflow_version_id, run_id, status, state_json, updated_at
             FROM workflow_executions
             WHERE workflow_version_id = ?1
             ORDER BY updated_at DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![workflow_version_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(crate::core::models::WorkflowExecutionRecord {
                id: row.get(0)?,
                workflow_version_id: row.get(1)?,
                run_id: row.get(2)?,
                status: row.get(3)?,
                state_json: row.get(4)?,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn get_workflow_execution(
        &self,
        execution_id: &str,
    ) -> Result<Option<crate::core::models::WorkflowExecutionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workflow_version_id, run_id, status, state_json, updated_at
             FROM workflow_executions
             WHERE id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![execution_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(crate::core::models::WorkflowExecutionRecord {
                id: row.get(0)?,
                workflow_version_id: row.get(1)?,
                run_id: row.get(2)?,
                status: row.get(3)?,
                state_json: row.get(4)?,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn save_workflow_state(
        &self,
        state: &crate::application::iflow_engine::engine::WorkflowState,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let state_json = serde_json::to_string(state)?;
        conn.execute(
            "INSERT OR REPLACE INTO workflow_states (execution_id, workflow_id, state_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![state.execution_id, state.workflow_id, state_json, now],
        )?;
        Ok(())
    }

    fn load_workflow_state(
        &self,
        execution_id: &str,
    ) -> Result<Option<crate::application::iflow_engine::engine::WorkflowState>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT state_json FROM workflow_states WHERE execution_id = ?1 LIMIT 1")?;
        let mut rows = stmt.query(params![execution_id])?;
        if let Some(row) = rows.next()? {
            let row: &rusqlite::Row = row;
            let state_json: String = row.get(0)?;
            let state = serde_json::from_str(&state_json)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    fn create_orchestration_run(
        &self,
        run: &crate::core::models::OrchestrationRunRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO orchestration_runs (
                id, session_id, instance_id, initiated_by, goal, mode, status,
                workflow_id, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                run.id,
                run.session_id,
                run.instance_id,
                run.initiated_by,
                run.goal,
                run.mode,
                run.status,
                run.workflow_id,
                run.created_at,
                run.updated_at,
            ],
        )?;
        Ok(())
    }

    fn get_orchestration_run(
        &self,
        run_id: &str,
    ) -> Result<Option<crate::core::models::OrchestrationRunRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, instance_id, initiated_by, goal, mode, status,
                    workflow_id, created_at, updated_at
             FROM orchestration_runs
             WHERE id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![run_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(crate::core::models::OrchestrationRunRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                instance_id: row.get(2)?,
                initiated_by: row.get(3)?,
                goal: row.get(4)?,
                mode: row.get(5)?,
                status: row.get(6)?,
                workflow_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn update_orchestration_run_status(
        &self,
        run_id: &str,
        status: &str,
        workflow_id: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE orchestration_runs
             SET status = ?1, workflow_id = COALESCE(?2, workflow_id), updated_at = ?3
             WHERE id = ?4",
            params![status, workflow_id, chrono::Utc::now().to_rfc3339(), run_id],
        )?;
        Ok(())
    }

    fn list_recent_orchestration_runs(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::core::models::OrchestrationRunRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, instance_id, initiated_by, goal, mode, status,
                    workflow_id, created_at, updated_at
             FROM orchestration_runs
             ORDER BY updated_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row: &rusqlite::Row| {
            Ok(crate::core::models::OrchestrationRunRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                instance_id: row.get(2)?,
                initiated_by: row.get(3)?,
                goal: row.get(4)?,
                mode: row.get(5)?,
                status: row.get(6)?,
                workflow_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        let mut runs = Vec::new();
        for row in rows {
            runs.push(row?);
        }
        Ok(runs)
    }

    fn insert_run_event(&self, event: &crate::core::models::RunEventRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO run_events (
                id, run_id, event_type, actor_type, actor_id, task_id, payload, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.id,
                event.run_id,
                event.event_type,
                event.actor_type,
                event.actor_id,
                event.task_id,
                event.payload,
                event.created_at,
            ],
        )?;
        Ok(())
    }

    fn list_recent_run_events(
        &self,
        run_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<crate::core::models::RunEventRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut events = Vec::new();
        if let Some(run_id) = run_id {
            let mut stmt = conn.prepare(
                "SELECT id, run_id, event_type, actor_type, actor_id, task_id, payload, created_at
                 FROM run_events
                 WHERE run_id = ?1
                 ORDER BY created_at DESC
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![run_id, limit], |row: &rusqlite::Row| {
                Ok(crate::core::models::RunEventRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    event_type: row.get(2)?,
                    actor_type: row.get(3)?,
                    actor_id: row.get(4)?,
                    task_id: row.get(5)?,
                    payload: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?;
            for row in rows {
                events.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, run_id, event_type, actor_type, actor_id, task_id, payload, created_at
                 FROM run_events
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit], |row: &rusqlite::Row| {
                Ok(crate::core::models::RunEventRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    event_type: row.get(2)?,
                    actor_type: row.get(3)?,
                    actor_id: row.get(4)?,
                    task_id: row.get(5)?,
                    payload: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?;
            for row in rows {
                events.push(row?);
            }
        }
        Ok(events)
    }

    fn create_approval_request(
        &self,
        request: &crate::core::models::ApprovalRequestRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO approval_requests (
                id, run_id, operation, requested_by, status, resolved_by,
                decision_reason, created_at, resolved_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                request.id,
                request.run_id,
                request.operation,
                request.requested_by,
                request.status,
                request.resolved_by,
                request.decision_reason,
                request.created_at,
                request.resolved_at,
            ],
        )?;
        Ok(())
    }

    fn list_pending_approval_requests(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::core::models::ApprovalRequestRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, operation, requested_by, status, resolved_by,
                    decision_reason, created_at, resolved_at
             FROM approval_requests
             WHERE status = 'pending'
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row: &rusqlite::Row| {
            Ok(crate::core::models::ApprovalRequestRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                operation: row.get(2)?,
                requested_by: row.get(3)?,
                status: row.get(4)?,
                resolved_by: row.get(5)?,
                decision_reason: row.get(6)?,
                created_at: row.get(7)?,
                resolved_at: row.get(8)?,
            })
        })?;
        let mut requests = Vec::new();
        for row in rows {
            requests.push(row?);
        }
        Ok(requests)
    }

    fn get_approval_request_for_operation(
        &self,
        run_id: &str,
        operation: &str,
    ) -> Result<Option<crate::core::models::ApprovalRequestRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, operation, requested_by, status, resolved_by,
                    decision_reason, created_at, resolved_at
             FROM approval_requests
             WHERE run_id = ?1 AND operation = ?2
             ORDER BY created_at DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![run_id, operation])?;
        if let Some(row) = rows.next()? {
            Ok(Some(crate::core::models::ApprovalRequestRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                operation: row.get(2)?,
                requested_by: row.get(3)?,
                status: row.get(4)?,
                resolved_by: row.get(5)?,
                decision_reason: row.get(6)?,
                created_at: row.get(7)?,
                resolved_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn resolve_approval_request(
        &self,
        request_id: &str,
        status: &str,
        resolved_by: Option<&str>,
        reason: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE approval_requests
             SET status = ?1, resolved_by = ?2, decision_reason = ?3, resolved_at = ?4
             WHERE id = ?5",
            params![
                status,
                resolved_by,
                reason,
                chrono::Utc::now().to_rfc3339(),
                request_id
            ],
        )?;
        Ok(())
    }

    fn upsert_tool_invocation(
        &self,
        invocation: &crate::core::models::ToolInvocationRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(
            "INSERT INTO tool_invocations
                (id, run_id, tool_name, sealed_payload_json, payload_hash, mode, status,
                 approval_request_id, result, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                approval_request_id = COALESCE(excluded.approval_request_id, tool_invocations.approval_request_id),
                result = COALESCE(excluded.result, tool_invocations.result),
                updated_at = excluded.updated_at
             WHERE tool_invocations.payload_hash = excluded.payload_hash
               AND tool_invocations.tool_name = excluded.tool_name
               AND tool_invocations.run_id = excluded.run_id",
            params![
                invocation.id,
                invocation.run_id,
                invocation.tool_name,
                invocation.sealed_payload_json,
                invocation.payload_hash,
                invocation.mode,
                invocation.status,
                invocation.approval_request_id,
                invocation.result,
                invocation.created_at,
                invocation.updated_at,
            ],
        )?;
        if changed != 1 {
            return Err(anyhow::anyhow!(
                "Invocation id already exists with a different sealed payload or tool."
            ));
        }
        Ok(())
    }

    fn update_tool_invocation_status(
        &self,
        invocation_id: &str,
        status: &str,
        approval_request_id: Option<&str>,
        result: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tool_invocations
             SET status = ?2,
                 approval_request_id = COALESCE(?3, approval_request_id),
                 result = COALESCE(?4, result),
                 updated_at = ?5
             WHERE id = ?1",
            params![
                invocation_id,
                status,
                approval_request_id,
                result,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    fn get_next_approved_tool_invocation_for_run(
        &self,
        run_id: &str,
    ) -> Result<Option<crate::core::models::ToolInvocationRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT i.id, i.run_id, i.tool_name, i.sealed_payload_json, i.payload_hash,
                    i.mode, i.status, i.approval_request_id, i.result, i.created_at, i.updated_at
             FROM tool_invocations i
             JOIN approval_requests a ON a.id = i.approval_request_id
             WHERE i.run_id = ?1 AND i.status = 'awaiting_approval' AND a.status = 'approved'
             ORDER BY i.created_at ASC LIMIT 1",
            params![run_id],
            |row| {
                Ok(crate::core::models::ToolInvocationRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    tool_name: row.get(2)?,
                    sealed_payload_json: row.get(3)?,
                    payload_hash: row.get(4)?,
                    mode: row.get(5)?,
                    status: row.get(6)?,
                    approval_request_id: row.get(7)?,
                    result: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn insert_mode_transition(
        &self,
        transition: &crate::core::models::ModeTransitionRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO mode_transitions (
                id, instance_id, run_id, actor_id, from_mode, to_mode,
                reason, policy_version, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                transition.id,
                transition.instance_id,
                transition.run_id,
                transition.actor_id,
                transition.from_mode,
                transition.to_mode,
                transition.reason,
                transition.policy_version,
                transition.created_at,
            ],
        )?;
        Ok(())
    }

    fn list_recent_mode_transitions(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::core::models::ModeTransitionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, instance_id, run_id, actor_id, from_mode, to_mode,
                    reason, policy_version, created_at
             FROM mode_transitions
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row: &rusqlite::Row| {
            Ok(crate::core::models::ModeTransitionRecord {
                id: row.get(0)?,
                instance_id: row.get(1)?,
                run_id: row.get(2)?,
                actor_id: row.get(3)?,
                from_mode: row.get(4)?,
                to_mode: row.get(5)?,
                reason: row.get(6)?,
                policy_version: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        let mut transitions = Vec::new();
        for row in rows {
            transitions.push(row?);
        }
        Ok(transitions)
    }

    fn insert_audit_log(
        &self,
        event: &crate::infrastructure::security::audit::AuditEvent,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_log (id, timestamp, user_id, action, resource, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                event.timestamp.to_rfc3339(),
                event.user_id,
                event.action,
                event.resource,
                event.details
            ],
        )?;
        Ok(())
    }

    fn list_recent_audit_logs(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::infrastructure::security::audit::AuditEvent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT timestamp, action, user_id, resource, details
             FROM audit_log
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;
        let iter = stmt.query_map(params![limit], |row: &rusqlite::Row| {
            let timestamp: String = row.get(0)?;
            Ok(crate::infrastructure::security::audit::AuditEvent {
                timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp)
                    .map(|value| value.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                action: row.get(1)?,
                user_id: row.get(2)?,
                resource: row.get(3)?,
                details: row.get(4)?,
            })
        })?;
        let mut events = Vec::new();
        for event in iter {
            events.push(event?);
        }
        Ok(events)
    }

    fn create_role(&self, role: &crate::application::teams::role::Role) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let permissions = normalize_permission_list(role.permissions.as_deref());
        let capabilities = normalize_permission_list(role.capabilities.as_deref());
        conn.execute(
            "INSERT OR IGNORE INTO roles (id, team_id, name, permissions, capabilities)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![role.id, role.team_id, role.name, permissions, capabilities],
        )?;
        conn.execute(
            "UPDATE roles
             SET permissions = ?1, capabilities = ?2
             WHERE id = ?3 AND (permissions = 'all' OR capabilities = 'all')",
            params![permissions, capabilities, role.id],
        )?;
        Ok(())
    }

    fn update_role_permissions(&self, role_id: &str, permissions: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE roles SET permissions = ?1 WHERE id = ?2",
            params![permissions, role_id],
        )?;
        Ok(())
    }

    fn check_role_permission(&self, role_id: &str, required_permission: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT permissions FROM roles WHERE id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![role_id])?;
        if let Some(row) = rows.next()? {
            let perms_str: String = row.get(0)?;
            let perms: Vec<String> = if perms_str.trim() == "all" {
                vec!["all".to_string()]
            } else {
                serde_json::from_str(&perms_str).unwrap_or_default()
            };
            Ok(perms.iter().any(|p| p == required_permission || p == "all"))
        } else {
            Ok(false)
        }
    }

    fn ensure_local_security_owner(&self, actor_id: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT OR IGNORE INTO security_actors (id, display_name, actor_kind, created_at)
             VALUES (?1, 'Local Desktop User', 'user', ?2)",
            params![actor_id, now],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO security_roles (id, name, created_at)
             VALUES ('security-owner', 'Owner', ?1)",
            params![now],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO security_permissions (id, code)
             VALUES ('security-permission-all', 'all')",
            [],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO security_actor_roles (actor_id, role_id)
             VALUES (?1, 'security-owner')",
            params![actor_id],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO security_role_permissions (role_id, permission_id)
             VALUES ('security-owner', 'security-permission-all')",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn check_actor_permission(&self, actor_id: &str, required_permission: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT p.code
             FROM security_actor_roles ar
             JOIN security_role_permissions rp ON rp.role_id = ar.role_id
             JOIN security_permissions p ON p.id = rp.permission_id
             WHERE ar.actor_id = ?1",
        )?;
        let permissions = stmt
            .query_map(params![actor_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(permissions
            .iter()
            .any(|permission| permission == "all" || permission == required_permission))
    }

    fn upsert_mcp_tool(&self, tool: &crate::infrastructure::mcp::registry::McpTool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let args_json = serde_json::to_string(&tool.args).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO mcp_tools (id, server_id, name, description, version, command, args, input_schema, is_active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
             server_id=excluded.server_id, name=excluded.name, description=excluded.description, version=excluded.version,
             command=excluded.command, args=excluded.args, input_schema=excluded.input_schema, is_active=excluded.is_active",
            rusqlite::params![
                tool.id, tool.server_id, tool.name, tool.description, tool.version, tool.command, args_json, tool.input_schema, tool.is_active
            ],
        )?;
        Ok(())
    }

    fn get_mcp_tool(
        &self,
        id: &str,
    ) -> Result<Option<crate::infrastructure::mcp::registry::McpTool>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, server_id, name, description, version, command, args, input_schema, is_active FROM mcp_tools WHERE id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![id])?;
        if let Some(row) = rows.next()? {
            let args_json: String = row.get(6)?;
            let args: Vec<String> = serde_json::from_str(&args_json).unwrap_or_default();
            Ok(Some(crate::infrastructure::mcp::registry::McpTool {
                id: row.get(0)?,
                server_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                version: row.get(4)?,
                command: row.get(5)?,
                args,
                input_schema: row.get(7)?,
                is_active: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn list_mcp_tools(&self) -> Result<Vec<crate::infrastructure::mcp::registry::McpTool>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, server_id, name, description, version, command, args, input_schema, is_active FROM mcp_tools")?;
        let tool_iter = stmt.query_map(rusqlite::params![], |row| {
            let args_json: String = row.get(6)?;
            let args: Vec<String> = serde_json::from_str(&args_json).unwrap_or_default();
            Ok(crate::infrastructure::mcp::registry::McpTool {
                id: row.get(0)?,
                server_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                version: row.get(4)?,
                command: row.get(5)?,
                args,
                input_schema: row.get(7)?,
                is_active: row.get(8)?,
            })
        })?;
        let mut tools = Vec::new();
        for tool in tool_iter {
            tools.push(tool?);
        }
        Ok(tools)
    }

    fn delete_mcp_tool(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM mcp_tools WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    fn upsert_mcp_server(
        &self,
        server: &crate::infrastructure::mcp::registry::McpServerRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let args_json = serde_json::to_string(&server.args).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO mcp_servers (
                id, name, transport, command, args, endpoint, env_secret_refs,
                header_secret_refs, source_kind, is_enabled, health_status,
                last_error, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(id) DO UPDATE SET
                name=excluded.name, transport=excluded.transport, command=excluded.command,
                args=excluded.args, endpoint=excluded.endpoint,
                env_secret_refs=excluded.env_secret_refs,
                header_secret_refs=excluded.header_secret_refs,
                source_kind=excluded.source_kind, is_enabled=excluded.is_enabled,
                health_status=excluded.health_status, last_error=excluded.last_error,
                updated_at=excluded.updated_at",
            params![
                server.id,
                server.name,
                server.transport,
                server.command,
                args_json,
                server.endpoint,
                server.env_secret_refs,
                server.header_secret_refs,
                server.source_kind,
                server.is_enabled,
                server.health_status,
                server.last_error,
                server.created_at,
                server.updated_at,
            ],
        )?;
        Ok(())
    }

    fn list_mcp_servers(
        &self,
    ) -> Result<Vec<crate::infrastructure::mcp::registry::McpServerRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, transport, command, args, endpoint, env_secret_refs,
                    header_secret_refs, source_kind, is_enabled, health_status,
                    last_error, created_at, updated_at
             FROM mcp_servers ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let args_json: String = row.get(4)?;
            Ok(crate::infrastructure::mcp::registry::McpServerRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                transport: row.get(2)?,
                command: row.get(3)?,
                args: serde_json::from_str(&args_json).unwrap_or_default(),
                endpoint: row.get(5)?,
                env_secret_refs: row.get(6)?,
                header_secret_refs: row.get(7)?,
                source_kind: row.get(8)?,
                is_enabled: row.get(9)?,
                health_status: row.get(10)?,
                last_error: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn upsert_capability_selection(
        &self,
        selection: &crate::core::models::CapabilitySelectionRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO capability_selections (
                id, scope_kind, scope_id, capability_kind, capability_id,
                enabled, selected_by, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(scope_kind, scope_id, capability_kind, capability_id)
             DO UPDATE SET enabled=excluded.enabled, selected_by=excluded.selected_by,
                           updated_at=excluded.updated_at",
            params![
                selection.id,
                selection.scope_kind,
                selection.scope_id,
                selection.capability_kind,
                selection.capability_id,
                selection.enabled,
                selection.selected_by,
                selection.created_at,
                selection.updated_at,
            ],
        )?;
        Ok(())
    }

    fn list_capability_selections(
        &self,
        scope_kind: &str,
        scope_id: &str,
    ) -> Result<Vec<crate::core::models::CapabilitySelectionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, scope_kind, scope_id, capability_kind, capability_id,
                    enabled, selected_by, created_at, updated_at
             FROM capability_selections
             WHERE scope_kind = ?1 AND scope_id = ?2
             ORDER BY capability_kind, capability_id",
        )?;
        let rows = stmt.query_map(params![scope_kind, scope_id], |row| {
            Ok(crate::core::models::CapabilitySelectionRecord {
                id: row.get(0)?,
                scope_kind: row.get(1)?,
                scope_id: row.get(2)?,
                capability_kind: row.get(3)?,
                capability_id: row.get(4)?,
                enabled: row.get(5)?,
                selected_by: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_llm_context_snapshot(
        &self,
        snapshot: &crate::core::models::LlmContextSnapshotRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO llm_context_snapshots (
                id, run_id, session_id, instance_id, agent_id, mode,
                selected_capabilities_json, context_hash, character_count, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                snapshot.id,
                snapshot.run_id,
                snapshot.session_id,
                snapshot.instance_id,
                snapshot.agent_id,
                snapshot.mode,
                snapshot.selected_capabilities_json,
                snapshot.context_hash,
                snapshot.character_count as i64,
                snapshot.created_at,
            ],
        )?;
        Ok(())
    }

    fn insert_llm_context_source(
        &self,
        source: &crate::core::models::LlmContextSourceRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO llm_context_sources (
                id, snapshot_id, source_kind, source_id, source_hash, rank,
                character_count, trust_level, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                source.id,
                source.snapshot_id,
                source.source_kind,
                source.source_id,
                source.source_hash,
                source.rank,
                source.character_count as i64,
                source.trust_level,
                source.created_at,
            ],
        )?;
        Ok(())
    }

    fn list_recent_llm_context_snapshots(
        &self,
        run_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<crate::core::models::LlmContextSnapshotRecord>> {
        let conn = self.conn.lock().unwrap();
        let sql = if run_id.is_some() {
            "SELECT id, run_id, session_id, instance_id, agent_id, mode,
                    selected_capabilities_json, context_hash, character_count, created_at
             FROM llm_context_snapshots
             WHERE run_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2"
        } else {
            "SELECT id, run_id, session_id, instance_id, agent_id, mode,
                    selected_capabilities_json, context_hash, character_count, created_at
             FROM llm_context_snapshots
             ORDER BY created_at DESC
             LIMIT ?2"
        };
        let mut stmt = conn.prepare(sql)?;
        let map_row = |row: &rusqlite::Row| {
            Ok(crate::core::models::LlmContextSnapshotRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                session_id: row.get(2)?,
                instance_id: row.get(3)?,
                agent_id: row.get(4)?,
                mode: row.get(5)?,
                selected_capabilities_json: row.get(6)?,
                context_hash: row.get(7)?,
                character_count: row.get::<_, i64>(8)? as usize,
                created_at: row.get(9)?,
            })
        };
        let rows = if let Some(run_id) = run_id {
            stmt.query_map(params![run_id, limit], map_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![Option::<String>::None, limit], map_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    fn list_llm_context_sources(
        &self,
        snapshot_id: &str,
    ) -> Result<Vec<crate::core::models::LlmContextSourceRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, snapshot_id, source_kind, source_id, source_hash, rank,
                    character_count, trust_level, created_at
             FROM llm_context_sources
             WHERE snapshot_id = ?1
             ORDER BY CASE WHEN rank IS NULL THEN 1 ELSE 0 END, rank ASC, created_at ASC",
        )?;
        let rows = stmt.query_map(params![snapshot_id], |row| {
            Ok(crate::core::models::LlmContextSourceRecord {
                id: row.get(0)?,
                snapshot_id: row.get(1)?,
                source_kind: row.get(2)?,
                source_id: row.get(3)?,
                source_hash: row.get(4)?,
                rank: row.get(5)?,
                character_count: row.get::<_, i64>(6)? as usize,
                trust_level: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_artifact(&self, artifact: &crate::core::models::ArtifactRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO artifacts (
                id, run_id, session_id, instance_id, agent_id, invocation_id,
                artifact_kind, path, content_hash, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                artifact.id,
                artifact.run_id,
                artifact.session_id,
                artifact.instance_id,
                artifact.agent_id,
                artifact.invocation_id,
                artifact.artifact_kind,
                artifact.path,
                artifact.content_hash,
                artifact.created_at,
            ],
        )?;
        Ok(())
    }

    fn list_artifacts_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<crate::core::models::ArtifactRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, session_id, instance_id, agent_id, invocation_id,
                    artifact_kind, path, content_hash, created_at
             FROM artifacts
             WHERE run_id = ?1
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![run_id], |row| {
            Ok(crate::core::models::ArtifactRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                session_id: row.get(2)?,
                instance_id: row.get(3)?,
                agent_id: row.get(4)?,
                invocation_id: row.get(5)?,
                artifact_kind: row.get(6)?,
                path: row.get(7)?,
                content_hash: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn upsert_knowledge_entry(
        &self,
        entry: &crate::core::models::knowledge::KnowledgeEntry,
    ) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let tags_json = serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string());
        let created_at_str = entry.created_at.to_rfc3339();

        tx.execute(
            "INSERT INTO knowledge_entries (
                id, agent_id, session_id, instance_id, run_id, title, content, tags, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
             session_id=excluded.session_id, instance_id=excluded.instance_id, run_id=excluded.run_id,
             title=excluded.title, content=excluded.content, tags=excluded.tags",
            rusqlite::params![
                entry.id,
                entry.agent_id,
                entry.session_id,
                entry.instance_id,
                entry.run_id,
                entry.title,
                entry.content,
                tags_json,
                created_at_str
            ],
        )?;
        tx.execute(
            "DELETE FROM knowledge_entries_fts WHERE id = ?1",
            rusqlite::params![entry.id],
        )?;
        tx.execute(
            "INSERT INTO knowledge_entries_fts (id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                entry.id,
                entry.title,
                entry.content,
                serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string())
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn get_knowledge_entry(
        &self,
        id: &str,
    ) -> Result<Option<crate::core::models::knowledge::KnowledgeEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, session_id, instance_id, run_id, title, content, tags, created_at
             FROM knowledge_entries WHERE id = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![id])?;
        if let Some(row) = rows.next()? {
            let tags_json: String = row.get(7)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let created_at_str: String = row.get(8)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            Ok(Some(crate::core::models::knowledge::KnowledgeEntry {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                session_id: row.get(2)?,
                instance_id: row.get(3)?,
                run_id: row.get(4)?,
                title: row.get(5)?,
                content: row.get(6)?,
                tags,
                created_at,
            }))
        } else {
            Ok(None)
        }
    }

    fn get_all_knowledge_entries(
        &self,
    ) -> Result<Vec<crate::core::models::knowledge::KnowledgeEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, session_id, instance_id, run_id, title, content, tags, created_at
             FROM knowledge_entries
             ORDER BY created_at DESC",
        )?;
        let entries = stmt.query_map([], |row| {
            let tags_json: String = row.get(7)?;
            let created_at_str: String = row.get(8)?;
            Ok(crate::core::models::knowledge::KnowledgeEntry {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                session_id: row.get(2)?,
                instance_id: row.get(3)?,
                run_id: row.get(4)?,
                title: row.get(5)?,
                content: row.get(6)?,
                tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        })?;
        entries
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn search_knowledge_entries_fts(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<crate::core::models::knowledge::KnowledgeEntry>> {
        let conn = self.conn.lock().unwrap();
        // Use FTS5 for search
        let mut stmt = conn.prepare(
            "SELECT e.id, e.agent_id, e.session_id, e.instance_id, e.run_id,
                    e.title, e.content, e.tags, e.created_at
             FROM knowledge_entries e
             JOIN knowledge_entries_fts fts ON e.id = fts.id
             WHERE knowledge_entries_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        // SQLite FTS5 requires query formatting, but for simplicity, just pass the raw query if it's safe,
        // or wrap it in quotes. A better way is to clean the query.
        let fts_query = format!("\"{}\"", query.replace("\"", ""));

        let entry_iter = stmt.query_map(rusqlite::params![fts_query, limit], |row| {
            let tags_json: String = row.get(7)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let created_at_str: String = row.get(8)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            Ok(crate::core::models::knowledge::KnowledgeEntry {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                session_id: row.get(2)?,
                instance_id: row.get(3)?,
                run_id: row.get(4)?,
                title: row.get(5)?,
                content: row.get(6)?,
                tags,
                created_at,
            })
        })?;

        let mut entries = Vec::new();
        for entry in entry_iter {
            entries.push(entry?);
        }
        Ok(entries)
    }

    fn upsert_cross_team_case(
        &self,
        correlation_id: &str,
        owner_instance_id: &str,
        target_instance_id: &str,
        latest_event_type: &str,
        summary: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO cross_team_cases
                (correlation_id, owner_instance_id, target_instance_id, latest_event_type, summary, created_at, updated_at)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(correlation_id) DO UPDATE SET
                owner_instance_id = excluded.owner_instance_id,
                target_instance_id = excluded.target_instance_id,
                latest_event_type = excluded.latest_event_type,
                summary = excluded.summary,
                updated_at = excluded.updated_at",
            params![
                correlation_id,
                owner_instance_id,
                target_instance_id,
                latest_event_type,
                summary,
                now,
                now
            ],
        )?;
        Ok(())
    }

    fn insert_cross_team_case_event(
        &self,
        event: &crate::core::models::CrossTeamCaseEventRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO cross_team_case_events
                (id, correlation_id, from_instance_id, reply_to_instance_id, event_type, summary, payload, created_at)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.id,
                event.correlation_id,
                event.from_instance_id,
                event.reply_to_instance_id,
                event.event_type,
                event.summary,
                event.payload,
                event.created_at
            ],
        )?;
        Ok(())
    }

    fn list_cross_team_cases(
        &self,
        instance_id: &str,
        limit: u32,
    ) -> Result<Vec<crate::core::models::CrossTeamCaseRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT correlation_id, owner_instance_id, target_instance_id, latest_event_type, summary, created_at, updated_at
             FROM cross_team_cases
             WHERE owner_instance_id = ?1 OR target_instance_id = ?1
             ORDER BY updated_at DESC
             LIMIT ?2",
        )?;
        let iter = stmt.query_map(params![instance_id, limit], |row: &rusqlite::Row| {
            Ok(crate::core::models::CrossTeamCaseRecord {
                correlation_id: row.get(0)?,
                owner_instance_id: row.get(1)?,
                target_instance_id: row.get(2)?,
                latest_event_type: row.get(3)?,
                summary: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        let mut cases = Vec::new();
        for c in iter {
            cases.push(c?);
        }
        Ok(cases)
    }

    fn list_cross_team_case_events(
        &self,
        correlation_id: &str,
        limit: u32,
    ) -> Result<Vec<crate::core::models::CrossTeamCaseEventRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, correlation_id, from_instance_id, reply_to_instance_id, event_type, summary, payload, created_at
             FROM cross_team_case_events
             WHERE correlation_id = ?1
             ORDER BY created_at ASC
             LIMIT ?2",
        )?;
        let iter = stmt.query_map(params![correlation_id, limit], |row: &rusqlite::Row| {
            Ok(crate::core::models::CrossTeamCaseEventRecord {
                id: row.get(0)?,
                correlation_id: row.get(1)?,
                from_instance_id: row.get(2)?,
                reply_to_instance_id: row.get(3)?,
                event_type: row.get(4)?,
                summary: row.get(5)?,
                payload: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        let mut events = Vec::new();
        for e in iter {
            events.push(e?);
        }
        Ok(events)
    }

    fn upsert_collaboration_case(
        &self,
        case: &crate::core::models::CollaborationCaseRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO collaboration_cases
                (id, legacy_correlation_id, origin_run_id, parent_case_id, owner_instance_id,
                 target_instance_id, owner_agent_id, state, priority, risk_level, objective,
                 acceptance_json, constraints_json, created_at, updated_at, resolved_at)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(id) DO UPDATE SET
                legacy_correlation_id = excluded.legacy_correlation_id,
                origin_run_id = COALESCE(excluded.origin_run_id, collaboration_cases.origin_run_id),
                parent_case_id = excluded.parent_case_id,
                owner_instance_id = excluded.owner_instance_id,
                target_instance_id = excluded.target_instance_id,
                owner_agent_id = excluded.owner_agent_id,
                state = excluded.state,
                priority = excluded.priority,
                risk_level = excluded.risk_level,
                objective = excluded.objective,
                acceptance_json = excluded.acceptance_json,
                constraints_json = excluded.constraints_json,
                updated_at = excluded.updated_at,
                resolved_at = excluded.resolved_at",
            params![
                case.id,
                case.legacy_correlation_id,
                case.origin_run_id,
                case.parent_case_id,
                case.owner_instance_id,
                case.target_instance_id,
                case.owner_agent_id,
                case.state,
                case.priority,
                case.risk_level,
                case.objective,
                case.acceptance_json,
                case.constraints_json,
                case.created_at,
                case.updated_at,
                case.resolved_at
            ],
        )?;
        Ok(())
    }

    fn get_collaboration_case(
        &self,
        case_id: &str,
    ) -> Result<Option<crate::core::models::CollaborationCaseRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, legacy_correlation_id, origin_run_id, parent_case_id, owner_instance_id,
                    target_instance_id, owner_agent_id, state, priority, risk_level, objective,
                    acceptance_json, constraints_json, created_at, updated_at, resolved_at
             FROM collaboration_cases WHERE id = ?1",
            params![case_id],
            |row| {
                Ok(crate::core::models::CollaborationCaseRecord {
                    id: row.get(0)?,
                    legacy_correlation_id: row.get(1)?,
                    origin_run_id: row.get(2)?,
                    parent_case_id: row.get(3)?,
                    owner_instance_id: row.get(4)?,
                    target_instance_id: row.get(5)?,
                    owner_agent_id: row.get(6)?,
                    state: row.get(7)?,
                    priority: row.get(8)?,
                    risk_level: row.get(9)?,
                    objective: row.get(10)?,
                    acceptance_json: row.get(11)?,
                    constraints_json: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                    resolved_at: row.get(15)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn get_collaboration_case_by_correlation_id(
        &self,
        correlation_id: &str,
    ) -> Result<Option<crate::core::models::CollaborationCaseRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, legacy_correlation_id, origin_run_id, parent_case_id, owner_instance_id,
                    target_instance_id, owner_agent_id, state, priority, risk_level, objective,
                    acceptance_json, constraints_json, created_at, updated_at, resolved_at
             FROM collaboration_cases WHERE legacy_correlation_id = ?1",
            params![correlation_id],
            |row| {
                Ok(crate::core::models::CollaborationCaseRecord {
                    id: row.get(0)?,
                    legacy_correlation_id: row.get(1)?,
                    origin_run_id: row.get(2)?,
                    parent_case_id: row.get(3)?,
                    owner_instance_id: row.get(4)?,
                    target_instance_id: row.get(5)?,
                    owner_agent_id: row.get(6)?,
                    state: row.get(7)?,
                    priority: row.get(8)?,
                    risk_level: row.get(9)?,
                    objective: row.get(10)?,
                    acceptance_json: row.get(11)?,
                    constraints_json: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                    resolved_at: row.get(15)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn get_collaboration_case_for_run(
        &self,
        run_id: &str,
    ) -> Result<Option<crate::core::models::CollaborationCaseRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT c.id, c.legacy_correlation_id, c.origin_run_id, c.parent_case_id,
                    c.owner_instance_id, c.target_instance_id, c.owner_agent_id, c.state,
                    c.priority, c.risk_level, c.objective, c.acceptance_json, c.constraints_json,
                    c.created_at, c.updated_at, c.resolved_at
             FROM collaboration_cases c
             WHERE c.origin_run_id = ?1
                OR c.id IN (SELECT case_id FROM delegated_grants WHERE run_id = ?1)
             ORDER BY c.updated_at DESC LIMIT 1",
            params![run_id],
            |row| {
                Ok(crate::core::models::CollaborationCaseRecord {
                    id: row.get(0)?,
                    legacy_correlation_id: row.get(1)?,
                    origin_run_id: row.get(2)?,
                    parent_case_id: row.get(3)?,
                    owner_instance_id: row.get(4)?,
                    target_instance_id: row.get(5)?,
                    owner_agent_id: row.get(6)?,
                    state: row.get(7)?,
                    priority: row.get(8)?,
                    risk_level: row.get(9)?,
                    objective: row.get(10)?,
                    acceptance_json: row.get(11)?,
                    constraints_json: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                    resolved_at: row.get(15)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn list_recent_collaboration_cases(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::core::models::CollaborationCaseRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, legacy_correlation_id, origin_run_id, parent_case_id, owner_instance_id,
                    target_instance_id, owner_agent_id, state, priority, risk_level, objective,
                    acceptance_json, constraints_json, created_at, updated_at, resolved_at
             FROM collaboration_cases ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(crate::core::models::CollaborationCaseRecord {
                id: row.get(0)?,
                legacy_correlation_id: row.get(1)?,
                origin_run_id: row.get(2)?,
                parent_case_id: row.get(3)?,
                owner_instance_id: row.get(4)?,
                target_instance_id: row.get(5)?,
                owner_agent_id: row.get(6)?,
                state: row.get(7)?,
                priority: row.get(8)?,
                risk_level: row.get(9)?,
                objective: row.get(10)?,
                acceptance_json: row.get(11)?,
                constraints_json: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
                resolved_at: row.get(15)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn update_collaboration_case_state(&self, case_id: &str, state: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE collaboration_cases
             SET state = ?2, updated_at = ?3,
                 resolved_at = CASE WHEN ?2 IN ('completed', 'cancelled', 'rejected')
                                    THEN ?3 ELSE resolved_at END
             WHERE id = ?1",
            params![case_id, state, now],
        )?;
        Ok(())
    }

    fn insert_handoff_package(
        &self,
        handoff: &crate::core::models::HandoffPackageRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO handoff_packages
                (id, case_id, run_id, from_instance_id, to_instance_id, from_agent_id,
                 to_agent_id, objective, acceptance_json, constraints_json, context_refs_json,
                 status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                handoff.id,
                handoff.case_id,
                handoff.run_id,
                handoff.from_instance_id,
                handoff.to_instance_id,
                handoff.from_agent_id,
                handoff.to_agent_id,
                handoff.objective,
                handoff.acceptance_json,
                handoff.constraints_json,
                handoff.context_refs_json,
                handoff.status,
                handoff.created_at
            ],
        )?;
        Ok(())
    }

    fn get_latest_handoff_for_case(
        &self,
        case_id: &str,
    ) -> Result<Option<crate::core::models::HandoffPackageRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, case_id, run_id, from_instance_id, to_instance_id, from_agent_id,
                    to_agent_id, objective, acceptance_json, constraints_json, context_refs_json,
                    status, created_at
             FROM handoff_packages WHERE case_id = ?1 ORDER BY created_at DESC LIMIT 1",
            params![case_id],
            |row| {
                Ok(crate::core::models::HandoffPackageRecord {
                    id: row.get(0)?,
                    case_id: row.get(1)?,
                    run_id: row.get(2)?,
                    from_instance_id: row.get(3)?,
                    to_instance_id: row.get(4)?,
                    from_agent_id: row.get(5)?,
                    to_agent_id: row.get(6)?,
                    objective: row.get(7)?,
                    acceptance_json: row.get(8)?,
                    constraints_json: row.get(9)?,
                    context_refs_json: row.get(10)?,
                    status: row.get(11)?,
                    created_at: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn insert_case_readback(
        &self,
        readback: &crate::core::models::CaseReadbackRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO case_readbacks
                (id, case_id, handoff_id, agent_id, understanding, assumptions_json,
                 questions_json, status, accepted_by, created_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                readback.id,
                readback.case_id,
                readback.handoff_id,
                readback.agent_id,
                readback.understanding,
                readback.assumptions_json,
                readback.questions_json,
                readback.status,
                readback.accepted_by,
                readback.created_at,
                readback.resolved_at
            ],
        )?;
        Ok(())
    }

    fn list_case_readbacks(
        &self,
        case_id: &str,
    ) -> Result<Vec<crate::core::models::CaseReadbackRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, case_id, handoff_id, agent_id, understanding, assumptions_json,
                    questions_json, status, accepted_by, created_at, resolved_at
             FROM case_readbacks WHERE case_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![case_id], |row| {
            Ok(crate::core::models::CaseReadbackRecord {
                id: row.get(0)?,
                case_id: row.get(1)?,
                handoff_id: row.get(2)?,
                agent_id: row.get(3)?,
                understanding: row.get(4)?,
                assumptions_json: row.get(5)?,
                questions_json: row.get(6)?,
                status: row.get(7)?,
                accepted_by: row.get(8)?,
                created_at: row.get(9)?,
                resolved_at: row.get(10)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn resolve_case_readback(
        &self,
        readback_id: &str,
        status: &str,
        accepted_by: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE case_readbacks
             SET status = ?2, accepted_by = ?3, resolved_at = ?4
             WHERE id = ?1",
            params![
                readback_id,
                status,
                accepted_by,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    fn insert_case_decision(
        &self,
        decision: &crate::core::models::CaseDecisionRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO case_decisions
                (id, case_id, run_id, author_agent_id, decision, rationale, alternatives_json,
                 evidence_refs_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                decision.id,
                decision.case_id,
                decision.run_id,
                decision.author_agent_id,
                decision.decision,
                decision.rationale,
                decision.alternatives_json,
                decision.evidence_refs_json,
                decision.created_at
            ],
        )?;
        Ok(())
    }

    fn list_case_decisions(
        &self,
        case_id: &str,
    ) -> Result<Vec<crate::core::models::CaseDecisionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, case_id, run_id, author_agent_id, decision, rationale,
                    alternatives_json, evidence_refs_json, created_at
             FROM case_decisions WHERE case_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![case_id], |row| {
            Ok(crate::core::models::CaseDecisionRecord {
                id: row.get(0)?,
                case_id: row.get(1)?,
                run_id: row.get(2)?,
                author_agent_id: row.get(3)?,
                decision: row.get(4)?,
                rationale: row.get(5)?,
                alternatives_json: row.get(6)?,
                evidence_refs_json: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_case_deliverable(
        &self,
        deliverable: &crate::core::models::CaseDeliverableRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO case_deliverables
                (id, case_id, run_id, agent_id, title, artifact_refs_json,
                 acceptance_evidence_json, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                deliverable.id,
                deliverable.case_id,
                deliverable.run_id,
                deliverable.agent_id,
                deliverable.title,
                deliverable.artifact_refs_json,
                deliverable.acceptance_evidence_json,
                deliverable.status,
                deliverable.created_at,
                deliverable.updated_at
            ],
        )?;
        Ok(())
    }

    fn list_case_deliverables(
        &self,
        case_id: &str,
    ) -> Result<Vec<crate::core::models::CaseDeliverableRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, case_id, run_id, agent_id, title, artifact_refs_json,
                    acceptance_evidence_json, status, created_at, updated_at
             FROM case_deliverables WHERE case_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![case_id], |row| {
            Ok(crate::core::models::CaseDeliverableRecord {
                id: row.get(0)?,
                case_id: row.get(1)?,
                run_id: row.get(2)?,
                agent_id: row.get(3)?,
                title: row.get(4)?,
                artifact_refs_json: row.get(5)?,
                acceptance_evidence_json: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn update_case_deliverable_status(&self, deliverable_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE case_deliverables SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![deliverable_id, status, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    fn insert_case_review(&self, review: &crate::core::models::CaseReviewRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO case_reviews
                (id, case_id, deliverable_id, reviewer_agent_id, verdict, findings_json,
                 required_actions_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                review.id,
                review.case_id,
                review.deliverable_id,
                review.reviewer_agent_id,
                review.verdict,
                review.findings_json,
                review.required_actions_json,
                review.created_at
            ],
        )?;
        Ok(())
    }

    fn list_case_reviews(
        &self,
        case_id: &str,
    ) -> Result<Vec<crate::core::models::CaseReviewRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, case_id, deliverable_id, reviewer_agent_id, verdict, findings_json,
                    required_actions_json, created_at
             FROM case_reviews WHERE case_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![case_id], |row| {
            Ok(crate::core::models::CaseReviewRecord {
                id: row.get(0)?,
                case_id: row.get(1)?,
                deliverable_id: row.get(2)?,
                reviewer_agent_id: row.get(3)?,
                verdict: row.get(4)?,
                findings_json: row.get(5)?,
                required_actions_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_case_consensus(
        &self,
        consensus: &crate::core::models::CaseConsensusRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO case_consensus_records
                (id, case_id, proposal, status, quorum_rule_json, resolution, created_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                consensus.id,
                consensus.case_id,
                consensus.proposal,
                consensus.status,
                consensus.quorum_rule_json,
                consensus.resolution,
                consensus.created_at,
                consensus.resolved_at
            ],
        )?;
        Ok(())
    }

    fn list_case_consensus_records(
        &self,
        case_id: &str,
    ) -> Result<Vec<crate::core::models::CaseConsensusRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, case_id, proposal, status, quorum_rule_json, resolution, created_at, resolved_at
             FROM case_consensus_records WHERE case_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![case_id], |row| {
            Ok(crate::core::models::CaseConsensusRecord {
                id: row.get(0)?,
                case_id: row.get(1)?,
                proposal: row.get(2)?,
                status: row.get(3)?,
                quorum_rule_json: row.get(4)?,
                resolution: row.get(5)?,
                created_at: row.get(6)?,
                resolved_at: row.get(7)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_case_consensus_vote(
        &self,
        vote: &crate::core::models::CaseConsensusVoteRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO case_consensus_votes
                (id, consensus_id, voter_id, vote, rationale, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                vote.id,
                vote.consensus_id,
                vote.voter_id,
                vote.vote,
                vote.rationale,
                vote.created_at
            ],
        )?;
        Ok(())
    }

    fn list_case_consensus_votes(
        &self,
        consensus_id: &str,
    ) -> Result<Vec<crate::core::models::CaseConsensusVoteRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, consensus_id, voter_id, vote, rationale, created_at
             FROM case_consensus_votes WHERE consensus_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![consensus_id], |row| {
            Ok(crate::core::models::CaseConsensusVoteRecord {
                id: row.get(0)?,
                consensus_id: row.get(1)?,
                voter_id: row.get(2)?,
                vote: row.get(3)?,
                rationale: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn resolve_case_consensus(
        &self,
        consensus_id: &str,
        status: &str,
        resolution: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE case_consensus_records
             SET status = ?1, resolution = ?2, resolved_at = ?3
             WHERE id = ?4 AND status = 'proposed'",
            params![
                status,
                resolution,
                chrono::Utc::now().to_rfc3339(),
                consensus_id
            ],
        )?;
        if updated != 1 {
            return Err(anyhow::anyhow!("Consensus proposal is no longer pending."));
        }
        Ok(())
    }

    fn insert_case_escalation(
        &self,
        escalation: &crate::core::models::CaseEscalationRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO case_escalations
                (id, case_id, raised_by, reason, severity, status, resolved_by, resolution,
                 created_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                escalation.id,
                escalation.case_id,
                escalation.raised_by,
                escalation.reason,
                escalation.severity,
                escalation.status,
                escalation.resolved_by,
                escalation.resolution,
                escalation.created_at,
                escalation.resolved_at
            ],
        )?;
        Ok(())
    }

    fn list_pending_case_escalations(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::core::models::CaseEscalationRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, case_id, raised_by, reason, severity, status, resolved_by, resolution,
                    created_at, resolved_at
             FROM case_escalations WHERE status = 'open' ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(crate::core::models::CaseEscalationRecord {
                id: row.get(0)?,
                case_id: row.get(1)?,
                raised_by: row.get(2)?,
                reason: row.get(3)?,
                severity: row.get(4)?,
                status: row.get(5)?,
                resolved_by: row.get(6)?,
                resolution: row.get(7)?,
                created_at: row.get(8)?,
                resolved_at: row.get(9)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn resolve_case_escalation(
        &self,
        escalation_id: &str,
        resolved_by: &str,
        resolution: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE case_escalations
             SET status = 'resolved', resolved_by = ?1, resolution = ?2, resolved_at = ?3
             WHERE id = ?4 AND status = 'open'",
            params![
                resolved_by,
                resolution,
                chrono::Utc::now().to_rfc3339(),
                escalation_id
            ],
        )?;
        if updated != 1 {
            return Err(anyhow::anyhow!("Escalation is no longer open."));
        }
        Ok(())
    }

    fn insert_delegated_grant(
        &self,
        grant: &crate::core::models::DelegatedGrantRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO delegated_grants
                (id, case_id, run_id, grantor_actor_id, grantee_agent_id, allowed_tools_json,
                 allowed_mcp_json, workspace_scope_json, token_limit, cost_limit, expires_at,
                 status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                grant.id,
                grant.case_id,
                grant.run_id,
                grant.grantor_actor_id,
                grant.grantee_agent_id,
                grant.allowed_tools_json,
                grant.allowed_mcp_json,
                grant.workspace_scope_json,
                grant.token_limit,
                grant.cost_limit,
                grant.expires_at,
                grant.status,
                grant.created_at
            ],
        )?;
        Ok(())
    }

    fn get_active_delegated_grant(
        &self,
        agent_id: &str,
        run_id: &str,
    ) -> Result<Option<crate::core::models::DelegatedGrantRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, case_id, run_id, grantor_actor_id, grantee_agent_id, allowed_tools_json,
                    allowed_mcp_json, workspace_scope_json, token_limit, cost_limit, expires_at,
                    status, created_at
             FROM delegated_grants
             WHERE grantee_agent_id = ?1 AND run_id = ?2 AND status = 'active'
               AND (expires_at IS NULL OR expires_at > ?3)
             ORDER BY created_at DESC LIMIT 1",
            params![agent_id, run_id, chrono::Utc::now().to_rfc3339()],
            |row| {
                Ok(crate::core::models::DelegatedGrantRecord {
                    id: row.get(0)?,
                    case_id: row.get(1)?,
                    run_id: row.get(2)?,
                    grantor_actor_id: row.get(3)?,
                    grantee_agent_id: row.get(4)?,
                    allowed_tools_json: row.get(5)?,
                    allowed_mcp_json: row.get(6)?,
                    workspace_scope_json: row.get(7)?,
                    token_limit: row.get(8)?,
                    cost_limit: row.get(9)?,
                    expires_at: row.get(10)?,
                    status: row.get(11)?,
                    created_at: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn list_active_delegated_grants_for_case(
        &self,
        case_id: &str,
    ) -> Result<Vec<crate::core::models::DelegatedGrantRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, case_id, run_id, grantor_actor_id, grantee_agent_id, allowed_tools_json,
                    allowed_mcp_json, workspace_scope_json, token_limit, cost_limit, expires_at,
                    status, created_at
             FROM delegated_grants
             WHERE case_id = ?1 AND status = 'active'
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![case_id], |row| {
            Ok(crate::core::models::DelegatedGrantRecord {
                id: row.get(0)?,
                case_id: row.get(1)?,
                run_id: row.get(2)?,
                grantor_actor_id: row.get(3)?,
                grantee_agent_id: row.get(4)?,
                allowed_tools_json: row.get(5)?,
                allowed_mcp_json: row.get(6)?,
                workspace_scope_json: row.get(7)?,
                token_limit: row.get(8)?,
                cost_limit: row.get(9)?,
                expires_at: row.get(10)?,
                status: row.get(11)?,
                created_at: row.get(12)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn update_delegated_grant_status(&self, grant_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE delegated_grants SET status = ?1 WHERE id = ?2",
            params![status, grant_id],
        )?;
        Ok(())
    }

    fn upsert_agent_competency(
        &self,
        competency: &crate::core::models::AgentCompetencyRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO agent_competencies
                (agent_id, competency_key, score, evidence_count, confidence, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(agent_id, competency_key) DO UPDATE SET
                score = excluded.score, evidence_count = excluded.evidence_count,
                confidence = excluded.confidence, updated_at = excluded.updated_at",
            params![
                competency.agent_id,
                competency.competency_key,
                competency.score,
                competency.evidence_count,
                competency.confidence,
                competency.updated_at
            ],
        )?;
        Ok(())
    }

    fn list_agent_competencies(
        &self,
        competency_key: Option<&str>,
    ) -> Result<Vec<crate::core::models::AgentCompetencyRecord>> {
        let conn = self.conn.lock().unwrap();
        let sql = if competency_key.is_some() {
            "SELECT agent_id, competency_key, score, evidence_count, confidence, updated_at
             FROM agent_competencies WHERE competency_key = ?1
             ORDER BY score DESC, confidence DESC"
        } else {
            "SELECT agent_id, competency_key, score, evidence_count, confidence, updated_at
             FROM agent_competencies
             ORDER BY competency_key ASC, score DESC, confidence DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let map_row = |row: &rusqlite::Row| {
            Ok(crate::core::models::AgentCompetencyRecord {
                agent_id: row.get(0)?,
                competency_key: row.get(1)?,
                score: row.get(2)?,
                evidence_count: row.get(3)?,
                confidence: row.get(4)?,
                updated_at: row.get(5)?,
            })
        };
        let mut records = Vec::new();
        if let Some(key) = competency_key {
            for row in stmt.query_map(params![key], map_row)? {
                records.push(row?);
            }
        } else {
            for row in stmt.query_map([], map_row)? {
                records.push(row?);
            }
        }
        Ok(records)
    }

    fn insert_routing_decision(
        &self,
        routing: &crate::core::models::RoutingDecisionRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO routing_decisions
                (id, case_id, selected_agent_id, competency_key, score_snapshot_json, rationale, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                routing.id,
                routing.case_id,
                routing.selected_agent_id,
                routing.competency_key,
                routing.score_snapshot_json,
                routing.rationale,
                routing.created_at
            ],
        )?;
        Ok(())
    }

    fn list_routing_decisions_for_case(
        &self,
        case_id: &str,
    ) -> Result<Vec<crate::core::models::RoutingDecisionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, case_id, selected_agent_id, competency_key, score_snapshot_json,
                    rationale, created_at
             FROM routing_decisions WHERE case_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![case_id], |row| {
            Ok(crate::core::models::RoutingDecisionRecord {
                id: row.get(0)?,
                case_id: row.get(1)?,
                selected_agent_id: row.get(2)?,
                competency_key: row.get(3)?,
                score_snapshot_json: row.get(4)?,
                rationale: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn upsert_evaluation_rubric(
        &self,
        rubric: &crate::core::models::EvaluationRubricRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO evaluation_rubrics (id, name, version, criteria_json, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name, version = excluded.version,
                criteria_json = excluded.criteria_json, status = excluded.status",
            params![
                rubric.id,
                rubric.name,
                rubric.version,
                rubric.criteria_json,
                rubric.status,
                rubric.created_at
            ],
        )?;
        Ok(())
    }

    fn insert_run_evaluation(
        &self,
        evaluation: &crate::core::models::RunEvaluationRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO run_evaluations
                (id, run_id, rubric_id, evaluator_id, score, verdict, evidence_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                evaluation.id,
                evaluation.run_id,
                evaluation.rubric_id,
                evaluation.evaluator_id,
                evaluation.score,
                evaluation.verdict,
                evaluation.evidence_json,
                evaluation.created_at
            ],
        )?;
        Ok(())
    }

    fn get_run_evaluation(
        &self,
        evaluation_id: &str,
    ) -> Result<Option<crate::core::models::RunEvaluationRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, run_id, rubric_id, evaluator_id, score, verdict, evidence_json, created_at
             FROM run_evaluations WHERE id = ?1",
            params![evaluation_id],
            |row| {
                Ok(crate::core::models::RunEvaluationRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    rubric_id: row.get(2)?,
                    evaluator_id: row.get(3)?,
                    score: row.get(4)?,
                    verdict: row.get(5)?,
                    evidence_json: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn list_recent_run_evaluations(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::core::models::RunEvaluationRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, rubric_id, evaluator_id, score, verdict, evidence_json, created_at
             FROM run_evaluations ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(crate::core::models::RunEvaluationRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                rubric_id: row.get(2)?,
                evaluator_id: row.get(3)?,
                score: row.get(4)?,
                verdict: row.get(5)?,
                evidence_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_feedback_record(&self, feedback: &crate::core::models::FeedbackRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO feedback_records
                (id, run_id, case_id, source_kind, source_id, subject_kind, subject_id,
                 content, validation_status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                feedback.id,
                feedback.run_id,
                feedback.case_id,
                feedback.source_kind,
                feedback.source_id,
                feedback.subject_kind,
                feedback.subject_id,
                feedback.content,
                feedback.validation_status,
                feedback.created_at
            ],
        )?;
        Ok(())
    }

    fn get_feedback_record(
        &self,
        feedback_id: &str,
    ) -> Result<Option<crate::core::models::FeedbackRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, run_id, case_id, source_kind, source_id, subject_kind, subject_id,
                    content, validation_status, created_at
             FROM feedback_records WHERE id = ?1",
            params![feedback_id],
            |row| {
                Ok(crate::core::models::FeedbackRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    case_id: row.get(2)?,
                    source_kind: row.get(3)?,
                    source_id: row.get(4)?,
                    subject_kind: row.get(5)?,
                    subject_id: row.get(6)?,
                    content: row.get(7)?,
                    validation_status: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn update_feedback_validation_status(&self, feedback_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feedback_records SET validation_status = ?2 WHERE id = ?1",
            params![feedback_id, status],
        )?;
        Ok(())
    }

    fn list_recent_feedback_records(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::core::models::FeedbackRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, case_id, source_kind, source_id, subject_kind, subject_id,
                    content, validation_status, created_at
             FROM feedback_records ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(crate::core::models::FeedbackRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                case_id: row.get(2)?,
                source_kind: row.get(3)?,
                source_id: row.get(4)?,
                subject_kind: row.get(5)?,
                subject_id: row.get(6)?,
                content: row.get(7)?,
                validation_status: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_lesson(&self, lesson: &crate::core::models::LessonRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO lessons
                (id, source_evaluation_id, source_feedback_id, scope_kind, scope_id,
                 instruction, status, validated_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                instruction = excluded.instruction, status = excluded.status,
                validated_by = excluded.validated_by, updated_at = excluded.updated_at",
            params![
                lesson.id,
                lesson.source_evaluation_id,
                lesson.source_feedback_id,
                lesson.scope_kind,
                lesson.scope_id,
                lesson.instruction,
                lesson.status,
                lesson.validated_by,
                lesson.created_at,
                lesson.updated_at
            ],
        )?;
        Ok(())
    }

    fn get_lesson(&self, lesson_id: &str) -> Result<Option<crate::core::models::LessonRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, source_evaluation_id, source_feedback_id, scope_kind, scope_id,
                    instruction, status, validated_by, created_at, updated_at
             FROM lessons WHERE id = ?1",
            params![lesson_id],
            |row| {
                Ok(crate::core::models::LessonRecord {
                    id: row.get(0)?,
                    source_evaluation_id: row.get(1)?,
                    source_feedback_id: row.get(2)?,
                    scope_kind: row.get(3)?,
                    scope_id: row.get(4)?,
                    instruction: row.get(5)?,
                    status: row.get(6)?,
                    validated_by: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn update_lesson_status(&self, lesson_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE lessons SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![lesson_id, status, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    fn list_recent_lessons(&self, limit: u32) -> Result<Vec<crate::core::models::LessonRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, source_evaluation_id, source_feedback_id, scope_kind, scope_id,
                    instruction, status, validated_by, created_at, updated_at
             FROM lessons ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(crate::core::models::LessonRecord {
                id: row.get(0)?,
                source_evaluation_id: row.get(1)?,
                source_feedback_id: row.get(2)?,
                scope_kind: row.get(3)?,
                scope_id: row.get(4)?,
                instruction: row.get(5)?,
                status: row.get(6)?,
                validated_by: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn list_active_lessons(
        &self,
        scope_kind: Option<&str>,
        scope_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<crate::core::models::LessonRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut lessons = Vec::new();
        if let Some(scope_kind) = scope_kind {
            let mut stmt = conn.prepare(
                "SELECT id, source_evaluation_id, source_feedback_id, scope_kind, scope_id,
                        instruction, status, validated_by, created_at, updated_at
                 FROM lessons
                 WHERE status = 'active' AND scope_kind = ?1
                   AND (?2 = '' OR scope_id = ?2 OR scope_id = '')
                 ORDER BY updated_at DESC LIMIT ?3",
            )?;
            let rows =
                stmt.query_map(params![scope_kind, scope_id.unwrap_or(""), limit], |row| {
                    Ok(crate::core::models::LessonRecord {
                        id: row.get(0)?,
                        source_evaluation_id: row.get(1)?,
                        source_feedback_id: row.get(2)?,
                        scope_kind: row.get(3)?,
                        scope_id: row.get(4)?,
                        instruction: row.get(5)?,
                        status: row.get(6)?,
                        validated_by: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })?;
            for row in rows {
                lessons.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, source_evaluation_id, source_feedback_id, scope_kind, scope_id,
                        instruction, status, validated_by, created_at, updated_at
                 FROM lessons WHERE status = 'active' ORDER BY updated_at DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit], |row| {
                Ok(crate::core::models::LessonRecord {
                    id: row.get(0)?,
                    source_evaluation_id: row.get(1)?,
                    source_feedback_id: row.get(2)?,
                    scope_kind: row.get(3)?,
                    scope_id: row.get(4)?,
                    instruction: row.get(5)?,
                    status: row.get(6)?,
                    validated_by: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?;
            for row in rows {
                lessons.push(row?);
            }
        }
        Ok(lessons)
    }

    fn insert_learning_candidate(
        &self,
        candidate: &crate::core::models::LearningCandidateRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO learning_candidates
                (id, candidate_kind, source_lesson_id, target_id, baseline_version_id,
                 proposed_definition_json, risk_level, status, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                candidate.id,
                candidate.candidate_kind,
                candidate.source_lesson_id,
                candidate.target_id,
                candidate.baseline_version_id,
                candidate.proposed_definition_json,
                candidate.risk_level,
                candidate.status,
                candidate.created_by,
                candidate.created_at,
                candidate.updated_at
            ],
        )?;
        Ok(())
    }

    fn get_learning_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Option<crate::core::models::LearningCandidateRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, candidate_kind, source_lesson_id, target_id, baseline_version_id,
                    proposed_definition_json, risk_level, status, created_by, created_at, updated_at
             FROM learning_candidates WHERE id = ?1",
            params![candidate_id],
            |row| {
                Ok(crate::core::models::LearningCandidateRecord {
                    id: row.get(0)?,
                    candidate_kind: row.get(1)?,
                    source_lesson_id: row.get(2)?,
                    target_id: row.get(3)?,
                    baseline_version_id: row.get(4)?,
                    proposed_definition_json: row.get(5)?,
                    risk_level: row.get(6)?,
                    status: row.get(7)?,
                    created_by: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn update_learning_candidate_status(&self, candidate_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE learning_candidates SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![candidate_id, status, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    fn list_recent_learning_candidates(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::core::models::LearningCandidateRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, candidate_kind, source_lesson_id, target_id, baseline_version_id,
                    proposed_definition_json, risk_level, status, created_by, created_at, updated_at
             FROM learning_candidates ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(crate::core::models::LearningCandidateRecord {
                id: row.get(0)?,
                candidate_kind: row.get(1)?,
                source_lesson_id: row.get(2)?,
                target_id: row.get(3)?,
                baseline_version_id: row.get(4)?,
                proposed_definition_json: row.get(5)?,
                risk_level: row.get(6)?,
                status: row.get(7)?,
                created_by: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_skill_version(
        &self,
        version: &crate::core::models::SkillVersionRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO skill_versions
                (id, skill_id, candidate_id, version, instructions, activation_status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                version.id,
                version.skill_id,
                version.candidate_id,
                version.version,
                version.instructions,
                version.activation_status,
                version.created_at
            ],
        )?;
        Ok(())
    }

    fn next_skill_version_number(&self, skill_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM skill_versions WHERE skill_id = ?1",
            params![skill_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn get_skill_version_for_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Option<crate::core::models::SkillVersionRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, skill_id, candidate_id, version, instructions, activation_status, created_at
             FROM skill_versions WHERE candidate_id = ?1 ORDER BY version DESC LIMIT 1",
            params![candidate_id],
            |row| {
                Ok(crate::core::models::SkillVersionRecord {
                    id: row.get(0)?,
                    skill_id: row.get(1)?,
                    candidate_id: row.get(2)?,
                    version: row.get(3)?,
                    instructions: row.get(4)?,
                    activation_status: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn list_active_skill_versions(&self) -> Result<Vec<crate::core::models::SkillVersionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, skill_id, candidate_id, version, instructions, activation_status, created_at
             FROM skill_versions WHERE activation_status = 'active'
             ORDER BY skill_id, version DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::core::models::SkillVersionRecord {
                id: row.get(0)?,
                skill_id: row.get(1)?,
                candidate_id: row.get(2)?,
                version: row.get(3)?,
                instructions: row.get(4)?,
                activation_status: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn update_skill_version_activation(&self, version_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if status == "active" {
            conn.execute(
                "UPDATE skill_versions SET activation_status = 'inactive'
                 WHERE skill_id = (SELECT skill_id FROM skill_versions WHERE id = ?1)
                   AND activation_status = 'active' AND id <> ?1",
                params![version_id],
            )?;
        }
        conn.execute(
            "UPDATE skill_versions SET activation_status = ?2 WHERE id = ?1",
            params![version_id, status],
        )?;
        Ok(())
    }

    fn insert_benchmark_run(
        &self,
        benchmark: &crate::core::models::BenchmarkRunRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO benchmark_runs
                (id, candidate_id, suite_id, status, aggregate_score, regression_count,
                 result_json, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                benchmark.id,
                benchmark.candidate_id,
                benchmark.suite_id,
                benchmark.status,
                benchmark.aggregate_score,
                benchmark.regression_count,
                benchmark.result_json,
                benchmark.created_at,
                benchmark.completed_at
            ],
        )?;
        Ok(())
    }

    fn list_benchmark_runs_for_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Vec<crate::core::models::BenchmarkRunRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, candidate_id, suite_id, status, aggregate_score, regression_count,
                    result_json, created_at, completed_at
             FROM benchmark_runs WHERE candidate_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![candidate_id], |row| {
            Ok(crate::core::models::BenchmarkRunRecord {
                id: row.get(0)?,
                candidate_id: row.get(1)?,
                suite_id: row.get(2)?,
                status: row.get(3)?,
                aggregate_score: row.get(4)?,
                regression_count: row.get(5)?,
                result_json: row.get(6)?,
                created_at: row.get(7)?,
                completed_at: row.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn upsert_benchmark_suite(
        &self,
        suite: &crate::core::models::BenchmarkSuiteRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO benchmark_suites (id, name, version, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                version = excluded.version,
                status = excluded.status",
            params![
                suite.id,
                suite.name,
                suite.version,
                suite.status,
                suite.created_at
            ],
        )?;
        Ok(())
    }

    fn get_benchmark_suite(
        &self,
        suite_id: &str,
    ) -> Result<Option<crate::core::models::BenchmarkSuiteRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, version, status, created_at
             FROM benchmark_suites WHERE id = ?1",
            params![suite_id],
            |row| {
                Ok(crate::core::models::BenchmarkSuiteRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    status: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    fn list_active_benchmark_suites(
        &self,
    ) -> Result<Vec<crate::core::models::BenchmarkSuiteRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, version, status, created_at
             FROM benchmark_suites WHERE status = 'active'
             ORDER BY name ASC, version DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::core::models::BenchmarkSuiteRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_benchmark_case(&self, case: &crate::core::models::BenchmarkCaseRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO benchmark_cases
                (id, suite_id, input_json, expectation_json, risk_level, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                case.id,
                case.suite_id,
                case.input_json,
                case.expectation_json,
                case.risk_level,
                case.created_at
            ],
        )?;
        Ok(())
    }

    fn list_benchmark_cases_for_suite(
        &self,
        suite_id: &str,
    ) -> Result<Vec<crate::core::models::BenchmarkCaseRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, suite_id, input_json, expectation_json, risk_level, created_at
             FROM benchmark_cases WHERE suite_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![suite_id], |row| {
            Ok(crate::core::models::BenchmarkCaseRecord {
                id: row.get(0)?,
                suite_id: row.get(1)?,
                input_json: row.get(2)?,
                expectation_json: row.get(3)?,
                risk_level: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_benchmark_result(
        &self,
        result: &crate::core::models::BenchmarkResultRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO benchmark_results
                (id, benchmark_run_id, benchmark_case_id, score, verdict, evidence_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                result.id,
                result.benchmark_run_id,
                result.benchmark_case_id,
                result.score,
                result.verdict,
                result.evidence_json,
                result.created_at
            ],
        )?;
        Ok(())
    }

    fn list_benchmark_results_for_run(
        &self,
        benchmark_run_id: &str,
    ) -> Result<Vec<crate::core::models::BenchmarkResultRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, benchmark_run_id, benchmark_case_id, score, verdict, evidence_json, created_at
             FROM benchmark_results WHERE benchmark_run_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![benchmark_run_id], |row| {
            Ok(crate::core::models::BenchmarkResultRecord {
                id: row.get(0)?,
                benchmark_run_id: row.get(1)?,
                benchmark_case_id: row.get(2)?,
                score: row.get(3)?,
                verdict: row.get(4)?,
                evidence_json: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_benchmark_runner_job(
        &self,
        job: &crate::core::models::BenchmarkRunnerJobRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO benchmark_runner_jobs
                (id, candidate_id, suite_id, status, requested_by, benchmark_run_id,
                 error, created_at, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                job.id,
                job.candidate_id,
                job.suite_id,
                job.status,
                job.requested_by,
                job.benchmark_run_id,
                job.error,
                job.created_at,
                job.started_at,
                job.completed_at
            ],
        )?;
        Ok(())
    }

    fn update_benchmark_runner_job(
        &self,
        job_id: &str,
        status: &str,
        benchmark_run_id: Option<&str>,
        error: Option<&str>,
        completed_at: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE benchmark_runner_jobs
             SET status = ?2, benchmark_run_id = ?3, error = ?4, completed_at = ?5
             WHERE id = ?1",
            params![job_id, status, benchmark_run_id, error, completed_at],
        )?;
        Ok(())
    }

    fn list_benchmark_runner_jobs_for_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Vec<crate::core::models::BenchmarkRunnerJobRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, candidate_id, suite_id, status, requested_by, benchmark_run_id,
                    error, created_at, started_at, completed_at
             FROM benchmark_runner_jobs WHERE candidate_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![candidate_id], |row| {
            Ok(crate::core::models::BenchmarkRunnerJobRecord {
                id: row.get(0)?,
                candidate_id: row.get(1)?,
                suite_id: row.get(2)?,
                status: row.get(3)?,
                requested_by: row.get(4)?,
                benchmark_run_id: row.get(5)?,
                error: row.get(6)?,
                created_at: row.get(7)?,
                started_at: row.get(8)?,
                completed_at: row.get(9)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn list_benchmark_runner_jobs_by_status(
        &self,
        status: &str,
        limit: u32,
    ) -> Result<Vec<crate::core::models::BenchmarkRunnerJobRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, candidate_id, suite_id, status, requested_by, benchmark_run_id,
                    error, created_at, started_at, completed_at
             FROM benchmark_runner_jobs WHERE status = ?1 ORDER BY created_at ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![status, limit], |row| {
            Ok(crate::core::models::BenchmarkRunnerJobRecord {
                id: row.get(0)?,
                candidate_id: row.get(1)?,
                suite_id: row.get(2)?,
                status: row.get(3)?,
                requested_by: row.get(4)?,
                benchmark_run_id: row.get(5)?,
                error: row.get(6)?,
                created_at: row.get(7)?,
                started_at: row.get(8)?,
                completed_at: row.get(9)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_canary_deployment(
        &self,
        deployment: &crate::core::models::CanaryDeploymentRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO canary_deployments
                (id, candidate_id, scope_json, traffic_percent, status, started_at, ended_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                deployment.id,
                deployment.candidate_id,
                deployment.scope_json,
                deployment.traffic_percent,
                deployment.status,
                deployment.started_at,
                deployment.ended_at
            ],
        )?;
        Ok(())
    }

    fn list_canary_deployments_for_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Vec<crate::core::models::CanaryDeploymentRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, candidate_id, scope_json, traffic_percent, status, started_at, ended_at
             FROM canary_deployments WHERE candidate_id = ?1 ORDER BY started_at DESC",
        )?;
        let rows = stmt.query_map(params![candidate_id], |row| {
            Ok(crate::core::models::CanaryDeploymentRecord {
                id: row.get(0)?,
                candidate_id: row.get(1)?,
                scope_json: row.get(2)?,
                traffic_percent: row.get(3)?,
                status: row.get(4)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn update_canary_deployment_status(&self, deployment_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE canary_deployments SET status = ?2, ended_at = ?3 WHERE id = ?1",
            params![deployment_id, status, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    fn insert_canary_observation(
        &self,
        observation: &crate::core::models::CanaryObservationRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO canary_observations
                (id, deployment_id, run_id, metric_json, verdict, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                observation.id,
                observation.deployment_id,
                observation.run_id,
                observation.metric_json,
                observation.verdict,
                observation.created_at
            ],
        )?;
        Ok(())
    }

    fn list_canary_observations_for_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<Vec<crate::core::models::CanaryObservationRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, deployment_id, run_id, metric_json, verdict, created_at
             FROM canary_observations WHERE deployment_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![deployment_id], |row| {
            Ok(crate::core::models::CanaryObservationRecord {
                id: row.get(0)?,
                deployment_id: row.get(1)?,
                run_id: row.get(2)?,
                metric_json: row.get(3)?,
                verdict: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn insert_promotion_decision(
        &self,
        decision: &crate::core::models::PromotionDecisionRecord,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO promotion_decisions (id, candidate_id, decided_by, decision, rationale, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                decision.id,
                decision.candidate_id,
                decision.decided_by,
                decision.decision,
                decision.rationale,
                decision.created_at
            ],
        )?;
        Ok(())
    }

    fn insert_rollback_record(&self, rollback: &crate::core::models::RollbackRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO rollback_records
                (id, candidate_id, deployment_id, initiated_by, reason, restored_version_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                rollback.id,
                rollback.candidate_id,
                rollback.deployment_id,
                rollback.initiated_by,
                rollback.reason,
                rollback.restored_version_id,
                rollback.created_at
            ],
        )?;
        Ok(())
    }

    fn list_recent_rollback_records(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::core::models::RollbackRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, candidate_id, deployment_id, initiated_by, reason, restored_version_id, created_at
             FROM rollback_records ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(crate::core::models::RollbackRecord {
                id: row.get(0)?,
                candidate_id: row.get(1)?,
                deployment_id: row.get(2)?,
                initiated_by: row.get(3)?,
                reason: row.get(4)?,
                restored_version_id: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_permission_list;

    #[test]
    fn normalizes_legacy_all_permission_to_json_array() {
        assert_eq!(
            normalize_permission_list(Some("all")),
            Some("[\"all\"]".to_string())
        );
    }

    #[test]
    fn replaces_invalid_permission_payload_with_empty_list() {
        assert_eq!(
            normalize_permission_list(Some("not-json")),
            Some("[]".to_string())
        );
    }
}
