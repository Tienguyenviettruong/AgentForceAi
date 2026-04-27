import re

filepath = "/workspace/agentforge-ui/src/ui/panels/agents.rs"
with open(filepath, 'r') as f:
    content = f.read()

# Replace the delete button to include an on_click
delete_button = """Button::new(gpui::SharedString::from(format!("delete-{}", agent.id))).ghost().icon(IconName::Delete)
                                            .on_click({
                                                let agent_id = agent.id.clone();
                                                cx.listener(move |this, _, _, cx| {
                                                    let db = crate::AppState::global(cx).db.clone();
                                                    let _ = db.delete_agent(&agent_id);
                                                    this.reload(cx);
                                                })
                                            })"""
                                            
content = content.replace("Button::new(gpui::SharedString::from(format!(\"delete-{}\", agent.id))).ghost().icon(IconName::Delete)", delete_button)

with open(filepath, 'w') as f:
    f.write(content)

