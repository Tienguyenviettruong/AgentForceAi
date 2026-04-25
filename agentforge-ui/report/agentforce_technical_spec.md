# Đặc Tả Kỹ Thuật (Technical Specification) - AgentForge Core Engine

Tài liệu này định nghĩa chi tiết các cấu trúc dữ liệu, schema và logic sẽ được triển khai theo `agentforce_implementation_plan.md`.

## 1. Database Schema Updates (`src/infrastructure/database/sqlite_adapter.rs`)

### 1.1 Bảng `mcp_tools`
Bảng này thay thế cho bộ nhớ RAM hiện tại của `McpToolRegistry`.
```sql
CREATE TABLE IF NOT EXISTS mcp_tools (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    version TEXT NOT NULL,
    command TEXT NOT NULL, -- Đường dẫn đến file chạy (node, python, v.v.)
    args TEXT NOT NULL, -- JSON Array các arguments truyền vào
    input_schema TEXT NOT NULL, -- JSON Schema chuẩn OpenAI/Claude cho arguments
    is_active BOOLEAN DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 1.2 Bảng `knowledge_entries` (Long-term Memory)
```sql
CREATE TABLE IF NOT EXISTS knowledge_entries (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT, -- JSON Array
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(agent_id) REFERENCES agents(id)
);

-- Bảng ảo FTS5 phục vụ tìm kiếm Vector/Semantic
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_entries_fts 
USING fts5(title, content, tags, content='knowledge_entries', content_rowid='id');
```

## 2. Agent Core Loop (ReAct / Function Calling)

### 2.1 Cấu trúc `AgentExecutor`
`AgentExecutor` sẽ nằm giữa UI (`chat.rs`) và `LlmProviderPort`.
```rust
pub struct AgentExecutor {
    provider: Arc<dyn LlmProviderPort>,
    mcp_registry: Arc<McpToolRegistry>,
    db: Arc<dyn DatabasePort>,
}

impl AgentExecutor {
    /// Vòng lặp chính của Agent
    pub async fn execute_task(&self, mut history: Vec<ChatMessage>) -> Result<String> {
        let mut iteration = 0;
        let max_iterations = 5; // Tránh loop vô hạn

        while iteration < max_iterations {
            // 1. Lấy response từ LLM (hỗ trợ tools)
            let response = self.provider.send_message_with_tools(history.clone(), self.get_available_tools()).await?;

            // 2. Nếu LLM trả về text thường -> Hoàn thành
            if response.tool_calls.is_empty() {
                return Ok(response.content);
            }

            // 3. Nếu LLM muốn gọi Tool (Function Calling)
            for tool_call in response.tool_calls {
                // Thực thi lệnh thật trong Sandbox/CLI hoặc WebSearch
                let result = self.execute_tool(&tool_call.name, &tool_call.arguments).await;
                
                // 4. Nhồi kết quả lại vào history dưới dạng role="tool"
                history.push(ChatMessage {
                    role: "tool".into(),
                    content: result.into(),
                    name: Some(tool_call.name),
                });
            }
            iteration += 1;
        }
        Ok("Max tool iterations reached".to_string())
    }
}
```

## 3. Quản lý Bộ nhớ (Context & Summarization)

### 3.1 Cắt tỉa ngữ cảnh (Sliding Window)
Trong `chat.rs`, thay vì `current_history.clone()`, áp dụng logic:
```rust
fn prune_history(history: &[ChatMessage], max_messages: usize) -> Vec<ChatMessage> {
    let mut pruned = Vec::new();
    // Luôn giữ System Prompt (Index 0)
    if let Some(sys) = history.first().filter(|m| m.role == "system") {
        pruned.push(sys.clone());
    }
    // Lấy K tin nhắn cuối cùng
    let tail_start = history.len().saturating_sub(max_messages);
    let start_idx = std::cmp::max(1, tail_start); // Bỏ qua index 0
    
    pruned.extend_from_slice(&history[start_idx..]);
    pruned
}
```

### 3.2 Tool nội bộ: `save_to_knowledge`
Định nghĩa một tool bắt buộc truyền vào mọi request của Agent:
```json
{
  "name": "save_to_knowledge",
  "description": "Lưu các quyết định quan trọng, bài học hoặc kết quả nghiên cứu vào bộ nhớ dài hạn để dùng cho sau này.",
  "parameters": {
    "type": "object",
    "properties": {
      "title": { "type": "string" },
      "content": { "type": "string" },
      "tags": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["title", "content"]
  }
}
```

## 4. Multi-Agent Collaboration (Shared Task List)

### 4.1 Luồng hoạt động (Workflow)
Thay vì vòng lặp `for` (Debate Mode), hệ thống sẽ chạy theo Event-Driven:
1. **User Input:** "Xây dựng website bán hàng".
2. **Coordinator Agent:** Được trigger. Nó phân tích và gọi tool `create_subtasks` sinh ra 2 task vào DB (`task_1: Thiết kế DB` cho BE, `task_2: Vẽ UI` cho FE).
3. **Background Polling:** Các Agent (BE, FE) chạy hàm `poll_tasks()` mỗi 2 giây.
4. **Claim Task:** Agent BE thấy `task_1` phù hợp. Thực hiện lệnh `UPDATE tasks SET status='in_progress', assignee='BE' WHERE id='task_1' AND status='pending'`.
5. **Execute & Broadcast:** BE chạy qua `AgentExecutor`. Xong việc, gọi hàm `team_bus.broadcast("Task 1 done. Kết quả: ...")`.
6. Agent FE nghe được event từ TeamBus, nhận thấy `task_1` xong nên bắt đầu làm `task_2`.