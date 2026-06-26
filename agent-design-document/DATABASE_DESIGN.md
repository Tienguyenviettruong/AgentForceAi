# Database Design — AgentForge AI
> **Phiên bản:** 2.0 (Từ mã nguồn SQLite adapter thực tế)
> **Cập nhật:** 2026-06-12

---

## 1. Tổng quan

AgentForge AI sử dụng **SQLite** (qua crate `rusqlite`) với các đặc điểm:
- **WAL Mode**: `PRAGMA journal_mode = WAL` — cho phép concurrent reads + non-blocking writes
- **Foreign Keys**: `PRAGMA foreign_keys = ON`
- **Synchronous**: `PRAGMA synchronous = NORMAL` — balance giữa safety và performance
- **Abstraction**: Toàn bộ DB operations được đưa qua `DatabasePort` trait (dependency inversion)
- **ATTACH**: Hỗ trợ attach thêm DB files (e.g., kiến thức từ instance khác)

---

## 2. DatabasePort Trait — `core/traits/database.rs`

`DatabasePort` là interface trung gian giữa application layer và storage layer. Tất cả methods của adapter đều implement trait này. Ví dụ các method quan trọng:

```rust
pub trait DatabasePort: Send + Sync {
    // Agent management
    fn get_agent(&self, id: &str) -> Result<Option<Agent>>;
    fn create_agent(&self, agent: &Agent) -> Result<()>;
    fn update_agent(&self, agent: &Agent) -> Result<()>;
    fn list_agents(&self) -> Result<Vec<Agent>>;
    
    // Task management (Atomic Claim)
    fn list_tasks_for_instance(&self, instance_id: &str) -> Result<Vec<Task>>;
    fn claim_task_for_instance(&self, task_id: &str, agent_id: &str, instance_id: &str) -> Result<bool>;
    fn mark_task_completed(&self, task_id: &str) -> Result<()>;
    fn mark_task_failed(&self, task_id: &str) -> Result<()>;
    fn mark_task_waiting_approval(&self, task_id: &str) -> Result<()>;
    fn is_task_unblocked(&self, task_id: &str) -> Result<bool>;
    fn recover_stale_in_progress_tasks(&self, max_age_seconds: u64) -> Result<usize>;
    
    // Orchestration
    fn create_orchestration_run(&self, run: &OrchestrationRunRecord) -> Result<()>;
    fn get_orchestration_run(&self, run_id: &str) -> Result<Option<OrchestrationRunRecord>>;
    fn update_orchestration_run_status(&self, run_id: &str, status: &str, error: Option<&str>) -> Result<()>;
    fn insert_run_event(&self, event: &RunEventRecord) -> Result<()>;
    fn list_recent_run_events(&self, run_id: Option<&str>, limit: usize) -> Result<Vec<RunEventRecord>>;
    fn get_total_tokens_for_run(&self, run_id: &str) -> Result<usize>;
    
    // Approval
    fn create_approval_request(&self, req: &ApprovalRequestRecord) -> Result<()>;
    fn get_approval_request_for_operation(&self, run_id: &str, operation: &str) -> Result<Option<ApprovalRequestRecord>>;
    fn update_approval_request_status(&self, id: &str, status: &str, resolved_by: Option<&str>) -> Result<()>;
    
    // Tool Invocations
    fn upsert_tool_invocation(&self, inv: &ToolInvocationRecord) -> Result<()>;
    fn update_tool_invocation_status(&self, id: &str, status: &str, approval_id: Option<&str>, result: Option<&str>) -> Result<()>;
    fn get_next_approved_tool_invocation_for_run(&self, run_id: &str) -> Result<Option<ToolInvocationRecord>>;
    
    // Workflow (iFlow)
    fn upsert_workflow(&self, wf: &WorkflowRecord) -> anyhow::Result<()>;
    fn get_workflow(&self, id: &str) -> Result<Option<WorkflowRecord>>;
    fn list_workflows(&self) -> Result<Vec<WorkflowRecord>>;
    fn save_workflow_state(&self, state: &WorkflowState) -> Result<()>;
    fn load_workflow_state(&self, execution_id: &str) -> Result<Option<WorkflowState>>;
    fn save_workflow_execution(&self, exec: &WorkflowExecutionRecord) -> Result<()>;
    fn get_workflow_execution(&self, id: &str) -> Result<Option<WorkflowExecutionRecord>>;
    fn get_latest_workflow_execution_for_version(&self, version_id: &str) -> Result<Option<WorkflowExecutionRecord>>;
    fn get_latest_workflow_version_for_run(&self, run_id: &str) -> Result<Option<WorkflowVersionRecord>>;
    fn get_latest_workflow_version_for_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowVersionRecord>>;
    
    // Cross-team collaboration
    fn upsert_cross_team_case(&self, correlation_id: &str, owner: &str, target: &str, event_type: &str, summary: &str) -> Result<()>;
    fn insert_cross_team_case_event(&self, event: &CrossTeamCaseEventRecord) -> Result<()>;
    fn get_collaboration_case(&self, case_id: &str) -> Result<Option<CollaborationCase>>;
    fn get_collaboration_case_by_correlation_id(&self, correlation_id: &str) -> Result<Option<CollaborationCase>>;
    fn get_collaboration_case_for_run(&self, run_id: &str) -> Result<Option<CollaborationCase>>;
    fn get_active_delegated_grant(&self, agent_id: &str, run_id: &str) -> Result<Option<DelegatedGrant>>;
    
    // Security
    fn check_actor_permission(&self, actor_id: &str, permission: &str) -> Result<bool>;
    fn insert_audit_log(&self, event: &AuditEvent) -> Result<()>;
    fn ensure_local_security_owner(&self, actor_id: &str) -> Result<()>;
    
    // Settings
    fn get_setting(&self, key: &str) -> Result<Option<String>>;
    fn set_setting(&self, key: &str, value: &str) -> Result<()>;
    
    // Knowledge
    fn save_knowledge_item(&self, item: &KnowledgeItem) -> Result<()>;
    fn search_knowledge(&self, query: &str, session_id: Option<&str>) -> Result<Vec<KnowledgeItem>>;
    
    // LLM Context Tracking
    fn insert_llm_context_snapshot(&self, snapshot: &LlmContextSnapshotRecord) -> Result<()>;
    fn insert_llm_context_source(&self, source: &LlmContextSourceRecord) -> Result<()>;
    
    // Session
    fn ensure_session(&self, session_id: &str, agent_id: &str, instance_id: Option<&str>) -> Result<()>;
    fn append_conversation_turn(&self, session_id: &str, role: &str, content: &str, metadata: Option<&str>) -> Result<()>;
    fn touch_session(&self, session_id: &str) -> Result<()>;
    fn create_session_for_instance(&self, instance_id: &str, agent_id: &str) -> Result<String>;
    fn get_latest_session_for_instance(&self, instance_id: &str) -> Result<Option<Session>>;
    
    // Team messages
    fn insert_team_message(&self, msg: &TeamMessage) -> Result<()>;
    fn update_team_message_content(&self, id: &str, content: &str) -> Result<()>;
    fn update_team_message_delivery_status(&self, id: &str, status: &str) -> Result<()>;
    
    // Instance agents
    fn get_instance_agents(&self, instance_id: &str) -> Result<Vec<String>>;
    fn get_instance_agent_name_mapping(&self, instance_id: &str) -> Result<HashMap<String, String>>;
    
    // Providers
    fn list_providers(&self) -> Result<Vec<ProviderRecord>>;
    fn get_provider_by_name(&self, name: &str) -> Result<Option<ProviderRecord>>;
}
```

---

## 3. Chi tiết Schema từng bảng

### 3.1. `agents`

```sql
CREATE TABLE IF NOT EXISTS agents (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    provider    TEXT NOT NULL,
    system_prompt TEXT,
    config      TEXT,                   -- JSON blob: role, position, details, responsibilities...
    status      TEXT NOT NULL DEFAULT 'online',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

**Quan trọng:** `config` là JSON string chứa toàn bộ agent personality:
```json
{
  "role": "Developer",
  "position": "Senior Rust Developer",
  "details": "Chuyên backend systems",
  "responsibilities": ["implement features", "code review", "testing"],
  "competencies": ["rust", "backend", "systems", "testing"],
  "allowed_task_types": ["implementation", "testing", "documentation"],
  "disallowed_task_types": ["marketing", "design"]
}
```

---

### 3.2. `teams` và `team_instances`

```sql
CREATE TABLE IF NOT EXISTS teams (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS team_instances (
    id          TEXT PRIMARY KEY,
    team_id     TEXT NOT NULL REFERENCES teams(id),
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_team_assignments (
    agent_id         TEXT NOT NULL REFERENCES agents(id),
    team_instance_id TEXT NOT NULL REFERENCES team_instances(id),
    created_at       TEXT NOT NULL,
    PRIMARY KEY (agent_id, team_instance_id)
);
```

---

### 3.3. `sessions` và `conversation_turns`

```sql
CREATE TABLE IF NOT EXISTS sessions (
    id               TEXT PRIMARY KEY,
    agent_id         TEXT REFERENCES agents(id),
    team_instance_id TEXT REFERENCES team_instances(id),
    title            TEXT,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conversation_turns (
    id         TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role       TEXT NOT NULL,    -- "user" | "assistant" | "system"
    content    TEXT NOT NULL,
    metadata   TEXT,             -- JSON: agent_name, task_id, stream_kind, thought_duration_secs,...
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_conversation_turns_session ON conversation_turns(session_id, created_at);
```

---

### 3.4. `team_messages`

```sql
CREATE TABLE IF NOT EXISTS team_messages (
    id                  TEXT PRIMARY KEY,
    team_instance_id    TEXT NOT NULL REFERENCES team_instances(id),
    sender_member_id    TEXT NOT NULL,
    recipient_member_id TEXT,            -- NULL = broadcast
    recipient_role      TEXT,
    message_type        TEXT NOT NULL,   -- "Direct" | "Broadcast" | "System"
    content             TEXT NOT NULL,
    metadata            TEXT,            -- JSON: agent_name, task_id, stream_kind,...
    delivery_status     TEXT NOT NULL,   -- "typing" | "delivered" | "failed" | "waiting_approval"
    created_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_team_messages_instance ON team_messages(team_instance_id, created_at);
```

**delivery_status flow:** `typing` → `delivered` | `failed` | `waiting_approval`

---

### 3.5. `tasks` — với Atomic Claim

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id               TEXT PRIMARY KEY,
    run_id           TEXT REFERENCES orchestration_runs(id),
    team_instance_id TEXT NOT NULL REFERENCES team_instances(id),
    assignee_id      TEXT REFERENCES agents(id),    -- NULL = unassigned (role-based routing)
    title            TEXT,
    status           TEXT NOT NULL DEFAULT 'pending',
    payload          TEXT,                           -- JSON: title, description, role, task_type,...
    dependencies     TEXT,                           -- JSON array of task_ids
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_instance_status ON tasks(team_instance_id, status);
```

**Status lifecycle:**
```
pending → in_progress → completed
                      → failed
                      → waiting_approval
```

**Atomic claim SQL:**
```sql
UPDATE tasks
SET status = 'in_progress',
    assignee_id = :agent_id,
    updated_at = :now
WHERE id = :task_id
  AND status = 'pending'
  AND team_instance_id = :instance_id
```
→ `changes() > 0` = claim thành công

**Stale recovery:**
```sql
UPDATE tasks
SET status = 'pending', updated_at = :now
WHERE status = 'in_progress'
  AND (unixepoch(:now) - unixepoch(updated_at)) > :max_age_seconds
```

**task.payload JSON schema:**
```json
{
  "title": "Implement API handler",
  "description": "Create REST endpoint for...",
  "role": "Developer",
  "task_type": "implementation"
}
```

---

### 3.6. `providers`

```sql
CREATE TABLE IF NOT EXISTS providers (
    id            TEXT PRIMARY KEY,
    provider_name TEXT NOT NULL UNIQUE,
    adapter_type  TEXT NOT NULL,   -- "claude" | "gemini" | "openrouter" | "custom" | "ollama"
    model         TEXT NOT NULL,
    api_key_ref   TEXT,            -- reference key để lookup API key
    command       TEXT,            -- base_url cho custom provider
    is_default    INTEGER DEFAULT 0,
    created_at    TEXT NOT NULL
);
```

---

### 3.7. `orchestration_runs`

```sql
CREATE TABLE IF NOT EXISTS orchestration_runs (
    id           TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL REFERENCES sessions(id),
    instance_id  TEXT NOT NULL REFERENCES team_instances(id),
    initiated_by TEXT,              -- security actor ID, e.g. "local-user"
    goal         TEXT NOT NULL,
    mode         TEXT NOT NULL,     -- "human_interaction" | "supervision" | "autonomous"
    status       TEXT NOT NULL,     -- "running" | "paused" | "completed" | "failed" | "waiting_approval"
    workflow_id  TEXT REFERENCES workflows(id),
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_runs_instance ON orchestration_runs(instance_id, created_at);
CREATE INDEX IF NOT EXISTS idx_runs_session ON orchestration_runs(session_id);
```

---

### 3.8. `run_events`

```sql
CREATE TABLE IF NOT EXISTS run_events (
    id         TEXT PRIMARY KEY,
    run_id     TEXT NOT NULL REFERENCES orchestration_runs(id),
    event_type TEXT NOT NULL,   -- "agent_execution_started", "tool_policy_allowed",...
    actor_type TEXT NOT NULL,   -- "agent" | "policy" | "system"
    actor_id   TEXT,            -- agent_id hoặc NULL
    task_id    TEXT,
    payload    TEXT,            -- human-readable details
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_run_events_run ON run_events(run_id, created_at);
```

**event_type values:**
| event_type | Mô tả |
|---|---|
| `agent_execution_started` | AgentExecutor bắt đầu |
| `agent_execution_cancelled` | cancel_flag set |
| `tool_policy_allowed` | Tool được authorize |
| `tool_policy_denied` | Tool bị deny |
| `tool_policy_approval_required` | Cần approval |
| `sealed_invocation_resumed` | Approved invocation re-executed |
| `budget_blocked` | Token budget vượt ngưỡng |
| `llm_request_context_snapshot` | Snapshot context trước LLM call |
| `workflow_execution_state` | iFlow state update |
| `workflow_node_waiting_approval` | iFlow node cần approval |
| `cross_team_run_created` | Cross-team run bắt đầu |
| `cross_team_run_completed` | Cross-team run hoàn thành |
| `cross_team_run_failed` | Cross-team run thất bại |

---

### 3.9. `approval_requests`

```sql
CREATE TABLE IF NOT EXISTS approval_requests (
    id              TEXT PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES orchestration_runs(id),
    operation       TEXT NOT NULL,   -- SHA256-based key: "tool:write_file:path=...:payload=...:mode=..."
    requested_by    TEXT,            -- actor_id
    status          TEXT NOT NULL DEFAULT 'pending',  -- "pending" | "approved" | "rejected"
    resolved_by     TEXT,
    decision_reason TEXT,
    created_at      TEXT NOT NULL,
    resolved_at     TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_approval_run_operation ON approval_requests(run_id, operation);
```

**Operation key format:**
```
tool:{tool_name}:{path=...|command=...}:payload={sha256_hex}:mode={human_interaction|supervision|autonomous}
```

Example: `tool:write_file:path=src/main.rs:payload=a1b2c3...:mode=supervision`

---

### 3.10. `tool_invocations`

```sql
CREATE TABLE IF NOT EXISTS tool_invocations (
    id                  TEXT PRIMARY KEY,   -- "{run_id}:{tool_call_id}"
    run_id              TEXT NOT NULL REFERENCES orchestration_runs(id),
    tool_name           TEXT NOT NULL,
    sealed_payload_json TEXT NOT NULL,      -- AEAD encrypted payload
    payload_hash        TEXT NOT NULL,      -- SHA256 của payload gốc
    mode                TEXT NOT NULL,      -- operating mode lúc invoke
    status              TEXT NOT NULL,      -- xem bên dưới
    approval_request_id TEXT REFERENCES approval_requests(id),
    result              TEXT,               -- sealed result (AEAD) hoặc error text
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_invocations_run ON tool_invocations(run_id, status);
```

**Status lifecycle:**
```
sealed → authorized → executed
       → awaiting_approval → authorized (sau khi approve) → executed
       → denied
       → rejected
```

---

### 3.11. Workflow Tables

```sql
CREATE TABLE IF NOT EXISTS workflows (
    id                TEXT PRIMARY KEY,
    instance_id       TEXT REFERENCES team_instances(id),
    name              TEXT NOT NULL,
    definition        TEXT NOT NULL,        -- JSON workflow definition
    activation_status TEXT NOT NULL DEFAULT 'draft',  -- "draft" | "active" | "archived"
    run_id            TEXT,                 -- nếu là cron workflow đang active
    created_at        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workflow_versions (
    id              TEXT PRIMARY KEY,
    workflow_id     TEXT NOT NULL REFERENCES workflows(id),
    definition_json TEXT NOT NULL,
    run_id          TEXT REFERENCES orchestration_runs(id),
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workflow_executions (
    id                  TEXT PRIMARY KEY,
    workflow_version_id TEXT NOT NULL REFERENCES workflow_versions(id),
    run_id              TEXT NOT NULL REFERENCES orchestration_runs(id),
    status              TEXT NOT NULL,      -- "pending" | "running" | "paused" | "completed" | "failed: ..."
    state_json          TEXT NOT NULL,      -- serialized WorkflowState JSON
    updated_at          TEXT NOT NULL
);
```

**state_json** là serialization của `WorkflowState`:
```json
{
  "execution_id": "...",
  "workflow_id": "...",
  "current_nodes": ["node3"],
  "completed_nodes": ["node1", "node2"],
  "data": {"variables": {"result": "output text"}},
  "status": "Running",
  "strategy": "Serial",
  "pending_review": null,
  "pending_agent_tasks": [],
  "pending_delays": {}
}
```

---

### 3.12. Knowledge Base

```sql
CREATE TABLE IF NOT EXISTS knowledge_items (
    id         TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    title      TEXT NOT NULL,
    content    TEXT NOT NULL,
    embedding  BLOB,           -- vector embedding (optional, for semantic search)
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_knowledge_session ON knowledge_items(session_id);
```

**FTS5 Virtual Table (nếu được enable):**
```sql
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
    title,
    content,
    content='knowledge_items',
    content_rowid='rowid'
);
```

Search flow:
1. Nếu FTS5 enabled: `SELECT * FROM knowledge_fts WHERE knowledge_fts MATCH ?`
2. Fallback: `SELECT * FROM knowledge_items WHERE content LIKE '%?%' OR title LIKE '%?%'`

---

### 3.13. Collaboration và Cross-Team

```sql
CREATE TABLE IF NOT EXISTS collaboration_cases (
    id                TEXT PRIMARY KEY,
    correlation_id    TEXT NOT NULL,
    owner_instance_id TEXT NOT NULL REFERENCES team_instances(id),
    target_instance_id TEXT NOT NULL REFERENCES team_instances(id),
    objective         TEXT NOT NULL,
    acceptance_json   TEXT NOT NULL DEFAULT '[]',
    constraints_json  TEXT NOT NULL DEFAULT '{}',
    context_refs_json TEXT NOT NULL DEFAULT '[]',
    priority          TEXT NOT NULL DEFAULT 'medium',
    risk_level        TEXT NOT NULL DEFAULT 'medium',
    status            TEXT NOT NULL DEFAULT 'open',  -- "open" | "closed" | "escalated"
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS collaboration_case_events (
    id         TEXT PRIMARY KEY,
    case_id    TEXT NOT NULL REFERENCES collaboration_cases(id),
    event_type TEXT NOT NULL,  -- "readback" | "decision" | "deliverable" | "review" | "escalation"
    actor_id   TEXT,
    payload    TEXT,           -- JSON
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS delegated_grants (
    id               TEXT PRIMARY KEY,
    case_id          TEXT NOT NULL REFERENCES collaboration_cases(id),
    run_id           TEXT NOT NULL REFERENCES orchestration_runs(id),
    grantor_agent_id TEXT NOT NULL,
    grantee_agent_id TEXT NOT NULL,
    allowed_tools_json TEXT NOT NULL DEFAULT '["all"]',   -- hoặc list cụ thể
    allowed_mcp_json   TEXT NOT NULL DEFAULT '[]',
    token_limit        INTEGER,       -- NULL = unlimited
    expires_at         TEXT,          -- NULL = no expiry
    created_at         TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_grants_agent_run ON delegated_grants(grantee_agent_id, run_id);

CREATE TABLE IF NOT EXISTS cross_team_cases (
    id                  TEXT PRIMARY KEY,
    correlation_id      TEXT NOT NULL UNIQUE,
    owner_instance_id   TEXT NOT NULL,
    target_instance_id  TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'active',
    event_type          TEXT NOT NULL,
    summary             TEXT,          -- max 180 chars
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cross_team_case_events (
    id                   TEXT PRIMARY KEY,
    correlation_id       TEXT NOT NULL,
    from_instance_id     TEXT NOT NULL,
    reply_to_instance_id TEXT NOT NULL,
    event_type           TEXT NOT NULL,
    summary              TEXT,
    payload              TEXT,
    created_at           TEXT NOT NULL
);
```

---

### 3.14. Security

```sql
CREATE TABLE IF NOT EXISTS security_actors (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    type       TEXT NOT NULL DEFAULT 'user',  -- "user" | "agent" | "system"
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS actor_permissions (
    actor_id    TEXT NOT NULL REFERENCES security_actors(id),
    permission  TEXT NOT NULL,      -- "tool:execute:write_file", "tool:execute:run_cli",...
    granted_at  TEXT NOT NULL,
    PRIMARY KEY (actor_id, permission)
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id        TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    action    TEXT NOT NULL,      -- "tool_gateway_allowed", "governance:budget_blocked",...
    user_id   TEXT,
    resource  TEXT NOT NULL,      -- tool_name hoặc resource name
    details   TEXT NOT NULL       -- full context string
);

CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp);
```

**Seeded permissions cho `local-user`:**
- `tool:execute:read_file`
- `tool:execute:analyze_file`
- `tool:execute:write_file`
- `tool:execute:edit_file`
- `tool:execute:run_cli`
- `tool:execute:save_to_knowledge`
- `tool:execute:search_knowledge`
- `tool:execute:handoff_to_team`
- `tool:execute:create_subtasks`
- ... (tất cả built-in tools)

---

### 3.15. LLM Context Tracking

```sql
CREATE TABLE IF NOT EXISTS llm_context_snapshots (
    id                        TEXT PRIMARY KEY,
    run_id                    TEXT REFERENCES orchestration_runs(id),
    session_id                TEXT REFERENCES sessions(id),
    instance_id               TEXT NOT NULL,
    agent_id                  TEXT NOT NULL,
    mode                      TEXT,            -- operating mode
    selected_capabilities_json TEXT NOT NULL,  -- JSON với tokenizer_profile + estimated_tokens
    context_hash              TEXT NOT NULL,   -- SHA256 của full request
    character_count           INTEGER NOT NULL,
    created_at                TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS llm_context_sources (
    id             TEXT PRIMARY KEY,
    snapshot_id    TEXT NOT NULL REFERENCES llm_context_snapshots(id),
    source_kind    TEXT NOT NULL,   -- "request_history" | "tool_result" | "policy_mode" | "knowledge"
    source_id      TEXT NOT NULL,   -- e.g. "request-0" | "request-0-tool-result-2"
    source_hash    TEXT NOT NULL,   -- SHA256 của source content
    rank           INTEGER,         -- ranking nếu là knowledge/search result
    character_count INTEGER NOT NULL,
    trust_level    TEXT NOT NULL,   -- "assembled_request" | "tool_output_untrusted" | "trusted_policy"
    created_at     TEXT NOT NULL
);
```

---

### 3.16. `settings`

```sql
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

**Các key quan trọng:**

| Key | Default | Mô tả |
|---|---|---|
| `orchestration_mode` | `human_interaction` | Operating mode toàn cục |
| `governance_max_tokens_per_run` | `1000000` | Token budget per run |
| `governance_autonomous_sensitive_allowed` | `false` | Enable sensitive tools in autonomous mode |
| `worker_poll_min_ms` | `500` | Min poll interval (ms) |
| `worker_poll_max_ms` | `15000` | Max poll interval (ms) |
| `worker_task_recovery_stale_seconds` | `900` | Stale task threshold (15 phút) |
| `agent_executor_max_tool_iterations` | (dynamic) | Override max iterations |
| `workspace_{instance_id}` | (none) | Workspace path per instance |

---

## 4. Indexes quan trọng

```sql
-- Tìm task nhanh theo instance và status
CREATE INDEX idx_tasks_instance_status ON tasks(team_instance_id, status);

-- Tìm run events theo run
CREATE INDEX idx_run_events_run ON run_events(run_id, created_at);

-- Dedup approval request
CREATE UNIQUE INDEX idx_approval_run_operation ON approval_requests(run_id, operation);

-- Tool invocations theo run và status (cho resume)
CREATE INDEX idx_invocations_run ON tool_invocations(run_id, status);

-- Delegated grants lookup
CREATE INDEX idx_grants_agent_run ON delegated_grants(grantee_agent_id, run_id);

-- Team messages lookup
CREATE INDEX idx_team_messages_instance ON team_messages(team_instance_id, created_at);

-- Conversation turns lookup
CREATE INDEX idx_conversation_turns_session ON conversation_turns(session_id, created_at);

-- Audit timestamp
CREATE INDEX idx_audit_timestamp ON audit_logs(timestamp);
```

---

## 5. Các Pattern đặc biệt

### 5.1. Atomic Task Claim (Race condition prevention)

```sql
-- Chỉ 1 worker thắng nếu nhiều worker race
UPDATE tasks
SET status = 'in_progress',
    assignee_id = :agent_id,
    updated_at = :now
WHERE id = :task_id
  AND status = 'pending'
  AND team_instance_id = :instance_id;
-- Kiểm tra: connection.changes() > 0 → đã claim thành công
```

### 5.2. is_task_unblocked() — Dependency Check

```sql
SELECT COUNT(*) FROM tasks
WHERE id IN (SELECT value FROM json_each(:deps_json))
  AND status != 'completed'
```
→ count == 0 → task unblocked

### 5.3. get_total_tokens_for_run() — Budget Check

```sql
SELECT COALESCE(SUM(token_count), 0) FROM conversation_turns
WHERE session_id IN (
  SELECT session_id FROM orchestration_runs WHERE id = :run_id
)
```

(hoặc từ run_events tracking token usage)

### 5.4. get_next_approved_tool_invocation_for_run() — Approved Resume

```sql
SELECT ti.* FROM tool_invocations ti
JOIN approval_requests ar ON ti.approval_request_id = ar.id
WHERE ti.run_id = :run_id
  AND ti.status = 'awaiting_approval'
  AND ar.status = 'approved'
ORDER BY ti.created_at ASC
LIMIT 1
```

### 5.5. Knowledge FTS5 Search

```sql
-- FTS5 (nếu available)
SELECT ki.* FROM knowledge_fts kf
JOIN knowledge_items ki ON kf.rowid = ki.rowid
WHERE knowledge_fts MATCH :query
ORDER BY rank
LIMIT 10

-- Fallback LIKE
SELECT * FROM knowledge_items
WHERE (title LIKE :pattern OR content LIKE :pattern)
  AND (session_id = :session_id OR :session_id IS NULL)
LIMIT 10
```

---

## 6. Migration Strategy

Hiện tại sử dụng **`CREATE TABLE IF NOT EXISTS`** pattern — mỗi lần khởi động, schema mới tự động được add nếu chưa có. Đây là forward-only migration approach.

Không có migration rollback mechanism — thích hợp cho desktop app có thể rebuild DB.
