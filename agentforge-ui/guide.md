# Hướng dẫn Tạo Agent (Agent Creation Guide)

Tài liệu này cung cấp một ví dụ mẫu và giải thích chi tiết cách điền thông tin để tạo một Agent chuyên nghiệp trong AgentForge. Hệ thống Multi-Agent của chúng ta giao tiếp dựa trên các **vai trò (Role)** và **cấu trúc lệnh (System Prompt)**, do đó việc điền đúng thông tin là rất quan trọng để các Agent phối hợp nhịp nhàng.

---

## Ví dụ Mẫu: Tạo "Backend Developer Agent"

Giả sử bạn muốn tạo một Agent chuyên viết mã nguồn Backend bằng ngôn ngữ Rust. Dưới đây là cách điền từng trường thông tin trong form **Create Agent**:

### 1. Name (Tên định danh)
- **Mục đích:** Tên hiển thị trên giao diện người dùng để bạn dễ quản lý và phân biệt.
- **Ví dụ:** `Senior Backend Developer`
- **Lưu ý:** Tên này có thể chứa khoảng trắng và các ký tự đặc biệt, nhưng nên ngắn gọn, súc tích.

### 2. Role / Position (Vai trò / Vị trí)
- **Mục đích:** Đây là trường **ĐẶC BIẾT QUAN TRỌNG**. Hệ thống `TeamBus Router` dùng trường này để định tuyến tin nhắn và chia việc (Sub-tasks). Nếu Team Leader giao việc cho `DEV`, những Agent có Role là `DEV` mới nhận được việc.
- **Ví dụ:** `DEV` hoặc `Backend`
- **Quy tắc:**
  - Không chứa khoảng trắng nếu không cần thiết. Nên dùng chữ hoa (VD: `DEV`, `QA`, `BA`, `PM`).
  - Người điều phối bắt buộc phải có Role là `Coordinator` (Nếu bạn tạo Agent đóng vai trò đội trưởng).

### 3. Agent Details (Chi tiết / Kỹ năng)
- **Mục đích:** Thông tin phụ trợ mô tả rõ hơn về công việc hoặc kỹ năng chuyên môn của Agent. Mặc dù không bắt buộc nhưng giúp ích cho người dùng khi xem danh sách.
- **Ví dụ:** `Chuyên gia lập trình Rust, tập trung vào hiệu năng, an toàn bộ nhớ và thiết kế API RESTful.`

### 4. Provider / Model (Mô hình AI)
- **Mục đích:** Chọn bộ não (LLM) sẽ cấp quyền suy nghĩ cho Agent này.
- **Ví dụ:** Chọn `OpenRouter / anthropic/claude-3.5-sonnet` (Vì viết code cần model thông minh).
- **Lưu ý:** Các Provider phải được cấu hình trước ở màn hình Settings (Provider config).

### 5. System Prompt (Chỉ dẫn hệ thống)
- **Mục đích:** "Linh hồn" của Agent. Nó quyết định cách Agent phản ứng, định dạng đầu ra, và các luật lệ mà Agent phải tuân thủ tuyệt đối.
- **Ví dụ:**
```text
Bạn là một Kỹ sư Backend cấp cao (Senior Backend Developer) chuyên lập trình Rust.
Vai trò của bạn là tiếp nhận các yêu cầu thiết kế hệ thống từ Coordinator hoặc BA, sau đó tiến hành phân tích và viết mã nguồn.

LUẬT LỆ:
1. Luôn tuân thủ các quy tắc an toàn bộ nhớ của Rust (Borrow Checker).
2. Khi viết mã nguồn, hãy giải thích rõ ràng kiến trúc và lý do bạn chọn các thư viện (crates).
3. Sau khi hoàn thành một tác vụ, nếu bạn phát hiện lỗi logic, hãy tự động sửa lỗi và báo cáo.
4. Mọi đoạn code phải được bọc trong block markdown (```rust).
```

---

## Các Vai Trò Khuyến Nghị (Recommended Roles)

Để một Team (đội nhóm) có thể tự động giao tiếp và tranh luận chéo, bạn nên thiết lập ít nhất 3 Agent với các Role sau:
1. **Coordinator** (Đội trưởng): Người giao tiếp với người dùng, lập kế hoạch (Plan) và phân công việc (Sub-tasks).
2. **DEV** (Lập trình viên): Người nhận yêu cầu và viết code.
3. **TESTER / QA** (Người kiểm thử): Người nhận code từ DEV, review, phản biện (Critique) và bắt lỗi.
4. **BA** (Phân tích nghiệp vụ): Người làm rõ yêu cầu trước khi DEV bắt tay vào làm.

Chúc bạn xây dựng được một đội nhóm AI xuất sắc!