import re

filepath = "/workspace/agentforge-ui/src/ui/components/dialogs.rs"
with open(filepath, 'r') as f:
    content = f.read()

parts = content.split("pub fn open_new_agent_dialog")
part1 = parts[0]
part2 = "pub fn open_new_agent_dialog" + parts[1]

# Fix footer closures to include role_input and details_input
part2 = part2.replace("""                let name_input = name_input.clone();
                let system_prompt_input = system_prompt_input.clone();
                let provider_select = provider_select.clone();""", """                let name_input = name_input.clone();
                let system_prompt_input = system_prompt_input.clone();
                let role_input = role_input.clone();
                let details_input = details_input.clone();
                let provider_select = provider_select.clone();""")

part2 = part2.replace("""                    let name_input2 = name_input.clone();
                    let system_prompt_input2 = system_prompt_input.clone();
                    let provider_select2 = provider_select.clone();""", """                    let name_input2 = name_input.clone();
                    let system_prompt_input2 = system_prompt_input.clone();
                    let role_input2 = role_input.clone();
                    let details_input2 = details_input.clone();
                    let provider_select2 = provider_select.clone();""")

part2 = part2.replace("""                                let name_input3 = name_input2.clone();
                                let system_prompt_input3 = system_prompt_input2.clone();
                                let provider_select3 = provider_select2.clone();""", """                                let name_input3 = name_input2.clone();
                                let system_prompt_input3 = system_prompt_input2.clone();
                                let role_input3 = role_input2.clone();
                                let details_input3 = details_input2.clone();
                                let provider_select3 = provider_select2.clone();""")

# Fix the config assignment inside the click handler
config_logic = """
                                    let role = role_input3.read(cx).text().to_string();
                                    let details = details_input3.read(cx).text().to_string();
                                    let config_json = serde_json::json!({
                                        "role": role,
                                        "details": details
                                    });
"""

part2 = part2.replace("""let system_prompt =
                                        system_prompt_input3.read(cx).text().to_string();
                                    let agent = Agent {""", config_logic + """let system_prompt =
                                        system_prompt_input3.read(cx).text().to_string();
                                    let agent = Agent {""")

part2 = part2.replace("config: None,", "config: Some(config_json.to_string()),")

with open(filepath, 'w') as f:
    f.write(part1 + part2)

