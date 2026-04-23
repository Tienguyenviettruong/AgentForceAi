# AgentForge — feature/agentforge-impl: Current-State Assessment & Improvement Plan (Diagram-Aligned)

This document is written against the correct code branch **feature/agentforge-impl** (commit `d0a3cc8`) and cross-checked against [AgentForge_Diagrams.md](file:///workspace/AgentForge_UI_Design_Document/AgentForge_Diagrams.md).

## 0. Target Architecture (What the Diagrams Require)

The diagrams define an explicit integration contract across subsystems:
- **Orchestration Engine** is the system’s “brain”: receives goals, decomposes work, schedules tasks, manages dependencies/budgets, coordinates agents, and aggregates results ([AgentForge_Diagrams.md](file:///workspace/AgentForge_UI_Design_Document/AgentForge_Diagrams.md#L436-L479)).
- **SessionManagerV2** creates/resumes sessions, persists conversation turns to SQLite, and provides recovery ([AgentForge_Diagrams.md](file:///workspace/AgentForge_UI_Design_Document/AgentForge_Diagrams.md#L207-L238)).
- **AgentManagerV2** executes work through provider adapters and participates in TeamBus while injecting Knowledge context ([AgentForge_Diagrams.md](file:///workspace/AgentForge_UI_Design_Document/AgentForge_Diagrams.md#L53-L109)).
- **Team Collaboration** is instance-scoped: `team_instances` → `team_members` + `team_tasks` + `team_messages`, with TeamBus routing semantics (direct/broadcast/role-group) and SharedTaskList atomic claiming ([AgentForge_Diagrams.md](file:///workspace/AgentForge_UI_Design_Document/AgentForge_Diagrams.md#L272-L385), [AgentForge_Diagrams.md](file:///workspace/AgentForge_UI_Design_Document/AgentForge_Diagrams.md#L388-L433)).
- **Knowledge Base (Brains)** persists to SQLite (`knowledge_entries` + FTS5), syncs with Obsidian vault, and feeds RAG context injection ([AgentForge_Diagrams.md](file:///workspace/AgentForge_UI_Design_Document/AgentForge_Diagrams.md#L578-L637)).
- **Operating Modes** (Human / Supervision / Autonomous) change orchestration behavior, not just UI labeling ([AgentForge_Diagrams.md](file:///workspace/AgentForge_UI_Design_Document/AgentForge_Diagrams.md#L483-L537)).

## 1. What Exists in feature/agentforge-impl (Subsystem Inventory)

This branch contains broad scaffolding across nearly all diagram domains:

### 1.1 App wiring & UI shell
- Global state + DB initialization: [lib.rs](file:///workspace/agentforge-ui/src/lib.rs#L79-L106)
- Main window routing model (activity bar + dock areas + dedicated team page): [lib.rs](file:///workspace/agentforge-ui/src/lib.rs#L262-L540)
- Activity bar (page IDs): [activity_bar.rs](file:///workspace/agentforge-ui/src/activity_bar.rs)
- Title bar mode dropdown wired to ModeManager: [title_bar.rs](file:///workspace/agentforge-ui/src/title_bar.rs#L24-L124)

### 1.2 Data layer (SQLite)
- Full schema creation is implemented in [db.rs](file:///workspace/agentforge-ui/src/db.rs#L171-L325) including:
  - provider configs/templates
  - teams/roles/instances/agents/members
  - tasks/messages/sessions/workflows/knowledge/audit_log

### 1.3 Provider adapter layer
- Adapter interface + ChatMessage/ChatResponse: [providers/mod.rs](file:///workspace/agentforge-ui/src/providers/mod.rs#L16-L73)
- Adapter registry (factory + caching): [providers/registry.rs](file:///workspace/agentforge-ui/src/providers/registry.rs#L6-L57)
- OpenRouter adapter with real HTTP call (401 errors come from config/api_key issues, not a missing module): [providers/openrouter.rs](file:///workspace/agentforge-ui/src/providers/openrouter.rs#L67-L149)

### 1.4 Team collaboration building blocks (exist, not integrated end-to-end)
- TeamBus router (Direct/Broadcast/RoleGroup): [teambus/routing.rs](file:///workspace/agentforge-ui/src/teambus/routing.rs#L8-L196)
- TeamBus persistence store for `team_messages`: [teambus/persistence.rs](file:///workspace/agentforge-ui/src/teambus/persistence.rs#L6-L120)
- SharedTaskList atomic claim logic on `tasks`: [tasks/shared_task_list.rs](file:///workspace/agentforge-ui/src/tasks/shared_task_list.rs#L17-L74)

### 1.5 Orchestration & modes (exist, not driving execution yet)
- Orchestration module map: [orchestration/mod.rs](file:///workspace/agentforge-ui/src/orchestration/mod.rs#L1-L6)
- Orchestrator core (DAG tasks + dependency resolver + state machine): [orchestration/core.rs](file:///workspace/agentforge-ui/src/orchestration/core.rs#L4-L260)
- ModeManager (OperatingMode): [orchestration/modes.rs](file:///workspace/agentforge-ui/src/orchestration/modes.rs#L1-L65)
- Orchestration UI panel (presentational): [panels/orchestration.rs](file:///workspace/agentforge-ui/src/panels/orchestration.rs#L11-L86)

### 1.6 Session subsystem (exists, not wired)
- SessionManagerV2 is implemented as in-memory lifecycle + concurrency guard with TODO persistence: [session/manager.rs](file:///workspace/agentforge-ui/src/session/manager.rs#L16-L63)
- No references exist outside `/src/session/*` (i.e., not used by Team chat or Session panel).

### 1.7 Knowledge subsystem (exists, partially wired, not activated)
- Knowledge module entry: [knowledge/mod.rs](file:///workspace/agentforge-ui/src/knowledge/mod.rs)
- Obsidian vault watcher + DB upsert: [knowledge/obsidian.rs](file:///workspace/agentforge-ui/src/knowledge/obsidian.rs#L101-L170)
- Knowledge panel is still mock UI (no DB queries): [panels/knowledge.rs](file:///workspace/agentforge-ui/src/panels/knowledge.rs#L77-L116)

## 2. Runtime Data Flow (What Actually Happens Today)

### 2.1 Navigation flow & where “Teams” lives
- The app uses DockAreas per page, persisted via `dock_layout` (most pages).
- The **Teams** view is special-cased: when `active_page == "teams"`, it renders the Team Workspace entity directly ([lib.rs](file:///workspace/agentforge-ui/src/lib.rs#L526-L533)).

### 2.2 Team chat execution path (current implementation)
In Team Workspace chat, sending a message triggers:
1. UI appends a user message to an instance-scoped in-memory history and persists it via `db.save_message` ([team_workspace/chat.rs](file:///workspace/agentforge-ui/src/panels/team_workspace/chat.rs#L240-L266)).
2. UI selects an agent by querying `db.get_team_agents(team_id)` and taking the first result ([team_workspace/chat.rs](file:///workspace/agentforge-ui/src/panels/team_workspace/chat.rs#L281-L285)).
3. UI loads provider config via `db.get_provider_by_name(&agent.provider)` and calls the provider adapter directly ([team_workspace/chat.rs](file:///workspace/agentforge-ui/src/panels/team_workspace/chat.rs#L285-L336)).
4. UI persists the assistant output via `db.save_message` again ([team_workspace/chat.rs](file:///workspace/agentforge-ui/src/panels/team_workspace/chat.rs#L301-L310)).

This is a direct UI-driven provider invocation. The diagram-intended path (UI → SessionMgr → Orch → AgentMgr → Provider) is not enforced.

### 2.3 Operating mode (current implementation)
- Title bar allows selecting mode and updates ModeManager state ([title_bar.rs](file:///workspace/agentforge-ui/src/title_bar.rs#L53-L67)).
- Team chat does not consult ModeManager; orchestration does not drive execution. Therefore mode has no effect on execution logic yet.

## 3. Persistence Model Assessment (Schema vs Diagram)

### 3.1 What is persisted and where
**In the main app DB schema** ([db.rs](file:///workspace/agentforge-ui/src/db.rs#L171-L325)):
- Team definitions: `teams`
- Instances: `instances` (team-scoped)
- Members: `members` (team_id plus optional instance_id)
- Tasks: `tasks` keyed by `team_id`
- Messages: `messages` keyed by `team_id` + optional `instance_id` ([db.rs](file:///workspace/agentforge-ui/src/db.rs#L261-L271))
- Knowledge: `knowledge`
- Sessions: `sessions` (but no conversations table)

**In TeamBus MessageStore**:
- Separate `team_messages` table created and used in [teambus/persistence.rs](file:///workspace/agentforge-ui/src/teambus/persistence.rs#L14-L34).

### 3.2 Core mismatch: duplicated “message persistence”
- Team UI uses `messages` table via `Database::save_message/get_messages`.
- TeamBus uses `team_messages` table via `MessageStore`.

Because these are separate stores, TeamBus cannot “replay” UI chat logs and UI cannot subscribe to TeamBus history without glue code. This is the main structural reason the modules feel disconnected.

### 3.3 Core mismatch: task scoping
- SharedTaskList operates on `tasks(team_id, ...)` ([tasks/shared_task_list.rs](file:///workspace/agentforge-ui/src/tasks/shared_task_list.rs#L28-L55)).
- The diagrams require tasks to be scoped by `team_instance_id` to support multiple concurrent instances and correct isolation.

### 3.4 Core mismatch: session persistence
The diagrams require `sessions` and `conversations` (per-turn persistence). In code:
- `sessions` table exists, but SessionManagerV2 does not persist to it ([session/manager.rs](file:///workspace/agentforge-ui/src/session/manager.rs#L54-L62)).
- No `conversations` table exists in the implemented schema.

## 4. Business Workflow Evaluation (What a User Actually Experiences)

### 4.1 What works end-to-end today
- Create teams/instances/agents via dialogs (seeded SDG example exists) and browse them via Team Workspace ([lib.rs](file:///workspace/agentforge-ui/src/lib.rs#L133-L135), [team_workspace/mod.rs](file:///workspace/agentforge-ui/src/panels/team_workspace/mod.rs#L84-L99)).
- Per-instance chat history is persisted and reloaded via `messages(team_id, instance_id)` ([team_workspace/mod.rs](file:///workspace/agentforge-ui/src/panels/team_workspace/mod.rs#L88-L94)).
- Provider configuration exists and OpenRouter can execute real calls if the API key is correct ([providers/openrouter.rs](file:///workspace/agentforge-ui/src/providers/openrouter.rs#L85-L113)).

### 4.2 What is still “mocked” or missing at the workflow level
- **No multi-agent collaboration loop**: there is no Coordinator driving other agents; the system picks the first agent and returns one response.
- **No TeamBus-based awareness**: members do not see each other’s actions or messages, because TeamBus is not used by Team Workspace UI.
- **No SharedTaskList usage**: tasks are not created/claimed/completed as a first-class workflow during chat or orchestration.
- **No session recovery / conversation turns**: restart persistence exists for `messages`, but not for session-scoped conversation logs and not in a way consistent with the diagrams’ session model.
- **Knowledge/Obsidian is not operational**: vault watcher exists but is not started and UI is mock content.
- **Orchestration UI is not backed by orchestration core**: the Orchestrator is not invoked; Orchestration panel shows placeholder metrics.

## 5. Root Cause Analysis (Why Integration is Still Weak)

### 5.1 Two competing architectures are present
- The codebase contains diagram-aligned “infrastructure modules” (TeamBus, Orchestrator, SessionManagerV2, SharedTaskList).
- The Team UI implements its own simplified execution path using direct provider calls and a separate persistence schema (`messages`).

This results in “parallel rails”:
- UI rail: `TeamWorkspace chat → DB messages → direct provider call`
- Architecture rail: `TeamBus → team_messages`, `SharedTaskList → tasks`, `Orchestrator`, `SessionManagerV2`

### 5.2 Missing integration points
To match the diagrams, at least these integration points must exist:
- TeamWorkspace must publish/subscribe via TeamBus and persist to `team_messages`.
- Orchestrator must be invoked from user events (chat goals) and drive task creation/assignment.
- SessionManagerV2 must persist sessions + conversation turns and be called by UI flows.
- Knowledge retrieval must be invoked as part of orchestration/agent execution, not ad-hoc.

## 6. Improvement Plan (Branch-Specific, Diagram-Aligned, Actionable)

### Phase 1 — Choose canonical tables and remove duplication
- Make a decision: either
  - adopt the diagram schema naming fully (`team_*`, `knowledge_entries`, `conversations`), or
  - formally revise the diagrams to match `instances/members/tasks/messages`.
- Recommended: adopt diagram schema to prevent long-term drift.

Concrete actions:
- Migrate `messages` → `team_messages` (or replace the `messages` table usage) so TeamWorkspace and TeamBus share one source of truth.
- Migrate `tasks(team_id)` → `team_tasks(team_instance_id)` and update SharedTaskList accordingly.
- Add `conversations` table and enforce per-turn persistence.

### Phase 2 — Make TeamBus the execution bus (not optional)
- TeamWorkspace chat should:
  - create a `TeamMessage` (broadcast/system/direct) and route via `TeamBusRouter`
  - persist via the same repository used by TeamBus persistence (not via a separate `db.save_message`)
  - subscribe UI to broadcast channel for the active instance

### Phase 3 — Make SharedTaskList the coordinator’s work queue
- Orchestrator output should be written into SharedTaskList tasks.
- Agents should claim tasks atomically and post progress/completions via TeamBus.
- UI should render tasks per instance (not team-wide) to match the diagrams.

### Phase 4 — Activate SessionManagerV2 and connect it to UI + DB
- Wire SessionManagerV2 into:
  - TeamWorkspace (team instance sessions)
  - Session panel (direct agent sessions)
- Implement persistence to `sessions` and `conversations`.
- Implement recovery: load active sessions from DB on startup.

### Phase 5 — Knowledge becomes a first-class orchestration dependency
- Add “Select Obsidian Vault” flow and start watcher lifecycle.
- Replace `knowledge` table with diagram-consistent `knowledge_entries` + FTS5 (or add FTS5 side table).
- Add retrieval APIs that return citations and respect team/session scope.
- Inject retrieval results from Orchestration/AgentManager level.

### Phase 6 — Enforce operating mode semantics
- ModeManager state must control:
  - whether Orchestrator proceeds automatically
  - whether review gates stop execution
  - whether agent-to-agent messages are executed without human intervention

## 7. Immediate Next Steps (Most Leverage)

1. Unify message persistence: migrate TeamWorkspace from `messages` to TeamBus `team_messages` and publish via TeamBusRouter.
2. Make membership and tasks instance-scoped: update DB + APIs so `get_team_agents` and SharedTaskList operate on instance membership.
3. Add `conversations` table and use SessionManagerV2 for persistence and replay.
4. Start Obsidian watcher from a user-selected vault path and show sync state in Knowledge UI.
5. Connect Orchestrator to real persistence and render its state in Orchestration panel (no hard-coded metrics).
