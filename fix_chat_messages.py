import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Find ChatMessage { ... }
    # Since there could be multiline, we need to be careful.
    # regex to find: ChatMessage { role: ..., content: ..., agent_name: ... }
    pattern = re.compile(r'ChatMessage\s*\{\s*role:\s*([^,]+),\s*content:\s*([^,]+),\s*agent_name:\s*([^}]+)\s*\}')
    
    def repl(m):
        role = m.group(1).strip()
        content = m.group(2).strip()
        agent = m.group(3).strip()
        return f'ChatMessage {{ role: {role}, content: {content}, agent_name: {agent}, tool_calls: None, tool_call_id: None }}'
    
    new_content = pattern.sub(repl, content)

    # For ChatResponse, add tool_calls: Vec::new()
    resp_pattern = re.compile(r'ChatResponse\s*\{\s*content:\s*([^,]+),\s*token_usage:\s*([^}]+)\s*\}')
    def repl_resp(m):
        content = m.group(1).strip()
        token = m.group(2).strip()
        return f'ChatResponse {{ content: {content}, token_usage: {token}, tool_calls: Vec::new() }}'

    new_content = resp_pattern.sub(repl_resp, new_content)

    if new_content != content:
        with open(filepath, 'w') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

for root, _, files in os.walk('src'):
    for file in files:
        if file.endswith('.rs'):
            process_file(os.path.join(root, file))
