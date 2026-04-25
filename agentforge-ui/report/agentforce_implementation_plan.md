# Kế Hoạch Triển Khai (Implementation Plan) - AgentForge Core Engine

Dựa trên tài liệu `agentforce_system_analysis_report.md` và `agentforce_remediation_plan.md`, dưới đây là kế hoạch (Plan) theo từng Sprint để tái cấu trúc và hoàn thiện hệ thống AgentForge từ một giao diện Mock thành một nền tảng Multi-Agent thực thụ.

## Sprint 1: Hoàn thiện Database & Nền tảng (Foundation)
**Mục tiêu:** Củng cố SQLite Schema, đảm bảo dữ liệu không bị mất và chuẩn bị sẵn sàng cho MCP & Knowledge.
- **Task 1.1:** Viết script migration (hoặc update `sqlite_adapter.rs`) để thêm bảng `mcp_tools`, `knowledge_entries`, và `knowledge_entries_fts`.
- **Task 1.2:** Implement các hàm CRUD cho `mcp_tools` trong `DatabasePort`.
- **Task 1.3:** Cập nhật `McpToolRegistry` (`src/infrastructure/mcp/registry.rs`) để đọc/ghi trực tiếp vào DB thay vì `RwLock<HashMap>` trên RAM.
- **Task 1.4:** Implement các hàm CRUD cho `knowledge_entries` phục vụ cho việc lưu trữ dài hạn (Long-term memory).

## Sprint 2: Nâng cấp Agent Core Loop & Function Calling
**Mục tiêu:** Xóa bỏ logic gọi LLM trực tiếp, tích hợp khả năng gọi Tool.
- **Task 2.1:** Thiết kế struct `AgentExecutor` (áp dụng ReAct Pattern) để bọc LLM Adapter.
- **Task 2.2:** Cập nhật LLM Adapters (Claude, OpenRouter) để hỗ trợ `tool_calls` và JSON Schema tools array.
- **Task 2.3:** Tích hợp `AgentExecutor` vào `src/ui/panels/team_workspace/chat.rs`. Thay vì truyền toàn bộ lịch sử rồi đợi chữ, AgentExecutor sẽ bắt các `tool_calls`, gọi MCP tool/CLI, rồi trả kết quả (`tool_result`) ngược lại cho LLM.
- **Task 2.4:** Kết nối tính năng WebSearch bằng API thật (Tavily/DuckDuckGo) vào `AgentExecutor` (thay thế mock timer trong `research_notebook.rs`).

## Sprint 3: Quản lý Bộ nhớ (Memory Management)
**Mục tiêu:** Xử lý triệt để lỗi "quên", "loạn ngữ cảnh" do quá tải token.
- **Task 3.1:** Implement cơ chế `Sliding Window` cho `full_history`. Chỉ clone K tin nhắn gần nhất và System Prompt, không clone toàn bộ lịch sử.
- **Task 3.2:** Viết background job `Summarizer`. Khi token vượt ngưỡng (VD: > 8000), lấy các tin nhắn cũ gọi LLM để tóm tắt thành `Working Memory Summary`.
- **Task 3.3:** Viết MCP Tool nội bộ `save_to_knowledge`. Dạy LLM cách gọi tool này sau khi phân tích xong để lưu quyết định vào `knowledge_entries`.

## Sprint 4: Phối hợp Team (Multi-Agent Routing & Task List)
**Mục tiêu:** Các Agent tự chia việc, không dùng vòng lặp `for` (Debate Mode) bị hardcode.
- **Task 4.1:** Kích hoạt `SharedTaskList` (bảng `tasks`). Viết logic cho Agent Coordinator bóc tách prompt người dùng thành sub-tasks và `INSERT` vào DB.
- **Task 4.2:** Viết worker loop cho từng Agent. Agent sẽ `poll` (quét) bảng `tasks`, khóa (Atomic Claim) task phù hợp chuyên môn.
- **Task 4.3:** Tích hợp `TeamBusRouter`. Khi Agent hoàn thành task, phát Broadcast event để cập nhật UI và báo cho Agent tiếp theo làm việc.

---
*Lưu ý: Plan này được thiết kế để thực hiện tuần tự. Sprint trước là tiền đề bắt buộc cho Sprint sau.*