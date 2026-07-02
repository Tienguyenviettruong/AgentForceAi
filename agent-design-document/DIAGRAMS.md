# System Diagrams — AgentForge AI
> **Phiên bản:** 2.1 (Từ mã nguồn thực tế)
> **Cập nhật:** 2026-06-26

---

## 1. Kiến trúc tổng thể (C4 Level 2)

```mermaid
graph TB
    subgraph Desktop["🖥️ Desktop Application (GPUI)"]
        UI["UI Layer\n(Shell + Panels + Components)"]
        AppState["AppState\n(lib.rs)"]
    end
    
    subgraph Application["📦 Application Layer"]
        ChatSvc["ChatService"]
        WorkerMgr["WorkerManager"]
        AgentWorker["AgentWorker"]
        AgentExec["AgentExecutor"]
        ToolGateway["ToolExecutionGateway"]
        iFlowEngine["WorkflowEngine\n(iFlow)"]
        iFlowAuto["IFlowAutomation"]
        GovernanceMgr["GovernanceManager"]
        RolePolicy["RolePolicy"]
        CollabSvc["CollaborationService"]
    end
    
    subgraph Infrastructure["🔧 Infrastructure Layer"]
        SQLite["SQLite\n(rusqlite + WAL)"]
        TeamBus["TeamBusRouter\n(tokio channels)"]
        Claude["Claude\nProvider"]
        Gemini["Gemini\nProvider"]
        OpenRouter["OpenRouter\nProvider"]
        Custom["Custom/Ollama\nProvider"]
        MCPReg["MCP Registry"]
        AuditLog["Audit Logger"]
        Keychain["AEAD Keychain"]
    end
    
    User(["👤 Desktop User"]) --> UI
    UI --> AppState
    AppState --> ChatSvc
    AppState --> WorkerMgr
    AppState --> iFlowAuto
    
    WorkerMgr --> AgentWorker
    AgentWorker --> AgentExec
    AgentWorker --> TeamBus
    AgentExec --> ToolGateway
    AgentExec --> Claude
    AgentExec --> Gemini
    AgentExec --> OpenRouter
    AgentExec --> Custom
    AgentExec --> MCPReg
    
    ToolGateway --> SQLite
    ToolGateway --> RolePolicy
    ToolGateway --> Keychain
    ToolGateway --> AuditLog
    
    iFlowAuto --> iFlowEngine
    iFlowEngine --> TeamBus
    iFlowEngine --> SQLite
    
    GovernanceMgr --> SQLite
    GovernanceMgr --> AuditLog
    
    ChatSvc --> SQLite
    ChatSvc --> TeamBus
    
    AgentExec --> SQLite
    AgentExec --> Keychain
    
    CollabSvc --> SQLite
```

---

## 2. Clean Architecture Layers

```mermaid
graph LR
    subgraph Core["Core Layer (Domain)"]
        Models["Models\nagent.rs\norchestration.rs\nworkflow.rs\n..."]
        Traits["Traits\nDatabasePort\nBaseProviderAdapter"]
    end
    
    subgraph Application["Application Layer (Use Cases)"]
        Orch["orchestration/\nworker.rs\nexecutor.rs\ngovernance.rs\ntool_gateway.rs\nrole_policy.rs"]
        IFlow["iflow_engine/\nengine.rs\nnodes.rs\nautomation.rs"]
        Services["services/\nchat_service.rs\ncapability_router.rs\nfile_intelligence.rs"]
    end
    
    subgraph Infrastructure["Infrastructure Layer (Adapters)"]
        DB["database/\nsqlite_adapter.rs"]
        LLM["llm_providers/\nclaude.rs\ngemini.rs\nopenrouter.rs\ncustom.rs"]
        Bus["message_bus/\nrouting.rs"]
        MCP["mcp/\nregistry.rs\ntools.rs"]
        Security["security/\naudit.rs\nkeychain.rs"]
    end
    
    subgraph UI["UI Layer"]
        Shell["shell/\ntitle_bar\nactivity_bar\nstatus_bar\ndock_layout"]
        Panels["panels/\nsession\nagents\norchestration\nworkflow..."]
    end
    
    UI --> Application
    Application --> Core
    Infrastructure --> Core
    Application -.->|"impl Traits"| Infrastructure
```

---

## 3. Worker Loop State Machine

```mermaid
stateDiagram-v2
    [*] --> CheckOnline
    CheckOnline --> RegisterBus: Agent online
    CheckOnline --> [*]: Agent offline
    
    RegisterBus --> IdleWait: Registered
    
    IdleWait --> CheckOnline: Timer expired
    IdleWait --> HandleMsg: Direct message
    IdleWait --> HandleMsg: Broadcast message
    
    HandleMsg --> TryExecute: Message handled
    
    state TryExecute {
        [*] --> AcquireLock
        AcquireLock --> LoadAgent: Got lock
        AcquireLock --> [*]: Lock busy (return false)
        LoadAgent --> FindTask: Agent found
        LoadAgent --> [*]: Agent not found
        FindTask --> ClaimTask: Unblocked task found
        FindTask --> [*]: No task (return false)
        ClaimTask --> ExecuteTask: Claimed (SQL atomic)
        ClaimTask --> [*]: Race lost (return false)
        ExecuteTask --> UpdateStatus: Done
        UpdateStatus --> [*]: return true
    }
    
    TryExecute --> IdleWait: Task executed (delay=min)
    TryExecute --> IdleWait: No task (delay *=2, capped)
```

---

## 4. Tool Authorization Flow

```mermaid
flowchart TD
    A["Tool Call from LLM"] --> B{run_id exists?}
    B -->|No| DENIED1["Denied: not traceable"]
    B -->|Yes| C["Load OrchestrationRun"]
    C --> D{run.instance_id\n== request.instance_id?}
    D -->|No| DENIED2["Denied: context mismatch"]
    D -->|Yes| E["seal_invocation()\n→ AEAD encrypt payload\n→ save tool_invocations"]
    E --> F{actor has\ntool:execute:X\npermission?}
    F -->|No| DENIED3["Denied: permission missing"]
    F -->|Yes| G{Delegated run?}
    G -->|Yes| H{grant exists\nand tool in\nallowed_tools?}
    H -->|No| DENIED4["Denied: outside grant scope"]
    H -->|Yes| I{token limit\nexceeded?}
    I -->|Yes| DENIED5["Denied: token budget"]
    G -->|No| J
    I -->|No| J
    J{Coordinator\nusing forbidden\ntool?}
    J -->|Yes| DENIED6["Denied: role restriction"]
    J -->|No| K["Check path scope\n(InWorkspace/External\nParentTraversal)"]
    K --> L{ParentTraversal?}
    L -->|Yes| DENIED7["Denied: traversal"]
    L -->|No| M{Determine\nrequires_approval\nby OperatingMode}
    M --> N{Human\nInteraction?}
    N -->|Yes, non-ReadOnly| APPROVAL
    M --> O{Supervision?}
    O -->|Sensitive or\nexternal file| APPROVAL
    M --> P{Autonomous?}
    P -->|External file| APPROVAL
    P -->|Workspace file\nnon-coordinator| ALLOWED
    P -->|Sensitive\nnot enabled| DENIED8["Denied: autonomous policy"]
    N -->|ReadOnly| ALLOWED
    O -->|ReadOnly/\nControlledMutation| ALLOWED
    
    APPROVAL["ApprovalRequired\n→ create ApprovalRequest{pending}\n→ return to user"]
    ALLOWED["Allowed\n→ execute tool\n→ return result"]
    
    style DENIED1 fill:#ff6b6b
    style DENIED2 fill:#ff6b6b
    style DENIED3 fill:#ff6b6b
    style DENIED4 fill:#ff6b6b
    style DENIED5 fill:#ff6b6b
    style DENIED6 fill:#ff6b6b
    style DENIED7 fill:#ff6b6b
    style DENIED8 fill:#ff6b6b
    style ALLOWED fill:#51cf66
    style APPROVAL fill:#fcc419
```

---

## 5. iFlow Workflow Engine State Transitions

```mermaid
stateDiagram-v2
    [*] --> Running: start_workflow()
    
    Running --> Running: step_execution() → node executed
    Running --> Paused: AgentTask dispatched\n(pending_agent_tasks += node_id)
    Running --> Paused: HumanReview encountered\n(pending_review = node_id)
    Running --> Completed: End node reached\nOR all queues empty
    Running --> Failed: execute_node() returns Err
    
    Paused --> Running: resolve_agent_task()\n(pending_agent_tasks becomes empty)
    Paused --> Paused: resolve_agent_task()\n(other tasks still pending)
    Paused --> Running: resolve_review(approved=true)
    Paused --> Running: resolve_review(approved=false)\n→ push rejected_next
    Paused --> Failed: reject_waiting_run()
    
    Completed --> [*]
    Failed --> [*]
    
    note right of Running
        Delay nodes → pending_delays map
        Expired delays unblocked on next step
    end note
```

---

## 6. iFlow Node Execution Graph

```mermaid
graph LR
    Start([▶ Start]) --> |push next_nodes|NextNodes
    CronTrigger([⏰ CronTrigger\ninterval_ms]) --> |push next_nodes|NextNodes
    
    AgentTask([🤖 AgentTask\nagent_id + instruction\ninput_vars + output_var]) --> |dispatch TeamMessage\niflow_dispatch:exec:node\nstate=Paused| Pending
    Pending -.->|"iflow_result:exec:node\nresolved by automation"| AgentTask2([resolve_agent_task\n→ state=Running\ndata[output_var]=result])
    
    Decision{🔀 Decision\ncondition_var} --> |bool=true|TrueNext
    Decision --> |bool=false|FalseNext
    
    HumanReview([👤 HumanReview\nprompt\nstate=Paused]) --> |approved|ApprovedNext
    HumanReview --> |rejected|RejectedNext
    
    Transform([🔄 Transform\nIdentity/ToString]) --> |output_var set|NextNodes
    Merge([⊕ Merge\njoin barrier]) --> |push next_nodes|NextNodes
    
    Delay([⏳ Delay\nduration_ms]) --> |pending_delays map\nnow_ms + duration|NextNodes
    
    End([⏹ End]) --> |status=Completed|Done([Done])
```

---

## 7. Cross-Team Collaboration Flow

```mermaid
sequenceDiagram
    participant U as 👤 User
    participant T1 as Team Instance 1\n(Sender)
    participant Bus as TeamBusRouter
    participant T2 as Team Instance 2\n(Receiver)
    participant DB as SQLite

    U->>T1: Assign task to agent
    T1->>T1: AgentExecutor runs
    T1->>Bus: handoff_to_team(\nhanded_type="review_request",\ntarget=T2, reply_to=T1\n)
    Bus->>T2: TeamMessage{metadata=[CrossTeamHandoff JSON]}
    T2->>DB: persist_cross_team_case_event()
    T2->>DB: ensure_governed_case() → CollaborationCase
    T2->>DB: start_cross_team_run() → OrchestrationRun
    T2->>DB: acknowledge_and_readback()
    T2->>DB: grant_readback_only() → DelegatedGrant
    T2->>Bus: emit_status_event(ACK_RECEIVED) → T1
    T2->>T2: AgentExecutor runs (CRITIC role)
    T2->>Bus: handoff_to_team(review_response) → T1
    T2->>Bus: emit_status_event(COMPLETED) → T1
    Bus->>T1: Review response received
```

---

## 8. Database Entity Relationship Diagram

```mermaid
erDiagram
    agents {
        string id PK
        string name
        string provider
        text system_prompt
        text config
        string status
        datetime created_at
        datetime updated_at
    }
    
    teams {
        string id PK
        string name
        text description
        datetime created_at
    }
    
    team_instances {
        string id PK
        string team_id FK
        string name
        text description
        datetime created_at
    }
    
    agent_team_assignments {
        string agent_id FK
        string team_instance_id FK
        datetime created_at
    }
    
    sessions {
        string id PK
        string agent_id FK
        string team_instance_id FK
        string title
        datetime created_at
        datetime updated_at
    }
    
    conversation_turns {
        string id PK
        string session_id FK
        string role
        text content
        text metadata
        datetime created_at
    }
    
    team_messages {
        string id PK
        string team_instance_id FK
        string sender_member_id
        string recipient_member_id
        string message_type
        text content
        text metadata
        string delivery_status
        datetime created_at
    }
    
    tasks {
        string id PK
        string run_id FK
        string team_instance_id FK
        string assignee_id FK
        string title
        string status
        text payload
        text dependencies
        datetime created_at
        datetime updated_at
    }
    
    orchestration_runs {
        string id PK
        string session_id FK
        string instance_id FK
        string initiated_by FK
        text goal
        string mode
        string status
        string workflow_id FK
        datetime created_at
        datetime updated_at
    }
    
    run_events {
        string id PK
        string run_id FK
        string event_type
        string actor_type
        string actor_id
        string task_id
        text payload
        datetime created_at
    }
    
    approval_requests {
        string id PK
        string run_id FK
        string operation
        string requested_by
        string status
        string resolved_by
        text decision_reason
        datetime created_at
        datetime resolved_at
    }
    
    tool_invocations {
        string id PK
        string run_id FK
        string tool_name
        text sealed_payload_json
        string payload_hash
        string mode
        string status
        string approval_request_id FK
        text result
        datetime created_at
        datetime updated_at
    }
    
    workflows {
        string id PK
        string instance_id FK
        string name
        text definition
        string activation_status
        string run_id
        datetime created_at
    }
    
    workflow_executions {
        string id PK
        string workflow_version_id FK
        string run_id FK
        string status
        text state_json
        datetime updated_at
    }
    
    collaboration_cases {
        string id PK
        string correlation_id
        string owner_instance_id FK
        string target_instance_id FK
        text objective
        string status
        datetime created_at
    }
    
    delegated_grants {
        string id PK
        string case_id FK
        string run_id FK
        string grantor_agent_id
        string grantee_agent_id
        text allowed_tools_json
        text allowed_mcp_json
        int token_limit
        datetime expires_at
    }
    
    audit_logs {
        string id PK
        datetime timestamp
        string action
        string user_id
        string resource
        text details
    }
    
    security_actors {
        string id PK
        string name
        string type
        datetime created_at
    }
    
    actor_permissions {
        string actor_id FK
        string permission
        datetime granted_at
    }

    teams ||--o{ team_instances : "has"
    team_instances ||--o{ agent_team_assignments : "has"
    agents ||--o{ agent_team_assignments : "assigned to"
    team_instances ||--o{ sessions : "hosts"
    sessions ||--o{ conversation_turns : "contains"
    team_instances ||--o{ team_messages : "receives"
    orchestration_runs ||--o{ run_events : "generates"
    orchestration_runs ||--o{ approval_requests : "creates"
    orchestration_runs ||--o{ tool_invocations : "records"
    approval_requests ||--o{ tool_invocations : "resolves"
    collaboration_cases ||--o{ delegated_grants : "grants"
    security_actors ||--o{ actor_permissions : "has"
```

---

## 9. Agent Executor Context Flow

```mermaid
flowchart TB
    A["execute_task(history)"] --> B["Check cancel_flag"]
    B --> C["smart_prune_history()\n> 20 messages → summarize evicted"]
    C --> D["Build tools_json:\n- save_to_knowledge\n- handoff_to_team\n- create_subtasks\n- read/write/edit_file\n- run_cli\n- search_knowledge\n- MCP tools\n+ collaboration tools"]
    D --> E["filter_tools_json_for_role()\nCoordinator: remove artifact tools"]
    E --> F["inject_context()\nKnowledge + history + cases"]
    F --> G["persist_request_context_snapshot()\nLlmContextSnapshotRecord + sources"]
    G --> H["LLM stream call"]
    H --> I{Tool call\nin response?}
    I -->|No| DONE["Return final text"]
    I -->|Yes| J["ToolExecutionGateway::\nauthorize_runtime()"]
    J --> K{Decision?}
    K -->|Allowed| L["execute_tool()"]
    K -->|ApprovalRequired| M["Return 'Approval required...'"]
    K -->|Denied| N["Append denial to history"]
    L --> O["redact_untrusted_context()\nSanitize tool output"]
    O --> P["Append '[UNTRUSTED TOOL OUTPUT]\ntool result' to history"]
    P --> Q{max_iterations\nreached?}
    Q -->|No| H
    Q -->|Yes| DONE
    N --> H
```

---

## 10. Operating Mode Decision Matrix

```mermaid
quadrantChart
    title Tool Execution Policy by Mode
    x-axis Safe → Sensitive
    y-axis Manual → Automatic
    quadrant-1 Auto in Supervision + Autonomous
    quadrant-2 Auto in All Modes
    quadrant-3 Always Approval
    quadrant-4 Auto in Autonomous only
    ReadOnly (workspace): [0.1, 0.9]
    save_to_knowledge: [0.3, 0.7]
    handoff_to_team: [0.3, 0.65]
    create_subtasks: [0.3, 0.7]
    write_file (workspace): [0.6, 0.55]
    edit_file (workspace): [0.6, 0.5]
    run_cli (workspace): [0.75, 0.35]
    MCP tools: [0.85, 0.1]
    write_file (external): [0.8, 0.1]
    run_cli (external): [0.9, 0.05]
```

---

## 11. Sequence: Task Execution với Approval

```mermaid
sequenceDiagram
    participant W as AgentWorker
    participant E as AgentExecutor
    participant G as ToolGateway
    participant K as Keychain
    participant DB as SQLite
    participant LLM as LLM Provider
    participant U as 👤 User

    W->>DB: claim_task_for_instance() [atomic SQL]
    W->>LLM: stream start (typing message)
    W->>E: execute_task(history)
    E->>LLM: stream_completion(messages, tools)
    LLM-->>E: streaming response with tool_call{write_file, path=...}
    E->>G: authorize_runtime(request)
    G->>K: seal_invocation(payload) → AEAD encrypt
    G->>DB: upsert_tool_invocations{status="sealed"}
    G->>DB: check_actor_permission("local-user", "tool:execute:write_file")
    G->>DB: create_approval_request{status="pending"}
    G-->>E: PolicyDecision::ApprovalRequired{request_id}
    E-->>W: "Approval required before executing write_file..."
    W->>DB: mark_task_waiting_approval()
    W->>U: display "Approval required" in UI

    U->>DB: approve_request(request_id)
    U->>W: trigger resume (IFlowAutomation or UI action)
    W->>E: execute_task (resume path)
    E->>E: resume_approved_invocations()
    E->>DB: get_next_approved_tool_invocation_for_run()
    E->>K: open_sensitive_payload(sealed, associated_data)
    E->>E: execute_tool("write_file", payload)
    E->>LLM: append "[UNTRUSTED TOOL OUTPUT]" + result
    LLM-->>E: final response
    E-->>W: Ok(final_text)
    W->>DB: mark_task_completed()
```
