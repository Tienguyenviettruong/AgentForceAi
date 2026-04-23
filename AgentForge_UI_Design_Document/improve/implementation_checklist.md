# Implementation Checklist (feature/agentforge-impl)

This checklist tracks the implementation of section “6. Improvement Plan (Branch-Specific, Diagram-Aligned, Actionable)” from [improvement_plan.md](file:///workspace/AgentForge_UI_Design_Document/improve/improvement_plan.md).

## Phase 1 — Unify persistence & remove duplication
- [x] Add `team_messages` schema creation into the main DB schema initializer (avoid per-module ad-hoc table creation).
- [x] Add DB APIs for `team_messages`: insert, list by `team_instance_id`, list by routing scope (direct/broadcast/role-group).
- [x] Add `conversations` schema (diagram-required) and DB APIs for conversation turns.
- [x] Migrate Team Workspace chat persistence from `messages` → `team_messages`.
- [x] Deprecate/bridge legacy `messages` usage (keep for backwards compatibility or migrate).

## Phase 2 — TeamBus as canonical messaging layer
- [x] Wire Team Workspace “send” to publish a TeamBus message (direct/broadcast) and persist via `team_messages`.
- [x] Subscribe Team Workspace UI to TeamBus updates (so members/agents can “hear” each other).
- [x] Ensure message routing metadata is stored (message_type, from, to/role-group, instance_id).

## Phase 3 — Instance-scoped membership & tasks
- [x] Make roster queries instance-scoped (members/agents by `instance_id`).
- [x] Update agent selection for response generation to use instance roster (not team-wide).
- [x] Update SharedTaskList to operate on instance-scoped tasks (`team_instance_id`), matching diagrams.

## Phase 4 — Sessions & conversations (SessionManagerV2)
- [x] Persist SessionManagerV2 to SQLite `sessions` and new `conversations`.
- [x] Wire Team Workspace sessions to instances (team-instance session identity).
- [x] Ensure reopen/resume loads conversation turns consistently.

## Phase 5 — Knowledge + Obsidian activation (incremental)
- [x] Add “Select Vault/Folder” flow and persist selection.
- [x] Start Obsidian watcher lifecycle and upsert into DB.
- [x] Add FTS-backed retrieval and inject citations into orchestration/agent prompts.

## Phase 6 — Orchestration as the only execution entry point
- [x] Route Team chat events into Orchestrator (goal decomposition + task persistence).
- [x] Connect Orchestrator → SharedTaskList and Orchestrator → TeamBus.
- [x] Apply ModeManager semantics (Human/Supervision/Auto) to orchestration execution gates.
