import re

filepath = "/workspace/agentforge-ui/src/ui/components/dialogs.rs"
with open(filepath, 'r') as f:
    content = f.read()

# Replace vec![i].into() with gpui_component::IndexPath::new(i)
content = content.replace("vec![i].into()", "gpui_component::IndexPath::new(i)")

with open(filepath, 'w') as f:
    f.write(content)

