# System Requirements Specification (SRS)
# AgentForge AI — Multi-Agent Desktop Platform

> **Phiên bản:** 2.0 (Từ mã nguồn thực tế)
> **Cập nhật:** 2026-06-12

---

## 1. Giới thiệu

### 1.1. Mục đích

Tài liệu này mô tả đầy đủ các yêu cầu chức năng (functional) và phi chức năng (non-functional) của AgentForge AI, dựa trực tiếp vào mã nguồn Rust đã implement. Mỗi yêu cầu có mã định danh và liên kết với code thực tế.

### 1.2. Phạm vi

AgentForge AI là một **desktop application** cho phép:
- Tạo và quản lý nhiều AI agents
- Tổ chức agents thành teams/instances
- Orchestrate agents theo 3 operating modes
- Tự động hóa workflows (iFlow engine)
- Cross-team collaboration với governance

### 1.3. Ký hiệu

- **FR** = Functional Requirement
- **NFR** = Non-Functional Requirement
- **Impl** = Implemented in (file reference)

---

## 2. Tổng quan hệ thống

### 2.1. Actors

| Actor | Mô tả |
|---|---|
| **Desktop User** | Người dùng trực tiếp, actor_id = `"local-user"` trong security system |
| **Agent** | AI agent nhận tasks, gọi tools, generate responses |
| **System** | Các automated processes: cron, iFlow engine, Worker recovery |
| **Policy Engine** | ToolExecutionGateway kiểm tra policy |

### 2.2. Các boundary chính

```
[Desktop User] ↔ [UI (egui)] ↔ [AppState] ↔ [Services/Orchestration]
                                                    ↕
                                    [SQLite DB] ↔ [DatabasePort]
                                                    ↕
                                    [TeamBusRouter] ↔ [AgentWorkers]
                                                    ↕
                                    [LLM Providers] (Claude/Gemini/OpenRouter/Custom)
```

---

## 3. Yêu cầu chức năng

### FR-1: Quản lý Agent

#### FR-1.1: Tạo Agent

**Mô tả:** User có thể tạo agent với các thuộc tính cấu hình

**Thuộc tính bắt buộc:**
- `name` (String): tên agent, dùng làm fallback routing_role
- `provider` (String): tên LLM provider

**Thuộc tính tùy chọn (trong `config` JSON):**
- `role`: routing role, dùng để match task assignment
- `position`: chức danh
- `details`: mô tả chi tiết
- `responsibilities`: mảng string
- `competencies`: mảng string  
- `allowed_task_types`: whitelist task kinds
- `disallowed_task_types`: blacklist task kinds
- `system_prompt`: optional override system prompt

**Impl:** `core/models/agent.rs`, `infrastructure/database/sqlite_adapter.rs`

#### FR-1.2: Agent Status

**Mô tả:** Agent có thể ở trạng thái `online` hoặc `offline`. Worker chỉ start khi agent `online`.

**Impl:** `AgentWorker::is_agent_online()`, `WorkerManager::start_workers_for_instance()`

#### FR-1.3: Agent Routing Role

**Mô tả:** `routing_role()` = `config["role"]` nếu có, else `name`. Dùng để:
1. Match task trong `pending_task_matches_agent()`
2. Kiểm tra capability trong role_policy
3. Đăng ký với TeamBusRouter

**Impl:** `core/models/agent.rs#routing_role()`

---

### FR-2: Quản lý Teams và Instances

#### FR-2.1: Team Structure

**Mô tả:** Hierarchy: `Team` → `TeamInstance` → `Agent assignments`

- `Team`: nhóm logical
- `TeamInstance`: deployment cụ thể của team (có workspace riêng)
- Một agent có thể được assign vào nhiều instances

**Impl:** `core/models/team.rs`, `sqlite_adapter.rs`

#### FR-2.2: Workspace per Instance

**Mô tả:** Mỗi instance có thể cấu hình workspace directory. Key: `settings["workspace_{instance_id}"]`

**Tác động:** File operations của agent bị giới hạn trong workspace (hoặc cần approval để ra ngoài)

**Impl:** `ToolExecutionGateway::configured_workspace()`

---

### FR-3: Operating Modes

#### FR-3.1: Human Interaction Mode

**Mô tả:** Default mode. **Mọi** tool call (trừ ReadOnly) đều cần human approval.

**Storage value:** `"human_interaction"`

**Impl:** `modes.rs`, `tool_gateway.rs#authorize_runtime()`

#### FR-3.2: Supervision Mode

**Mô tả:** Automation mode với human oversight.
- `ReadOnly` tools → auto-allowed
- `ControlledMutation` tools → auto-allowed (nếu trong workspace)
- `Sensitive` tools → cần approval
- File ngoài workspace → cần approval

**Impl:** `tool_gateway.rs` — Supervision branch

#### FR-3.3: Autonomous Mode

**Mô tả:** Full automation với policy constraints.
- `ReadOnly` tools → auto-allowed
- Workspace file mutation (write_file/edit_file) bởi non-coordinator → auto-allowed
- `Sensitive` filesystem tools ngoài workspace → cần approval
- `Sensitive` non-filesystem tools → **Denied** (trừ khi `governance_autonomous_sensitive_allowed=true`)
- File ngoài workspace → cần approval

**Setting:** `governance_autonomous_sensitive_allowed` = `"true"` để enable

**Impl:** `tool_gateway.rs` — Autonomous branch

#### FR-3.4: Mode Transition

**Mô tả:** User có thể chuyển mode bất kỳ lúc nào. Transition được log với timestamp + reason.

**Constraint:** `can_transition()` block chỉ khi `from == to`

**Impl:** `ModeManager::transition_to()`

---

### FR-4: Tool Execution Gateway

#### FR-4.1: Run Traceability

**Mô tả:** Mọi tool call phải được gắn với một `run_id`. Tool call không có `run_id` → Denied với message "not attached to a traceable run".

**Impl:** `tool_gateway.rs` — check `request.run_id`

#### FR-4.2: Payload Sealing

**Mô tả:** Trước khi kiểm tra policy, tool payload được AEAD-encrypt và lưu vào `tool_invocations` table. 

**Format:** `invocation_id = "{run_id}:{tool_call_id}"`

**Impl:** `tool_gateway.rs#seal_invocation()`

#### FR-4.3: Permission Check

**Mô tả:** Actor (run initiator) phải có permission `"tool:execute:{tool_name}"` trong `actor_permissions` table.

**Default:** `local-user` được cấp các permissions standard khi app khởi tạo (seeding)

**Impl:** `tool_gateway.rs` — `check_actor_permission()`

#### FR-4.4: Delegated Grant Enforcement

**Mô tả:** Khi một agent làm việc trong context cross-team (delegated run):
- Phải có `active_delegated_grant` trong DB
- Tool phải nằm trong `allowed_tools_json` của grant hoặc `"all"`
- Token limit của grant không được vượt

**Impl:** `tool_gateway.rs` — delegated grant check

#### FR-4.5: Coordinator Tool Restriction

**Mô tả:** Agent có routing_role = `"coordinator"` (case-insensitive, alphanumeric normalized) **không được** sử dụng:
- `write_file`, `edit_file`, `delete_file`
- `run_cli`
- `render_document`, `render_pdf`
- `generate_image`, `generate_video`

**Impl:** `role_policy.rs#tool_allowed_for_role()`

#### FR-4.6: Path Traversal Prevention

**Mô tả:** Bất kỳ path nào chứa `..` (parent traversal) → Denied với message "parent-directory traversal is not allowed"

**Impl:** `tool_gateway.rs#has_parent_traversal()`

#### FR-4.7: Approval Deduplication

**Mô tả:** Operation key được tạo từ SHA256 của payload + mode + tool_name + path summary. Nếu approval request với operation key này đã `approved` → không hỏi lại.

**Impl:** `tool_gateway.rs#operation_key()`, `#is_request_approved()`

---

### FR-5: Task Management

#### FR-5.1: Task Model

**Fields:**
- `id`, `run_id`, `team_instance_id`, `assignee_id` (nullable)
- `title`, `status`, `payload` (JSON), `dependencies` (JSON array of task_ids)
- `created_at`, `updated_at`

**Status values:** `pending` → `in_progress` → `completed` | `failed` | `waiting_approval`

#### FR-5.2: Atomic Task Claiming

**Mô tả:** Worker claim task bằng atomic SQL UPDATE với WHERE `status='pending'`. Nếu 2 workers race, chỉ 1 thắng (`affected_rows > 0`).

**Impl:** `sqlite_adapter.rs#claim_task_for_instance()`

#### FR-5.3: Task Dependency

**Mô tả:** Task có thể phụ thuộc vào các tasks khác. `is_task_unblocked()` kiểm tra tất cả dependencies đã `completed`.

**Impl:** `sqlite_adapter.rs#is_task_unblocked()`

#### FR-5.4: Stale Task Recovery

**Mô tả:** Khi app restart, tasks ở trạng thái `in_progress` lâu hơn `worker_task_recovery_stale_seconds` (default 900s = 15 phút) được reset về `pending`.

**Impl:** `WorkerManager::recover_stale_in_progress_tasks()`

#### FR-5.5: Task Capability Validation

**Mô tả:** Trước khi execute task, system validate task type phù hợp với agent capabilities. Vi phạm → task `failed` + error message broadcast.

**Impl:** `AgentWorker::task_capability_violation()`

---

### FR-6: Orchestration Run

#### FR-6.1: Run Lifecycle

```
created → running → [paused] → completed | failed | waiting_approval
```

**Impl:** `orchestration_runs` table, `governance.rs#OrchestrationState`

#### FR-6.2: Run Events

**Mô tả:** Mọi sự kiện trong run được ghi vào `run_events` table:
- `agent_execution_started` / `agent_execution_cancelled`
- `tool_policy_allowed` / `tool_policy_denied` / `tool_policy_approval_required`
- `sealed_invocation_resumed`
- `budget_blocked`
- `llm_request_context_snapshot`
- `workflow_node_waiting_approval`
- `cross_team_run_created` / `cross_team_run_completed` / `cross_team_run_failed`

**Impl:** `executor.rs#record_run_event()`, `governance.rs#enforce_run_budget_for_db()`

#### FR-6.3: Token Budget

**Mô tả:** Per-run token budget được enforce từ `settings["governance_max_tokens_per_run"]` (default: 1,000,000). Khi vượt → `budget_blocked` event + run failed.

**Impl:** `GovernanceManager::enforce_run_budget_for_db()`

---

### FR-7: iFlow Workflow Engine

#### FR-7.1: Workflow Definition

**Mô tả:** Workflow được define bằng JSON với structure:
```json
{
  "id": "...",
  "name": "...",
  "version": "1.0",
  "start_node_id": "node1",
  "team_id": "optional",
  "instance_id": "required for execution",
  "nodes": {
    "node1": { "id": "node1", "name": "Start", "node_type": {"type": "start"}, "next_nodes": ["node2"] }
  }
}
```

#### FR-7.2: Validation Rules

Workflow phải thỏa mãn:
1. Ít nhất 1 node
2. `start_node_id` tồn tại trong `nodes`
3. Start node phải là `Start` hoặc `CronTrigger`
4. Phải có ít nhất 1 `End` node
5. Tất cả destination nodes phải tồn tại
6. `AgentTask` phải có `agent_id` và `instruction` không rỗng
7. Phải có `instance_id` trước khi execute (validate_workflow_draft bỏ qua rule này)
8. `SystemCommand` và `HttpRequest` không được phép (sẽ fail validation)

**Impl:** `engine.rs#validate_workflow_contract()`

#### FR-7.3: Execution Strategies

- **Serial**: Execute từng node một (pop from queue)
- **Parallel**: Execute tất cả nodes hiện tại cùng lúc (drain queue)

#### FR-7.4: AgentTask Dispatch

**Mô tả:** Khi gặp AgentTask node:
1. Build prompt từ instruction + input_vars
2. Gửi `TeamMessage{metadata="iflow_dispatch:{exec_id}:{node_id}"}` trực tiếp tới agent
3. Set `state.status = Paused`
4. Add node_id vào `pending_agent_tasks`
5. Persist state
6. Worker nhận message → execute via `AgentExecutor` → publish `TeamMessage{metadata="iflow_result:{exec_id}:{node_id}"}`
7. IFlowAutomation listener nhận result → `resolve_agent_task()` → resume execution

#### FR-7.5: CronTrigger

**Mô tả:** Workflow với `CronTrigger` start node được IFlowAutomation poll mỗi 500ms. Khi `now >= next_run` → `start_workflow()` với `ExecutionStrategy::Parallel`.

#### FR-7.6: HumanReview

**Mô tả:** Workflow pause và chờ human approve/reject. `resolve_review(execution_id, approved: bool)`:
- approved → set output_var=true, push approved_next
- rejected → set output_var=false, push rejected_next

#### FR-7.7: Delay Node

**Mô tả:** `pending_delays[node_id] = now_ms + duration_ms`. Mỗi step_execution check delays và unblock khi expired.

#### FR-7.8: Approval Resume (iFlow)

**Mô tả:** Khi tool trong iFlow agent task cần approval và được approve:
- `IFlowAutomation::resume_approved_run()` → load version + execution
- `redispatch_pending_agent_tasks()` → gửi lại messages cho agents đang chờ

---

### FR-8: Cross-Team Collaboration

#### FR-8.1: Handoff Types

| Type | Mô tả |
|---|---|
| `message` | Yêu cầu thực thi task từ team khác |
| `handoff` | Chuyển giao công việc |
| `review_request` | Yêu cầu review từ team khác |
| `review_response` | Response của review |
| `status_event` | Status update: `ACK_RECEIVED`, `READBACK_SUBMITTED`, `COMPLETED`, `FAILED` |

#### FR-8.2: Governed Collaboration Case

**Mô tả:** Mọi cross-team interaction tạo ra một `CollaborationCase` với:
- `correlation_id`: UUID liên kết tất cả events của một interaction
- `owner_instance_id`: team gửi
- `target_instance_id`: team nhận

**Impl:** `worker.rs#ensure_governed_case()`, `collaboration.rs`

#### FR-8.3: Readback Protocol

**Mô tả:** Trước khi thực thi delegated work:
1. Receiving team submit readback: "Tôi hiểu yêu cầu là..."
2. `grant_readback_only()` → tạo `DelegatedGrant` trong DB
3. Grant kiểm soát tools nào agent được phép dùng trong run này

#### FR-8.4: Coordinator Selection

**Mô tả:** Agent xử lý cross-team message được chọn bằng:
1. Tìm agent với `routing_role = "coordinator"` (normalized comparison)
2. Nếu không có → agent đầu tiên trong instance

**Impl:** `worker.rs#select_cross_team_coordinator_agent_id()`

---

### FR-9: Knowledge Base

#### FR-9.1: save_to_knowledge

**Mô tả:** Agent có thể gọi tool `save_to_knowledge(title, content)` để lưu thông tin dài hạn vào `knowledge_items` table.

#### FR-9.2: search_knowledge

**Mô tả:** Agent có thể search knowledge base. Nếu FTS5 được setup → full-text search. Fallback → LIKE query.

---

### FR-10: Session và Chat

#### FR-10.1: Session Lifecycle

**Mô tả:** `Session` là context của conversation. Tự động tạo nếu chưa có khi worker cần lưu conversation turn.

#### FR-10.2: Conversation Turns

**Mô tả:** Mỗi agent response được append vào session qua `append_conversation_turn(session_id, "assistant", text, metadata)`.

**Impl:** `worker.rs#try_execute_next_task()` — cuối function

#### FR-10.3: Dynamic System Prompt

**Mô tả:** `ChatService::build_dynamic_system_prompt()` inject:
- Agent profile: `agent_profile_for_prompt()` = name + role + position + responsibilities + competencies + inferred_competencies
- Team context
- Knowledge base context (nếu configured)
- Collaboration case context (nếu có active case)

---

## 4. Yêu cầu phi chức năng

### NFR-1: Performance

| Metric | Target | Impl |
|---|---|---|
| Worker poll interval | 500ms min, 15s max (exponential backoff) | `worker_poll_bounds()` |
| Task claim concurrency | Atomic SQL, no lock contention | `claim_task_for_instance()` |
| Max parallel agents | 10 (configurable) | `GovernancePolicy.max_concurrent_agents` |
| Stale task recovery | 900s threshold | `recover_stale_in_progress_tasks()` |
| Cron poll | 500ms | `IFlowAutomation` |
| iFlow listener poll | 2s | `IFlowAutomation` |

### NFR-2: Security

| Requirement | Impl |
|---|---|
| Tool payload encrypted at rest | AEAD sealing, `keychain.rs` |
| Path traversal prevention | `has_parent_traversal()` |
| Credential redaction in tool output | `redact_untrusted_context()` |
| Actor-based permissions | `actor_permissions` table |
| Audit trail | `audit_logs` table |
| Delegated grant token limit | `delegated_grants.token_limit` |

### NFR-3: Reliability

| Requirement | Impl |
|---|---|
| WAL mode SQLite | `PRAGMA journal_mode = WAL` |
| State crash recovery | iFlow state persisted to DB mỗi step |
| Task recovery after restart | `recover_stale_in_progress_tasks()` |
| iFlow execution restore | `get_state()` fallback to DB |
| Worker restart on agent online | `WorkerManager::start_workers_for_instance()` |

### NFR-4: Scalability

| Constraint | Value |
|---|---|
| Max tool iterations per task | 25 (implementation), 8 (default) |
| Max approved invocation resume | 8 per execute cycle |
| Max concurrent orchestrations | 10 (governance policy) |
| Token budget per run | 1,000,000 tokens (default) |

### NFR-5: Maintainability

- Clean Architecture: `core` → `application` → `infrastructure` (không import ngược)
- `DatabasePort` trait: database-agnostic abstraction
- `BaseProviderAdapter` trait: provider-agnostic LLM abstraction
- Task kinds: 15 canonical types với aliases — dễ extend

### NFR-6: Memory và Resource

- Tokio runtime stack size: **32MB** (override vì recursive async)
- `ProviderAdapterCache`: shared `Arc` across workers — tránh init redundant
- Smart context pruning: summarize messages khi history > 20 messages

---

## 5. Luồng xử lý chính

### 5.1. Luồng: User gửi message → Agent response

```
1. UI: User nhập text → ChatService::send_message()
2. ChatService: append user turn vào session
3. ChatService: build system prompt + load history
4. ChatService: gọi LLM provider stream
5. Stream callback: cập nhật team_message content real-time
6. Final response: lưu assistant turn vào session
```

### 5.2. Luồng: Agent gọi tool

```
1. LLM response chứa tool_call
2. AgentExecutor parse tool_call
3. ToolExecutionGateway::authorize_runtime():
   a. seal_invocation() → encrypt payload → DB
   b. Kiểm tra permissions, mode, path scope, role
   c. Return: Allowed | ApprovalRequired | Denied
4. Nếu Allowed → execute_tool()
5. Nếu ApprovalRequired → response "Approval required before executing..."
   - DB: approval_request{status="pending"}
   - UI hiển thị approval UI
   - User approve/reject
   - Nếu approve: resume_approved_invocations() → execute tool
6. Kết quả được redact → append vào history → LLM tiếp tục
```

### 5.3. Luồng: iFlow workflow execution

```
1. User trigger hoặc CronTrigger
2. IFlowAutomation::dispatch_persisted_workflow() hoặc cron loop
3. WorkflowEngine::start_workflow() → state{status=Running, current_nodes=[start]}
4. IFlowAutomation::run_execution() loop:
   a. step_execution() → execute nodes
   b. Gặp AgentTask → TeamMessage dispatch → state=Paused
   c. Worker nhận message → execute via AgentExecutor
   d. Worker publish iflow_result:{exec_id}:{node_id}
   e. Listener nhận → resolve_agent_task() → state=Running
   f. Tiếp tục step_execution()
5. Gặp End node → state=Completed
```

### 5.4. Luồng: Cross-team review request

```
Sending Team:
1. Agent gọi handoff_to_team(handoff_type="review_request", target_team=T2, reply_to_team=T1)
2. AgentExecutor::execute_tool("handoff_to_team") → publish TeamMessage tới T2

Receiving Team (T2):
3. Worker nhận broadcast message với metadata=[CrossTeamHandoff JSON]
4. parse_cross_team_handoff() → CrossTeamHandoff
5. persist_cross_team_case_event() → DB
6. ensure_governed_case() → tạo CollaborationCase
7. select_review_handler_agent_id() → Coordinator
8. execute_cross_team_review():
   a. start_cross_team_run() → tạo OrchestrationRun
   b. acknowledge_and_readback() → ghi readback
   c. grant_readback_only() → tạo DelegatedGrant
   d. AgentExecutor với CRITIC system prompt
   e. Agent gọi handoff_to_team(handoff_type="review_response") → publish về T1
   f. emit_status_event("COMPLETED") về T1
```

---

## 6. Error Handling

### 6.1. Task Failures

- Capability violation → `mark_task_failed()` + broadcast error
- Provider not found → `mark_task_failed()`
- AgentExecutor error → `mark_task_failed()`

### 6.2. Tool Denied

- Return text bắt đầu bằng `"Tool denied"`, `"Permission denied"`, `"Failed"`, `"Error"`, `" denied"`
- `finalize_invocation()` → detect denied → không seal result → update status="denied"

### 6.3. Budget Exceeded

- `enforce_run_budget_for_db()` → insert `budget_blocked` event → return Err
- Calling code update run status → "failed"

### 6.4. iFlow Error

- `WorkflowStatus::Failed(reason)` → persist state + update run status="failed"
- `SystemCommand`/`HttpRequest` nodes → trả lỗi ngay trong `execute_node()`
