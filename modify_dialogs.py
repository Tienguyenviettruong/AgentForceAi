import re

filepath = "/workspace/agentforge-ui/src/ui/components/dialogs.rs"
with open(filepath, 'r') as f:
    content = f.read()

content = content.replace("use gpui_component::{", "use gpui_component::{ActiveTheme as _, ")
content = content.replace("let theme = _cx.global::<gpui_component::Theme>();", "let theme = _cx.theme().clone();")

with open(filepath, 'w') as f:
    f.write(content)

