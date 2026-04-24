**AgentForge**

UI Design Document

UI 设计文档

Version: **1.0**

Date: **April 6, 2026**

Author: **AgentForge Design Team**

Status: **Draft**

Classification: **Confidential**

**Introduction / 引言**

**Purpose / 目的**

This document provides a comprehensive UI design specification for AgentForge, an AI-powered multi-agent orchestration platform. It defines the visual language, component architecture, layout systems, and interaction patterns. 本文档为AgentForge提供全面的UI设计规范，AgentForge是一个基于AI的多智能体编排平台。它定义了视觉语言、组件架构、布局系统和交互模式。

**Scope / 范围**

> • Application shell and overall layout structure --- 应用程序外壳和整体布局结构
>
> • Color scheme, typography, and design tokens --- 配色方案、字体排版和设计令牌
>
> • Component library specifications --- 组件库规范
>
> • Navigation, panels, and workspace design --- 导航、面板和工作区设计
>
> • Agent team management interface --- 智能体团队管理界面
>
> • iFlow workflow designer --- iFlow工作流设计器
>
> • Monitoring and observability dashboards --- 监控与可观测性仪表板
>
> • Knowledge base (Brains) interface --- 知识库（Brains）界面
>
> • Responsive behavior and accessibility --- 响应式行为和无障碍设计

**Design Principles / 设计原则**

> • Dark-first Design: Optimized for extended use with deep navy color scheme --- 暗色优先设计：采用深海军蓝配色方案，减少眼睛疲劳
>
> • Information Density: Maximize useful information per screen --- 信息密度：最大化每个屏幕的有用信息
>
> • Composability: Reusable components composed into complex layouts --- 可组合性：可复用组件组合成复杂布局
>
> • Real-time Feedback: Visual indicators update in real time --- 实时反馈：视觉指示器实时更新
>
> • Accessibility First: Full keyboard nav, screen reader, WCAG AA --- 无障碍优先：完全键盘导航、屏幕阅读器、WCAG AA

**Overall Layout Structure / 整体布局结构**

**Application Shell / 应用外壳**

The application shell provides the foundational frame for AgentForge using a hybrid IDE-dashboard pattern. It consists of a fixed icon-based navigation rail on the far left (~60px width), a collapsible secondary panel for team hierarchy navigation, the main contextual workspace in the center that adapts based on content type (code/configuration view, visual canvas, or runtime monitor), and an inspector/detail panel on the right. The shell uses a flex-based layout system that allows panels to be collapsed, expanded, or rearranged. 应用程序外壳采用混合IDE-仪表板模式。由左侧固定图标导航栏（约60px）、可折叠的团队层级导航面板、中央上下文工作区和右侧检查器/详情面板组成。

**Layout Grid System / 布局网格系统**

AgentForge uses a 12-column grid system with a base unit of 8px. Column widths are fluid within defined breakpoints, and gutters maintain a fixed 16px width. AgentForge使用12列网格系统，基础单元为8px。列宽在断点内是流动的，槽保持16px宽度。

**Panel Architecture / 面板架构**

The layout follows a three-tier navigation structure: (1) Global Icon Rail (~60px fixed, far left) with icons for AgentForge logo, Teams, Chat, Tools, Settings, and user profile; (2) Collapsible Team Hierarchy Panel showing "AGENT TEAMS" tree structure with expandable team folders, agent configurations, version indicators with colored status dots (green/yellow/red), and a "Create New Team" button; (3) Main Contextual Workspace (flexible, center) that switches between Design, Run, Test, and other tabbed views; (4) Inspector/Detail Panel (right side) showing task configuration, role definitions, workflow documentation, and status information. 布局采用三层导航结构：（1）全局图标栏（约60px），（2）可折叠团队层级面板，显示"AGENT TEAMS"树结构，（3）主上下文工作区，支持设计、运行、测试等标签视图切换，（4）检查器/详情面板。

**Main Dashboard Overview / 主仪表板概览**

The main dashboard serves as the central hub of AgentForge, providing an at-a-glance overview of the entire system. It uses a configurable widget layout where users can arrange dashboard components to their preference. The dashboard displays agent team status cards, active workflow indicators, a real-time activity feed, system health metrics, token consumption trends (7-day bar chart), token distribution by project (pie chart), cost estimation panel, and recent alerts. All widgets support real-time data updates and are rendered within the application shell framework. 主仪表板是AgentForge的中央枢纽，提供系统整体概览。使用可配置的部件布局，用户可自定义排列仪表板组件。

**Color Scheme & Design Tokens / 配色方案与设计令牌**

**Primary Color Palette / 主色调**

The primary color palette establishes the visual foundation for AgentForge using a dark theme optimized for extended use. 主色调为AgentForge建立了视觉基础，使用了针对长时间使用优化的暗色主题。

  ---------------------- ---------------- ---------- ------------- -------------- ----------
  **Element**            **Color Name**   **Hex**    **元素**      **颜色名称**   **Hex**
  Primary Background     Deep Dark        \#0F1419   主背景        深黑色         \#0F1419
  Secondary Background   Dark Blue-Gray   \#1A1F2E   次背景        深蓝灰色       \#1A1F2E
  Card Surface           Elevated Panel   \#252B3A   卡片表面      提升面板       \#252B3A
  Border/Divider         Subtle Gray      \#374151   边框/分割线   微妙灰色       \#374151
  Primary Accent         Teal/Cyan        \#00D4AA   主强调色      青绿色         \#00D4AA
  Secondary Accent       Blue             \#3B82F6   次强调色      蓝色           \#3B82F6
  Success/Running        Green            \#00D4AA   成功/运行中   绿色           \#00D4AA
  Warning/Waiting        Orange           \#F59E0B   警告/等待中   橙色           \#F59E0B
  Error                  Red              \#EF4444   错误          红色           \#EF4444
  Text Primary           Off-white        \#F0F0F0   主文本        米白色         \#F0F0F0
  Text Secondary         Muted Gray       \#9CA3AF   次文本        柔和灰色       \#9CA3AF
  Knowledge Accent       Purple           \#8B5CF6   知识强调色    紫色           \#8B5CF6
  ---------------------- ---------------- ---------- ------------- -------------- ----------

**Semantic Colors / 语义色彩**

Semantic colors communicate meaning and state across the interface. 语义色彩在整个界面中传达含义和状态。

> • Success (\#00D4AA): Agent running, task completed, healthy state --- 成功：智能体运行中、任务完成、健康状态
>
> • Warning (\#F59E0B): Agent waiting, pending review, in-progress --- 警告：智能体等待中、待审核、进行中
>
> • Error (\#EF4444): Agent failure, critical alert, authentication errors --- 错误：智能体故障、严重警报、认证错误
>
> • Info (\#3B82F6): Links, interactive elements, informational messages --- 信息：链接、交互元素、信息消息
>
> • Knowledge (\#8B5CF6): Knowledge base elements --- 知识：知识库元素、大脑节点

**Typography Scale / 字体排版**

AgentForge uses a consistent type scale based on a 1.25 ratio. Primary font: Inter (Latin), Microsoft YaHei (CJK). AgentForge使用基于1.25比例的字体比例。主要字体：Inter（拉丁）、Microsoft YaHei（CJK）。

  ----------- ---------- ---------------- ----------------- ---------- ---------- -------------- ----------
  **Level**   **Size**   **Weight**       **Line Height**   **级别**   **大小**   **字重**       **行高**
  Display     32px       Bold (700)       1.2               展示       32px       粗体 (700)     1.2
  Heading 1   24px       Bold (700)       1.3               标题 1     24px       粗体 (700)     1.3
  Heading 2   20px       SemiBold (600)   1.35              标题 2     20px       半粗体 (600)   1.35
  Heading 3   16px       SemiBold (600)   1.4               标题 3     16px       半粗体 (600)   1.4
  Body        14px       Regular (400)    1.5               正文       14px       常规 (400)     1.5
  Caption     12px       Regular (400)    1.4               说明文字   12px       常规 (400)     1.4
  Code        13px       Mono (400)       1.6               代码       13px       等宽 (400)     1.6
  ----------- ---------- ---------------- ----------------- ---------- ---------- -------------- ----------

**Spacing & Sizing System / 间距与尺寸系统**

The spacing system is based on a 4px base unit, with common values of 4, 8, 12, 16, 20, 24, 32, 40, 48, 56, and 64px. Component sizing follows a similar 4px grid. 间距系统基于4px基础单元，常用值为4、8、12、16、20、24、32、40、48、56和64px。

**Elevation & Shadow System / 层级与阴影系统**

Elevation is conveyed through background color shifts. Level 0 (`#0F1419`), Level 1 (`#1A1F2E`), Level 2 (`#252B3A`), Level 3 (`#374151` with 1px border). Shadow elevation uses subtle shadows (approximately `0 1px 3px rgba(0,0,0,0.3)`). 层级通过背景颜色变化传达。等级 0 (`#0F1419`)，等级 1 (`#1A1F2E`)，等级 2 (`#252B3A`)，等级 3 (`#374151`)。

**Component Library / 组件库**

**GPUI Components Overview / GPUI组件概览**

AgentForge leverages the longbridge/gpui-component library as its foundational UI framework. Key crates: gpui-component (core UI), gpui-component-code-editor (syntax highlighting), gpui-component-dock (dockable panels), gpui-component-markdown (markdown rendering). AgentForge利用longbridge/gpui-component库作为基础UI框架。关键crate：gpui-component、gpui-component-code-editor、gpui-component-dock、gpui-component-markdown。

**Component Specifications Table / 组件规格表**

  ------------------ ---------------------- ---------------------------- -------------- ---------------------- --------------------
  **Component**      **Size**               **Description**              **组件**       **尺寸**               **描述**
  Button (Primary)   H: 36px, Min-W: 80px   Main action trigger          按钮（主要）   H: 36px, Min-W: 80px   主要操作触发器
  Button (Ghost)     H: 36px, Min-W: 80px   Secondary action             按钮（幻灵）   H: 36px, Min-W: 80px   次要操作
  Input Field        H: 36px, Full-width    Text input with focus ring   输入字段       H: 36px, 全宽          带焦点环的文本输入
  Dropdown Menu      H: 36px, W: 200px      Select with search           下拉菜单       H: 36px, W: 200px      带搜索的选择控件
  Tooltip            Auto, Max-W: 280px     Contextual help              工具提示       自动, Max-W: 280px     上下文帮助
  Badge              H: 20px, Auto-W        Status indicator             徽章           H: 20px, 自动宽        状态指示器
  Card               Pad: 16px, R: 8px      Elevated container           卡片           Pad: 16px, R: 8px      提升容器
  Tab Bar            H: 40px                Navigation tabs              标签栏         H: 40px                导航标签
  Avatar             32/40/48px circle      User/agent avatar            头像           32/40/48px 圆形        用户/智能体头像
  Progress Bar       H: 4px, Full-width     Linear progress              进度条         H: 4px, 全宽           线性进度
  Modal Dialog       Min: 480x320px         Overlay dialog               模态对话框     Min: 480x320px         覆盖对话框
  Toast              W: 360px, Auto-H       Auto-dismiss msg             消息通知       W: 360px, 自动高       临时消息
  ------------------ ---------------------- ---------------------------- -------------- ---------------------- --------------------

**Custom Components for AgentForge / AgentForge自定义组件**

> • AgentCard: Agent info with role icon, name, status, skills --- AgentCard：智能体信息，包括角色图标、状态、技能
>
> • WorkflowNode: DAG node with I/O ports, status, assignment --- WorkflowNode：DAG节点，带输入输出端口
>
> • ChatBubble: Message with sender identity, timestamp, avatar --- ChatBubble：消息容器，带发送者身份和头像
>
> • KnowledgeNode: Tree item with expand/collapse, type icon --- KnowledgeNode：知识库树形项目，带展开折叠
>
> • MetricCard: Dashboard card with value, trend, sparkline --- MetricCard：仪表板卡片，带值、趋势和迷你图
>
> • StatusDot: 8px circle with pulse animation --- StatusDot：8px圆形指示器，带脉冲动画

**Sidebar Navigation / 侧边栏导航**

**Icon Rail Design / 图标栏设计**

The sidebar uses a slim icon rail design, approximately 60px wide, with filled icons for active states and outlined icons for inactive states. Background: `#1A1F2E`, with the AgentForge logo at the top. Selected state indicated by filled icons vs. outlined inactive states. 侧边栏使用细图标栏设计，约60px宽。活动状态使用填充图标，非活动状态使用轮廓图标。

**Top Section Icons / 顶部区域图标**

> • AgentForge Logo: Application branding at top --- AgentForge标志：顶部应用品牌
>
> • Teams (Users): Agent team hierarchy and management --- 团队：智能体团队层级和管理
>
> • Chat (MessageSquare): Communication with agents --- 聊天：与智能体通信
>
> • Tools (Wrench): Tool configuration and MCP management --- 工具：工具配置和MCP管理
>
> • Settings (Settings): App settings and theme configuration --- 设置：应用设置和主题配置

**Bottom Section Icons / 底部区域图标**

> • User Profile (UserCircle): Account and workspace management --- 用户资料：账户和工作区管理

**Icon States / 图标状态**

Two visual states: Active (filled icon, `#00D4AA` accent highlight) and Inactive (outlined icon, `#9CA3AF` muted color). Hover states show subtle background highlight. Transitions: 150ms ease-in-out. 两种视觉状态：活动（填充图标，`#00D4AA`强调色高亮）和非活动（轮廓图标，`#9CA3AF`柔和色）。悬停显示微妙背景高亮。

**Team Workspace Navigation View / 团队工作区导航视图**

The Team Workspace provides a collaborative environment for managing agent teams. It features a team composition view showing all team members with their assigned roles and provider bindings (e.g., Team Leader/Claude, Architect/Gemini, Backend Dev/Codex, Frontend Dev/iFlow, Tester/OpenCode). The workspace includes a shared task board (SharedTaskList) with Kanban-style columns for task status tracking (pending, claimed, in-progress, completed), a real-time communication feed showing inter-agent messages, and a team performance metrics panel displaying task completion rates, average response times, and resource utilization per member. 团队工作区提供管理智能体团队的协作环境。包含团队组成视图、共享任务看板、实时通信流和团队性能指标面板。

**Team Hierarchy Panel / 团队层级面板**

**Tree Structure Design / 树结构设计**

Hierarchical tree showing "AGENT TEAMS" as the root header. Each team node is expandable/collapsible with chevron indicators. Sub-items include agent configurations, version branches, and individual agents with colored status dots (green = active/healthy, yellow = waiting/in-progress, red = error/failed). Bottom section includes a "Create New Team" (新建团队) action button. 层次树结构，以"AGENT TEAMS"为根标题。每个团队节点可展开/折叠。子项包含智能体配置、版本分支和带状态指示器的智能体。

**Status Indicators / 状态指示器**

> • Active/Healthy: Green dot --- 活跃/健康：绿色圆点
>
> • In-Progress/Waiting: Yellow/Orange dot --- 进行中/等待：黄色/橙色圆点
>
> • Error/Failed: Red dot --- 错误/失败：红色圆点

**Main Contextual Workspace / 主上下文工作区**

**Tab System Design / 标签系统设计**

The main workspace uses a tabbed interface with primary mode tabs: "设计" (Design), "运行" (Run), "测试" (Test), and additional contextual tabs. The active tab is highlighted with the accent color (`#00D4AA`). The workspace content adapts based on the selected tab — switching between code/configuration views, visual canvas views, and runtime monitoring views. 主工作区使用标签界面，包含主要模式标签：设计、运行、测试等。活动标签以强调色高亮。工作区内容根据所选标签自适应切换。

**Task/Agent List Panel / 任务/智能体列表面板**

The left sub-panel within the workspace displays a vertical list of agent tasks with status badges. Each item shows: task name, status label (已完成/Completed, 进行中/In Progress, 空闲/Idle), a progress bar with gradient fill, and action buttons. Tasks are color-coded by status: green for complete, orange for in-progress, gray for idle. Example tasks include: "解析与服务编排" (Parsing and Service Orchestration), "质量与合规测试" (Quality and Compliance Testing), "交付协调与汇总" (Delivery Coordination and Summary). 工作区左侧子面板显示智能体任务列表，每项包含任务名称、状态标签、进度条和操作按钮。

**Code Editor / 代码编辑器**

Built on gpui-component-code-editor with Tree Sitter syntax highlighting for Rust, TypeScript, Python, YAML. LSP integration for auto-completion, go-to-definition, find-references, diagnostics. Includes line numbers, code folding, bracket matching, minimap. 基于gpui-component-code-editor，带Tree Sitter语法高亮。LSP集成自动补全、跳转定义、查找引用、诊断。包括行号、代码折叠、括号匹配、迷你图。

**Markdown Editor / Markdown编辑器**

Uses MDXEditor or Milkdown for rich text editing with real-time preview. Supports GFM extensions, KaTeX math, mermaid diagrams, and custom agent directives. Split-view mode with synchronized scrolling. 使用MDXEditor或Milkdown进行富文本编辑和实时预览。支持GFM扩展、KaTeX数学、mermaid图表、自定义智能体指令。分割视图同步滚动。

**Code Highlighting / 代码高亮**

Prism.js provides syntax highlighting for web views with dark theme matching AgentForge colors. Supports 200+ languages with custom tokens for keywords, strings, comments, functions, and types. Prism.js提供与AgentForge配色匹配的暗色主题语法高亮。支持200+语言。

**Inspector/Detail Panel / 检查器/详情面板**

The right sub-panel displays contextual detail information based on the selected task or agent. It includes: a header with task title and status badge, a role definition section describing the agent's purpose, workflow documentation with numbered steps, error message display area (e.g., authentication errors with red styling), task assignment notifications with structured data, and an input field at the bottom for user interaction. The panel follows a progressive disclosure pattern: role definition → workflow → current state. 右侧子面板显示基于所选任务或智能体的上下文详情。包含任务标题和状态徽章、角色定义、工作流文档、错误消息显示区域和用户交互输入框。

**Floating Input / 浮动输入**

A persistent floating input field at the bottom of the workspace provides natural language interaction. It supports text input for requests, commands, and agent communication. The input field uses the card surface background (`#252B3A`) with subtle border and focus glow effect. 工作区底部持久浮动输入字段提供自然语言交互，支持文本输入、命令和智能体通信。

**Agent Team Layout / 智能体团队布局**

**Team Hierarchy Panel / 团队层级面板**

Displays organizational structure of agent teams in tree format. Shows team leaders, sub-teams, and individual agents with roles and status. Supports drag-and-drop reorganization, inline renaming, and quick-add. 以树形格式显示智能体团队组织结构。显示团队领导者、子团队和单个智能体。支持拖放重组、内联重命名。

**Agent Cards Design / 智能体卡片设计**

Each agent is represented by a distinctive pixel-art style avatar (resembling Minecraft-style characters) with different colored shirts for identification. Agent cards include: task name, status badge (已完成/进行中/空闲 with color-coded labels), progress bar with gradient fill, and quick-action buttons. Cards use the elevated panel background (`#252B3A`) with consistent border-radius (6-8px). Status colors: green (`#00D4AA`) for complete, orange (`#F59E0B`) for in-progress, gray for idle. 每个智能体由独特的像素艺术风格头像表示（类似Minecraft风格角色），不同颜色的衬衫用于识别。智能体卡片包含任务名称、状态徽章、渐变填充进度条和快捷操作按钮。

**Team Status Indicators / 团队状态指示器**

> • All Running: Green dots on all agent nodes --- 全部运行：所有智能体节点显示绿色圆点
>
> • Partial Running: Mix of green and orange dots --- 部分运行：绿色和橙色圆点混合
>
> • Error State: Red dots with error count --- 错误状态：红色圆点带错误计数
>
> • Idle: Gray status labels, no animation --- 空闲：灰色状态标签，无动画

**Agent Communication Visualization / 智能体通信可视化**

Animated connection lines between agent cards show communication. Line color matches sender accent, animation speed varies by urgency. Communication log panel shows detailed message history. 动画连接线显示智能体间通信。线条颜色与发送者强调色匹配。可切换通信日志面板。

**Team Workspace Visual Reference / 团队工作区视觉参考**

The Agent Team Workspace centers on the organizational hierarchy of agent teams displayed in a tree format. Each agent is represented by a card containing: a role icon (32px, Lucide icons), the agent name (bold, primary color), a status badge (8px dot + label indicating active/running/idle/suspended/retired), a skills count, and quick-action buttons (configure, run, pause, logs). Cards have a 12px border radius with a `#21262D` background and a left border colored by status. The team container shows animated connection lines between collaborating agents, with line colors matching the sender's accent color. A communication log panel provides detailed message history with filtering by agent, type, and time range. 团队工作区以树形格式显示智能体团队的组织层级。每个智能体由卡片表示，包含角色图标、名称、状态徽章、技能计数和快捷操作。

**Communication Area / 通信区域**

**Chat Interface Design / 聊天界面设计**

Real-time chat interface for interacting with AI agents. Occupies lower workspace or full panel. Features scrollable message history, agent identity headers, and rich input area. 与AI智能体交互的实时聊天界面。占据主工作区下部或完整面板。

**Message Bubbles / 消息气泡**

> • User Messages: Right-aligned, blue bg (\#3B82F6 15%), white text --- 用户消息：右对齐，蓝色背景，白色文本
>
> • AI Agent Messages: Left-aligned, card surface (\#21262D), avatar --- AI智能体消息：左对齐，卡片表面，带头像
>
> • System Messages: Centered, muted (\#8B949E), italic --- 系统消息：居中，柔和灰色，斜体
>
> • Error Messages: Red-tinted bg (\#EF4444 10%), error icon --- 错误消息：红色色调背景，错误图标

**Agent Identity in Messages / 消息中的智能体身份**

Each AI message includes agent avatar (32px circle), display name (bold, agent accent color), role label (caption), and timestamp. Multi-agent responses show group header with all participants. 每条AI消息包括头像（32px圆形）、名称（粗体）、角色标签和时间戳。多智能体响应显示组标题。

**Input Area with Actions / 带操作的输入区域**

Multi-line text input with auto-resize (max 6 lines), send button (blue accent), action buttons for: attach files, insert code blocks, mention agents (@), voice input. Supports Markdown preview, slash commands (/), and @-mentions. 多行文本输入（最多6行），发送按钮，操作按钮：附加文件、插入代码块、\@提及智能体、语音输入。支持Markdown预览、/命令、\@提及。

**Voice Input Support / 语音输入支持**

Uses Web Speech API for speech-to-text. Pulsing microphone icon with real-time transcription preview. Voice commands for navigation, agent control, and message dictation. Multi-language with auto-detection. 使用Web Speech API语音转文本。脉冲麦克风图标带实时转录预览。语音命令用于导航、控制智能体和口述消息。多语言自动检测。

**iFlow Workflow Designer / 智能工作流设计器**

**DAG Canvas Design / DAG画布设计**

Infinite canvas for building DAG-based workflows. Supports pan (scroll/drag), zoom (Ctrl+scroll, 25%-400%), selection (click, shift-click, drag-select). Minimap in bottom-right. Grid background with subtle dots (\#30363D at 30% opacity). 用于构建DAG工作流的无限画布。支持平移、缩放（25%-400%）、选择。右下角迷你图。网格背景带微妙点。

**Workflow Node Types / 工作流节点类型**

> • Trigger Node: Entry point (webhook, schedule, manual) --- 触发节点：入口点（webhook、计划、手动）
>
> • Agent Task Node: Assigns work to agent with I/O ports --- 智能体任务节点：将工作分配给智能体
>
> • Decision Node: Conditional branching with rules --- 决策节点：带规则的条件分支
>
> • Transform Node: Data transformation and mapping --- 转换节点：数据转换和映射
>
> • Knowledge Node: Retrieve/store from knowledge base --- 知识节点：从知识库检索或存储
>
> • Output Node: Terminal for results and notifications --- 输出节点：结果和通知的终端

**Connection & Routing Visualization / 连接与路由可视化**

Smooth Bezier curves with animated flow indicators. Active connections show pulsing dots for data flow direction. Colors: blue (text/data), green (success), orange (conditional), red (error). Connection ports are 12px circles with hover glow. 平滑贝塞尔曲线带动画流动指示器。活动连接显示脉冲点。颜色：蓝色、绿色、橙色、红色。连接端口12px圆形。

**Agent Assignment in Workflows / 工作流中的智能体分配**

Agent Task Nodes include assignment panel to select agents from team roster. Shows availability, workload, skill compatibility. Drag-and-drop from team panel to node creates assignment. Assigned agents appear as small avatars on the node. 智能体任务节点包含分配面板。显示可用性、工作负载、技能兼容性。拖放创建分配。已分配智能体作为小头像显示。

**iFlow Workflow Designer Visual / iFlow工作流设计器视觉**

The iFlow Workflow Designer provides a visual DAG (Directed Acyclic Graph) canvas for building and managing workflows. The canvas supports pan (scroll/drag), zoom (Ctrl+scroll, 25%-400%), and selection (click, shift-click, drag-select) with a minimap in the bottom-right corner. A drag-and-drop node palette on the left provides step types including: Trigger Nodes (webhook, schedule, manual), Agent Task Nodes (with agent assignment panel showing availability, workload, and skill compatibility), Decision Nodes (conditional branching with AND/OR/NOT logic), Transform Nodes (data transformation and mapping), Knowledge Nodes (retrieve/store from knowledge base), and Output Nodes (results and notifications). Connections between nodes use smooth Bezier curves with animated flow indicators showing data direction. A property panel on the right allows configuration of selected nodes including agent assignment, parameters, conditions, and retry policies. Real-time execution preview shows step status (pending, running, completed, failed), execution timing, and bottleneck identification. The designer also includes a workflow template library, version control panel, and input/output schema editor. iFlow工作流设计器提供可视化DAG画布，支持拖放节点组合、贝塞尔曲线连接、实时执行预览和工作流模板库。

**Visualization Area / 可视化区域**

**Image/Graph Rendering Panel / 图像/图表渲染面板**

Renders images, charts, and graph data within the workspace. Supports PNG, SVG, and Mermaid diagrams. Includes zoom controls, pan navigation, and export button. 在工作区内渲染图像、图表和图形数据。支持PNG、SVG和Mermaid图表。包含缩放控件和导出按钮。

**Isometric Office View / 等轴测视图**

Spatial representation of agent teams in an isometric office. Each agent depicted as an avatar with activity cues: coding (terminal), reviewing (document), communicating (chat bubble), or idle (coffee cup). 等轴测办公室中智能体团队的空间表示。每个智能体带活动提示：编码、审查、通信或空闲。

**Chart Components / 图表组件**

Integrates Recharts for data visualization. Supported types: Line (time-series), Bar (token usage), Donut/Pie (resource distribution), Area (cumulative metrics), Scatter (correlation). All use AgentForge palette with interactive tooltips. 集成Recharts数据可视化。支持：折线图、柱状图、环形/饼图、面积图、散点图。使用AgentForge配色方案。

**Real-time Communication Visualization / 实时通信可视化**

Network graph rendering real-time agent communication. Nodes = agents, edges = channels. Edge thickness = frequency, color = type. Updates dynamically as agents interact. 网络图渲染实时智能体通信。节点=智能体，边=通道。边粗细=频率，颜色=类型。动态更新。

**Monitoring & Observability / 监控与可观测性**

**Dashboard Metrics Cards / 仪表板指标卡片**

Key metrics displayed as cards at the top: large numeric value, trend indicator (arrow + percentage), sparkline (24h), comparison to previous period. Responsive grid layout. 关键指标以卡片形式显示：大数字值、趋势指示器、迷你图（24h）、与上期比较。响应式网格布局。

**Token Usage Charts / 令牌使用图表**

Bar chart: daily token consumption by agent (stacked input/output). Donut chart: distribution across teams. Date range selection, agent filtering, hover tooltips with exact counts and estimated costs. 柱状图：按智能体的每日令牌消耗。环形图：团队分布。日期范围选择、智能体过滤、悬停提示。

**Activity Feed / 活动流**

Real-time scrollable feed showing agent activities: task started, completed, failed, workflow triggered, knowledge updated. Each entry has timestamp, agent avatar, activity type icon, and description. Supports filtering by agent, type, and time range. 实时可滚动活动流：任务启动、完成、失败、工作流触发、知识更新。每条记录带时间戳、头像、类型图标。支持过滤。

**Agent Health Monitoring / 智能体健康监控**

Individual agent health cards showing: uptime percentage, average response time, error rate, token consumption trend, last activity timestamp. Color-coded status (green/yellow/red) based on health score thresholds. 单个智能体健康卡片：正常运行时间百分比、平均响应时间、错误率、令牌消耗趋势、最后活动时间戳。基于健康分数阈值的颜色编码。

**Monitoring Dashboard Visual / 监控仪表板视觉**

The Monitoring & Observability dashboard provides comprehensive real-time system visibility. The top section displays metric cards with large numeric values, trend indicators (arrow + percentage), sparklines (24h), and comparison to previous periods. The dashboard includes: an Agent Communication Graph (interactive network visualization showing communication patterns and collaboration networks between agents), a Workflow Execution Timeline (Gantt-style timeline showing workflow progress and step durations), Token Usage Charts (daily trend bar chart, distribution pie chart, cost estimation panel with per-agent/per-provider/per-task breakdowns), a Security Event Log Viewer with filtering and search, a Performance Metrics Panel (response latency, throughput, provider availability, agent utilization), and an Alert Management Panel for budget overruns, provider outages, security incidents, and workflow failures. Multiple view modes are supported: Network view, Timeline view, Kanban view, and Gantt view. All visualizations support real-time data updates, interactive exploration (zoom, pan, click for details), and data export to external systems such as Prometheus and Grafana. 监控与可观测性仪表板提供全面的实时系统可见性。包含智能体通信图、工作流执行时间线、令牌使用图表、安全事件日志和告警管理面板。

**Knowledge Base (Brains) Interface / 知识库界面**

**Knowledge Tree Navigation / 知识树导航**

Hierarchical tree navigation for knowledge base with expand/collapse. Each node shows type icon (document, folder, brain), title, and token count. Supports drag-and-drop reorganization, search filtering, and bulk operations. 知识库层次树导航，带展开折叠。每个节点显示类型图标、标题和令牌计数。支持拖放重组、搜索过滤、批量操作。

**Document Viewer with Markdown Rendering / 带Markdown渲染的文档查看器**

Full-featured document viewer rendering Markdown with syntax highlighting, KaTeX math, mermaid diagrams, and embedded code blocks. Supports table of contents sidebar, backlinks panel, and full-text search within documents. 全功能文档查看器，渲染Markdown带语法高亮、KaTeX数学、mermaid图表和嵌入代码块。支持目录侧边栏、反向链接面板和全文搜索。

**Obsidian Integration Panel / Obsidian集成面板**

Dedicated panel for Obsidian vault integration. Shows vault structure, allows linking between AgentForge knowledge and Obsidian notes. Supports bidirectional sync, graph view, and tag-based navigation. 专用于Obsidian仓库集成的面板。显示仓库结构，允许AgentForge知识和Obsidian笔记之间链接。支持双向同步、图视图和标签导航。

**Token Optimization Settings / 令牌优化设置**

Configuration panel for token optimization: chunk size settings, overlap configuration, embedding model selection, compression level, and cache management. Shows estimated token savings and retrieval accuracy metrics. 令牌优化配置面板：分块大小、重叠配置、嵌入模型选择、压缩级别、缓存管理。显示估计令牌节省和检索准确度指标。

**Knowledge Base Visual / 知识库视觉**

The Knowledge Base (Brains) interface provides a comprehensive knowledge management environment. Features an infinite canvas or structured graph view for visualizing knowledge relationships. Supports pan (scroll/drag), zoom (Ctrl+scroll, 25%-400%), and selection. Minimap in bottom-right. Grid background with subtle dots. 知识库（Brains）界面提供全面的知识管理环境。采用无限画布或结构化图视图来可视化知识关系。支持平移、缩放（25%-400%）和选择。右下角包含迷你图（Minimap）。网格背景带微妙点。 The left panel contains a hierarchical navigation tree with expand/collapse functionality, where each node displays a type icon (document, folder, brain), title, and token count. A search bar at the top supports semantic search with keyword matching, tag filtering, and content similarity scoring. The main content area features a full-featured Markdown document viewer with syntax highlighting, KaTeX math rendering, Mermaid diagram support, and embedded code blocks. A table of contents sidebar and backlinks panel support navigation between related entries using Obsidian-style `[[wiki links]]`. The right panel provides an Obsidian Integration Panel showing vault structure, sync status indicators (synced, pending, conflict), and bidirectional sync controls. A Knowledge Graph Visualization offers an interactive graph showing relationships between knowledge entries, agents, and workflows. Additional panels include tag management, version history with diff viewing, a knowledge template selector (procedures, reference documents, meeting notes, decision records, agent briefings), and token optimization settings (chunk size, overlap configuration, embedding model selection, compression level, cache management) with estimated token savings and retrieval accuracy metrics. 知识库界面提供全面的知识管理环境。包含层次导航树、语义搜索、Markdown文档查看器、Obsidian集成面板和知识图谱可视化。

**Screen Descriptions / 屏幕描述**

**Screen 1: Main Dashboard / 屏幕 1：主仪表板**

The main dashboard provides an at-a-glance overview of the entire AgentForge system using a configurable widget layout. It displays: agent team status cards with real-time health indicators, active workflow status indicators, a real-time activity feed showing system events (task started, completed, failed, workflow triggered, knowledge updated), system health metrics, token consumption trends (7-day bar chart), token distribution by project (pie chart), a cost estimation panel with real-time forecasting, and a recent alerts section. Users can rearrange widgets to their preference. The layout uses the full application shell with sidebar navigation, file tree panel, and main workspace area. 主仪表板使用可配置部件布局提供AgentForge系统概览。显示智能体团队状态卡片、活动工作流指示器、实时活动流、系统健康指标、令牌消耗趋势、成本预估面板和最近警报。

**Screen 2: Agent Manager / 屏幕 2：智能体管理器**

The Agent Manager provides a comprehensive CRUD interface for managing AI agents. It features an agent list/grid view with real-time status indicators (active, running, idle, suspended, retired), an agent creation form with configurable properties (name, description, role, provider binding with primary/fallback selection, system prompt editor with template variables, temperature/max tokens/top-p parameters, memory configuration with short-term/working/long-term tiers, skill assignments from the Skill Registry, MCP tool assignments, and resource limits), an agent detail/configuration panel, an agent template library for common use cases (code review, documentation, data analysis, customer support), a version history panel with compare and rollback capabilities, and an agent monitoring dashboard showing real-time status, active sessions, token consumption, response latency, and error rates. 智能体管理器提供全面的CRUD界面。包含智能体列表/网格视图、创建表单、配置面板、模板库、版本历史和监控仪表板。

**Screen 3: Team Workspace / 屏幕 3：团队工作区**

The team workspace displays the organizational hierarchy of agent teams with a team composition view showing all members, their assigned roles, and provider bindings (e.g., Team Leader/Claude, Architect/Gemini, Backend Dev/Codex, Frontend Dev/iFlow, Tester/OpenCode). It includes a shared task board (SharedTaskList) with Kanban-style columns for task status tracking (pending, claimed, in-progress, completed) with priority ordering and assignment tracking, a real-time communication feed showing inter-agent messages (broadcast, direct, role-group), a team performance metrics panel (task completion rate, average response time, communication volume, resource utilization), and team configuration/settings panels. The workspace supports dynamic addition/removal of agents, cross-team collaboration through controlled channels, and conflict resolution UI for contradictory outputs. 团队工作区显示智能体团队的组织层级。包含团队组成视图、共享任务看板、实时通信流、团队性能指标面板和团队配置面板。

**Screen 4: iFlow Workflow Designer / 屏幕 4：iFlow工作流设计器**

The workflow designer provides an infinite DAG canvas for building workflows through drag-and-drop composition. Users can add Trigger Nodes (webhook, schedule, manual), Agent Task Nodes (with assignment panel showing availability, workload, skill compatibility), Decision Nodes (conditional branching with AND/OR/NOT logic), Transform Nodes (data transformation and mapping), Knowledge Nodes (retrieve/store from knowledge base), and Output Nodes (results and notifications). Connections use animated Bezier curves showing data flow direction. A property panel allows node configuration including parameters, conditions, and retry policies. Real-time execution preview shows step status, execution timing, and bottleneck identification. The designer includes a workflow template library (CI/CD, code review, testing, documentation, deployment), version control with branching, input/output schema editor, and real-time validation. 工作流设计器提供无限DAG画布，通过拖放组合构建工作流。支持触发、任务、决策、转换、知识和输出节点，带实时执行预览和工作流模板库。

**Screen 5: Session Manager / 屏幕 5：会话管理器**

The session manager provides a chat-style conversation interface for interacting with AI agents. It features a session list for browsing active and historical sessions, a conversation view with streaming response display, context management panel showing the context window with automatic summarization indicators, and session controls for creating, resuming, pausing, branching, and merging sessions. A mode indicator/selector allows switching between operating modes (Human Interaction, Supervision, Autonomous, Debug, Simulation, Batch, Scheduled, Reactive, Learning, Maintenance). In Human Interaction Mode, every agent action requires explicit human approval with proposed actions and expected impact presented for review. In Supervision Mode, real-time visualization of all agent communications is provided with the ability to intervene, pause, redirect, or terminate. Sessions are persistable, resumable, and searchable with cross-session context sharing support. 会话管理器提供聊天式对话界面。包含会话列表、流式响应显示、上下文管理面板和会话控制。支持多种操作模式切换。

**Screen 6: Monitoring Console / 屏幕 6：监控控制台**

The monitoring console provides comprehensive real-time observability. It includes: an Agent Communication Graph (interactive network visualization of communication patterns), a Workflow Execution Timeline (Gantt-style timeline with step durations), Token Usage Charts (daily trend, distribution pie chart, cost estimation with per-agent/per-provider/per-task breakdowns), a Security Event Log Viewer with filtering and search, a Performance Metrics Panel (response latency, throughput, provider availability, agent utilization), and an Alert Management Panel for budget overruns, provider outages, security incidents, and workflow failures. Multiple view modes are supported: Network view, Timeline view, Kanban view, and Gantt view. All visualizations support real-time updates, interactive exploration, and data export to external systems (Prometheus, Grafana). 监控控制台提供全面的实时可观测性。包含智能体通信图、工作流执行时间线、令牌使用图表、安全事件日志和告警管理面板。支持多种视图模式。

**Screen 7: Knowledge Explorer / 屏幕 7：知识浏览器**

The knowledge explorer provides a hierarchical knowledge browser with a navigation tree, semantic search bar (keyword matching, tag filtering, content similarity scoring), a full Markdown document viewer with syntax highlighting, KaTeX math, Mermaid diagrams, and embedded code blocks. It includes a table of contents sidebar, backlinks panel with Obsidian-style `[[wiki links]]`, an Obsidian Integration Panel with vault structure display and sync status indicators (synced, pending, conflict), a Knowledge Graph Visualization showing relationships between entries, agents, and workflows, tag management, version history with diff viewing, a knowledge template selector (procedures, reference documents, meeting notes, decision records, agent briefings), an auto-extraction queue for reviewing knowledge extracted from agent interactions, and token optimization settings (chunk size, overlap, embedding model, compression, cache) with estimated savings and accuracy metrics. 知识浏览器提供层次知识浏览、语义搜索、Markdown文档查看器、Obsidian集成面板和知识图谱可视化。

**Screen 8: Settings & Configuration / 屏幕 8：设置与配置**

The settings screen provides comprehensive configuration organized in a tabbed layout with sidebar navigation. Sections include: Provider Configuration (manage AI provider connections, API keys, endpoints), Security Policy Configuration (RBAC role definitions, permissions, data masking rules), API Key Management (centralized encrypted storage with rotation schedules and usage tracking), User Preferences (theme selection, language, notification preferences), Token Budget Configuration (allocation at system, team, agent, and session levels), Data Retention Policy Settings (configurable retention for conversation histories, audit logs, metrics, knowledge entries), and Vault Configuration (Obsidian vault paths, synchronization settings, conflict resolution preferences). Configuration changes support hot-reload without application restart. 设置屏幕提供全面的配置界面，包含提供者配置、安全策略、API密钥管理、用户偏好、令牌预算、数据保留策略和仓库配置。

**Technology Stack / 技术栈**

AgentForge is built on a modern technology stack optimized for performance, extensibility, and developer experience. AgentForge基于现代技术栈构建，针对性能、可扩展性和开发体验优化。

  ---------------- ---------------------------- --------------------------- ------------ ---------------------------- ----------------
  **Technology**   **Library/Tool**             **Purpose**                 **技术**     **库/工具**                  **用途**
  UI Framework     GPUI (Rust)                  GPU-accelerated rendering   UI框架       GPUI (Rust)                  GPU加速渲染
  Components       longbridge/gpui-component    Core UI components          组件         longbridge/gpui-component    核心UI组件
  Code Editor      gpui-component-code-editor   Syntax highlighting, LSP    代码编辑器   gpui-component-code-editor   语法高亮、LSP
  Dock Layout      gpui-component-dock          Dockable panel system       停靠布局     gpui-component-dock          可停靠面板系统
  Markdown         MDXEditor / Milkdown         Rich text editing           Markdown     MDXEditor / Milkdown         富文本编辑
  Code Highlight   Prism.js / Tree Sitter       Syntax highlighting         代码高亮     Prism.js / Tree Sitter       语法高亮
  Charts           Recharts                     Data visualization          图表         Recharts                     数据可视化
  Icons            Lucide Icons                 Icon library                图标         Lucide Icons                 图标库
  Math             KaTeX                        Math rendering              数学         KaTeX                        数学渲染
  Diagrams         Mermaid                      Diagram generation          图表         Mermaid                      图表生成
  ---------------- ---------------------------- --------------------------- ------------ ---------------------------- ----------------

**Responsive Behavior / 响应式行为**

**Minimum Window Size / 最小窗口尺寸**

Minimum supported window size is 1024x768 pixels. Below this threshold, panels automatically collapse and the interface enters a simplified single-column layout. A warning notification appears suggesting a larger window. 最小支持窗口尺寸为1024x768像素。低于此阈值，面板自动折叠，界面进入简化单列布局。

**Panel Collapse/Expand / 面板折叠/展开**

All panels support collapse/expand via sidebar icon toggle, keyboard shortcut (Ctrl+B for file tree, Ctrl+J for auxiliary panel), or double-clicking the panel divider. Collapsed panels show a thin strip with an expand arrow. Panel state is persisted across sessions. 所有面板支持通过侧边栏图标切换、键盘快捷键或双击分割线折叠/展开。折叠的面板显示带展开箭头的细条。状态跨会话保持。

**Focus Mode / 专注模式**

Focus mode (Ctrl+Shift+F) hides all panels except the main workspace, maximizing the editing area. A minimal floating toolbar provides access to essential actions. Pressing Escape exits focus mode and restores the previous panel configuration. 专注模式（Ctrl+Shift+F）隐藏除主工作区外的所有面板，最大化编辑区域。浮动工具栏提供基本操作。Escape退出并恢复配置。

**Accessibility / 无障碍设计**

**Operating Modes UI / 操作模式界面**

AgentForge supports multiple operating modes, each with distinct UI behaviors and visual indicators. A central Mode Switch mechanism allows dynamic switching between modes without interrupting active sessions. Different teams can operate in different modes simultaneously. 智能体支持多种操作模式，每种模式具有独特的UI行为和视觉指示器。

> • Human Interaction Mode (HIM): Direct chat interface where every agent action requires explicit human approval. Proposed actions and expected impact are presented for review before execution. Visual indicator: Blue accent (`#3B82F6`). --- 人机交互模式：直接聊天界面，每个智能体操作需要明确的人工审批。
>
> • Supervision Mode: Agents communicate autonomously while the human observes via live visualization. Real-time status updates flow from agents to the observer. Human can intervene, pause, redirect, or terminate at any point. Anomalous behavior triggers escalation UI. Visual indicator: Orange/Amber accent (`#F59E0B`). --- 监督模式：智能体自主通信，人类通过实时可视化观察。
>
> • Autonomous Mode: Agents operate independently with minimal UI interaction. Notifications sent upon task completion. Results compiled into reports. Safety limits enforced (task boundaries, resource limits, escalation triggers, mandatory check-in intervals). Visual indicator: Green accent (`#00D4AA`). --- 自主模式：智能体独立运行，最小化UI交互。
>
> • Debug Mode: Enhanced logging, step-by-step execution, variable inspection, and breakpoint capabilities. --- 调试模式：增强日志、逐步执行、变量检查和断点功能。
>
> • Simulation Mode: Agent actions are predicted and displayed without actual execution (preview before commit). --- 模拟模式：智能体操作被预测和显示，但不实际执行。
>
> • Batch Processing Mode: Progress tracking, per-task error handling, and summary reporting. --- 批处理模式：进度跟踪、逐任务错误处理和汇总报告。
>
> • Scheduled Execution Mode: Cron-like and calendar-based scheduling interface. --- 定时执行模式：类似Cron和基于日历的调度界面。
>
> • Reactive Mode: Event filtering, priority-based response, and escalation procedures. --- 响应模式：事件过滤、基于优先级的响应和升级程序。
>
> • Learning Mode: Performance and feedback collection UI. --- 学习模式：性能和反馈收集界面。
>
> • Maintenance Mode: Non-critical operations suspended; essential monitoring continues. --- 维护模式：非关键操作暂停，基本监控继续。

**Keyboard Navigation / 键盘导航**

Full keyboard navigation support: Tab/Shift+Tab for focus movement, Enter/Space for activation, Arrow keys for list navigation, Escape for dismissal. All interactive elements have visible focus indicators (2px blue outline with 2px offset). Custom keyboard shortcuts are configurable. 完全键盘导航支持：Tab/Shift+Tab移动焦点，Enter/Space激活，方向键列表导航，Escape关闭。所有交互元素有可见焦点指示器。自定义快捷键可配置。

**Screen Reader Support / 屏幕阅读器支持**

All interactive elements include ARIA labels and roles. Dynamic content updates use ARIA live regions. Agent status changes are announced. Charts include accessible data tables as alternatives. Image mockups include descriptive alt text. 所有交互元素包含ARIA标签和角色。动态内容更新使用ARIA live regions。智能体状态变化会被宣告。图表包含可访问数据表。

**Color Contrast Requirements / 颜色对比度要求**

All text meets WCAG AA contrast requirements: primary text (\#E6EDF3 on \#0D1117) = 13.4:1 ratio, secondary text (\#8B949E on \#0D1117) = 5.6:1 ratio. Interactive elements maintain minimum 4.5:1 contrast. Status colors are supplemented with icons and patterns for color-blind accessibility. 所有文本满足WCAG AA对比度要求：主文本13.4:1，次文本5.6:1。交互元素最屏4.5:1。状态颜色补充图标和图案以支持色盲无障碍。
