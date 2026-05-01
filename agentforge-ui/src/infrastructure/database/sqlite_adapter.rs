use crate::knowledge::core::KnowledgeItem;
use crate::core::models::{Agent, Instance, Provider, ProviderTemplate, SessionRecord, Team, WorkflowRecord};
use chrono::Utc;
use rusqlite::{params, Connection};
use anyhow::Result;
use crate::core::traits::database::DatabasePort;
use std::sync::Mutex;



pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = std::env::var("AGENTFORGE_DB_PATH").unwrap_or_else(|_| "agentforge.db".to_string());
        let conn = Connection::open(&db_path)?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA foreign_keys=ON;
            
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
                updated_at TEXT NOT NULL,
                UNIQUE(provider_name, model)
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

            CREATE TABLE IF NOT EXISTS knowledge_entries (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                session_id TEXT,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT, 
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(agent_id) REFERENCES agents(id)
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_entries_fts 
            USING fts5(title, content, tags, content='knowledge_entries', content_rowid='id');

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
                source_message_id TEXT,
                event_type TEXT NOT NULL,
                summary TEXT NOT NULL,
                payload TEXT,
                created_at TEXT NOT NULL
            );
            
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

        conn.execute(
            "ALTER TABLE cross_team_case_events ADD COLUMN source_message_id TEXT",
            [],
        )
        .ok();

        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_cross_team_case_events_correlation_source_message
             ON cross_team_case_events(correlation_id, source_message_id)",
            [],
        )
        .ok();

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.seed_provider_templates().ok();
        Ok(db)
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
        conn.execute(
            "INSERT OR REPLACE INTO provider_configs (id, provider_name, model, adapter_type, command, api_key_ref, status, is_builtin, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9)",
            params![p.id, p.provider_name, p.model, p.adapter_type, p.command, p.api_key_ref, p.status, now, now],
        )?;
        Ok(())
    }

    fn list_providers(&self) -> Result<Vec<Provider>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, provider_name, model, adapter_type, command, api_key_ref, status FROM provider_configs WHERE is_builtin = 0")?;
        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            Ok(Provider {
                id: row.get(0)?,
                provider_name: row.get(1)?,
                model: row.get(2)?,
                adapter_type: row.get(3)?,
                command: row.get(4)?,
                api_key_ref: row.get(5)?,
                status: row.get(6)?,
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
        let normalized = provider_name
            .split(" / ")
            .next()
            .unwrap_or(provider_name)
            .trim()
            .to_string();
        let mut stmt = conn.prepare("SELECT id, provider_name, model, adapter_type, command, api_key_ref, status FROM provider_configs WHERE provider_name = ?1 LIMIT 1")?;
        let mut rows = stmt.query(rusqlite::params![normalized])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Provider {
                id: row.get(0)?,
                provider_name: row.get(1)?,
                model: row.get(2)?,
                adapter_type: row.get(3)?,
                command: row.get(4)?,
                api_key_ref: row.get(5)?,
                status: row.get(6)?,
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
        conn.execute("DELETE FROM team_agents WHERE agent_id = ?1", params![agent_id])?;
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
        Ok(agents)
    }

    fn get_instance_agents(&self, instance_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        // Priority to Coordinator
        let mut stmt = conn.prepare("
            SELECT m.agent_id 
            FROM members m
            LEFT JOIN roles r ON m.role_id = r.id
            WHERE m.instance_id = ?1
            ORDER BY CASE WHEN LOWER(r.name) = 'coordinator' THEN 0 ELSE 1 END, m.joined_at ASC
        ")?;
        let iter = stmt.query_map(params![instance_id], |row: &rusqlite::Row| row.get(0))?;
        let mut agents = Vec::new();
        for a in iter {
            agents.push(a?);
        }
        if !agents.is_empty() {
            return Ok(agents);
        }

        let mut stmt = conn.prepare("SELECT team_id FROM instances WHERE id = ?1")?;
        let team_id: String = stmt.query_row(params![instance_id], |row: &rusqlite::Row| row.get(0))?;
        drop(stmt);

        let mut stmt = conn.prepare("
            SELECT m.agent_id 
            FROM members m
            LEFT JOIN roles r ON m.role_id = r.id
            WHERE m.team_id = ?1
            ORDER BY CASE WHEN LOWER(r.name) = 'coordinator' THEN 0 ELSE 1 END, m.joined_at ASC
        ")?;
        let iter = stmt.query_map(params![team_id], |row: &rusqlite::Row| row.get(0))?;
        let mut agents = Vec::new();
        for a in iter {
            agents.push(a?);
        }
        Ok(agents)
    }

    fn get_instance_agent_name_mapping(&self, instance_id: &str) -> Result<std::collections::HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut map = std::collections::HashMap::new();

        let mut stmt = conn.prepare(
            "SELECT a.name, m.agent_id 
             FROM members m 
             JOIN agents a ON m.agent_id = a.id 
             WHERE m.instance_id = ?1"
        )?;
        
        let iter = stmt.query_map(rusqlite::params![instance_id], |row: &rusqlite::Row| {
            let agent_name: String = row.get(0)?;
            let agent_id: String = row.get(1)?;
            Ok((agent_name, agent_id))
        })?;
        
        for (agent_name, agent_id) in iter.flatten() {
            map.insert(agent_name, agent_id);
        }
        
        if !map.is_empty() {
            return Ok(map);
        }

        // Fallback to team_id
        let mut stmt_team = conn.prepare("SELECT team_id FROM instances WHERE id = ?1")?;
        let team_id_result: Result<String, _> = stmt_team.query_row(rusqlite::params![instance_id], |row: &rusqlite::Row| row.get(0));
        drop(stmt_team);

        if let Ok(team_id) = team_id_result {
            let mut stmt = conn.prepare(
                "SELECT a.name, m.agent_id 
                 FROM members m 
                 JOIN agents a ON m.agent_id = a.id 
                 WHERE m.team_id = ?1"
            )?;
            
            let iter = stmt.query_map(rusqlite::params![team_id], |row: &rusqlite::Row| {
                let agent_name: String = row.get(0)?;
                let agent_id: String = row.get(1)?;
                Ok((agent_name, agent_id))
            })?;
            
            for (agent_name, agent_id) in iter.flatten() {
                map.insert(agent_name, agent_id);
            }
        }
        
        Ok(map)
    }


    fn upsert_task(
        &self,
        id: &str,
        team_id: &str,
        instance_id: Option<&str>,
        assignee_id: Option<&str>,
        status: &str,
        priority: &str,
        payload: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO tasks
                (id, team_id, instance_id, assignee_id, status, priority, payload, claimed_at, created_at, updated_at)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9)",
            params![id, team_id, instance_id, assignee_id, status, priority, payload, now, now],
        )?;
        Ok(())
    }

    
    fn get_total_tokens_per_agent(&self) -> Result<Vec<(String, usize)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.name, SUM(t.total_tokens)
             FROM token_usage t
             JOIN agents a ON t.agent_id = a.id
             GROUP BY t.agent_id
             ORDER BY SUM(t.total_tokens) DESC"
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
             ORDER BY SUM(t.total_tokens) DESC"
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
             ORDER BY COUNT(DISTINCT m.instance_id) DESC"
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
        let mut stmt = conn.prepare(
            "SELECT SUM(total_tokens) FROM token_usage WHERE date(created_at) = ?1"
        )?;
        let count: Option<usize> = stmt.query_row(rusqlite::params![now], |row: &rusqlite::Row| row.get(0)).unwrap_or(Some(0));
        Ok(count.unwrap_or(0))
    }

    
    fn get_total_tasks_completed(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM tasks WHERE status = 'completed'")?;
        let count: usize = stmt.query_row(rusqlite::params![], |row: &rusqlite::Row| row.get(0)).unwrap_or(0);
        Ok(count)
    }

    fn get_active_agents_count(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM agents WHERE status = 'active'")?;
        let count: usize = stmt.query_row(rusqlite::params![], |row: &rusqlite::Row| row.get(0)).unwrap_or(0);
        Ok(count)
    }

    fn insert_token_usage(&self, instance_id: Option<&str>, agent_id: &str, input_tokens: usize, output_tokens: usize, total_tokens: usize) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO token_usage (id, instance_id, agent_id, input_tokens, output_tokens, total_tokens, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, instance_id, agent_id, input_tokens, output_tokens, total_tokens, now],
        )?;
        Ok(())
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
            "SELECT id, team_id, instance_id, assignee_id, status, priority, payload, claimed_at, created_at, updated_at
             FROM tasks
             WHERE instance_id = ?1
             ORDER BY created_at DESC",
        )?;

        let iter = stmt.query_map(params![instance_id], |row: &rusqlite::Row| {
            Ok(crate::tasks::shared_task_list::Task {
                id: row.get(0)?,
                team_id: row.get(1)?,
                instance_id: row.get(2)?,
                assignee_id: row.get(3)?,
                status: row.get(4)?,
                priority: row.get(5)?,
                payload: row.get(6)?,
                claimed_at: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut tasks = Vec::new();
        for r in iter {
            tasks.push(r?);
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
            "SELECT id, team_id, instance_id, assignee_id, status, priority, payload, claimed_at, created_at, updated_at
             FROM tasks
             WHERE instance_id = ?1 AND status = 'pending'
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
                assignee_id: row.get(3)?,
                status: row.get(4)?,
                priority: row.get(5)?,
                payload: row.get(6)?,
                claimed_at: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut tasks = Vec::new();
        for t in iter {
            tasks.push(t?);
        }
        Ok(tasks)
    }

    fn claim_task_for_instance(&self, task_id: &str, agent_id: &str, instance_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let rows_affected = conn.execute(
            "UPDATE tasks
             SET assignee_id = ?1, status = 'in_progress', claimed_at = ?2, updated_at = ?3
             WHERE id = ?4 AND status = 'pending' AND instance_id = ?5",
            params![agent_id, now, now, task_id, instance_id],
        )?;
        Ok(rows_affected > 0)
    }

    fn mark_task_completed(&self, task_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
        )?;
        Ok(())
    }

    fn mark_task_failed(&self, task_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = 'failed', updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
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

        conn.execute(
            "INSERT INTO teams (id, name, description, objectives, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![team_id, "SDG", "SDG Team", "Test real data with OpenRouter", now, now],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO instances (id, team_id, config, state, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["sdg-instance-123", team_id, None::<String>, None::<String>, now],
        )?;

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
            ("Coordinator", "sdg-coord-123", "You are the Coordinator/Leader of the SDG team. You are responsible for breaking down goals, assigning tasks to other agents, and orchestrating the workflow."),
            ("PM", "sdg-pm-123", "You are the PM of the SDG team. Please provide short, direct responses about product management."),
            ("DEV", "sdg-dev-123", "You are the DEV of the SDG team. You write code and solve technical issues."),
            ("BA", "sdg-ba-123", "You are the BA of the SDG team. You analyze business requirements and metrics.")
        ];
        for (name, agent_id, prompt) in agents {
            conn.execute(
                "INSERT INTO agents (id, name, provider, system_prompt, config, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![agent_id, name, "openrouter", prompt, None::<String>, "online", now, now],
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

    fn insert_team_message(&self, msg: &crate::infrastructure::message_bus::routing::TeamMessage) -> Result<()> {
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

        let iter = stmt.query_map(params![team_instance_id, msg_type, limit], |row: &rusqlite::Row| {
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
        let mut sessions = self.list_sessions_for_instance(instance_id)?;
        Ok(sessions.pop())
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
            if let Some(meta_str) = metadata {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                    if let Some(name) = json.get("agent_name").and_then(|n| n.as_str()) {
                        agent_name = Some(name.to_string().into());
                    }
                }
            }
            
            msgs.push(crate::core::models::ChatMessage { role: role.into(), content: content.into(), agent_name });
        }
        Ok(msgs)
    }

    fn save_message(&self, team_id: &str, instance_id: Option<&str>, role: &str, content: &str) -> Result<()> {
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

    fn get_messages(&self, team_id: &str, instance_id: Option<&str>) -> Result<Vec<crate::core::models::ChatMessage>> {
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
            msgs.push(crate::core::models::ChatMessage { role: role.into(), content: content.into(), agent_name: None });
        }
        Ok(msgs)
    }

    fn upsert_knowledge_item(&self, item: &KnowledgeItem) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        let tags_json = serde_json::to_string(&item.tags).unwrap_or_default();
        
        conn.execute(
            "INSERT INTO knowledge (id, title, content, tags, created_at, updated_at, vault_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                title=excluded.title,
                content=excluded.content,
                tags=excluded.tags,
                updated_at=excluded.updated_at,
                vault_path=excluded.vault_path",
            params![
                item.id.to_string(),
                item.title,
                item.content,
                tags_json,
                item.created_at.to_rfc3339(),
                item.updated_at.to_rfc3339(),
                item.vault_path.as_deref().unwrap_or(""),
            ],
        )?;

        conn.execute("DELETE FROM knowledge_fts WHERE id = ?1", params![item.id.to_string()])
            .ok();
        conn.execute(
            "INSERT INTO knowledge_fts (id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
            params![item.id.to_string(), item.title, item.content, tags_json],
        )
        .ok();
        Ok(())
    }

    fn get_all_knowledge_items(&self) -> Result<Vec<KnowledgeItem>> {
        let conn = self.conn.lock().unwrap();
        // Use GROUP BY title to deduplicate entries with the same title
        let mut stmt = conn.prepare("SELECT id, title, content, tags, created_at, updated_at, vault_path FROM knowledge GROUP BY title ORDER BY updated_at DESC")?;
        
        let mut rows = stmt.query(params![])?;
        let mut items = Vec::new();
        
        while let Some(row) = rows.next()? {
            let row: &rusqlite::Row = row;
            let id_str: String = row.get(0)?;
            let id = uuid::Uuid::parse_str(&id_str).unwrap_or_default();
            let title = row.get::<usize, String>(1)?;
            let content = row.get::<usize, String>(2)?;
            let tags_str = row.get::<usize, String>(3)?;
            let tags: Vec<crate::knowledge::core::Tag> = serde_json::from_str(&tags_str).unwrap_or_default();
            let created_at_str = row.get::<usize, String>(4)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let updated_at_str = row.get::<usize, String>(5)?;
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            
            let vault_path = row.get::<usize, String>(6).ok();
            
            items.push(KnowledgeItem {
                id,
                title,
                content,
                tags,
                retention_policy: crate::knowledge::core::RetentionPolicy::KeepForever,
                created_at,
                updated_at,
                vault_path,
                
                
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
            "SELECT id, title, content, tags, created_at, updated_at
             FROM knowledge 
             WHERE content LIKE ?1 OR title LIKE ?1
             LIMIT 5"
        )?;
        
        let search_pattern = format!("%{}%", query);
        let rows = stmt.query_map(params![search_pattern], |row: &rusqlite::Row| {
            let tags_str = row.get::<usize, String>(3)?;
            let tags = serde_json::from_str(&tags_str).unwrap_or_default();
            
            Ok(KnowledgeItem {
                id: uuid::Uuid::parse_str(&row.get::<usize, String>(0)?).unwrap_or_default(),
                title: row.get::<usize, String>(1)?,
                content: row.get::<usize, String>(2)?,
                tags,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<usize, String>(4)?).unwrap().into(),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<usize, String>(5)?).unwrap().into(),
                retention_policy: crate::knowledge::core::RetentionPolicy::KeepForever,
                vault_path: None,
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
            "SELECT id, title, content, tags
             FROM knowledge_fts
             WHERE knowledge_fts MATCH ?1
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, limit], |row: &rusqlite::Row| {
            let id_str = row.get::<usize, String>(0)?;
            let tags_str = row.get::<usize, String>(3)?;
            let tags = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(KnowledgeItem {
                id: uuid::Uuid::parse_str(&id_str).unwrap_or_default(),
                title: row.get::<usize, String>(1)?,
                content: row.get::<usize, String>(2)?,
                tags,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                retention_policy: crate::knowledge::core::RetentionPolicy::KeepForever,
                vault_path: None,
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
             VALUES (?1, ?2, ?3, ?4, ?5)"
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

    fn search_similar_chunks(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<(String, String, f32)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT k.title, c.content, c.embedding 
             FROM knowledge_chunks c 
             JOIN knowledge k ON c.document_id = k.id"
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
            "INSERT OR REPLACE INTO workflows (id, name, definition, version, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4,
                COALESCE((SELECT created_at FROM workflows WHERE id = ?1), ?5),
                ?5
             )",
            params![wf.id, wf.name, wf.definition, wf.version, now],
        )?;
        Ok(())
    }

    fn list_workflows(&self) -> Result<Vec<WorkflowRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, definition, version, created_at, updated_at
             FROM workflows
             ORDER BY updated_at DESC",
        )?;
        let iter = stmt.query_map([], |row: &rusqlite::Row| {
            Ok(WorkflowRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                definition: row.get(2)?,
                version: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
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
            "SELECT id, name, definition, version, created_at, updated_at
             FROM workflows
             WHERE id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![workflow_id])?;
        if let Some(row) = rows.next()? {
            let row: &rusqlite::Row = row;
            Ok(Some(WorkflowRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                definition: row.get(2)?,
                version: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn save_workflow_state(&self, state: &crate::application::iflow_engine::engine::WorkflowState) -> Result<()> {
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

    fn load_workflow_state(&self, execution_id: &str) -> Result<Option<crate::application::iflow_engine::engine::WorkflowState>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT state_json FROM workflow_states WHERE execution_id = ?1 LIMIT 1"
        )?;
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

    fn insert_audit_log(&self, event: &crate::infrastructure::security::audit::AuditEvent) -> Result<()> {
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

    fn create_role(&self, role: &crate::application::teams::role::Role) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO roles (id, team_id, name, permissions, capabilities)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                role.id,
                role.team_id,
                role.name,
                role.permissions,
                role.capabilities
            ],
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
            let perms: Vec<String> = serde_json::from_str(&perms_str).unwrap_or_default();
            Ok(perms.iter().any(|p| p == required_permission || p == "all"))
        } else {
            Ok(false)
        }
    }

    fn upsert_mcp_tool(&self, tool: &crate::infrastructure::mcp::registry::McpTool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let args_json = serde_json::to_string(&tool.args).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO mcp_tools (id, name, description, version, command, args, input_schema, is_active) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET 
             name=excluded.name, description=excluded.description, version=excluded.version, 
             command=excluded.command, args=excluded.args, input_schema=excluded.input_schema, is_active=excluded.is_active",
            rusqlite::params![
                tool.id, tool.name, tool.description, tool.version, tool.command, args_json, tool.input_schema, tool.is_active
            ],
        )?;
        Ok(())
    }

    fn get_mcp_tool(&self, id: &str) -> Result<Option<crate::infrastructure::mcp::registry::McpTool>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, description, version, command, args, input_schema, is_active FROM mcp_tools WHERE id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![id])?;
        if let Some(row) = rows.next()? {
            let args_json: String = row.get(5)?;
            let args: Vec<String> = serde_json::from_str(&args_json).unwrap_or_default();
            Ok(Some(crate::infrastructure::mcp::registry::McpTool {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                version: row.get(3)?,
                command: row.get(4)?,
                args,
                input_schema: row.get(6)?,
                is_active: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn list_mcp_tools(&self) -> Result<Vec<crate::infrastructure::mcp::registry::McpTool>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, description, version, command, args, input_schema, is_active FROM mcp_tools")?;
        let tool_iter = stmt.query_map(rusqlite::params![], |row| {
            let args_json: String = row.get(5)?;
            let args: Vec<String> = serde_json::from_str(&args_json).unwrap_or_default();
            Ok(crate::infrastructure::mcp::registry::McpTool {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                version: row.get(3)?,
                command: row.get(4)?,
                args,
                input_schema: row.get(6)?,
                is_active: row.get(7)?,
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

    fn upsert_knowledge_entry(&self, entry: &crate::core::models::knowledge::KnowledgeEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tags_json = serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string());
        let created_at_str = entry.created_at.to_rfc3339();
        
        conn.execute(
            "INSERT INTO knowledge_entries (id, agent_id, session_id, title, content, tags, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET 
             title=excluded.title, content=excluded.content, tags=excluded.tags",
            rusqlite::params![
                entry.id, entry.agent_id, entry.session_id, entry.title, entry.content, tags_json, created_at_str
            ],
        )?;
        Ok(())
    }

    fn get_knowledge_entry(&self, id: &str) -> Result<Option<crate::core::models::knowledge::KnowledgeEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, agent_id, session_id, title, content, tags, created_at FROM knowledge_entries WHERE id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![id])?;
        if let Some(row) = rows.next()? {
            let tags_json: String = row.get(5)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let created_at_str: String = row.get(6)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            
            Ok(Some(crate::core::models::knowledge::KnowledgeEntry {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                session_id: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                tags,
                created_at,
            }))
        } else {
            Ok(None)
        }
    }

    fn search_knowledge_entries_fts(&self, query: &str, limit: u32) -> Result<Vec<crate::core::models::knowledge::KnowledgeEntry>> {
        let conn = self.conn.lock().unwrap();
        // Use FTS5 for search
        let mut stmt = conn.prepare(
            "SELECT e.id, e.agent_id, e.session_id, e.title, e.content, e.tags, e.created_at
             FROM knowledge_entries e
             JOIN knowledge_entries_fts fts ON e.id = fts.id
             WHERE knowledge_entries_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        )?;
        
        // SQLite FTS5 requires query formatting, but for simplicity, just pass the raw query if it's safe, 
        // or wrap it in quotes. A better way is to clean the query.
        let fts_query = format!("\"{}\"", query.replace("\"", ""));
        
        let entry_iter = stmt.query_map(rusqlite::params![fts_query, limit], |row| {
            let tags_json: String = row.get(5)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let created_at_str: String = row.get(6)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
                
            Ok(crate::core::models::knowledge::KnowledgeEntry {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                session_id: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
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

    fn insert_cross_team_case_event(&self, event: &crate::core::models::CrossTeamCaseEventRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO cross_team_case_events
                (id, correlation_id, from_instance_id, reply_to_instance_id, source_message_id, event_type, summary, payload, created_at)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                event.id,
                event.correlation_id,
                event.from_instance_id,
                event.reply_to_instance_id,
                event.source_message_id,
                event.event_type,
                event.summary,
                event.payload,
                event.created_at
            ],
        )?;
        Ok(())
    }

    fn list_cross_team_cases(&self, instance_id: &str, limit: u32) -> Result<Vec<crate::core::models::CrossTeamCaseRecord>> {
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

    fn list_cross_team_case_events(&self, correlation_id: &str, limit: u32) -> Result<Vec<crate::core::models::CrossTeamCaseEventRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, correlation_id, from_instance_id, reply_to_instance_id, source_message_id, event_type, summary, payload, created_at
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
                source_message_id: row.get(4)?,
                event_type: row.get(5)?,
                summary: row.get(6)?,
                payload: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        let mut events = Vec::new();
        for e in iter {
            events.push(e?);
        }
        Ok(events)
    }
}
