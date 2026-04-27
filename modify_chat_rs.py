import re

filepath = "/workspace/agentforge-ui/src/ui/panels/team_workspace/chat.rs"
with open(filepath, 'r') as f:
    content = f.read()

# We need to replace adapter.send_message_stream(...) with AgentExecutor.execute_task(...)
# Let's search for blocks around match adapter.send_message_stream(full_history).await {
# Since AgentExecutor executes the loop internally and returns a single String (Result<String>),
# we can simulate the "stream" by just emitting the whole text at once, or we can change the logic.
# Wait, the current UI expects a stream to update the UI progressively.
# AgentExecutor currently returns Result<String>. Let's look at what the UI does with stream:
# It pushes chunks to `full_text` and calls `cx.update(|cx| cx.notify())`.
# If we use AgentExecutor, we will just get the final String. The UI will update instantly (after waiting).
# Let's change AgentExecutor to just return Result<String> and we update UI.

# Alternatively, we can just replace the block.
# Let's find the exact blocks.
