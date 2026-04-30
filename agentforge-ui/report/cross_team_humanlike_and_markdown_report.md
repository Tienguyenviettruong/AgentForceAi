# Báo cáo thiết kế: Cross-team Human-like Protocol & Thống nhất Markdown Renderer

## 0. Mục tiêu

Hệ thống hướng tới trải nghiệm “các nhóm trao đổi như con người”, nhưng vẫn tự động hóa tối đa ở chế độ Auto:

- Mỗi handoff liên instance phải mang đủ bối cảnh (tài liệu, quyết định, việc cần làm, câu hỏi mở).
- Instance nhận phải “ack + readback” (xác nhận đã hiểu) trước khi triển khai.
- Trong quá trình làm việc phải có trạng thái/timeline rõ ràng (UI stepper).
- Kết quả cuối phải được bàn giao ngược lại để team nguồn review/consensus.
- Markdown hiển thị thống nhất, hỗ trợ markdown chuẩn và Obsidian-specific syntax cho Knowledge.

Phạm vi báo cáo này:

1) Chuẩn hóa “cross-team protocol” theo 3 mode: Human / Supervisor / Auto.
2) Đề xuất UI timeline bằng Stepper: https://longbridge.github.io/gpui-component/docs/components/stepper
3) Đánh giá và refactor markdown renderer theo hướng thống nhất + context-aware (Chat vs Knowledge vs Obsidian).

---

## 1) Hiện trạng kỹ thuật liên quan

### 1.1 Cross-team handoff hiện tại chỉ là message payload

Tool `handoff_to_team` gửi message broadcast kèm payload JSON trong content và metadata.

- [executor.rs](file:///workspace/agentforge-ui/src/application/orchestration/executor.rs#L330-L357)

Payload hiện tại chỉ có:

- `handoff_type`
- `correlation_id`
- `from_team`
- `reply_to_team`
- `briefing_package` (text)

Điểm thiếu: không có “deliverables list”, “decisions”, “open questions”, “references (docs/chunks)”, “status events”.

### 1.2 “Đọc tài liệu” hiện tại chủ yếu dựa vào RAG theo task_text

Khi worker chạy task, nó có bước embedding → search_similar_chunks → inject vào instructions nếu similarity > 0.6.

- [worker.rs](file:///workspace/agentforge-ui/src/application/orchestration/worker.rs#L225-L243)

Điểm thiếu: không có đảm bảo instance nhận handoff sẽ thấy được toàn bộ tài liệu output của instance gửi (PRD/SRS/Plan), vì:

- indexing/chunking có thể chưa kịp,
- độ tương đồng có thể thấp,
- correlation_id chưa được dùng như “case key” để truy xuất tài liệu liên quan.

### 1.3 Viết file trong task runner dựa vào chuẩn ```file:

Agent chỉ “viết file” nếu output đúng format markdown code block:

```md
```file:/path/to/file
contents
```
```

Parser/Writer:

- [chat_service.rs](file:///workspace/agentforge-ui/src/application/services/chat_service.rs#L77-L126)

---

## 2) Cross-team Protocol “giống con người” theo 3 chế độ

### 2.1 Định danh các vai trò trong protocol

- **Owner instance (nguồn)**: instance đang sở hữu bối cảnh/tài liệu gốc (ví dụ document-6560).
- **Target instance (đích)**: instance thực hiện phần việc tiếp theo (ví dụ backend-c501).
- **Handler (người tiếp nhận)**: 1 agent được chọn để handle cross-team request trong instance đích.
- **Contributors**: các agent được giao subtask.

Chú ý: runtime hiện dùng `agent.name` như “role string” cho routing/assignment ([worker.rs](file:///workspace/agentforge-ui/src/application/orchestration/worker.rs#L64-L67)).

### 2.2 Chuẩn hóa gói handoff (Handoff Package v2)

Đề xuất mở rộng payload JSON (không phá backward compatibility):

```json
{
  "handoff_type": "message",
  "correlation_id": "uuid",
  "from_team": "document-6560",
  "reply_to_team": "document-6560",
  "briefing_package": "text summary (short)",
  "context": {
    "objective": "…",
    "decisions": [
      {"id":"D1","text":"RBAC Owner/Viewer/Admin"},
      {"id":"D2","text":"Storage: S3/MinIO + Postgres metadata"}
    ],
    "open_questions": [
      {"id":"Q1","text":"Auth provider? OIDC vs local"},
      {"id":"Q2","text":"File virus scan required?"}
    ],
    "references": [
      {"type":"obsidian_note","title":"SRS_Document_Management_System.md"},
      {"type":"obsidian_note","title":"Project_Plan.md"}
    ],
    "deliverables": [
      {"id":"DEL1","type":"api_spec","expected":"OpenAPI + endpoints list"},
      {"id":"DEL2","type":"db_schema","expected":"DDL + ERD"},
      {"id":"DEL3","type":"implementation","expected":"FastAPI skeleton + auth middleware"}
    ]
  }
}
```

Nguyên tắc:

- `briefing_package` vẫn giữ dạng text ngắn để hiển thị trong thread list.
- `context` chứa phần “human-like”: quyết định đã chốt, việc cần làm, tài liệu tham chiếu.
- `references` là metadata, giúp instance đích biết “cần đọc gì”; không bắt buộc embed full content.

### 2.3 Event protocol cho timeline (status updates)

Đề xuất các event type chuẩn để instance đích gửi về instance nguồn:

- `ACK_RECEIVED`: đã nhận yêu cầu.
- `READBACK_CONFIRMED`: đã hiểu lại (tóm tắt lại yêu cầu + xác nhận).
- `PLAN_CREATED`: đã có plan + phân công.
- `SUBTASKS_DISPATCHED`: subtasks đã được tạo.
- `BLOCKED`: bị chặn + câu hỏi cần trả lời.
- `PARTIAL_RESULT`: có kết quả một phần + references.
- `FINAL_RESULT`: kết quả cuối + deliverables list.
- `REVIEW_REQUEST`: đề nghị review.
- `REVIEW_RESPONSE`: phản hồi review.
- `CONSENSUS_REACHED`: chốt.

Payload tối thiểu cho event:

```json
{
  "handoff_type": "status_event",
  "correlation_id": "uuid",
  "from_team": "backend-c501",
  "reply_to_team": "document-6560",
  "event": {
    "type": "PLAN_CREATED",
    "summary": "…",
    "references": [{"type":"file","path":"..."}],
    "timestamp": "RFC3339"
  }
}
```

### 2.4 Mapping vào 3 chế độ

#### Human mode (ít tự động hơn, giống người hơn)

- Bắt buộc phát `ACK_RECEIVED` + `READBACK_CONFIRMED` trước khi tạo subtasks.
- Nếu còn `open_questions`, bắt buộc phát `BLOCKED` và chờ phản hồi.
- UI stepper hiển thị “Waiting for input”.

#### Supervisor mode (có kiểm duyệt)

- Bắt buộc `REVIEW_REQUEST/REVIEW_RESPONSE` cho deliverables quan trọng (API spec/DB schema).
- Không tự “commit/merge” code nếu chưa có `REVIEW_RESPONSE`.
- Stepper có step “Supervisor Review”.

#### Auto mode (tự động tối đa)

- Tự tạo subtasks ngay sau ACK/Readback.
- Tự gửi status event theo milestone (mỗi 1–2 phút hoặc khi task state đổi).
- Khi tất cả subtasks hoàn thành, tự gửi `FINAL_RESULT` + đề nghị review.

---

## 3) UI Timeline bằng Stepper

### 3.1 Đơn vị hiển thị: “Case” theo correlation_id

Mỗi cross-team request là một “case”.

Đề xuất UI:

- “Cross-team Inbox” hiển thị list case.
- Click case mở detail view gồm:
  - Stepper timeline
  - Thread messages (markdown)
  - Deliverables panel (file list + links)

### 3.2 Stepper states (đề xuất)

Stepper steps:

1) Received
2) Readback
3) Plan
4) In Progress
5) Review
6) Done

Mapping event → step:

- ACK_RECEIVED → Received
- READBACK_CONFIRMED → Readback
- PLAN_CREATED/SUBTASKS_DISPATCHED → Plan
- PARTIAL_RESULT → In Progress
- REVIEW_REQUEST/REVIEW_RESPONSE → Review
- FINAL_RESULT/CONSENSUS_REACHED → Done

---

## 4) Báo cáo Markdown: vấn đề & hướng refactor

### 4.1 Vấn đề cốt lõi: 2 renderer song song, không tương thích

Hiện trạng (theo quan sát codebase và mô tả issue):

- Renderer A: `TextView::markdown()` (gpui_component)
  - Ưu: selectable/copy tốt.
  - Nhược: ít khả năng “context-aware” (Obsidian link/tags/custom blocks).
- Renderer B: `render_markdown_message()` (custom pulldown-cmark)
  - Ưu: custom style, có thể embed behavior.
  - Nhược: không selectable; thiếu wikilinks/images/footnotes; spacing split_whitespace gây méo formatting.

Hậu quả: Knowledge panel và Chat panel nhìn/behavior khác nhau, gây cảm giác hệ thống “không thống nhất”.

### 4.2 Mục tiêu refactor markdown

1) Một nguồn render thống nhất (hoặc ít nhất thống nhất trong Knowledge).
2) Selectable/copy là bắt buộc với Knowledge.
3) Context-aware cho Obsidian:
   - wikilinks `[[...]]`
   - tags `#tag` (chỉ bật trong Knowledge)
   - image
4) Hỗ trợ “file code blocks” (```file:) ổn định cho Chat/Task outputs.

### 4.3 Khuyến nghị triển khai theo 3 phase

#### Phase 1 (Quick wins)

- Knowledge tree click và graph double-click dùng chung 1 renderer.
  - Ưu tiên `TextView::markdown` cho Knowledge để có selectable/copy ngay.
- Preprocess wikilinks trước khi render (Knowledge only):
  - `[[note]]` → `[note](obsidian://open?...)` hoặc link nội bộ app.
- Giới hạn tag styling theo context (Knowledge only), không áp dụng cho Chat.
- Codeblock: hiển thị label ngôn ngữ ở header (không cần highlight ngay).

#### Phase 2 (Unified renderer)

- Tạo “MarkdownRenderer” có config theo context:

```rust
pub struct MarkdownRenderConfig {
    pub enable_obsidian_tags: bool,
    pub enable_wikilinks: bool,
    pub enable_file_code_blocks: bool,
    pub enable_selection: bool,
}
```

- Chat dùng config enable_file_code_blocks.
- Knowledge dùng enable_selection + wikilinks + obsidian_tags.

#### Phase 3 (Enhancements)

- Syntax highlighting (syntect/tree-sitter) cho code blocks.
- Image loading async.
- Footnotes rendering.
- Math/LaTeX (nếu cần).

---

## 5) Những thay đổi tối thiểu để đạt mục tiêu “tự động nhiều & giống con người”

### 5.1 Handoff package v2 + status events

- Mở rộng payload JSON của `[CROSS_TEAM_HANDOFF]` để mang đủ context.
- Thêm `handoff_type = status_event`.
- Persist event theo correlation_id để UI stepper hiển thị timeline.

### 5.2 Hành vi bắt buộc của handler

- Bắt buộc ACK + Readback ở mọi mode.
- Auto/Supervisor khác nhau ở chỗ:
  - Auto: tự tạo subtasks ngay.
  - Supervisor: chặn tại “Review” cho deliverables quan trọng.

### 5.3 Cơ chế “sync tài liệu” có định danh

- Trong handoff payload, references phải trỏ rõ “note/file nào”.
- Instance đích khi chạy task:
  - ưu tiên truy xuất theo correlation_id (messages + refs),
  - sau đó mới dùng similarity search như fallback.

---

## 6) Checklist test/acceptance criteria

### Cross-team

- Khi instance nguồn gửi handoff, instance đích luôn phát ACK và readback trong 1 phút.
- Timeline stepper hiển thị đúng thứ tự step theo correlation_id.
- Instance đích gửi tối thiểu 1 status update mỗi khi:
  - tạo plan,
  - dispatch subtasks,
  - hoàn thành subtasks,
  - blocked,
  - final result.
- Final result phải kèm deliverables list và references.

### Markdown

- Knowledge: selectable/copy hoạt động.
- Knowledge: wikilinks và tags hiển thị đúng, không ảnh hưởng chat.
- Chat: code blocks hiển thị ổn định; file code blocks render rõ ràng.
- Knowledge tree click và graph double-click hiển thị nhất quán.

