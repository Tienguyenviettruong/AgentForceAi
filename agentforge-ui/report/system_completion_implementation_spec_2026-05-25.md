# AgentForge UI - Implementation Plan and System Completion Specification

**Date:** 2026-05-25  
**Updated:** 2026-05-27  
**Input audits:** `report/module_database_logic_audit_2026-05-22.md`, `report/module_database_logic_audit_2026-05-25.md`, `report/system_completion_reassessment_2026-05-26.md`  
**Status:** Phase 0-3 and substantial Phase 4-5 runtime slices are implemented. The 2026-05-27 pass closes iFlow direct side effects, iFlow/cross-team run-bound executor coverage, per-request context inspection, MCP transport/keychain foundations, artifact provenance and normalized dependency enforcement. Production completion remains blocked by exact approval continuation, full Chat-to-iFlow execution authority, external MCP process isolation/catalog installation, remaining credential migrations, learned-promotion workflow and full regression gates. Section 12 supersedes older residual statements.

## 1. Objective

Build AgentForge as one traceable IDE workflow:

```text
User goal in Chat
  -> persisted session and orchestration run
  -> generated/validated iFlow
  -> assigned and executed tasks under mode + permission policy
  -> auditable events, approvals and token cost
  -> artifacts and knowledge with provenance
  -> reusable learned workflow
```

This specification addresses the failures confirmed by audit:

- Knowledge documents, agent memory and FTS storage are not expressed as a coherent product contract.
- Role/MCP permission state is malformed and runtime can follow a different authorization path from UI.
- Orchestration UI shows mock values and its Mode Transition does not control the shared runtime state.
- Chat, iFlow, tasks, logs, governance and monitoring do not share a persisted run identity.
- Multiple persistence contracts exist for messages, dependencies, skills and database locations.
- MCP tools and skills must be selected by the user before they are injected into LLM input.
- Knowledge, orchestration and session context supplied to an LLM request must be traceable as a context snapshot.
- MCP management must model installed servers and local/remote configuration, not a flat tool-runner table.

## 2. Delivery strategy

System completion is split into deployable phases. Each phase removes misleading behavior before adding additional autonomy.

| Phase | Goal | Release gate |
|---|---|---|
| Phase 0 - Integrity and truthful UI | Fix data corruption paths, permission inconsistency and visible mock/orphan state | Existing workflows remain usable; visible panels no longer report fabricated operational state |
| Phase 1 - Execution spine | Introduce persisted run/event/approval/mode contract | Chat action can be traced through tasks, logs, costs and mode |
| Phase 2 - iFlow lifecycle | Make iFlow the inspectable plan/execution surface for a run | A generated flow opens from Chat, validates, executes and resumes after refresh |
| Phase 3 - Knowledge and learned automation | Make outputs/memories traceable and reusable | Every saved knowledge record has source; learned workflows are not confused with built-in tools |
| Phase 4 - Governed autonomy | Enforce policy, RBAC, secrets and approval at all side-effect boundaries | Autonomous mode passes threat, authorization and budget tests |
| Phase 5 - Context assembly and MCP configuration | Select capabilities, configure MCP servers and persist model-input provenance | An LLM request contains only selected MCP/skills and has an auditable context snapshot |
| Phase 6 - Production completion gates | Close all remaining runtime/security/observability/test blockers | End-to-end IDE flow passes release test matrix without placeholder or bypass paths |

## 3. Phase 0 vertical slice

### 3.1 In scope in the current implementation pass

| Work item | Required behavior | Code status |
|---|---|---|
| RBAC legacy repair | Existing routing-role permissions remain readable during transition; runtime authorization is superseded by Phase 4 security principals | Superseded by Phase 4 |
| MCP runtime/UI consistency | MCP runtime and Marketplace use the Phase 4 actor-based gateway; legacy role identifiers are not operational policy inputs | Superseded by Phase 4 |
| Knowledge FTS consistency | Updating a document or memory and its FTS row is one SQLite transaction | Implemented |
| Obsidian idempotent sync | File watcher uses the same normalized-path update path as initial/manual sync | Implemented |
| Knowledge transparency | UI repository no longer hides existing duplicate documents with `GROUP BY title` | Implemented |
| Memory/session semantics | Chat-triggered memory stores real `session_id`; background execution without a session stores no fake instance-as-session value | Implemented |
| Shared Mode state | Orchestration Mode Transition reads/updates the global manager, persists current setting and audits changes | Implemented |
| Honest Orchestration views | Dashboard, Tracking and Logs read tasks/token/audit persistence instead of fixed values/sample tasks | Implemented |

### 3.2 Deliberately deferred from Phase 0

These items require schema or product-context decisions and must not be simulated in UI:

| Deferred item | Reason |
|---|---|
| Delete/merge existing duplicate `knowledge` rows | Requires reviewed migration and chunk/FTS remapping; automatic destructive repair is unsafe |
| Actor-specific MCP roles | Superseded by the Phase 4 local security principal and gateway implementation |
| Full mode enforcement | Superseded in the governed runtime slice; secret isolation and full policy configuration remain open |
| iFlow generation for every actionable chat | Requires run identity, workflow versions and validation policy |
| Secret migration | Requires a real OS keychain/storage integration rather than changing column names |

### 3.3 Phase 1 execution spine implementation status

Implementation pass on 2026-05-25 establishes persistent run identity and truthful runtime projections without claiming governed autonomy or full iFlow lifecycle.

| Work item | Implemented behavior | Status |
|---|---|---|
| Migration foundation | `schema_migrations` records the additive `execution_spine` schema application | Implemented |
| Run lifecycle storage | Added `orchestration_runs`; each accepted chat submission creates a run snapshot with session, instance, goal and operating mode | Implemented |
| Runtime timeline | Added `run_events`; chat start/result, agent execution, tool invocation, workflow creation and linked task state changes write timeline rows | Implemented |
| Task/run relation | Added nullable `tasks.run_id`; tasks created from executor `create_subtasks` retain their originating run | Implemented |
| Cost/run relation | Added nullable `token_usage.run_id`; executor/worker token records retain run context where it exists | Implemented |
| Workflow/run relation | Added nullable `workflows.run_id`; workflows generated by `create_subtasks` are linked back to the run and update `orchestration_runs.workflow_id` | Implemented as current-workflow bridge |
| Approval storage | Added `approval_requests` APIs and Governance pending-count projection | Implemented in Phase 1; enforcement connected in Phase 4 |
| Mode transition history | Added `mode_transitions`; title bar and Orchestration transitions persist from/to mode, actor and reason | Implemented |
| Orchestration projections | Dashboard reads runs; Logs reads run events; Tracking displays task-to-run references; Governance reads persistent approvals/transitions | Implemented |
| Session selection correctness | Fixed latest-session lookup to return the newest session rather than the oldest row of a descending query | Implemented |

Remaining work packages after this pass:

| Item | Reason it remains open |
|---|---|
| `task_dependencies`/task event normalization and scheduler migration | Closed on 2026-05-27: dependency rows are backfilled from payloads, maintained on task upsert, read by workers and enforced atomically during claim |
| `workflow_versions`, validation and `workflow_executions` | Required for selecting/opening/resuming an exact generated iFlow; retained as Phase 2 |
| Approval decision gateway | Closed by the Phase 4 governed runtime slice below |
| Artifact/knowledge run provenance | Requires the unified Knowledge/provenance changes specified for Phase 3 |

The workflow lifecycle, knowledge provenance and governed runtime gateway items in this table are closed by the subsequent implementation sections below. Task dependency normalization is closed by the 2026-05-27 additive migration and runtime claim rule.

### 3.4 Phase 2 iFlow lifecycle implementation status

| Work item | Implemented behavior | Status |
|---|---|---|
| Workflow version/execution persistence | Added `workflow_versions` and `workflow_executions` with migration marker `iflow_lifecycle`; execution state and run status are written during engine progress | Implemented |
| Chat to iFlow trace | A normal chat goal seeds a validated `Start -> AgentTask -> End` flow linked to its run; Chat exposes the selected run's iFlow entry point | Implemented |
| Delegated planning | `create_subtasks` validates generated definitions and stores a run-scoped workflow version before exposing it as executable | Implemented |
| Navigation and reload | Chat and Orchestration select a run through shared app state; iFlow loads its latest validated version and last durable execution state | Implemented |
| Scope correctness | Builder/engine no longer use hardcoded team or instance values; execution requires persisted instance context | Implemented |
| Runtime completion resume | Automation listens on persisted instance broadcasts so paused agent-task flows can resume from worker responses | Implemented |
| Interim side-effect control | `SystemCommand` and `HttpRequest` iFlow nodes fail validation until the governed execution gateway exists | Implemented safety boundary |

Phase 2 limitation: approval generation and actor-aware policy enforcement are not claimed; these remain Phase 4 responsibilities.

### 3.5 Phase 3 knowledge and learned automation implementation status

| Work item | Implemented behavior | Status |
|---|---|---|
| Knowledge provenance schema | Added document `source_kind`, normalized source URI, content hash, originating run/session; added memory instance/run provenance and migration marker `knowledge_provenance` | Implemented |
| Audited document dedupe | Migration canonicalizes Obsidian source identity, retains the latest row for a duplicate source, rebuilds document FTS, cascades obsolete chunks and records every removed/retained pair in `knowledge_migration_audit` | Implemented |
| Idempotent external ingestion | Obsidian sync and document upsert resolve records by normalized external URI; repeated file synchronization updates one canonical document | Implemented |
| Unified Knowledge projection | `KnowledgeService` combines documents and agent memories; Knowledge UI provides All/Documents/Memories views and displays source, run, session and agent provenance | Implemented |
| Retrieval provenance | RAG includes both memory and document matches instead of treating them as fallback stores, and supplies source/run/session identifiers in agent context | Implemented |
| Memory scope | Auto-summary and `save_to_knowledge` persist the actual session, instance and run identifiers available to the executor | Implemented |
| Learned automation separation | Recorded-action workflows are stored as `learned`/`review_required` drafts; Skills UI shows only learned drafts, and automatic execution reads only active run-scoped workflows | Implemented |

Phase 3 update on 2026-05-27: generated output artifacts now have a dedicated provenance model and Orchestration projection. Promoting a reviewed learned draft into an executable run remains open.

### 3.6 Phase 4 governed runtime implementation status

| Work item | Implemented behavior | Status |
|---|---|---|
| Security principal separation | Added `security_actors`, `security_roles`, permission joins and migration marker `governed_tool_gateway`; bootstraps the local desktop actor as owner without using an agent routing role as authorization identity | Implemented |
| Run-scoped delegation | New chat runs store the local actor; every agent tool call requires a matching persisted run, initiating actor and actor permission before execution | Implemented |
| Tool Execution Gateway | `AgentExecutor`, iFlow agent execution, cross-team agent execution and MCP middleware resolve runtime decisions through `ToolExecutionGateway`; direct iFlow shell/HTTP nodes now fail at runtime as well as validation | Partial: durable exact approval continuation and any legacy/non-runtime utilities remain to be retired or isolated |
| Sensitive approvals | CLI, file write/edit, external output generation, web/fetch and MCP runtime operations create persisted approval requests; executor stops with `waiting_approval` rather than falsely completing | Implemented |
| Queue continuation | Queued tasks waiting on approval are stored as `waiting_approval`; approval restores them to `pending` for retry in the same run, while rejection marks the run/task failed | Implemented |
| Workspace boundary | CLI working directories and agent file/output paths are resolved inside the configured instance workspace; absolute escapes and parent traversal are rejected | Implemented |
| Markdown file bypass | Chat no longer interprets model-returned file/edit fenced blocks as filesystem writes; prompts instruct agents to call governed tools | Implemented |
| Budget and audit | Executor checks a persisted per-run token ceiling before provider iterations; gateway decisions and approval actions produce audit/run events without logging raw tool payloads | Implemented |
| Explicit research actions | Research auto-search no longer silently persists a notebook; explicit search/save UI actions are actor-authorized and audited | Implemented |

Phase 4 residual work:

| Item | Reason it remains open |
|---|---|
| Secret storage | MCP/provider/output new paths resolve `secret://` through OS keychain on Windows/macOS and reject raw stored values; legacy credential migration, vault UX and redaction verification remain open |
| Interactive chat approval continuation | A queued task resumes after approval, but a sensitive tool request emitted during a free-form live chat is not durably replayed because raw tool payloads are deliberately not persisted |
| External MCP process sandbox | Runtime authorization and approval occur before spawn, but OS-level isolation of a permitted MCP subprocess is not yet implemented |
| Delegated security scope | Cross-team runs now receive a traceable local initiating actor, but a separately modelled delegated actor/capability grant and budget scope remains open |

### 3.7 Phase 5 context assembly and MCP configuration implementation status

This phase is added from the 2026-05-26 user clarification: selected MCP servers/tools and skills must be injected into the LLM request together with relevant Knowledge and Orchestration context.

| Work item | Implemented behavior on 2026-05-26 | Status |
|---|---|---|
| MCP server persistence | Added `mcp_servers`, linked `mcp_tools.server_id`, and migration marker `context_and_mcp_server_lifecycle` | Implemented |
| Capability selection persistence | Added `capability_selections` with default/run scope; MCP and Skills panels write explicit enable/disable choices | Implemented |
| Skill instruction contract | Built-in skill metadata now carries instructions; executor injects instructions only for enabled skills | Implemented |
| MCP tool exposure contract | Executor injects and executes MCP tools only when the tool is selected and its owning server is enabled | Implemented |
| Context provenance | `llm_context_snapshots` and sources are written once per provider request/iteration, including tool-result and mode sources; Orchestration includes a Context Inspector | Implemented with bounded/redacted untrusted context; tokenizer-level accounting remains open |
| Orchestration context injection | Each run-bound request includes run ID, goal, mode, status and instance in the assembled system context | Implemented |
| MCP settings surface | MCP panel lists servers, toggles selection/state, imports multiple local/remote server definitions, stores secrets through keychain references and refreshes discovered tool schemas | Implemented configuration and refresh UX |
| Unsafe interactive runner removal | MCP panel no longer offers a direct process `Run` action outside a traceable run/gateway | Implemented |

Phase 5 residual work:

| Item | Required completion |
|---|---|
| MCP catalog install and isolation | Stdio/remote JSON-RPC discovery/invocation and refresh are implemented; curated install lifecycle and OS-level subprocess/network/resource sandbox remain open |
| Secret resolution completion | OS-keychain `secret://` resolution, MCP args/URL rejection and provider/output reference-only runtime are implemented; migration of existing raw settings and DB/log negative tests remain open |
| Context safety completion | Retrieved knowledge/tool output are marked untrusted, credential-like lines redacted and retrieval character-budgeted; exact tokenizer budgeting and broader prompt-injection evaluation remain open |
| Run-specific UX | Current UI writes default selections consumed by future/current executions; expose per-session/per-run overrides in Chat/iFlow |
| Unified planning authority | iFlow and cross-team agent runtime use `AgentExecutor`; Chat can still run while a generated iFlow is a linked plan rather than the sole execution authority, and legacy `Orchestrator::decompose_goal` is not yet folded into the same contract |
| Exact approval continuation | Persist and resume the approved invocation payload itself for both free-form chat and queues; hash-based retry is not exact continuation |

## 4. Target bounded contexts

| Context | Owns | Public responsibility |
|---|---|---|
| Identity and Policy | users, memberships, roles, permissions, policy decisions | Resolve who may execute actions |
| Workspace | teams, instances, sessions, messages | Scope user/agent interaction |
| Orchestration | runs, workflow versions, tasks, dependencies, approvals, run events | Execute a goal with traceability |
| Knowledge | documents, memories, artifacts, chunks, indexes, provenance | Store and retrieve usable knowledge |
| Integrations | providers, MCP servers/tools, secrets, skill packages, capability selections | Supply selected external capabilities safely |
| Model Context | context snapshots, retrieved sources, prompt policy inputs | Explain exactly what a model request received |
| Observability | audit, token usage, cost projections | Explain execution and resource usage |

## 5. Canonical workflows

## 5.1 Actionable chat request

1. A user submits a message under an active `team_instance_id` and `session_id`.
2. Chat persists the turn and classifies whether it is informational or actionable.
3. For an actionable goal, `RunService.create_run` creates an `orchestration_run`.
4. Planner generates a validated `workflow_version`; any generated nodes/actions that fail schema validation are rejected and returned as a plan error.
5. Tasks are materialized from workflow nodes and linked by normalized dependencies.
6. Chat displays the generated run/iFlow reference; iFlow opens the same workflow version.
7. Executor routes all side effects through policy checks and writes `run_events`.
8. Artifacts and memories store source run/session/task metadata.

## 5.2 Mode transition

1. Mode may be configured at instance scope and is snapshotted onto a new run.
2. A transition writes a `mode_transition` event with actor, from/to, reason and policy version.
3. UI reads current scope state from the same service used by execution.
4. Tool gateway decides whether execution proceeds, requests approval or rejects based on run mode plus policy.

## 5.3 Knowledge ingestion and retrieval

1. Every document write passes through `KnowledgeIngestionService`, which normalizes source identity.
2. Main row, FTS projection and chunk/index job state are transactionally managed or repairable.
3. Every memory belongs to an agent and optionally a real session/run; instance is a separate field.
4. Retrieval merges ranked document, memory and artifact candidates and returns provenance.

## 5.4 LLM context assembly and MCP capability selection

1. The user enables built-in skills and MCP tools; MCP tools belong to an enabled installed server.
2. A request bound to a run loads effective default/run selections before building the LLM input.
3. The assembler adds trusted orchestration fields (`run`, `goal`, `mode`, `status`, `instance`) and selected skill instructions.
4. Retrieval contributes Knowledge memories/documents/semantic chunks as untrusted retrieved context with source IDs/hashes.
5. Only selected MCP tool schemas are exposed to the LLM; an attempted call to a registered but unselected MCP tool is denied.
6. A `llm_context_snapshot` plus source rows records context hash, selected capability IDs and provenance metadata before the provider call.
7. MCP server execution remains governed and must not be reintroduced as a UI process-run shortcut.

## 6. Target schema additions and migrations

Schema changes require a migration table and ordered migrations. They must run against one configured absolute database location.

## 6.1 Migration foundation

```sql
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    applied_at TEXT NOT NULL
);
```

Startup must:

- Resolve `AGENTFORGE_DB_PATH`, or derive one stable application data path.
- Log the resolved database path.
- Refuse silent creation of a second database when an existing configured database cannot be reached.

## 6.2 Authorization

```sql
CREATE TABLE permissions (
    id TEXT PRIMARY KEY,
    resource TEXT NOT NULL,
    action TEXT NOT NULL,
    UNIQUE(resource, action)
);

CREATE TABLE member_roles (
    member_id TEXT NOT NULL REFERENCES members(id) ON DELETE CASCADE,
    role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY(member_id, role_id)
);

CREATE TABLE role_permissions (
    role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id TEXT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY(role_id, permission_id)
);
```

Migration steps:

1. Convert legacy JSON/string permission fields into normalized permission rows.
2. Add a selected/current actor model before removing the interim built-in admin gate.
3. Preserve routing positions as `agent_profile.position` or `agent_capability`, not RBAC roles.

## 6.3 Execution spine

```sql
CREATE TABLE orchestration_runs (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    initiated_by_member_id TEXT REFERENCES members(id),
    goal TEXT NOT NULL,
    mode TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE run_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES orchestration_runs(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    actor_type TEXT NOT NULL,
    actor_id TEXT,
    task_id TEXT,
    payload TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE approval_requests (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES orchestration_runs(id) ON DELETE CASCADE,
    operation TEXT NOT NULL,
    requested_by TEXT,
    status TEXT NOT NULL,
    resolved_by TEXT,
    decision_reason TEXT,
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE TABLE mode_transitions (
    id TEXT PRIMARY KEY,
    instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE,
    run_id TEXT REFERENCES orchestration_runs(id) ON DELETE CASCADE,
    actor_id TEXT,
    from_mode TEXT NOT NULL,
    to_mode TEXT NOT NULL,
    reason TEXT,
    policy_version TEXT,
    created_at TEXT NOT NULL
);
```

Task normalization:

```sql
CREATE TABLE task_dependencies (
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    depends_on_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    PRIMARY KEY(task_id, depends_on_task_id)
);

CREATE TABLE task_events (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    details TEXT,
    created_at TEXT NOT NULL
);
```

The worker must be changed to read dependency state from `task_dependencies`; payload JSON may retain display input but must not remain the scheduler contract.

## 6.4 Workflow/iFlow

```sql
CREATE TABLE workflow_versions (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    run_id TEXT REFERENCES orchestration_runs(id) ON DELETE SET NULL,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    definition_json TEXT NOT NULL,
    validation_status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(workflow_id, version)
);

CREATE TABLE workflow_executions (
    id TEXT PRIMARY KEY,
    workflow_version_id TEXT NOT NULL REFERENCES workflow_versions(id),
    run_id TEXT NOT NULL REFERENCES orchestration_runs(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    state_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

Rules:

- Remove hardcoded team/instance IDs from the iFlow builder.
- A generated definition must parse and validate before it is saved as executable.
- iFlow panels must receive either `run_id` or active instance context, never invent scope.

## 6.5 Knowledge and provenance

Recommended direction: retain separate documents and memories because retention and editing differ, but present both under one Knowledge service/UI.

```sql
ALTER TABLE knowledge ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE knowledge ADD COLUMN source_uri_normalized TEXT;
ALTER TABLE knowledge ADD COLUMN content_hash TEXT;
ALTER TABLE knowledge ADD COLUMN origin_run_id TEXT REFERENCES orchestration_runs(id);
ALTER TABLE knowledge ADD COLUMN origin_session_id TEXT REFERENCES sessions(id);

CREATE UNIQUE INDEX ux_knowledge_external_source
ON knowledge(source_kind, source_uri_normalized)
WHERE source_uri_normalized IS NOT NULL;

ALTER TABLE knowledge_entries ADD COLUMN instance_id TEXT REFERENCES instances(id);
ALTER TABLE knowledge_entries ADD COLUMN run_id TEXT REFERENCES orchestration_runs(id);
```

Migration for current duplicates:

1. Generate a canonical normalized source URI for existing `vault_path`.
2. Identify duplicate groups before adding the unique index.
3. Retain the most recent canonical row; repoint or rebuild chunks; rebuild FTS from retained document rows.
4. Store a migration audit record listing removed IDs and retained ID.
5. Add the unique index only after duplicate resolution.

## 6.6 Messages and observability

Choose `team_messages` as event transport and project conversation views from session messages, or consolidate both into a canonical `messages` table. Do not continue dual writes with incompatible actor fields.

Required message identity fields:

```sql
sender_type TEXT CHECK(sender_type IN ('user','agent','system','team')),
sender_id TEXT,
session_id TEXT REFERENCES sessions(id),
run_id TEXT REFERENCES orchestration_runs(id),
instance_id TEXT REFERENCES instances(id)
```

`token_usage` must gain `run_id` and optionally `task_id` to make Dashboard cost explainable.

## 6.7 Context assembly and MCP server lifecycle

The implemented Phase 5 additive migration uses:

```sql
CREATE TABLE mcp_servers (
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

ALTER TABLE mcp_tools
ADD COLUMN server_id TEXT REFERENCES mcp_servers(id) ON DELETE SET NULL;

CREATE TABLE capability_selections (
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

CREATE TABLE llm_context_snapshots (
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

CREATE TABLE llm_context_sources (
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
```

Storage rule: MCP `env` values and credential-bearing headers such as `Authorization` accept opaque `secret://...` references only; credential-bearing args and URLs are rejected. On Windows/macOS the resolver stores referenced values in the OS keychain. External authenticated MCP servers still require deployment-level process/network isolation and migration checks before production enablement.

## 7. Service and UI contracts

## 7.1 Required application services

| Service | Responsibility |
|---|---|
| `CurrentContextService` | Current member/team/instance/session selection |
| `AuthorizationService` | Evaluate normalized permissions for actor/resource/action |
| `ModePolicyService` | Load/persist scoped mode and decide approval requirements |
| `RunService` | Create and progress runs; expose projections for Dashboard/Logs |
| `WorkflowService` | Validate/version/execute iFlows linked to a run |
| `KnowledgeService` | Ingest documents/memories, dedupe, retrieve with provenance |
| `ToolExecutionGateway` | Single boundary for MCP/CLI/files/provider side effects |
| `ContextAssembler` | Resolve effective capability selections, retrieval and orchestration policy input; persist LLM context snapshot |
| `McpServerService` | Install/configure/enable MCP servers, discover tools and invoke transports through gateway |

## 7.2 UI data ownership

| UI panel | Must consume |
|---|---|
| Chat | Active session plus generated run/iFlow links |
| iFlow | Workflow version and execution for selected run |
| Orchestration Dashboard | Run/task/event/token projections only |
| Mode Transition | `ModePolicyService`, not panel-local state |
| Governance | Approval/policy/budget data from persistence |
| Knowledge | Documents, memories and artifacts with provenance/type filters |
| Skills | Built-in capabilities can be enabled for LLM input; learned workflow definitions remain separated until reviewed |
| MCP Servers | Installed server/config state, enabled status and tool selections; no direct process runner |
| Context Inspector | Projection of per-request `llm_context_snapshots` and sources, including trust marker and selected capability metadata |
| Artifacts | Projection of generated artifact kind/path/hash linked to run, agent and tool invocation |

## 8. Tool execution policy

All side-effect execution must be funneled through one gateway:

```text
request(actor, instance, session, run, tool, payload)
  -> permission decision
  -> mode/risk decision
  -> optional approval request
  -> workspace/sandbox/secret redaction check
  -> execute
  -> event, audit, usage and artifact recording
```

### Implemented policy boundary in Phase 4 and Phase 5

The application exposes a local desktop security principal separate from business/routing roles. A run snapshots that initiating actor and mode; runtime tools execute only after actor permission checks, and sensitive operations enter persistent approval state. In Phase 5, the MCP surface is a configuration/selection view and no longer executes arbitrary processes interactively. MCP schemas enter model input only when explicitly selected and their installed server is enabled. Legacy routing-role tables remain for business assignment compatibility, not runtime security decisions.

### Final policy gate

- No hardcoded role IDs.
- Agent tool execution runs as the initiating member plus delegated agent scope.
- High-risk file/CLI/MCP actions require approval depending on mode and policy.
- Autonomous mode never bypasses RBAC, sandbox, cost limit or forbidden action list.
- Each LLM call exposes only selected capabilities and persists a reviewable context/provenance snapshot.
- Configured external MCP servers are not production enabled until transport discovery, secret resolution and sandbox execution are complete.

## 9. Verification plan

## 9.1 Phase 0 tests

| Scenario | Expected |
|---|---|
| Start application with built-in role stored as `permissions='all'` | Role is accepted/repaired and MCP Tools no longer incorrectly show `denied` |
| Agent calls an MCP tool without permission | Command is not spawned and audit logs a denial |
| Agent calls an allowed MCP tool | Audit records permission allowance before execution |
| Sync the same Obsidian file from scan and watcher | Both routes update the same document ID; no new duplicate is created |
| Force an FTS write failure during document/memory update | Main row does not commit independently of FTS |
| Open Knowledge with historical duplicates | Duplicate rows are visible for migration review rather than silently hidden |
| Save memory from a chat session | `knowledge_entries.session_id` equals a `sessions.id` value |
| Switch mode through Orchestration | Current mode changes immediately, remains after restart via setting, and creates audit log |
| Open Dashboard/Tracking/Logs | Values and rows correspond to database tasks/token/audit, not sample content |

## 9.2 Phase 1-5 release gates

| Gate | Verification |
|---|---|
| End-to-end traceability | A chat-created run can be followed through flow, task, events, tokens, artifacts and knowledge |
| Persistence across reload | Flow execution state, approvals and mode transitions survive restart |
| Authorization consistency | Same actor/action decision is returned from UI and runtime calls |
| Knowledge correctness | Idempotent ingestion, no FTS drift, session/run provenance and ranked retrieval |
| Governed autonomy | Sensitive side effects stop for approval or policy rejection with complete audit |
| Selected context only | Enable one MCP tool/skill; only those selected capability IDs appear in `llm_context_snapshots` and unselected MCP calls are denied |
| Context provenance | Run request persists Knowledge/orchestration/tool sources with hashes, trust level and selected capability metadata; untrusted injected content is bounded and redacted |
| MCP configuration safety | Local/remote server config rejects raw credentials in env/header/args/URL and resolves approved references through OS keychain; sandbox and migration negative tests remain release blockers |

## 9.3 Current verification result

| Check | Result |
|---|---|
| `cargo check --lib` | Passed on 2026-05-27 after cross-team/context/artifact/dependency changes; only existing unused-method warnings in `src/ui/text` remain |
| Targeted `rustfmt` and `git diff --check` for files changed through 2026-05-27 | Passed; repository reports LF-to-CRLF conversion warnings only |
| `cargo build --bin agentforge-ui` | Passed after changing `fastembed` to dynamically load ONNX Runtime; semantic retrieval now requires an available runtime at execution and degrades with a returned error otherwise |
| Targeted Phase 5 tests before the 2026-05-27 extension | Previously passed individually: selection override, disabled-server exposure and injectable skill instructions |
| Added 2026-05-27 tests | Source adds per-request/tool-source snapshot, artifact provenance, mode-policy and normalized dependency scenarios; execution was intentionally not claimed after the user requested that `cargo test --test integration_tests -- --test-threads=1` not be run |
| `cargo fmt --all -- --check` | Failed: formatting/trailing whitespace remains in pre-existing/other changed panels and worker files; null-byte test source has been replaced and is no longer the cause |
| `cargo test --lib` / full tests | Not yet a passing gate; a sequential lib test attempt was interrupted before completion, so only targeted Phase 5 tests can be claimed |

## 10. Implementation order

| Order | Work package | Dependencies |
|---:|---|---|
| 0 | Phase 0 integrity/truthful UI changes in current branch | Audit complete |
| 1 | Database path selection and `schema_migrations` | None |
| 2 | Normalized member/role/permission model and current actor context | Migration foundation |
| 3 | Run/event/mode/approval schema plus `RunService` | Current actor/context |
| 4 | Task dependency/history migration and worker update | Run/event model |
| 5 | Workflow versions/executions; Chat -> Run -> iFlow navigation | Run/task model |
| 6 | Knowledge provenance/dedupe migration; unified retrieval/UI filters | Run/session model |
| 7 | Tool Execution Gateway, secret storage, governed autonomy | Authorization/run/mode models |
| 8 | Context snapshots, selected Skill/MCP exposure and MCP configuration UX | Run/Knowledge/gateway models |
| 9 | MCP transport discovery/invocation and OS secret resolver (implemented); subprocess sandbox and catalog installation (open) | Phase 5 configuration foundation |
| 10 | iFlow/cross-team executor coverage (implemented); exact approval continuation and full Chat/iFlow authority (open) | Invocation/run/gateway contracts |
| 11 | Artifact/dependency/monitoring projections (implemented in part); learned automation promotion (open) | Unified execution data sources |
| 12 | Automated release gates and corrupted test repair | All critical workflows |

## 11. Completion definition

The system is complete for the intended IDE workflow only when:

- An actionable chat creates or references a persistent run and a validated iFlow.
- iFlow reflects real task execution, approval and logs after reload.
- Operating mode is a persisted policy input enforced by all side effects.
- Permissions are actor-based and enforced identically in UI and runtime.
- Knowledge shows typed, sourced records; ingestion is idempotent and retrieval is consistent.
- Skill/MCP capabilities are explicitly selected; every LLM request has a context snapshot showing supplied orchestration and knowledge provenance.
- MCP servers support safe local/remote configuration, discovery and governed invocation without raw credentials or direct UI process execution.
- Dashboard/Tracking/Logs/Governance and token metrics are projections of the same run history.
- No operational panel relies on mock rows or hardcoded workspace identities.
- The test suite and end-to-end workflow gates pass; no critical blocker remains marked partial or residual in this specification.

## 12. Implementation reconciliation - 2026-05-27

This section is the current status ledger and supersedes conflicting `open` descriptions in the historical phase narrative above.

### 12.1 Delivered in this pass

| Contract | Source evidence | Status |
|---|---|---|
| iFlow direct side-effect boundary | `src/application/iflow_engine/engine.rs` rejects `SystemCommand` and `HttpRequest` during execution even if an old/stale definition bypasses validation; iFlow logs no longer display fixed initializing/waiting rows. | Implemented |
| Unified executor coverage for iFlow task | `src/application/orchestration/worker.rs::execute_iflow_task` resolves the workflow execution's run/session and delegates the LLM call to `AgentExecutor`; unbound executions are denied. | Implemented |
| Cross-team run context | Cross-team review/message handlers create traceable runs using persisted operating mode, call `AgentExecutor` with run/session scope, and only advertise completion after a governed `handoff_to_team` success event. | Implemented runtime slice |
| Per-provider request context provenance | `AgentExecutor` persists one snapshot before each provider request, including full assembled-request hash, mode source and subsequent tool-output source rows. `Orchestration > Context` renders the records. | Implemented |
| Context trust controls | Retrieved knowledge and tool output are explicitly marked untrusted, possible credential lines are redacted, embedded `<tool_call>` tags are neutralized, and injected retrieval is capped by `governance_max_retrieved_context_chars`. | Implemented minimum boundary |
| MCP server runtime | Local stdio and remote MCP JSON-RPC initialize/list/call flows are implemented; the UI supports refresh and multi-server config import, while execution is routed through the gateway. | Implemented foundation |
| MCP secret handling | MCP config rejects credential args/URLs and requires references for sensitive env/headers; the resolver uses the Windows/macOS OS keychain. | Implemented for new MCP configuration |
| Mode runtime policy | Gateway differentiates Human Interaction, Supervision and Autonomous decisions; UI reports the active mutation policy. | Implemented |
| Normalized task dependencies | Migration `artifact_and_dependency_provenance` creates/backfills `task_dependencies`; upsert maintains links and SQL claim/list paths refuse blocked tasks. | Implemented |
| Output provenance | `artifacts` links generated output to run/session/instance/agent/invocation and content hash; `Orchestration > Artifacts` exposes it. | Implemented |
| Monitoring truthfulness | Usage analytics no longer shows fixed trends or resolution time; it displays persisted completed tasks, tokens today and active agent counts. | Implemented limited projection |

### 12.2 Remaining completion blockers

| Priority | Remaining item | Why it is not complete |
|---:|---|---|
| P0 | Exact approval continuation | An approval recognizes the operation hash on retry, but a free-form tool invocation is not persisted as a sealed payload and replayed exactly after approval. |
| P0 | Chat-to-iFlow execution authority | Chat creates/links a validated workflow and runtime tasks are traceable, but the displayed iFlow is not yet the sole authoritative scheduler for every actionable chat response. |
| P1 | Delegated authorization scope | Cross-team execution is traceable under the local initiating actor; separate persisted grants/budgets for delegated agents are not implemented. |
| P1 | MCP production isolation/install lifecycle | Protocol discovery/invocation exists, but external server catalog install, subprocess/network/resource sandboxing and live-server contract tests remain absent. |
| P1 | Secret migration completeness | New MCP/provider/output configuration is protected, while legacy/raw values and migration/negative leakage verification remain open. |
| P1 | Context safety maturity | Character budget and basic redaction are enforced; tokenizer-accurate budgets and adversarial prompt-injection evaluation remain open. |
| P2 | Learned automation promotion | Learned workflow drafts require review, but a governed promote/activate flow with audit and rollback is still not implemented. |
| P2 | Legacy/experimental modules | Unwired prototype modules still contain mock comments/algorithms (`application/knowledge/search.rs`, message-bus security, monitoring System Health shell); they must be deleted, marked experimental or implemented before a broad production claim. |
| P0 | Release gate | Full formatter/test/e2e matrix has not passed. The user requested that the newly extended integration target not be run in this pass. |

### 12.3 Verification performed for this pass

| Check | Result |
|---|---|
| `cargo check --lib` | Pass on 2026-05-27; only existing dead-code warnings in `src/ui/text/mod.rs` and `src/ui/text/state.rs`. |
| `rustfmt` on edited Rust modules except the existing whitespace-heavy chat panel | Pass. |
| `git diff --check` on edited source/test files | Pass; only repository LF/CRLF conversion warnings. |
| Integration/e2e execution for newly added scenarios | Not executed after the user explicitly requested not to run `cargo test --test integration_tests -- --test-threads=1`; no pass claim is made. |

### 12.4 Verdict

The implementation is materially more coherent than the state described by the earlier audit: selected capabilities, model context, iFlow/cross-team execution, dependency scheduling and generated outputs now connect to persisted run provenance. It is still not a production-complete autonomous IDE until the P0 items and security/release gates above are closed.

## 13. Continuation reconciliation va Phase 7/8 extension - 2026-05-27

### 13.1 Runtime fixes delivered after section 12

| Contract | Source evidence | Status |
|---|---|---|
| Provider/output credential reference-only path | `src/infrastructure/security/keychain.rs`, `src/infrastructure/llm_providers/{claude,gemini,openrouter}.rs`, `src/ui/panels/custom_provider.rs`, `src/application/output_tools.rs`, `src/ui/panels/settings.rs` | Implemented cho new configuration/runtime; raw legacy rows can migration/removal. |
| Shared OS credential reference resolution | `resolve_credential_reference` va `SECURE_SECRET_SERVICE`; `src/infrastructure/mcp/server.rs` dung cung service. | Implemented. |
| Executor-only retrieval tren worker va `/run` task path | Raw `search_similar_chunks` prompt assembly da bi xoa khoi `src/application/orchestration/worker.rs` va `src/ui/panels/team_workspace/chat.rs`; retrieval con lai nam trong executor. | Implemented cho hai bypass da audit. |

### 13.2 Remaining blockers sau continuation

| Priority | Contract con mo |
|---:|---|
| P0 | Migrate/remove raw credential legacy va pass leakage tests. |
| P0 | Durable sealed tool invocation va exact approval continuation. |
| P0 | Chat-created iFlow tro thanh execution authority duy nhat. |
| P1 | Tat ca product LLM paths phai di qua context/request service; Research Notebook direct path con mo. |
| P1 | Delegated security grant va MCP subprocess/network/resource isolation. |
| P0 | Full release migration/security/e2e test gate. |

### 13.3 Phase 7 va Phase 8

Specification mo rong cho `Human-like Collaboration Contract` va `Governed Learning and Evolution` duoc ghi tai `report/phase_7_8_humanlike_collaboration_governed_learning_spec_2026-05-27.md`.

| Contract | Source evidence ngay 2026-05-27 | Status |
|---|---|---|
| Phase 7 collaboration persistence va state gate | `src/core/models/collaboration.rs`, `src/infrastructure/database/sqlite_adapter.rs`, `src/application/orchestration/collaboration.rs` | Foundation implemented. |
| Phase 7 delegated authority, routing va projection | `src/application/orchestration/tool_gateway.rs`, `src/application/orchestration/worker.rs`, `src/application/orchestration/executor.rs`, `src/ui/panels/orchestration.rs` | Foundation implemented; resume/voting/escalation operations con mo. |
| Phase 8 evidence, lessons va candidate pipeline | `src/core/models/learning.rs`, `src/application/orchestration/learning.rs`, `src/application/orchestration/executor.rs` | Foundation implemented; validated lessons moi duoc inject. |
| Phase 8 benchmark/canary/promotion/rollback | `src/application/orchestration/learning.rs`, `src/infrastructure/database/sqlite_adapter.rs`, `src/ui/panels/orchestration.rs` | Gated record lifecycle implemented; actual active version materialization va automatic operations con mo. |

Phase 7/8 khong xoa cac blocker P0: sealed invocation/exact approval resume, authoritative iFlow, secret migration, MCP isolation va full release test matrix van la dieu kien bat buoc de tuyen bo IDE autonomous production-grade.

## 14. Authoritative continuation status - 2026-05-27

Muc nay supersede cac residual tai muc 12-13 da duoc dong bang source trong continuation pass.

| Contract | Trang thai moi | Evidence |
|---|---|---|
| Tool invocation/approval | Implemented source-level; verify release | `tool_invocations` co AES-256-GCM sealed payload/result, payload hash, approval link; executor exact-resume va iFlow pending-node redispatch. |
| Normal Chat -> iFlow authority | Implemented cho normal chat; `/run` la queue command rieng | Chat dispatch persisted workflow va AgentTask qua executor; direct normal-chat provider branch compile-disabled. |
| Cross-team delegation | Implemented foundation | User handoff/tool handoff persist case/package; readback-only va expanded grants; gateway fail-closed; consensus/escalation operator resolution. |
| Credential legacy | Implemented migration; verify release | Migration `2026052704` scrub known raw setting/provider/MCP references va ghi audit count. |
| Phase 8 activation | Implemented manual governed lifecycle | Evidence admission, human governor UI, candidate benchmark/canary, promoted skill/workflow materialization va rollback. |
| Knowledge/observability | Implemented projection foundation | Artifact Knowledge projection/context metadata va persisted Monitoring data. |

### 14.1 Remaining release gates

| Priority | Gate |
|---:|---|
| P0 | Upgraded-DB migration, sealed approval restart/resume, negative leakage va full end-to-end release verification chua co pass record; forbidden integration command khong chay theo yeu cau nguoi dung. |
| P1 | MCP external sandbox/catalog install: stdio external bi chan mac dinh tru khi operator explicitly bat unsafe override, nhung chua co isolation runtime that. |
| P1 | Automatic governed evolution: chua co benchmark runner, canary eligible routing/observation va automatic circuit breaker/pullback. |
| P1 | Context security maturity: chua co tokenizer-aware budget va adversarial evaluation gate. |
| P2 | Retire legacy/experimental source: direct-chat block compile-disabled va cac prototype khong tren execution spine. |
