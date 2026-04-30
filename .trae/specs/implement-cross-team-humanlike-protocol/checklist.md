# Checklist

- [x] Handoff Package v2 được parse được cả payload cũ và payload mới (có `context`)
- [x] Status events được phát và persist theo `correlation_id` (ACK, Readback, Plan, InProgress, Review, Done)
- [x] Case timeline được persist và reload UI vẫn hiển thị đúng step hiện tại
- [x] UI Stepper hiển thị đúng mapping event → step và cập nhật theo event mới nhất
- [x] Knowledge tree click và graph double-click render markdown nhất quán
- [x] Knowledge markdown có selectable/copy hoạt động
- [x] Wikilinks `[[...]]` và tag `#tag` được render đúng trong Knowledge và không ảnh hưởng Chat
- [x] Codeblock hiển thị rõ lang label (tối thiểu) và không phá layout hiện tại
