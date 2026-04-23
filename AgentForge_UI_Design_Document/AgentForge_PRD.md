**AgentForge**

Multi-AI Agent Orchestration Platform

多智能体编排平台

**Product Requirements Document (PRD)**

**产品需求文档**

Version 1.0 \| April 2026

版本 1.0 \| 2026年4月

Classification: Internal / Confidential

分级：内部 / 机密

**Table of Contents / 目录**

**1** Product Vision & Strategy / 产品愿景与策略

**2** Target User Personas / 目标用户画像

**3** User Stories & Use Cases / 用户故事与用例

**4** Functional Requirements / 功能需求

> 4.1 Agent Management / 智能体管理
>
> 4.2 Team Management / 团队管理
>
> 4.3 Communication System / 通信系统
>
> 4.4 Operating Modes / 运行模式
>
> 4.5 iFlows & Workflows / 智能工作流
>
> 4.6 Provider Adapter / 提供者适配器
>
> 4.7 Orchestration Engine / 编排引擎
>
> 4.8 Knowledge Base (Brains) / 知识库
>
> 4.9 MCP Tools / MCP工具
>
> 4.10 Document Generation / 文档生成
>
> 4.11 Token Management / Token管理
>
> 4.12 Security & Access Control / 安全与访问控制
>
> 4.13 Monitoring & Observability / 监控与可观测性

**5** Non-Functional Requirements / 非功能需求

**6** Data Model / 数据模型

**7** API Specifications / API规范

**8** Integration Requirements / 集成需求

**9** User Interface Requirements / 用户界面需求

**10** Release Phases & Roadmap / 发布阶段与路线图

**11** Success Metrics / 成功指标

**A** Appendix: Glossary / 附录：术语表

**1. Product Vision & Strategy / 产品愿景与策略**

**1.1 Vision Statement / 愿景声明**

AgentForge envisions a future where multiple AI agents, powered by diverse models and providers, collaborate seamlessly under human governance to tackle complex, multi-domain development and operational tasks. The platform serves as the central nervous system for AI-driven workflows, enabling organizations to harness the collective intelligence of Gemini, Codex, Claude, ChatGPT, and custom models through a unified, secure, and extensible orchestration layer.

*AgentForge描绘的未来是多个由不同模型和提供者驱动的AI智能体，在人类治理下无缝协作，共同处理复杂的多领域开发和运营任务。该平台作为AI驱动工作流的中枢神经系统，使组织能够通过统一、安全、可扩展的编排层，充分利用Gemini、Codex、Claude、ChatGPT及自定义模型的集体智慧。*

**1.2 Strategic Goals / 战略目标**

**Unified Multi-AI Orchestration**

Provide a single platform to manage, coordinate, and monitor multiple AI agents from different providers, eliminating tool fragmentation.

*统一多AI编排：提供单一平台来管理、协调和监控来自不同提供者的多个AI智能体，消除工具碎片化。*

**Governance-First Design**

Embed access control, audit logging, and security constraints at every layer of the system.

*治理优先设计：在系统的每一层嵌入访问控制、审计日志和安全约束。*

**Token Efficiency**

Maximize the value extracted from every token through intelligent context management, knowledge compression, and budget allocation.

*Token效率最大化：通过智能上下文管理、知识压缩和预算分配，从每个token中提取最大价值。*

**Extensibility**

Support custom providers, MCP tools, and workflow templates through a plugin-based architecture.

*可扩展性：通过基于插件的架构支持自定义提供者、MCP工具和工作流模板。*

**Developer Experience**

Provide intuitive visual interfaces, comprehensive documentation, and seamless integration with existing development workflows.

*开发者体验：提供直观的可视化界面、全面的文档以及与现有开发工作流的无缝集成。*

**1.3 Product Positioning / 产品定位**

AgentForge is positioned as the enterprise-grade operating system for multi-AI agent teams. Unlike single-model coding assistants or simple chatbot frameworks, AgentForge provides deep orchestration capabilities, cross-model communication, and governance controls required for production-grade AI deployments. It bridges the gap between individual AI tools and enterprise AI infrastructure.

*AgentForge定位为企业级多AI智能体团队的操作系统。与单一模型编码助手或简单的聊天机器人框架不同，AgentForge提供了生产级AI部署所需的深度编排能力、跨模型通信和治理控制。它弥补了单一AI工具与企业AI基础设施之间的差距。*

**1.4 Key Differentiators / 核心差异化因素**

-   Native multi-provider support with unified adapter layer (Claude Code, Codex, Gemini, iFlow, OpenCode, custom providers)

-   Three operating modes spanning from fully interactive to fully autonomous, with granular governance controls

-   Token-efficient knowledge management (Brains) with Obsidian integration and structured storage

-   DAG-based intelligent workflows (iFlows) with visual builder and template marketplace

-   Built-in MCP tool ecosystem with custom tool creation capabilities

-   Real-time agent communication visualization and system observability

-   *原生多提供者支持，具有统一适配器层（Claude Code、Codex、Gemini、iFlow、OpenCode、自定义提供者）*

-   *三种运行模式覆盖从完全交互到完全自主，具有精细的治理控制*

-   *Token高效的知识管理（Brains），支持Obsidian集成和结构化存储*

-   *基于DAG的智能工作流（iFlows），具有可视化构建器和模板市场*

-   *内置MCP工具生态系统，支持自定义工具创建*

-   *实时智能体通信可视化和系统可观测性*

**2. Target User Personas / 目标用户画像**

**Alex - AI Platform Engineer / Alex - AI平台工程师**

Alex is a senior software engineer responsible for integrating AI capabilities into the development workflow. They manage API keys, configure provider adapters, and ensure system reliability. Alex needs fine-grained control over token budgets, monitoring dashboards, and the ability to create custom MCP tools.

*Alex是一名负责将AI能力集成到开发工作流中的高级软件工程师。他们管理API密钥、配置提供者适配器，并确保系统可靠性。Alex需要对token预算的精细控制、监控仪表板以及创建自定义MCP工具的能力。*

**Goals / 目标**

1.  Configure and manage multiple AI provider connections

2.  *配置和管理多个AI提供者连接*

3.  Monitor token usage and optimize budgets

4.  *监控token使用并优化预算*

5.  Create custom MCP tools for domain-specific tasks

6.  *为特定领域任务创建自定义MCP工具*

7.  Set up security policies and access controls

8.  *设置安全策略和访问控制*

**Maya - Product Manager / Maya - 产品经理**

Maya leads cross-functional product teams and uses AgentForge to automate documentation generation, coordinate AI-assisted research, and manage multi-agent workflows for competitive analysis and feature planning. She values the supervision mode for maintaining oversight while leveraging AI productivity.

*Maya领导跨职能产品团队，使用AgentForge自动化文档生成、协调AI辅助研究，并管理用于竞争分析和功能规划的多智能体工作流。她看重监督模式，以在利用AI生产力的同时保持监督。*

**Goals / 目标**

9.  Create and manage AI agent teams for product research

10. *创建和管理用于产品研究的AI智能体团队*

11. Generate PRDs, SRS, and documentation automatically

12. *自动生成PRD、SRS和文档*

13. Monitor workflow progress and agent outputs

14. *监控工作流进度和智能体输出*

15. Configure governance policies for team operations

16. *为团队运营配置治理策略*

**Chen - DevOps & Security Lead / Chen - DevOps与安全负责人**

Chen is responsible for the security posture and operational reliability of AI systems within the organization. He needs comprehensive audit logging, role-based access control, API key rotation, and real-time monitoring of all agent activities to ensure compliance.

*Chen负责组织内AI系统的安全态势和运营可靠性。他需要全面的审计日志、基于角色的访问控制、API密钥轮换以及对所有智能体活动的实时监控。*

**Goals / 目标**

17. Implement and enforce role-based access control

18. *实施和执行基于角色的访问控制*

19. Review audit logs for compliance

20. *审查审计日志以确保合规*

21. Manage API key lifecycle and rotation

22. *管理API密钥生命周期和轮换*

23. Monitor system health and security events

24. *监控系统健康和安全事件*

**Sarah - Full-Stack Developer / Sarah - 全栈开发者**

Sarah is a full-stack developer who uses AgentForge to accelerate her development workflow. She creates specialized agent teams for frontend, backend, and testing tasks, uses iFlows for CI/CD automation, and leverages the knowledge base to maintain coding standards.

*Sarah是一名全栈开发者，使用AgentForge加速她的开发工作流。她为前端、后端和测试任务创建专业化的智能体团队，使用iFlows进行CI/CD自动化。*

**Goals / 目标**

25. Create specialized agent teams for different development tasks

26. *为不同的开发任务创建专业化智能体团队*

27. Build and reuse iFlow workflow templates

28. *构建和复用iFlow工作流模板*

29. Maintain project knowledge in the Brains system

30. *在Brains系统中维护项目知识*

31. Use autonomous mode for repetitive coding tasks

32. *对重复性编码任务使用自主模式*

**3. User Stories & Use Cases / 用户故事与用例**

**3.1 Epic: Multi-Agent Team Creation / 史诗：多智能体团队创建**

**US-001**

As a platform engineer, I want to create a team of AI agents with specialized roles (frontend, backend, testing, DevOps) so that they can collaboratively develop a full-stack application.

*作为平台工程师，我希望创建一个具有专业角色的AI智能体团队，以便它们可以协作开发全栈应用程序。*

**US-002**

As a product manager, I want to assign different AI providers to different agents based on their strengths (e.g., Claude for architecture, GPT for content generation).

*作为产品经理，我希望根据不同智能体的优势分配不同的AI提供者。*

**US-003**

As a developer, I want to define agent responsibilities, knowledge domains, and skill sets so that agents have clear boundaries and specialized expertise.

*作为开发者，我希望定义智能体的职责、知识领域和技能集。*

**3.2 Epic: Workflow Automation / 史诗：工作流自动化**

**US-004**

As a DevOps engineer, I want to create DAG-based iFlows that automate code review, testing, and deployment pipelines across multiple AI agents.

*作为DevOps工程师，我希望创建基于DAG的iFlows，自动化跨多个AI智能体的代码审查、测试和部署流水线。*

**US-005**

As a developer, I want to use a visual workflow builder to design, test, and debug iFlows without writing code.

*作为开发者，我希望使用可视化工作流构建器来设计、测试和调试iFlows。*

**US-006**

As a team lead, I want to save and share iFlow templates across teams so that best practices are standardized and reusable.

*作为团队负责人，我希望在团队间保存和共享iFlow模板。*

**3.3 Epic: Knowledge Management / 史诗：知识管理**

**US-007**

As a developer, I want agents to automatically capture and store architectural decisions, code patterns, and lessons learned in a structured knowledge base.

*作为开发者，我希望智能体自动捕获并存储架构决策和代码模式到结构化知识库中。*

**US-008**

As a platform engineer, I want to integrate AgentForge\'s knowledge base with Obsidian so that knowledge is accessible in both systems.

*作为平台工程师，我希望将AgentForge的知识库与Obsidian集成。*

**US-009**

As a team lead, I want to manage token budgets for knowledge base operations to ensure cost-effective knowledge retrieval.

*作为团队负责人，我希望管理知识库操作的token预算。*

**3.4 Epic: Governance & Security / 史诗：治理与安全**

**US-010**

As a security lead, I want to define role-based access control policies that restrict which agents can access sensitive data or perform critical operations.

*作为安全负责人，我希望定义基于角色的访问控制策略。*

**US-011**

As a compliance officer, I want to review comprehensive audit logs of all agent activities, API calls, and data accesses for regulatory compliance.

*作为合规官员，我希望审查所有智能体活动的全面审计日志。*

**US-012**

As a platform engineer, I want to rotate and manage API keys for all configured providers through a centralized management interface.

*作为平台工程师，我希望通过集中管理界面轮换和管理所有已配置提供者的API密钥。*

**4. Functional Requirements / 功能需求**

**4.1 Agent Management / 智能体管理**

**FR-001 \[P0\]**

Agent Creation: The system shall allow users to create AI agents with configurable properties including name, role, responsibilities, knowledge domains, memory configuration, and skill assignments.

*智能体创建：系统应允许用户创建具有可配置属性的AI智能体，包括名称、角色、职责、知识领域、内存配置和技能分配。*

**FR-002 \[P0\]**

Agent Provider Binding: Each agent shall be bound to a specific AI provider (Claude, GPT, Gemini, Codex, custom) through the Provider Adapter Layer, with the ability to switch providers at runtime.

*智能体提供者绑定：每个智能体应通过提供者适配器层绑定到特定的AI提供者，并能在运行时切换提供者。*

**FR-003 \[P0\]**

Agent Memory System: Each agent shall maintain a persistent memory store divided into short-term (session context), working (current task state), and long-term (knowledge base) memory tiers.

*智能体内存系统：每个智能体应维护一个持久化的内存存储，分为短期、工作和长期内存层。*

**FR-004 \[P1\]**

Agent Skill Registry: The system shall provide a skill registry where agents can discover, acquire, and manage capabilities including code generation, testing, documentation, and custom MCP tools.

*智能体技能注册表：系统应提供技能注册表，智能体可以发现、获取和管理能力。*

**FR-005 \[P1\]**

Agent Lifecycle Management: The system shall support the full agent lifecycle including creation, activation, suspension, reconfiguration, and decommissioning with state persistence.

*智能体生命周期管理：系统应支持完整的智能体生命周期，具有状态持久化。*

**FR-006 \[P1\]**

Agent Briefing System: The system shall support the BriefingManager component that generates contextual briefings for agents before task execution.

*智能体简报系统：系统应支持BriefingManager组件，在任务执行前为智能体生成上下文简报。*

**FR-007 \[P1\]**

Agent MCPServer Integration: Each agent shall run an embedded MCP server (AgentMCPServer) that exposes its capabilities as MCP tools for other agents.

*智能体MCP服务器集成：每个智能体应运行嵌入式MCP服务器，将其能力作为MCP工具暴露。*

**4.2 Team Management / 团队管理**

**FR-008 \[P0\]**

Team Creation & Configuration: The system shall allow users to create teams with a defined purpose, assign agents with specific roles, and configure team-level settings.

*团队创建与配置：系统应允许用户创建团队，分配智能体，并配置团队级设置。*

**FR-009 \[P0\]**

SharedTaskList (SQLite-backed): The system shall implement a SQLite-backed SharedTaskList for collaborative task management with status transitions and priority ordering.

*SharedTaskList（SQLite支持）：系统应实现基于SQLite的共享任务列表。*

**FR-010 \[P0\]**

TeamBus Message Routing: The system shall implement a TeamBus component for peer-to-peer message routing between agents, supporting pub-sub, direct messaging, and broadcast.

*TeamBus消息路由：系统应实现TeamBus组件用于智能体之间的点对点消息路由。*

**FR-011 \[P1\]**

Cross-Team Collaboration: The system shall support cross-team collaboration through controlled channels with governance oversight.

*跨团队协作：系统应支持通过受控通道的跨团队协作。*

**FR-012 \[P0\]**

Team Database Schema: The system shall maintain 6 core database tables: teams, team\_members, shared\_tasks, team\_messages, team\_configs, and team\_audit\_log.

*团队数据库模式：系统应维护6个核心数据库表。*

**FR-013 \[P2\]**

Team Templates: The system shall provide pre-configured team templates for common use cases that users can customize and extend.

*团队模板：系统应为常见用例提供预配置团队模板。*

**4.3 Communication System / 通信系统**

**FR-014 \[P0\]**

Inter-Agent Messaging: The system shall provide reliable messaging infrastructure with guaranteed delivery, message ordering, and persistence.

*智能体间消息传递：系统应提供可靠的消息传递基础设施。*

**FR-015 \[P0\]**

Message Types: The system shall support task assignments, status updates, knowledge sharing, requests for assistance, and coordination signals.

*消息类型：系统应支持多种消息类型。*

**FR-016 \[P0\]**

Communication Governance: All inter-agent communications shall be subject to governance policies including content filtering and audit logging.

*通信治理：所有通信应受治理策略约束。*

**FR-017 \[P1\]**

Communication Visualization: The system shall provide real-time visualization of agent communication patterns and collaboration networks.

*通信可视化：系统应提供实时可视化。*

**4.4 Operating Modes / 运行模式**

**FR-018 \[P0\]**

Human Interaction Mode (HIM): Every agent action requires explicit human approval before execution. The system shall present proposed actions and expected impact for review.

*人机交互模式（HIM）：每个智能体操作都需要明确的人类批准。*

**FR-019 \[P0\]**

Supervision Mode (SM): Agents execute autonomously within pre-defined guardrails. The system monitors actions in real-time and escalates anomalous behavior.

*监督模式（SM）：智能体在预定义护栏内自主执行，系统实时监控并升级异常行为。*

**FR-020 \[P0\]**

All-in-one Autonomous Mode (AM): Agents operate independently with minimal human intervention, enforcing all governance policies and budget limits.

*全自主模式（AM）：智能体独立运行，执行所有治理策略和预算限制。*

**FR-021 \[P1\]**

Mode Switching: Dynamic switching between operating modes at system and per-agent level with smooth state transitions.

*模式切换：在系统级和智能体级动态切换运行模式。*

**FR-022 \[P1\]**

Governance Policy Configuration: Each mode shall support configurable policies including action whitelists, resource limits, and escalation rules.

*治理策略配置：每种模式应支持可配置的治理策略。*

**4.5 iFlows & Workflows / 智能工作流**

**FR-023 \[P0\]**

DAG-Based Workflow Engine: A Directed Acyclic Graph based engine supporting parallel execution, conditional branching, error handling, and retry.

*基于DAG的工作流引擎：支持并行执行、条件分支、错误处理和重试。*

**FR-024 \[P0\]**

Visual Workflow Builder: Drag-and-drop visual builder to create, edit, and debug iFlows without writing code, with real-time validation.

*可视化工作流构建器：拖放式可视化构建器。*

**FR-025 \[P1\]**

Workflow Templates: Pre-built templates for CI/CD, code review, testing, documentation, and deployment workflows.

*工作流模板：用于常见场景的预构建模板。*

**FR-026 \[P1\]**

Workflow Versioning: Support for versioning, comparison, rollback, and release channel management (dev, staging, production).

*工作流版本管理：支持版本比较、回滚和发布渠道管理。*

**FR-027 \[P1\]**

Workflow Monitoring: Real-time monitoring of execution including node status, timing, token consumption, and error tracking.

*工作流监控：实时监控执行状态、时间和错误跟踪。*

**4.6 Provider Adapter / 提供者适配器**

**FR-028 \[P0\]**

BaseProviderAdapter Interface: Abstract interface standardizing interactions with all AI providers for chat, streaming, tools, and context.

*BaseProviderAdapter接口：标准化与所有AI提供者交互的抽象接口。*

**FR-029 \[P0\]**

AdapterRegistry Factory: Dynamic registration, discovery, and instantiation of provider adapters with hot-swapping support.

*AdapterRegistry工厂：动态注册、发现和实例化提供者适配器。*

**FR-030 \[P0\]**

Built-in Adapters: Claude Code, Codex, Gemini, iFlow, and OpenCode adapters with provider-specific optimizations.

*内置适配器：Claude Code、Codex、Gemini、iFlow和OpenCode适配器。*

**FR-031 \[P1\]**

Custom Provider Support: Users can create custom adapters implementing BaseProviderAdapter with configuration-driven registration.

*自定义提供者支持：用户可创建自定义适配器。*

**FR-032 \[P1\]**

Provider Fallback: Automatic failover between providers with configurable fallback chains.

*提供者故障转移：自动故障转移和可配置的转移链。*

**4.7 Orchestration Engine / 编排引擎**

**FR-033 \[P0\]**

Task Decomposition: Automatic decomposition of complex tasks into subtasks with agent assignment and dependency management.

*任务分解：自动将复杂任务分解为子任务并管理依赖关系。*

**FR-034 \[P0\]**

Agent Scheduling: Intelligent scheduling considering workload, expertise, token budgets, and priority levels.

*智能体调度：考虑工作负载、专业知识、token预算和优先级的调度。*

**FR-035 \[P0\]**

Dependency Resolution: Automatic resolution ensuring correct execution order and maximized parallelism.

*依赖解析：自动解析任务依赖，确保正确顺序和最大并行。*

**FR-036 \[P0\]**

Session Management: SessionManagerV2 with ConcurrencyGuard for concurrent sessions, conflict prevention, and isolation.

*会话管理：带有ConcurrencyGuard的SessionManagerV2。*

**FR-037 \[P1\]**

Error Recovery: Comprehensive recovery with exponential backoff retries, alternative agent assignment, and checkpoint-based recovery.

*错误恢复：全面的错误恢复机制。*

**4.8 Knowledge Base (Brains) / 知识库**

**FR-038 \[P0\]**

Structured Knowledge Storage: Entity-relationship modeling supporting architectural decisions, code patterns, API specs, and domain knowledge.

*结构化知识存储：实体-关系建模的知识存储。*

**FR-039 \[P0\]**

Token-Efficient Retrieval: Summarization, compression, and relevance ranking to minimize token usage while maximizing value.

*Token高效知识检索：使用摘要、压缩和排名优化token使用。*

**FR-040 \[P1\]**

Obsidian Integration: Bidirectional sync, editing, and referencing between AgentForge and Obsidian.

*Obsidian集成：与Obsidian的双向集成。*

**FR-041 \[P2\]**

Knowledge Versioning: Version history with diff views, rollback, and evolution tracking.

*知识版本管理：版本历史和差异视图。*

**FR-042 \[P1\]**

Knowledge Sharing: Inter-agent and inter-team sharing with configurable access policies.

*知识共享：具有可配置访问策略的知识共享。*

**4.9 MCP Tools / MCP工具**

**FR-043 \[P0\]**

MCP Protocol Support: Full MCP support using \@modelcontextprotocol/sdk for standardized tool discovery and invocation.

*MCP协议支持：使用\@modelcontextprotocol/sdk的完整MCP支持。*

**FR-044 \[P1\]**

Custom Tool Creation: Framework for custom MCP tools with input/output schemas, permissions, and token cost estimates.

*自定义工具创建：自定义MCP工具的框架。*

**FR-045 \[P1\]**

Tool Registry: Centralized catalog of all MCP tools with capabilities, permissions, and usage statistics.

*工具注册表：所有MCP工具的集中化目录。*

**FR-046 \[P1\]**

Tool Permission Management: Granular controls restricting tool access by agent, team, or operating mode.

*工具权限管理：按智能体、团队或模式限制工具访问。*

**4.10 Document Generation / 文档生成**

**FR-047 \[P1\]**

PRD Generation: Auto-generate PRDs from structured input with bilingual English/Chinese output.

*PRD生成：从结构化输入自动生成双语PRD。*

**FR-048 \[P1\]**

SRS Generation: Generate Software Requirements Specifications with technical requirements and data models.

*SRS生成：生成软件需求规格说明书。*

**FR-049 \[P1\]**

Multi-Format Output: Support .docx, .md, .pdf, and .html with consistent styling.

*多格式输出：支持.docx、.md、.pdf和.html。*

**FR-050 \[P2\]**

Template-Based Generation: Customizable document templates with structure, styling, and content placeholders.

*基于模板的生成：可自定义的文档模板。*

**4.11 Token Management / Token管理**

**FR-051 \[P0\]**

Context Window Optimization: Intelligent prioritization, compression, and truncation to fit provider-specific token limits.

*上下文窗口优化：智能优先级排序、压缩和截断。*

**FR-052 \[P0\]**

Token Budget Allocation: Configurable budgets at system, team, agent, and task levels with real-time enforcement.

*Token预算分配：多级别的可配置预算分配。*

**FR-053 \[P1\]**

Token Usage Analytics: Per-agent, per-provider, per-task breakdowns with trend analysis and cost forecasting.

*Token使用分析：多维度的使用分析和成本预测。*

**FR-054 \[P1\]**

Token Caching: Caching mechanisms to avoid reprocessing identical context across sessions.

*Token缓存：避免重复处理相同上下文的缓存机制。*

**4.12 Security & Access Control / 安全与访问控制**

**FR-055 \[P0\]**

RBAC: Comprehensive role-based access control with predefined roles (Admin, Operator, Developer, Viewer) and custom roles.

*RBAC：全面的基于角色的访问控制。*

**FR-056 \[P0\]**

API Key Management: Centralized management with encrypted storage, auto-rotation, usage tracking, and revocation.

*API密钥管理：集中化管理，加密存储和自动轮换。*

**FR-057 \[P0\]**

Audit Logging: Comprehensive logs of all activities with tamper-proof storage.

*审计日志：所有活动的全面日志，防篡改存储。*

**FR-058 \[P0\]**

Data Encryption: AES-256 at rest and TLS 1.3 in transit for all sensitive data.

*数据加密：静态AES-256和传输TLS 1.3加密。*

**FR-059 \[P1\]**

Security Event Monitoring: Real-time anomaly detection, alerting, and automated response.

*安全事件监控：实时异常检测和自动响应。*

**4.13 Monitoring & Observability / 监控与可观测性**

**FR-060 \[P0\]**

Real-Time Dashboard: System health, active agents, workflows, token consumption, and error rates with configurable widgets.

*实时仪表板：系统状态、智能体、工作流和错误率的可配置仪表板。*

**FR-061 \[P1\]**

Agent Communication Visualization: Interactive graph visualizations of communication patterns and collaboration networks.

*智能体通信可视化：交互式图形可视化。*

**FR-062 \[P1\]**

Session Monitoring: Session duration, token usage, state transitions, and error logs.

*会话监控：会话持续时间、token使用和状态转换。*

**FR-063 \[P1\]**

Performance Metrics: Response latency, throughput, provider availability, and agent utilization.

*性能指标：响应延迟、吞吐量和利用率。*

**FR-064 \[P1\]**

Alerting System: Configurable alerts for budget overruns, provider outages, security incidents, and workflow failures.

*警报系统：可配置的关键事件警报。*

**5. Non-Functional Requirements / 非功能需求**

**5.1 Performance / 性能**

**NFR-001**

The system shall support concurrent operation of at least 50 AI agents with sub-second UI response times.

*系统应支持至少50个AI智能体的并发操作，UI响应时间不超过1秒。*

**NFR-002**

Agent message delivery latency shall not exceed 100ms intra-team and 500ms cross-team.

*消息传递延迟不超过100ms（团队内）和500ms（跨团队）。*

**NFR-003**

The orchestration engine shall manage at least 1000 concurrent tasks across all agent teams.

*编排引擎应管理至少1000个并发任务。*

**NFR-004**

Knowledge base queries shall return results within 200ms for datasets up to 100,000 entries.

*知识库查询应在200ms内返回结果。*

**5.2 Reliability / 可靠性**

**NFR-005**

The system shall achieve 99.9% availability for core orchestration and agent management functions.

*系统应达到99.9%的可用性。*

**NFR-006**

All data persisted to SQLite with automatic backup every 15 minutes and RPO \< 5 minutes.

*数据持久化到SQLite，每15分钟自动备份，RPO \< 5分钟。*

**NFR-007**

Graceful degradation when external AI providers are unavailable, queuing tasks for later execution.

*外部提供者不可用时优雅降级。*

**5.3 Scalability / 可扩展性**

**NFR-008**

Horizontal scaling of agent instances and vertical scaling of provider connections.

*智能体实例的水平扩展和提供者连接的垂直扩展。*

**NFR-009**

Plugin architecture allows adding providers, tools, and workflow nodes without modifying core code.

*插件架构允许无需修改核心代码即可添加功能。*

**5.4 Security / 安全**

**NFR-010**

AES-256-GCM encryption at rest, TLS 1.3 in transit for all sensitive data.

*静态AES-256-GCM加密，传输TLS 1.3加密。*

**NFR-011**

OWASP Top 10 compliance and SOC 2 Type II certification.

*通过OWASP Top 10安全评估和SOC 2 Type II认证。*

**NFR-012**

Immutable audit logs retained for 12+ months with export capabilities.

*不可变的审计日志保留12个月以上。*

**5.5 Usability / 可用性**

**NFR-013**

Responsive GPUI-based desktop UI rendering at 60fps for all interactive elements.

*基于GPUI的响应式桌面UI，60fps渲染。*

**NFR-014**

New users can create their first agent team and execute a basic workflow within 30 minutes.

*新用户30分钟内创建第一个智能体团队并执行基本工作流。*

**5.6 Compatibility / 兼容性**

**NFR-015**

Support Windows 10+, macOS 12+, and Ubuntu 22.04+ as target platforms.

*支持Windows 10+、macOS 12+和Ubuntu 22.04+。*

**NFR-016**

Backward compatibility with provider API versions for at least 2 major versions.

*与提供者API版本保持至少2个主版本的向后兼容。*

**6. Data Model / 数据模型**

AgentForge uses SQLite as its primary storage engine with a Repository pattern for data access abstraction. The data model is organized into the following core domains, each with its own set of tables and relationships.

*AgentForge使用SQLite作为主要存储引擎，并使用Repository模式进行数据访问抽象。数据模型组织为以下核心域。*

**6.1 Core Database Tables / 核心数据库表**

  -------------------- ------------------ ----------------------------------------------------------------------------------------
  **Table Name**       **Domain**         **Description**
  agents               Agent Management   Stores agent configurations, provider bindings, memory settings, and skill assignments
  teams                Team Management    Team definitions with purpose, governance rules, and configuration
  team\_members        Team Management    Agent-to-team membership mappings with role assignments
  shared\_tasks        Team Management    Collaborative task list with status, priority, and assignment tracking
  team\_messages       Communication      Inter-agent message log with type, content, and delivery status
  team\_configs        Team Management    Team-level configuration settings and governance policies
  team\_audit\_log     Security           Immutable audit trail of all team-level operations
  knowledge\_entries   Knowledge Base     Structured knowledge items with categories, tags, and version history
  iflows               Workflows          DAG-based workflow definitions with nodes, edges, and metadata
  sessions             Session Mgmt       Active and historical session records with state snapshots
  api\_keys            Security           Encrypted API key storage with rotation schedules and usage tracking
  audit\_log           Security           System-wide audit log for compliance and security monitoring
  -------------------- ------------------ ----------------------------------------------------------------------------------------

**6.2 Entity Relationships / 实体关系**

The data model follows these key relationships: Agents belong to Teams (many-to-many via team\_members). Teams own SharedTasks and TeamMessages. Knowledge entries are scoped to agents and teams. iFlows reference agents as node executors. Sessions track agent runtime state. All mutations are recorded in audit logs.

*数据模型遵循以下关键关系：智能体属于团队（通过team\_members多对多）。团队拥有共享任务和团队消息。知识条目与智能体和团队关联。iFlows引用智能体作为节点执行者。*

**7. API Specifications / API规范**

AgentForge exposes both internal APIs (for component communication) and external APIs (for extension and integration). All APIs follow RESTful conventions where applicable and use JSON for request/response payloads.

*AgentForge暴露内部API（用于组件通信）和外部API（用于扩展和集成）。所有API在适用的地方遵循RESTful约定，并使用JSON作为请求/响应负载。*

**7.1 Core API Endpoints / 核心API端点**

  ------------ ------------------------------- -----------------------------------------
  **Method**   **Endpoint**                    **Description**
  POST         /api/v1/agents                  Create a new AI agent
  GET          /api/v1/agents/{id}             Retrieve agent configuration and status
  PUT          /api/v1/agents/{id}             Update agent configuration
  DELETE       /api/v1/agents/{id}             Decommission an agent
  POST         /api/v1/teams                   Create a new team
  GET          /api/v1/teams/{id}              Retrieve team details and members
  POST         /api/v1/teams/{id}/tasks        Create a shared task
  POST         /api/v1/messages                Send an inter-agent message
  POST         /api/v1/iflows                  Create a new iFlow workflow
  PUT          /api/v1/iflows/{id}/execute     Execute an iFlow
  GET          /api/v1/sessions                List active sessions
  GET          /api/v1/knowledge               Query the knowledge base
  POST         /api/v1/mcp/tools/{id}/invoke   Invoke an MCP tool
  GET          /api/v1/metrics/token           Retrieve token usage metrics
  GET          /api/v1/audit/logs              Query audit log entries
  ------------ ------------------------------- -----------------------------------------

**8. Integration Requirements / 集成需求**

AgentForge is designed as an integration-friendly platform with well-defined interfaces for connecting to external systems, AI providers, and development tools.

*AgentForge被设计为一个对集成友好的平台，具有用于连接外部系统、AI提供者和开发工具的良好定义的接口。*

**8.1 AI Provider Integrations / AI提供者集成**

  ------------------------- ---------------------- ---------------------- --------------
  **Provider**              **Protocol**           **Integration Type**   **Priority**
  Claude Code (Anthropic)   REST API + Streaming   Native Adapter         P0
  Codex (OpenAI)            REST API + Streaming   Native Adapter         P0
  Gemini (Google)           REST API + Streaming   Native Adapter         P0
  iFlow                     Custom Protocol        Native Adapter         P1
  OpenCode                  REST API               Native Adapter         P1
  Custom Providers          BaseProviderAdapter    Plugin                 P2
  ------------------------- ---------------------- ---------------------- --------------

**8.2 Tool & Platform Integrations / 工具与平台集成**

  --------------------------------- ---------------------------- ----------------------------------------
  **System**                        **Integration Method**       **Purpose**
  Obsidian                          File System Sync + API       Knowledge base bidirectional sync
  Git/GitHub                        CLI + Webhooks               Version control for code and workflows
  CI/CD (Jenkins, GitHub Actions)   Webhook + API                Automated deployment pipelines
  Slack/Teams                       Webhook + Bot API            Notification and alert delivery
  Prometheus/Grafana                Metrics Export               System monitoring and dashboards
  MCP Ecosystem                     \@modelcontextprotocol/sdk   Tool discovery and invocation
  --------------------------------- ---------------------------- ----------------------------------------

**9. User Interface Requirements / 用户界面需求**

The AgentForge desktop application is built using GPUI components (Rust-based, from longbridge/gpui-component) for high-performance native rendering. The UI follows a modern design system with consistent theming, responsive layouts, and accessibility support.

*AgentForge桌面应用程序使用GPUI组件（基于Rust，来自 longbridge/gpui-component）构建，以实现高性能原生渲染。UI遵循现代设计系统。*

**9.1 Main Views / 主要视图**

**Dashboard / 仪表板**

Central overview displaying system health, active agents, running workflows, token consumption metrics, and recent alerts with configurable widget layout.

*显示系统健康、活动智能体、运行中工作流、token消耗和近期警报的中心概览。*

**Agent Manager / 智能体管理器**

CRUD interface for managing agents with provider selection, role configuration, memory settings, and skill assignment panels.

*用于管理智能体的CRUD界面，包括提供者选择、角色配置和技能分配。*

**Team Workspace / 团队工作区**

Collaborative workspace showing team members, shared task board, communication feed, and team-level metrics.

*显示团队成员、共享任务板、通信流和团队指标的协作工作区。*

**iFlow Builder / iFlow构建器**

Visual DAG editor with drag-and-drop nodes, connection wiring, property panels, and real-time execution preview.

*具有拖放节点、连接布线、属性面板和实时执行预览的可视化DAG编辑器。*

**Knowledge Explorer / 知识浏览器**

Hierarchical knowledge browser with search, filtering, tag management, and Obsidian sync status indicators.

*具有搜索、过滤、标签管理和Obsidian同步状态指示器的层次知识浏览器。*

**Monitoring Console / 监控控制台**

Real-time agent communication graph, workflow execution timeline, token usage charts, and security event log viewer.

*实时智能体通信图、工作流执行时间线、token使用图表和安全事件日志查看器。*

**10. Release Phases & Roadmap / 发布阶段与路线图**

AgentForge follows a phased release strategy, delivering core functionality first and progressively adding advanced features. Each phase includes internal milestones and external release targets.

*AgentForge采用分阶段发布策略，首先交付核心功能，然后逐步添加高级功能。*

**10.1 Phase Overview / 阶段概览**

  ----------- --------------- -------------- -----------------------------------------------------------------------------------
  **Phase**   **Name**        **Timeline**   **Key Deliverables**
  Alpha       Foundation      Q2 2026        Core agent management, single provider support, basic UI, SQLite storage
  Beta        Collaboration   Q3 2026        Multi-provider adapters, team management, TeamBus, SharedTaskList, HIM mode
  RC1         Intelligence    Q4 2026        iFlows engine, visual builder, knowledge base (Brains), supervision mode
  RC2         Governance      Q1 2027        RBAC, audit logging, API key management, security monitoring, autonomous mode
  GA          Production      Q2 2027        Performance optimization, Obsidian integration, MCP ecosystem, full documentation
  v1.1        Ecosystem       Q3 2027        Plugin marketplace, advanced analytics, custom provider SDK, enterprise features
  ----------- --------------- -------------- -----------------------------------------------------------------------------------

**11. Success Metrics / 成功指标**

Success is measured across four dimensions: adoption, efficiency, quality, and reliability. The following key performance indicators (KPIs) will be tracked from day one.

*成功从四个维度衡量：采用率、效率、质量和可靠性。以下关键绩效指标将从第一天开始跟踪。*

  ------------------------------ ---------------------------------------------------- ------------------------- ------------------
  **Metric**                     **Target**                                           **Measurement Method**    **Timeline**
  Agent Team Creation Rate       100+ teams created within 30 days of GA              Analytics tracking        Post-GA 30 days
  Task Completion Rate           \> 85% of assigned tasks completed successfully      Session monitoring        Ongoing
  Token Efficiency Improvement   30% reduction in token usage vs. baseline            Token analytics           6 months post-GA
  Provider Uptime                99.5% availability across all configured providers   Health checks             Ongoing
  User Onboarding Time           \< 30 minutes to first workflow execution            Session analytics         Post-GA
  Workflow Success Rate          \> 90% of iFlow executions complete without errors   Workflow monitoring       Ongoing
  Security Compliance            100% audit log coverage, zero critical findings      Security audit            Quarterly
  Cross-Team Collaboration       50+ cross-team interactions per month                Communication analytics   6 months post-GA
  ------------------------------ ---------------------------------------------------- ------------------------- ------------------

**Appendix A: Glossary / 附录A：术语表**

  --------------------- ----------------------------------------------------------------------------------------------------------------------------
  **Term**              **Definition / 定义**
  Agent                 An AI-powered entity with a defined role, provider binding, memory, and skill set that participates in teams and workflows
  Team                  A group of agents collaborating toward a shared purpose with defined governance rules and communication policies
  iFlow                 An intelligent workflow based on a Directed Acyclic Graph (DAG) that orchestrates multi-agent task execution
  TeamBus               The peer-to-peer message routing component enabling inter-agent communication within and across teams
  SharedTaskList        A SQLite-backed collaborative task management system for agent teams
  Brains                The token-efficient knowledge management system with structured storage and Obsidian integration
  MCP                   Model Context Protocol - a standardized protocol for tool discovery, invocation, and result handling
  BaseProviderAdapter   Abstract interface standardizing interactions with all AI providers
  AdapterRegistry       Factory pattern for dynamic registration and instantiation of provider adapters
  SessionManagerV2      Session management component with ConcurrencyGuard for concurrent agent session handling
  BriefingManager       Component that generates contextual briefings for agents before task execution
  AgentMCPServer        Embedded MCP server within each agent that exposes its capabilities as MCP tools
  HIM                   Human Interaction Mode - every agent action requires explicit human approval
  SM                    Supervision Mode - agents operate autonomously within pre-defined guardrails
  AM                    Autonomous Mode - agents operate independently with minimal human intervention
  GPUI                  Rust-based UI framework (from longbridge/gpui-component) used for the desktop application
  RBAC                  Role-Based Access Control - security model restricting access based on user/agent roles
  DAG                   Directed Acyclic Graph - graph structure used for workflow definition with no circular dependencies
  --------------------- ----------------------------------------------------------------------------------------------------------------------------
