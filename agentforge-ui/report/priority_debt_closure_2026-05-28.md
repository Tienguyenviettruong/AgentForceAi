# Priority Debt Closure - 2026-05-28

## Scope

This note closes the latest priority batch requested after the earlier audit reports:

- `team_workspace/chat.rs` legacy provider/executor duplication.
- MCP stdio sandbox, local install/config catalog, and remote config safety.
- Canary/benchmark telemetry and automatic canary pass/fail decisions.
- Token/context budget moving beyond a plain character cap.

## 1. Team Workspace Chat Legacy Path

Implemented:

- `src/ui/panels/team_workspace/chat.rs` now delegates `provider_kind()` and `build_provider_adapter()` to `src/application/services/provider_factory.rs`.
- The legacy debate/direct provider path no longer contains manual provider matches or direct calls to `OpenRouterAdapter::new`, `ClaudeAdapter::new`, `GeminiAdapter::new`, `CodexAdapter::new`, or `OpenCodeAdapter::new`.
- The legacy path uses the shared `AgentExecutor` with a factory-built adapter, reducing duplicated executor logic.
- `provider_factory::create_adapter()` now maps `codex` to `CodexAdapter` and `opencode` to `OpenCodeAdapter`, not the OpenRouter fallback.

Residual:

- The retained legacy block is still inside `#[cfg(any())]`, so it is not part of the active runtime path. The active chat flow already exits through persisted iFlow/run dispatch. A future cleanup can delete the retained migration block once no historical fallback is needed.

## 2. MCP Stdio Sandbox And Catalog

Implemented:

- Stdio MCP remains fail-closed unless one of these is true:
  - `AGENTFORGE_MCP_SANDBOX_WRAPPER` is configured.
  - The old explicit override `AGENTFORGE_ALLOW_UNSANDBOXED_MCP_STDIO=true` is set.
- When a wrapper is configured, MCP stdio launches through `wrapper -- command args`.
- Added command allowlist control through `AGENTFORGE_MCP_STDIO_ALLOWLIST`.
- Default stdio allowlist is limited to common launchers: `npx`, `npx.cmd`, `uvx`, `node`, `node.exe`, `python`, `python.exe`.
- MCP Marketplace now has a local catalog with add actions for:
  - GitHub
  - Sequential Thinking
  - Memory
- Catalog entries create persisted MCP server records with secret refs and are disabled as `blocked_until_isolated` when the stdio sandbox wrapper is missing.
- Remote MCP config still requires HTTPS, rejects loopback endpoints, and rejects credentials in args/URLs.

Residual:

- The app now integrates with an OS sandbox wrapper, but it does not itself implement a native Windows/macOS/Linux sandbox. True local process isolation depends on the configured wrapper.

## 3. Canary Telemetry And Circuit Breaker

Implemented:

- Added persisted `canary_observations`.
- Added database methods:
  - `insert_canary_observation`
  - `list_canary_observations_for_deployment`
- Added `LearningService::record_canary_observation()`.
- A canary observation accepts metric JSON containing:
  - `quality_score`
  - `error_rate`
  - `latency_ms`
  - `safety_violations`
  - `unauthorized_side_effects`
- `record_evaluation()` now acts as an automatic runtime collector:
  - When a terminal run is evaluated, the service checks running canary deployments.
  - If a deployment scope matches the evaluated run, the evaluation is converted into a canary observation.
- Supported automatic canary scope constraints:
  - `run_id`
  - `run_ids`
  - `instance_id`
  - `mode`
  - `workflow_id`
  - `workflow_ids`
- Observation fail triggers the circuit breaker:
  - deployment becomes `failed`;
  - candidate activation/baseline is rolled back;
  - `rollback_record` is persisted;
  - candidate becomes `rolled_back`;
  - source lesson returns to `validated`.
- Observation pass counts toward `learning_canary_required_passes`; when the threshold is reached with no failures, the deployment becomes `passed` and the candidate becomes `canary_passed`.
- Latency threshold is read from `learning_canary_max_latency_ms`, defaulting to `30000`.

Residual:

- This closes automatic canary telemetry for evaluated real runs. It does not yet add a separate benchmark runner that generates benchmark traffic without evaluation records.

## 4. Token And Context Budget

Implemented:

- `AgentExecutor` no longer truncates retrieved/governed context by a plain character budget.
- Added `ContextTokenizerProfile` selected by provider/model family.
- Claude and local/code model families such as Qwen, DeepSeek, Llama, Mistral, Codestral, and Gemma use a more conservative profile.
- Gemini/OpenAI/OpenRouter fallback profiles are handled separately.
- CJK characters, punctuation, and newlines are counted separately to reduce under-counting for multilingual text and code.
- Retrieval context uses `governance_max_retrieved_context_tokens`.
- Governed context uses `governance_max_governed_context_tokens`.
- Old `*_chars` settings remain only as migration fallback.

Residual:

- This is now model-family token accounting rather than a character cap. It is not exact vendor tokenization. Exact accounting still requires a future dependency or bundled tokenizer data for each target model family.

## Verification

- `cargo fmt`: pass.
- `cargo check --lib`: pass.
- Known warnings remain unchanged: two old dead-code warnings under `src/ui/text`.

Not run:

- `cargo test --test integration_tests -- --test-threads=1`, per prior user instruction.
