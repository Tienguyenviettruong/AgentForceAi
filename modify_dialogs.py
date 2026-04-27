import re

filepath = "/workspace/agentforge-ui/src/ui/components/dialogs.rs"
with open(filepath, 'r') as f:
    content = f.read()

# We only want to replace inside `open_new_agent_dialog`
parts = content.split("pub fn open_new_agent_dialog")
part1 = parts[0]
part2 = "pub fn open_new_agent_dialog" + parts[1]

# Add theme import
part2 = part2.replace("dialog\n            .title(\"Create Agent\")", "let theme = _cx.global::<gpui_component::Theme>();\n        dialog\n            .title(\"Create Agent\")")

# Replace field blocks
name_field = """field()
                            .label("Name")
                            .required(true)
                            .child(Input::new(&name_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Tên định danh của Agent (VD: Backend Developer, Data Analyst)."))"""
                            
role_field = """field()
                            .label("Role / Position")
                            .child(Input::new(&role_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Vai trò chuyên môn trong dự án (VD: DEV, QA, PM). Dùng cho TeamBus Router."))"""
                            
details_field = """field()
                            .label("Agent Details")
                            .child(Input::new(&details_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Thông tin bổ sung, kỹ năng hoặc giới hạn của Agent."))"""

provider_field = """field().label("Provider / Model").child(
                        Select::new(&provider_select).placeholder("Select Provider / Model"),
                    )
                    .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Mô hình LLM sẽ cung cấp năng lực cho Agent này (VD: Claude-3-Opus, GPT-4o)."))"""

system_prompt_field = """field()
                            .label("System Prompt")
                            .child(Input::new(&system_prompt_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Chỉ dẫn hệ thống cốt lõi để định hình hành vi, phong cách trả lời và luồng suy nghĩ của Agent."))"""

part2 = part2.replace("field()\n                            .label(\"Name\")\n                            .required(true)\n                            .child(Input::new(&name_input))", name_field)
part2 = part2.replace("field()\n                            .label(\"Role / Position\")\n                            .child(Input::new(&role_input))", role_field)
part2 = part2.replace("field()\n                            .label(\"Agent Details\")\n                            .child(Input::new(&details_input))", details_field)
part2 = part2.replace("field().label(\"Provider / Model\").child(\n                        Select::new(&provider_select).placeholder(\"Select Provider / Model\"),\n                    )", provider_field)
part2 = part2.replace("field()\n                            .label(\"System Prompt\")\n                            .child(Input::new(&system_prompt_input))", system_prompt_field)

with open(filepath, 'w') as f:
    f.write(part1 + part2)

