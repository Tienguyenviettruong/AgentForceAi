import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # fix double comma ,, -> ,
    content = content.replace(",,", ",")
    
    # E0063
    content = content.replace("agent_name: Some(agent.name.clone().into()), tool_calls: None, tool_call_id: None, tool_calls: None, tool_call_id: None }", "agent_name: Some(agent.name.clone().into()), tool_calls: None, tool_call_id: None }")
    content = content.replace("agent_name: None, tool_calls: None, tool_call_id: None, tool_calls: None, tool_call_id: None }", "agent_name: None, tool_calls: None, tool_call_id: None }")

    content = content.replace("agent_name: Some(agent.name.clone().into()) }", "agent_name: Some(agent.name.clone().into()), tool_calls: None, tool_call_id: None }")
    content = content.replace("agent_name: None }", "agent_name: None, tool_calls: None, tool_call_id: None }")
    content = content.replace("agent_name }", "agent_name, tool_calls: None, tool_call_id: None }")

    content = content.replace("token_usage: TokenUsage::default(), tool_calls: Vec::new(), tool_calls: Vec::new() }", "token_usage: TokenUsage::default(), tool_calls: Vec::new() }")
    content = content.replace("token_usage: TokenUsage::default() }", "token_usage: TokenUsage::default(), tool_calls: Vec::new() }")
    content = content.replace("token_usage: crate::providers::TokenUsage::default() }", "token_usage: crate::providers::TokenUsage::default(), tool_calls: Vec::new() }")

    # fix openrouter.rs and iflow.rs
    content = content.replace("token_usage: TokenUsage::default(),\n                })", "token_usage: TokenUsage::default(),\n                    tool_calls: Vec::new(),\n                })")
    
    with open(filepath, 'w') as f:
        f.write(content)

for root, _, files in os.walk('src'):
    for file in files:
        if file.endswith('.rs'):
            process_file(os.path.join(root, file))
