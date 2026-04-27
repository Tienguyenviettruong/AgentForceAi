import re

filepath = "/workspace/agentforge-ui/src/ui/panels/team_workspace/chat.rs"
with open(filepath, 'r') as f:
    content = f.read()

# Fix team_bus_clone field issue
content = content.replace("self.team_bus_clone.clone()", "self.team_bus.clone()")
content = content.replace("this.team_bus_clone.clone()", "this.team_bus.clone()")

with open(filepath, 'w') as f:
    f.write(content)

