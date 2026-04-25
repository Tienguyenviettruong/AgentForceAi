# Báo cáo Phân tích và Đánh giá Mã nguồn Thực tế Hệ thống AgentForge (Nhánh `agentforce`)

*Báo cáo này được thực hiện dựa trên việc đọc, phân tích và đối chiếu trực tiếp mã nguồn Rust (source code) hiện tại trên nhánh `agentforce`, không dựa trên các tài liệu thiết kế lý thuyết.*

---

## 1. Logic, Khả năng làm việc và Phối hợp giữa 2 Team
- **Thực trạng mã nguồn:** Mặc dù có các file như `TeamBusRouter` và `SharedTaskList`, nhưng trong file xử lý luồng chat chính (`src/ui/panels/team_workspace/chat.rs`), **sự phối hợp tự động giữa các Agent không hề tồn tại**.
- **Cách hoạt động hiện tại:** Hệ thống thực chất đang hoạt động như một "phòng chat nhóm" thông thường. Khi người dùng (hoặc hệ thống) chỉ định một Agent trả lời, mã nguồn chỉ đơn giản lấy toàn bộ lịch sử tin nhắn (`current_history`), nối thêm System Prompt của Agent đó và gọi API của LLM (Claude/OpenRouter) thông qua hàm `adapter.send_message_stream(full_history).await`.
- **Đánh giá:** Không có vòng lặp cộng tác (Multi-agent collaboration loop). Không có "Coordinator" (Người điều phối) tự động chia việc cho team khác. Các Team hiện tại hoàn toàn độc lập và không thể giao tiếp chéo với nhau một cách tự động.

## 2. Quản lý Bộ nhớ, Cache, Chọn lọc thông minh và Context
- **Nguyên nhân gây "quên" và "loạn ngữ cảnh":**
  - Trong mã nguồn (`chat.rs` dòng ~1238 đến 1300), biến `full_history` được tạo ra bằng cách `clone()` trực tiếp toàn bộ lịch sử hội thoại hiện tại.
  - **Không hề có code xử lý Summarization (Tóm tắt) hay Context Pruning (Cắt tỉa ngữ cảnh).** Khi cuộc hội thoại dài ra, toàn bộ tin nhắn cũ bị nhồi nhét liên tục vào API của LLM.
  - Khi vượt quá giới hạn Token của model, hoặc khi có quá nhiều thông tin rác, model sẽ tự động bị "loạn" và không nhớ được trọng tâm.
  - **Không có Caching:** Hệ thống không có bất kỳ cơ chế cache memory nào cho hội thoại. Mỗi lần gọi LLM là một lần gửi lại toàn bộ chuỗi text từ đầu.

## 3. Cách Suy nghĩ của Agent và Lưu trữ (Sự thật về bảng `knowledge_entries`)
- **Sự thật về Database:** Đã kiểm tra trực tiếp file khởi tạo CSDL `src/infrastructure/database/sqlite_adapter.rs`. **Các bảng `knowledge_entries` và `knowledge_entries_fts` KHÔNG HỀ TỒN TẠI trong mã nguồn.** Hệ thống chỉ có bảng `knowledge` và `knowledge_chunks`.
- **Cách suy nghĩ:** Agent không có quá trình "suy nghĩ" (Thought process) hay "lưu trữ chọn lọc". Nó hoạt động theo cơ chế **Request-Response (Hỏi-Đáp) thuần túy**. Nó không có khả năng tự nhận thức để trích xuất một thông tin quan trọng trong chat và tự động `INSERT` vào bảng kiến thức. 

## 4. Independent Research & Websearch (Tìm kiếm Web độc lập)
- **Thực trạng:** **Hoàn toàn là Giả lập (Mocked). Không có thật.**
- **Bằng chứng trong code:** Tại file `src/ui/panels/research_notebook.rs`, khi kích hoạt tính năng tìm kiếm ("Auto-Search via Agent"), mã nguồn thực thi như sau:
  ```rust
  // Mô phỏng thời gian chờ
  cx.background_executor().timer(std::time::Duration::from_secs(2)).await;
  
  // Trả về dữ liệu được code cứng (hardcoded)
  let real_results = vec![
      SearchResult {
          title: "Ultimate Guide to AI Agents (Real Data)".to_string(),
          url: "https://example.com/guide".to_string(),
          snippet: "Comprehensive overview from actual agent execution.".to_string(),
      }, ...
  ];
  ```
- **Đánh giá:** Không có API tìm kiếm nào (Google, Bing, Tavily) được kết nối. Tính năng chọn lọc tin tức thông minh hoàn toàn không tồn tại trong code.

## 5. Các công cụ bổ trợ (MCP, Chạy CLI, Websearch trong Chat)
- **Sự thật về MCP Tools:**
  - Bảng `mcp_tools` **KHÔNG TỒN TẠI** trong CSDL (`sqlite_adapter.rs`).
  - Trong `src/infrastructure/mcp/registry.rs`, danh sách công cụ MCP đang được lưu tạm trên RAM (bộ nhớ trong) bằng cấu trúc dữ liệu `RwLock<HashMap<String, McpTool>>`. Do đó, mỗi lần tắt app, mọi thiết lập về Tool sẽ biến mất.
  - Các công cụ MCP "Available" trên giao diện chưa được kết nối với luồng thực thi của Agent.
- **Khả năng chạy CLI và Websearch trực tiếp trong Chat:**
  - **Không chạy được.**
  - Khi xem file `src/application/services/chat_service.rs`, hệ thống không cấp quyền Function Calling (gọi hàm) cho LLM để thực thi lệnh Terminal (CLI) hay Websearch.
  - Việc "chạy" duy nhất mà code đang làm là dùng Regular Expression (Regex) để tìm các khối văn bản có dạng ````file:/path/to/file````, sau đó tự động trích xuất nội dung và ghi đè ra file vật lý trên máy. Không hề có môi trường Sandbox hay Terminal thực thụ chạy trong chat.

---
### TỔNG KẾT
Hiện tại, nhánh `agentforce` có một giao diện (UI) rất đẹp và các tài liệu thiết kế rất hoành tráng. Tuy nhiên, **logic lõi (Core Engine) bên dưới hầu hết đang bị làm giả (Mocked) hoặc triển khai ở mức độ sơ khai nhất (chỉ là Wrapper gọi API LLM bình thường).**

Để hệ thống hoạt động đúng như tài liệu (có bộ nhớ thông minh, tự động phối hợp team, tự tìm kiếm web, tự chạy CLI), dự án cần phải đập bỏ luồng gọi LLM trực tiếp hiện tại trong `chat.rs`, và xây dựng một **Agent Loop (Vòng lặp Agent)** chuẩn mực (như ReAct hoặc Plan-and-Execute) có tích hợp Function Calling thực sự.
