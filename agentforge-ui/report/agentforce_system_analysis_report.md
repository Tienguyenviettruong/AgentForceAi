# Báo cáo Phân tích và Đánh giá Hệ thống AgentForge (Nhánh `agentforce`)

Dựa trên việc đối chiếu giữa tài liệu thiết kế (PRD, SRS, Architecture) và thực trạng mã nguồn hiện tại của nhánh `agentforce`, dưới đây là báo cáo đánh giá chi tiết về logic hệ thống, khả năng phối hợp, quản lý bộ nhớ, và các công cụ bổ trợ.

## 1. Logic, Khả năng làm việc và Phối hợp giữa 2 Team
- **Thiết kế vs Thực tế:** Về mặt thiết kế kiến trúc, hệ thống có `TeamBus` (định tuyến tin nhắn P2P: Direct, Broadcast, Role Group) và `SharedTaskList` (quản lý công việc chung qua SQLite). Tuy nhiên, **thực tế trong mã nguồn hiện tại**, các thành phần này chưa được kết nối hoàn chỉnh vào giao diện và luồng thực thi.
- **Vấn đề "Đường ray song song" (Parallel Rails):** Theo phân tích từ các file như `improvement_plan.md`, UI của Team Workspace đang hoạt động độc lập, gọi trực tiếp đến LLM Provider và lưu vào bảng `messages` riêng lẻ. Nó bỏ qua hoàn toàn `TeamBus` và bộ điều phối trung tâm (Coordinator).
- **Kết luận:** Khả năng phối hợp thực tế giữa 2 team hoặc giữa các Agent trong cùng một team hiện tại chưa hoạt động. Các Agent không thực sự "nhìn thấy" công việc của nhau trong một vòng lặp cộng tác (Multi-agent collaboration loop).

## 2. Quản lý Bộ nhớ, Context, Caching và Lọc thông minh (Nguyên nhân gây "quên" và "loạn")
- **Hiện trạng Context:** Việc bộ nhớ của Agent không nhớ được thông tin từ hội thoại và context bị loạn phản ánh đúng thực trạng mã nguồn hiện tại.
- **Nguyên nhân:**
  - **Quản lý Phiên (Session Recovery) bị lỗi:** Logic khôi phục phiên làm việc đang bị đứt gãy. Dữ liệu chat được lưu tạm bợ và khi tải lại, ngữ cảnh không được nạp đầy đủ hoặc nạp sai thứ tự.
  - **Thiếu Caching và Smart Filtering:** Mặc dù có logic chèn ngữ cảnh (RAG) vào prompt (`[SYSTEM KNOWLEDGE RETRIEVAL]`), nhưng luồng quản lý bộ nhớ dài hạn (Long-term memory) và cắt tỉa ngữ cảnh (Context Pruning) chưa hoàn thiện. Hệ thống đang nhồi nhét quá nhiều hoặc cắt bỏ sai thông tin khiến LLM bị mất ngữ cảnh quan trọng hoặc quên thông tin cũ.

## 3. Cách suy nghĩ, lưu trữ và chọn lọc của Agent (Vấn đề bảng `knowledge_entries`)
- **Về Database:** Bảng `knowledge_entries` và bảng tìm kiếm toàn văn bản `knowledge_entries_fts` **thực sự tồn tại** trong schema của `sqlite_adapter.rs`. Nó không bị thiếu ở mức cơ sở dữ liệu.
- **Về Thực tế sử dụng:** Mã nguồn hiện tại **chưa sử dụng bảng này cho luồng suy nghĩ thực tế của Agent**. Giao diện Knowledge (Knowledge panel) chủ yếu đang hiển thị dữ liệu giả lập (Mock UI) hoặc chưa kết nối trực tiếp với Database.
- **Cách suy nghĩ:** Agent hiện tại hoạt động theo dạng phản hồi trực tiếp (Request-Response) thay vì có một vòng lặp suy nghĩ độc lập (Thought Loop) tự động trích xuất thông tin quan trọng để lưu vào `knowledge_entries`. Do đó, thông tin chỉ nằm trong lịch sử chat thay vì trở thành "kiến thức" có thể tìm kiếm độc lập.

## 4. Independent Research & Websearch (Tìm kiếm Web và Chọn lọc tin tức)
- **Thực tế:** Tính năng này **chưa được triển khai thực tế**.
- **Mã nguồn:** Trong file `src/ui/panels/research_notebook.rs`, khi kích hoạt tính năng "Auto-Search via Agent", mã nguồn chỉ sử dụng hàm `timer` để giả lập thời gian chờ (khoảng 2 giây), sau đó trả về một mảng dữ liệu được code cứng (hardcoded) như *"Ultimate Guide to AI Agents (Real Data)"*.
- **Kết luận:** Hệ thống hiện tại không gọi bất kỳ API tìm kiếm thực tế nào (như Google, Bing, Tavily), không có khả năng đọc web thực sự, và do đó không thể chọn lọc thông minh tin tức như thiết kế.

## 5. Hệ thống Công cụ (MCP, Run CLI, Websearch trong Chat)
- **Tình trạng MCP và DB:** Bảng `mcp_tools` đã được tạo trong SQLite để lưu trữ thông tin công cụ. Tuy nhiên, luồng thực thi (Tool Loop) để LLM chủ động gọi MCP Tool, chờ kết quả từ DB/Sandbox rồi suy nghĩ tiếp **chưa được kết nối (not connected)**. Giao diện báo "Available" nhưng lõi bên dưới chưa xử lý logic gọi tool.
- **Khả năng chạy CLI và Websearch trực tiếp trong Chat:**
  - **Chưa chạy được thực sự.**
  - Thay vì cung cấp công cụ (Function Calling / Tool Use) cho LLM để nó tự chạy lệnh CLI hoặc tìm kiếm web và nhận lại kết quả, hệ thống hiện tại đang dùng một cơ chế Parser thô sơ: LLM được prompt để trả về các khối Markdown đặc biệt (ví dụ: ````file:/path/to/file````). Sau đó, ứng dụng dùng Regex (Biểu thức chính quy) để bóc tách nội dung và tự động ghi đè ra file (thông qua `chat_service.parse_and_write_files`).
  - Không có môi trường Sandbox thực thi lệnh CLI trực tiếp trả kết quả về cho LLM ngay trong lúc chat.

## Tổng kết Đánh giá
**AgentForge** đang sở hữu một bộ khung kiến trúc (Architecture) và Thiết kế Cơ sở dữ liệu (Database Schema) rất đồ sộ, chuẩn mực (sẵn sàng cho RAG, MCP, Multi-Agent Routing).
Tuy nhiên, **logic cốt lõi nối giữa giao diện UI và Core Engine đang bị đứt gãy hoặc được làm giả (Mocked) rất nhiều**. Đây chính là lý do khiến hệ thống bị "loạn ngữ cảnh", "quên trí nhớ", không thể chạy Websearch/CLI thật, và MCP báo Available nhưng không thể kết nối. Hệ thống cần được "đi dây" (wiring) lại phần Core Engine để chuyển từ UI-Driven sang Engine-Driven đúng như tài liệu đã thiết kế.
