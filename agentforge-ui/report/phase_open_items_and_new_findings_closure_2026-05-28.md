# Audit bổ sung và trạng thái xử lý ngày 2026-05-28

## Phạm vi kiểm tra

Đã đối chiếu lại mã nguồn với các báo cáo/kế hoạch trước đó và các nhận định bổ sung:

- WorkerManager poll interval cứng 2 giây.
- Provider adapter bị instantiate lại trong mỗi task execution.
- `list_instances()` bị gọi trong từng task để lấy `team_id`.
- WorkerManager có nguy cơ spawn trùng worker cho cùng `(instance, agent)`.
- `DatabasePort` là God Interface.
- Mixed line endings CRLF/LF.
- `GovernanceManager` đã định nghĩa nhưng chưa được inject vào `AppState`.

## Kết luận nhanh

| Nhận định | Kết quả kiểm chứng | Trạng thái sau chỉnh sửa |
|---|---|---|
| Worker poll cứng 2 giây | Đúng ở `AgentWorker::start`; worker discovery trong `lib.rs` cũng sleep 2 giây. | Đã sửa: worker dùng adaptive backoff, discovery poll cấu hình được. |
| Provider adapter instantiate mỗi task | Đúng trong `worker.rs`, có nhiều match tự tạo adapter. | Đã sửa cho orchestration worker bằng `ProviderAdapterCache`. |
| `list_instances()` trong mỗi task để lấy `team_id` | Đúng trong worker task/cross-team execution. | Đã sửa: `team_id` được truyền từ discovery loop vào worker. |
| WorkerManager không dedup | Không còn đúng với mã hiện tại: key đã là `instance_id_agent_id`, guarded bằng mutex và có `contains_key`. | Giữ nguyên logic dedup; bổ sung cache shared theo worker manager. |
| `DatabasePort` quá lớn | Đúng: 185 method, khoảng 663 dòng, đang ép mock/test phải kéo toàn bộ interface. | Đã thêm các facet trait bước đầu; cần migrate dần call site để đóng nợ hoàn toàn. |
| Mixed CRLF/LF | Đúng: scan ban đầu có 9 mixed files; thêm nhiều file CRLF đồng nhất. | Đã chuẩn hóa LF toàn bộ text tracked, thêm `.editorconfig` và `.gitattributes`. |
| GovernanceManager chưa inject | Đúng: file tồn tại nhưng không export qua `mod.rs`, không có trong `AppState`. | Đã export module và inject `governance_manager` vào `AppState`. |

## Chi tiết thay đổi đã thực hiện

### 1. WorkerManager / AgentWorker

File chính: `src/application/orchestration/worker.rs`

Đã thay `tokio::time::interval(Duration::from_secs(2))` bằng adaptive backoff:

- Poll ngay khi worker start.
- Nếu có task được claim/executed: quay về `worker_poll_min_ms`.
- Nếu idle: delay tăng dần tới `worker_poll_max_ms`.
- Default hiện tại: min 500ms, max 15s.
- Có thể cấu hình qua settings:
  - `worker_poll_min_ms`
  - `worker_poll_max_ms`

Worker discovery trong `src/lib.rs` không còn hard-code 2 giây:

- Setting: `worker_manager_poll_ms`
- Default: 5000ms
- Clamp: 1000ms đến 300000ms

Ghi chú: đây là cải thiện CPU khi idle và bớt hard-code. Để reactive hoàn toàn, cần thêm event/notify khi task được insert hoặc khi workflow dispatch tạo task.

### 2. Provider adapter cache

File chính: `src/application/services/provider_factory.rs`

Đã thêm `ProviderAdapterCache`:

- Cache key gồm `provider.id`, `provider_name`, `model`, `adapter_type`, `command`, `api_key_ref`.
- WorkerManager giữ một cache shared.
- AgentWorker dùng cache thay vì instantiate adapter ở mỗi task/cross-team/iFlow execution.

Tác động:

- Giảm setup cost mỗi task.
- Tránh lặp logic match provider trong worker.
- Provider adapter vẫn có thể invalidate theo provider id qua `invalidate()`.

Phần còn lại: UI chat legacy/direct execution vẫn có nhánh tự dựng adapter trong `team_workspace/chat.rs`; đây là phần P2 legacy cleanup đã nêu trong các báo cáo trước.

### 3. Bỏ N+1 `list_instances()` trong worker execution

Trước đó worker gọi `list_instances()` trong task execution và cross-team execution chỉ để lấy `team_id`.

Đã sửa:

- `WorkerManager::start_workers_for_instance(instance_id, team_id)` nhận sẵn `team_id`.
- `AgentWorker` lưu `team_id`.
- Dynamic system prompt dùng `self.team_id`.

Kết quả kiểm tra sau sửa:

- `src/application/orchestration/worker.rs` không còn `list_instances()`.
- `src/lib.rs` vẫn gọi `list_instances()` ở discovery loop, đây là nơi hợp lý để scan instance định kỳ.

### 4. Dedup worker

Kiểm chứng source hiện tại:

- Worker key là `format!("{}_{}", instance_id, agent_id)`.
- Map workers được bảo vệ bởi `tokio::sync::Mutex`.
- Có `workers.retain(|_, handle| !handle.is_finished())`.
- Có `if !workers.contains_key(&worker_key)` trước khi spawn.

Kết luận: nhận định "không dedup" không còn đúng với mã hiện tại. Không cần sửa sâu ở điểm này.

### 5. DatabasePort God Interface

Kiểm chứng:

- `src/core/traits/database.rs` có 185 method.
- Interface gom provider/team/instance/task/chat/knowledge/collaboration/workflow/orchestration/audit/MCP/learning vào một trait duy nhất.

Đã thêm bước nền:

- `src/core/traits/database_facets.rs`
- Export qua `src/core/traits/mod.rs`

Các facet bước đầu:

- `ProviderConfigPort`
- `WorkerCoordinationPort`
- `GovernanceRuntimePort`

Ý nghĩa:

- Cho phép service mới hoặc test mới depend vào interface nhỏ hơn thay vì toàn bộ `DatabasePort`.
- Không phá vỡ adapter SQLite hiện tại nhờ blanket impl từ `DatabasePort`.

Phần còn mở:

- Cần migrate từng service sang facet cụ thể, bắt đầu từ provider resolution, worker coordination, tool gateway/governance, knowledge read/write, collaboration/learning.
- Chỉ khi các call site chính không còn nhận `Arc<dyn DatabasePort>` thì nợ "mock phải implement 185 method" mới được đóng hoàn toàn.

### 6. Line endings

Kết quả trước sửa:

- 9 file mixed CRLF/LF.
- Nhiều file CRLF đồng nhất.
- Không có `.editorconfig`/`.gitattributes` trong `agentforge-ui`.

Đã sửa:

- Thêm `.editorconfig`: UTF-8, LF, final newline, trim trailing whitespace.
- Thêm `.gitattributes`: text LF, DB/media binary.
- Normalize toàn bộ text tracked sang LF.
- Chạy `cargo fmt`.

Kết quả sau sửa:

- `CRLF files: 0`
- `Mixed files: 0`
- `cargo fmt --check` pass.

### 7. GovernanceManager

Trước đó:

- `src/application/orchestration/governance.rs` tồn tại nhưng không export trong `orchestration/mod.rs`.
- `AppState` không có `GovernanceManager`.

Đã sửa:

- Export `pub mod governance`.
- Thêm `governance_manager: Arc<GovernanceManager>` vào `AppState`.
- Khởi tạo bằng `GovernanceManager::default().with_db(db.clone())`.

Ghi chú nghiệp vụ:

- Token budget enforcement thực tế đang nằm trong `ToolExecutionGateway::enforce_run_budget()` và được gọi từ `AgentExecutor`.
- Audit trail thực tế đang persist qua `insert_audit_log()` trong gateway/executor/collaboration.
- `GovernanceManager` đã được đưa vào AppState để không còn là module chết, nhưng còn cần hợp nhất dần policy engine cũ với gateway runtime để tránh hai nguồn policy song song.

## Các mục còn mở sau lần xử lý này

### P0/P1 còn mở từ các báo cáo trước

1. E2E release gate với DB nâng cấp thật, restart app, flow chat -> iFlow -> approval -> artifact -> learning promotion/rollback.
2. MCP true sandbox/catalog install theo chuẩn config/remote server; hiện tại đã fail-closed stdio và có secret refs, nhưng market/install UX vẫn cần chuẩn hóa thêm.
3. Automatic benchmark/canary/circuit breaker/pullback cho learning evolution.
4. Tokenizer/adversarial context safety để đo budget/context chính xác hơn thay vì char cap.
5. RBAC admin UX cho role/grant/policy operation.

### Nợ kỹ thuật còn mở từ nhận định bổ sung

1. Refactor `DatabasePort` sang facet traits tại call site chính.
2. Reactive worker queue hoàn toàn: thay discovery/poll bằng notification khi task được insert/claimable.
3. Hợp nhất `GovernanceManager` và `ToolExecutionGateway` thành một policy runtime duy nhất.
4. Dọn nhánh adapter legacy trong UI chat để dùng chung provider factory/cache.

## Verification đã chạy

- `cargo fmt --check`: pass.
- `cargo check --lib`: pass, còn 2 warning dead code cũ trong `src/ui/text`.
- Line ending scan: `CRLF files: 0`, `Mixed files: 0`.

Không chạy `cargo test --test integration_tests -- --test-threads=1` theo yêu cầu trước đó.

## Cập nhật triển khai bổ sung

Sau yêu cầu "thực hiện nốt các mục còn mở", đã bổ sung thêm các thay đổi sau:

### Worker reactive hơn

- Khi worker nhận direct message hoặc broadcast message, worker gọi `try_execute_next_task()` ngay sau `handle_message()`.
- Điều này giúp task vừa được tạo kèm message `[NEW_TASK]` hoặc iFlow dispatch không phải chờ vòng poll idle tiếp theo.
- Adaptive backoff vẫn giữ vai trò fallback nếu có task được ghi trực tiếp xuống DB mà không phát message.

### Governance policy runtime

- Đưa logic token budget enforcement về `GovernanceManager::enforce_run_budget_for_db()`.
- `ToolExecutionGateway::enforce_run_budget()` giờ gọi vào `GovernanceManager`, tránh policy runtime song song.
- Khi budget bị chặn, hệ thống ghi cả `run_event` và `audit_log`.

### Learning evolution circuit breaker

- Benchmark fail vì safety violation/unauthorized side effect sẽ ghi `promotion_decision=blocked_by_benchmark_circuit`.
- Nếu một candidate có từ 3 benchmark fail trở lên, hệ thống tự ghi quyết định blocked.
- Canary fail giờ kích hoạt automatic pullback:
  - cập nhật deployment thành `failed`;
  - rollback candidate activation/baseline;
  - ghi `rollback_record`;
  - transition candidate sang `rolled_back`;
  - đưa lesson về `validated` thay vì active production.

### Context budget và adversarial context safety

- Retrieval/governed context không chỉ cắt theo character cap nữa.
- Bổ sung estimator token nội bộ:
  - `governance_max_retrieved_context_tokens`;
  - `governance_max_governed_context_tokens`;
  - fallback từ các setting `*_chars` cũ bằng quy đổi xấp xỉ.
- Untrusted retrieved context vẫn redact credential markers và escape `<tool_call>` tags trước khi inject.

### Provider factory cleanup

- `provider_factory::create_adapter()` giờ instantiate đúng adapter cho `codex` và `opencode`.
- UI chat helper `build_provider_adapter()` đã chuyển sang dùng shared provider factory.
- Phần debate/chat legacy còn một match lớn dựng adapter thủ công ở sâu trong `team_workspace/chat.rs`; compile vẫn pass, nhưng nên refactor tiếp bằng helper chung vì đây là đoạn legacy trùng lặp lớn và khó review.

### Release gate script

Đã thêm `scripts/verify_system_closure.ps1` để kiểm tra nhanh các gate không cần chạy integration target đã bị loại trừ:

- `cargo fmt --check`;
- `cargo check --lib`;
- line ending LF/mixed scan;
- worker không còn fixed interval/list_instances trong execution;
- worker dùng `ProviderAdapterCache`;
- governance manager có trong AppState;
- learning có benchmark/canary circuit breaker.

Verification bổ sung:

- `cargo fmt --check`: pass.
- `cargo check --lib`: pass, còn 2 warning dead code cũ trong `src/ui/text`.
- `powershell -ExecutionPolicy Bypass -File scripts\verify_system_closure.ps1 -SkipCargo`: pass.
- Line ending scan gồm tracked + untracked text: `CRLF files: 0`, `Mixed files: 0`.

## Trạng thái còn lại sau bổ sung

Các phần còn lại chủ yếu là release/operational hoặc refactor legacy lớn:

1. Chạy E2E bằng app thật qua các flow chat -> iFlow -> approval -> artifact -> promotion -> rollback sau khi user cho phép test runtime đầy đủ.
2. Refactor sâu `team_workspace/chat.rs` để thay match provider legacy bằng một executor helper duy nhất.
3. Migrate các service chính từ `Arc<dyn DatabasePort>` sang facet traits đã tạo.
4. Stdio MCP true OS sandbox vẫn chưa thể được bảo đảm trong app process; hiện vẫn fail-closed trừ khi bật env `AGENTFORGE_ALLOW_UNSANDBOXED_MCP_STDIO=true`. Remote MCP/config import đã có validation và secret refs.
