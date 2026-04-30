# Đặc tả: Cross-team Human-like Protocol & Thống nhất Markdown Renderer

## Why
Hiện tại cross-team chủ yếu truyền một đoạn text ngắn nên instance nhận không nắm đủ bối cảnh/tài liệu/điểm đã chốt, dẫn đến trao đổi không tự nhiên như con người. Song song đó, UI đang dùng 2 markdown renderer khác nhau khiến hiển thị không nhất quán và thiếu hỗ trợ Obsidian-specific syntax.

## What Changes
- Chuẩn hóa payload `[CROSS_TEAM_HANDOFF]` theo “Handoff Package v2” (giữ tương thích ngược), bổ sung `context` (decisions/open_questions/references/deliverables).
- Chuẩn hóa “Status Events” theo `correlation_id` để thể hiện timeline (ACK/Readback/Plan/InProgress/Review/Done).
- Persist “cross-team cases” và “status events” để UI có thể hiển thị stepper theo từng case.
- Thêm UI “Cross-team Inbox + Case Detail” dùng Stepper để hiển thị tiến độ liên instance.
- Thống nhất markdown renderer trong Knowledge (tree click vs graph double-click) và bổ sung cấu hình render theo context (Chat/Knowledge).
- Bổ sung hỗ trợ tối thiểu cho Obsidian syntax: wikilinks `[[...]]`, tags `#tag` (chỉ trong Knowledge), và image (giai đoạn sau).

## Impact
- Affected specs:
  - Cross-team communication protocol (message schema, workflow)
  - Orchestration mode behavior (Human/Supervisor/Auto)
  - UI timeline visualization (Stepper)
  - Markdown rendering consistency (Chat/Knowledge)
  - Knowledge/Obsidian interoperability (wikilinks/tags)
- Affected code:
  - Cross-team routing/tooling: [executor.rs](file:///workspace/agentforge-ui/src/application/orchestration/executor.rs), [worker.rs](file:///workspace/agentforge-ui/src/application/orchestration/worker.rs)
  - Message persistence: [sqlite_adapter.rs](file:///workspace/agentforge-ui/src/infrastructure/database/sqlite_adapter.rs)
  - UI: [team_workspace/chat.rs](file:///workspace/agentforge-ui/src/ui/panels/team_workspace/chat.rs), [knowledge.rs](file:///workspace/agentforge-ui/src/ui/panels/knowledge.rs)
  - Markdown: [markdown.rs](file:///workspace/agentforge-ui/src/ui/components/markdown.rs)

## ADDED Requirements

### Requirement: Handoff Package v2
Hệ thống SHALL gửi cross-team handoff theo payload JSON có mở rộng trường `context` nhưng vẫn tương thích với payload cũ.

#### Scenario: Success case
- **WHEN** instance nguồn gọi `handoff_to_team`
- **THEN** message payload được lưu trong `metadata` và `content` theo format `[CROSS_TEAM_HANDOFF] {json}`
- **AND** payload có thể chứa `context` gồm `objective`, `decisions`, `open_questions`, `references`, `deliverables`
- **AND** instance đích có thể parse payload thành cấu trúc thống nhất

### Requirement: Status Events (Case Timeline)
Hệ thống SHALL hỗ trợ `handoff_type = "status_event"` để cập nhật tiến độ theo `correlation_id`.

#### Scenario: Success case
- **WHEN** instance đích bắt đầu xử lý một handoff
- **THEN** hệ thống phát ra event `ACK_RECEIVED`
- **AND** trước khi tạo subtasks, hệ thống phát ra event `READBACK_CONFIRMED`
- **AND** khi đã có plan/subtasks, hệ thống phát ra `PLAN_CREATED` và `SUBTASKS_DISPATCHED`
- **AND** khi hoàn tất, hệ thống phát `FINAL_RESULT` và/hoặc `CONSENSUS_REACHED`

### Requirement: Persist Case & Events
Hệ thống SHALL persist các case cross-team và events để UI có thể render timeline ổn định sau khi reload.

#### Scenario: Success case
- **WHEN** nhận message `[CROSS_TEAM_HANDOFF]` hoặc `status_event`
- **THEN** hệ thống upsert bản ghi “case” theo `correlation_id`
- **AND** insert bản ghi “event” theo `correlation_id` và `created_at`
- **AND** UI reload vẫn hiển thị đúng tiến độ và lịch sử

### Requirement: UI Stepper Timeline
UI SHALL hiển thị timeline theo stepper cho mỗi `correlation_id` (case).

#### Scenario: Success case
- **WHEN** user mở “Cross-team Inbox”
- **THEN** UI hiển thị danh sách case theo `correlation_id` và trạng thái hiện tại
- **WHEN** user mở chi tiết một case
- **THEN** UI hiển thị stepper với các step: Received → Readback → Plan → In Progress → Review → Done
- **AND** step hiện tại được suy ra từ event mới nhất

### Requirement: Unified Markdown Rendering (Knowledge Consistency)
Hệ thống SHALL dùng một renderer thống nhất cho Knowledge tree click và graph double-click để đảm bảo nhất quán.

#### Scenario: Success case
- **WHEN** user mở cùng một knowledge note từ tree hoặc graph
- **THEN** heading/link/code/list style nhất quán
- **AND** selectable/copy hoạt động (ưu tiên trong Knowledge)

### Requirement: Context-aware Markdown Rendering
Hệ thống SHALL cung cấp cấu hình render markdown theo context (Chat vs Knowledge).

#### Scenario: Success case
- **WHEN** render trong Chat
- **THEN** bật `file code blocks` (```file:) và tắt Obsidian tag styling mặc định
- **WHEN** render trong Knowledge
- **THEN** bật wikilinks/tags theo Obsidian và bật selectable/copy

## MODIFIED Requirements

### Requirement: Existing Cross-team Handoff Parsing
Parser hiện tại SHALL tiếp tục parse payload cũ (không có `context`) và map vào cấu trúc v2 với các trường thiếu để trống.

## REMOVED Requirements
Không có.

