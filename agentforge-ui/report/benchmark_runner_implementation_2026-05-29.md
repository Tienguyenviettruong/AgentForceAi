# Benchmark Runner Implementation - 2026-05-29

## Muc tieu

Dong vong tu tien hoa co kiem soat bang Benchmark Runner tu dong. Runner khong promote candidate, khong chay tool production, va khong bypass governance. No chi chay shadow benchmark, ghi evidence, roi cap nhat candidate sang `benchmark_passed` hoac `benchmark_failed`.

## Thay doi chinh

### 1. Data model

Da bo sung model:

- `BenchmarkSuiteRecord`
- `BenchmarkCaseRecord`
- `BenchmarkResultRecord`
- `BenchmarkRunnerJobRecord`

Schema moi:

- `benchmark_runner_jobs`

Schema da co va duoc noi vao API:

- `benchmark_suites`
- `benchmark_cases`
- `benchmark_results`

DatabasePort da co them cac method:

- `upsert_benchmark_suite`
- `get_benchmark_suite`
- `list_active_benchmark_suites`
- `insert_benchmark_case`
- `list_benchmark_cases_for_suite`
- `insert_benchmark_result`
- `list_benchmark_results_for_run`
- `insert_benchmark_runner_job`
- `update_benchmark_runner_job`
- `list_benchmark_runner_jobs_for_candidate`
- `list_benchmark_runner_jobs_by_status`

### 2. Benchmark Runner service

File moi:

- `src/application/orchestration/benchmark_runner.rs`

Runner thuc hien:

1. Kiem tra actor co quyen `governance:learning:promote`.
2. Resolve benchmark suite.
3. Neu khong co suite duoc chon, tao default suite theo candidate kind:
   - `default-governed-skill-v1`
   - `default-governed-workflow-v1`
4. Tao `benchmark_runner_jobs` status `running`.
5. Chay deterministic shadow check de bat loi schema, governance, safety va direct side effect.
6. Tao traffic LLM rieng cho tung benchmark case bang provider duoc cau hinh.
7. Dung evaluator LLM/rubric JSON de cham output theo expectation, safety violation va unauthorized side effect.
8. Lay diem cuoi cung theo min(deterministic score, evaluator score), chi pass khi ca hai lop deu pass.
9. Ghi aggregate benchmark qua `LearningService::record_benchmark`.
10. Ghi tung `benchmark_results`.
11. Cap nhat job thanh `completed` hoac `failed`.
12. Neu provider/evaluator loi sau khi candidate da vao `benchmark_running`, runner ghi benchmark evidence that bai neu co the va dua candidate ve `benchmark_failed`.

### 3. Default benchmark suites

Skill candidate duoc kiem:

- Definition JSON hop le va `instructions` du dai.
- Instruction co dau hieu governance/constraint/safety.
- Instruction co dau hieu grounding/context/evidence.
- Chan cac cum tu nguy hiem nhu `bypass approval`, `disable safety`, `raw token`.

Workflow candidate duoc kiem:

- Workflow parse/validate duoc o draft mode.
- Co cac node toi thieu: `Start`, `AgentTask`, `End`.
- Co `HumanReview` cho governance.
- Khong co direct side-effect node: `SystemCommand`, `HttpRequest`.

### 4. Learning lifecycle

Da them trang thai trung gian:

- `benchmark_running`

Transition moi:

- `draft -> benchmark_running`
- `benchmark_failed -> benchmark_running`
- `benchmark_running -> benchmark_passed`
- `benchmark_running -> benchmark_failed`

`record_benchmark()` chap nhan state `benchmark_running`, nen runner co the chuyen state truoc khi ghi evidence.

### 5. UI

Tab Learning trong Orchestration co them:

- `Run Auto Benchmark`
- `Record Benchmark`
- Hien `Auto jobs` theo candidate.

Nut `Run Auto Benchmark` goi `BenchmarkRunner::run_candidate(candidate_id, None, current_actor)`.

### 6. Auto scheduling

Khi `LearningService::create_candidate()` tao candidate moi, he thong tu enqueue mot `benchmark_runner_jobs` status `queued` neu setting `learning_auto_benchmark_on_candidate_create` khong bang `false`.

Khi app khoi dong, `BenchmarkRunner::start_scheduler()` poll cac job `queued` va chay nen theo batch nho. Poll interval dung setting `learning_benchmark_runner_poll_ms`, mac dinh `60000ms`, clamp tu `5000ms` den `3600000ms`.

Provider cau hinh:

- `learning_benchmark_provider_name`: provider tao traffic LLM cho benchmark case.
- `learning_evaluator_provider_name`: provider cham diem/rubric. Neu khong cau hinh hoac resolve fail, fallback sang traffic provider.

Neu khong co provider LLM nao duoc cau hinh trong database, job bi mark `failed` va candidate van o state truoc benchmark neu benchmark chua bat dau.

## Trang thai sau bo sung theo yeu cau

Da hoan thanh cac muc ban neu:

- Tao traffic LLM thuc cho tung benchmark case.
- So sanh output bang evaluator LLM/rubric model voi JSON verdict bat buoc.
- Lap lich benchmark tu dong nen khi candidate moi duoc tao.

Muc "chay workflow trong sandbox runtime day du" duoc loai khoi scope theo chi dao moi cua ban. Runner hien tai van la shadow/evaluation runner co kiem soat, khong promote candidate, khong goi tool production va khong bypass governance.

## Verification

- `cargo fmt`: pass.
- `cargo fmt --check`: pass.
- `cargo check --lib`: pass.
- `git diff --check`: pass.
- `powershell -ExecutionPolicy Bypass -File scripts\verify_system_closure.ps1 -SkipCargo`: pass.

Khong chay `cargo test --test integration_tests -- --test-threads=1` theo yeu cau truoc do.
