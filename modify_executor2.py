import re

filepath = "/workspace/agentforge-ui/src/application/orchestration/executor.rs"
with open(filepath, 'r') as f:
    content = f.read()

content = content.replace("""                tool_calls: None,
                tool_call_id: None,""", "")

with open(filepath, 'w') as f:
    f.write(content)
