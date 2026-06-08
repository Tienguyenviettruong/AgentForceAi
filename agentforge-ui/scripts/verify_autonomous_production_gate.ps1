param(
    [switch]$SkipCargo
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Assert-Ok($Condition, $Message) {
    if (-not $Condition) {
        throw $Message
    }
}

if (-not $SkipCargo) {
    cargo fmt --check
    cargo check --lib
}

powershell -ExecutionPolicy Bypass -File scripts\verify_system_closure.ps1 -SkipCargo

$lib = Get-Content "src/lib.rs" -Raw
$worker = Get-Content "src/application/orchestration/worker.rs" -Raw
$executor = Get-Content "src/application/orchestration/executor.rs" -Raw
$toolGateway = Get-Content "src/application/orchestration/tool_gateway.rs" -Raw
$learning = Get-Content "src/application/orchestration/learning.rs" -Raw
$benchmarkRunner = Get-Content "src/application/orchestration/benchmark_runner.rs" -Raw
$providerFactory = Get-Content "src/application/services/provider_factory.rs" -Raw
$customProvider = Get-Content "src/infrastructure/llm_providers/custom.rs" -Raw
$db = Get-Content "src/infrastructure/database/sqlite_adapter.rs" -Raw
$orchestrationUi = Get-Content "src/ui/panels/orchestration.rs" -Raw

# 1. Full E2E release matrix foundation: chat/iFlow/executor/approval/artifact/learning are wired.
Assert-Ok ($lib -match "BenchmarkRunner::start_scheduler") "Benchmark scheduler is not started by the app."
Assert-Ok ($executor -match "AgentExecutor") "AgentExecutor is missing."
Assert-Ok ($executor -match "persist_request_context_snapshot") "LLM request context snapshots are not persisted."
Assert-Ok ($executor -match "ToolExecutionGateway") "Runtime tool gateway is not wired into executor."
Assert-Ok ($orchestrationUi -match "LLM Context Inspector") "Context inspector UI is missing."

# 2. Restart/resume hardening.
Assert-Ok ($worker -match "recover_stale_in_progress_tasks") "Worker stale task recovery is missing."
Assert-Ok ($benchmarkRunner -match "recover_stale_jobs") "Benchmark stale job recovery is missing."
Assert-Ok ($executor -match "resume_approved_invocations") "Sealed invocation resume is missing."

# 3. Provider smoke/static tests.
Assert-Ok ($providerFactory -match '"CustomAdapter"\s*=>\s*"custom"') "CustomAdapter does not map to the custom adapter."
Assert-Ok ($providerFactory -match "_\s*=>\s*None") "Unknown providers do not fail closed."
Assert-Ok ($customProvider -match "claude_endpoint_builder_handles_bare_v1_and_full_paths") "Claude endpoint builder smoke test is missing."
Assert-Ok ($customProvider -match "openai_endpoint_builder_handles_bare_v1_and_full_paths") "OpenAI endpoint builder smoke test is missing."

# 4. Secret leakage regression.
Assert-Ok ($db -match "migrate_legacy_credentials") "Legacy credential migration is missing."
Assert-Ok ($db -match "legacy_credential_scrub") "Legacy credential scrub audit is missing."
$leakScan = rg "api_key resolved, len|api_key_ref='\{:?\}'|\[DIAG|\[DIAG-STREAM|\[EMBEDDING]|sk-[A-Za-z0-9_-]{20,}|Bearer\s+<|Bearer\s+sk-" src
Assert-Ok ($LASTEXITCODE -ne 0) "Secret or diagnostic leakage pattern remains in source."

# 5. Queue/worker resilience.
Assert-Ok ($worker -match "task_exec_lock") "Worker execution lock is missing."
Assert-Ok ($worker -match "ProviderAdapterCache") "Provider adapter cache is missing."
Assert-Ok ($db -match "WHERE id = \?4 AND status = 'pending' AND instance_id = \?5") "Atomic task claim guard is missing."
Assert-Ok ($db -match "task_recovered_after_restart") "Task recovery event is missing."

# 6. Model-specific tokenizer metadata.
Assert-Ok ($executor -match "ContextTokenizerProfile") "Context tokenizer profile is missing."
Assert-Ok ($executor -match "tokenizer_profile") "Tokenizer profile is not persisted in context snapshot."
Assert-Ok ($executor -match "estimated_request_tokens") "Estimated request tokens are not persisted."

# 7. Context replay/audit.
Assert-Ok ($db -match "llm_context_snapshots") "LLM context snapshot table is missing."
Assert-Ok ($db -match "llm_context_sources") "LLM context source table is missing."
Assert-Ok ($orchestrationUi -match "list_recent_llm_context_snapshots") "Context inspector does not read snapshots."
Assert-Ok ($executor -match "context_hash_only_full_prompt_not_persisted") "Context replay metadata does not declare redaction policy."

# 8. Autonomous mode threat checks.
Assert-Ok ($toolGateway -match "Autonomous") "Tool gateway does not branch autonomous mode."
Assert-Ok ($executor -match "redact_untrusted_context") "Model input redaction is missing."
Assert-Ok ($toolGateway -match "no workspace is configured") "Workspace-bound file operation guard is missing."
Assert-Ok ($executor -match "MCP capability.*was not selected") "Selected MCP invocation guard is missing."

# 9. Benchmark/canary reliability.
Assert-Ok ($benchmarkRunner -match "recover_stale_jobs") "Benchmark runner restart recovery is missing."
Assert-Ok ($learning -match "blocked_by_benchmark_circuit") "Benchmark circuit breaker is missing."
Assert-Ok ($learning -match "learning_canary_required_passes") "Canary pass threshold is missing."
Assert-Ok ($learning -match "Automatic canary circuit breaker") "Canary rollback circuit breaker is missing."

# 10. Observability production.
Assert-Ok ($executor -match "insert_token_usage") "Token usage telemetry is missing."
Assert-Ok ($executor -match "record_run_event") "Run event telemetry is missing."
Assert-Ok ($orchestrationUi -match "Benchmark Runner") "Benchmark telemetry UI is missing."
Assert-Ok ($orchestrationUi -match "Canary Telemetry") "Canary telemetry UI is missing."
Assert-Ok ($db -match "audit_log") "Audit log persistence is missing."

Write-Host "AgentForge autonomous production gate passed."
