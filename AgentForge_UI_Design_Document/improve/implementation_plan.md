# Implementation Plan (feature/agentforge-impl)

This plan operationalizes the work items in [implementation_checklist.md](file:///workspace/AgentForge_UI_Design_Document/improve/implementation_checklist.md) and implements them in dependency order (schema → services → UI wiring).

## Guiding constraints
- Single source of truth: persistence and routing metadata must live in SQLite and be queryable for replay.
- No parallel rails: Team Workspace must not maintain its own incompatible persistence model separate from TeamBus.
- Instance-scoped by default: roster, messages, and tasks must key off `team_instance_id` (diagram requirement).
- Incremental migration: keep existing tables where necessary, but stop writing new data to legacy paths as soon as the canonical path is available.

## Step 1 — DB schema & APIs (canonical message + conversation model)
1. Extend schema initialization in [db.rs](file:///workspace/agentforge-ui/src/db.rs) to create:
   - `team_messages` (if not present)
   - `conversations` (diagram-required, per-turn persistence)
2. Add DB APIs:
   - `insert_team_message(...)`
   - `list_team_messages(instance_id, filters...)`
   - `append_conversation_turn(session_id, role, content, metadata...)`
   - `list_conversation_turns(session_id)`
3. Update/bridge legacy `messages` reads if needed for existing DBs:
   - Prefer: read both `messages` and `team_messages` during transition; write only to `team_messages`.

## Step 2 — Team Workspace message path migration
1. Update Team Workspace reload to populate `chat_histories` from `team_messages`.
2. Update send flow to:
   - publish TeamBus message
   - persist the message to `team_messages`
   - append to in-memory view for immediate UI feedback

## Step 3 — TeamBus integration (publish/subscribe)
1. Provide an app-wide TeamBus instance (composition root) and inject it into Team Workspace.
2. Use TeamBus router semantics to determine recipients and message_type.
3. Subscribe Team Workspace to receive new messages for the active instance and update UI live.

## Step 4 — Instance-scoped roster + agent selection
1. Implement DB query `get_instance_agents(instance_id)` (or equivalent).
2. Update Members column to render instance roster when an instance is selected.
3. Update Chat response generation to select coordinator/agents from instance roster (not team-wide).

## Step 5 — SessionManagerV2 persistence
1. Implement persistence in SessionManagerV2 using `sessions` + `conversations`.
2. Wire Team Workspace to create/resume a session per team instance.
3. Ensure restart/resume loads conversations consistently.

## Step 6 — Verification and stabilization
1. Run `cargo check` and any repository tests.
2. Add targeted unit tests for:
   - Team message persistence (insert/list)
   - SharedTaskList atomic claim behavior
   - Session append/list conversation turns

