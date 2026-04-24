**AgentForge**

Software Requirements Specification

软件需求规格说明书

Version 1.0

版本 1.0

Date: April 6, 2026

日期：2026年4月6日

Classification: Confidential

密级：机密

**Table of Contents / 目录**

*See auto-generated TOC field.*

**1. Introduction / 引言**

**1.1 Purpose / 目的**

The purpose of this Software Requirements Specification (SRS) is to define the complete functional and non-functional requirements for AgentForge, a desktop application designed to orchestrate multiple AI systems for collaborative development and operations. This document serves as the authoritative reference for all stakeholders, including developers, architects, project managers, quality assurance teams, and security auditors.

本软件需求规格说明书（SRS）的目的是定义AgentForge的完整功能需求和非功能需求。AgentForge是一款桌面应用程序，旨在编排多个AI系统进行协作开发和运营。本文档是所有利益相关者的权威参考，包括开发人员、架构师、项目经理、质量保证团队和安全审计人员。

This SRS establishes a clear and unambiguous description of what AgentForge shall do, the constraints under which it must operate, and the criteria against which its completeness and correctness shall be verified. It is intended to reduce the risk of miscommunication among project participants and to provide a baseline for project planning, design, and verification activities.

本SRS建立了对AgentForge应具备的功能、必须遵守的约束条件以及验证其完整性和正确性的标准的清晰明确的描述。旨在降低项目参与者之间误解的风险，并为项目规划、设计和验证活动提供基线。

**1.2 Scope / 范围**

AgentForge is a comprehensive desktop application that orchestrates multiple AI provider systems --- including Google Gemini, OpenAI Codex, Anthropic Claude, ChatGPT, and locally developed models --- to simultaneously develop and operate across various domains. The system provides a unified interface for managing AI agents, organizing them into teams, defining intelligent workflows (iFlows), and enforcing governance constraints on access control and security.

AgentForge是一款综合性桌面应用程序，编排多个AI提供者系统------包括Google Gemini、OpenAI Codex、Anthropic Claude、ChatGPT以及本地开发的模型------在各种领域同时进行开发和运营。该系统提供统一界面来管理AI智能体、将它们组织成团队、定义智能工作流（iFlows），并执行访问控制和安全方面的治理约束。

The scope of this SRS encompasses the core orchestration engine, provider adapter layer, session management, agent communication infrastructure, knowledge base system, MCP tool integration, document generation, token management, security and access control, monitoring and observability, and Obsidian integration. Out of scope are the internal implementations of third-party AI models and external cloud infrastructure management.

本SRS的范围涵盖核心编排引擎、提供者适配器层、会话管理、智能体通信基础设施、知识库系统、MCP工具集成、文档生成、Token管理、安全与访问控制、监控与可观测性以及Obsidian集成。不包括第三方AI模型的内部实现和外部云基础设施管理。

**1.3 Definitions, Acronyms & Abbreviations / 定义、缩写与术语**

  ---------------------- ------------------------------------------------------------------------------------------------------------------
  **Term / 术语**        **Definition / 定义**
  Agent / 智能体         An autonomous AI entity that can perform tasks, communicate with other agents, and interact with external tools.
  AgentForge             The desktop application described in this SRS for orchestrating multiple AI systems.
  iFlow / 智能工作流     An intelligent workflow that defines the sequence and conditions of agent interactions and task execution.
  MCP / 模型上下文协议   Model Context Protocol --- a standardized protocol for communication between AI models and external tools.
  Provider / 提供者      An external AI service or model that provides inference capabilities (e.g., Claude, Gemini, Codex).
  Session / 会话         A managed conversation context between a user and one or more AI agents.
  Team / 团队            A group of agents organized to collaborate on shared tasks with defined roles and responsibilities.
  Token / 令牌           A unit of text processing in AI models; also refers to authentication credentials in security context.
  Briefing / 简报        A summary context document provided to an agent to establish its operational context and objectives.
  GPUI                   A Rust-based GPU-accelerated UI framework used for the desktop interface (from longbridge/gpui-component).
  ConcurrencyGuard       A mechanism within SessionManagerV2 to manage concurrent access to shared session resources.
  TeamBus                A peer-to-peer message routing system for inter-agent communication within a team.
  SharedTaskList         A SQLite-based atomic task claiming system for distributed task management within teams.
  Vault / 仓库           An Obsidian vault used as a knowledge storage and retrieval backend.
  Governance / 治理      Policies and mechanisms that enforce access control, security, and compliance across the system.
  ---------------------- ------------------------------------------------------------------------------------------------------------------

**1.4 References / 参考文献**

-   IEEE Std 830-1998: IEEE Recommended Practice for Software Requirements Specifications.

-   IEEE Std 830-1998：IEEE软件需求规格说明推荐实践。

-   ISO/IEC/IEEE 29148:2011: Systems and software engineering --- Life cycle processes --- Requirements engineering.

-   ISO/IEC/IEEE 29148:2011：系统与软件工程------生命周期过程------需求工程。

-   Model Context Protocol (MCP) Specification --- https://modelcontextprotocol.io

-   模型上下文协议（MCP）规范 --- https://modelcontextprotocol.io

-   longbridge/gpui-component --- Rust-based GPUI component library for desktop UI development.

-   longbridge/gpui-component --- 基于Rust的GPUI组件库，用于桌面UI开发。

-   Anthropic Claude API Documentation, Google Gemini API Documentation, OpenAI API Documentation.

-   Anthropic Claude API文档、Google Gemini API文档、OpenAI API文档。

**1.5 Overview / 概述**

The remainder of this document is organized as follows: Section 2 provides an overall description of the product, its functions, user characteristics, constraints, and dependencies. Section 3 describes the system architecture, including the core orchestration system, provider adapter layer, session management, agent communication, and knowledge base. Section 4 details the functional requirements organized by subsystem. Section 5 specifies non-functional requirements. Section 6 defines interface requirements. Section 7 covers data requirements. Section 8 addresses verification and validation criteria.

本文档的其余部分组织如下：第2节提供产品的总体描述，包括其功能、用户特征、约束和依赖。第3节描述系统架构，包括核心编排系统、提供者适配器层、会话管理、智能体通信和知识库。第4节按子系统详细说明功能需求。第5节规定非功能需求。第6节定义接口需求。第7节涵盖数据需求。第8节阐述验证和确认标准。

**2. Overall Description / 总体描述**

**2.1 Product Perspective / 产品视角**

AgentForge is a standalone desktop application built with a Rust-based GPUI framework (longbridge/gpui-component) for the frontend, with Node.js-based backend services for provider communication, data persistence, and orchestration logic. It operates as an orchestrator layer that sits between human users and multiple AI provider systems, managing the lifecycle of AI agents, their interactions, and the governance policies that constrain their behavior.

AgentForge是一款独立的桌面应用程序，前端使用基于Rust的GPUI框架（longbridge/gpui-component）构建，后端使用基于Node.js的服务进行提供者通信、数据持久化和编排逻辑。它作为编排层位于人类用户和多个AI提供者系统之间，管理AI智能体的生命周期、交互以及约束其行为的治理策略。

The system does not replace any individual AI provider but rather provides a unified management plane for coordinating multiple providers simultaneously. It integrates with external tools through the Model Context Protocol (MCP), stores knowledge in Obsidian-compatible Markdown vaults, and persists operational data in SQLite databases using the better-sqlite3 library with a Repository pattern for data access.

该系统不替代任何单个AI提供者，而是提供一个统一的管理平面来同时协调多个提供者。它通过模型上下文协议（MCP）与外部工具集成，以Obsidian兼容的Markdown仓库存储知识，并使用better-sqlite3库通过Repository模式进行数据访问，将运营数据持久化到SQLite数据库中。

**2.2 Product Functions / 产品功能**

AgentForge provides the following major functional capabilities:

AgentForge提供以下主要功能能力：

-   Agent Management: Create, configure, deploy, monitor, and retire AI agents with customizable personalities, capabilities, and provider bindings.

-   智能体管理：创建、配置、部署、监控和退役AI智能体，支持自定义个性、能力和提供者绑定。

-   Team Management: Organize agents into collaborative teams with defined roles, shared task lists, and peer-to-peer communication channels.

-   团队管理：将智能体组织成协作团队，具有明确的角色、共享任务列表和点对点通信通道。

-   Operating Modes: Support multiple operational modes including autonomous, semi-autonomous, supervised, and manual execution modes.

-   运行模式：支持多种运行模式，包括自主、半自主、监督和手动执行模式。

-   iFlows & Workflows: Define and execute intelligent workflows that orchestrate complex multi-step tasks across multiple agents and providers.

-   智能工作流：定义和执行编排跨多个智能体和提供者的复杂多步骤任务的智能工作流。

-   Provider Adapter: Abstract provider-specific communication through a unified adapter layer supporting Claude, Gemini, Codex, and custom models.

-   提供者适配器：通过统一的适配器层抽象提供者特定的通信，支持Claude、Gemini、Codex和自定义模型。

-   Orchestration Engine: Coordinate agent activities, manage resource allocation, enforce governance policies, and handle error recovery.

-   编排引擎：协调智能体活动、管理资源分配、执行治理策略并处理错误恢复。

-   Knowledge Base (Brains): Maintain and retrieve organizational knowledge through Obsidian-integrated Markdown-based storage.

-   知识库（大脑）：通过Obsidian集成的基于Markdown的存储维护和检索组织知识。

-   MCP Tools: Integrate external tools and services through the Model Context Protocol for extended agent capabilities.

-   MCP工具：通过模型上下文协议集成外部工具和服务，扩展智能体能力。

-   Document Generation: Automatically generate reports, documentation, and summaries from agent activities and workflow outputs.

-   文档生成：从智能体活动和工作流输出自动生成报告、文档和摘要。

-   Token Management: Monitor, allocate, and optimize token usage across providers and agents to control costs and ensure efficient resource utilization.

-   Token管理：跨提供者和智能体监控、分配和优化Token使用，以控制成本并确保高效的资源利用。

-   Security & Access Control: Enforce role-based access control, API key management, data encryption, and audit logging.

-   安全与访问控制：执行基于角色的访问控制、API密钥管理、数据加密和审计日志记录。

-   Monitoring & Observability: Provide real-time dashboards, metrics collection, log aggregation, and alerting for system health monitoring.

-   监控与可观测性：提供实时仪表板、指标收集、日志聚合和告警，用于系统健康监控。

-   Obsidian Integration: Seamless integration with Obsidian vaults for knowledge storage, retrieval, and bi-directional synchronization.

-   Obsidian集成：与Obsidian仓库无缝集成，实现知识存储、检索和双向同步。

**2.3 User Characteristics / 用户特征**

  ----------------------------------- ----------------------------------------------------------------------------------------------- ----------------------------
  **User Type / 用户类型**            **Description / 描述**                                                                          **Skill Level / 技能水平**
  System Administrator / 系统管理员   Manages system configuration, security policies, provider connections, and user accounts.       Advanced / 高级
  AI Engineer / AI工程师              Creates and configures agents, defines iFlows, and manages provider adapters.                   Advanced / 高级
  Team Lead / 团队负责人              Organizes agent teams, assigns roles, monitors team performance, and manages shared tasks.      Intermediate / 中级
  Developer / 开发者                  Uses agents for code generation, review, testing, and documentation tasks.                      Intermediate / 中级
  Analyst / 分析师                    Uses agents for data analysis, research, report generation, and knowledge retrieval.            Intermediate / 中级
  End User / 最终用户                 Interacts with agents through natural language for task delegation and information retrieval.   Basic / 基础
  ----------------------------------- ----------------------------------------------------------------------------------------------- ----------------------------

**2.4 Constraints / 约束**

-   The application shall be developed as a desktop application using the GPUI framework (Rust-based) for the frontend.

-   应用程序应作为桌面应用程序开发，前端使用GPUI框架（基于Rust）。

-   The backend services shall use Node.js runtime with TypeScript for business logic and orchestration.

-   后端服务应使用Node.js运行时和TypeScript进行业务逻辑和编排。

-   Data persistence shall use SQLite via the better-sqlite3 library with a Repository pattern.

-   数据持久化应通过better-sqlite3库使用SQLite，采用Repository模式。

-   The system shall support concurrent sessions with thread-safe access managed by ConcurrencyGuard.

-   系统应支持并发会话，通过ConcurrencyGuard管理线程安全访问。

-   Provider adapters shall implement the BaseProviderAdapter interface for standardized communication.

-   提供者适配器应实现BaseProviderAdapter接口以进行标准化通信。

-   MCP integration shall use the \@modelcontextprotocol/sdk package.

-   MCP集成应使用\@modelcontextprotocol/sdk包。

-   The system shall operate within the context window limits of each AI provider (e.g., 200K tokens for Claude, 128K for Gemini).

-   系统应在每个AI提供者的上下文窗口限制内运行（例如Claude为200K Token，Gemini为128K Token）。

-   Team communication shall use WebSocket-based AgentBridge for real-time messaging.

-   团队通信应使用基于WebSocket的AgentBridge进行实时消息传递。

**2.5 Assumptions & Dependencies / 假设与依赖**

Assumptions:

假设：

-   Users have valid API keys or access credentials for the AI providers they intend to use.

-   用户拥有其打算使用的AI提供者的有效API密钥或访问凭证。

-   The desktop environment meets the minimum hardware requirements for running Rust-based GPUI applications.

-   桌面环境满足运行基于Rust的GPUI应用程序的最低硬件要求。

-   Network connectivity is available for accessing cloud-based AI providers.

-   有可用的网络连接来访问基于云的AI提供者。

-   Users have basic familiarity with AI concepts and agent-based systems.

-   用户对AI概念和基于智能体的系统有基本了解。

Dependencies:

依赖：

-   GPUI framework (longbridge/gpui-component) for desktop UI rendering.

-   GPUI框架（longbridge/gpui-component）用于桌面UI渲染。

-   better-sqlite3 for embedded database functionality.

-   better-sqlite3用于嵌入式数据库功能。

-   \@modelcontextprotocol/sdk for MCP tool integration.

-   \@modelcontextprotocol/sdk用于MCP工具集成。

-   Anthropic Claude SDK, Google Gemini SDK, OpenAI SDK for provider communication.

-   Anthropic Claude SDK、Google Gemini SDK、OpenAI SDK用于提供者通信。

-   Obsidian application (optional) for knowledge vault management and visualization.

-   Obsidian应用程序（可选）用于知识仓库管理和可视化。

**3. System Architecture / 系统架构**

**3.1 High-Level Architecture Diagram Description / 高层架构图描述**

AgentForge follows a layered architecture pattern with the following tiers from top to bottom:

AgentForge采用分层架构模式，从上到下分为以下层次：

-   Presentation Layer (GPUI): Rust-based desktop UI components providing the user interface for agent management, team oversight, workflow design, and system monitoring. Built on the longbridge/gpui-component library.

-   表示层（GPUI）：基于Rust的桌面UI组件，提供智能体管理、团队监督、工作流设计和系统监控的用户界面。基于longbridge/gpui-component库构建。

-   Application Layer (Node.js/TypeScript): Core business logic including orchestration engine, session management, agent lifecycle management, and workflow execution.

-   应用层（Node.js/TypeScript）：核心业务逻辑，包括编排引擎、会话管理、智能体生命周期管理和工作流执行。

-   Adapter Layer: Provider-specific adapters (ClaudeSdkAdapter, CodexAppServerAdapter, GeminiHeadlessAdapter, IFlowAcpAdapter) implementing the BaseProviderAdapter interface.

-   适配器层：特定提供者的适配器（ClaudeSdkAdapter、CodexAppServerAdapter、GeminiHeadlessAdapter、IFlowAcpAdapter），实现BaseProviderAdapter接口。

-   Communication Layer: WebSocket-based AgentBridge for real-time agent communication, TeamBus for P2P message routing, and MCP client for tool integration.

-   通信层：基于WebSocket的AgentBridge用于实时智能体通信，TeamBus用于P2P消息路由，MCP客户端用于工具集成。

-   Data Layer: SQLite databases (better-sqlite3) with Repository pattern, Obsidian vault integration for knowledge storage, and file system for document generation.

-   数据层：SQLite数据库（better-sqlite3）使用Repository模式，Obsidian仓库集成用于知识存储，文件系统用于文档生成。

**3.2 Core Orchestration System / 核心编排系统**

The Core Orchestration System is the central nervous system of AgentForge. It is responsible for coordinating all agent activities, managing resource allocation across providers, enforcing governance policies, and handling error recovery. The orchestration engine maintains a global state model that tracks the status of all active agents, teams, workflows, and sessions.

核心编排系统是AgentForge的中枢神经系统。它负责协调所有智能体活动、管理跨提供者的资源分配、执行治理策略并处理错误恢复。编排引擎维护一个全局状态模型，跟踪所有活跃智能体、团队、工作流和会话的状态。

The orchestration engine implements a priority-based task scheduler that considers agent availability, provider rate limits, token budgets, and task dependencies when scheduling work. It supports both synchronous execution (where the user waits for results) and asynchronous execution (where tasks run in the background with notification upon completion).

编排引擎实现了一个基于优先级的任务调度器，在调度工作时考虑智能体可用性、提供者速率限制、Token预算和任务依赖关系。它支持同步执行（用户等待结果）和异步执行（任务在后台运行，完成后通知）。

**3.3 Provider Adapter Layer / 提供者适配器层**

The Provider Adapter Layer abstracts the differences between various AI providers through a unified BaseProviderAdapter interface. Each provider-specific adapter (ClaudeSdkAdapter, CodexAppServerAdapter, GeminiHeadlessAdapter, IFlowAcpAdapter) implements this interface to provide standardized methods for sending messages, receiving responses, managing context windows, and handling provider-specific features.

提供者适配器层通过统一的BaseProviderAdapter接口抽象了各种AI提供者之间的差异。每个特定提供者的适配器（ClaudeSdkAdapter、CodexAppServerAdapter、GeminiHeadlessAdapter、IFlowAcpAdapter）实现此接口，提供标准化的方法来发送消息、接收响应、管理上下文窗口和处理提供者特定的功能。

The adapter layer handles provider-specific concerns such as authentication, rate limiting, error mapping, response format normalization, and streaming support. It implements circuit breaker patterns to gracefully handle provider outages and automatic failover to alternative providers when configured.

适配器层处理提供者特定的关注点，如身份验证、速率限制、错误映射、响应格式标准化和流式传输支持。它实现了断路器模式以优雅地处理提供者故障，并在配置时自动故障转移到备用提供者。

**3.4 Session & Conversation Management / 会话与对话管理**

The Session Management subsystem, built around SessionManagerV2, provides lifecycle management for conversation contexts between users and agents. Each session maintains a complete conversation history, context state, and associated metadata. The ConcurrencyGuard mechanism ensures thread-safe access to shared session resources, preventing race conditions when multiple agents or workflows attempt to access the same session simultaneously.

会话管理子系统围绕SessionManagerV2构建，提供用户和智能体之间对话上下文的生命周期管理。每个会话维护完整的对话历史、上下文状态和关联的元数据。ConcurrencyGuard机制确保对共享会话资源的线程安全访问，防止多个智能体或工作流同时尝试访问同一会话时出现竞态条件。

Sessions support context window management with automatic summarization of long conversations, configurable context retention policies, and the ability to branch conversations for exploratory purposes without affecting the main conversation thread.

会话支持上下文窗口管理，具有长对话自动摘要、可配置的上下文保留策略以及分支对话的能力（用于探索目的而不影响主对话线程）。

**3.5 Agent Communication Infrastructure / 智能体通信基础设施**

The Agent Communication Infrastructure enables real-time messaging between agents through the AgentBridge WebSocket system and the TeamBus peer-to-peer routing mechanism. AgentBridge provides a low-latency communication channel for direct agent-to-agent messaging, while TeamBus handles group communication within teams, including broadcast, multicast, and targeted message delivery.

智能体通信基础设施通过AgentBridge WebSocket系统和TeamBus点对点路由机制实现智能体之间的实时消息传递。AgentBridge为直接的智能体间消息传递提供低延迟通信通道，而TeamBus处理团队内的组通信，包括广播、多播和定向消息传递。

The BriefingManager component prepares context summaries (briefings) that agents can share with each other to establish common operational context. This is particularly important when agents from different providers need to collaborate, as each agent may have different context window limitations and capabilities.

BriefingManager组件准备智能体可以相互共享的上下文摘要（简报），以建立共同的运营上下文。当来自不同提供者的智能体需要协作时，这一点尤为重要，因为每个智能体可能有不同的上下文窗口限制和能力。

**3.6 Knowledge Base System / 知识库系统**

The Knowledge Base System (referred to as \'Brains\') provides a centralized knowledge management capability integrated with Obsidian vaults. It stores organizational knowledge in Markdown format, enabling compatibility with Obsidian\'s linking, tagging, and graph visualization features. The system supports both structured knowledge (templates, procedures, reference data) and unstructured knowledge (notes, research findings, conversation summaries).

知识库系统（称为\'大脑\'）提供与Obsidian仓库集成的集中式知识管理能力。它以Markdown格式存储组织知识，实现与Obsidian的链接、标签和图可视化功能的兼容性。系统支持结构化知识（模板、程序、参考数据）和非结构化知识（笔记、研究发现、对话摘要）。

The knowledge base provides semantic search capabilities, automatic knowledge extraction from agent interactions, and bi-directional synchronization with Obsidian vaults. Knowledge entries can be tagged, categorized, and linked to create a rich knowledge graph that agents can query during their operations. The knowledge graph interface shall include a minimap (similar to the iFlow builder) for easier navigation and orientation within complex knowledge structures.

知识库提供语义搜索能力、从智能体交互中自动提取知识以及与Obsidian仓库的双向同步。知识条目可以被标记、分类和链接，以创建智能体在运营期间可以查询的丰富知识图谱。知识图谱界面应包含一个迷你图（类似于iFlow构建器中的迷你图），以便在复杂的知识结构中更轻松地导航和定位。

**4. Functional Requirements / 功能需求**

**4.1 Agent Management / 智能体管理 (REQ-AM-001 to REQ-AM-015)**

**REQ-AM-001: Agent Creation** \[High\]

REQ-AM-001: 智能体创建 \[高\]

The system shall allow users to create new AI agents with configurable names, descriptions, provider bindings, system prompts, and capability profiles. Each agent shall be assigned a unique identifier that persists across sessions.

系统应允许用户创建新的AI智能体，具有可配置的名称、描述、提供者绑定、系统提示和能力配置文件。每个智能体应分配一个在会话间持久存在的唯一标识符。

**REQ-AM-002: Agent Configuration** \[High\]

REQ-AM-002: 智能体配置 \[高\]

The system shall provide a configuration interface for setting agent parameters including temperature, max tokens, top-p, frequency penalty, presence penalty, and provider-specific options.

系统应提供配置界面来设置智能体参数，包括温度、最大Token数、top-p、频率惩罚、存在惩罚和提供者特定的选项。

**REQ-AM-003: Agent Provider Binding** \[High\]

REQ-AM-003: 智能体提供者绑定 \[高\]

The system shall allow binding an agent to one or more AI providers with configurable primary and fallback providers. The system shall automatically failover to fallback providers when the primary provider is unavailable.

系统应允许将智能体绑定到一个或多个AI提供者，具有可配置的主提供者和备用提供者。当主提供者不可用时，系统应自动故障转移到备用提供者。

**REQ-AM-004: Agent Lifecycle Management** \[High\]

REQ-AM-004: 智能体生命周期管理 \[高\]

The system shall support the full agent lifecycle including creation, activation, deactivation, suspension, and retirement. Retired agents shall be archived with their configuration and conversation history preserved.

系统应支持完整的智能体生命周期，包括创建、激活、停用、暂停和退役。退役的智能体应被归档，其配置和对话历史应被保留。

**REQ-AM-005: Agent Personality & System Prompt** \[High\]

REQ-AM-005: 智能体个性与系统提示 \[高\]

The system shall allow users to define custom system prompts that establish the agent\'s personality, expertise areas, communication style, and behavioral constraints. System prompts shall support template variables for dynamic context injection.

系统应允许用户定义自定义系统提示，以建立智能体的个性、专业领域、沟通风格和行为约束。系统提示应支持模板变量用于动态上下文注入。

**REQ-AM-006: Agent Briefing Management** \[High\]

REQ-AM-006: 智能体简报管理 \[高\]

The system shall support the BriefingManager component for creating, updating, and distributing operational briefings to agents. Briefings shall include task context, objectives, constraints, and relevant knowledge references.

系统应支持BriefingManager组件来创建、更新和向智能体分发运营简报。简报应包括任务上下文、目标、约束和相关知识引用。

**REQ-AM-007: Agent MCP Server Integration** \[High\]

REQ-AM-007: 智能体MCP服务器集成 \[高\]

The system shall provide AgentMCPServer functionality that allows each agent to expose and consume MCP tools. Each agent shall have a configurable set of available MCP tools based on its role and permissions.

系统应提供AgentMCPServer功能，允许每个智能体暴露和使用MCP工具。每个智能体应根据其角色和权限拥有可配置的可用MCP工具集。

**REQ-AM-008: Agent WebSocket Bridge** \[High\]

REQ-AM-008: 智能体WebSocket桥接 \[高\]

The system shall implement AgentBridge using WebSocket connections for real-time bidirectional communication between agents. The bridge shall support message queuing, delivery acknowledgment, and automatic reconnection.

系统应使用WebSocket连接实现AgentBridge，用于智能体之间的实时双向通信。桥接应支持消息排队、送达确认和自动重新连接。

**REQ-AM-009: Agent Monitoring Dashboard** \[High\]

REQ-AM-009: 智能体监控仪表板 \[高\]

The system shall provide a real-time monitoring dashboard showing agent status, active sessions, token consumption, response latency, error rates, and current task assignments.

系统应提供实时监控仪表板，显示智能体状态、活跃会话、Token消耗、响应延迟、错误率和当前任务分配。

**REQ-AM-010: Agent Version Control** \[High\]

REQ-AM-010: 智能体版本控制 \[高\]

The system shall maintain version history for agent configurations, allowing users to view, compare, and rollback to previous configurations. Each configuration change shall be logged with a timestamp and user identifier.

系统应维护智能体配置的版本历史，允许用户查看、比较和回滚到以前的配置。每次配置更改应记录时间戳和用户标识符。

**REQ-AM-011: Agent Template Library** \[High\]

REQ-AM-011: 智能体模板库 \[高\]

The system shall provide a library of pre-configured agent templates for common use cases such as code review, documentation generation, data analysis, and customer support. Users shall be able to create custom templates.

系统应提供预配置智能体模板库，用于代码审查、文档生成、数据分析和客户支持等常见用例。用户应能够创建自定义模板。

**REQ-AM-012: Agent Import/Export** \[High\]

REQ-AM-012: 智能体导入/导出 \[高\]

The system shall support importing and exporting agent configurations in a standardized JSON format. Exported configurations shall include all agent settings, prompts, and associated MCP tool configurations.

系统应支持以标准化JSON格式导入和导出智能体配置。导出的配置应包括所有智能体设置、提示和关联的MCP工具配置。

**REQ-AM-013: Agent Resource Limits** \[High\]

REQ-AM-013: 智能体资源限制 \[高\]

The system shall allow administrators to set per-agent resource limits including maximum concurrent sessions, token budget per session, maximum response length, and rate limits for provider API calls.

系统应允许管理员设置每个智能体的资源限制，包括最大并发会话数、每个会话的Token预算、最大响应长度和提供者API调用的速率限制。

**REQ-AM-014: Agent Error Handling** \[High\]

REQ-AM-014: 智能体错误处理 \[高\]

The system shall implement comprehensive error handling for agents including provider errors, timeout handling, retry logic with exponential backoff, and graceful degradation when providers are unavailable.

系统应为智能体实现全面的错误处理，包括提供者错误、超时处理、指数退避重试逻辑以及提供者不可用时的优雅降级。

**REQ-AM-015: Agent Health Checks** \[High\]

REQ-AM-015: 智能体健康检查 \[高\]

The system shall perform periodic health checks on agents to verify provider connectivity, configuration validity, and operational readiness. Failed health checks shall trigger alerts and automatic remediation when possible.

系统应对智能体执行定期健康检查，以验证提供者连接性、配置有效性和运营准备状态。失败的健康检查应触发警报并在可能时自动修复。

**4.2 Team Management / 团队管理 (REQ-TM-001 to REQ-TM-015)**

**REQ-TM-001: Team Creation** \[High\]

REQ-TM-001: 团队创建 \[高\]

The system shall allow users to create teams with configurable names, descriptions, and objectives. Each team shall be assigned a unique identifier and shall be persisted in the teams database table.

系统应允许用户创建具有可配置名称、描述和目标的团队。每个团队应分配一个唯一标识符，并应持久化到teams数据库表中。

**REQ-TM-002: Role Definition** \[High\]

REQ-TM-002: 角色定义 \[高\]

The system shall support defining roles within a team with specific permissions, capabilities, and responsibilities. Roles shall be stored in the roles database table and can be assigned to multiple agents.

系统应支持在团队中定义具有特定权限、能力和职责的角色。角色应存储在roles数据库表中，可以分配给多个智能体。

**REQ-TM-003: Team Instance Management** \[High\]

REQ-TM-003: 团队实例管理 \[高\]

The system shall support creating multiple instances of a team configuration, stored in the instances database table. Each instance shall maintain its own state, task list, and member assignments.

系统应支持创建团队配置的多个实例，存储在instances数据库表中。每个实例应维护自己的状态、任务列表和成员分配。

**REQ-TM-004: Team Membership Management** \[High\]

REQ-TM-004: 团队成员管理 \[高\]

The system shall manage team membership through the members database table, supporting dynamic addition and removal of agents. Membership changes shall be reflected in real-time across all team communication channels.

系统应通过members数据库表管理团队成员，支持动态添加和移除智能体。成员变更应在所有团队通信通道中实时反映。

**REQ-TM-005: Shared Task List** \[High\]

REQ-TM-005: 共享任务列表 \[高\]

The system shall implement SharedTaskList using SQLite atomic claim operations for distributed task management within teams. Tasks shall be claimable by any authorized team member with atomic locking to prevent duplicate processing.

系统应使用SQLite原子声明操作实现SharedTaskList，用于团队内的分布式任务管理。任务应由任何授权团队成员声明，具有原子锁定以防止重复处理。

**REQ-TM-006: TeamBus P2P Routing** \[High\]

REQ-TM-006: TeamBus点对点路由 \[高\]

The system shall implement TeamBus for peer-to-peer message routing within teams. TeamBus shall support broadcast messages to all team members, targeted messages to specific members, and group messages to role-based subsets.

系统应实现TeamBus用于团队内的点对点消息路由。TeamBus应支持向所有团队成员广播消息、向特定成员发送定向消息以及向基于角色的子集发送组消息。

**REQ-TM-007: Team Communication Persistence** \[High\]

REQ-TM-007: 团队通信持久化 \[高\]

The system shall persist all team communication messages in the messages database table. Messages shall include metadata such as sender, recipient, timestamp, message type, and delivery status.

系统应将所有团队通信消息持久化到messages数据库表中。消息应包括发送者、接收者、时间戳、消息类型和送达状态等元数据。

**REQ-TM-008: Team Task Assignment** \[High\]

REQ-TM-008: 团队任务分配 \[高\]

The system shall support both manual and automatic task assignment within teams. Automatic assignment shall consider agent capabilities, current workload, provider availability, and task priority.

系统应支持团队内的手动和自动任务分配。自动分配应考虑智能体能力、当前工作负载、提供者可用性和任务优先级。

**REQ-TM-009: Team Collaboration Workflows** \[High\]

REQ-TM-009: 团队协作工作流 \[高\]

The system shall support defining collaboration workflows that specify how team members interact to complete complex tasks. Workflows shall support sequential, parallel, and conditional execution patterns.

系统应支持定义协作工作流，指定团队成员如何交互以完成复杂任务。工作流应支持顺序、并行和条件执行模式。

**REQ-TM-010: Team Performance Metrics** \[High\]

REQ-TM-010: 团队性能指标 \[高\]

The system shall collect and display team performance metrics including task completion rate, average response time, inter-agent communication volume, and resource utilization per team member.

系统应收集和显示团队性能指标，包括任务完成率、平均响应时间、智能体间通信量和每个团队成员的资源利用率。

**REQ-TM-011: Team Scalability** \[High\]

REQ-TM-011: 团队可扩展性 \[高\]

The system shall support teams of varying sizes, from small focused teams (2-5 agents) to large-scale teams (50+ agents). Performance shall degrade gracefully as team size increases.

系统应支持不同规模的团队，从小型专注团队（2-5个智能体）到大规模团队（50+个智能体）。随着团队规模增加，性能应优雅降级。

**REQ-TM-012: Team Template Management** \[High\]

REQ-TM-012: 团队模板管理 \[高\]

The system shall provide pre-configured team templates for common collaboration patterns such as code development teams, research teams, and operations teams. Users shall be able to create and share custom templates.

系统应为常见协作模式提供预配置的团队模板，如代码开发团队、研究团队和运营团队。用户应能够创建和共享自定义模板。

**REQ-TM-013: Team Conflict Resolution** \[High\]

REQ-TM-013: 团队冲突解决 \[高\]

The system shall implement conflict resolution mechanisms for situations where multiple agents attempt to claim the same task or produce contradictory outputs. Resolution strategies shall include priority-based, consensus-based, and arbitrator-based approaches.

系统应实现冲突解决机制，用于多个智能体尝试声明同一任务或产生矛盾输出的情况。解决策略应包括基于优先级、基于共识和基于仲裁的方法。

**REQ-TM-014: Team Audit Trail** \[High\]

REQ-TM-014: 团队审计跟踪 \[高\]

The system shall maintain a comprehensive audit trail of all team activities including membership changes, task assignments, role modifications, and configuration updates. Audit records shall be immutable and tamper-proof.

系统应维护所有团队活动的全面审计跟踪，包括成员变更、任务分配、角色修改和配置更新。审计记录应不可变且防篡改。

**REQ-TM-015: Team Dissolution** \[High\]

REQ-TM-015: 团队解散 \[高\]

The system shall support graceful team dissolution with configurable options for archiving team data, reassigning active tasks, notifying team members, and preserving knowledge generated during the team\'s operation.

系统应支持优雅的团队解散，具有可配置的选项来归档团队数据、重新分配活跃任务、通知团队成员以及保留团队运营期间生成的知识。

**4.3 Operating Modes / 运行模式 (REQ-OM-001 to REQ-OM-015)**

**REQ-OM-001: Autonomous Mode** \[High\]

REQ-OM-001: 自主模式 \[高\]

The system shall support an autonomous operating mode where agents independently execute tasks without human intervention. In this mode, agents shall make decisions, invoke tools, and collaborate with other agents based on their configured objectives and constraints.

系统应支持自主运行模式，智能体独立执行任务而无需人工干预。在此模式下，智能体应根据其配置的目标和约束做出决策、调用工具并与其它智能体协作。

**REQ-OM-002: Semi-Autonomous Mode** \[High\]

REQ-OM-002: 半自主模式 \[高\]

The system shall support a semi-autonomous mode where agents execute tasks independently but require human approval for critical operations such as external API calls, data modifications, and resource-intensive operations.

系统应支持半自主模式，智能体独立执行任务但需要人工批准关键操作，如外部API调用、数据修改和资源密集型操作。

**REQ-OM-003: Supervised Mode** \[High\]

REQ-OM-003: 监督模式 \[高\]

The system shall support a supervised mode where all agent actions are presented to a human supervisor for review and approval before execution. The supervisor shall be able to modify, approve, or reject proposed actions.

系统应支持监督模式，所有智能体操作在执行前提交给人类监督者审查和批准。监督者应能够修改、批准或拒绝提议的操作。

**REQ-OM-004: Manual Mode** \[High\]

REQ-OM-004: 手动模式 \[高\]

The system shall support a manual mode where agents only execute explicitly directed commands from human users. Agents shall not initiate any actions autonomously in this mode.

系统应支持手动模式，智能体仅执行人类用户明确指示的命令。在此模式下，智能体不应自主发起任何操作。

**REQ-OM-005: Mode Transition** \[High\]

REQ-OM-005: 模式切换 \[高\]

The system shall allow seamless transition between operating modes without interrupting active sessions or losing context. Mode transitions shall be logged and all pending actions shall be handled according to the new mode\'s policies.

系统应允许在运行模式之间无缝切换，而不中断活跃会话或丢失上下文。模式切换应被记录，所有待处理操作应根据新模式策略处理。

**REQ-OM-006: Mode-Specific Governance** \[High\]

REQ-OM-006: 模式特定治理 \[高\]

The system shall enforce mode-specific governance policies that define what actions are permitted, restricted, or require approval in each operating mode. Governance policies shall be configurable per team and per agent.

系统应执行模式特定的治理策略，定义每种运行模式中允许、限制或需要批准的操作。治理策略应可按团队和按智能体配置。

**REQ-OM-007: Autonomous Safety Limits** \[High\]

REQ-OM-007: 自主安全限制 \[高\]

The system shall enforce safety limits in autonomous mode including maximum consecutive autonomous actions, spending limits, scope restrictions, and mandatory human check-in intervals.

系统应在自主模式下执行安全限制，包括最大连续自主操作数、支出限制、范围限制和强制人工签到间隔。

**REQ-OM-008: Batch Processing Mode** \[High\]

REQ-OM-008: 批处理模式 \[高\]

The system shall support a batch processing mode where multiple tasks are queued and executed sequentially or in parallel. Batch mode shall support progress tracking, error handling per task, and summary reporting.

系统应支持批处理模式，多个任务排队并顺序或并行执行。批处理模式应支持进度跟踪、每个任务的错误处理和摘要报告。

**REQ-OM-009: Debug Mode** \[High\]

REQ-OM-009: 调试模式 \[高\]

The system shall provide a debug mode with enhanced logging, step-by-step execution, variable inspection, and breakpoint capabilities for troubleshooting agent behavior and workflow execution.

系统应提供调试模式，具有增强的日志记录、逐步执行、变量检查和断点功能，用于故障排除智能体行为和工作流执行。

**REQ-OM-010: Simulation Mode** \[High\]

REQ-OM-010: 模拟模式 \[高\]

The system shall support a simulation mode where agent actions are predicted and displayed without actual execution. This allows users to preview the effects of agent decisions before committing to them.

系统应支持模拟模式，智能体操作被预测和显示而不实际执行。这允许用户在提交之前预览智能体决策的效果。

**REQ-OM-011: Scheduled Execution Mode** \[High\]

REQ-OM-011: 定时执行模式 \[高\]

The system shall support scheduled execution of tasks and workflows at specified times or intervals. Schedules shall support cron-like expressions and calendar-based scheduling.

系统应支持在指定时间或间隔定时执行任务和工作流。调度应支持类cron表达式和基于日历的调度。

**REQ-OM-012: Reactive Mode** \[High\]

REQ-OM-012: 响应模式 \[高\]

The system shall support a reactive mode where agents respond to external events, webhooks, or triggers. Reactive mode shall support event filtering, priority-based response, and escalation procedures.

系统应支持响应模式，智能体响应外部事件、webhook或触发器。响应模式应支持事件过滤、基于优先级的响应和升级程序。

**REQ-OM-013: Learning Mode** \[High\]

REQ-OM-013: 学习模式 \[高\]

The system shall support a learning mode where agent performance and user feedback are collected to improve future task execution. Learning data shall be used to refine agent prompts, workflow configurations, and task assignment strategies.

系统应支持学习模式，收集智能体性能和用户反馈以改进未来的任务执行。学习数据应用于优化智能体提示、工作流配置和任务分配策略。

**REQ-OM-014: Maintenance Mode** \[High\]

REQ-OM-014: 维护模式 \[高\]

The system shall support a maintenance mode where non-critical operations are suspended while essential monitoring and alerting continue. Maintenance mode shall prevent new task creation and gracefully complete in-progress tasks.

系统应支持维护模式，非关键操作暂停而基本监控和告警继续。维护模式应阻止新任务创建并优雅地完成进行中的任务。

**REQ-OM-015: Mode Configuration Persistence** \[High\]

REQ-OM-015: 模式配置持久化 \[高\]

The system shall persist mode configurations and preferences across application restarts. Default operating modes shall be configurable at the system, team, and agent levels.

系统应在应用程序重启之间持久化模式配置和偏好。默认运行模式应在系统、团队和智能体级别可配置。

**4.4 iFlows & Workflows / 智能工作流 (REQ-IF-001 to REQ-IF-015)**

**REQ-IF-001: iFlow Definition** \[High\]

REQ-IF-001: 智能工作流定义 \[高\]

The system shall provide a visual workflow editor for defining iFlows that specify the sequence, conditions, and data flow between agent operations. iFlows shall support drag-and-drop composition of workflow steps.

系统应提供可视化工作流编辑器，用于定义指定智能体操作之间序列、条件和数据流的iFlows。iFlows应支持拖放式工作流步骤组合。

**REQ-IF-002: Workflow Step Types** \[High\]

REQ-IF-002: 工作流步骤类型 \[高\]

The system shall support multiple workflow step types including agent invocation, conditional branching, parallel execution, data transformation, human approval gates, tool invocation, and error handling steps.

系统应支持多种工作流步骤类型，包括智能体调用、条件分支、并行执行、数据转换、人工审批门、工具调用和错误处理步骤。

**REQ-IF-003: IFlowAcpAdapter Integration** \[High\]

REQ-IF-003: IFlowAcpAdapter集成 \[高\]

The system shall implement IFlowAcpAdapter for integrating iFlows with the provider adapter layer. The adapter shall translate workflow definitions into provider-specific execution plans.

系统应实现IFlowAcpAdapter，用于将iFlows与提供者适配器层集成。适配器应将工作流定义转换为提供者特定的执行计划。

**REQ-IF-004: Workflow Variables & Context** \[High\]

REQ-IF-004: 工作流变量与上下文 \[高\]

The system shall support workflow variables and context passing between steps. Variables shall support type checking, default values, and transformation functions.

系统应支持工作流变量和步骤之间的上下文传递。变量应支持类型检查、默认值和转换函数。

**REQ-IF-005: Workflow Conditional Logic** \[High\]

REQ-IF-005: 工作流条件逻辑 \[高\]

The system shall support conditional branching based on agent outputs, variable values, external conditions, and composite expressions. Conditions shall support AND, OR, NOT logical operators and comparison functions.

系统应支持基于智能体输出、变量值、外部条件和复合表达式的条件分支。条件应支持AND、OR、NOT逻辑运算符和比较函数。

**REQ-IF-006: Workflow Parallel Execution** \[High\]

REQ-IF-006: 工作流并行执行 \[高\]

The system shall support parallel execution of independent workflow steps with configurable concurrency limits. The system shall handle synchronization points where parallel branches must converge before proceeding.

系统应支持独立工作流步骤的并行执行，具有可配置的并发限制。系统应处理并行分支必须在继续之前汇聚的同步点。

**REQ-IF-007: Workflow Error Handling** \[High\]

REQ-IF-007: 工作流错误处理 \[高\]

The system shall provide comprehensive error handling within workflows including try-catch blocks, retry policies, fallback steps, and error propagation control. Users shall be able to define custom error handling strategies per workflow step.

系统应在工作流内提供全面的错误处理，包括try-catch块、重试策略、回退步骤和错误传播控制。用户应能够为每个工作流步骤定义自定义错误处理策略。

**REQ-IF-008: Workflow Versioning** \[High\]

REQ-IF-008: 工作流版本控制 \[高\]

The system shall maintain version history for iFlow definitions. Users shall be able to view version differences, rollback to previous versions, and create branches for experimental workflow modifications.

系统应维护iFlow定义的版本历史。用户应能够查看版本差异、回滚到以前的版本以及创建分支用于实验性工作流修改。

**REQ-IF-009: Workflow Execution Monitoring** \[High\]

REQ-IF-009: 工作流执行监控 \[高\]

The system shall provide real-time monitoring of workflow execution including step status, execution time, data flow visualization, and bottleneck identification. Users shall be able to pause, resume, and cancel running workflows.

系统应提供工作流执行的实时监控，包括步骤状态、执行时间、数据流可视化和瓶颈识别。用户应能够暂停、恢复和取消正在运行的工作流。

**REQ-IF-010: Workflow Templates** \[High\]

REQ-IF-010: 工作流模板 \[高\]

The system shall provide a library of pre-built workflow templates for common patterns such as code review pipelines, document generation workflows, data processing pipelines, and multi-agent research workflows.

系统应提供预构建工作流模板库，用于常见模式，如代码审查流水线、文档生成工作流、数据处理流水线和多智能体研究工作流。

**REQ-IF-011: Workflow Scheduling** \[High\]

REQ-IF-011: 工作流调度 \[高\]

The system shall support scheduling workflows for execution at specific times or intervals. Scheduled workflows shall support parameterized inputs and conditional execution based on external triggers.

系统应支持调度工作流在特定时间或间隔执行。调度的工作流应支持参数化输入和基于外部触发的条件执行。

**REQ-IF-012: Workflow Input/Output** \[High\]

REQ-IF-012: 工作流输入/输出 \[高\]

The system shall support defining explicit input and output schemas for workflows. Input validation shall be performed before workflow execution, and output shall be formatted according to the defined schema.

系统应支持为工作流定义显式的输入和输出模式。输入验证应在工作流执行前执行，输出应根据定义的模式格式化。

**REQ-IF-013: Sub-workflow Composition** \[High\]

REQ-IF-013: 子工作流组合 \[高\]

The system shall support composing complex workflows from reusable sub-workflows. Sub-workflows shall accept parameters, return values, and maintain their own error handling context.

系统应支持从可重用的子工作流组合复杂工作流。子工作流应接受参数、返回值并维护自己的错误处理上下文。

**REQ-IF-014: Workflow Audit Logging** \[High\]

REQ-IF-014: 工作流审计日志 \[高\]

The system shall log all workflow execution events including step initiation, completion, errors, data transformations, and human interventions. Audit logs shall be queryable and exportable for compliance purposes.

系统应记录所有工作流执行事件，包括步骤启动、完成、错误、数据转换和人工干预。审计日志应可查询和导出以用于合规目的。

**REQ-IF-015: Workflow Import/Export** \[High\]

REQ-IF-015: 工作流导入/导出 \[高\]

The system shall support importing and exporting workflow definitions in a standardized format (JSON/YAML). Exported workflows shall include all dependencies, configurations, and associated agent references.

系统应支持以标准化格式（JSON/YAML）导入和导出工作流定义。导出的工作流应包括所有依赖项、配置和关联的智能体引用。

**4.5 Provider Adapter / 提供者适配器 (REQ-PA-001 to REQ-PA-010)**

**REQ-PA-001: BaseProviderAdapter Interface** \[High\]

REQ-PA-001: BaseProviderAdapter接口 \[高\]

The system shall define a BaseProviderAdapter interface that all provider-specific adapters must implement. The interface shall specify standard methods for initialization, message sending, response receiving, streaming, and health checking.

系统应定义一个BaseProviderAdapter接口，所有特定提供者的适配器必须实现。接口应指定初始化、消息发送、响应接收、流式传输和健康检查的标准方法。

**REQ-PA-002: ClaudeSdkAdapter** \[High\]

REQ-PA-002: ClaudeSdkAdapter \[高\]

The system shall implement ClaudeSdkAdapter for integration with Anthropic Claude API. The adapter shall support all Claude models, streaming responses, tool use, and vision capabilities.

系统应实现ClaudeSdkAdapter用于与Anthropic Claude API集成。适配器应支持所有Claude模型、流式响应、工具使用和视觉能力。

**REQ-PA-003: CodexAppServerAdapter** \[High\]

REQ-PA-003: CodexAppServerAdapter \[高\]

The system shall implement CodexAppServerAdapter for integration with OpenAI Codex through an application server interface. The adapter shall support code generation, completion, and editing operations.

系统应实现CodexAppServerAdapter用于通过应用服务器接口与OpenAI Codex集成。适配器应支持代码生成、补全和编辑操作。

**REQ-PA-004: GeminiHeadlessAdapter** \[High\]

REQ-PA-004: GeminiHeadlessAdapter \[高\]

The system shall implement GeminiHeadlessAdapter for integration with Google Gemini in headless mode. The adapter shall support multimodal inputs, function calling, and long-context operations.

系统应实现GeminiHeadlessAdapter用于在无头模式下与Google Gemini集成。适配器应支持多模态输入、函数调用和长上下文操作。

**REQ-PA-005: Custom Model Adapter** \[High\]

REQ-PA-005: 自定义模型适配器 \[高\]

The system shall support creating custom provider adapters for locally developed or proprietary AI models. Custom adapters shall implement the BaseProviderAdapter interface and support configurable endpoints and authentication.

系统应支持为本地开发或专有AI模型创建自定义提供者适配器。自定义适配器应实现BaseProviderAdapter接口并支持可配置的端点和身份验证。

**REQ-PA-006: Adapter Response Normalization** \[High\]

REQ-PA-006: 适配器响应标准化 \[高\]

The system shall normalize responses from different providers into a unified internal format. Normalization shall handle differences in response structure, content types, metadata formats, and error representations.

系统应将来自不同提供者的响应标准化为统一的内部格式。标准化应处理响应结构、内容类型、元数据格式和错误表示的差异。

**REQ-PA-007: Adapter Rate Limiting** \[High\]

REQ-PA-007: 适配器速率限制 \[高\]

The system shall implement rate limiting per provider adapter based on provider-specific rate limits. Rate limiting shall support token-based, request-based, and time-based limits with configurable buffer thresholds.

系统应根据提供者特定的速率限制为每个提供者适配器实现速率限制。速率限制应支持基于Token、基于请求和基于时间的限制，具有可配置的缓冲阈值。

**REQ-PA-008: Adapter Circuit Breaker** \[High\]

REQ-PA-008: 适配器断路器 \[高\]

The system shall implement a circuit breaker pattern for each provider adapter. The circuit breaker shall detect provider failures, prevent cascading failures, and automatically attempt recovery after a configured cooldown period.

系统应为每个提供者适配器实现断路器模式。断路器应检测提供者故障、防止级联故障并在配置的冷却期后自动尝试恢复。

**REQ-PA-009: Adapter Streaming Support** \[High\]

REQ-PA-009: 适配器流式传输支持 \[高\]

The system shall support streaming responses from all provider adapters. Streaming shall deliver partial responses to the user interface in real-time with configurable chunk sizes and buffering strategies.

系统应支持所有提供者适配器的流式响应。流式传输应以可配置的块大小和缓冲策略实时向用户界面传递部分响应。

**REQ-PA-010: Adapter Metrics Collection** \[High\]

REQ-PA-010: 适配器指标收集 \[高\]

The system shall collect performance metrics for each provider adapter including request latency, response time, error rates, token usage, and throughput. Metrics shall be exposed through the monitoring subsystem.

系统应为每个提供者适配器收集性能指标，包括请求延迟、响应时间、错误率、Token使用量和吞吐量。指标应通过监控子系统暴露。

**4.6 Orchestration Engine / 编排引擎 (REQ-OE-001 to REQ-OE-015)**

**REQ-OE-001: Task Scheduling** \[High\]

REQ-OE-001: 任务调度 \[高\]

The orchestration engine shall implement a priority-based task scheduler that considers agent availability, provider rate limits, token budgets, task dependencies, and team workload when scheduling work items.

编排引擎应实现基于优先级的任务调度器，在调度工作项时考虑智能体可用性、提供者速率限制、Token预算、任务依赖和团队工作负载。

**REQ-OE-002: Resource Allocation** \[High\]

REQ-OE-002: 资源分配 \[高\]

The orchestration engine shall dynamically allocate computational resources across agents and providers based on current demand, priority levels, and configured quotas. Resource allocation shall be adjustable in real-time.

编排引擎应根据当前需求、优先级和配置的配额，在智能体和提供者之间动态分配计算资源。资源分配应可实时调整。

**REQ-OE-003: Governance Policy Enforcement** \[High\]

REQ-OE-003: 治理策略执行 \[高\]

The orchestration engine shall enforce governance policies across all operations including access control, data handling, provider usage, and agent behavior. Policy violations shall be logged and blocked.

编排引擎应在所有操作中执行治理策略，包括访问控制、数据处理、提供者使用和智能体行为。策略违规应被记录和阻止。

**REQ-OE-004: AgentManagerV2 Integration** \[High\]

REQ-OE-004: AgentManagerV2集成 \[高\]

The orchestration engine shall integrate with AgentManagerV2 for agent lifecycle management, capability discovery, and health monitoring. The engine shall query AgentManagerV2 for available agents and their current states.

编排引擎应与AgentManagerV2集成，用于智能体生命周期管理、能力发现和健康监控。引擎应查询AgentManagerV2以获取可用智能体及其当前状态。

**REQ-OE-005: Concurrency Management** \[High\]

REQ-OE-005: 并发管理 \[高\]

The orchestration engine shall manage concurrent operations across multiple agents, teams, and workflows. It shall implement deadlock prevention, resource contention resolution, and fair scheduling algorithms.

编排引擎应管理跨多个智能体、团队和工作流的并发操作。它应实现死锁预防、资源争用解决和公平调度算法。

**REQ-OE-006: Error Recovery** \[High\]

REQ-OE-006: 错误恢复 \[高\]

The orchestration engine shall implement automatic error recovery including retry logic, alternative provider selection, workflow rollback, and state restoration. Recovery actions shall be logged and reported to the monitoring subsystem.

编排引擎应实现自动错误恢复，包括重试逻辑、备用提供者选择、工作流回滚和状态恢复。恢复操作应被记录并报告给监控子系统。

**REQ-OE-007: Event-Driven Architecture** \[High\]

REQ-OE-007: 事件驱动架构 \[高\]

The orchestration engine shall implement an event-driven architecture where state changes, task completions, and external triggers generate events that are processed by registered handlers. Events shall support both synchronous and asynchronous processing.

编排引擎应实现事件驱动架构，其中状态变更、任务完成和外部触发器生成由注册处理程序处理的事件。事件应支持同步和异步处理。

**REQ-OE-008: Workflow Orchestration** \[High\]

REQ-OE-008: 工作流编排 \[高\]

The orchestration engine shall manage the execution of iFlows and workflows including step sequencing, parallel branch management, conditional evaluation, and data flow between steps.

编排引擎应管理iFlows和工作流的执行，包括步骤排序、并行分支管理、条件评估和步骤之间的数据流。

**REQ-OE-009: Priority Queue Management** \[High\]

REQ-OE-009: 优先级队列管理 \[高\]

The orchestration engine shall maintain priority queues for pending tasks with configurable priority levels (critical, high, medium, low). Priority shall be dynamically adjustable based on deadlines and dependencies.

编排引擎应维护待处理任务的优先级队列，具有可配置的优先级（关键、高、中、低）。优先级应根据截止日期和依赖关系动态调整。

**REQ-OE-010: Deadlock Detection** \[High\]

REQ-OE-010: 死锁检测 \[高\]

The orchestration engine shall implement deadlock detection for circular dependencies between agents, tasks, and resources. Detected deadlocks shall be automatically resolved or escalated to human operators.

编排引擎应实现智能体、任务和资源之间循环依赖的死锁检测。检测到的死锁应自动解决或升级给人工操作员。

**REQ-OE-011: Load Balancing** \[High\]

REQ-OE-011: 负载均衡 \[高\]

The orchestration engine shall implement load balancing across providers and agents to optimize throughput and minimize latency. Load balancing strategies shall include round-robin, least-loaded, and cost-optimized approaches.

编排引擎应实现跨提供者和智能体的负载均衡，以优化吞吐量并最小化延迟。负载均衡策略应包括轮询、最少负载和成本优化的方法。

**REQ-OE-012: State Management** \[High\]

REQ-OE-012: 状态管理 \[高\]

The orchestration engine shall maintain a consistent global state model tracking all active agents, teams, workflows, sessions, and tasks. State changes shall be persisted to prevent data loss during system restarts.

编排引擎应维护一致的全局状态模型，跟踪所有活跃智能体、团队、工作流、会话和任务。状态变更应被持久化以防止系统重启期间数据丢失。

**REQ-OE-013: Timeout Management** \[High\]

REQ-OE-013: 超时管理 \[高\]

The orchestration engine shall implement configurable timeout policies for all operations including agent responses, workflow steps, provider API calls, and task execution. Timeouts shall trigger appropriate error handling and recovery procedures.

编排引擎应为所有操作实现可配置的超时策略，包括智能体响应、工作流步骤、提供者API调用和任务执行。超时应触发适当的错误处理和恢复程序。

**REQ-OE-014: Graceful Shutdown** \[High\]

REQ-OE-014: 优雅关闭 \[高\]

The orchestration engine shall support graceful shutdown that completes in-progress tasks, persists state, notifies active agents, and releases resources in an orderly manner. Forced shutdown shall be available as a last resort.

编排引擎应支持优雅关闭，完成进行中的任务、持久化状态、通知活跃智能体并以有序方式释放资源。强制关闭应作为最后手段可用。

**REQ-OE-015: Orchestration Metrics** \[High\]

REQ-OE-015: 编排指标 \[高\]

The orchestration engine shall expose comprehensive metrics including queue depths, processing throughput, agent utilization rates, error recovery statistics, and governance policy violation counts.

编排引擎应暴露全面的指标，包括队列深度、处理吞吐量、智能体利用率、错误恢复统计和治理策略违规计数。

**4.7 Knowledge Base / Brains / 知识库 (REQ-KB-001 to REQ-KB-010)**

**REQ-KB-001: Knowledge Storage** \[High\]

REQ-KB-001: 知识存储 \[高\]

The system shall store knowledge entries in Obsidian-compatible Markdown format within configured vault directories. Each knowledge entry shall include metadata in YAML frontmatter for indexing and categorization.

系统应以Obsidian兼容的Markdown格式在配置的仓库目录中存储知识条目。每个知识条目应在YAML前置元数据中包含元数据，用于索引和分类。

**REQ-KB-002: Knowledge Retrieval** \[High\]

REQ-KB-002: 知识检索 \[高\]

The system shall provide semantic search capabilities for knowledge retrieval using keyword matching, tag filtering, and content similarity scoring. Search results shall be ranked by relevance and displayed with contextual excerpts.

系统应提供知识检索的语义搜索能力，使用关键词匹配、标签过滤和内容相似度评分。搜索结果应按相关性排序并显示上下文摘录。

**REQ-KB-003: Knowledge Categorization** \[High\]

REQ-KB-003: 知识分类 \[高\]

The system shall support hierarchical categorization of knowledge entries using folders, tags, and custom taxonomies. Categories shall be navigable through both the AgentForge interface and Obsidian.

系统应支持使用文件夹、标签和自定义分类法对知识条目进行分层分类。类别应可通过AgentForge界面和Obsidian导航。

**REQ-KB-004: Knowledge Auto-Extraction** \[High\]

REQ-KB-004: 知识自动提取 \[高\]

The system shall automatically extract and store knowledge from agent interactions, workflow outputs, and conversation summaries. Extracted knowledge shall be reviewed and approved before being added to the knowledge base.

系统应从智能体交互、工作流输出和对话摘要中自动提取和存储知识。提取的知识在添加到知识库之前应经过审查和批准。

**REQ-KB-005: Knowledge Versioning** \[High\]

REQ-KB-005: 知识版本控制 \[高\]

The system shall maintain version history for knowledge entries, tracking all modifications with timestamps and author information. Users shall be able to view diffs between versions and restore previous versions.

系统应维护知识条目的版本历史，跟踪所有修改的时间戳和作者信息。用户应能够查看版本之间的差异并恢复以前的版本。

**REQ-KB-006: Knowledge Linking** \[High\]

REQ-KB-006: 知识链接 \[高\]

The system shall support bidirectional linking between knowledge entries using Obsidian-style \[\[wiki links\]\]. The system shall maintain link integrity when entries are renamed, moved, or deleted.

系统应支持使用Obsidian风格的\[\[wiki链接\]\]在知识条目之间进行双向链接。当条目被重命名、移动或删除时，系统应维护链接完整性。

**REQ-KB-007: Knowledge Graph Visualization** \[High\]

REQ-KB-007: 知识图谱可视化 \[高\]

The system shall provide a knowledge graph visualization showing relationships between knowledge entries, agents, and workflows. The visualization shall be interactive, allowing users to navigate and explore connections. A minimap shall be provided in the bottom-right corner to assist with navigation in complex graphs, similar to the iFlow builder.

系统应提供知识图谱可视化，显示知识条目、智能体和工作流之间的关系。可视化应是交互式的，允许用户导航和探索连接。右下角应提供一个迷你图（类似于iFlow构建器），以协助在复杂的图谱中导航。

**REQ-KB-008: Knowledge Access Control** \[High\]

REQ-KB-008: 知识访问控制 \[高\]

The system shall implement access control for knowledge entries based on user roles and team memberships. Access levels shall include read, write, approve, and administer permissions.

系统应根据用户角色和团队成员资格实现知识条目的访问控制。访问级别应包括读取、写入、批准和管理权限。

**REQ-KB-009: Knowledge Synchronization** \[High\]

REQ-KB-009: 知识同步 \[高\]

The system shall support bi-directional synchronization with Obsidian vaults. Changes made in AgentForge shall be reflected in Obsidian and vice versa. Synchronization conflicts shall be detected and resolved through configurable strategies.

系统应支持与Obsidian仓库的双向同步。在AgentForge中所做的更改应反映在Obsidian中，反之亦然。同步冲突应通过可配置的策略检测和解决。

**REQ-KB-010: Knowledge Templates** \[High\]

REQ-KB-010: 知识模板 \[高\]

The system shall provide knowledge entry templates for common types such as procedures, reference documents, meeting notes, decision records, and agent briefings. Users shall be able to create custom templates.

系统应为常见类型提供知识条目模板，如程序、参考文档、会议记录、决策记录和智能体简报。用户应能够创建自定义模板。

**4.8 MCP Tools / MCP工具 (REQ-MCP-001 to REQ-MCP-010)**

**REQ-MCP-001: MCP SDK Integration** \[High\]

REQ-MCP-001: MCP SDK集成 \[高\]

The system shall integrate with the \@modelcontextprotocol/sdk package for standardized communication between agents and external tools. Integration shall support both client and server roles.

系统应与\@modelcontextprotocol/sdk包集成，用于智能体和外部工具之间的标准化通信。集成应支持客户端和服务器角色。

**REQ-MCP-002: MCP Tool Registration** \[High\]

REQ-MCP-002: MCP工具注册 \[高\]

The system shall provide a tool registration interface where MCP tools can be discovered, registered, and configured. Registered tools shall be available to agents based on their permissions and capabilities.

系统应提供工具注册界面，可以在此发现、注册和配置MCP工具。注册的工具应根据智能体的权限和能力提供给智能体使用。

**REQ-MCP-003: MCP Tool Execution** \[High\]

REQ-MCP-003: MCP工具执行 \[高\]

The system shall execute MCP tool calls on behalf of agents, handling the complete request-response lifecycle including parameter validation, execution, result formatting, and error handling.

系统应代表智能体执行MCP工具调用，处理完整的请求-响应生命周期，包括参数验证、执行、结果格式化和错误处理。

**REQ-MCP-004: Team MCP Tools** \[High\]

REQ-MCP-004: 团队MCP工具 \[高\]

The system shall provide 5 specialized MCP tools for team operations: task creation, task claiming, team messaging, status reporting, and knowledge sharing. These tools shall be available to all team members.

系统应为团队操作提供5个专门的MCP工具：任务创建、任务声明、团队消息传递、状态报告和知识共享。这些工具应可供所有团队成员使用。

**REQ-MCP-005: MCP Tool Permissions** \[High\]

REQ-MCP-005: MCP工具权限 \[高\]

The system shall implement fine-grained permission control for MCP tool access. Permissions shall be configurable per agent, per team, and per tool, supporting allow, deny, and require-approval policies.

系统应为MCP工具访问实现细粒度的权限控制。权限应可按智能体、按团队和按工具配置，支持允许、拒绝和需要批准的策略。

**REQ-MCP-006: MCP Tool Monitoring** \[High\]

REQ-MCP-006: MCP工具监控 \[高\]

The system shall monitor MCP tool execution including invocation counts, execution times, success rates, and error patterns. Monitoring data shall be available through the observability dashboard.

系统应监控MCP工具执行，包括调用次数、执行时间、成功率和错误模式。监控数据应可通过可观测性仪表板获取。

**REQ-MCP-007: MCP Tool Sandbox** \[High\]

REQ-MCP-007: MCP工具沙箱 \[高\]

The system shall provide a sandboxed execution environment for MCP tools that require isolation. Sandboxed tools shall have restricted access to system resources and shall be subject to resource limits.

系统应为需要隔离的MCP工具提供沙箱执行环境。沙箱工具对系统资源的访问应受到限制，并应遵守资源限制。

**REQ-MCP-008: Custom MCP Tool Development** \[High\]

REQ-MCP-008: 自定义MCP工具开发 \[高\]

The system shall support development and registration of custom MCP tools through a plugin architecture. Custom tools shall be developed using a documented API and shall undergo security review before deployment.

系统应支持通过插件架构开发和注册自定义MCP工具。自定义工具应使用文档化的API开发，并在部署前经过安全审查。

**REQ-MCP-009: MCP Tool Versioning** \[High\]

REQ-MCP-009: MCP工具版本控制 \[高\]

The system shall track version information for registered MCP tools. When a tool is updated, the system shall maintain backward compatibility or provide migration paths for dependent agents and workflows.

系统应跟踪注册MCP工具的版本信息。当工具更新时，系统应保持向后兼容性或为依赖的智能体和工作流提供迁移路径。

**REQ-MCP-010: MCP Tool Rate Limiting** \[High\]

REQ-MCP-010: MCP工具速率限制 \[高\]

The system shall implement rate limiting for MCP tool invocations to prevent abuse and ensure fair resource allocation. Rate limits shall be configurable per tool, per agent, and per time window.

系统应为MCP工具调用实现速率限制，以防止滥用并确保公平的资源分配。速率限制应可按工具、按智能体和按时间窗口配置。

**4.9 Document Generation / 文档生成 (REQ-DG-001 to REQ-DG-010)**

**REQ-DG-001: Report Generation** \[High\]

REQ-DG-001: 报告生成 \[高\]

The system shall automatically generate reports from agent activities, workflow outputs, and team performance data. Reports shall support multiple formats including Markdown, PDF, HTML, and DOCX.

系统应从智能体活动、工作流输出和团队性能数据自动生成报告。报告应支持多种格式，包括Markdown、PDF、HTML和DOCX。

**REQ-DG-002: Documentation Templates** \[High\]

REQ-DG-002: 文档模板 \[高\]

The system shall provide a library of document templates for common documentation types including API documentation, user guides, technical specifications, and meeting minutes.

系统应为常见文档类型提供文档模板库，包括API文档、用户指南、技术规范和会议记录。

**REQ-DG-003: Summary Generation** \[High\]

REQ-DG-003: 摘要生成 \[高\]

The system shall generate concise summaries of long conversations, workflow executions, and team activities. Summaries shall preserve key decisions, action items, and important context.

系统应生成长对话、工作流执行和团队活动的简洁摘要。摘要应保留关键决策、行动项和重要上下文。

**REQ-DG-004: Code Documentation** \[High\]

REQ-DG-004: 代码文档生成 \[高\]

The system shall generate documentation from code artifacts including inline comments, function descriptions, class hierarchies, and usage examples. Documentation shall follow configurable style guidelines.

系统应从代码工件生成文档，包括内联注释、函数描述、类层次结构和使用示例。文档应遵循可配置的风格指南。

**REQ-DG-005: Knowledge Document Export** \[High\]

REQ-DG-005: 知识文档导出 \[高\]

The system shall support exporting knowledge base entries as formatted documents. Exported documents shall preserve formatting, links, and metadata from the original Markdown entries.

系统应支持将知识库条目导出为格式化文档。导出的文档应保留原始Markdown条目的格式、链接和元数据。

**REQ-DG-006: Automated Documentation Updates** \[High\]

REQ-DG-006: 自动化文档更新 \[高\]

The system shall support automated documentation updates triggered by code changes, workflow modifications, or configuration updates. Update rules shall be configurable per documentation type.

系统应支持由代码更改、工作流修改或配置更新触发的自动化文档更新。更新规则应可按文档类型配置。

**REQ-DG-007: Collaborative Editing** \[High\]

REQ-DG-007: 协作编辑 \[高\]

The system shall support collaborative document editing where multiple agents can contribute to the same document. The system shall handle concurrent edits, conflict resolution, and version merging.

系统应支持协作文档编辑，多个智能体可以贡献到同一文档。系统应处理并发编辑、冲突解决和版本合并。

**REQ-DG-008: Document Review Workflow** \[High\]

REQ-DG-008: 文档审查工作流 \[高\]

The system shall support document review workflows where generated documents are routed to human reviewers for approval. Review workflows shall support comments, annotations, and revision tracking.

系统应支持文档审查工作流，生成的文档路由给人工审查者审批。审查工作流应支持评论、注释和修订跟踪。

**REQ-DG-009: Multi-Language Support** \[High\]

REQ-DG-009: 多语言支持 \[高\]

The system shall support generating documents in multiple languages. Language selection shall be configurable per document type and per audience. Translation quality shall be monitored and feedback collected for improvement.

系统应支持生成多语言文档。语言选择应可按文档类型和受众配置。翻译质量应被监控并收集反馈以用于改进。

**REQ-DG-010: Document Storage & Organization** \[High\]

REQ-DG-010: 文档存储与组织 \[高\]

The system shall provide organized document storage with configurable directory structures, naming conventions, and metadata tagging. Documents shall be searchable and retrievable through the knowledge base system.

系统应提供有组织的文档存储，具有可配置的目录结构、命名约定和元数据标记。文档应可通过知识库系统搜索和检索。

**4.10 Token Management / Token管理 (REQ-TK-001 to REQ-TK-010)**

**REQ-TK-001: Token Usage Tracking** \[High\]

REQ-TK-001: Token使用跟踪 \[高\]

The system shall track token usage across all providers, agents, sessions, and workflows. Tracking shall include input tokens, output tokens, and total tokens with real-time updates.

系统应跟踪跨所有提供者、智能体、会话和工作流的Token使用。跟踪应包括输入Token、输出Token和总Token，具有实时更新。

**REQ-TK-002: Token Budget Allocation** \[High\]

REQ-TK-002: Token预算分配 \[高\]

The system shall support configurable token budget allocation at the system, team, agent, and session levels. Budget allocations shall enforce hard limits with configurable alerting thresholds.

系统应支持在系统、团队、智能体和会话级别的可配置Token预算分配。预算分配应执行硬限制，具有可配置的告警阈值。

**REQ-TK-003: Context Window Management** \[High\]

REQ-TK-003: 上下文窗口管理 \[高\]

The system shall manage context windows for each session, ensuring that conversations remain within provider-specific limits. The system shall implement automatic context compression, summarization, and pruning strategies.

系统应管理每个会话的上下文窗口，确保对话保持在提供者特定的限制内。系统应实现自动上下文压缩、摘要和修剪策略。

**REQ-TK-004: Token Cost Estimation** \[High\]

REQ-TK-004: Token成本估算 \[高\]

The system shall estimate token costs based on provider-specific pricing models. Cost estimation shall be displayed in real-time and included in budget planning and reporting.

系统应根据提供者特定的定价模型估算Token成本。成本估算应实时显示并包含在预算规划和报告中。

**REQ-TK-005: Token Optimization** \[High\]

REQ-TK-005: Token优化 \[高\]

The system shall implement token optimization strategies including prompt compression, response caching, context deduplication, and intelligent context selection to minimize token usage while maintaining quality.

系统应实现Token优化策略，包括提示压缩、响应缓存、上下文去重和智能上下文选择，以最小化Token使用同时保持质量。

**REQ-TK-006: Token Quota Enforcement** \[High\]

REQ-TK-006: Token配额执行 \[高\]

The system shall enforce token quotas at multiple levels. When a quota is reached, the system shall either block further requests, switch to a lower-cost provider, or reduce response quality based on configured policies.

系统应在多个级别执行Token配额。当达到配额时，系统应根据配置的策略阻止进一步请求、切换到低成本提供者或降低响应质量。

**REQ-TK-007: Token Usage Reporting** \[High\]

REQ-TK-007: Token使用报告 \[高\]

The system shall generate comprehensive token usage reports including per-provider breakdowns, trend analysis, cost projections, and optimization recommendations. Reports shall be exportable in multiple formats.

系统应生成全面的Token使用报告，包括按提供者分解、趋势分析、成本预测和优化建议。报告应可导出为多种格式。

**REQ-TK-008: Token Pooling** \[High\]

REQ-TK-008: Token池化 \[高\]

The system shall support token pooling where a shared token budget is distributed across multiple agents or teams. Pooling shall support priority-based allocation and dynamic rebalancing.

系统应支持Token池化，其中共享Token预算在多个智能体或团队之间分配。池化应支持基于优先级的分配和动态重新平衡。

**REQ-TK-009: Historical Token Analytics** \[High\]

REQ-TK-009: 历史Token分析 \[高\]

The system shall maintain historical token usage data for trend analysis, anomaly detection, and capacity planning. Historical data shall be retained for a configurable period and aggregated at multiple time granularities.

系统应维护历史Token使用数据，用于趋势分析、异常检测和容量规划。历史数据应保留可配置的期限并以多个时间粒度聚合。

**REQ-TK-010: Token Alert System** \[High\]

REQ-TK-010: Token告警系统 \[高\]

The system shall implement an alert system for token usage that notifies administrators when usage approaches or exceeds configured thresholds. Alerts shall support multiple notification channels including in-app, email, and webhook.

系统应为Token使用实现告警系统，当使用接近或超过配置的阈值时通知管理员。告警应支持多种通知渠道，包括应用内、电子邮件和webhook。

**4.11 Security & Access Control / 安全与访问控制 (REQ-SA-001 to REQ-SA-015)**

**REQ-SA-001: Role-Based Access Control** \[High\]

REQ-SA-001: 基于角色的访问控制 \[高\]

The system shall implement role-based access control (RBAC) with configurable roles and permissions. Roles shall be hierarchical and support inheritance. Default roles shall include Administrator, Operator, Developer, Analyst, and Viewer.

系统应实现基于角色的访问控制（RBAC），具有可配置的角色和权限。角色应分层并支持继承。默认角色应包括管理员、操作员、开发者、分析师和查看者。

**REQ-SA-002: API Key Management** \[High\]

REQ-SA-002: API密钥管理 \[高\]

The system shall provide secure storage and management of API keys for all configured AI providers. API keys shall be encrypted at rest, masked in the user interface, and never logged or exposed in error messages.

系统应为所有配置的AI提供者提供API密钥的安全存储和管理。API密钥应在静态时加密、在用户界面中遮蔽，并且绝不应被记录或在错误消息中暴露。

**REQ-SA-003: Data Encryption** \[High\]

REQ-SA-003: 数据加密 \[高\]

The system shall encrypt sensitive data at rest using AES-256 encryption and in transit using TLS 1.3. Encryption keys shall be managed through a secure key management system with automatic key rotation.

系统应使用AES-256加密对静态敏感数据进行加密，使用TLS 1.3对传输中的数据进行加密。加密密钥应通过安全的密钥管理系统管理，具有自动密钥轮换。

**REQ-SA-004: Audit Logging** \[High\]

REQ-SA-004: 审计日志 \[高\]

The system shall maintain comprehensive audit logs of all security-relevant events including authentication attempts, access control decisions, configuration changes, and data access. Audit logs shall be immutable and retained for a configurable period.

系统应维护所有安全相关事件的全面审计日志，包括身份验证尝试、访问控制决策、配置更改和数据访问。审计日志应不可变并保留可配置的期限。

**REQ-SA-005: Authentication** \[High\]

REQ-SA-005: 身份验证 \[高\]

The system shall support user authentication through multiple methods including local password authentication, LDAP/Active Directory integration, and single sign-on (SSO) via SAML 2.0 or OpenID Connect.

系统应支持通过多种方法进行用户身份验证，包括本地密码身份验证、LDAP/Active Directory集成以及通过SAML 2.0或OpenID Connect的单点登录（SSO）。

**REQ-SA-006: Session Security** \[High\]

REQ-SA-006: 会话安全 \[高\]

The system shall implement secure session management with configurable session timeouts, automatic logout on inactivity, session encryption, and protection against session fixation and hijacking attacks.

系统应实现安全的会话管理，具有可配置的会话超时、不活动时自动注销、会话加密以及防止会话固定和劫持攻击。

**REQ-SA-007: Input Validation** \[High\]

REQ-SA-007: 输入验证 \[高\]

The system shall validate and sanitize all user inputs to prevent injection attacks including SQL injection, cross-site scripting (XSS), and prompt injection attacks against AI providers.

系统应验证和清理所有用户输入，以防止注入攻击，包括SQL注入、跨站脚本（XSS）和针对AI提供者的提示注入攻击。

**REQ-SA-008: Prompt Injection Prevention** \[High\]

REQ-SA-008: 提示注入防护 \[高\]

The system shall implement prompt injection detection and prevention mechanisms to protect AI providers from malicious prompt manipulation. Detection shall use pattern matching, anomaly detection, and content filtering.

系统应实现提示注入检测和防护机制，以保护AI提供者免受恶意提示操纵。检测应使用模式匹配、异常检测和内容过滤。

**REQ-SA-009: Network Security** \[High\]

REQ-SA-009: 网络安全 \[高\]

The system shall implement network security measures including firewall rules, network segmentation, and secure communication channels. All inter-component communication shall use encrypted channels.

系统应实施网络安全措施，包括防火墙规则、网络分段和安全通信通道。所有组件间通信应使用加密通道。

**REQ-SA-010: Data Masking** \[High\]

REQ-SA-010: 数据脱敏 \[高\]

The system shall support data masking for sensitive information displayed in the user interface, logs, and reports. Masking rules shall be configurable per data type and per user role.

系统应支持在用户界面、日志和报告中显示的敏感信息的数据脱敏。脱敏规则应可按数据类型和按用户角色配置。

**REQ-SA-011: Compliance Reporting** \[High\]

REQ-SA-011: 合规报告 \[高\]

The system shall generate compliance reports for security audits including access logs, configuration snapshots, vulnerability assessments, and policy compliance status. Reports shall support common compliance frameworks.

系统应为安全审计生成合规报告，包括访问日志、配置快照、漏洞评估和策略合规状态。报告应支持常见的合规框架。

**REQ-SA-012: Secret Management** \[High\]

REQ-SA-012: 密钥管理 \[高\]

The system shall integrate with external secret management systems (e.g., HashiCorp Vault, AWS Secrets Manager) for secure storage and retrieval of sensitive configuration values including API keys, encryption keys, and credentials.

系统应与外部密钥管理系统（例如HashiCorp Vault、AWS Secrets Manager）集成，以安全存储和检索敏感配置值，包括API密钥、加密密钥和凭证。

**REQ-SA-013: Vulnerability Management** \[High\]

REQ-SA-013: 漏洞管理 \[高\]

The system shall support vulnerability scanning and management including dependency vulnerability checking, configuration vulnerability assessment, and security patch management. Critical vulnerabilities shall trigger automatic alerts.

系统应支持漏洞扫描和管理，包括依赖漏洞检查、配置漏洞评估和安全补丁管理。关键漏洞应触发自动告警。

**REQ-SA-014: Data Retention Policies** \[High\]

REQ-SA-014: 数据保留策略 \[高\]

The system shall enforce configurable data retention policies for conversation histories, audit logs, performance metrics, and knowledge base entries. Data shall be automatically purged or archived based on retention rules.

系统应执行对话历史、审计日志、性能指标和知识库条目的可配置数据保留策略。数据应根据保留规则自动清除或归档。

**REQ-SA-015: Security Incident Response** \[High\]

REQ-SA-015: 安全事件响应 \[高\]

The system shall implement security incident detection and response capabilities including anomaly detection, automated containment procedures, incident logging, and notification workflows. Security incidents shall be tracked through resolution.

系统应实现安全事件检测和响应能力，包括异常检测、自动化遏制程序、事件记录和通知工作流。安全事件应跟踪到解决。

**4.12 Monitoring & Observability / 监控与可观测性 (REQ-MO-001 to REQ-MO-010)**

**REQ-MO-001: Real-Time Dashboard** \[High\]

REQ-MO-001: 实时仪表板 \[高\]

The system shall provide a real-time monitoring dashboard displaying system health, agent status, provider connectivity, active workflows, token usage, and alert status. The dashboard shall support customizable views and widgets.

系统应提供实时监控仪表板，显示系统健康、智能体状态、提供者连接性、活跃工作流、Token使用和告警状态。仪表板应支持可自定义的视图和小部件。

**REQ-MO-002: Metrics Collection** \[High\]

REQ-MO-002: 指标收集 \[高\]

The system shall collect comprehensive metrics across all subsystems including orchestration engine performance, provider adapter latency, agent response times, workflow execution statistics, and resource utilization.

系统应在所有子系统中收集全面的指标，包括编排引擎性能、提供者适配器延迟、智能体响应时间、工作流执行统计和资源利用率。

**REQ-MO-003: Log Aggregation** \[High\]

REQ-MO-003: 日志聚合 \[高\]

The system shall aggregate logs from all components into a centralized logging system. Logs shall support structured formatting, severity levels, correlation IDs, and configurable retention policies.

系统应将所有组件的日志聚合到集中式日志系统中。日志应支持结构化格式、严重性级别、关联ID和可配置的保留策略。

**REQ-MO-004: Alert Management** \[High\]

REQ-MO-004: 告警管理 \[高\]

The system shall implement a comprehensive alert management system with configurable alert rules, severity levels, notification channels (in-app, email, webhook), escalation policies, and alert acknowledgment workflows.

系统应实现全面的告警管理系统，具有可配置的告警规则、严重性级别、通知渠道（应用内、电子邮件、webhook）、升级策略和告警确认工作流。

**REQ-MO-005: Distributed Tracing** \[High\]

REQ-MO-005: 分布式追踪 \[高\]

The system shall implement distributed tracing across all components to track request flows from user input through agent execution to provider API calls. Traces shall include timing information, dependency maps, and error context.

系统应在所有组件中实现分布式追踪，以跟踪从用户输入到智能体执行再到提供者API调用的请求流。追踪应包括时序信息、依赖图和错误上下文。

**REQ-MO-006: Health Check Endpoints** \[High\]

REQ-MO-006: 健康检查端点 \[高\]

The system shall expose health check endpoints for all major subsystems. Health checks shall verify component availability, database connectivity, provider reachability, and resource utilization levels.

系统应暴露所有主要子系统的健康检查端点。健康检查应验证组件可用性、数据库连接性、提供者可达性和资源利用率水平。

**REQ-MO-007: Performance Profiling** \[High\]

REQ-MO-007: 性能分析 \[高\]

The system shall provide performance profiling capabilities including CPU and memory usage tracking, response time histograms, throughput measurement, and bottleneck identification for all major operations.

系统应提供性能分析能力，包括CPU和内存使用跟踪、响应时间直方图、吞吐量测量和所有主要操作的瓶颈识别。

**REQ-MO-008: Anomaly Detection** \[High\]

REQ-MO-008: 异常检测 \[高\]

The system shall implement anomaly detection for system metrics, agent behavior, and provider responses. Detected anomalies shall trigger alerts and, where possible, automated remediation actions.

系统应为系统指标、智能体行为和提供者响应实现异常检测。检测到的异常应触发告警，并在可能时触发自动修复操作。

**REQ-MO-009: Custom Metrics** \[High\]

REQ-MO-009: 自定义指标 \[高\]

The system shall support defining and collecting custom metrics for application-specific monitoring needs. Custom metrics shall be integrated into the standard monitoring dashboard and alerting system.

系统应支持定义和收集自定义指标，用于应用程序特定的监控需求。自定义指标应集成到标准监控仪表板和告警系统中。

**REQ-MO-010: Observability Export** \[High\]

REQ-MO-010: 可观测性导出 \[高\]

The system shall support exporting observability data (metrics, logs, traces) to external monitoring systems such as Prometheus, Grafana, Elasticsearch, and Jaeger through standard protocols (OpenTelemetry, StatsD).

系统应支持通过标准协议（OpenTelemetry、StatsD）将可观测性数据（指标、日志、追踪）导出到外部监控系统，如Prometheus、Grafana、Elasticsearch和Jaeger。

**4.13 Obsidian Integration / Obsidian集成 (REQ-OI-001 to REQ-OI-005)**

**REQ-OI-001: Vault Configuration** \[High\]

REQ-OI-001: 仓库配置 \[高\]

The system shall allow users to configure one or more Obsidian vault directories for knowledge storage. Configuration shall include vault path, synchronization settings, and conflict resolution preferences.

系统应允许用户配置一个或多个Obsidian仓库目录用于知识存储。配置应包括仓库路径、同步设置和冲突解决偏好。

**REQ-OI-002: Markdown Compatibility** \[High\]

REQ-OI-002: Markdown兼容性 \[高\]

The system shall generate and manage Markdown files that are fully compatible with Obsidian\'s format including frontmatter, wiki links, tags, callouts, and embedded content. Files shall be readable and editable in both AgentForge and Obsidian.

系统应生成和管理与Obsidian格式完全兼容的Markdown文件，包括前置元数据、wiki链接、标签、标注和嵌入内容。文件应在AgentForge和Obsidian中均可读取和编辑。

**REQ-OI-003: Bi-Directional Sync** \[High\]

REQ-OI-003: 双向同步 \[高\]

The system shall implement bi-directional synchronization with Obsidian vaults, detecting changes made in either AgentForge or Obsidian and propagating them. The system shall handle sync conflicts using configurable resolution strategies.

系统应实现与Obsidian仓库的双向同步，检测在AgentForge或Obsidian中进行的更改并传播它们。系统应使用可配置的解决策略处理同步冲突。

**REQ-OI-004: Graph View Integration** \[High\]

REQ-OI-004: 图视图集成 \[高\]

The system shall leverage Obsidian\'s graph view capabilities by maintaining proper link structure and metadata in knowledge entries. The system shall also provide its own graph visualization for knowledge exploration within AgentForge.

系统应通过在知识条目中维护适当的链接结构和元数据来利用Obsidian的图视图能力。系统还应在AgentForge内提供自己的图可视化用于知识探索。

**REQ-OI-005: Plugin Ecosystem Compatibility** \[High\]

REQ-OI-005: 插件生态系统兼容性 \[高\]

The system shall ensure compatibility with common Obsidian plugins by maintaining standard Markdown structures and metadata formats. The system shall document any known compatibility issues and workarounds.

系统应通过维护标准Markdown结构和元数据格式来确保与常见Obsidian插件的兼容性。系统应记录任何已知的兼容性问题和解决方法。

**5. Non-Functional Requirements / 非功能需求**

**5.1 Performance Requirements / 性能需求**

-   The system shall support at least 50 concurrent agent sessions with response latency not exceeding 2 seconds for local operations and not exceeding the provider\'s API response time plus 500ms for provider-dependent operations.

-   系统应支持至少50个并发智能体会话，本地操作的响应延迟不超过2秒，提供者相关操作的响应延迟不超过提供者API响应时间加500毫秒。

-   The orchestration engine shall process at least 100 workflow steps per second under normal load conditions.

-   编排引擎在正常负载条件下应每秒处理至少100个工作流步骤。

-   The user interface shall render all views within 500ms of user interaction, maintaining a minimum of 30 FPS during active operations.

-   用户界面应在用户交互后500毫秒内渲染所有视图，在活跃操作期间保持至少30 FPS。

-   Database operations shall complete within 100ms for standard queries and within 1 second for complex aggregation queries.

-   数据库操作应在100毫秒内完成标准查询，在1秒内完成复杂聚合查询。

-   The system shall handle up to 1,000 messages per second through the TeamBus communication infrastructure.

-   系统应通过TeamBus通信基础设施每秒处理多达1,000条消息。

-   Token usage calculations and budget checks shall complete within 10ms to avoid adding latency to agent operations.

-   Token使用计算和预算检查应在10毫秒内完成，以避免增加智能体操作的延迟。

**5.2 Security Requirements / 安全需求**

-   All sensitive data shall be encrypted at rest using AES-256 and in transit using TLS 1.3.

-   所有敏感数据应在静态时使用AES-256加密，在传输中使用TLS 1.3加密。

-   The system shall implement defense-in-depth security architecture with multiple layers of protection.

-   系统应实现纵深防御安全架构，具有多层保护。

-   Authentication credentials shall never be stored in plain text or logged in any format.

-   身份验证凭证绝不应以明文存储或以任何格式记录。

-   The system shall be resistant to common OWASP Top 10 vulnerabilities including injection, broken authentication, and security misconfiguration.

-   系统应能抵抗常见的OWASP Top 10漏洞，包括注入、身份验证中断和安全配置错误。

-   Regular security audits and penetration testing shall be supported through comprehensive audit logging and security event tracking.

-   应通过全面的审计日志记录和安全事件跟踪支持定期安全审计和渗透测试。

**5.3 Reliability & Availability / 可靠性与可用性**

-   The system shall achieve 99.5% availability during business hours (excluding scheduled maintenance windows).

-   系统在工作时间应达到99.5%的可用性（不包括计划维护窗口）。

-   The system shall recover from failures without data loss. All state changes shall be persisted before acknowledgment.

-   系统应从故障中恢复而不丢失数据。所有状态变更应在确认之前持久化。

-   The system shall implement automatic failover for all critical components with recovery time objective (RTO) of less than 30 seconds.

-   系统应为所有关键组件实现自动故障转移，恢复时间目标（RTO）小于30秒。

-   The system shall handle provider outages gracefully by queuing requests, switching to fallback providers, and notifying users of service degradation.

-   系统应通过排队请求、切换到备用提供者和通知用户服务降级来优雅地处理提供者故障。

-   Mean time between failures (MTBF) shall exceed 720 hours for the core orchestration engine.

-   核心编排引擎的平均故障间隔时间（MTBF）应超过720小时。

**5.4 Scalability / 可扩展性**

-   The system shall support horizontal scaling of agent instances to handle increased workload.

-   系统应支持智能体实例的水平扩展以处理增加的工作负载。

-   The architecture shall support adding new provider adapters without modifying the core orchestration engine.

-   架构应支持添加新的提供者适配器而无需修改核心编排引擎。

-   The database schema shall support partitioning and sharding strategies for high-volume deployments.

-   数据库模式应支持高容量部署的分区和分片策略。

-   The system shall support up to 500 agents and 50 teams in a single deployment without performance degradation.

-   系统应在单个部署中支持多达500个智能体和50个团队，而不会出现性能下降。

**5.5 Usability / 可用性**

-   The user interface shall follow platform-native design guidelines for consistency with the desktop operating system.

-   用户界面应遵循平台原生设计指南，与桌面操作系统保持一致。

-   All user-facing text shall support internationalization (i18n) with English and Chinese as the initial supported languages.

-   所有面向用户的文本应支持国际化（i18n），英语和中文为初始支持的语言。

-   The system shall provide context-sensitive help, tooltips, and documentation accessible from within the application.

-   系统应提供上下文相关的帮助、工具提示和可从应用程序内访问的文档。

-   New users shall be able to create and configure their first agent within 15 minutes of initial application launch.

-   新用户应在首次启动应用程序后15分钟内能够创建和配置其第一个智能体。

-   The system shall provide keyboard shortcuts for all common operations and support accessibility features (screen reader compatibility, high contrast mode).

-   系统应为所有常见操作提供键盘快捷键，并支持辅助功能（屏幕阅读器兼容性、高对比度模式）。

**5.6 Maintainability / 可维护性**

-   The codebase shall follow consistent coding standards with comprehensive inline documentation.

-   代码库应遵循一致的编码标准，具有全面的内联文档。

-   The system shall achieve a minimum of 80% code coverage through automated unit and integration tests.

-   系统应通过自动化单元测试和集成测试达到至少80%的代码覆盖率。

-   The system shall support hot-reloading of configuration changes without requiring application restart.

-   系统应支持配置更改的热重载，而无需重新启动应用程序。

-   All system components shall expose health check endpoints and diagnostic information for troubleshooting.

-   所有系统组件应暴露健康检查端点和诊断信息以用于故障排除。

-   The system shall generate structured logs with correlation IDs for end-to-end request tracing.

-   系统应生成带有关联ID的结构化日志，用于端到端请求追踪。

**5.7 Portability / 可移植性**

-   The application shall be packaged for Windows, macOS, and Linux desktop platforms.

-   应用程序应打包为Windows、macOS和Linux桌面平台版本。

-   The system shall not depend on platform-specific APIs except where necessary for desktop integration (file system, notifications, system tray).

-   系统不应依赖平台特定的API，除非桌面集成（文件系统、通知、系统托盘）所必需。

-   Configuration files and data storage shall use cross-platform compatible formats.

-   配置文件和数据存储应使用跨平台兼容的格式。

-   The system shall support both light and dark theme modes following the operating system\'s theme preference.

-   系统应支持浅色和深色主题模式，遵循操作系统的主题偏好。

**6. Interface Requirements / 接口需求**

**6.1 User Interfaces / 用户界面**

The user interface shall be built using GPUI components from the longbridge/gpui-component library, providing a native desktop experience with GPU-accelerated rendering. The interface shall consist of the following major views:

用户界面应使用longbridge/gpui-component库的GPUI组件构建，提供具有GPU加速渲染的原生桌面体验。界面应包含以下主要视图：

-   Agent Management Console: A dashboard for creating, configuring, and monitoring individual agents with real-time status indicators.

-   智能体管理控制台：用于创建、配置和监控单个智能体的仪表板，具有实时状态指示器。

-   Team Workspace: A collaborative workspace showing team composition, shared task boards, communication channels, and performance metrics.

-   团队工作区：显示团队组成、共享任务板、通信通道和性能指标的协作工作区。

-   Workflow Designer: A visual editor for creating and modifying iFlows with drag-and-drop step composition and real-time validation.

-   工作流设计器：用于创建和修改iFlows的可视化编辑器，具有拖放式步骤组合和实时验证。

-   Session Manager: An interface for managing active conversations with agents, including conversation history, context management, and session controls.

-   会话管理器：用于管理与智能体活跃对话的界面，包括对话历史、上下文管理和会话控制。

-   Knowledge Explorer: A browser for the knowledge base with search, categorization, and graph visualization capabilities.

-   知识浏览器：知识库浏览器，具有搜索、分类和图可视化能力。

-   Monitoring Dashboard: A real-time observability view with system metrics, alerts, and performance visualizations.

-   监控仪表板：具有系统指标、告警和性能可视化的实时可观测性视图。

-   Settings Panel: A configuration interface for system settings, provider configurations, security policies, and user preferences.

-   设置面板：用于系统设置、提供者配置、安全策略和用户偏好的配置界面。

**6.2 Hardware Interfaces / 硬件接口**

-   The application shall require a minimum of 8 GB RAM (16 GB recommended) for standard operation.

-   应用程序在标准操作下至少需要8 GB RAM（推荐16 GB）。

-   The application shall require a GPU that supports OpenGL 3.3 or later for GPUI rendering.

-   应用程序应需要支持OpenGL 3.3或更高版本的GPU用于GPUI渲染。

-   The application shall require a minimum of 2 GB free disk space for installation and local data storage.

-   应用程序应至少需要2 GB可用磁盘空间用于安装和本地数据存储。

-   The application shall support standard input devices (keyboard, mouse) and optionally touch input on supported devices.

-   应用程序应支持标准输入设备（键盘、鼠标），并在支持的设备上可选地支持触摸输入。

**6.3 Software Interfaces / 软件接口**

-   Provider APIs: Anthropic Claude API, Google Gemini API, OpenAI API --- for AI model inference.

-   提供者API：Anthropic Claude API、Google Gemini API、OpenAI API --- 用于AI模型推理。

-   MCP Protocol: \@modelcontextprotocol/sdk --- for tool integration and agent-tool communication.

-   MCP协议：\@modelcontextprotocol/sdk --- 用于工具集成和智能体-工具通信。

-   Database: SQLite via better-sqlite3 --- for data persistence with Repository pattern access.

-   数据库：通过better-sqlite3的SQLite --- 用于使用Repository模式访问的数据持久化。

-   File System: Standard OS file system APIs --- for document generation, knowledge storage, and configuration management.

-   文件系统：标准操作系统文件系统API --- 用于文档生成、知识存储和配置管理。

-   WebSocket: Native WebSocket implementation --- for AgentBridge real-time communication.

-   WebSocket：原生WebSocket实现 --- 用于AgentBridge实时通信。

-   Obsidian: File system integration with Obsidian vault format --- for knowledge base synchronization.

-   Obsidian：与Obsidian仓库格式的文件系统集成 --- 用于知识库同步。

**6.4 Communication Interfaces / 通信接口**

-   HTTPS/TLS 1.3: For all external API communications with AI providers.

-   HTTPS/TLS 1.3：用于与AI提供者的所有外部API通信。

-   WebSocket (WSS): For real-time agent communication via AgentBridge and TeamBus.

-   WebSocket（WSS）：用于通过AgentBridge和TeamBus的实时智能体通信。

-   Local IPC: For inter-process communication between the GPUI frontend and Node.js backend services.

-   本地IPC：用于GPUI前端和Node.js后端服务之间的进程间通信。

-   SQLite Protocol: For local database access via better-sqlite3.

-   SQLite协议：用于通过better-sqlite3的本地数据库访问。

-   File System Watcher: For detecting changes in Obsidian vaults and configuration files.

-   文件系统监视器：用于检测Obsidian仓库和配置文件中的更改。

**7. Data Requirements / 数据需求**

**7.1 Data Model / 数据模型**

The AgentForge data model is organized around the following core entities: Agents, Teams, Roles, Instances, Members, Tasks, Messages, Sessions, Workflows, Knowledge Entries, and Provider Configurations. Relationships between entities are maintained through foreign key references with referential integrity enforced at the application level.

AgentForge数据模型围绕以下核心实体组织：智能体、团队、角色、实例、成员、任务、消息、会话、工作流、知识条目和提供者配置。实体之间的关系通过外键引用维护，参照完整性在应用层强制执行。

The data model follows the Repository pattern for data access, with each entity type having a dedicated repository class that encapsulates all database operations. This pattern provides a clean separation between business logic and data access, facilitating testing and future database migration.

数据模型遵循Repository模式进行数据访问，每种实体类型都有专用的存储库类，封装所有数据库操作。这种模式提供了业务逻辑和数据访问之间的清晰分离，便于测试和未来的数据库迁移。

**7.2 Database Schema / 数据库模式**

The system uses SQLite as its primary database with the following core tables for team management:

系统使用SQLite作为主数据库，团队管理使用以下核心表：

  ------------------- -------------------------------------------- --------------------------------------------------------------------
  **Table / 表**      **Purpose / 用途**                           **Key Fields / 关键字段**
  teams               Stores team definitions and configurations   id, name, description, objectives, created\_at, updated\_at
  roles               Defines roles within teams                   id, team\_id, name, permissions, capabilities
  instances           Stores team instance configurations          id, team\_id, config, state, created\_at
  members             Manages team membership                      id, team\_id, instance\_id, agent\_id, role\_id, joined\_at
  tasks               Shared task list for team operations         id, team\_id, assignee\_id, status, priority, payload, claimed\_at
  messages            Persists team communication                  id, team\_id, sender\_id, recipient\_id, type, content, sent\_at
  agents              Stores agent configurations                  id, name, provider, system\_prompt, config, status
  sessions            Manages conversation contexts                id, agent\_id, user\_id, context, created\_at, updated\_at
  workflows           Stores iFlow definitions                     id, name, definition, version, created\_at
  knowledge           Knowledge base entries                       id, title, content, tags, category, vault\_path
  provider\_configs   Provider connection settings                 id, provider\_type, api\_key\_encrypted, endpoint, settings
  audit\_log          Security and operations audit trail          id, timestamp, user\_id, action, resource, details
  ------------------- -------------------------------------------- --------------------------------------------------------------------

**7.3 Data Storage / 数据存储**

-   Application data shall be stored in SQLite database files using the better-sqlite3 library with WAL (Write-Ahead Logging) mode for concurrent access.

-   应用程序数据应使用better-sqlite3库存储在SQLite数据库文件中，使用WAL（预写日志）模式进行并发访问。

-   Knowledge base content shall be stored as Markdown files in Obsidian-compatible vault directories.

-   知识库内容应以Markdown文件存储在Obsidian兼容的仓库目录中。

-   Generated documents shall be stored in a configurable output directory with organized subdirectories by type and date.

-   生成的文档应存储在可配置的输出目录中，按类型和日期组织子目录。

-   Configuration files shall be stored in JSON format in the application\'s configuration directory.

-   配置文件应以JSON格式存储在应用程序的配置目录中。

-   Temporary data and caches shall be stored in the operating system\'s standard temporary directory.

-   临时数据和缓存应存储在操作系统的标准临时目录中。

**7.4 Backup & Recovery / 备份与恢复**

-   The system shall support automated database backups at configurable intervals (minimum: daily).

-   系统应支持以可配置的间隔（最低：每日）进行自动数据库备份。

-   Backups shall include the SQLite database, configuration files, and knowledge base content.

-   备份应包括SQLite数据库、配置文件和知识库内容。

-   The system shall support point-in-time recovery from backups with configurable retention periods.

-   系统应支持从备份进行时间点恢复，具有可配置的保留期。

-   Backup integrity shall be verified automatically after creation using checksum validation.

-   备份完整性应在创建后使用校验和验证自动验证。

-   The system shall support both full and incremental backup strategies to optimize storage usage and backup time.

-   系统应支持完整和增量备份策略，以优化存储使用和备份时间。

-   Recovery procedures shall be documented and tested as part of the system\'s disaster recovery plan.

-   恢复程序应作为系统灾难恢复计划的一部分进行文档化和测试。

**8. Verification & Validation / 验证与确认**

This section defines the approach and criteria for verifying and validating that AgentForge meets its specified requirements.

本节定义了验证和确认AgentForge满足其规定需求的方法和标准。

**8.1 Verification Approach / 验证方法**

-   Code Reviews: All code changes shall undergo peer review before merging. Reviews shall verify adherence to coding standards, requirement traceability, and test coverage.

-   代码审查：所有代码更改在合并前应经过同行审查。审查应验证对编码标准的遵守、需求可追溯性和测试覆盖率。

-   Static Analysis: Automated static analysis tools shall be configured to detect code quality issues, security vulnerabilities, and potential bugs.

-   静态分析：应配置自动化静态分析工具来检测代码质量问题、安全漏洞和潜在错误。

-   Unit Testing: Each functional requirement shall have corresponding unit tests with a minimum of 80% code coverage.

-   单元测试：每个功能需求应有相应的单元测试，最低代码覆盖率为80%。

-   Integration Testing: End-to-end integration tests shall verify the interaction between all major subsystems including provider adapters, orchestration engine, and session management.

-   集成测试：端到端集成测试应验证所有主要子系统之间的交互，包括提供者适配器、编排引擎和会话管理。

**8.2 Validation Approach / 确认方法**

-   User Acceptance Testing (UAT): Representative users from each user characteristic category shall participate in UAT to validate that the system meets their operational needs.

-   用户验收测试（UAT）：每个用户特征类别的代表性用户应参与UAT，以验证系统满足其运营需求。

-   Performance Testing: Load testing shall validate that the system meets all performance requirements under expected and peak load conditions.

-   性能测试：负载测试应验证系统在预期和峰值负载条件下满足所有性能需求。

-   Security Testing: Penetration testing and vulnerability scanning shall validate that the system meets all security requirements and is resistant to common attack vectors.

-   安全测试：渗透测试和漏洞扫描应验证系统满足所有安全需求并能抵抗常见攻击向量。

-   Compatibility Testing: The application shall be tested on all supported platforms (Windows, macOS, Linux) to validate portability requirements.

-   兼容性测试：应用程序应在所有支持的平台（Windows、macOS、Linux）上测试，以验证可移植性需求。

**8.3 Traceability Matrix / 可追溯性矩阵**

A requirements traceability matrix shall be maintained linking each requirement ID to its corresponding design elements, source code modules, test cases, and validation results. The matrix shall be updated throughout the development lifecycle.

应维护需求可追溯性矩阵，将每个需求ID链接到其对应的设计元素、源代码模块、测试用例和确认结果。矩阵应在整个开发生命周期中更新。

**8.4 Acceptance Criteria / 验收标准**

  --------------------------------------------------- ----------------------------------------------------------------------
  **Criterion / 标准**                                **Threshold / 阈值**
  Functional Requirements Coverage / 功能需求覆盖率   100% of requirements implemented and tested / 100%的需求已实现并测试
  Unit Test Coverage / 单元测试覆盖率                 \>= 80% code coverage / \>= 80%代码覆盖率
  Performance Requirements / 性能需求                 All performance benchmarks met / 所有性能基准达标
  Security Assessment / 安全评估                      No critical or high vulnerabilities / 无关键或高危漏洞
  User Acceptance / 用户验收                          \>= 90% user satisfaction score / \>= 90%用户满意度评分
  Platform Compatibility / 平台兼容性                 All supported platforms validated / 所有支持的平台已验证
  Documentation Completeness / 文档完整性             All required documentation delivered / 所有要求的文档已交付
  --------------------------------------------------- ----------------------------------------------------------------------
