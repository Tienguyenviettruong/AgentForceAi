# AgentForge UI - Code Wiki Documentation

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Architecture Overview](#2-architecture-overview)
3. [Core Layer](#3-core-layer)
4. [Application Layer](#4-application-layer)
5. [Infrastructure Layer](#5-infrastructure-layer)
6. [UI Layer](#6-ui-layer)
7. [Dependency Relationships](#7-dependency-relationships)
8. [Running the Project](#8-running-the-project)

---

## 1. Project Overview

### Project Name
**AgentForge UI** - A Multi-AI Orchestration Platform

### Project Type
Desktop Application (Cross-platform: Windows, macOS, Linux)

### Core Technology Stack
- **Language**: Rust (Edition 2021)
- **UI Framework**: GPUI (gpui) v0.2.2
- **UI Component Library**: GPUI Component v0.5.1
- **Database**: SQLite (rusqlite v0.31.0)
- **Async Runtime**: Tokio v1.51.1
- **HTTP Client**: Reqwest v0.12

### Key Dependencies
| Category | Library | Purpose |
|----------|---------|---------|
| UI | gpui, gpui-component | Cross-platform UI framework |
| Database | rusqlite | SQLite database adapter |
| Async | tokio, futures | Async runtime and streams |
| Serialization | serde, serde_json | Data serialization |
| LLM | reqwest | HTTP requests to AI providers |
| Embeddings | fastembed | Local embedding generation |
| Security | ring, keyring | Encryption and key management |

---

## 2. Architecture Overview

### Layered Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         UI Layer                                 │
│  (panels, shell, framework, components, text)                    │
├─────────────────────────────────────────────────────────────────┤
│                      Application Layer                           │
│  (orchestration, agents, teams, skills, knowledge, research,      │
│   iflow_engine, cross_team_router, doc_engine, services)         │
├─────────────────────────────────────────────────────────────────┤
│                        Core Layer                                │
│  (models, traits, errors)                                        │
├─────────────────────────────────────────────────────────────────┤
│                     Infrastructure Layer                         │
│  (database, llm_providers, mcp, message_bus, monitoring,         │
│   security, performance, fs)                                    │
└─────────────────────────────────────────────────────────────────┘
```

### Entry Point
- **Main Entry**: [main.rs](file:///workspace/agentforge-ui/src/main.rs)
- **Library Root**: [lib.rs](file:///workspace/agentforge-ui/src/lib.rs)
- **Initialization**: `init()` function sets up themes, global state, panels, and key bindings

### Global State Management
The `AppState` struct (defined in [lib.rs](file:///workspace/agentforge-ui/src/lib.rs#L83-L100)) manages:
- Database connection
- Tok runtime
- Message bus router
- Mode manager
- Governance manager
- Service instances (chat, team, knowledge)
- Obsidian watcher

---

## 3. Core Layer

### 3.1 Models (`src/core/models/`)

The core data structures used throughout the application.

#### Key Models

| Model | File | Description |
|-------|------|-------------|
| `Agent` | [agent.rs](file:///workspace/agentforge-ui/src/core/models/agent.rs) | AI agent definition with routing role |
| `Team` | [team.rs](file:///workspace/agentforge-ui/src/core/models/team.rs) | Team container for agents |
| `Instance` | [team.rs](file:///workspace/agentforge-ui/src/core/models/team.rs) | Running instance of a team |
| `WorkflowRecord` | [workflow.rs](file:///workspace/agentforge-ui/src/core/models/workflow.rs) | Workflow definition |
| `WorkflowExecutionRecord` | [workflow.rs](file:///workspace/agentforge-ui/src/core/models/workflow.rs) | Workflow execution state |
| `OrchestrationRunRecord` | [orchestration.rs](file:///workspace/agentforge-ui/src/core/models/orchestration.rs) | Orchestration execution run |
| `CollaborationCaseRecord` | [collaboration.rs](file:///workspace/agentforge-ui/src/core/models/collaboration.rs) | Cross-agent collaboration case |
| `Task` | [task.rs](file:///workspace/agentforge-ui/src/core/models/task.rs) | Task definition |
| `ChatMessage` | [chat.rs](file:///workspace/agentforge-ui/src/core/models/chat.rs) | Chat message |
| `Provider` | [provider.rs](file:///workspace/agentforge-ui/src/core/models/provider.rs) | LLM provider configuration |

#### Agent Model
```rust
pub struct Agent {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub system_prompt: Option<String>,
    pub config: Option<String>,  // JSON config with role, position, details
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
```

#### Collaboration Models
Supports human-like collaboration with:
- `HandoffPackageRecord`: Case transfer between agents
- `CaseReadbackRecord`: Understanding verification
- `CaseDecisionRecord`: Decision tracking
- `CaseDeliverableRecord`: Deliverable submission
- `CaseReviewRecord`: Deliverable review
- `CaseConsensusRecord`: Consensus building
- `CaseEscalationRecord`: Escalation handling
- `DelegatedGrantRecord`: Execution delegation

### 3.2 Traits (`src/core/traits/`)

Abstract interfaces defining system contracts.

#### DatabasePort
[database.rs](file:///workspace/agentforge-ui/src/core/traits/database.rs)

The primary database interface with 100+ methods covering:
- Agent/Team/Instance CRUD operations
- Task management
- Session management
- Knowledge storage and search
- Collaboration case management
- Workflow execution
- Orchestration runs
- Security and RBAC
- MCP tool registry

#### LlmProviderPort
[llm_provider.rs](file:///workspace/agentforge-ui/src/core/traits/llm_provider.rs)

Interface for LLM provider adapters.

#### EventBusPort
[event_bus.rs](file:///workspace/agentforge-ui/src/core/traits/event_bus.rs)

Interface for event distribution.

#### FileSystemPort
[file_system.rs](file:///workspace/agentforge-ui/src/core/traits/file_system.rs)

Interface for file system operations.

---

## 4. Application Layer

### 4.1 Orchestration (`src/application/orchestration/`)

The orchestration engine manages multi-agent task execution.

#### Core Module
[core.rs](file:///workspace/agentforge-ui/src/application/orchestration/core.rs)

**OrchestrationStateMachine** - State machine for orchestration runs:
- States: `Planning`, `Executing`, `Paused`, `Completed`, `Failed`
- Valid transitions enforced

**DagTask** - Task definition with:
- Dependencies (DAG structure)
- Priority
- Deadline
- Assignee

**DependencyResolver** - Topological sort with cycle detection

**Orchestrator** - Main orchestration engine:
- Task decomposition
- DAG execution
- State management

#### Modes Module
[modes.rs](file:///workspace/agentforge-ui/src/application/orchestration/modes.rs)

**OperatingMode** - System operating modes:
- `HumanInteraction`: Direct human-agent chat
- `Supervision`: Agent-agent collaboration with human monitoring
- `Autonomous`: Fully autonomous execution

**ModeManager** - Mode transition management with history tracking

#### Collaboration Module
[collaboration.rs](file:///workspace/agentforge-ui/src/application/orchestration/collaboration.rs)

**CollaborationService** - Human-like collaboration contract:
- `create_handoff()`: Create case transfer
- `acknowledge_and_readback()`: Understanding verification
- `accept_readback()`: Human approval
- `record_decision()`: Decision tracking
- `submit_deliverable()`: Deliverable submission
- `review_deliverable()`: Peer review
- `record_consensus()`: Consensus building
- `cast_consensus_vote()`: Voting
- `escalate()`: Escalation handling
- `route_by_competency()`: Competency-based routing

#### Other Orchestration Modules
| Module | Description |
|--------|-------------|
| `executor.rs` | Agent execution engine |
| `governance.rs` | Governance and policy enforcement |
| `tool_gateway.rs` | Tool execution gateway |
| `worker.rs` | Worker management |
| `learning.rs` | Governed learning system |
| `benchmark_runner.rs` | Benchmark execution |
| `primitives.rs` | Common primitives |
| `briefing.rs` | Context briefing generation |

### 4.2 Agents (`src/application/agents/`)

AI agent lifecycle management.

| Module | Description |
|--------|-------------|
| `lifecycle.rs` | Agent creation, update, deletion |
| `identity.rs` | Agent identification |
| `template.rs` | Agent templates |
| `health.rs` | Health monitoring |
| `monitoring.rs` | Agent metrics |

### 4.3 Teams (`src/application/teams/`)

Team management and role-based access.

#### Role Management
[role.rs](file:///workspace/agentforge-ui/src/application/teams/role.rs)

**Role** - Role definition:
```rust
pub struct Role {
    pub id: String,
    pub name: String,
    pub permissions: Vec<String>,
}
```

**RoleManager** - Role lifecycle and permission checking

#### Team Member Management
[member.rs](file:///workspace/agentforge-ui/src/application/teams/member.rs)

### 4.4 Skills (`src/application/skills/`)

Built-in skill catalog and execution framework.

#### Built-in Skills
```rust
pub fn builtin_skill_catalog() -> Vec<SkillMetadata> {
    vec![
        CodeReviewSkill,
        CodeGenerateSkill,
        CodeDebugSkill,
        WebSearchSkill,
        DocumentAnalysisSkill,
        DataExtractionSkill,
        SummarizeSkill,
        TranslateSkill,
        ExplainSkill,
    ]
}
```

### 4.5 Knowledge (`src/application/knowledge/`)

Long-term memory and knowledge management.

| Module | Description |
|--------|-------------|
| `core.rs` | Knowledge item management |
| `search.rs` | Semantic search with embeddings |
| `tokens.rs` | Token counting and estimation |

### 4.6 IFlow Engine (`src/application/iflow_engine/`)

Workflow automation engine.

| Module | Description |
|--------|-------------|
| `engine.rs` | Workflow state machine |
| `nodes.rs` | Node definitions |
| `automation.rs` | Automation triggers |

### 4.7 Research (`src/application/research/`)

Research and web synthesis.

| Module | Description |
|--------|-------------|
| `core.rs` | Core research functionality |
| `web.rs` | Web content fetching |
| `synthesis.rs` | Research synthesis |

### 4.8 Cross-Team Router (`src/application/cross_team_router/`)

Inter-team communication and coordination.

| Module | Description |
|--------|-------------|
| `routing.rs` | Message routing |
| `coordination.rs` | Cross-team coordination |
| `protocols.rs` | Communication protocols |

### 4.9 Services (`src/application/services/`)

Application-level services.

| Service | Description |
|---------|-------------|
| `chat_service.rs` | Chat functionality |
| `team_service.rs` | Team operations |
| `knowledge_service.rs` | Knowledge operations |
| `provider_factory.rs` | LLM provider factory |

### 4.10 Other Application Modules

| Module | Description |
|--------|-------------|
| `session_manager/` | Session lifecycle |
| `tasks/` | Task management |
| `doc_engine/` | Document processing |
| `marketplace_service/` | MCP marketplace |
| `cost_optimization/` | Cost management |
| `capability_router.rs` | File capability routing |

---

## 5. Infrastructure Layer

### 5.1 Database (`src/infrastructure/database/`)

SQLite-based persistence layer.

#### SQLite Adapter
[sqlite_adapter.rs](file:///workspace/agentforge-ui/src/infrastructure/database/sqlite_adapter.rs)

Implements `DatabasePort` trait with:
- WAL mode for concurrency
- Foreign key enforcement
- Full CRUD for all entities
- Full-text search (FTS)
- Vector similarity search for embeddings

**Key Tables:**
- `provider_configs` - LLM provider configurations
- `teams` - Team definitions
- `agents` - Agent definitions
- `instances` - Team instances
- `tasks` - Task queue
- `sessions` - Chat sessions
- `messages` - Chat messages
- `knowledge_items` - Knowledge base
- `knowledge_chunks` - Embedding chunks
- `collaboration_cases` - Collaboration cases
- `workflows` - Workflow definitions
- `orchestration_runs` - Execution runs

### 5.2 LLM Providers (`src/infrastructure/llm_providers/`)

Multi-provider support with adapter pattern.

#### BaseProviderAdapter Trait
```rust
pub trait BaseProviderAdapter: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn capabilities(&self) -> ModelCapability;
    fn initialize(&mut self, config: &Provider) -> Result<()>;
    fn send_message(&self, messages: Vec<ChatMessage>) -> Future<Result<ChatResponse>>;
    fn send_message_stream(&self, messages: Vec<ChatMessage>) -> StreamResult;
    fn check_health(&self) -> Future<Result<bool>>;
}
```

#### Provider Implementations

| Provider | File | Description |
|----------|------|-------------|
| Claude | [claude.rs](file:///workspace/agentforge-ui/src/infrastructure/llm_providers/claude.rs) | Anthropic Claude |
| Gemini | [gemini.rs](file:///workspace/agentforge-ui/src/infrastructure/llm_providers/gemini.rs) | Google Gemini |
| OpenRouter | [openrouter.rs](file:///workspace/agentforge-ui/src/infrastructure/llm_providers/openrouter.rs) | OpenRouter aggregator |
| OpenCode | [opencode.rs](file:///workspace/agentforge-ui/src/infrastructure/llm_providers/opencode.rs) | OpenCode |
| Codex | [codex.rs](file:///workspace/agentforge-ui/src/infrastructure/llm_providers/codex.rs) | OpenAI Codex |
| Custom | [custom.rs](file:///workspace/agentforge-ui/src/infrastructure/llm_providers/custom.rs) | Custom endpoint |
| iFlow | [iflow.rs](file:///workspace/agentforge-ui/src/infrastructure/llm_providers/iflow.rs) | Internal workflow |

#### Registry
[registry.rs](file:///workspace/agentforge-ui/src/infrastructure/llm_providers/registry.rs)

**AdapterRegistry** - Factory for provider adapters:
- Register provider factories
- Get or create adapter instances
- List available providers

### 5.3 MCP (Model Context Protocol) (`src/infrastructure/mcp/`)

MCP server integration and tool management.

#### Registry
[registry.rs](file:///workspace/agentforge-ui/src/infrastructure/mcp/registry.rs)

**McpToolRegistry** - Tool discovery and selection:
- `register_tool()`: Register MCP tools
- `list_selected_tools()`: Get enabled tools for run
- Tool filtering by server enabled status

**McpTool** - Tool definition:
```rust
pub struct McpTool {
    pub id: String,
    pub server_id: Option<String>,
    pub name: String,
    pub description: String,
    pub version: String,
    pub command: String,
    pub args: Vec<String>,
    pub input_schema: String,
    pub is_active: bool,
}
```

#### Other MCP Modules
| Module | Description |
|--------|-------------|
| `server.rs` | MCP server lifecycle |
| `tools.rs` | Built-in tool definitions |
| `permissions.rs` | Permission definitions |
| `action_recorder.rs` | Action audit logging |

### 5.4 Message Bus (`src/infrastructure/message_bus/`)

Inter-component messaging.

| Module | Description |
|--------|-------------|
| `routing.rs` | TeamBusRouter for team messaging |
| `queue.rs` | Message queuing |
| `persistence.rs` | Message persistence |
| `security.rs` | Message security |

### 5.5 Security (`src/infrastructure/security/`)

Security infrastructure.

| Module | Description |
|--------|-------------|
| `rbac.rs` | Role-based access control |
| `audit.rs` | Audit logging |
| `encryption.rs` | Encryption utilities |
| `keychain.rs` | Secure key storage |
| `api_keys.rs` | API key management |
| `hardening.rs` | Security hardening |

### 5.6 Monitoring (`src/infrastructure/monitoring/`)

System monitoring and analytics.

| Module | Description |
|--------|-------------|
| `metrics.rs` | Metrics collection |
| `analytics.rs` | Usage analytics |
| `agent_health.rs` | Agent health tracking |
| `activity.rs` | Activity tracking |
| `usage_analytics.rs` | Usage patterns |

### 5.7 Performance (`src/infrastructure/performance/`)

Performance optimization and benchmarking.

| Module | Description |
|--------|-------------|
| `benchmark.rs` | Performance benchmarking |
| `db_opt.rs` | Database optimizations |
| `memory.rs` | Memory management |
| `profiler.rs` | Profiling utilities |
| `crash.rs` | Crash handling |
| `startup.rs` | Startup optimization |

### 5.8 File System (`src/infrastructure/fs/`)

File system operations.

| Module | Description |
|--------|-------------|
| `obsidian_adapter.rs` | Obsidian vault integration |

---

## 6. UI Layer

### 6.1 Shell (`src/ui/shell/`)

Application shell components.

| Component | Description |
|-----------|-------------|
| `title_bar.rs` | Custom title bar |
| `activity_bar.rs` | Left sidebar navigation |
| `status_bar.rs` | Bottom status bar |
| `dock_layout.rs` | Dock panel layout persistence |
| `app_menus.rs` | Application menus |

### 6.2 Panels (`src/ui/panels/`)

Main application panels.

| Panel | File | Description |
|-------|------|-------------|
| Session | [session.rs](file:///workspace/agentforge-ui/src/ui/panels/session.rs) | Chat interface |
| Agents | [agents.rs](file:///workspace/agentforge-ui/src/ui/panels/agents.rs) | Agent management |
| Settings | [settings.rs](file:///workspace/agentforge-ui/src/ui/panels/settings.rs) | App settings |
| Monitoring | [monitoring.rs](file:///workspace/agentforge-ui/src/ui/panels/monitoring.rs) | System monitoring |
| Knowledge | [knowledge.rs](file:///workspace/agentforge-ui/src/ui/panels/knowledge.rs) | Knowledge base |
| IFlow Builder | [iflow_builder.rs](file:///workspace/agentforge-ui/src/ui/panels/iflow_builder.rs) | Workflow editor |
| MCP Marketplace | [mcp_marketplace.rs](file:///workspace/agentforge-ui/src/ui/panels/mcp_marketplace.rs) | MCP tool marketplace |
| Orchestration | [orchestration.rs](file:///workspace/agentforge-ui/src/ui/panels/orchestration.rs) | Orchestration control |
| Research Notebook | [research_notebook.rs](file:///workspace/agentforge-ui/src/ui/panels/research_notebook.rs) | Research workspace |
| Custom Provider | [custom_provider.rs](file:///workspace/agentforge-ui/src/ui/panels/custom_provider.rs) | Custom LLM provider |
| Team Workspace | [team_workspace/](file:///workspace/agentforge-ui/src/ui/panels/team_workspace/) | Team collaboration |

### 6.3 Framework (`src/ui/framework/`)

UI framework utilities.

| Module | Description |
|--------|-------------|
| `themes.rs` | Theme management |
| `i18n.rs` | Internationalization |
| `animations.rs` | Animation utilities |
| `shortcuts.rs` | Keyboard shortcuts |
| `notifications.rs` | Notification system |
| `onboarding.rs` | Onboarding flow |
| `accessibility.rs` | Accessibility support |
| `responsive.rs` | Responsive design |
| `tooltip.rs` | Tooltip system |
| `command_palette.rs` | Command palette |
| `export.rs` | Export functionality |
| `theme_transition.rs` | Theme transitions |
| `reentrancy.rs` | Reentrancy guards |

### 6.4 Components (`src/ui/components/`)

Reusable UI components.

| Component | Description |
|-----------|-------------|
| `dialogs.rs` | Dialog components |
| `markdown.rs` | Markdown rendering |

### 6.5 Text (`src/ui/text/`)

Text processing and rendering.

| Module | Description |
|--------|-------------|
| `document.rs` | Document model |
| `text_view.rs` | Text view component |
| `style.rs` | Text styling |
| `format/` | Format converters (Markdown, HTML) |

---

## 7. Dependency Relationships

### Module Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────┐
│                              UI Layer                                │
│   shell/ → panels/ → framework/ → components/ → text/               │
└─────────────────────────────────────────────────────────────────────┘
                                  ↓
┌─────────────────────────────────────────────────────────────────────┐
│                          Application Layer                           │
│                                                                       │
│   orchestration/ ← collaboration/ ← core/models/                      │
│         ↓                    ↓                                      │
│   agents/ ← teams/ ← services/                                       │
│         ↓                                                            │
│   skills/, knowledge/, research/, iflow_engine/                      │
└─────────────────────────────────────────────────────────────────────┘
                                  ↓
┌─────────────────────────────────────────────────────────────────────┐
│                            Core Layer                                │
│   models/ ← traits/ ← errors/                                        │
└─────────────────────────────────────────────────────────────────────┘
                                  ↓
┌─────────────────────────────────────────────────────────────────────┐
│                        Infrastructure Layer                           │
│                                                                       │
│   database/ ← llm_providers/ ← mcp/ ← message_bus/                    │
│       ↓                                                              │
│   monitoring/, security/, performance/, fs/                            │
└─────────────────────────────────────────────────────────────────────┘
```

### Key Dependency Flows

1. **UI → Application → Core → Infrastructure**: Standard request flow
2. **Application → Infrastructure**: Direct infrastructure access for I/O
3. **Infrastructure → Core**: Database ports and trait implementations

### Shared State

`AppState` (in lib.rs) provides:
- Database access via `db: Arc<dyn DatabasePort>`
- Async runtime via `tokio_runtime: Arc<tokio::runtime::Runtime>`
- Message bus via `team_bus: Arc<TeamBusRouter>`
- Mode management via `mode_manager: Arc<Mutex<ModeManager>>`
- Services via `Arc<ChatService>`, `Arc<TeamService>`, `Arc<KnowledgeService>`

---

## 8. Running the Project

### Prerequisites

- Rust 1.70+ (Edition 2021)
- Cargo
- For Windows/macOS: WebView2 (Windows) or WebKit (macOS)

### Build

```bash
cd agentforge-ui
cargo build --release
```

### Run

```bash
cargo run --release
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AGENTFORGE_DB_PATH` | `agentforge.db` | Database file path |
| `AGENTFORGE_OBSIDIAN_VAULT` | - | Obsidian vault path |

### Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check formatting
cargo fmt --check

# Lint
cargo clippy
```

### Testing

- **Unit Tests**: Inline in modules (`#[cfg(test)]` blocks)
- **Integration Tests**: [tests/integration_tests.rs](file:///workspace/agentforge-ui/tests/integration_tests.rs)
- **E2E Tests**: [tests/e2e_tests.rs](file:///workspace/agentforge-ui/tests/e2e_tests.rs)
- **Performance Tests**: [tests/performance_tests.rs](file:///workspace/agentforge-ui/tests/performance_tests.rs)

### Database Schema

The SQLite database is created automatically on first run. Key tables:
- `provider_configs` - LLM providers
- `teams`, `agents`, `instances` - Agent system
- `tasks`, `sessions`, `messages` - Execution state
- `knowledge_items`, `knowledge_chunks` - Knowledge base
- `collaboration_cases` - Collaboration tracking
- `workflows`, `workflow_versions` - Workflow definitions
- `orchestration_runs` - Execution history
- `audit_logs` - Security audit trail

---

## Appendix: Key Patterns

### Port/Adapter Pattern
Infrastructure implements `trait` interfaces from `core/traits/` allowing substitution.

### Service Pattern
Application services (`ChatService`, `TeamService`, `KnowledgeService`) encapsulate business logic.

### State Machine Pattern
Used in orchestration (`OrchestrationStateMachine`) and modes (`ModeManager`).

### DAG Pattern
Task dependencies resolved with topological sort and cycle detection.

### Registry Pattern
Used for providers (`AdapterRegistry`), tools (`McpToolRegistry`), and skills (`SkillRegistry`).
