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

$textFiles = git ls-files | Where-Object {
    $_ -match '\.(rs|md|toml|json|yml|yaml|sql|js|ts|tsx|css|html|ps1)$' -and
    $_ -notmatch '^(target|\.git)/'
}
$mixed = @()
$crlf = @()
foreach ($file in $textFiles) {
    $bytes = [System.IO.File]::ReadAllBytes((Join-Path $Root $file))
    $hasCrlf = $false
    $hasLoneLf = $false
    for ($i = 0; $i -lt $bytes.Length; $i++) {
        if ($bytes[$i] -eq 10) {
            if ($i -gt 0 -and $bytes[$i - 1] -eq 13) {
                $hasCrlf = $true
            } else {
                $hasLoneLf = $true
            }
        }
    }
    if ($hasCrlf) { $crlf += $file }
    if ($hasCrlf -and $hasLoneLf) { $mixed += $file }
}
Assert-Ok ($mixed.Count -eq 0) "Mixed line endings detected: $($mixed -join ', ')"
Assert-Ok ($crlf.Count -eq 0) "CRLF line endings detected: $($crlf -join ', ')"

$workerSource = Get-Content "src/application/orchestration/worker.rs" -Raw
Assert-Ok ($workerSource -notmatch "tokio::time::interval\(") "WorkerManager still uses fixed tokio interval."
Assert-Ok ($workerSource -notmatch "list_instances\(") "Worker execution still calls list_instances()."
Assert-Ok ($workerSource -match "ProviderAdapterCache") "Worker does not use ProviderAdapterCache."
Assert-Ok ($workerSource -match "recover_stale_in_progress_tasks") "Worker startup recovery for stale in-progress tasks is missing."

$libSource = Get-Content "src/lib.rs" -Raw
Assert-Ok ($libSource -match "governance_manager") "GovernanceManager is not injected into AppState."
Assert-Ok ($libSource -match "prewarm_embedding_model\(") "Embedding model is not prewarmed at app startup."

$learningSource = Get-Content "src/application/orchestration/learning.rs" -Raw
Assert-Ok ($learningSource -match "blocked_by_benchmark_circuit") "Benchmark circuit breaker is missing."
Assert-Ok ($learningSource -match "Automatic canary circuit breaker") "Canary pullback circuit breaker is missing."

$benchmarkRunnerSource = Get-Content "src/application/orchestration/benchmark_runner.rs" -Raw
Assert-Ok ($benchmarkRunnerSource -match "recover_stale_jobs") "Benchmark runner startup recovery is missing."

$providerFactorySource = Get-Content "src/application/services/provider_factory.rs" -Raw
Assert-Ok ($providerFactorySource -match '"CustomAdapter"\s*=>\s*"custom"') "CustomAdapter is not mapped to the custom provider adapter."
Assert-Ok ($providerFactorySource -match "_\s*=>\s*None") "Unknown provider adapters must fail closed instead of falling back."
Assert-Ok ($providerFactorySource -notmatch "Default fallback to openrouter") "Provider factory still documents an unsafe OpenRouter fallback."

$customProviderSource = Get-Content "src/infrastructure/llm_providers/custom.rs" -Raw
Assert-Ok ($customProviderSource -match "claude_messages_endpoint") "Custom provider Claude endpoint builder is missing."
Assert-Ok ($customProviderSource -match "openai_chat_completions_endpoint") "Custom provider OpenAI endpoint builder is missing."

$executorSource = Get-Content "src/application/orchestration/executor.rs" -Raw
Assert-Ok ($executorSource -match "tokenizer_profile") "LLM context snapshots do not persist tokenizer profile metadata."
Assert-Ok ($executorSource -match "estimated_request_tokens") "LLM context snapshots do not persist estimated token metadata."

$providerSources = Get-ChildItem "src/infrastructure/llm_providers" -Filter "*.rs" | ForEach-Object {
    [PSCustomObject]@{ Path = $_.FullName; Text = Get-Content $_.FullName -Raw }
}
$runtimeStreamLeaks = @()
foreach ($source in $providerSources) {
    if ($source.Text -match "let\s+rt\s*=\s*get_runtime\(\);\s*[\r\n\s]*rt\.spawn\(") {
        $runtimeStreamLeaks += $source.Path
    }
}
Assert-Ok ($runtimeStreamLeaks.Count -eq 0) "Provider streaming still spawns on private runtimes: $($runtimeStreamLeaks -join ', ')"

$diagLeaks = rg "\[DIAG|\[DIAG-EXEC|\[DIAG-STREAM|\[EMBEDDING|api_key resolved, len|api_key_ref='\{:?\}'" src
Assert-Ok ($LASTEXITCODE -ne 0) "Diagnostic or credential-shaped debug logging remains in source."

$dbSource = Get-Content "src/infrastructure/database/sqlite_adapter.rs" -Raw
Assert-Ok ($dbSource -match "migrate_legacy_credentials") "Legacy credential migration is missing."
Assert-Ok ($dbSource -match "legacy_credential_scrub") "Legacy credential scrub audit event is missing."

Write-Host "AgentForge closure verification passed."
