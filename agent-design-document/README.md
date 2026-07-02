# README — Agent Design Documents
# AgentForge AI — Tài liệu thiết kế kỹ thuật

> **Phiên bản:** 2.1 — Phân tích sâu từ mã nguồn Rust thực tế
> **Cập nhật:** 2026-06-26

---

## Tổng quan

Thư mục này chứa tài liệu thiết kế đầy đủ cho **AgentForge AI** — một desktop application multi-agent viết bằng Rust. Tất cả tài liệu được phân tích trực tiếp từ mã nguồn (không phải từ giả thiết) để đảm bảo độ chính xác tối đa.

---

## Danh sách tài liệu

| File | Mô tả | Mức độ kỹ thuật |
|---|---|---|
| [BRD.md](./BRD.md) | Business Requirements Document | ⭐ Kinh doanh |
| [PRD.md](./PRD.md) | Product Requirements Document | ⭐⭐ Product |
| [SRS.md](./SRS.md) | System Requirements Specification — đầy đủ FR + NFR | ⭐⭐⭐ Technical |
| [TECHNICAL_SPEC.md](./TECHNICAL_SPEC.md) | Đặc tả kỹ thuật chi tiết từ code | ⭐⭐⭐⭐ Deep Tech |
| [DATABASE_DESIGN.md](./DATABASE_DESIGN.md) | Schema + SQL patterns + DatabasePort | ⭐⭐⭐⭐ Deep Tech |
| [DIAGRAMS.md](./DIAGRAMS.md) | Mermaid diagrams: architecture, flows, ERD | ⭐⭐⭐ Visual |
| [UI_DESIGN.md](./UI_DESIGN.md) | Nguyên tắc và cấu trúc UI desktop | ⭐⭐⭐ UX |
| [ROADMAP.md](./ROADMAP.md) | Kế hoạch phát triển, milestone và checklist | ⭐⭐⭐ Product + Engineering |

---

## Kiến trúc tóm tắt

```
AgentForge AI (Rust / GPUI)
│
├── Core Layer (domain models + DatabasePort trait)
│   ├── models/: Agent, Orchestration, Workflow, Task, ...
│   └── traits/: DatabasePort (abstraction interface)
│
├── Application Layer (business logic)
│   ├── orchestration/: Worker, Executor, Governance, ToolGateway, RolePolicy
│   ├── iflow_engine/: WorkflowEngine, Nodes, Automation
│   └── services/: ChatService, CapabilityRouter
│
├── Infrastructure Layer (adapters)
│   ├── database/: SQLite adapter (WAL, AEAD payloads)
│   ├── llm_providers/: Claude, Gemini, OpenRouter, Custom
│   ├── message_bus/: TeamBusRouter (tokio channels)
│   ├── mcp/: MCP tool registry
│   └── security/: Audit logger, AEAD keychain
│
└── UI Layer (GPUI + gpui-component)
    ├── shell/: TitleBar, ActivityBar, StatusBar, DockLayout
    └── panels/: Session, Agents, Team Workspace, Orchestration, iFlow, Monitoring, ...
```

---

## Các tính năng cốt lõi

### 1. Multi-Agent Teams
- Team → TeamInstance → Agent assignments
- Routing bằng `routing_role()` = config JSON["role"] hoặc fallback name
- WorkerManager: spawn AgentWorker per online agent

### 2. Tool Execution Gateway (Policy Engine)
Luồng decision: `run_id check → actor permission → delegated grant → role policy → path scope → operating mode → Allowed/ApprovalRequired/Denied`

### 3. Three Operating Modes
| Mode | Tool behavior |
|---|---|
| Human Interaction | Tất cả mutations → approval |
| Supervision | Sensitive → approval; Controlled → auto |
| Autonomous | Workspace mutations → auto; Sensitive → Denied (configurable) |

### 4. iFlow Workflow Engine
State machine với 9 node types. State được persist sau mỗi step. AgentTask dispatch qua TeamBus và resume khi agent complete.

### 5. Cross-Team Collaboration
Governed protocol: Handoff → CollaborationCase → Readback → DelegatedGrant → Execute → Status Events

### 6. Workspace-First Execution
- Slash commands là điểm vào nhanh cho goal, spec, plan, review, test và verify
- Orchestration run, workflow execution, run event, tool approval và artifact đều có persistence
- Virtual Office cung cấp bề mặt quan sát cho agent team trong Team Workspace

---

## Công nghệ chính

| Công nghệ | Mục đích |
|---|---|
| Rust | Application code |
| GPUI + gpui-component | Native desktop UI, dock panels, input và component system |
| rusqlite | SQLite storage (WAL mode) |
| tokio | Async runtime cho background services; app thread GPUI dùng stack 64MB |
| serde_json | JSON serialization |
| sha2 | SHA256 payload hashing |
| AEAD | Tool payload encryption |
| reqwest | HTTP/SSE streaming |

---

## Cách đọc tài liệu

1. **Bắt đầu với BRD.md** — hiểu context kinh doanh và vấn đề cần giải quyết
2. **Đọc PRD.md** — feature map đầy đủ theo Epics
3. **Đọc SRS.md** — yêu cầu chức năng + phi chức năng chi tiết với error cases
4. **Tham khảo TECHNICAL_SPEC.md** — khi cần hiểu implementation chi tiết (structs, algorithms, flows)
5. **Xem DIAGRAMS.md** — khi cần visualize kiến trúc hoặc data flows
6. **Tham khảo DATABASE_DESIGN.md** — khi cần làm việc với DB (schema, SQL patterns, indexes)
7. **Xem ROADMAP.md** — trước khi bắt đầu feature mới để biết phạm vi, dependency và Definition of Done

---

## Phạm vi tài liệu

- Các tài liệu đặc tả mô tả năng lực đã kiểm chứng từ code tại thời điểm cập nhật.
- `ROADMAP.md` mô tả kế hoạch; một hạng mục trong roadmap không được hiểu là đã triển khai.
- Khi feature thay đổi schema, policy hoặc workflow contract, tài liệu liên quan phải được cập nhật cùng pull request.
