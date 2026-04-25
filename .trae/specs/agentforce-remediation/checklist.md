# AgentForge Core Engine Remediation - Verification Checklist

## Database Schema Verification
- [ ] Checkpoint 1: Verify `knowledge_entries` table is created with proper schema
- [ ] Checkpoint 2: Verify `knowledge_entries_fts` virtual table is created
- [ ] Checkpoint 3: Verify `mcp_tools` table is created with proper schema
- [ ] Checkpoint 4: Verify CRUD operations work for all new tables

## Agent Core Loop Verification
- [ ] Checkpoint 5: Verify AgentExecutor implements ReAct pattern correctly
- [ ] Checkpoint 6: Verify LLM adapters support function calling
- [ ] Checkpoint 7: Verify agent can call tools and use results
- [ ] Checkpoint 8: Verify reasoning steps are included in agent responses

## Memory Management Verification
- [ ] Checkpoint 9: Verify sliding window works for conversation history
- [ ] Checkpoint 10: Verify summarization occurs for long conversations
- [ ] Checkpoint 11: Verify important information is stored in long-term memory
- [ ] Checkpoint 12: Verify agents can retrieve information from long-term memory

## Websearch Integration Verification
- [ ] Checkpoint 13: Verify websearch returns real results from search engine API
- [ ] Checkpoint 14: Verify agents can use search results in their responses
- [ ] Checkpoint 15: Verify research_notebook.rs uses real search instead of mock data

## Multi-Agent Collaboration Verification
- [ ] Checkpoint 16: Verify SharedTaskList functionality works with tasks table
- [ ] Checkpoint 17: Verify agents can poll and claim tasks
- [ ] Checkpoint 18: Verify TeamBusRouter enables inter-agent communication
- [ ] Checkpoint 19: Verify agents can split and collaborate on complex tasks

## CLI Execution Verification
- [ ] Checkpoint 20: Verify CLI commands execute in sandboxed environment
- [ ] Checkpoint 21: Verify agents can use CLI results in their responses
- [ ] Checkpoint 22: Verify security of CLI execution environment

## MCP Tool Registry Verification
- [ ] Checkpoint 23: Verify McpToolRegistry reads/writes to mcp_tools table
- [ ] Checkpoint 24: Verify tools persist across application restarts
- [ ] Checkpoint 25: Verify tool registration and discovery works correctly

## Chat Interface Integration Verification
- [ ] Checkpoint 26: Verify chat.rs uses AgentExecutor instead of direct LLM calls
- [ ] Checkpoint 27: Verify proper context management in chat interface
- [ ] Checkpoint 28: Verify UI elements for tool usage and memory management

## System Performance Verification
- [ ] Checkpoint 29: Verify system handles long conversations without issues
- [ ] Checkpoint 30: Verify multi-agent collaboration works correctly
- [ ] Checkpoint 31: Verify overall system performance and usability
- [ ] Checkpoint 32: Verify all acceptance criteria are met