# UI Design Document — AgentForge AI

> **Phiên bản:** 2.0 (Dựa trên mã nguồn UI thực tế)
> **Cập nhật:** 2026-06-12
> **Framework:** egui (immediate mode GUI) + wgpu (GPU rendering)

---

## 1. Tổng quan kiến trúc UI

AgentForge AI sử dụng **egui** — một immediate mode GUI framework cho Rust. Không có component tree hay state management framework; thay vào đó, mỗi frame render lại toàn bộ UI dựa trên `AppState`.

### 1.1. Shell Layout

```
┌─────────────────────────────────────────────────────────┐
│ TitleBar: [Mode Switcher] [Menu Bar] [Window Controls]   │
├───┬─────────────────────────────────────────────────────┤
│ A │                                                      │
│ c │           Main Content Area (Panels)                │
│ t │           DockLayout manages panel positions         │
│ i │                                                      │
│ v │                                                      │
│ i │                                                      │
│ t │                                                      │
│ y │                                                      │
│ B │                                                      │
│ a │                                                      │
│ r │                                                      │
├───┴─────────────────────────────────────────────────────┤
│ StatusBar: [Agent count] [Active runs] [Mode] [Metrics] │
└─────────────────────────────────────────────────────────┘
```

### 1.2. UI Files Structure

```
ui/
├── shell/
│   ├── title_bar.rs      (11KB) — Mode switcher, menus, window controls
│   ├── activity_bar.rs   (6KB)  — Left sidebar icons
│   ├── status_bar.rs     (5KB)  — Bottom status metrics
│   ├── dock_layout.rs    (1KB)  — Panel dock management
│   └── app_menus.rs      (4KB)  — Menu bar items
├── panels/
│   ├── session.rs        (33KB) — Main chat panel
│   ├── agents.rs         (22KB) — Agent management
│   ├── orchestration.rs  (119KB)— Orchestration + approval UI (lớn nhất)
│   ├── knowledge.rs      (66KB) — Knowledge base browser
│   ├── iflow_builder.rs  (92KB) — Visual workflow builder
│   ├── mcp_marketplace.rs(46KB) — MCP tool management
│   ├── custom_provider.rs(34KB) — Custom LLM provider config
│   ├── settings.rs       (24KB) — App settings
│   ├── research_notebook.rs(27KB)— Research notebook
│   ├── profile.rs        (14KB) — Agent profile view
│   ├── monitoring.rs     (6KB)  — System monitoring
│   └── team_workspace/         — Team workspace panels
└── framework/            — Panel registry, dock engine
```

---

## 2. Shell Components

### 2.1. TitleBar — `shell/title_bar.rs`

**Components:**
- **Mode Switcher**: Dropdown/segmented control để chọn `HumanInteraction | Supervision | Autonomous`
  - Lưu selection vào `settings["orchestration_mode"]`
  - Color-coded theo mode (green/yellow/red)
- **Application Title**: "AgentForge AI"
- **Menu Bar**: File | Edit | View | Agents | Teams | Help
- **Window Controls**: Minimize, Maximize, Close (native OS controls via eframe)

**Mode Switcher behavior:**
```
Human Interaction → màu xanh an toàn
Supervision       → màu vàng cẩn thận  
Autonomous        → màu đỏ tự động hoàn toàn
```

### 2.2. ActivityBar — `shell/activity_bar.rs`

Icons theo chiều dọc bên trái, mỗi icon kích hoạt một panel:

| Icon | Panel | Mô tả |
|---|---|---|
| 💬 Chat | Session | Chat với agent |
| 🤖 Agents | Agents | Manage agents |
| 👥 Teams | Team Workspace | Manage teams |
| 🎯 Orchestration | Orchestration | Run management, approvals |
| ⚡ iFlow | iFlow Builder | Workflow designer |
| 📚 Knowledge | Knowledge | Knowledge base |
| 🔧 MCP | MCP Marketplace | MCP tools |
| ⚙️ Settings | Settings | App settings |
| 📊 Monitor | Monitoring | System metrics |

### 2.3. StatusBar — `shell/status_bar.rs`

Bottom bar hiển thị:
- Online agents count
- Active orchestration runs count  
- Current operating mode badge
- Memory/token usage indicator
- Last activity timestamp

### 2.4. DockLayout — `shell/dock_layout.rs`

Quản lý panel positions. Layout được persist vào settings.

---

## 3. Session Panel — `panels/session.rs`

Panel quan trọng nhất cho daily use.

### 3.1. Layout

```
┌─────────────────────────────────────┐
│ [Session Selector]    [Agent Info]  │
├─────────────────────────────────────┤
│                                     │
│   Chat History                      │
│                                     │
│   [User]: message                   │
│   [Agent 🤖]: response              │
│   [Agent 🤖]: [typing...]           │
│                                     │
│   ⚠️ APPROVAL REQUIRED              │
│   Tool: write_file                  │
│   Path: src/main.rs                 │
│   [Approve] [Reject]                │
│                                     │
├─────────────────────────────────────┤
│ [Text Input...            ] [Send]  │
└─────────────────────────────────────┘
```

### 3.2. Message Rendering

**Message types:**
- User message: background khác biệt
- Agent message với metadata:
  - `agent_name`: hiển thị tên agent
  - `thought_duration_secs`: thời gian xử lý
  - `stream_kind`: "task_worker" | "cross_team_review" | "cross_team_message"
  - `delivery_status`: "typing" (spinner) | "delivered" | "failed" | "waiting_approval"

**Streaming:**
- Message với `delivery_status="typing"` → hiển thị với animation
- Nội dung được update real-time qua `update_team_message_content()`
- Stream markers bị stripped: `<|channel>thought<channel|>`, etc.

### 3.3. Approval UI

Khi agent response bắt đầu bằng `"Approval required before executing..."`:

```
┌────────────────────────────────────┐
│ ⚠️ Governance Approval Required    │
├────────────────────────────────────┤
│ Tool: write_file                   │
│ Path: /workspace/src/main.rs       │
│ Operation: Create/modify file      │
│ Mode: Supervision                  │
├────────────────────────────────────┤
│ [✅ Approve]      [❌ Reject]      │
└────────────────────────────────────┘
```

---

## 4. Orchestration Panel — `panels/orchestration.rs` (119KB — lớn nhất)

### 4.1. Sub-sections

```
┌──────────────────────────────────────────┐
│ Tabs: [Runs] [Tasks] [Approvals] [Events]│
└──────────────────────────────────────────┘
```

**Runs Tab:**
- List recent orchestration runs với status badges
- Status colors: running=blue, completed=green, failed=red, waiting_approval=yellow
- Click để xem run events timeline

**Tasks Tab:**
- List tasks per instance
- Status filter: pending | in_progress | completed | failed | waiting_approval
- Task details: title, role, dependencies

**Approvals Tab:**
- Queue của pending approval requests
- Operation details: tool name + path/command + payload summary
- Approve/Reject buttons
- Approved → resume invocation + re-run agent

**Events Tab:**
- run_events timeline với event_type + payload
- Filter by run_id hoặc event_type

### 4.2. Orchestration Run Detail

```
Run ID: abc123
Goal: Implement REST API endpoint
Mode: Supervision
Status: ● running
Started: 2026-06-12 08:30:15

Timeline:
  08:30:15 [agent]   agent_execution_started
  08:30:18 [policy]  tool_policy_allowed → read_file
  08:30:22 [policy]  tool_policy_approval_required → write_file (src/main.rs)
  08:31:05 [policy]  tool_policy_allowed → write_file (APPROVED)
  08:31:10 [agent]   sealed_invocation_resumed → write_file completed

Token Usage: 4,521 / 1,000,000
```

---

## 5. iFlow Builder — `panels/iflow_builder.rs` (92KB)

### 5.1. Visual Workflow Canvas

```
┌─────────────────────────────────────────────────────┐
│ [Workflow: "Daily Report"] [Activate] [Run Now]      │
├──────────────┬──────────────────────────────────────┤
│ Node Palette │  Canvas                               │
│              │                                       │
│ ○ Start      │  ┌──────────┐   ┌───────────────┐    │
│ ⏰ Cron      │  │  Start   ├──►│  AgentTask:   │    │
│ 🤖 AgentTask │  └──────────┘   │  Analyst      │    │
│ 🔀 Decision  │                 └───────┬───────┘    │
│ 👤 Review    │                         │            │
│ ⏳ Delay     │                 ┌───────▼───────┐    │
│ 🔄 Transform │                 │  Decision     │    │
│ ⊕ Merge     │                 │  condition:   │    │
│ ⏹ End       │                 │  has_data     │    │
│              │                 └──┬────────┬──┘    │
│              │                  YES│        │NO      │
│              │               ┌────▼──┐  ┌──▼───┐   │
│              │               │Report │  │ End  │   │
│              │               └───────┘  └──────┘   │
└──────────────┴──────────────────────────────────────┘
```

### 5.2. Node Configuration

Khi click vào node → right panel hiển thị properties:

**AgentTask Node:**
```
Node ID: task_analyst
Name: Run Analysis
Agent: [Dropdown: Analyst (online)]
Instruction: [Multi-line text]
Input Variables: [result_prev]
Output Variable: analysis_result
```

**Decision Node:**
```
Node ID: check_data
Condition Variable: has_data (bool)
True → next_node_a
False → next_node_b
```

**CronTrigger Node:**
```
Interval: [500] ms
→ Run every 500ms
```

### 5.3. Validation UI

Khi save workflow → validate tự động:
- ✅ Valid: "Workflow saved successfully"
- ❌ Error: "Node 'task1' references missing destination 'task99'"

---

## 6. Agent Panel — `panels/agents.rs`

### 6.1. Agent List

```
Agents (5 online, 2 offline)
┌────────────────────────────────────┐
│ ● Coordinator (online)  Provider: Claude │
│   Role: Coordinator | Instance: Dev Team │
│ ● Developer (online)    Provider: Gemini │
│   Role: Developer | Instance: Dev Team   │
│ ○ Designer (offline)    Provider: Ollama │
│   Role: Designer                         │
└────────────────────────────────────┘
[+ New Agent]
```

### 6.2. Agent Create/Edit Form

```
Name: [Developer]
Status: [○ Online] [○ Offline]
Provider: [Claude ▼]

System Prompt:
[Multi-line text area]

Configuration (JSON):
{
  "role": "Developer",
  "position": "Senior Rust Developer",
  "responsibilities": ["implement", "test", "review"],
  "competencies": ["rust", "backend", "testing"],
  "allowed_task_types": ["implementation", "testing"]
}
```

---

## 7. Knowledge Panel — `panels/knowledge.rs` (66KB)

### 7.1. Layout

```
┌─────────────────────────────────────┐
│ [Search: _______________] [Search]  │
├──────────┬──────────────────────────┤
│ Items    │  Content View            │
│          │                          │
│ 📄 Title1│  # Title: Architecture   │
│ 📄 Title2│  Content: The system uses│
│ 📄 Title3│  Clean Architecture...   │
│          │                          │
│ [+ New]  │  [Edit] [Delete]         │
└──────────┴──────────────────────────┘
```

### 7.2. Search

- FTS5 search (nếu available): full-text match với ranking
- Fallback: LIKE search trên title + content
- Kết quả sorted bởi relevance rank

---

## 8. Settings Panel — `panels/settings.rs`

### 8.1. Sections

**LLM Providers:**
- List providers (Claude, Gemini, OpenRouter, Custom)
- Add/Edit/Delete provider
- Fields: name, adapter_type, model, api_key_ref, base_url

**Governance:**
- `governance_max_tokens_per_run`: input number
- `governance_autonomous_sensitive_allowed`: checkbox
- `worker_poll_min_ms`, `worker_poll_max_ms`: sliders

**Workspace:**
- Per-instance workspace directory picker
- Key: `workspace_{instance_id}`

**Operating Mode:**
- Default mode selector (sync với TitleBar)

---

## 9. MCP Marketplace — `panels/mcp_marketplace.rs` (46KB)

### 9.1. Layout

```
┌─────────────────────────────────────┐
│ MCP Tool Servers                    │
├───────────┬─────────────────────────┤
│ Installed │ Available               │
│ ─────── │ ─────────────────────── │
│ server1  │ filesystem-server        │
│ server2  │ [Install]               │
│ [Config] │ database-server          │
│ [Remove] │ [Install]               │
└───────────┴─────────────────────────┘
```

---

## 10. Design Principles

### 10.1. Immediate Mode (egui)

- **Không có React-style state lifting**: AppState là single source of truth
- **Mỗi frame**: UI được rebuild hoàn toàn từ current state
- **Response time**: ~16ms render cycle (60fps target)

### 10.2. Streaming Updates

- Agent responses stream in real-time qua TeamBus
- UI re-renders khi message content được cập nhật
- `update_team_message_content()` trong DB + TeamBus broadcast → UI pick up next frame

### 10.3. Modal Approvals

- Approval requests được hiển thị inline trong chat hoặc trong Approvals tab
- Non-blocking: user có thể tiếp tục chat trong khi approval pending
- Approval action → gọi DB update → agent resume trên background thread

### 10.4. Color Coding

| Element | Color |
|---|---|
| Online agent | Xanh lá |
| Offline agent | Xám |
| Running status | Xanh dương |
| Completed status | Xanh lá |
| Failed status | Đỏ |
| Waiting approval | Vàng |
| Human Interaction mode | Xanh |
| Supervision mode | Vàng |
| Autonomous mode | Cam/Đỏ |
| Coordinator role | Tím |

---

## 11. Accessibility và UX

- **Keyboard shortcuts**: Đăng ký qua keybindings trong lib.rs init
- **Font**: egui default fonts, có thể override qua FontDefinitions
- **Theme**: Light/Dark toggle (egui built-in visuals)
- **Panel resize**: Drag-to-resize giữa các panels
- **Scroll**: Tất cả panels có scroll support

---

## 12. Panels được Register trong lib.rs

Thứ tự khởi tạo trong `init()`:
1. `register_panel("session")` → SessionPanel
2. `register_panel("agents")` → AgentsPanel
3. `register_panel("orchestration")` → OrchestrationPanel
4. `register_panel("iflow_builder")` → IFlowBuilderPanel
5. `register_panel("knowledge")` → KnowledgePanel
6. `register_panel("mcp_marketplace")` → McpMarketplacePanel
7. `register_panel("settings")` → SettingsPanel
8. `register_panel("monitoring")` → MonitoringPanel
9. `register_panel("team_workspace")` → TeamWorkspacePanel
10. `register_panel("custom_provider")` → CustomProviderPanel
11. `register_panel("research_notebook")` → ResearchNotebookPanel
12. `register_panel("profile")` → ProfilePanel

---

## 13. Main.rs — App Entry Point

```rust
fn main() {
    // Override stack size: 32MB (needed for recursive async functions)
    let builder = std::thread::Builder::new().stack_size(32 * 1024 * 1024);
    builder.spawn(|| {
        // Create tokio runtime
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        
        // Init eframe window
        eframe::run_native(
            "AgentForge AI",
            NativeOptions {
                viewport: ViewportBuilder::default()
                    .with_inner_size([1400.0, 900.0]),
                renderer: eframe::Renderer::Wgpu,  // GPU renderer
                ..Default::default()
            },
            Box::new(|cc| Box::new(App::new(cc, runtime))),
        )
    }).unwrap().join().unwrap();
}
```
