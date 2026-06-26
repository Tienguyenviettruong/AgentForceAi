pub trait DatabasePort: Send + Sync {
    fn seed_provider_templates(&self) -> anyhow::Result<()>;
    fn list_provider_templates(&self)
        -> anyhow::Result<Vec<crate::core::models::ProviderTemplate>>;
    fn insert_provider(&self, p: &crate::core::models::Provider) -> anyhow::Result<()>;
    fn list_providers(&self) -> anyhow::Result<Vec<crate::core::models::Provider>>;
    fn get_provider_by_name(
        &self,
        provider_name: &str,
    ) -> anyhow::Result<Option<crate::core::models::Provider>>;
    fn insert_team(&self, team: &crate::core::models::Team) -> anyhow::Result<()>;
    fn create_instance(
        &self,
        id: &str,
        name: &str,
        team_id: &str,
        config: Option<&str>,
        state: Option<&str>,
    ) -> anyhow::Result<()>;
    fn update_instance_name(&self, instance_id: &str, name: &str) -> anyhow::Result<()>;
    fn update_instance_state(&self, instance_id: &str, state: &str) -> anyhow::Result<()>;
    fn list_instances(&self) -> anyhow::Result<Vec<crate::core::models::Instance>>;
    fn list_teams(&self) -> anyhow::Result<Vec<crate::core::models::Team>>;
    fn insert_agent(&self, agent: &crate::core::models::Agent) -> anyhow::Result<()>;
    fn delete_agent(&self, agent_id: &str) -> anyhow::Result<()>;
    fn list_agents(&self) -> anyhow::Result<Vec<crate::core::models::Agent>>;
    fn assign_agent_to_team(&self, team_id: &str, agent_id: &str) -> anyhow::Result<()>;
    fn remove_agent_from_team(&self, team_id: &str, agent_id: &str) -> anyhow::Result<()>;
    fn get_team_agents(&self, team_id: &str) -> anyhow::Result<Vec<String>>;
    fn get_instance_agents(&self, instance_id: &str) -> anyhow::Result<Vec<String>>;
    fn get_instance_agent_name_mapping(
        &self,
        instance_id: &str,
    ) -> anyhow::Result<std::collections::HashMap<String, String>>;
    fn upsert_task(
        &self,
        id: &str,
        team_id: &str,
        instance_id: Option<&str>,
        run_id: Option<&str>,
        assignee_id: Option<&str>,
        status: &str,
        priority: &str,
        payload: Option<&str>,
    ) -> anyhow::Result<()>;
    fn get_total_tokens_per_agent(&self) -> anyhow::Result<Vec<(String, usize)>>;
    fn get_total_tokens_per_instance(&self) -> anyhow::Result<Vec<(String, usize)>>;
    fn get_agent_instance_count(&self) -> anyhow::Result<Vec<(String, usize)>>;
    fn get_total_daily_tokens(&self) -> anyhow::Result<usize>;
    fn get_total_tasks_count(&self) -> anyhow::Result<usize>;
    fn get_total_tasks_completed(&self) -> anyhow::Result<usize>;
    fn get_active_agents_count(&self) -> anyhow::Result<usize>;
    fn insert_token_usage(
        &self,
        instance_id: Option<&str>,
        run_id: Option<&str>,
        agent_id: &str,
        input_tokens: usize,
        output_tokens: usize,
        total_tokens: usize,
    ) -> anyhow::Result<()>;
    fn get_total_tokens_for_run(&self, run_id: &str) -> anyhow::Result<usize>;
    fn assign_task_to_agent(&self, task_id: &str, agent_id: &str) -> anyhow::Result<()>;
    fn list_tasks_for_instance(
        &self,
        instance_id: &str,
    ) -> anyhow::Result<Vec<crate::application::tasks::shared_task_list::Task>>;
    fn list_recent_tasks(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::application::tasks::shared_task_list::Task>>;
    fn list_pending_tasks_for_instance(
        &self,
        instance_id: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::application::tasks::shared_task_list::Task>>;
    fn is_task_unblocked(&self, task_id: &str) -> anyhow::Result<bool>;
    fn claim_task_for_instance(
        &self,
        task_id: &str,
        agent_id: &str,
        instance_id: &str,
    ) -> anyhow::Result<bool>;
    fn recover_stale_in_progress_tasks(&self, max_age_seconds: u64) -> anyhow::Result<usize>;
    fn mark_task_completed(&self, task_id: &str) -> anyhow::Result<()>;
    fn mark_task_failed(&self, task_id: &str) -> anyhow::Result<()>;
    fn mark_task_waiting_approval(&self, task_id: &str) -> anyhow::Result<()>;
    fn resolve_waiting_tasks_for_run(&self, run_id: &str, status: &str) -> anyhow::Result<()>;
    fn seed_sdg_team(&self) -> anyhow::Result<()>;
    fn get_agent(&self, agent_id: &str) -> anyhow::Result<Option<crate::core::models::Agent>>;
    fn insert_team_message(
        &self,
        msg: &crate::infrastructure::message_bus::routing::TeamMessage,
    ) -> anyhow::Result<()>;
    fn get_team_messages_for_instance(
        &self,
        team_instance_id: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::infrastructure::message_bus::routing::TeamMessage>>;
    fn get_team_messages_for_instance_by_type(
        &self,
        team_instance_id: &str,
        message_type: crate::infrastructure::message_bus::routing::MessageType,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::infrastructure::message_bus::routing::TeamMessage>>;
    fn update_team_message_delivery_status(
        &self,
        message_id: &str,
        status: &str,
    ) -> anyhow::Result<()>;
    fn update_team_message_content(&self, message_id: &str, content: &str) -> anyhow::Result<()>;
    fn append_conversation_turn(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        metadata: Option<&str>,
    ) -> anyhow::Result<()>;
    fn ensure_session(
        &self,
        session_id: &str,
        agent_id: &str,
        team_instance_id: Option<&str>,
    ) -> anyhow::Result<()>;
    fn create_session_for_instance(
        &self,
        instance_id: &str,
        agent_id: &str,
    ) -> anyhow::Result<String>;
    fn list_sessions_for_instance(
        &self,
        instance_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::SessionRecord>>;
    fn get_latest_session_for_instance(
        &self,
        instance_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::SessionRecord>>;
    fn touch_session(&self, session_id: &str) -> anyhow::Result<()>;
    fn get_conversation_turns(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::ChatMessage>>;
    fn save_message(
        &self,
        team_id: &str,
        instance_id: Option<&str>,
        role: &str,
        content: &str,
    ) -> anyhow::Result<()>;
    fn get_messages(
        &self,
        team_id: &str,
        instance_id: Option<&str>,
    ) -> anyhow::Result<Vec<crate::core::models::ChatMessage>>;
    fn upsert_knowledge_item(
        &self,
        item: &crate::core::models::KnowledgeItem,
    ) -> anyhow::Result<()>;
    fn get_all_knowledge_items(&self) -> anyhow::Result<Vec<crate::core::models::KnowledgeItem>>;
    fn set_setting(&self, key: &str, value: &str) -> anyhow::Result<()>;
    fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>>;
    fn get_recent_workspaces(&self) -> anyhow::Result<Vec<String>>;
    fn search_knowledge(
        &self,
        query: &str,
    ) -> anyhow::Result<Vec<crate::core::models::KnowledgeItem>>;
    fn search_knowledge_fts(
        &self,
        query: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::KnowledgeItem>>;
    fn upsert_knowledge_chunks(
        &self,
        document_id: &str,
        chunks: Vec<(usize, String, Vec<f32>)>,
    ) -> anyhow::Result<()>;
    fn search_similar_chunks(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> anyhow::Result<Vec<(String, String, f32)>>;

    fn upsert_cross_team_case(
        &self,
        correlation_id: &str,
        owner_instance_id: &str,
        target_instance_id: &str,
        latest_event_type: &str,
        summary: &str,
    ) -> anyhow::Result<()>;
    fn insert_cross_team_case_event(
        &self,
        event: &crate::core::models::CrossTeamCaseEventRecord,
    ) -> anyhow::Result<()>;
    fn list_cross_team_cases(
        &self,
        instance_id: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::CrossTeamCaseRecord>>;
    fn list_cross_team_case_events(
        &self,
        correlation_id: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::CrossTeamCaseEventRecord>>;

    // Phase 7: human-like collaboration contract
    fn upsert_collaboration_case(
        &self,
        case: &crate::core::models::CollaborationCaseRecord,
    ) -> anyhow::Result<()>;
    fn get_collaboration_case(
        &self,
        case_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::CollaborationCaseRecord>>;
    fn get_collaboration_case_by_correlation_id(
        &self,
        correlation_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::CollaborationCaseRecord>>;
    fn get_collaboration_case_for_run(
        &self,
        run_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::CollaborationCaseRecord>>;
    fn list_recent_collaboration_cases(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::CollaborationCaseRecord>>;
    fn update_collaboration_case_state(&self, case_id: &str, state: &str) -> anyhow::Result<()>;
    fn insert_handoff_package(
        &self,
        handoff: &crate::core::models::HandoffPackageRecord,
    ) -> anyhow::Result<()>;
    fn get_latest_handoff_for_case(
        &self,
        case_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::HandoffPackageRecord>>;
    fn insert_case_readback(
        &self,
        readback: &crate::core::models::CaseReadbackRecord,
    ) -> anyhow::Result<()>;
    fn list_case_readbacks(
        &self,
        case_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::CaseReadbackRecord>>;
    fn resolve_case_readback(
        &self,
        readback_id: &str,
        status: &str,
        accepted_by: Option<&str>,
    ) -> anyhow::Result<()>;
    fn insert_case_decision(
        &self,
        decision: &crate::core::models::CaseDecisionRecord,
    ) -> anyhow::Result<()>;
    fn list_case_decisions(
        &self,
        case_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::CaseDecisionRecord>>;
    fn insert_case_deliverable(
        &self,
        deliverable: &crate::core::models::CaseDeliverableRecord,
    ) -> anyhow::Result<()>;
    fn list_case_deliverables(
        &self,
        case_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::CaseDeliverableRecord>>;
    fn update_case_deliverable_status(
        &self,
        deliverable_id: &str,
        status: &str,
    ) -> anyhow::Result<()>;
    fn insert_case_review(
        &self,
        review: &crate::core::models::CaseReviewRecord,
    ) -> anyhow::Result<()>;
    fn list_case_reviews(
        &self,
        case_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::CaseReviewRecord>>;
    fn insert_case_consensus(
        &self,
        consensus: &crate::core::models::CaseConsensusRecord,
    ) -> anyhow::Result<()>;
    fn list_case_consensus_records(
        &self,
        case_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::CaseConsensusRecord>>;
    fn insert_case_consensus_vote(
        &self,
        vote: &crate::core::models::CaseConsensusVoteRecord,
    ) -> anyhow::Result<()>;
    fn list_case_consensus_votes(
        &self,
        consensus_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::CaseConsensusVoteRecord>>;
    fn resolve_case_consensus(
        &self,
        consensus_id: &str,
        status: &str,
        resolution: &str,
    ) -> anyhow::Result<()>;
    fn insert_case_escalation(
        &self,
        escalation: &crate::core::models::CaseEscalationRecord,
    ) -> anyhow::Result<()>;
    fn list_pending_case_escalations(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::CaseEscalationRecord>>;
    fn resolve_case_escalation(
        &self,
        escalation_id: &str,
        resolved_by: &str,
        resolution: &str,
    ) -> anyhow::Result<()>;
    fn insert_delegated_grant(
        &self,
        grant: &crate::core::models::DelegatedGrantRecord,
    ) -> anyhow::Result<()>;
    fn get_active_delegated_grant(
        &self,
        agent_id: &str,
        run_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::DelegatedGrantRecord>>;
    fn list_active_delegated_grants_for_case(
        &self,
        case_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::DelegatedGrantRecord>>;
    fn update_delegated_grant_status(&self, grant_id: &str, status: &str) -> anyhow::Result<()>;
    fn upsert_agent_competency(
        &self,
        competency: &crate::core::models::AgentCompetencyRecord,
    ) -> anyhow::Result<()>;
    fn list_agent_competencies(
        &self,
        competency_key: Option<&str>,
    ) -> anyhow::Result<Vec<crate::core::models::AgentCompetencyRecord>>;
    fn insert_routing_decision(
        &self,
        routing: &crate::core::models::RoutingDecisionRecord,
    ) -> anyhow::Result<()>;
    fn list_routing_decisions_for_case(
        &self,
        case_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::RoutingDecisionRecord>>;

    fn upsert_workflow(&self, wf: &crate::core::models::WorkflowRecord) -> anyhow::Result<()>;
    fn list_workflows(&self) -> anyhow::Result<Vec<crate::core::models::WorkflowRecord>>;
    fn get_workflow(
        &self,
        workflow_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::WorkflowRecord>>;
    fn delete_workflow(&self, workflow_id: &str) -> anyhow::Result<()>;
    fn next_workflow_version_number(&self, workflow_id: &str) -> anyhow::Result<i64>;
    fn save_workflow_version(
        &self,
        version: &crate::core::models::WorkflowVersionRecord,
    ) -> anyhow::Result<()>;
    fn get_latest_workflow_version_for_run(
        &self,
        run_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::WorkflowVersionRecord>>;
    fn get_latest_workflow_version_for_workflow(
        &self,
        workflow_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::WorkflowVersionRecord>>;
    fn get_workflow_version(
        &self,
        version_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::WorkflowVersionRecord>>;
    fn save_workflow_execution(
        &self,
        execution: &crate::core::models::WorkflowExecutionRecord,
    ) -> anyhow::Result<()>;
    fn get_latest_workflow_execution_for_version(
        &self,
        workflow_version_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::WorkflowExecutionRecord>>;
    fn get_workflow_execution(
        &self,
        execution_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::WorkflowExecutionRecord>>;

    fn save_workflow_state(
        &self,
        state: &crate::application::iflow_engine::engine::WorkflowState,
    ) -> anyhow::Result<()>;
    fn load_workflow_state(
        &self,
        execution_id: &str,
    ) -> anyhow::Result<Option<crate::application::iflow_engine::engine::WorkflowState>>;

    // Orchestration execution spine
    fn create_orchestration_run(
        &self,
        run: &crate::core::models::OrchestrationRunRecord,
    ) -> anyhow::Result<()>;
    fn get_orchestration_run(
        &self,
        run_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::OrchestrationRunRecord>>;
    fn update_orchestration_run_status(
        &self,
        run_id: &str,
        status: &str,
        workflow_id: Option<&str>,
    ) -> anyhow::Result<()>;
    fn list_recent_orchestration_runs(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::OrchestrationRunRecord>>;
    fn insert_run_event(&self, event: &crate::core::models::RunEventRecord) -> anyhow::Result<()>;
    fn list_recent_run_events(
        &self,
        run_id: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::RunEventRecord>>;
    fn create_approval_request(
        &self,
        request: &crate::core::models::ApprovalRequestRecord,
    ) -> anyhow::Result<()>;
    fn list_pending_approval_requests(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::ApprovalRequestRecord>>;
    fn get_approval_request_for_operation(
        &self,
        run_id: &str,
        operation: &str,
    ) -> anyhow::Result<Option<crate::core::models::ApprovalRequestRecord>>;
    fn resolve_approval_request(
        &self,
        request_id: &str,
        status: &str,
        resolved_by: Option<&str>,
        reason: Option<&str>,
    ) -> anyhow::Result<()>;
    fn upsert_tool_invocation(
        &self,
        invocation: &crate::core::models::ToolInvocationRecord,
    ) -> anyhow::Result<()>;
    fn update_tool_invocation_status(
        &self,
        invocation_id: &str,
        status: &str,
        approval_request_id: Option<&str>,
        result: Option<&str>,
    ) -> anyhow::Result<()>;
    fn get_next_approved_tool_invocation_for_run(
        &self,
        run_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::ToolInvocationRecord>>;
    fn insert_mode_transition(
        &self,
        transition: &crate::core::models::ModeTransitionRecord,
    ) -> anyhow::Result<()>;
    fn list_recent_mode_transitions(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::ModeTransitionRecord>>;

    fn insert_audit_log(
        &self,
        event: &crate::infrastructure::security::audit::AuditEvent,
    ) -> anyhow::Result<()>;
    fn list_recent_audit_logs(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::infrastructure::security::audit::AuditEvent>>;
    fn create_role(&self, role: &crate::application::teams::role::Role) -> anyhow::Result<()>;
    fn update_role_permissions(&self, role_id: &str, permissions: &str) -> anyhow::Result<()>;
    fn check_role_permission(
        &self,
        role_id: &str,
        required_permission: &str,
    ) -> anyhow::Result<bool>;
    fn ensure_local_security_owner(&self, actor_id: &str) -> anyhow::Result<()>;
    fn check_actor_permission(
        &self,
        actor_id: &str,
        required_permission: &str,
    ) -> anyhow::Result<bool>;

    // MCP Tools
    fn upsert_mcp_tool(
        &self,
        tool: &crate::infrastructure::mcp::registry::McpTool,
    ) -> anyhow::Result<()>;
    fn get_mcp_tool(
        &self,
        id: &str,
    ) -> anyhow::Result<Option<crate::infrastructure::mcp::registry::McpTool>>;
    fn list_mcp_tools(&self) -> anyhow::Result<Vec<crate::infrastructure::mcp::registry::McpTool>>;
    fn delete_mcp_tool(&self, id: &str) -> anyhow::Result<()>;
    fn upsert_mcp_server(
        &self,
        server: &crate::infrastructure::mcp::registry::McpServerRecord,
    ) -> anyhow::Result<()>;
    fn list_mcp_servers(
        &self,
    ) -> anyhow::Result<Vec<crate::infrastructure::mcp::registry::McpServerRecord>>;
    fn upsert_capability_selection(
        &self,
        selection: &crate::core::models::CapabilitySelectionRecord,
    ) -> anyhow::Result<()>;
    fn list_capability_selections(
        &self,
        scope_kind: &str,
        scope_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::CapabilitySelectionRecord>>;
    fn insert_llm_context_snapshot(
        &self,
        snapshot: &crate::core::models::LlmContextSnapshotRecord,
    ) -> anyhow::Result<()>;
    fn insert_llm_context_source(
        &self,
        source: &crate::core::models::LlmContextSourceRecord,
    ) -> anyhow::Result<()>;
    fn list_recent_llm_context_snapshots(
        &self,
        run_id: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::LlmContextSnapshotRecord>>;
    fn list_llm_context_sources(
        &self,
        snapshot_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::LlmContextSourceRecord>>;
    fn insert_artifact(&self, artifact: &crate::core::models::ArtifactRecord)
        -> anyhow::Result<()>;
    fn list_artifacts_for_run(
        &self,
        run_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::ArtifactRecord>>;

    // Phase 8: governed learning and evolution
    fn upsert_evaluation_rubric(
        &self,
        rubric: &crate::core::models::EvaluationRubricRecord,
    ) -> anyhow::Result<()>;
    fn insert_run_evaluation(
        &self,
        evaluation: &crate::core::models::RunEvaluationRecord,
    ) -> anyhow::Result<()>;
    fn get_run_evaluation(
        &self,
        evaluation_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::RunEvaluationRecord>>;
    fn list_recent_run_evaluations(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::RunEvaluationRecord>>;
    fn insert_feedback_record(
        &self,
        feedback: &crate::core::models::FeedbackRecord,
    ) -> anyhow::Result<()>;
    fn get_feedback_record(
        &self,
        feedback_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::FeedbackRecord>>;
    fn update_feedback_validation_status(
        &self,
        feedback_id: &str,
        status: &str,
    ) -> anyhow::Result<()>;
    fn list_recent_feedback_records(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::FeedbackRecord>>;
    fn insert_lesson(&self, lesson: &crate::core::models::LessonRecord) -> anyhow::Result<()>;
    fn get_lesson(
        &self,
        lesson_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::LessonRecord>>;
    fn update_lesson_status(&self, lesson_id: &str, status: &str) -> anyhow::Result<()>;
    fn list_recent_lessons(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::LessonRecord>>;
    fn list_active_lessons(
        &self,
        scope_kind: Option<&str>,
        scope_id: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::LessonRecord>>;
    fn insert_learning_candidate(
        &self,
        candidate: &crate::core::models::LearningCandidateRecord,
    ) -> anyhow::Result<()>;
    fn get_learning_candidate(
        &self,
        candidate_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::LearningCandidateRecord>>;
    fn update_learning_candidate_status(
        &self,
        candidate_id: &str,
        status: &str,
    ) -> anyhow::Result<()>;
    fn list_recent_learning_candidates(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::LearningCandidateRecord>>;
    fn insert_skill_version(
        &self,
        version: &crate::core::models::SkillVersionRecord,
    ) -> anyhow::Result<()>;
    fn next_skill_version_number(&self, skill_id: &str) -> anyhow::Result<i64>;
    fn get_skill_version_for_candidate(
        &self,
        candidate_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::SkillVersionRecord>>;
    fn list_active_skill_versions(
        &self,
    ) -> anyhow::Result<Vec<crate::core::models::SkillVersionRecord>>;
    fn update_skill_version_activation(&self, version_id: &str, status: &str)
        -> anyhow::Result<()>;
    fn insert_benchmark_run(
        &self,
        benchmark: &crate::core::models::BenchmarkRunRecord,
    ) -> anyhow::Result<()>;
    fn list_benchmark_runs_for_candidate(
        &self,
        candidate_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::BenchmarkRunRecord>>;
    fn upsert_benchmark_suite(
        &self,
        suite: &crate::core::models::BenchmarkSuiteRecord,
    ) -> anyhow::Result<()>;
    fn get_benchmark_suite(
        &self,
        suite_id: &str,
    ) -> anyhow::Result<Option<crate::core::models::BenchmarkSuiteRecord>>;
    fn list_active_benchmark_suites(
        &self,
    ) -> anyhow::Result<Vec<crate::core::models::BenchmarkSuiteRecord>>;
    fn insert_benchmark_case(
        &self,
        case: &crate::core::models::BenchmarkCaseRecord,
    ) -> anyhow::Result<()>;
    fn list_benchmark_cases_for_suite(
        &self,
        suite_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::BenchmarkCaseRecord>>;
    fn insert_benchmark_result(
        &self,
        result: &crate::core::models::BenchmarkResultRecord,
    ) -> anyhow::Result<()>;
    fn list_benchmark_results_for_run(
        &self,
        benchmark_run_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::BenchmarkResultRecord>>;
    fn insert_benchmark_runner_job(
        &self,
        job: &crate::core::models::BenchmarkRunnerJobRecord,
    ) -> anyhow::Result<()>;
    fn update_benchmark_runner_job(
        &self,
        job_id: &str,
        status: &str,
        benchmark_run_id: Option<&str>,
        error: Option<&str>,
        completed_at: Option<&str>,
    ) -> anyhow::Result<()>;
    fn list_benchmark_runner_jobs_for_candidate(
        &self,
        candidate_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::BenchmarkRunnerJobRecord>>;
    fn list_benchmark_runner_jobs_by_status(
        &self,
        status: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::BenchmarkRunnerJobRecord>>;
    fn insert_canary_deployment(
        &self,
        deployment: &crate::core::models::CanaryDeploymentRecord,
    ) -> anyhow::Result<()>;
    fn list_canary_deployments_for_candidate(
        &self,
        candidate_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::CanaryDeploymentRecord>>;
    fn update_canary_deployment_status(
        &self,
        deployment_id: &str,
        status: &str,
    ) -> anyhow::Result<()>;
    fn insert_canary_observation(
        &self,
        observation: &crate::core::models::CanaryObservationRecord,
    ) -> anyhow::Result<()>;
    fn list_canary_observations_for_deployment(
        &self,
        deployment_id: &str,
    ) -> anyhow::Result<Vec<crate::core::models::CanaryObservationRecord>>;
    fn insert_promotion_decision(
        &self,
        decision: &crate::core::models::PromotionDecisionRecord,
    ) -> anyhow::Result<()>;
    fn insert_rollback_record(
        &self,
        rollback: &crate::core::models::RollbackRecord,
    ) -> anyhow::Result<()>;
    fn list_recent_rollback_records(
        &self,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::RollbackRecord>>;

    // Knowledge Entries (Long-term memory)
    fn upsert_knowledge_entry(
        &self,
        entry: &crate::core::models::knowledge::KnowledgeEntry,
    ) -> anyhow::Result<()>;
    fn get_knowledge_entry(
        &self,
        id: &str,
    ) -> anyhow::Result<Option<crate::core::models::knowledge::KnowledgeEntry>>;
    fn get_all_knowledge_entries(
        &self,
    ) -> anyhow::Result<Vec<crate::core::models::knowledge::KnowledgeEntry>>;
    fn search_knowledge_entries_fts(
        &self,
        query: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::core::models::knowledge::KnowledgeEntry>>;
}
