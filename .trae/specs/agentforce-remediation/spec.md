# AgentForge Core Engine Remediation - Product Requirement Document

## Overview
- **Summary**: This project aims to remediate the core engine of AgentForge, transforming it from a mock UI with basic LLM wrapper functionality into a fully functional multi-agent orchestration platform with real memory management, tool integration, and team collaboration capabilities.
- **Purpose**: To address the critical issues identified in the system analysis report, including missing database tables, lack of agent memory, non-functional websearch, and incomplete team collaboration features.
- **Target Users**: Developers, system administrators, and end-users who need a robust multi-agent orchestration platform for complex task management and collaboration.

## Goals
- Implement a proper agent core loop with function calling capabilities
- Create missing database tables and implement proper memory management
- Enable real websearch and independent research capabilities
- Implement true multi-agent collaboration and task sharing
- Establish a sustainable memory management system to prevent context confusion

## Non-Goals (Out of Scope)
- Complete UI redesign (existing UI is already well-designed)
- Integration with external enterprise systems
- Advanced security features beyond basic authentication
- Mobile app development
- Support for additional LLM providers beyond those already implemented

## Background & Context
- The current AgentForge system has a well-designed UI but lacks functional core engine capabilities
- Key issues identified include missing database tables, mock implementations, and lack of real agent collaboration
- The system currently uses a simple request-response model for LLM interactions without proper memory management
- Several critical features like websearch, CLI execution, and team collaboration are either mocked or incomplete

## Functional Requirements
- **FR-1**: Implement missing database tables (knowledge_entries, mcp_tools) and their corresponding CRUD operations
- **FR-2**: Develop a proper agent core loop using ReAct pattern with function calling capabilities
- **FR-3**: Implement memory management system with sliding window, summarization, and long-term memory
- **FR-4**: Enable real websearch integration with actual search engines
- **FR-5**: Implement true multi-agent collaboration with task sharing and team communication
- **FR-6**: Add CLI execution capabilities within the chat interface

## Non-Functional Requirements
- **NFR-1**: Performance: Agent responses should be generated within 5-10 seconds for typical tasks
- **NFR-2**: Reliability: System should handle long conversations without losing context or memory
- **NFR-3**: Scalability: System should support up to 10 concurrent agents per team
- **NFR-4**: Maintainability: Code should follow Rust best practices and include comprehensive documentation
- **NFR-5**: Security: CLI execution should be sandboxed to prevent system harm

## Constraints
- **Technical**: Must maintain compatibility with existing Rust codebase and GPUI framework
- **Business**: Implementation should follow the existing architectural patterns
- **Dependencies**: Requires API keys for websearch functionality

## Assumptions
- Existing LLM adapters (Claude, OpenRouter) can be extended to support function calling
- SQLite database is sufficient for the current scale of operations
- Users will provide necessary API keys for websearch functionality

## Acceptance Criteria

### AC-1: Database Schema Completeness
- **Given**: The system is initialized
- **When**: The database is created
- **Then**: All required tables (knowledge_entries, mcp_tools) are created with proper schemas
- **Verification**: `programmatic`
- **Notes**: Tables should include proper indexes and foreign key constraints

### AC-2: Agent Core Loop Functionality
- **Given**: An agent is asked to perform a task
- **When**: The agent needs additional information or tool use
- **Then**: The agent should call the appropriate tool and use the results to complete the task
- **Verification**: `programmatic`
- **Notes**: Should follow ReAct pattern with clear reasoning and action steps

### AC-3: Memory Management Effectiveness
- **Given**: A long conversation with multiple turns
- **When**: The conversation exceeds token limits
- **Then**: The system should automatically summarize older parts and maintain context
- **Verification**: `programmatic`
- **Notes**: Should maintain coherence and not lose important information

### AC-4: Websearch Functionality
- **Given**: An agent is asked to research a topic
- **When**: The agent needs current information
- **Then**: The agent should perform a real websearch and use the results
- **Verification**: `programmatic`
- **Notes**: Should use actual search engine APIs and return relevant results

### AC-5: Multi-Agent Collaboration
- **Given**: A complex task that requires multiple agents
- **When**: The task is assigned to a team
- **Then**: Agents should automatically split the task and collaborate to complete it
- **Verification**: `human-judgment`
- **Notes**: Should demonstrate effective task division and coordination

### AC-6: CLI Execution Capability
- **Given**: An agent is asked to execute a command
- **When**: The agent determines CLI execution is necessary
- **Then**: The agent should execute the command in a sandboxed environment and return results
- **Verification**: `programmatic`
- **Notes**: Should handle both successful and error cases gracefully

## Open Questions
- [ ] Which websearch API should be used (Tavily, DuckDuckGo, Google)?
- [ ] What are the specific token limits for different LLM providers?
- [ ] How should the sandboxed environment for CLI execution be implemented?
- [ ] What is the optimal sliding window size for different LLM models?
- [ ] How should task priority be determined in the shared task list?