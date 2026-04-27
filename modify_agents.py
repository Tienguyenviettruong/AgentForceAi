import re

filepath = "/workspace/agentforge-ui/src/ui/panels/agents.rs"
with open(filepath, 'r') as f:
    content = f.read()

# Replace `cx.view().clone()` with `cx.entity().clone()` inside the edit_button
content = content.replace("let view = cx.view().clone();", "let view = cx.entity().clone();")

with open(filepath, 'w') as f:
    f.write(content)

