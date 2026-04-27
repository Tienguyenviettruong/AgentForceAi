import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    content = content.replace(",,", ",")

    # Fix ChatMessage { ... }
    # Using regex to find all ChatMessage initializations
    def repl_msg(m):
        inner = m.group(1)
        if "tool_calls" not in inner:
            if inner.endswith(","):
                return f"ChatMessage {{ {inner} tool_calls: None, tool_call_id: None }}"
            else:
                return f"ChatMessage {{ {inner}, tool_calls: None, tool_call_id: None }}"
        return m.group(0)

    content = re.sub(r'ChatMessage\s*\{\s*([^}]+?)\s*\}', repl_msg, content)

    # Fix ChatResponse { ... }
    def repl_resp(m):
        inner = m.group(1)
        if "tool_calls" not in inner:
            if inner.endswith(","):
                return f"ChatResponse {{ {inner} tool_calls: Vec::new() }}"
            else:
                return f"ChatResponse {{ {inner}, tool_calls: Vec::new() }}"
        return m.group(0)

    content = re.sub(r'ChatResponse\s*\{\s*([^}]+?)\s*\}', repl_resp, content)

    with open(filepath, 'w') as f:
        f.write(content)

for root, _, files in os.walk('src'):
    for file in files:
        if file.endswith('.rs'):
            process_file(os.path.join(root, file))
