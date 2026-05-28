# AgentForge UI - Delivery Plan de hoan thien IDE

**Ngay cap nhat:** 2026-05-27  
**Spec chi phoi:** `report/system_completion_implementation_spec_2026-05-25.md`  
**Audit doi chieu:** `report/system_completion_reassessment_2026-05-26.md`

## 1. Release objective

Mot ban release chi duoc xem la IDE hoan thien khi mot muc tieu tu Chat co the di qua cung mot run/iFlow/executor/policy/context/provenance spine, va nguoi dung co the truy vet dung kha nang va du lieu da duoc trao cho LLM.

## 2. Trang thai hien tai

| Workstream | Trang thai | Ket qua hien co |
|---|---|---|
| Phase 0-3 data/UI slices | Substantial implemented | Run/event, iFlow version state, Knowledge provenance, normalized task dependencies va artifact provenance co persistence/projection. |
| Phase 4 governed runtime | Substantial, release verification blocked | Gateway, actor, mode matrix, AES-GCM exact invocation continuation, delegated grants, workspace bound va iFlow/cross-team executor coverage da co; external-process sandbox va release tests con mo. |
| Phase 5 context va MCP configuration | Substantial implemented ngay 2026-05-27 | Selected injection, per-request snapshots/Context Inspector, MCP stdio/remote discovery-invocation, OS-keychain reference va context untrusted boundary da co. |
| Production release | Not ready | Con P0/P1 va test blockers ben duoi. |

## 3. Work packages con lai

| Thu tu | Muc do | Work package | Dependency | Gate hoan tat |
|---:|---|---|---|---|
| 1 | Delivered; verify | Chat/iFlow execution authority cho normal chat | iFlow/cross-team AgentTask da dung executor | Persisted flow dispatch va paused-node resume da implemented; can e2e gate. |
| 2 | Delivered; verify | Durable tool invocation va exact approval continuation | Unified executor/gateway | AES-GCM sealed invocation/result va exact resume da implemented; can restart gate. |
| 3 | Delivered; verify | Delegated RBAC va budget scope | Mode matrix da trien khai | Readback grant va fail-closed gateway da implemented; can cross-team test gate. |
| 4 | P1 | MCP catalog install va process/network sandbox | MCP protocol runtime da trien khai | Installed-server workflow hoan chinh; process/env/network/resource isolation va audit co gate. |
| 5 | Delivered; verify | Secret migration hoan chinh | MCP OS keychain da trien khai | Migration scrub known rows da implemented; can upgraded DB/leakage gate. |
| 6 | P1 | Context safety maturity | Inspector, redaction va char budget da trien khai | Tokenizer budget/ranking, injection-defense evaluation va policy evidence dat gate. |
| 7 | Partial | Learned automation promotion va monitoring completion | Artifact/event spine | Manual promotion/rollback va persisted Monitoring da implemented; automated canary telemetry con mo. |
| 8 | P2 | Prototype module retirement | Runtime contract hien tai | Xoa/gan experimental/trien khai that cho semantic legacy, message security va plugin shell. |
| 9 | P0 | Test/build release gate repair | Tat ca contract tren | Null-byte tests/GPUI config da sua mot phan; can full unit/integration/e2e matrix va formatter pass. |

## 4. Phase 5 implementation delivered trong pass nay

| Hanh vi | Code/datastore |
|---|---|
| Luu Installed MCP Servers va transport/config | `mcp_servers`, `src/ui/panels/mcp_marketplace.rs` |
| Gan MCP tool voi server va chi expose tool cua server enabled | `mcp_tools.server_id`, `McpToolRegistry::list_selected_tools()` |
| Luu lua chon MCP/Skill cua nguoi dung | `capability_selections`, MCP/Skills panels |
| Inject dung selected MCP schema va selected skill instruction | `src/application/orchestration/executor.rs`, `src/application/skills/*.rs` |
| Inject run/mode/goal/status cung Knowledge retrieval | `AgentExecutor::execute_task()` |
| Luu dau vet input context cua LLM | `llm_context_snapshots`, `llm_context_sources` |
| Khong cho MCP UI chay process truc tiep | MCP surface chi config/selection, execution phai den tu runtime co run |
| Chan raw MCP credentials moi | `env` va credential headers yeu cau `secret://` reference |
| Run selection override default selection | `McpToolRegistry::list_selected_tools()` va `AgentExecutor::selected_capability_ids()` ap dung override theo scope |
| Build binary khong con bat buoc link ONNX Runtime tai compile time | `fastembed` dung `ort-load-dynamic`; embedding bao loi co kiem soat neu runtime thieu |
| Dua iFlow AgentTask va cross-team agent turn qua run-bound executor | `worker.rs`, `AgentExecutor`, `run_events`; cross-team reply phai di qua governed `handoff_to_team` |
| Chan side-effect node iFlow cu o runtime | `WorkflowEngine::execute_node()` tu choi `SystemCommand` va `HttpRequest` |
| Snapshot theo tung provider request va inspection UI | `llm_context_snapshots`, `llm_context_sources`, tab `Context` |
| Bounded/redacted untrusted context | Knowledge/tool output injection duoc gan trust boundary, redact credential-like line va gioi han chars |
| MCP protocol va secret reference runtime | JSON-RPC stdio/remote discovery/invocation, Refresh Tools, multi-server import va OS keychain |
| Dependency scheduling chuan hoa | `task_dependencies` backfill/upsert + SQL unblocked/atomic claim |
| Artifact provenance va projection | `artifacts` + tab `Artifacts`, lien ket run/session/agent/invocation/hash |
| Analytics/iFlow log truthful | Loai metric xu huong/thoi gian co dinh va log `Waiting` gia khi chua co execution |

## 5. Verification va release discipline

| Check | Trang thai ngay 2026-05-27 |
|---|---|
| `cargo check --lib` | Pass sau cac thay doi Phase 4-5/observability; chi con warning `dead_code` co san trong `src/ui/text`. |
| `cargo build --bin agentforge-ui` | Pass sau khi sua cau hinh embedding dynamic-load. |
| Targeted `rustfmt` cho cac module da sua (ngoai file chat co whitespace legacy) | Pass. |
| `git diff --check` cho file source/test da sua | Pass, ngoai canh bao LF/CRLF cua repository. |
| Test Phase 5 cu | Da pass truoc extension 2026-05-27 cho selection override, disable server va skill instruction. |
| Test bo sung artifact/dependency/context/mode | Da viet; khong chay trong pass nay theo yeu cau nguoi dung khong chay target `integration_tests`. |
| `cargo fmt --all -- --check` | Fail do diff/trailing whitespace con lai trong cac file khac (`worker.rs`, `knowledge.rs`, `team_workspace/chat.rs`, ...). |
| Full tests | Chua dat release gate; `cargo test --lib` bi dung truoc ket qua cuoi va chua co full regression pass. |

## 6. Quyet dinh trien khai

- Khong danh dau he thong la complete cho den khi work packages P0 va P1 deu pass acceptance test.
- Khong mo direct MCP execution tu UI; moi invocation can gan run va gateway.
- Khong chap nhan completion claim cho MCP security cho den khi sandbox/install lifecycle va migration/leakage test hoan tat; new-config validation/keychain da bao phu `env`, headers, args va endpoint URL.
- Khong che day gap Chat/iFlow bang UI label; flow phai tro thanh execution authority hoac san pham phai tuyen bo ro no chi la plan.

## 7. Continuation va Phase 7/8 extension - 2026-05-27

| Item | Trang thai moi |
|---|---|
| Provider/output raw secret path | Runtime va UI moi da chuyen sang `secret://`/`env:` reference-only; legacy DB values van can migration/removal. |
| Raw RAG assembly ngoai executor | Da xoa tai worker va `/run` chat path; retrieval cua hai luong nay dung executor context contract. |
| Phase 7 - Human-like Collaboration Contract | Source foundation implemented: schema/model/service, handoff/readback gate, delegated grants, competency routing, context, consensus/escalation operator resolution va exact paused-node resume. MCP isolation/e2e con mo. |
| Phase 8 - Governed Learning and Evolution | Source/manual lifecycle implemented: evidence admission, human-governed lesson/candidate/benchmark/canary/promotion/rollback, skill/workflow activation va UI. Automated scheduler/circuit breaker con mo. |

Thu tu delivery mo rong:

| Order | Work package |
|---:|---|
| 10 | Dong sealed invocation/exact approval, iFlow authority, direct LLM boundary, legacy secret migration va MCP/delegated isolation. |
| 11 | Phase 7 schema va lifecycle cho case/handoff/readback/decision/deliverable/review/consensus/escalation. **Foundation delivered 2026-05-27.** |
| 12 | Phase 7 delegated grants va competency-based routing voi context/UI projection. **Foundation delivered 2026-05-27.** |
| 13 | Phase 8 evaluation/feedback/lesson va candidate lifecycle. **Foundation delivered 2026-05-27; active version materialization con mo.** |
| 14 | Phase 8 benchmark/canary/promotion/rollback. **Persistence va gated runtime delivered; automated release operations con mo.** |

## 8. Continuation implementation reconciliation - 2026-05-27

Muc nay supersede bang work package snapshot tai muc 2-3 cho nhung contract da duoc trien khai sau do.

| Work package cu | Trang thai source moi | Bang chung |
|---|---|---|
| Chat/iFlow authority cho chat thuong | Implemented cho message khong phai `/run` | `src/ui/panels/team_workspace/chat.rs`, `src/application/iflow_engine/automation.rs`, `engine.rs`, `src/application/orchestration/worker.rs`. |
| Durable invocation/exact approval | Implemented source-level | `tool_invocations`, AES-GCM seal/open trong `security/keychain.rs`, gateway persist, executor resume, Governance redispatch workflow node. |
| Delegated RBAC/budget | Implemented source-level | `delegated_grants`, readback-only grant, expanded grant sau accept, gateway fail-closed va competency routing. |
| Secret migration | Implemented migration; release verification pending | Migration `2026052704` scrub known raw legacy values va ghi audit count; can upgraded-DB/leakage tests. |
| Learned automation promotion | Implemented governed manual lifecycle | Evidence/lesson/candidate/benchmark/canary/promote/rollback services va operator UI; skill/workflow activation/rollback co materialization. |
| Knowledge artifact visibility | Implemented projection/context metadata | Artifacts xuat hien trong Knowledge va metadata cua artifact hien tai duoc inject bounded theo run. |

### 8.1 Gate van con mo

| Priority | Gate chua dong | Huong xu ly tiep |
|---:|---|---|
| P0 | Release verification tren DB upgrade, approval restart, secret leakage va end-to-end run | Tao fixture DB cu va test matrix release; integration target khong duoc chay trong pass nay theo chi dan nguoi dung. |
| P1 | MCP install/catalog va sandbox that | Stdio hien fail-closed tru mac dinh; can isolated process/network/resource runtime thay cho unsafe override. |
| P1 | Automatic benchmark/canary/circuit breaker | UI va state machine co that; can runner va telemetry policy de tu promote/pullback an toan. |
| P1 | Context/adversarial maturity | Them tokenizer-aware budget, prompt-injection/secret exfiltration evaluation va policy evidence. |
| P2 | Cleanup duong legacy/experimental | Xoa source chat direct compile-disabled va retire hoac thay the message-security/plugin/semantic prototype khong thuoc authoritative spine. |
