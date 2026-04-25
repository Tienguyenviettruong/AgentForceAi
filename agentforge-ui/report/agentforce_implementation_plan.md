# Kế Hoạch Triển Khai (Implementation Plan) - AgentForge Core Engine (Cập nhật Chi tiết)

Dựa trên tài liệu phân tích hệ thống (`agentforce_system_analysis_report.md`), bản kế hoạch này đã được cập nhật để khắc phục tình trạng "sơ sài", đặc biệt tập trung sâu vào **Logic phối hợp, giao tiếp và tranh luận (Debate/Collaboration)** giữa các Agent trong cùng một Team và giữa các Team với nhau.

---

## Sprint 1: Hoàn thiện Database & Nền tảng (Foundation)
**Mục tiêu:** Củng cố SQLite Schema, đảm bảo dữ liệu không bị mất và chuẩn bị sẵn sàng cho MCP & Knowledge.
- **Task 1.1:** Viết script migration (hoặc update `sqlite_adapter.rs`) để thêm bảng `mcp_tools`, `knowledge_entries`, và `knowledge_entries_fts`.
- **Task 1.2:** Cập nhật bảng `tasks` để hỗ trợ cơ chế khóa nguyên tử (atomic claim) bằng cách thêm các trường `claimed_by`, `status`, `dependencies`, `parent_task_id`.
- **Task 1.3:** Cập nhật `McpToolRegistry` (`src/infrastructure/mcp/registry.rs`) để đọc/ghi trực tiếp vào DB thay vì `RwLock<HashMap>` trên RAM.

## Sprint 2: Nâng cấp Agent Core Loop & Quản lý Bộ nhớ (Memory Management)
**Mục tiêu:** Xóa bỏ logic gọi LLM trực tiếp, tích hợp khả năng gọi Tool, và khắc phục lỗi "loạn ngữ cảnh".
- **Task 2.1:** Thiết kế struct `AgentExecutor` (áp dụng ReAct Pattern) để bọc LLM Adapter, cho phép LLM trả về `tool_calls` và gọi thực thi lệnh.
- **Task 2.2 (Context Pruning):** Implement cơ chế `Sliding Window` cho `full_history`. Xóa bỏ việc `clone()` toàn bộ lịch sử. Chỉ giữ lại K tin nhắn gần nhất và System Prompt.
- **Task 2.3 (Summarization):** Viết background job `Summarizer`. Khi token vượt ngưỡng (VD: > 8000), tóm tắt các đoạn hội thoại cũ thành `Working Memory Summary` và chèn vào System Prompt.
- **Task 2.4 (Thought Extraction):** Viết MCP Tool nội bộ `save_to_knowledge`. Agent sẽ gọi tool này sau mỗi tác vụ để lưu kết luận vào `knowledge_entries`.

## Sprint 3: Phối hợp và Tranh luận trong 1 Team (Intra-team Collaboration & Debate)
**Mục tiêu:** Các Agent trong cùng 1 Team (vd: Dev, Tester, BA) tự phân chia công việc, nhận thức vai trò, và có khả năng tranh luận thực sự để đi đến đồng thuận (không dùng vòng lặp `for` hardcode).
- **Task 3.1 (Role Awareness):** Cập nhật `ChatService::build_dynamic_system_prompt` để nhúng rõ "Sơ đồ tổ chức" (Role hierarchy) vào System Prompt, giúp Agent biết ai là sếp (Coordinator), ai là đồng nghiệp, và mình có quyền gì.
- **Task 3.2 (Task Polling & Claiming):** Xóa bỏ cách gọi luân phiên. Các Agent (trừ Coordinator) sẽ chạy một vòng lặp nền (Background Worker) để `poll` bảng `tasks`. Khi thấy task phù hợp chuyên môn -> `Atomic Claim` -> Xử lý -> Trả kết quả lên `TeamBus`.
- **Task 3.3 (Debate Mode Logic):** Xây dựng luồng `Debate Protocol`:
  - Khi một Agent đưa ra giải pháp, hệ thống phát tín hiệu `Debate_Requested` qua `TeamBus`.
  - Các Agent khác (như Reviewer, Tester) nhận được tín hiệu sẽ đọc giải pháp, đánh giá (Critique) và phản biện.
  - Vòng lặp phản biện diễn ra tự do cho đến khi `Coordinator` (Agent quản lý) gọi một Tool nội bộ tên là `declare_consensus` (Chốt đồng thuận) để kết thúc vòng tranh luận và sinh ra Task mới.

## Sprint 4: Phối hợp chéo giữa các Team (Cross-team Routing)
**Mục tiêu:** Team này (vd: Design Team) làm xong việc, tự động bàn giao kết quả và yêu cầu Team khác (vd: Dev Team) làm tiếp thông qua `TeamBusRouter`.
- **Task 4.1 (Cross-team Subscriptions):** Cho phép `Coordinator` của Team B "lắng nghe" (subscribe) vào các sự kiện hoàn thành (Milestone) của Team A thông qua `TeamBusRouter`.
- **Task 4.2 (Handoff Protocol):** Khi Team A hoàn thành, `Coordinator` của Team A gọi Tool `handoff_to_team`. Gói gọn toàn bộ `Working Memory Summary` và các file kết quả thành một `Briefing Package`.
- **Task 4.3 (Cross-team Execution):** `TeamBusRouter` định tuyến `Briefing Package` này sang Team B. `Coordinator` Team B nhận được, đọc gói bàn giao, và tự động tạo ra các `tasks` mới trong `SharedTaskList` của Team B để các thành viên Team B bắt đầu làm việc.