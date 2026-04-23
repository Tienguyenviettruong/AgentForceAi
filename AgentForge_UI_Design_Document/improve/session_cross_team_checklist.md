## Session & Conversation (Multi-Conversation per Instance)
- [x] Add `team_instance_id` (instance_id) linkage to `sessions` table to match diagrams
- [x] Add DB APIs: list sessions for instance, create new session for instance, load latest session
- [x] Update Team Workspace UI to support:
  - [x] New Conversation button
  - [x] Session selector (switch between sessions)
  - [x] Per-session chat history rendering
- [x] Ensure message persistence uses `sessions` + `conversations` (not instance_id as session_id)
- [ ] Verify switching sessions loads correct conversation turns from DB

## Cross-Team Collaboration (Inter-Team)
- [x] Add UI affordance to select a target instance/team for cross-team messages
- [x] Send cross-team messages through TeamBus (route + persist) with source metadata
- [x] Ensure cross-team messages are visible in the target instance message feed
- [ ] Add minimal “handoff” mechanism (optional) to attach task/artifact references in cross-team messages

## UX / Discoverability
- [x] Make “Current workspace folder” clearly visible and persisted per instance
- [ ] Provide an empty-state hint when no workspace is set (where files are stored)

## Verification
- [x] `cargo check`
- [ ] Manual flow: create instance → create 2 sessions → switch sessions → verify histories differ
- [ ] Manual flow: send cross-team message → verify target instance receives it
