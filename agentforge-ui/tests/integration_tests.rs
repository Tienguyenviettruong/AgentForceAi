use agentforge_ui::application::orchestration::tool_gateway::{
    PolicyDecision, ToolExecutionGateway, ToolRequest, LOCAL_DESKTOP_ACTOR_ID,
};
use agentforge_ui::core::models::{
    Agent, CapabilitySelectionRecord, OrchestrationRunRecord, Provider, Team,
};
use agentforge_ui::core::traits::database::DatabasePort;
use agentforge_ui::infrastructure::database::sqlite_adapter::Database;
use agentforge_ui::mcp::registry::McpToolRegistry;
use agentforge_ui::mcp::tools::register_team_tools;
use std::sync::Arc;

fn isolated_database() -> Arc<dyn DatabasePort> {
    let path = std::env::temp_dir().join(format!(
        "agentforge-server-toggle-{}.db",
        uuid::Uuid::new_v4()
    ));
    std::env::set_var("AGENTFORGE_DB_PATH", &path);
    let db = Database::new().expect("test database should initialize");
    std::env::remove_var("AGENTFORGE_DB_PATH");
    Arc::new(db)
}

#[test]
fn provider_configuration_supports_update_and_delete() {
    let db = isolated_database();
    let provider_id = uuid::Uuid::new_v4().to_string();
    let mut provider = Provider {
        id: provider_id.clone(),
        provider_name: "Initial Provider".to_string(),
        model: "initial-model".to_string(),
        adapter_type: "CustomAdapter".to_string(),
        command: Some("https://initial.example/v1".to_string()),
        api_key_ref: Some("env:INITIAL_KEY".to_string()),
        status: "available".to_string(),
        capabilities: None,
    };
    db.insert_provider(&provider).expect("provider insert");

    provider.provider_name = "Updated Provider".to_string();
    provider.model = "updated-model".to_string();
    provider.command = Some("https://updated.example/v1".to_string());
    provider.api_key_ref = Some("env:UPDATED_KEY".to_string());
    db.update_provider(&provider).expect("provider update");

    let updated = db
        .list_providers()
        .expect("provider list")
        .into_iter()
        .find(|candidate| candidate.id == provider_id)
        .expect("updated provider should exist");
    assert_eq!(updated.provider_name, "Updated Provider");
    assert_eq!(updated.model, "updated-model");
    assert_eq!(
        updated.command.as_deref(),
        Some("https://updated.example/v1")
    );
    assert_eq!(updated.api_key_ref.as_deref(), Some("env:UPDATED_KEY"));

    db.delete_provider(&provider_id).expect("provider delete");
    assert!(!db
        .list_providers()
        .expect("provider list after delete")
        .into_iter()
        .any(|candidate| candidate.id == provider_id));
}

#[test]
fn disabled_mcp_server_cannot_expose_selected_tools() {
    let db = isolated_database();
    let registry = McpToolRegistry::new(db.clone());
    register_team_tools(&registry).expect("built-in server should register");

    let now = chrono::Utc::now().to_rfc3339();
    db.upsert_capability_selection(&CapabilitySelectionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        scope_kind: "default".to_string(),
        scope_id: "".to_string(),
        capability_kind: "mcp_tool".to_string(),
        capability_id: "mcp-team-broadcast".to_string(),
        enabled: true,
        selected_by: Some("test".to_string()),
        created_at: now.clone(),
        updated_at: now.clone(),
    })
    .expect("selection should persist");
    assert_eq!(registry.list_selected_tools(None).len(), 1);

    let mut server = registry
        .list_servers()
        .into_iter()
        .find(|server| server.id == "builtin-team-tools")
        .expect("built-in MCP server should exist");
    server.is_enabled = false;
    server.updated_at = now;
    registry
        .register_server(&server)
        .expect("server status should update");

    assert!(registry.list_selected_tools(None).is_empty());
}

fn seed_governed_run(db: &Arc<dyn DatabasePort>, mode: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    db.insert_team(&Team {
        id: "team".to_string(),
        name: "Team".to_string(),
        description: None,
        objectives: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    })
    .expect("team fixture");
    db.create_instance("instance", "Instance", "team", None, None)
        .expect("instance fixture");
    db.insert_agent(&Agent {
        id: "agent".to_string(),
        name: "Agent".to_string(),
        provider: "test".to_string(),
        system_prompt: None,
        config: None,
        status: "online".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
    })
    .expect("agent fixture");
    db.ensure_session("session", "agent", Some("instance"))
        .expect("session fixture");
    db.ensure_local_security_owner(LOCAL_DESKTOP_ACTOR_ID)
        .expect("actor fixture");
    db.create_orchestration_run(&OrchestrationRunRecord {
        id: "run".to_string(),
        session_id: "session".to_string(),
        instance_id: "instance".to_string(),
        initiated_by: Some(LOCAL_DESKTOP_ACTOR_ID.to_string()),
        goal: "mode policy".to_string(),
        mode: mode.to_string(),
        status: "running".to_string(),
        workflow_id: None,
        created_at: now.clone(),
        updated_at: now,
    })
    .expect("run fixture");
}

fn request<'a>(tool_name: &'a str, payload: &'a serde_json::Value) -> ToolRequest<'a> {
    ToolRequest {
        tool_name,
        payload,
        instance_id: "instance",
        session_id: Some("session"),
        run_id: Some("run"),
        invocation_id: None,
        delegated_agent_id: "agent",
        is_mcp: false,
    }
}

#[test]
fn mode_policy_distinguishes_human_supervision_and_autonomous_execution() {
    let payload = serde_json::json!({});

    let human_db = isolated_database();
    seed_governed_run(&human_db, "human_interaction");
    assert!(matches!(
        ToolExecutionGateway::new(human_db)
            .authorize_runtime(&request("save_to_knowledge", &payload)),
        PolicyDecision::ApprovalRequired { .. }
    ));

    let supervision_db = isolated_database();
    seed_governed_run(&supervision_db, "supervision");
    assert_eq!(
        ToolExecutionGateway::new(supervision_db)
            .authorize_runtime(&request("save_to_knowledge", &payload)),
        PolicyDecision::Allowed
    );

    let autonomous_db = isolated_database();
    seed_governed_run(&autonomous_db, "autonomous");
    let gateway = ToolExecutionGateway::new(autonomous_db.clone());
    assert!(matches!(
        gateway.authorize_runtime(&request("run_cli", &payload)),
        PolicyDecision::Denied(_)
    ));
    autonomous_db
        .set_setting("governance_autonomous_sensitive_allowed", "true")
        .expect("policy setting");
    assert_eq!(
        gateway.authorize_runtime(&request("run_cli", &payload)),
        PolicyDecision::Allowed
    );
}

#[test]
fn external_file_access_requires_approval_before_path_resolution() {
    let db = isolated_database();
    seed_governed_run(&db, "supervision");
    let workspace =
        std::env::temp_dir().join(format!("agentforge-workspace-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&workspace).expect("workspace fixture");
    db.set_setting("workspace_instance", &workspace.to_string_lossy())
        .expect("workspace setting");

    let external_file =
        std::env::temp_dir().join(format!("agentforge-external-{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&external_file, "external").expect("external fixture");
    let external_path = external_file.to_string_lossy().to_string();
    let payload = serde_json::json!({ "path": external_path });
    let gateway = ToolExecutionGateway::new(db.clone());
    let tool_request = request("read_file", &payload);

    assert!(matches!(
        gateway.authorize_runtime(&tool_request),
        PolicyDecision::ApprovalRequired { .. }
    ));
    assert!(gateway
        .resolve_authorized_tool_path(&tool_request, payload["path"].as_str().unwrap())
        .unwrap_err()
        .contains("requires approval"));

    let approvals = db
        .list_pending_approval_requests(10)
        .expect("approval list");
    assert_eq!(approvals.len(), 1);
    assert!(approvals[0].operation.contains("path="));
    db.resolve_approval_request(
        &approvals[0].id,
        "approved",
        Some(LOCAL_DESKTOP_ACTOR_ID),
        Some("test approval"),
    )
    .expect("approval resolution");

    let resolved = gateway
        .resolve_authorized_tool_path(&tool_request, payload["path"].as_str().unwrap())
        .expect("approved external path should resolve");
    assert_eq!(resolved, external_file);
}

#[test]
fn parent_directory_traversal_is_denied_instead_of_approved() {
    let db = isolated_database();
    seed_governed_run(&db, "human_interaction");
    let workspace =
        std::env::temp_dir().join(format!("agentforge-workspace-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&workspace).expect("workspace fixture");
    db.set_setting("workspace_instance", &workspace.to_string_lossy())
        .expect("workspace setting");

    let payload = serde_json::json!({ "path": "../secret.txt" });
    assert!(matches!(
        ToolExecutionGateway::new(db).authorize_runtime(&request("read_file", &payload)),
        PolicyDecision::Denied(reason) if reason.contains("parent-directory traversal")
    ));
}

#[test]
fn normalized_task_dependency_blocks_claim_until_prerequisite_is_complete() {
    let db = isolated_database();
    seed_governed_run(&db, "supervision");
    let prerequisite = serde_json::json!({
        "id": "prepare",
        "name": "Prepare",
        "description": "Prepare input",
        "dependencies": [],
        "priority": 1,
        "deadline": null,
        "assignee_id": "agent"
    })
    .to_string();
    let dependent = serde_json::json!({
        "id": "publish",
        "name": "Publish",
        "description": "Publish result",
        "dependencies": ["prepare"],
        "priority": 1,
        "deadline": null,
        "assignee_id": "agent"
    })
    .to_string();

    db.upsert_task(
        "instance:publish",
        "team",
        Some("instance"),
        Some("run"),
        Some("agent"),
        "pending",
        "medium",
        Some(&dependent),
    )
    .expect("dependent task");
    assert!(!db
        .is_task_unblocked("instance:publish")
        .expect("dependency lookup"));
    assert!(!db
        .claim_task_for_instance("instance:publish", "agent", "instance")
        .expect("claim result"));

    db.upsert_task(
        "instance:prepare",
        "team",
        Some("instance"),
        Some("run"),
        Some("agent"),
        "pending",
        "medium",
        Some(&prerequisite),
    )
    .expect("prerequisite task");
    assert!(!db
        .is_task_unblocked("instance:publish")
        .expect("dependency remains blocked"));
    db.mark_task_completed("instance:prepare")
        .expect("prerequisite completion");
    assert!(db
        .is_task_unblocked("instance:publish")
        .expect("dependency resolved"));
    assert!(db
        .claim_task_for_instance("instance:publish", "agent", "instance")
        .expect("dependent claim"));
}
