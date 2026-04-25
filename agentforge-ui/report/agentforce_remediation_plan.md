# Kế hoạch Khắc phục và Hoàn thiện Core Engine (Nhánh `agentforce`)

Dựa trên các vấn đề cốt lõi đã được phát hiện trong báo cáo phân tích mã nguồn thực tế (`agentforce_system_analysis_report.md`), dưới đây là kế hoạch chi tiết từng bước để "đi dây" (wiring) lại hệ thống, chuyển đổi AgentForge từ một ứng dụng Chat UI đơn thuần thành một nền tảng Multi-Agent thực thụ.

## Giai đoạn 1: Hoàn thiện Database Schema & Hạ tầng Công cụ
**Mục tiêu:** Đảm bảo tất cả các bảng thiết yếu cho Memory và Tools đều tồn tại và được kết nối thay vì lưu tạm trên RAM (Mock).

1. **Khôi phục Hệ thống Knowledge (Lưu trữ suy nghĩ):**
   - **Hành động:** Bổ sung các câu lệnh `CREATE TABLE` cho bảng `knowledge_entries` và bảng `knowledge_entries_fts` (Full-Text Search) vào file `src/infrastructure/database/sqlite_adapter.rs`.
   - **Hành động:** Viết các hàm CRUD (Create, Read, Update, Delete) cho bảng này để chuẩn bị cho luồng tự động trích xuất thông tin (Thought Loop) của Agent.

2. **Khôi phục Sổ đăng ký MCP Tools:**
   - **Hành động:** Thêm `CREATE TABLE mcp_tools` vào CSDL. Bảng này sẽ lưu trữ tên công cụ, schema (định dạng đầu vào/đầu ra), phiên bản, và phân quyền (RBAC).
   - **Hành động:** Sửa đổi `McpToolRegistry` (`src/infrastructure/mcp/registry.rs`) để đọc/ghi từ SQLite thay vì dùng `RwLock<HashMap>` trên RAM (hiện tại tắt app là mất dữ liệu).

---

## Giai đoạn 2: Xây dựng Agent Core Loop & Tích hợp Function Calling
**Mục tiêu:** Cấp cho Agent khả năng tự suy nghĩ (Reason) và sử dụng công cụ (Act) thay vì chỉ Hỏi-Đáp một cách thụ động.

1. **Đập bỏ luồng gọi LLM trực tiếp (Request-Response):**
   - **Hành động:** Refactor file `src/ui/panels/team_workspace/chat.rs`. Thay thế việc gọi trực tiếp `adapter.send_message_stream` bằng một lớp `AgentExecutor` (trung gian).
   - **Hành động:** Áp dụng kiến trúc **ReAct** (Reason + Act) hoặc **Plan-and-Execute** để khi người dùng giao việc, Agent sẽ tự động lập kế hoạch trước khi trả lời.

2. **Tích hợp Function Calling (Gọi Tool thực sự):**
   - **Hành động:** Ánh xạ các công cụ MCP, lệnh CLI, và WebSearch thành định dạng JSON Schema theo chuẩn của OpenAI/Claude (ví dụ: mảng `tools` trong API request).
   - **Hành động:** Cập nhật Adapter của LLM. Khi LLM trả về tín hiệu `tool_calls` (muốn dùng công cụ), `AgentExecutor` sẽ chặn lại, thực thi lệnh đó trong môi trường Sandbox/Terminal ảo, và gửi kết quả (`tool_result`) ngược lại cho LLM để nó suy nghĩ tiếp.

3. **Triển khai WebSearch và CLI thực tế:**
   - **Hành động:** Xóa bỏ hàm `timer` giả lập (mock delay) trong `src/ui/panels/research_notebook.rs`.
   - **Hành động:** Tích hợp API thật (ví dụ: DuckDuckGo, Tavily, hoặc Google Custom Search) vào làm một Tool chuẩn để Agent có thể gọi khi cần xác minh kiến thức từ Internet.

---

## Giai đoạn 3: Quản lý Context & Bộ nhớ thông minh (Memory Management)
**Mục tiêu:** Khắc phục triệt để lỗi "quên", "loạn ngữ cảnh" và tối ưu hóa Token khi hội thoại kéo dài.

1. **Context Pruning (Cắt tỉa ngữ cảnh):**
   - **Hành động:** Ngừng việc `clone()` toàn bộ lịch sử tin nhắn.
   - **Hành động:** Cài đặt thuật toán "Sliding Window" (Cửa sổ trượt). Chỉ giữ lại System Prompt, K tin nhắn gần nhất, và các thông tin liên quan, loại bỏ các tin nhắn rác hoặc đã lỗi thời ở giữa cuộc trò chuyện trước khi gửi cho LLM.

2. **Tóm tắt tự động (Summarization) & Cache Memory:**
   - **Hành động:** Khi số lượng Token của phiên làm việc vượt mức cảnh báo (ví dụ: >8000 tokens), kích hoạt một LLM chạy ngầm (Background Task) để đọc toàn bộ hội thoại cũ và tóm tắt thành một đoạn văn ngắn (`Working Memory Summary`).
   - **Hành động:** Chèn đoạn tóm tắt này vào đầu System Prompt cho các vòng chat tiếp theo để tiết kiệm chi phí và giữ LLM tập trung.

3. **Thought Extraction (Trích xuất Kiến thức Dài hạn):**
   - **Hành động:** Sau mỗi khi một Agent hoàn thành một tác vụ phức tạp, cấu hình hệ thống tự động gọi một Tool nội bộ tên là `save_to_knowledge`.
   - **Hành động:** Tool này sẽ ghi nhận lại các quyết định quan trọng, công thức chuẩn, hoặc kết quả nghiên cứu vào bảng `knowledge_entries` (từ Giai đoạn 1) để dùng lại cho các phiên sau.

---

## Giai đoạn 4: Phối hợp Team thông minh & Shared Task List
**Mục tiêu:** Đưa hệ thống thực sự trở thành Multi-Agent, nơi các Agent tự phân chia, phối hợp và giao tiếp với nhau mà không cần kịch bản hardcode.

1. **Kích hoạt SharedTaskList (Hàng đợi công việc chung):**
   - **Hành động:** Khi nhận yêu cầu phức tạp từ người dùng, Agent Đội trưởng (Coordinator) sẽ gọi tool `create_subtasks` để tự động bóc tách yêu cầu thành các tác vụ nhỏ và `INSERT` vào bảng `tasks` trong SQLite.

2. **Cơ chế Atomic Claim (Nhận việc độc quyền):**
   - **Hành động:** Thay vì dùng vòng lặp `for` để ép các Agent nói lần lượt ("Debate Mode"), hãy lập trình cho các Agent trong team (như Dev, Tester, BA) tự động thăm dò (poll) bảng `tasks`.
   - **Hành động:** Agent nào rảnh rỗi và đúng chuyên môn sẽ "claim" (khóa) task đó lại, thực hiện xong thì cập nhật trạng thái thành `completed` và nộp lại kết quả.

3. **Kết nối TeamBus Router (Giao tiếp chéo):**
   - **Hành động:** Đưa `TeamBusRouter` vào hoạt động thực tế. Khi một Agent hoàn thành xong công việc của mình, nó sẽ gửi tín hiệu `Broadcast` qua TeamBus để đánh thức các Agent khác ở các Team khác (hoặc cùng Team) tiếp nhận kết quả đầu vào và bắt đầu vòng lặp công việc tiếp theo.