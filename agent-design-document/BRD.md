# Business Requirements Document (BRD)
# AgentForge AI — Multi-Agent Desktop Platform

> **Phiên bản:** 2.0 (Dựa trên mã nguồn thực tế)
> **Cập nhật:** 2026-06-12

---

## 1. Tóm tắt điều hành

AgentForge AI là một **desktop application** cho phép cá nhân và tổ chức xây dựng, điều phối, và tự động hóa các nhóm AI agents để thực hiện công việc phức tạp. Khác với các chatbot thông thường, AgentForge cho phép nhiều AI agents cùng làm việc theo team, chia sẻ thông tin, và phối hợp tự động — tất cả với sự kiểm soát chặt chẽ về quyền và tính an toàn.

---

## 2. Bối cảnh kinh doanh

### 2.1. Vấn đề cần giải quyết

| Vấn đề | Tác động |
|---|---|
| Các tác vụ phức tạp đòi hỏi nhiều loại AI khác nhau | Người dùng phải chạy nhiều chat sessions riêng lẻ, không tích hợp |
| AI agents không có memory/context giữa các sessions | Công việc không liên tục, cần nhập lại context liên tục |
| Không có cơ chế kiểm soát an toàn khi AI tự động hóa | Rủi ro về data leak, file system damage |
| Workflow phức tạp không thể tự động hóa dễ dàng | Bottleneck ở con người cho các quy trình lặp lại |
| Cross-team AI collaboration không có governance | Không audit trail, không kiểm soát scope |

### 2.2. Cơ hội

- Nhu cầu tăng về "agentic AI" — AI có thể tự ra quyết định và thực thi
- Desktop-native: không phụ thuộc internet cho UI, data privacy cao hơn
- Tích hợp nhiều LLM providers: Claude, Gemini, OpenRouter, Ollama local

---

## 3. Mục tiêu kinh doanh

### BR-1: Multi-Agent Team Management
Người dùng có thể tạo và quản lý nhiều agents với vai trò khác nhau (Coordinator, Developer, BA, Designer...), tổ chức thành teams và instances.

**Đo lường:** Số lượng agents và teams được tạo; thời gian thiết lập team mới

### BR-2: Governed Automation
Hệ thống tự động hóa công việc với mức độ kiểm soát có thể điều chỉnh: từ toàn bộ manual (Human Interaction) đến supervision đến fully autonomous.

**Đo lường:** Tỷ lệ tasks tự động hóa thành công; số lần cần human intervention

### BR-3: Security và Audit
Mọi hành động của AI (đặc biệt file operations và CLI commands) phải có audit trail. Người dùng luôn có quyền approve/reject.

**Đo lường:** 100% tool invocations có audit record; 0 unauthorized file operations

### BR-4: Workflow Automation (iFlow)
Người dùng có thể thiết kế và chạy workflows phức tạp với branching, loops, human review checkpoints, và agent delegation.

**Đo lường:** Số workflows được tạo và chạy; tỷ lệ workflow completion thành công

### BR-5: Cross-Team Collaboration
Các team instances có thể giao tiếp, handoff work, và request review từ nhau theo giao thức có governance.

**Đo lường:** Cross-team handoffs thành công; audit trail đầy đủ

---

## 4. Stakeholders

| Stakeholder | Role | Nhu cầu chính |
|---|---|---|
| Power User (Developer) | Primary User | Tự động hóa coding tasks, setup multi-agent dev teams |
| Product Manager | Primary User | Orchestrate research + planning + content agents |
| Business Analyst | Primary User | Data analysis workflows với AI agents |
| IT Admin | Secondary | Security, permissions, audit trail |
| End User | Secondary | Simple chat với single agent |

---

## 5. Phạm vi sản phẩm

### 5.1. Trong phạm vi (In-Scope)

✅ Multi-agent team setup và management
✅ 3 operating modes (Human Interaction / Supervision / Autonomous)
✅ Tool execution với governance (file ops, CLI, MCP tools)
✅ iFlow workflow engine (Start, AgentTask, Decision, HumanReview, Delay, Merge, CronTrigger, End)
✅ Cross-team handoff và review protocol
✅ Knowledge base (save + search)
✅ Session và conversation history
✅ Multiple LLM providers (Claude, Gemini, OpenRouter, Custom/Ollama)
✅ Audit trail toàn diện
✅ Token budget enforcement
✅ Role-based task routing

### 5.2. Ngoài phạm vi (Out-of-Scope)

❌ Web-based UI (hiện tại là desktop only)
❌ Multi-user collaboration trên cùng instance (single desktop user)
❌ Cloud deployment / SaaS
❌ Native mobile apps
❌ SystemCommand và HttpRequest nodes trong iFlow (disabled, planned)

---

## 6. Ràng buộc kinh doanh

### 6.1. Ràng buộc kỹ thuật

| Ràng buộc | Lý do |
|---|---|
| Rust/egui stack | Performance, memory safety cho desktop |
| SQLite (không phải server DB) | Desktop app, no server dependency |
| AEAD encryption cho tool payloads | Security: sensitive data không rõ ràng trong DB |
| WAL mode SQLite | Concurrent reads không block |
| 32MB stack size | Recursive async functions trong Rust |

### 6.2. Ràng buộc governance

| Ràng buộc | Impl |
|---|---|
| Coordinator không được tự tạo/sửa files | `role_policy.rs#tool_allowed_for_role()` |
| Mọi file op cần được attach vào traceable run | `tool_gateway.rs` check `run_id` |
| Path traversal (`..`) luôn bị block | `tool_gateway.rs#has_parent_traversal()` |
| Cross-team delegated work phải có persisted grant | `delegated_grants` table |

---

## 7. Yêu cầu phi chức năng kinh doanh

### 7.1. Availability
- App phải khởi động trong vòng 5 giây
- Stale tasks phải được recover trong lần khởi động tiếp theo (15 phút threshold)

### 7.2. Data Privacy
- Tool payloads được AEAD-encrypt trước khi lưu DB
- API keys không được lưu plaintext (reference-based)
- Tool outputs được scan để redact credentials trước khi đưa vào LLM context

### 7.3. Reliability
- Task execution không bị duplicate nhờ atomic claim
- iFlow state được persist sau mỗi step (crash recovery)
- Worker exponential backoff tránh CPU spinning khi idle

### 7.4. Auditability
- 100% tool decisions (allowed/denied/approval_required) có `audit_logs` record
- 100% governance events có `run_events` record
- Token usage được track per run

---

## 8. Dependency map

```
AgentForge AI
├── LLM Providers (external)
│   ├── Claude API (Anthropic)
│   ├── Gemini API (Google)
│   ├── OpenRouter API
│   └── Custom/Ollama (local)
├── MCP Tools (optional, external)
│   └── Any MCP-compatible tool server
└── Local filesystem (workspace operations)
```

---

## 9. Rủi ro kinh doanh

| Rủi ro | Mức độ | Mitigations |
|---|---|---|
| LLM provider API thay đổi format | Trung bình | Provider adapter pattern, easy to update |
| AI thực hiện file ops ngoài ý muốn | Cao | Tool gateway với 3-mode approval; path scope check |
| Token cost vượt budget | Trung bình | `governance_max_tokens_per_run` enforcement |
| SQLite corruption | Thấp | WAL mode; backup recommendations |
| Cross-team scope creep (AI làm ngoài mandate) | Cao | Delegated grant scope; readback protocol |

---

## 10. Điều kiện thành công

| Điều kiện | Metric |
|---|---|
| User có thể setup team với 3+ agents trong < 5 phút | Onboarding time |
| Agent hoàn thành coding task end-to-end không cần intervention | Task completion rate (Autonomous mode) |
| Không có unauthorized file operations xảy ra | Security audit: 0 violations |
| iFlow workflow chạy tự động theo cron schedule | Automation reliability |
| Cross-team review request được process đúng protocol | Collaboration protocol compliance |
