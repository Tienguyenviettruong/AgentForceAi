**AGENTFORGE**

Multi-AI Agent Orchestration Platform

AgentForge - 多AI智能体编排平台

**Business Requirements Document (BRD)**

**业务需求文档 (BRD)**

Version 1.0

Date: April 6, 2026

Classification: Confidential

版本：1.0 \| 日期：2026年4月6日 \| 分级：机密

**Document Control / 文档控制**

**Revision History / 修订历史**

**Table of Contents / 目录**

1\. Executive Summary / 执行摘要

2\. Business Background & Problem Statement / 业务背景与问题陈述

3\. Business Objectives & Success Criteria / 业务目标与成功标准

4\. Stakeholder Analysis / 利益相关者分析

5\. Market Analysis & Competitive Landscape / 市场分析与竞争格局

6\. Business Requirements (BR-001 \~ BR-025) / 业务需求

7\. Scope Definition / 范围定义

8\. Operating Modes Requirements / 运行模式需求

9\. Governance, Risk & Compliance / 治理、风险与合规

10\. High-Level Architecture / 高层架构

11\. Budget & Resource Estimation / 预算与资源估算

12\. Timeline & Milestones / 时间线与里程碑

13\. Assumptions & Constraints / 假设与约束

14\. Appendices / 附录

**1. Executive Summary / 执行摘要**

**1.1 Executive Summary**

AgentForge is a next-generation desktop application designed to orchestrate multiple artificial intelligence systems into cohesive, collaborative teams. By unifying leading AI platforms including Google Gemini, OpenAI Codex, Anthropic Claude, ChatGPT, and locally developed proprietary models, AgentForge enables organizations to deploy intelligent agent teams that can simultaneously develop, operate, and innovate across diverse domains. The platform addresses the growing complexity of AI-driven workflows by providing a centralized governance layer with robust access control, security constraints, and intelligent resource management.

The core value proposition of AgentForge lies in its ability to transform isolated AI capabilities into a coordinated force. Through its three distinct operating modes (Human Interaction, Supervision, and All-in-one), AgentForge provides flexibility ranging from fully human-guided interactions to complete autonomous operation. The platform\'s iFlows (Intelligent Workflows) system enables the design, tracking, and automation of complex multi-step processes, while the innovative \'Brains\' system optimizes token usage and integrates with knowledge management tools like Obsidian for structured storage and retrieval.

AgentForge is built on a modern technology stack featuring GPUI components for a responsive desktop interface, a Provider Adapter architecture for seamless multi-AI integration, and MCP (Model Context Protocol) infrastructure for extensible tool support. With comprehensive document generation capabilities and real-time visualization of agent interactions, AgentForge represents a paradigm shift in how organizations leverage AI for software development, research, and operational excellence.

**1.1 执行摘要**

AgentForge是一款新一代桌面应用程序，旨在将多个人工智能系统编排为有机的协作团队。通过统一Google Gemini、OpenAI Codex、Anthropic Claude、ChatGPT以及本地开发的专有模型等领先的AI平台，AgentForge使组织能够部署智能体团队，在多个领域同时进行开发、运营和创新。该平台通过提供集中式治理层、健全的访问控制、安全约束和智能资源管理，解决了AI驱动工作流日益增长的复杂性问题。

AgentForge的核心价值主张在于其将孤立的AI能力转化为协调力量的能力。通过其三种不同的运行模式（人机交互模式、监督模式和全自动模式），AgentForge提供了从完全人类引导的交互到完全自主运营的灵活性。平台的iFlows智能工作流系统能够设计、跟踪和自动化复杂的多步骤流程，而创新的大脑系统则优化token使用并与Obsidian等知识管理工具集成。

AgentForge基于现代技术栈构建，采用GPUI组件提供响应式桌面界面，提供商适配器架构实现无缝的多AI集成，以及MCP模型上下文协议基础设施提供可扩展的工具支持。凭借全面的文档生成能力和实时的代理交互可视化，AgentForge代表了组织利用AI进行软件开发、研究和运营卓越性的范式转变。

**2. Business Background & Problem Statement / 业务背景与问题陈述**

**2.1 Business Background / 业务背景**

The rapid proliferation of AI capabilities across the enterprise landscape has created both unprecedented opportunities and significant operational challenges. Organizations now have access to multiple powerful AI systems, each with distinct strengths: Gemini excels in multimodal reasoning, Claude in nuanced text generation, Codex in code generation, ChatGPT in general-purpose conversation, and proprietary models offering domain-specific advantages. However, these systems operate in isolation, requiring human operators to manually coordinate tasks, transfer context, and manage complexity.

The enterprise AI market is projected to exceed \$500 billion by 2027. Despite this investment, a critical gap remains: there is no comprehensive platform that enables these AI systems to work together as coordinated teams, governed by centralized policies and security frameworks. Current solutions focus on single-model interactions or provide basic API routing without sophisticated orchestration, identity management, and workflow automation capabilities.

AgentForge bridges this gap by creating a unified orchestration layer that treats each AI system as a capable team member with defined roles, responsibilities, and communication protocols, transforming the paradigm from \'human managing multiple AI tools\' to \'human directing an AI team that manages itself.\'

**2.1 业务背景**

人工智能能力在企业领域的快速增长既带来了前所未有的机遇，也带来了重大的运营挑战。组织现在可以访问多个强大的AI系统，每个系统都有独特的优势：Gemini擅长多模态推理，Claude擅长细腔入微的文本生成和分析，Codex擅长代码生成和软件开发，ChatGPT擅长通用对话和任务完成。然而，这些系统往往独立运行，需要人类操作员手动协调任务并管理复杂性。

企业AI市场预计到2027年将超过5000亿美元。尽管如此，仍然存在一个关键的缺口：目前没有一个全面的平台能够让这些AI系统作为协调的团队工作。当前的解决方案要么专注于单模型交互，要么提供基本的API路由，而缺乏企业团队所需的复杂编排、身份管理和工作流自动化能力。

AgentForge的构想正是为了弥合这一缺口。通过创建统一的编排层，将每个AI系统视为具有明确角色、职责和通信协议的能干的团队成员，AgentForge将范式从人类管理多个AI工具转变为人类指挥一个自我管理的AI团队。

**2.2 Problem Statement / 问题陈述**

Organizations face the following critical challenges:

-   Fragmented AI Ecosystem: Multiple AI platforms operate in silos, requiring manual context switching.

-   AI生态破碎：多个AI平台孤立运行，需要手动切换上下文。

-   No Collaborative Intelligence: AI systems cannot share knowledge or coordinate without human mediation.

-   缺乏协作智能：AI系统无法在没有人类介入的情况下共享知识。

-   Token Inefficiency: Repeated context loading leads to excessive token consumption and costs.

-   Token使用低效：重复的上下文加载导致过度的token消耗。

-   Security and Governance Gaps: Decentralized AI usage creates blind spots in access control.

-   安全和治理缺口：分散的AI使用在访问控制方面创造了盲区。

-   Workflow Fragmentation: Complex processes cannot be automated across AI systems.

-   工作流破碎：复杂的多步骤流程无法在AI系统之间自动化。

-   Knowledge Loss: Insights from AI interactions are not systematically captured or reused.

-   知识流失：AI交互中的洞察未被系统地捕获或重用。

**3. Business Objectives & Success Criteria / 业务目标与成功标准**

**3.1 Business Objectives / 业务目标**

The following strategic objectives guide AgentForge development:

**Obj 1: Unified AI Orchestration / 目标一：统一AI编排**

-   Enable seamless orchestration of 5+ AI providers through a unified interface.

-   通过统一界面实现对5个以上AI提供商的无缝编排。

-   Reduce context-switching overhead by 80%.

-   将上下文切换开销减少80%。

**Obj 2: Intelligent Collaboration / 目标二：智能协作**

-   Enable AI agents to form self-organizing teams with defined roles.

-   使AI代理能够组建具有明确角色的自组织团队。

-   Support cross-team collaboration with automated task distribution.

-   支持跨团队协作，具备自动任务分配。

**Obj 3: Cost Optimization / 目标三：成本优化**

-   Reduce AI token consumption by 40-60% through the Brains system.

-   通过大脑系统将AI token消耗减少40-60%。

-   Eliminate redundant API calls through shared knowledge bases.

-   通过共享知识库消除冗余的API调用。

**Obj 4: Enterprise Security / 目标四：企业安全**

-   Implement RBAC with granular permissions for all AI interactions.

-   为所有AI交互实施精细权限的RBAC。

-   Ensure compliance with SOC 2, GDPR, and data residency requirements.

-   确保符合SOC 2、GDPR和数据驻留要求。

**Obj 5: Workflow Automation / 目标五：工作流自动化**

-   Enable complex multi-step iFlows spanning multiple AI agents and teams.

-   支持跨多个AI代理和团队的复杂iFlows。

-   Achieve 70% reduction in manual process steps for common workflows.

-   对常见工作流实现70%的手动步骤减少。

**3.2 Success Criteria / 成功标准**

**4. Stakeholder Analysis / 利益相关者分析**

**5. Market Analysis & Competitive Landscape / 市场分析与竞争格局**

**5.1 Market Overview / 市场概述**

The AI orchestration market is expected to grow at 35% CAGR through 2028, reaching \~\$18B. Key drivers include specialized AI model proliferation, cost optimization needs, and growing AI governance requirements. The target market includes mid-to-large enterprises (500+ employees) in technology, financial services, healthcare, and consulting. TAM: \$8.5B, SAM: \~\$2.1B for desktop orchestration.

AI编排市场预计以35%的年复合增长率增长至2028年，达到约180亿美元。目标市场包括科技、金融、医疗和咨询等行业的中大型企业。总可寻址市场（TAM）为85亿美元，桌面编排解决方案的可服务可寻址市场（SAM）约为21亿美元。

**5.2 Competitive Landscape / 竞争格局**

**5.3 Differentiation / 差异化策略**

-   Multi-Provider Native: Orchestrates Gemini, Claude, Codex, ChatGPT, and custom models simultaneously.

-   多提供商原生：同时编排多个AI模型。

-   Enterprise Governance: RBAC, audit trails, data residency, compliance built into the core.

-   企业治理：RBAC、审计跟踪、数据驻留和合规内置于核心。

-   Intelligent Cost Management: Brains system and token optimization reduce costs by 40-60%.

-   智能成本管理：大脑系统和token优化降低40-60%成本。

-   Flexible Operating Modes: Three modes for varying human involvement levels.

-   灵活的运行模式：三种模式适应不同程度的人类参与。

-   Knowledge Persistence: Deep Obsidian integration ensures institutional knowledge is reusable.

-   知识持久化：深度Obsidian集成确保知识可重用。

**6. Business Requirements / 业务需求**

This section defines comprehensive business requirements for AgentForge. Each requirement has a unique BR identifier, priority, and bilingual description.

**BR-001: Multi-AI Provider Integration / 多AI提供商集成**

The system shall provide a unified Provider Adapter layer integrating at least five AI providers: Google Gemini, OpenAI Codex, Anthropic Claude, ChatGPT, and locally developed models. Each adapter shall handle provider-specific protocols (Claude Code Agent SDK V2, Codex CLI JSON-RPC, Gemini CLI NDJSON, iFlow CLI ACP) and present a standardized interface to the orchestration engine.

系统应提供统一的提供商适配器层，集成至少五个AI提供商。每个适配器应处理特定于提供商的通信协议并向编排引擎提供标准化接口。

**BR-002: Agent Team Formation / 代理团队组建**

The system shall allow users to create multiple agent teams with AI agents from different providers. Users shall assign roles, define team objectives, and configure inter-agent communication rules. The system shall support 10+ concurrent teams with up to 20 agents per team.

系统应允许用户创建多个代理团队。用户应能分配角色、定义团队目标并配置通信规则。系统应支持10个以上并发团队，每团队最多20个代理。

**BR-003: Agent Identity & Profile / 代理身份与配置**

Each AI agent shall have a comprehensive identity profile: unique name, defined role, responsibilities, knowledge base, persistent memory, contextual awareness, specialized skills, and independent research capabilities. Profiles shall be persistent across sessions and customizable.

每个AI代理应具有全面的身份配置：唯一名称、角色、职责、知识库、持久记忆、上下文意识、专业技能和独立研究能力。配置应持久化并可自定义。

**BR-004: Agent Communication / 代理通信系统**

The system shall provide a robust communication framework for agents to exchange messages, share context, distribute tasks, and collaborate. Communication shall be supported intra-team and inter-team, with message queuing, priority routing, and conflict resolution.

系统应提供健全的通信框架，使代理能够交换消息、共享上下文、分配任务。通信应支持团队内和团队间，包括消息队列、优先级路由和冲突解决。

**BR-005: iFlows - Intelligent Workflows / iFlows - 智能工作流**

The system shall provide an iFlows designer for creating complex multi-step workflows with conditional branching, parallel execution, error handling, retry logic, and MCP integration. Each iFlow shall have real-time progress tracking and status reporting.

系统应提供iFlows设计器，支持条件分支、并行执行、错误处理、重试逻辑和MCP集成。每个iFlow应具有实时进度跟踪和状态报告。

**BR-006: Human Interaction Mode / 人机交互模式**

The system shall support Human Interaction Mode where AI agents communicate directly with human users. Agents shall present information, answer questions, provide recommendations, and execute tasks under explicit human direction.

系统应支持人机交互模式，AI代理与人类用户直接通信。代理应展示信息、回答问题、提供建议并在人类指导下执行任务。

**BR-007: Supervision Mode / 监督模式**

The system shall support Supervision Mode where AI agents communicate autonomously while humans monitor in real-time. Supervisors can intervene, pause, redirect, or terminate communications at any point.

系统应支持监督模式，AI代理自主通信而人类实时监控。监督者可干预、暂停、重定向或终止通信。

**BR-008: All-in-one Autonomous Mode / 全自动模式**

The system shall support All-in-one Mode where agent teams receive high-level tasks and autonomously collaborate, distribute work, and execute without human involvement. Safeguards include task boundaries, resource limits, and escalation triggers.

系统应支持全自动模式，代理团队接收高层任务并自主协作和执行。安全措施包括任务边界、资源限制和升级触发器。

**BR-009: Document Generation / 文档生成**

The system shall support automated generation of PRD, SRS, .doc, and .md documents leveraging agent collaboration with customizable templates and bilingual output.

系统应支持自动生成PRD、SRS、.doc和.md文档，利用代理协作，支持可自定义模板和双语输出。

**BR-010: GPUI Desktop Interface / GPUI桌面界面**

The system shall provide a modern desktop interface built with GPUI components (Rust-based, longbridge/gpui-component) with real-time agent communication visualization, drag-and-drop team configuration, and customizable dashboards.

系统应提供基于GPUI组件构建的现代桌面界面，具有实时代理通信可视化、拖放配置和可自定义仪表板。

**BR-011: Session & Conversation Mgmt / 会话管理**

The system shall provide session and conversation management with independent histories, agent states, context windows. Sessions shall be persistable, resumable, searchable, with branching, merging, and cross-session context sharing.

系统应提供会话和对话管理，具有独立历史、代理状态和上下文窗口。会话应可持久化、可恢复、可搜索。

**BR-012: Orchestration Engine / 编排引擎**

The system shall implement a central Orchestration Engine for task decomposition, agent assignment, workload balancing, and execution coordination with dynamic re-planning, priority queues, dependency graphs, and deadlock prevention.

系统应实现中央编排引擎，负责任务分解、代理分配、工作负载平衡和执行协调。引擎应支持动态重规划和死锁预防。

**BR-013: Agent Skills & Rules / 代理技能与规则**

The system shall provide a configurable skills and rules framework. Skills define what agents can do; rules define how they behave. Skills and rules shall be composable, versioned, and shareable.

系统应提供可配置的技能和规则框架。技能定义代理能做什么；规则定义行为方式。技能和规则应可组合、可版本化并可共享。

**BR-014: Brains Knowledge System / 大脑知识系统**

The system shall implement a Brains knowledge system that avoids inefficient token usage by maintaining structured knowledge bases separate from context. Integrates with Obsidian for markdown knowledge graphs and semantic search.

系统应实现大脑知识系统，通过将结构化知识库与对话上下文分离来避免低效token使用。与Obsidian集成支持知识图和语义搜索。

**BR-015: MCP Tools Support / MCP工具支持**

The system shall support MCP for extensible tool integration, enabling agents to interact with external tools and data sources. Includes built-in MCP registry, custom tool development, and sandboxed execution.

系统应支持MCP实现可扩展工具集成。包括内置MCP注册表、自定义工具开发和沙箱执行环境。

**BR-016: Token Usage Management / Token管理**

The system shall implement token management to prevent context explosion: context monitoring, automatic summarization, intelligent pruning, budget allocation per agent/team, and real-time cost dashboards.

系统应实现token管理以防止上下文爆炸：上下文监控、自动摘要、智能修剪、预算分配和实时成本仪表板。

**BR-017: Access Control & Security / 访问控制与安全**

The system shall implement RBAC, encrypted communications (TLS 1.3), secure credential storage, audit logging, DLP policies, and enterprise SSO (SAML 2.0, OAuth 2.0).

系统应实施RBAC、加密通信、安全凭证存储、审计日志、DLP策略和企业SSO集成。

**BR-018: Data Persistence / 数据持久化**

The system shall use SQLite with Repository pattern for persistence, managing agent profiles, sessions, conversation logs, workflows, knowledge entries, and configuration with export/import.

系统应使用SQLite和存储库模式进行持久化，管理代理配置、会话、对话日志、工作流等，支持导出/导入。

**BR-019: Real-time Visualization / 实时可视化**

The system shall provide real-time visualization of agent activities: communication flows, task assignments, collaboration graphs, and workflow progress with multiple views (network, timeline, kanban, Gantt).

系统应提供代理活动的实时可视化，支持多种视图：网络图、时间线、看板、甘特图。

**BR-020: Independent Research / 独立研究**

Each agent shall conduct independent research: web searches, documentation analysis, codebase exploration, and data synthesis. Results stored in knowledge base and shared with team.

每个代理应能进行独立研究：网络搜索、文档分析、代码库探索和数据综合。结果存储在知识库中并与团队共享。

**BR-021: Cross-Team Collaboration / 跨团队协作**

The system shall enable cross-team collaboration with shared task boards, inter-team messaging, resource sharing, and coordinated multi-team workflow execution while maintaining team isolation.

系统应实现跨团队协作，包括共享任务板、团队间消息传递、资源共享和多团队工作流协调执行。

**BR-022: Obsidian Integration / Obsidian集成**

The system shall integrate with Obsidian for structured knowledge storage: automatic vault creation, bidirectional linking, graph navigation, and markdown with frontmatter metadata.

系统应与Obsidian集成，支持自动创建知识库、双向链接、图视图导航和带frontmatter的markdown。

**BR-023: Context Explosion Prevention / 上下文爆炸防止**

The system shall prevent context explosion via automatic summarization, hierarchical context management (global/team/agent/session), smart pruning by relevance, and context compression.

系统应通过自动摘要、分层上下文管理、智能修剪和上下文压缩来防止上下文爆炸。

**BR-024: Audit Logging & Compliance / 审计日志与合规**

The system shall maintain tamper-proof, searchable, exportable audit logs. Support compliance reporting for SOC 2, GDPR, HIPAA.

系统应维护防篡改、可搜索、可导出的审计日志。支持SOC 2、GDPR、HIPAA合规报告。

**BR-025: Desktop Deployment / 桌面部署**

The system shall be a native desktop app for Windows, macOS, Linux with auto-updates, offline sync, and enterprise deployment. Built with Electron or native frameworks with GPUI.

系统应为Windows、macOS和Linux的原生桌面应用，支持自动更新、离线同步和企业部署。

**7. Scope Definition / 范围定义**

**7.1 In-Scope / 范围内**

The following items are within AgentForge v1.0 scope:

以下项目属于AgentForge v1.0范围内：

-   Multi-AI provider integration

-   多AI提供商集成

-   Agent team management with identity profiles

-   代理团队管理

-   Three operating modes

-   三种运行模式

-   iFlows workflow designer

-   工作流设计器

-   Document generation (PRD, SRS, .doc, .md)

-   文档生成

-   GPUI desktop interface

-   桌面界面

-   Brains + Obsidian integration

-   大脑+Obsidian集成

-   MCP tools

-   工具支持

-   Token management

-   管理

-   RBAC security

-   RBAC安全

-   SQLite persistence

-   SQLite持久化

-   Session management

-   会话管理

**7.2 Out-of-Scope / 范围外**

The following are out of scope for v1.0:

以下项目不在v1.0范围内：

-   Cloud SaaS deployment (v2.0)

-   云SaaS部署

-   Mobile app

-   移动应用

-   Model training/fine-tuning

-   模型训练

-   Voice interaction

-   语音交互

-   Multi-user same-session collaboration

-   多用户协作

-   CI/CD integration (v1.5)

-   持续集成

-   Blockchain

-   区块链

**8. Operating Modes Requirements / 运行模式需求**

AgentForge supports three operating modes. Users can switch dynamically; different teams can operate in different modes simultaneously.

AgentForge支持三种运行模式。用户可动态切换，不同团队可同时使用不同模式。

**8.1 Human Interaction Mode / 人机交互模式**

Users directly communicate with agents. Messages routed by team config and agent roles. Ideal for exploratory work and human judgment scenarios.

用户直接与代理通信。消息根据团队配置和代理角色路由。适合探索性工作。

**8.2 Supervision Mode / 监督模式**

Agents collaborate autonomously while humans observe. Live visualization of all communications. Ideal for complex tasks with human oversight for quality.

代理自主协作，人类观察。实时可视化所有通信。适合复杂任务，人类监督确保质量。

**8.3 All-in-one Mode / 全自动模式**

Agent teams receive objectives and execute autonomously. Includes task boundaries, resource limits, escalation triggers, and automated checkpointing. Results compiled into reports.

代理团队接收目标并自主执行。包括任务边界、资源限制和自动检查点。结果编译为报告。

**9. Governance, Risk & Compliance / 治理、风险与合规**

**9.1 Governance Framework / 治理框架**

AgentForge implements a comprehensive governance framework covering access control, data protection, audit compliance, and risk management. The framework ensures all AI operations are traceable, controllable, and compliant with enterprise policies and regulations.

AgentForge实施全面的治理框架，涵盖访问控制、数据保护、审计合规和风险管理。框架确保所有AI操作可追溯、可控制并符合企业策略和法规。

**9.2 Risk Assessment / 风险评估**

**9.3 Compliance Requirements / 合规要求**

-   SOC 2 Type II: Access controls, encryption, monitoring, incident response.

-   SOC 2 Type II：访问控制、加密、监控、事件响应。

-   GDPR: Data minimization, right to erasure, consent management, data portability.

-   GDPR：数据最小化、删除权、同意管理、数据可携性。

-   HIPAA: PHI protection, access logging, encryption at rest and in transit.

-   HIPAA：PHI保护、访问日志、静态和传输加密。

-   AI Governance: Model bias monitoring, explainability, human oversight requirements.

-   AI治理：模型偏见监控、可解释性、人类监督要求。

**10. High-Level Architecture / 高层架构**

AgentForge is built on a layered architecture with the following components:

AgentForge基于分层架构构建，包含以下组件：

**10.1 Architecture Layers / 架构层次**

**10.2 Provider Adapter Architecture / 提供商适配器架构**

The Provider Adapter layer normalizes interactions with different AI providers:

提供商适配器层规范化与不同AI提供商的交互：

-   Claude Adapter: Anthropic Claude via Claude Code Agent SDK V2

-   Claude适配器：通过Claude Code Agent SDK V2

-   Codex Adapter: OpenAI Codex via CLI JSON-RPC protocol

-   Codex适配器：通过CLI JSON-RPC协议

-   Gemini Adapter: Google Gemini via CLI NDJSON streaming

-   Gemini适配器：通过CLI NDJSON流式传输

-   iFlow Adapter: Custom models via iFlow CLI ACP protocol

-   iFlow适配器：通过iFlow CLI ACP协议

-   Custom Adapter: Extensible interface for additional providers

-   自定义适配器：可扩展接口支持额外提供商

**11. Budget & Resource Estimation / 预算与资源估算**

**12. Timeline & Milestones / 时间线与里程碑**

**13. Assumptions & Constraints / 假设与约束**

**13.1 Assumptions / 假设**

-   AI provider APIs will remain available and stable throughout development.

-   AI提供商API在开发期间将保持可用和稳定。

-   Target users have basic familiarity with AI concepts and agent-based systems.

-   目标用户对AI概念和基于代理的系统有基本了解。

-   GPUI component library will provide sufficient UI components for desktop needs.

-   GPUI组件库将提供足够的UI组件。

-   SQLite performance will be adequate for single-user desktop workloads.

-   SQLite性能将满足单用户桌面工作负载。

-   Obsidian will maintain its current plugin API and vault format.

-   Obsidian将维持其当前的插件API和知识库格式。

-   MCP protocol will gain broader adoption across AI providers.

-   MCP协议将在更多AI提供商中获得更广泛采用。

**13.2 Constraints / 约束**

-   Desktop-first: v1.0 is desktop-only; cloud SaaS deferred to v2.0.

-   桌面优先：v1.0仅支持桌面；云SaaS延后至v2.0。

-   No model training: AgentForge orchestrates existing models; does not train new ones.

-   无模型训练：AgentForge编排现有模型，不训练新模型。

-   Token budget: Each organization has finite API budgets requiring optimization.

-   Token预算：每个组织的API预算有限，需要优化。

-   Provider dependency: Features depend on third-party AI provider capabilities.

-   提供商依赖：功能依赖于第三方AI提供商能力。

-   Cross-platform: Must support Windows, macOS, and Linux simultaneously.

-   跨平台：必须同时支持Windows、macOS和Linux。

**14. Appendices / 附录**

**14.1 Glossary / 术语表**

**14.2 Reference Documents / 参考文档**

-   Claude Code Agent SDK V2 Documentation

-   Claude Code Agent SDK V2文档

-   OpenAI Codex CLI JSON-RPC Specification

-   OpenAI Codex CLI JSON-RPC规范

-   Google Gemini CLI NDJSON Protocol

-   Google Gemini CLI NDJSON协议

-   Model Context Protocol (MCP) Specification

-   MCP规范说明书

-   longbridge/gpui-component Repository

-   longbridge/gpui-component代码库

-   Obsidian Plugin API Documentation

-   Obsidian插件API文档

**14.3 Approval / 审批**
