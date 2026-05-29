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

$libSource = Get-Content "src/lib.rs" -Raw
Assert-Ok ($libSource -match "governance_manager") "GovernanceManager is not injected into AppState."

$learningSource = Get-Content "src/application/orchestration/learning.rs" -Raw
Assert-Ok ($learningSource -match "blocked_by_benchmark_circuit") "Benchmark circuit breaker is missing."
Assert-Ok ($learningSource -match "Automatic canary circuit breaker") "Canary pullback circuit breaker is missing."

Write-Host "AgentForge closure verification passed."
