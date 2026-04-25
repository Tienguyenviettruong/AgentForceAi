# AgentForge Core Engine Remediation - Implementation Plan

## [ ] Task 1: Update Database Schema with Missing Tables
- **Priority**: P0
- **Depends On**: None
- **Description**: 
  - Update `sqlite_adapter.rs` to add missing tables: `knowledge_entries`, `knowledge_entries_fts`, and `mcp_tools`
  - Implement CRUD operations for these new tables in the `DatabasePort` trait
- **Acceptance Criteria Addressed**: AC-1
- **Test Requirements**:
  - `programmatic` TR-1.1: Verify all new tables are created with proper schemas
  - `programmatic` TR-1.2: Verify CRUD operations work correctly for all new tables
- **Notes**: Follow the schema definitions provided in the technical spec

## [ ] Task 2: Implement AgentExecutor with ReAct Pattern
- **Priority**: P0
- **Depends On**: Task 1
- **Description**: 
  - Create a new `AgentExecutor` struct that implements the ReAct pattern
  - Extend LLM adapters to support function calling
  - Integrate the executor into the chat flow
- **Acceptance Criteria Addressed**: AC-2
- **Test Requirements**:
  - `programmatic` TR-2.1: Verify agent can call tools and use results
  - `programmatic` TR-2.2: Verify ReAct pattern is followed with reasoning steps
- **Notes**: This will replace the current direct LLM call in chat.rs

## [ ] Task 3: Implement Memory Management System
- **Priority**: P0
- **Depends On**: Task 2
- **Description**: 
  - Implement sliding window for conversation history
  - Create background summarization job for long conversations
  - Implement long-term memory using knowledge_entries table
- **Acceptance Criteria Addressed**: AC-3
- **Test Requirements**:
  - `programmatic` TR-3.1: Verify sliding window works correctly
  - `programmatic` TR-3.2: Verify summarization happens for long conversations
  - `programmatic` TR-3.3: Verify important information is stored in long-term memory
- **Notes**: Use token counting to determine when to summarize

## [ ] Task 4: Implement Real Websearch Integration
- **Priority**: P1
- **Depends On**: Task 2
- **Description**: 
  - Integrate with a real search engine API (Tavily recommended)
  - Implement websearch tool for agents
  - Update research_notebook.rs to use real search instead of mock data
- **Acceptance Criteria Addressed**: AC-4
- **Test Requirements**:
  - `programmatic` TR-4.1: Verify websearch returns real results
  - `programmatic` TR-4.2: Verify agents can use search results in their responses
- **Notes**: Will require API key configuration

## [ ] Task 5: Implement Multi-Agent Collaboration
- **Priority**: P1
- **Depends On**: Task 2
- **Description**: 
  - Implement SharedTaskList functionality using the tasks table
  - Create worker loops for agents to poll and claim tasks
  - Integrate TeamBusRouter for inter-agent communication
- **Acceptance Criteria Addressed**: AC-5
- **Test Requirements**:
  - `human-judgment` TR-5.1: Verify agents can split and collaborate on complex tasks
  - `programmatic` TR-5.2: Verify task assignment and completion workflow
- **Notes**: Replace the current hardcoded Debate Mode with real collaboration

## [ ] Task 6: Implement CLI Execution Capability
- **Priority**: P1
- **Depends On**: Task 2
- **Description**: 
  - Create a sandboxed environment for CLI execution
  - Implement CLI tool for agents
  - Integrate CLI execution into the agent workflow
- **Acceptance Criteria Addressed**: AC-6
- **Test Requirements**:
  - `programmatic` TR-6.1: Verify CLI commands execute in sandbox
  - `programmatic` TR-6.2: Verify agents can use CLI results in responses
- **Notes**: Security is critical - ensure proper sandboxing

## [ ] Task 7: Update McpToolRegistry to Use Database
- **Priority**: P2
- **Depends On**: Task 1
- **Description**: 
  - Modify McpToolRegistry to read/write to the mcp_tools table instead of in-memory HashMap
  - Implement tool discovery and registration from the database
- **Acceptance Criteria Addressed**: AC-1, AC-2
- **Test Requirements**:
  - `programmatic` TR-7.1: Verify tools persist across application restarts
  - `programmatic` TR-7.2: Verify tool registration and discovery works correctly
- **Notes**: This ensures MCP tools are not lost when the application restarts

## [ ] Task 8: Create save_to_knowledge Tool
- **Priority**: P2
- **Depends On**: Task 1, Task 3
- **Description**: 
  - Create a built-in tool for agents to save important information to long-term memory
  - Train agents to use this tool appropriately
- **Acceptance Criteria Addressed**: AC-3
- **Test Requirements**:
  - `programmatic` TR-8.1: Verify tool saves information to knowledge_entries table
  - `programmatic` TR-8.2: Verify agents can retrieve saved information
- **Notes**: This will help agents build their long-term memory

## [ ] Task 9: Update Chat Interface to Use New Core Engine
- **Priority**: P0
- **Depends On**: Tasks 2, 3, 5
- **Description**: 
  - Update chat.rs to use the new AgentExecutor instead of direct LLM calls
  - Implement proper context management and memory usage
  - Add UI elements for tool usage and memory management
- **Acceptance Criteria Addressed**: AC-2, AC-3, AC-5
- **Test Requirements**:
  - `human-judgment` TR-9.1: Verify chat interface works smoothly with new engine
  - `programmatic` TR-9.2: Verify memory management works correctly in UI
- **Notes**: This is the final integration task that brings all components together

## [ ] Task 10: Testing and Performance Optimization
- **Priority**: P2
- **Depends On**: All previous tasks
- **Description**: 
  - Test the complete system with various scenarios
  - Optimize performance and memory usage
  - Fix any bugs or issues found during testing
- **Acceptance Criteria Addressed**: All ACs
- **Test Requirements**:
  - `programmatic` TR-10.1: Verify system handles long conversations without issues
  - `programmatic` TR-10.2: Verify multi-agent collaboration works correctly
  - `human-judgment` TR-10.3: Verify overall system performance and usability
- **Notes**: This task ensures the system is ready for production use