# Tasks

- [x] Task 1: Chuẩn hóa schema cross-team (Handoff Package v2 + Status Events)
  - [x] Thêm struct/model cho payload v2 và status_event (giữ backward compatible với payload cũ)
  - [x] Bổ sung parse/route cho `handoff_type = status_event`
  - [x] Bổ sung “readback + ack” behavior theo mode (Human/Supervisor/Auto)

- [x] Task 2: Persist case timeline theo correlation_id
  - [x] Thiết kế bảng DB tối thiểu: `cross_team_cases`, `cross_team_case_events`
  - [x] Implement DB port + sqlite adapter: upsert case, insert event, list cases, list events theo case
  - [x] Backfill tối thiểu từ `team_messages` (nếu có thể) hoặc chỉ áp dụng cho message mới

- [x] Task 3: UI Cross-team Inbox + Case Detail dùng Stepper
  - [x] Tạo panel/section hiển thị danh sách case (group theo correlation_id)
  - [x] Tạo view chi tiết case: Stepper + message thread + deliverables list
  - [x] Mapping event → step (Received/Readback/Plan/In Progress/Review/Done)

- [x] Task 4: Thống nhất Markdown renderer cho Knowledge
  - [x] Chọn renderer chuẩn cho Knowledge (ưu tiên selectable/copy)
  - [x] Đồng bộ tree click và graph double-click dùng cùng renderer

- [x] Task 5: Context-aware markdown (Quick wins)
  - [x] Preprocess wikilinks `[[...]]` (Knowledge-only) trước khi đưa vào markdown parser
  - [x] Giới hạn tag styling theo context (Knowledge-only)
  - [x] Hiển thị label ngôn ngữ codeblock (lang) trong custom renderer (không cần highlight)

- [x] Task 6: Validation
  - [x] Bổ sung test DB cho case/event persistence (unit/integration)
  - [x] Smoke test UI: tạo handoff → nhận ack/readback → tạo subtasks → phát events → stepper cập nhật

# Task Dependencies
- Task 2 depends on Task 1
- Task 3 depends on Task 1 and Task 2
- Task 4 and Task 5 can run in parallel
- Task 6 depends on Task 1-5
