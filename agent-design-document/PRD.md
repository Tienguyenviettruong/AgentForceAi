# Product Requirements Document (PRD)
# AgentForge AI

> **Phiên bản:** 2.0 (Dựa trên mã nguồn thực tế)
> **Cập nhật:** 2026-06-12

---

## 1. Product Vision

AgentForge AI là nền tảng desktop cho phép người dùng xây dựng **đội ngũ AI agents có cấu trúc, có governance, và có khả năng tự động hóa** — từ chat đơn giản đến orchestration phức tạp với cross-team collaboration.

---

## 2. User Personas

### Persona 1: Developer Power User — "Tuấn"
- **Nhu cầu:** Muốn setup một team AI gồm Coordinator + Developer + Tester để tự động hóa coding tasks
- **Pain points:** Phải switch giữa nhiều AI tools, không có context sharing
- **Metric thành công:** 70% tasks có thể chạy tự động trong Autonomous mode

### Persona 2: Product Manager — "Linh"
- **Nhu cầu:** Muốn orchestrate research, analysis, và content creation agents theo workflow
- **Pain points:** Workflow phức tạp không thể encode; human bottleneck ở approval stages
- **Metric thành công:** iFlow workflow chạy cron tự động hàng ngày

### Persona 3: Solo Analyst — "Nam"
- **Nhu cầu:** Chat đơn giản với single agent, save insights vào knowledge base
- **Pain points:** Mất context giữa các sessions
- **Metric thành công:** Knowledge search tìm được insights từ session cũ

---

## 3. Feature Map

### Epic 1: Agent Management

**F1.1 — Agent Config**
- Tạo agent với name, provider, system_prompt
- Cấu hình via `config` JSON: role, position, responsibilities, competencies, allowed/disallowed task types
- Status toggle: online/offline

**F1.2 — Agent Routing**
- `routing_role()` = config["role"] hoặc fallback name
- Task assignment bằng role matching (normalized alphanumeric comparison)
- Coordinator role automatically restricted từ artifact-changing tools

---

### Epic 2: Team & Instance Management

**F2.1 — Team Hierarchy**
- Team → TeamInstance (1:many)
- Agent assignments per instance (many:many)
- Instance workspace directory setting

**F2.2 — Worker Lifecycle**
- Auto-start workers cho agents online khi instance activate
- Exponential backoff polling: 500ms → 15s
- Stale task recovery sau restart (900s threshold)

---

### Epic 3: Chat & Sessions

**F3.1 — Human Interaction Chat**
- Chat với single agent (session-based)
- Real-time streaming (typing indicator)
- Stream markers stripped từ display (`<|channel>thought<channel|>`, etc.)

**F3.2 — Team Chat**
- Multi-agent broadcast trong team instance
- Messages có metadata: agent_name, task_id, stream_kind, thought_duration_secs
- delivery_status: typing → delivered | failed | waiting_approval

**F3.3 — Session Persistence**
- Conversation turns lưu vào DB
- Touch session khi activity mới

---

### Epic 4: Operating Modes

**F4.1 — Human Interaction Mode** (default)
- Tất cả tool calls (ngoài ReadOnly) → cần approval
- UI hiển thị pending approval với operation details

**F4.2 — Supervision Mode**
- ReadOnly + ControlledMutation tools → auto
- Sensitive tools (file ops sensitive, MCP, CLI) → approval

**F4.3 — Autonomous Mode**
- Workspace file mutations (write_file, edit_file) bởi non-Coordinator → auto
- File ngoài workspace → approval
- Sensitive tools → Denied (trừ khi governance_autonomous_sensitive_allowed=true)

**F4.4 — Mode Switcher**
- UI cho phép switch mode runtime
- Mode được persist vào settings["orchestration_mode"]
- Transition history được log

---

### Epic 5: Tool Execution Gateway

**F5.1 — Traceability**
- Mọi tool call phải có run_id
- Tool invocation được log với AEAD-sealed payload

**F5.2 — Approval UI**
- Khi approval required: hiển thị operation details + tool name + payload summary
- User approve/reject
- Approved → resume invocation (open sealed payload + execute)

**F5.3 — Built-in Tools**

| Tool | Risk | Mô tả |
|---|---|---|
| `save_to_knowledge` | ControlledMutation | Lưu thông tin dài hạn |
| `search_knowledge` | ReadOnly | Search knowledge base |
| `handoff_to_team` | ControlledMutation | Chuyển work cho team khác |
| `create_subtasks` | ControlledMutation | Tạo tasks cho agents trong team |
| `read_file` | ReadOnly | Đọc file trong workspace |
| `analyze_file` | ReadOnly | Phân tích file |
| `write_file` | Sensitive | Tạo/ghi file |
| `edit_file` | Sensitive | Chỉnh sửa file |
| `run_cli` | Sensitive | Chạy command |
| `submit_readback` | ControlledMutation | Ghi nhận understanding của delegated work |
| `record_decision` | ControlledMutation | Ghi lại decision trong collaboration case |
| `submit_deliverable` | ControlledMutation | Submit artifact để review |
| `record_review` | ControlledMutation | Review deliverable với verdict |
| `raise_escalation` | ControlledMutation | Escalate case |
| `declare_consensus` | ControlledMutation | Submit consensus proposal |

**F5.4 — MCP Tool Integration**
- `McpToolRegistry` quản lý MCP tool servers
- MCP tools được inject vào tool list
- MCP tools luôn là Sensitive risk

---

### Epic 6: Task Management

**F6.1 — Task Creation**
- Agents tạo tasks cho nhau qua `create_subtasks` tool
- Payload: title, description, role, task_type, dependencies

**F6.2 — Role-Based Routing**
- Unassigned task → agents poll và match bằng `routing_role()`
- Normalized comparison: lowercase, alphanumeric only
- Assignee_id override: nếu có → chỉ agent đó nhận

**F6.3 — Task Dependency**
- `dependencies` = JSON array of task_ids
- Task chỉ được execute khi tất cả dependencies `completed`

**F6.4 — Capability Validation**
- Trước khi execute: validate task type vs agent capabilities
- Violation → fail task + broadcast error → prevent silent misrouting

---

### Epic 7: iFlow Workflow Engine

**F7.1 — Workflow Designer**
- Tạo workflow bằng JSON definition
- Validate trước khi activate: end node, instance scope, no disabled nodes

**F7.2 — Node Types**

| Node | Behavior |
|---|---|
| Start | Entry point |
| CronTrigger | Periodic trigger (interval_ms) |
| AgentTask | Dispatch to specific agent, pause, resume on result |
| Decision | Branch on boolean variable |
| HumanReview | Pause, wait for approve/reject |
| Delay | Wait duration_ms |
| Transform | Map variable Identity/ToString |
| Merge | Join parallel branches |
| End | Complete workflow |

**F7.3 — Execution Strategies**
- Serial: one node at a time
- Parallel: all current nodes simultaneously
- CronTrigger uses Parallel; manual trigger uses Serial

**F7.4 — State Persistence**
- WorkflowState serialized to JSON, lưu vào `workflow_executions`
- Khôi phục sau crash: fallback load từ DB khi state không có trong memory

**F7.5 — Approval Integration**
- Khi iFlow agent task cần tool approval → run paused với `waiting_approval`
- User approve → `IFlowAutomation::resume_approved_run()` → redispatch pending tasks

---

### Epic 8: Cross-Team Collaboration

**F8.1 — Team Handoff**
```
Agent gọi handoff_to_team(
    target_team: "instance-id-T2",
    briefing_package: "Yêu cầu...",
    handoff_type: "message" | "review_request",
    reply_to_team: "instance-id-T1",
    correlation_id: "uuid",
    acceptance_criteria: [...],
    context: {deadline, constraints, related_files}
)
```

**F8.2 — Governed Case Protocol**
1. Receiving team tạo `CollaborationCase` với correlation_id
2. `acknowledge_and_readback()`: submit understanding
3. `grant_readback_only()`: tạo `DelegatedGrant` với scope
4. Execute within grant scope
5. `emit_status_event()`: report back status

**F8.3 — Status Events**
```
ACK_RECEIVED → READBACK_SUBMITTED → COMPLETED | FAILED
```

**F8.4 — Cross-Team Review**
- Sending team request review
- Receiving Coordinator reviews với "ROLE: CRITIC" system prompt
- Must call `handoff_to_team(handoff_type="review_response")` để complete

---

### Epic 9: Knowledge Base

**F9.1 — Save Knowledge**
- `save_to_knowledge(title, content)` → insert vào `knowledge_items`
- Optional embedding (BLOB) cho semantic search

**F9.2 — Search Knowledge**
- FTS5 full-text search nếu available
- Fallback LIKE search
- Session-scoped hoặc global search

---

### Epic 10: LLM Provider Management

**F10.1 — Provider Types**
- Claude (Anthropic) — SSE streaming, multimodal
- Gemini (Google) — SSE streaming, multimodal  
- OpenRouter — proxy nhiều models
- Custom/Ollama — configurable base_url, local models

**F10.2 — ProviderAdapterCache**
- Shared `Arc<ProviderAdapterCache>` giữa tất cả workers của một WorkerManager
- Tránh reinit expensive connections

**F10.3 — Context Management**
- Smart context pruning khi history > 20 messages
- Provider-specific token estimation (Anthropic/Google/OpenAI heuristics)
- LlmContextSnapshot lưu trước mỗi request (hash + estimated_tokens, không lưu raw content)

---

## 4. UI/UX Features

### F-UI.1 — Shell Layout
- **TitleBar**: Mode switcher, menu bar (File/Edit/View/Help), window controls
- **ActivityBar**: Icons dẫn tới các panels (Chat, Agents, Teams, Workflow, Orchestration, ...)
- **StatusBar**: Metrics (agent count, active runs, ...)
- **DockLayout**: Configurable panel layout, persist giữa sessions

### F-UI.2 — Session Panel
- Chat history với streaming support
- Typing indicator khi agent đang respond
- Approval UI: modal với operation details + Approve/Reject buttons
- Message metadata: agent_name, thought_duration_secs

### F-UI.3 — Agent Panel
- List agents với status indicator
- Create/Edit agent form
- Config JSON editor cho role/competencies/...

### F-UI.4 — Orchestration Panel
- List recent runs với status badges
- Run event timeline
- Token usage indicator
- Approval requests queue

### F-UI.5 — Workflow Panel  
- Workflow list với activation status
- JSON editor cho workflow definition
- Execution history

---

## 5. Non-Functional Requirements

| Category | Requirement | Target |
|---|---|---|
| Performance | Worker poll interval | 500ms - 15s (adaptive) |
| Performance | Task claim race | Atomic (no duplicates) |
| Security | Tool payload at rest | AEAD encrypted |
| Security | Credential redaction | Scan all tool outputs |
| Security | Path traversal | Block all `..` paths |
| Reliability | iFlow crash recovery | State persisted per step |
| Reliability | Task recovery | Stale tasks reset after 15min |
| Reliability | DB durability | WAL mode + fsync NORMAL |
| Scalability | Max concurrent agents | 10 (configurable) |
| Scalability | Max tool iterations | 8-25 (by task type) |
| Scalability | Token budget | 1M tokens/run (configurable) |

---

## 6. Acceptance Criteria

### Epic 4 (Operating Modes)
- [ ] Human Interaction: write_file luôn trigger approval dialog
- [ ] Supervision: write_file trong workspace tự động; write_file ngoài workspace → approval
- [ ] Autonomous: write_file trong workspace bởi Developer agent → tự động; bởi Coordinator → Denied

### Epic 5 (Tool Gateway)
- [ ] Tool invocation không có run_id → Denied với message rõ ràng
- [ ] `..` trong path → Denied với message rõ ràng
- [ ] Coordinator gọi write_file → Denied với message rõ ràng
- [ ] Approval sau đó approve → tool được execute với payload đúng

### Epic 7 (iFlow)
- [ ] CronTrigger workflow chạy đúng interval
- [ ] AgentTask pause và resume sau khi agent complete
- [ ] HumanReview pause và resume sau approve/reject
- [ ] Delay node wait đúng duration_ms
- [ ] Decision node branch đúng theo condition variable boolean value
- [ ] Workflow validate fail nếu missing End node

### Epic 8 (Cross-Team)
- [ ] Handoff message được deliver tới target instance
- [ ] Receiving Coordinator selected correctly (role match → fallback first agent)
- [ ] Status events delivered back to reply_to_team
- [ ] DelegatedGrant created before execution
