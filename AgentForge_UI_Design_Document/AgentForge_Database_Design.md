**AgentForge**

Database Design Document

AgentForge 数据库设计文档

Version 1.0 \| April 2026 \| Internal / Confidential

版本 1.0 \| 2026年4月 \| 内部 / 机密

# **1. Introduction / 引言**

## **1.1 Purpose / 目的**

This Database Design Document defines the complete database schema, data
access patterns, migration strategy, and performance optimization
guidelines for AgentForge. It serves as the authoritative reference for
all database-related implementation decisions, ensuring consistency,
reliability, and maintainability of the data persistence layer.

本数据库设计文档定义了AgentForge的完整数据库模式、数据访问模式、迁移策略和性能优化指南。它是所有数据库相关实现决策的权威参考。

## **1.2 Scope / 范围**

This document covers the following areas:

-   Complete schema definition for 14 core database tables

-   Entity-relationship model and domain organization

-   Repository pattern implementation for data access in Rust

-   Migration strategy with versioned SQL scripts

-   Index strategy for query optimization

-   Backup, recovery, and data lifecycle management

-   Security considerations for data protection

## **1.3 References / 参考文档**

-   AgentForge BRD (Business Requirements Document)

-   AgentForge PRD (Product Requirements Document)

-   AgentForge SRS (Software Requirements Specification)

-   AgentForge Technical Specification

-   AgentForge Roadmap and Checklist

-   SQLite Documentation: https://www.sqlite.org/docs.html

-   rusqlite Documentation: https://docs.rs/rusqlite/

# **2. Database Selection & Rationale / 数据库选型与理由**

## **2.1 Why SQLite / 为什么选择SQLite**

SQLite is selected as the primary database engine for AgentForge based
on the following criteria:

-   Embedded: Zero-configuration, no separate server process required.
    Ships as a single file with the application.

-   Serverless: No network overhead, ideal for desktop applications.
    Direct in-process access via rusqlite.

-   ACID Compliant: Full transactional support ensuring data integrity
    even during crashes or power failures.

-   Cross-Platform: Identical behavior on Windows, macOS, and Linux.
    Single database file is portable.

-   WAL Mode: Write-Ahead Logging enables concurrent read access while a
    write is in progress, essential for a responsive desktop UI.

-   Single-File Deployment: The entire database is contained in one
    file, simplifying backup, transfer, and versioning.

-   FTS5: Built-in full-text search extension for knowledge base content
    search.

-   Mature & Battle-Tested: Used by billions of devices worldwide.
    Extremely stable and well-documented.

## **2.2 Database Comparison / 数据库对比**

  ---------------------------------------------------------------------------------------------------
  **Criteria**     **SQLite**    **PostgreSQL**   **DuckDB**        **IndexedDB**       **LMDB**
  ---------------- ------------- ---------------- ----------------- ------------------- -------------
  Deployment       Embedded      Server (separate Embedded          Browser-based       Embedded
                   (single file) process)         (in-process)                          (key-value)

  Configuration    Zero-config   Complex setup    Zero-config       Browser API         Zero-config

  Concurrency      WAL mode (1   MVCC (many       Single-threaded   Transaction-based   Lock-free
                   writer, N     writers)                                               reads
                   readers)                                                             

  ACID             Full          Full             Partial           Partial             Partial

  Full-Text Search FTS5 built-in tsvector         No                No                  No

  Cross-Platform   Excellent     Good             Good              Browser only        Good

  Rust Integration rusqlite      tokio-postgres   duckdb-rs         N/A                 heed (good)
                   (excellent)                                                          

  Binary Size      \~1 MB        \~50 MB          \~10 MB           N/A                 \~200 KB

  Network Access   No            Yes              No                No                  No

  Suitability      BEST FIT      Overkill         Analytics only    Web only            Key-value
                                                                                        only
  ---------------------------------------------------------------------------------------------------

## **2.3 Why rusqlite / 为什么选择rusqlite**

rusqlite is the Rust crate for SQLite access, selected for the following
reasons:

-   Synchronous API: rusqlite provides a synchronous interface that
    integrates naturally with Rust async runtimes (Tokio) via
    spawn_blocking, avoiding callback complexity.

-   Bundled Feature: Compiles SQLite from source, ensuring version
    consistency across all platforms without requiring system SQLite
    installation.

-   Prepared Statements: First-class support for prepared statement
    caching, critical for query performance in high-frequency
    operations.

-   Type Safety: Provides strongly-typed result column access, reducing
    runtime errors.

-   Custom Functions: Supports registering Rust functions as SQLite
    custom functions and aggregations.

-   Session Extension: Supports SQLite session extension for efficient
    change tracking and incremental backups.

-   Active Maintenance: Actively maintained, 1000+ GitHub stars, regular
    releases.

## **2.4 Limitations & Mitigations / 局限性与缓解措施**

  -----------------------------------------------------------------------
  **Limitation**          **Impact**              **Mitigation**
  ----------------------- ----------------------- -----------------------
  Single Writer           Only one write          WAL mode + short
                          transaction at a time   transactions; write
                                                  batching for bulk
                                                  operations

  No Network Access       Cannot be shared across Obsidian vault sync for
                          machines                knowledge; future
                                                  optional PostgreSQL
                                                  backend

  Database Size           Practical limit \~140   Auto-VACUUM; periodic
                          TB                      maintenance; data
                                                  archival for old
                                                  sessions

  No Built-in Replication No native master-slave  SQLite backup API for
                          replication             file-based replication;
                                                  application-level sync

  Limited ALTER TABLE     Cannot drop columns     Versioned migrations
                          before SQLite 3.35.0    with CREATE new +
                                                  COPY + DROP old pattern
  -----------------------------------------------------------------------

# **3. Database Architecture / 数据库架构**

## **3.1 Storage Model / 存储模型**

AgentForge uses a single-file SQLite database stored in the user
application data directory. The database operates in WAL (Write-Ahead
Logging) mode to enable concurrent read access while maintaining ACID
compliance.

Database file location pattern:

{APP_DATA}/agentforge/agentforge.db \# Main database

{APP_DATA}/agentforge/agentforge.db-wal \# Write-ahead log

{APP_DATA}/agentforge/agentforge.db-shm \# Shared memory file

## **3.2 WAL Mode Configuration / WAL模式配置**

WAL mode is enabled at database open time via PRAGMA statements:

PRAGMA journal_mode = WAL;

PRAGMA synchronous = NORMAL;

PRAGMA wal_autocheckpoint = 1000;

PRAGMA busy_timeout = 5000;

WAL mode benefits for AgentForge:

-   Readers do not block writers: The UI can read dashboard data while
    background tasks write agent messages.

-   Writers do not block readers: Agent responses can be displayed in
    real-time while other agents update the database.

-   Better crash recovery: WAL mode is more resistant to data loss
    following application crashes.

## **3.3 Repository Pattern / Repository模式**

All database access is abstracted through the Repository pattern. Each
entity type has a dedicated Rust trait and implementation:

pub trait SessionRepository {

fn create(&self, session: &NewSession) -\> Result\<Session\>;

fn get_by_id(&self, id: &str) -\> Result\<Option\<Session\>\>;

fn list(&self, filter: &SessionFilter) -\> Result\<Vec\<Session\>\>;

fn update(&self, id: &str, updates: &SessionUpdate) -\> Result\<()\>;

fn delete(&self, id: &str) -\> Result\<()\>;

}

Repositories are implemented using rusqlite with prepared statement
caching:

-   SessionRepository: CRUD for AI sessions

-   AgentRepository: Agent lifecycle management

-   TeamRepository: Team and role management

-   TaskRepository: SharedTaskList atomic operations

-   MessageRepository: TeamBus message persistence

-   ConversationRepository: Structured conversation history

-   UsageRepository: Token and cost tracking

-   KnowledgeRepository: Knowledge base CRUD with FTS5

-   McpToolRepository: MCP tool registry

-   AuditLogRepository: Immutable audit trail

## **3.4 Transaction Management / 事务管理**

Transactions are managed at the repository layer with the following
principles:

-   Short-lived transactions: Keep write transactions under 100ms to
    minimize lock contention.

-   Explicit transaction boundaries: Use Rust RAII guards (Transaction
    struct with Drop) to ensure commits or rollbacks.

-   Retry on SQLITE_BUSY: Automatically retry write operations up to 5
    times with exponential backoff.

-   Read transactions: Use IMMEDIATE transactions for reads that may
    escalate to writes.

## **3.5 Connection Lifecycle / 连接生命周期**

A single database connection is maintained for the application lifetime:

-   Primary connection: Used for all write operations (serialized via
    mutex).

-   Read connections: Multiple read-only connections can be opened for
    parallel queries.

-   Connection pooling: A simple pool of read connections is maintained
    for dashboard and monitoring queries.

-   Shutdown: WAL checkpoint is triggered on graceful shutdown to ensure
    data consistency.

# **4. Entity-Relationship Overview / 实体关系概览**

The AgentForge data model is organized into 7 core domains across 14
tables:

## **4.1 Core Domains / 核心域**

  -----------------------------------------------------------------------
  **Domain**              **Tables**              **Description**
  ----------------------- ----------------------- -----------------------
  Provider Management     providers               AI provider
                                                  configurations and
                                                  adapter settings

  Agent Management        agents                  Agent definitions,
                                                  roles, capabilities,
                                                  and provider bindings

  Session Management      sessions, conversations AI session lifecycle
                                                  and structured
                                                  conversation history

  Team Collaboration      teams, team_roles,      Team definition, roles,
                          team_instances,         instances, membership,
                          team_members,           task queue, and
                          team_tasks,             communication
                          team_messages           

  Workflow (iFlow)        (referenced via         DAG workflow execution
                          sessions and agents)    state and step tracking

  Knowledge Base          knowledge_entries       Structured knowledge
                                                  storage with FTS5
                                                  full-text search

  Security & Audit        mcp_tools, audit_log,   Tool registry,
                          usage_stats             immutable audit trail,
                                                  and usage analytics
  -----------------------------------------------------------------------

## **4.2 Key Relationships / 关键关系**

  -----------------------------------------------------------------------
  **Relationship**        **Type**                **Description**
  ----------------------- ----------------------- -----------------------
  Provider -\> Agent      One-to-Many             One provider serves
                                                  multiple agents

  Agent -\> Session       One-to-Many             One agent can have
                                                  multiple sessions

  Team -\> TeamRole       One-to-Many             A team defines multiple
                                                  roles

  Team -\> TeamInstance   One-to-Many             A team template can
                                                  spawn multiple
                                                  instances

  TeamInstance -\>        One-to-Many             An instance has
  TeamMember                                      multiple agent members

  TeamInstance -\>        One-to-Many             An instance manages a
  TeamTask                                        shared task list

  TeamInstance -\>        One-to-Many             An instance logs all
  TeamMessage                                     inter-agent messages

  Session -\>             One-to-Many             A session contains
  Conversation                                    ordered conversation
                                                  turns

  Agent -\>               One-to-Many             An agent accumulates
  KnowledgeEntry                                  knowledge entries

  Team -\> KnowledgeEntry One-to-Many             Team-scoped knowledge
                                                  entries

  TeamTask -\> TeamTask   Self-referential        Parent-child subtask
                                                  relationships

  All Entities -\>        Polymorphic             All mutations are
  AuditLog                                        recorded in audit log
  -----------------------------------------------------------------------

# **5. Detailed Table Specifications / 详细表规范**

This section defines all 14 database tables with complete column
specifications, indexes, foreign keys, sample data, and CREATE TABLE SQL
statements.

## **Table 1: providers / 提供者**

**Purpose: Stores AI provider configurations including adapter type, CLI
command, and connection settings.**

**目的：存储AI提供者配置，包括适配器类型、CLI命令和连接设置。**

**Column Specification / 列规范**

  ---------------------------------------------------------------------------------------------------
  **Column**        **Type**          **Constraints**   **Description / 描述**
  ----------------- ----------------- ----------------- ---------------------------------------------
  id                TEXT              PK                Unique provider identifier / 唯一提供者标识符

  name              TEXT              NN, UNIQUE        Provider name
                                                        (claude/gemini/codex/iflow/opencode/custom)

  adapter_type      TEXT              NN                Adapter class name / 适配器类名

  command           TEXT              NULLABLE          CLI command path / CLI命令路径

  node_version      TEXT              NULLABLE          Required Node.js version (if applicable)

  config            TEXT              NULLABLE          JSON configuration / JSON配置

  api_key_ref       TEXT              NULLABLE          Reference to OS keychain / 密钥存储引用

  status            TEXT              NN, DEFAULT       available/unavailable/error
                                      available         

  is_builtin        INTEGER           DEFAULT 0         1=built-in, 0=custom

  created_at        TEXT              NN                ISO 8601 timestamp

  updated_at        TEXT              NN                ISO 8601 timestamp
  ---------------------------------------------------------------------------------------------------

**Indexes / 索引**

  --------------------------------------------------------------------------
  **Index Name**       **Columns**       **Type**          **Description**
  -------------------- ----------------- ----------------- -----------------
  idx_providers_name   name              UNIQUE            Fast provider
                                                           lookup by name

  --------------------------------------------------------------------------

**Sample Data / 示例数据**

  --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**          **name**   **adapter_type**        **command**       **node_version**   **config**   **api_key_ref**   **status**   **is_builtin**   **created_at**         **updated_at**
  --------------- ---------- ----------------------- ----------------- ------------------ ------------ ----------------- ------------ ---------------- ---------------------- ----------------------
  pv-claude-001   claude     ClaudeSdkAdapter        /usr/bin/claude   None               None         keychain:claude   available    1                2026-04-01T00:00:00Z   2026-04-01T00:00:00Z

  pv-gemini-002   gemini     GeminiHeadlessAdapter   /usr/bin/gemini   None               None         keychain:gemini   available    1                2026-04-01T00:00:00Z   2026-04-01T00:00:00Z
  --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE providers (\
id TEXT PRIMARY KEY,\
name TEXT NOT NULL UNIQUE,\
adapter_type TEXT NOT NULL,\
command TEXT,\
node_version TEXT,\
config TEXT,\
api_key_ref TEXT,\
status TEXT NOT NULL DEFAULT \'available\',\
is_builtin INTEGER DEFAULT 0,\
created_at TEXT NOT NULL,\
updated_at TEXT NOT NULL\
);

## **Table 2: agents / 智能体**

**Purpose: Stores AI agent definitions including role, provider binding,
capabilities, skills, and memory configuration.**

**目的：存储AI智能体定义，包括角色、提供者绑定、能力、技能和记忆配置。**

**Column Specification / 列规范**

  ----------------------------------------------------------------------------------------------------
  **Column**                 **Type**          **Constraints**     **Description / 描述**
  -------------------------- ----------------- ------------------- -----------------------------------
  id                         TEXT              PK                  Unique agent identifier

  name                       TEXT              NN, UNIQUE          Agent display name / 智能体显示名称

  role                       TEXT              NN                  Role title (e.g., Architect,
                                                                   Backend Engineer)

  description                TEXT              NULLABLE            Agent description / 智能体描述

  provider_id                TEXT              FK-\>providers.id   Primary AI provider / 主要AI提供者

  system_prompt              TEXT              NULLABLE            System prompt template /
                                                                   系统提示模板

  capabilities               TEXT              NULLABLE            JSON array of capabilities

  skills                     TEXT              NULLABLE            JSON array of skill IDs

  memory_config              TEXT              NULLABLE            JSON memory configuration

  knowledge_scope            TEXT              NULLABLE            JSON knowledge domain filters

  status                     TEXT              NN, DEFAULT         inactive/active/suspended/retired
                                               inactive            

  max_concurrent_sessions    INTEGER           DEFAULT 3           Max parallel sessions

  token_budget_per_session   INTEGER           DEFAULT 0           Token budget (0=unlimited)

  created_at                 TEXT              NN                  ISO 8601 timestamp

  updated_at                 TEXT              NN                  ISO 8601 timestamp

  retired_at                 TEXT              NULLABLE            ISO 8601 timestamp
  ----------------------------------------------------------------------------------------------------

**Indexes / 索引**

  ---------------------------------------------------------------------------
  **Index Name**        **Columns**       **Type**          **Description**
  --------------------- ----------------- ----------------- -----------------
  idx_agents_provider   provider_id       INDEX             Find agents by
                                                            provider

  idx_agents_status     status            INDEX             Filter agents by
                                                            status
  ---------------------------------------------------------------------------

**Foreign Keys / 外键**

  -----------------------------------------------------------------------
  **Column**              **References**          **On Delete**
  ----------------------- ----------------------- -----------------------
  provider_id             providers(id)           CASCADE

  -----------------------------------------------------------------------

**Sample Data / 示例数据**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**        **name**    **role**    **description**   **provider_id**   **system_prompt**   **capabilities**                 **skills**                        **memory_config**   **knowledge_scope**   **status**   **max_concurrent_sessions**   **token_budget_per_session**   **created_at**         **updated_at**         **retired_at**
  ------------- ----------- ----------- ----------------- ----------------- ------------------- -------------------------------- --------------------------------- ------------------- --------------------- ------------ ----------------------------- ------------------------------ ---------------------- ---------------------- ----------------
  ag-arch-001   Chief       Architect   Designs system    pv-claude-001     You are a senior    \[\"code_review\",\"design\"\]   \[\"skill-001\",\"skill-002\"\]   None                None                  active       5                             100000                         2026-04-01T00:00:00Z   2026-04-01T00:00:00Z   None
                Architect               architecture                        architect\...                                                                                                                                                                                                                                            

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE agents (\
id TEXT PRIMARY KEY,\
name TEXT NOT NULL UNIQUE,\
role TEXT NOT NULL,\
description TEXT,\
provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,\
system_prompt TEXT,\
capabilities TEXT,\
skills TEXT,\
memory_config TEXT,\
knowledge_scope TEXT,\
status TEXT NOT NULL DEFAULT \'inactive\',\
max_concurrent_sessions INTEGER DEFAULT 3,\
token_budget_per_session INTEGER DEFAULT 0,\
created_at TEXT NOT NULL,\
updated_at TEXT NOT NULL,\
retired_at TEXT\
);

## **Table 3: sessions / 会话**

**Purpose: Manages AI session lifecycle including provider binding,
operating mode, token tracking, and state.**

**目的：管理AI会话生命周期，包括提供者绑定、运行模式、token跟踪和状态。**

**Column Specification / 列规范**

  -------------------------------------------------------------------------------------------------------------
  **Column**             **Type**          **Constraints**           **Description / 描述**
  ---------------------- ----------------- ------------------------- ------------------------------------------
  id                     TEXT              PK                        Unique session identifier

  name                   TEXT              NN                        Session display name

  provider_id            TEXT              FK-\>providers.id         AI provider used

  agent_id               TEXT              FK-\>agents.id, NULLABLE  Assigned agent

  team_instance_id       TEXT              FK-\>team_instances.id,   Parent team instance
                                           NULLABLE                  

  status                 TEXT              NN, DEFAULT created       created/active/paused/completed/failed

  mode                   TEXT              NN, DEFAULT               human_interaction/supervision/autonomous
                                           human_interaction         

  working_directory      TEXT              NULLABLE                  File system path

  context_window_used    INTEGER           DEFAULT 0                 Tokens used in context

  context_window_limit   INTEGER           DEFAULT 0                 Max context window size

  total_tokens_in        INTEGER           DEFAULT 0                 Total input tokens

  total_tokens_out       INTEGER           DEFAULT 0                 Total output tokens

  metadata               TEXT              NULLABLE                  JSON metadata

  created_at             TEXT              NN                        ISO 8601 timestamp

  updated_at             TEXT              NN                        ISO 8601 timestamp

  completed_at           TEXT              NULLABLE                  ISO 8601 timestamp
  -------------------------------------------------------------------------------------------------------------

**Indexes / 索引**

  -----------------------------------------------------------------------------
  **Index Name**         **Columns**        **Type**          **Description**
  ---------------------- ------------------ ----------------- -----------------
  idx_sessions_agent     agent_id           INDEX             Find sessions by
                                                              agent

  idx_sessions_team      team_instance_id   INDEX             Find sessions by
                                                              team instance

  idx_sessions_status    status             INDEX             Filter sessions
                                                              by status

  idx_sessions_created   created_at         INDEX             Order sessions by
                                                              creation time
  -----------------------------------------------------------------------------

**Foreign Keys / 外键**

  -----------------------------------------------------------------------
  **Column**              **References**          **On Delete**
  ----------------------- ----------------------- -----------------------
  provider_id             providers(id)           CASCADE

  agent_id                agents(id)              SET NULL

  team_instance_id        team_instances(id)      SET NULL
  -----------------------------------------------------------------------

**Sample Data / 示例数据**

  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**   **name**   **provider_id**   **agent_id**   **team_instance_id**   **status**   **mode**            **working_directory**   **context_window_used**   **context_window_limit**   **total_tokens_in**   **total_tokens_out**   **metadata**   **created_at**         **updated_at**         **completed_at**
  -------- ---------- ----------------- -------------- ---------------------- ------------ ------------------- ----------------------- ------------------------- -------------------------- --------------------- ---------------------- -------------- ---------------------- ---------------------- ------------------
  ss-001   Code       pv-claude-001     ag-arch-001    None                   active       human_interaction   /home/user/project      45000                     200000                     52000                 18000                  None           2026-04-07T10:00:00Z   2026-04-07T10:30:00Z   None
           Review                                                                                                                                                                                                                                                                                     
           Session                                                                                                                                                                                                                                                                                    

  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE sessions (\
id TEXT PRIMARY KEY,\
name TEXT NOT NULL,\
provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,\
agent_id TEXT REFERENCES agents(id) ON DELETE SET NULL,\
team_instance_id TEXT REFERENCES team_instances(id) ON DELETE SET NULL,\
status TEXT NOT NULL DEFAULT \'created\',\
mode TEXT NOT NULL DEFAULT \'human_interaction\',\
working_directory TEXT,\
context_window_used INTEGER DEFAULT 0,\
context_window_limit INTEGER DEFAULT 0,\
total_tokens_in INTEGER DEFAULT 0,\
total_tokens_out INTEGER DEFAULT 0,\
metadata TEXT,\
created_at TEXT NOT NULL,\
updated_at TEXT NOT NULL,\
completed_at TEXT\
);

## **Table 4: teams / 团队**

**Purpose: Defines team templates with purpose, governance
configuration, and member capacity.**

**目的：定义团队模板，包含目标、治理配置和成员容量。**

**Column Specification / 列规范**

  ----------------------------------------------------------------------------------------------
  **Column**          **Type**          **Constraints**           **Description / 描述**
  ------------------- ----------------- ------------------------- ------------------------------
  id                  TEXT              PK                        Unique team identifier

  name                TEXT              NN                        Team display name

  description         TEXT              NULLABLE                  Team purpose and objectives

  template_id         TEXT              FK-\>team_templates.id,   Source template
                                        NULLABLE                  

  governance_config   TEXT              NULLABLE                  JSON governance rules

  max_members         INTEGER           DEFAULT 10                Maximum team size

  status              TEXT              NN, DEFAULT draft         draft/active/paused/archived

  created_at          TEXT              NN                        ISO 8601 timestamp

  updated_at          TEXT              NN                        ISO 8601 timestamp
  ----------------------------------------------------------------------------------------------

**Indexes / 索引**

  ------------------------------------------------------------------------
  **Index Name**     **Columns**       **Type**          **Description**
  ------------------ ----------------- ----------------- -----------------
  idx_teams_status   status            INDEX             Filter teams by
                                                         status

  ------------------------------------------------------------------------

**Sample Data / 示例数据**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**       **name**      **description**   **template_id**   **governance_config**     **max_members**   **status**   **created_at**         **updated_at**
  ------------ ------------- ----------------- ----------------- ------------------------- ----------------- ------------ ---------------------- ----------------------
  tm-dev-001   Development   Full-stack        None              {\"approval_required\":   10                active       2026-04-01T00:00:00Z   2026-04-01T00:00:00Z
               Team          development team                    true}                                                                           

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE teams (\
id TEXT PRIMARY KEY,\
name TEXT NOT NULL,\
description TEXT,\
template_id TEXT,\
governance_config TEXT,\
max_members INTEGER DEFAULT 10,\
status TEXT NOT NULL DEFAULT \'draft\',\
created_at TEXT NOT NULL,\
updated_at TEXT NOT NULL\
);

## **Table 5: team_roles / 团队角色**

**Purpose: Defines roles within a team with specific permissions,
capabilities, and provider preferences.**

**目的：定义团队内的角色，包含权限、能力和提供者偏好。**

**Column Specification / 列规范**

  ---------------------------------------------------------------------------
  **Column**            **Type**          **Constraints**   **Description /
                                                            描述**
  --------------------- ----------------- ----------------- -----------------
  id                    TEXT              PK                Unique role
                                                            identifier

  team_id               TEXT              FK-\>teams.id, NN Parent team

  role_name             TEXT              NN                Role name (e.g.,
                                                            Leader,
                                                            Architect)

  permissions           TEXT              NULLABLE          JSON permissions
                                                            array

  capabilities          TEXT              NULLABLE          JSON capabilities
                                                            array

  provider_preference   TEXT              NULLABLE          Preferred
                                                            provider for this
                                                            role

  color                 TEXT              NULLABLE          UI color code
                                                            (hex)

  created_at            TEXT              NN                ISO 8601
                                                            timestamp
  ---------------------------------------------------------------------------

**Indexes / 索引**

  ---------------------------------------------------------------------------
  **Index Name**        **Columns**       **Type**          **Description**
  --------------------- ----------------- ----------------- -----------------
  idx_team_roles_team   team_id           INDEX             Find roles by
                                                            team

  ---------------------------------------------------------------------------

**Foreign Keys / 外键**

  -----------------------------------------------------------------------
  **Column**              **References**          **On Delete**
  ----------------------- ----------------------- -----------------------
  team_id                 teams(id)               CASCADE

  -----------------------------------------------------------------------

**Sample Data / 示例数据**

  ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**   **team_id**   **role_name**   **permissions**                          **capabilities**                  **provider_preference**   **color**   **created_at**
  -------- ------------- --------------- ---------------------------------------- --------------------------------- ------------------------- ----------- ----------------------
  tr-001   tm-dev-001    Team Leader     \[\"approve_tasks\",\"assign_roles\"\]   \[\"coordination\",\"review\"\]   claude                    #f59e0b     2026-04-01T00:00:00Z

  ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE team_roles (\
id TEXT PRIMARY KEY,\
team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,\
role_name TEXT NOT NULL,\
permissions TEXT,\
capabilities TEXT,\
provider_preference TEXT,\
color TEXT,\
created_at TEXT NOT NULL\
);

## **Table 6: team_instances / 团队实例**

**Purpose: Represents a running instance of a team with its own state,
task list, and member assignments.**

**目的：表示团队的运行实例，拥有独立的状态、任务列表和成员分配。**

**Column Specification / 列规范**

  ----------------------------------------------------------------------------------------------------
  **Column**        **Type**          **Constraints**   **Description / 描述**
  ----------------- ----------------- ----------------- ----------------------------------------------
  id                TEXT              PK                Unique instance identifier

  team_id           TEXT              FK-\>teams.id, NN Parent team definition

  name              TEXT              NN                Instance display name

  status            TEXT              NN, DEFAULT       initializing/running/paused/completed/failed
                                      initializing      

  objective         TEXT              NULLABLE          Current task objective

  started_at        TEXT              NULLABLE          ISO 8601 timestamp

  completed_at      TEXT              NULLABLE          ISO 8601 timestamp

  created_at        TEXT              NN                ISO 8601 timestamp

  updated_at        TEXT              NN                ISO 8601 timestamp
  ----------------------------------------------------------------------------------------------------

**Indexes / 索引**

  ----------------------------------------------------------------------------
  **Index Name**         **Columns**       **Type**          **Description**
  ---------------------- ----------------- ----------------- -----------------
  idx_team_inst_team     team_id           INDEX             Find instances by
                                                             team

  idx_team_inst_status   status            INDEX             Filter instances
                                                             by status
  ----------------------------------------------------------------------------

**Foreign Keys / 外键**

  -----------------------------------------------------------------------
  **Column**              **References**          **On Delete**
  ----------------------- ----------------------- -----------------------
  team_id                 teams(id)               CASCADE

  -----------------------------------------------------------------------

**Sample Data / 示例数据**

  -------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**   **team_id**   **name**   **status**   **objective**    **started_at**         **completed_at**   **created_at**         **updated_at**
  -------- ------------- ---------- ------------ ---------------- ---------------------- ------------------ ---------------------- ----------------------
  ti-001   tm-dev-001    Dev Team   running      Build            2026-04-07T09:00:00Z   None               2026-04-07T09:00:00Z   2026-04-07T10:30:00Z
                         Sprint 1                authentication                                                                    
                                                 module                                                                            

  -------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE team_instances (\
id TEXT PRIMARY KEY,\
team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,\
name TEXT NOT NULL,\
status TEXT NOT NULL DEFAULT \'initializing\',\
objective TEXT,\
started_at TEXT,\
completed_at TEXT,\
created_at TEXT NOT NULL,\
updated_at TEXT NOT NULL\
);

## **Table 7: team_members / 团队成员**

**Purpose: Manages agent membership within team instances, linking
agents to roles.**

**目的：管理团队实例中的智能体成员关系，将智能体链接到角色。**

**Column Specification / 列规范**

  ---------------------------------------------------------------------------------------
  **Column**         **Type**          **Constraints**           **Description / 描述**
  ------------------ ----------------- ------------------------- ------------------------
  id                 TEXT              PK                        Unique membership
                                                                 identifier

  team_instance_id   TEXT              FK-\>team_instances.id,   Parent team instance
                                       NN                        

  agent_id           TEXT              FK-\>agents.id, NN        Agent reference

  role_id            TEXT              FK-\>team_roles.id, NN    Assigned role

  status             TEXT              NN, DEFAULT active        active/idle/error/left

  joined_at          TEXT              NN                        ISO 8601 timestamp

  left_at            TEXT              NULLABLE                  ISO 8601 timestamp
  ---------------------------------------------------------------------------------------

**Indexes / 索引**

  -----------------------------------------------------------------------------------
  **Index Name**              **Columns**         **Type**          **Description**
  --------------------------- ------------------- ----------------- -----------------
  idx_members_instance        team_instance_id    INDEX             Find members by
                                                                    instance

  idx_members_agent           agent_id            INDEX             Find memberships
                                                                    by agent

  uq_members_instance_agent   team_instance_id,   UNIQUE            Prevent duplicate
                              agent_id                              membership
  -----------------------------------------------------------------------------------

**Foreign Keys / 外键**

  -----------------------------------------------------------------------
  **Column**              **References**          **On Delete**
  ----------------------- ----------------------- -----------------------
  team_instance_id        team_instances(id)      CASCADE

  agent_id                agents(id)              CASCADE

  role_id                 team_roles(id)          CASCADE
  -----------------------------------------------------------------------

**Sample Data / 示例数据**

  ----------------------------------------------------------------------------------------------------------------
  **id**     **team_instance_id**   **agent_id**   **role_id**   **status**   **joined_at**          **left_at**
  ---------- ---------------------- -------------- ------------- ------------ ---------------------- -------------
  mb-001     ti-001                 ag-arch-001    tr-001        active       2026-04-07T09:00:00Z   None

  ----------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE team_members (\
id TEXT PRIMARY KEY,\
team_instance_id TEXT NOT NULL REFERENCES team_instances(id) ON DELETE
CASCADE,\
agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,\
role_id TEXT NOT NULL REFERENCES team_roles(id) ON DELETE CASCADE,\
status TEXT NOT NULL DEFAULT \'active\',\
joined_at TEXT NOT NULL,\
left_at TEXT,\
UNIQUE(team_instance_id, agent_id)\
);

## **Table 8: team_tasks / 团队任务 (SharedTaskList)**

**Purpose: Implements the SharedTaskList with SQLite atomic claim
operations for distributed task management.**

**目的：实现SharedTaskList，使用SQLite原子声明操作进行分布式任务管理。**

**Column Specification / 列规范**

  ------------------------------------------------------------------------------------------------------------------------
  **Column**         **Type**          **Constraints**           **Description / 描述**
  ------------------ ----------------- ------------------------- ---------------------------------------------------------
  id                 TEXT              PK                        Unique task identifier

  team_instance_id   TEXT              FK-\>team_instances.id,   Parent team instance
                                       NN                        

  title              TEXT              NN                        Task title

  description        TEXT              NULLABLE                  Task description

  status             TEXT              NN, DEFAULT pending       pending/claimed/in_progress/completed/blocked/cancelled

  priority           TEXT              NN, DEFAULT medium        critical/high/medium/low

  claimed_by         TEXT              FK-\>team_members.id,     Member who claimed
                                       NULLABLE                  

  claimed_at         TEXT              NULLABLE                  ISO 8601 timestamp

  completed_by       TEXT              FK-\>team_members.id,     Member who completed
                                       NULLABLE                  

  parent_task_id     TEXT              FK-\>team_tasks.id,       Parent task for subtasks
                                       NULLABLE                  

  dependencies       TEXT              NULLABLE                  JSON array of task IDs

  result             TEXT              NULLABLE                  JSON task result/output

  due_at             TEXT              NULLABLE                  ISO 8601 due date

  created_at         TEXT              NN                        ISO 8601 timestamp

  updated_at         TEXT              NN                        ISO 8601 timestamp

  completed_at       TEXT              NULLABLE                  ISO 8601 timestamp
  ------------------------------------------------------------------------------------------------------------------------

**Indexes / 索引**

  -----------------------------------------------------------------------------------
  **Index Name**              **Columns**         **Type**          **Description**
  --------------------------- ------------------- ----------------- -----------------
  idx_tasks_instance_status   team_instance_id,   INDEX             Atomic claim
                              status                                query: WHERE
                                                                    status=pending

  idx_tasks_claimed           claimed_by          INDEX             Find tasks by
                                                                    claiming member

  idx_tasks_parent            parent_task_id      INDEX             Find subtasks

  idx_tasks_priority          priority            INDEX             Filter by
                                                                    priority
  -----------------------------------------------------------------------------------

**Foreign Keys / 外键**

  -----------------------------------------------------------------------
  **Column**              **References**          **On Delete**
  ----------------------- ----------------------- -----------------------
  team_instance_id        team_instances(id)      CASCADE

  claimed_by              team_members(id)        SET NULL

  completed_by            team_members(id)        SET NULL

  parent_task_id          team_tasks(id)          SET NULL
  -----------------------------------------------------------------------

**Sample Data / 示例数据**

  --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**   **team_instance_id**   **title**   **description**   **status**    **priority**   **claimed_by**   **claimed_at**         **completed_by**   **parent_task_id**   **dependencies**   **result**   **due_at**             **created_at**         **updated_at**         **completed_at**
  -------- ---------------------- ----------- ----------------- ------------- -------------- ---------------- ---------------------- ------------------ -------------------- ------------------ ------------ ---------------------- ---------------------- ---------------------- ------------------
  tk-001   ti-001                 Design auth Create database   in_progress   high           mb-001           2026-04-07T09:15:00Z   None               None                 None               None         2026-04-07T12:00:00Z   2026-04-07T09:00:00Z   2026-04-07T10:00:00Z   None
                                  schema      schema for auth                                                                                                                                                                                                                     
                                              module                                                                                                                                                                                                                              

  --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE team_tasks (\
id TEXT PRIMARY KEY,\
team_instance_id TEXT NOT NULL REFERENCES team_instances(id) ON DELETE
CASCADE,\
title TEXT NOT NULL,\
description TEXT,\
status TEXT NOT NULL DEFAULT \'pending\',\
priority TEXT NOT NULL DEFAULT \'medium\',\
claimed_by TEXT REFERENCES team_members(id) ON DELETE SET NULL,\
claimed_at TEXT,\
completed_by TEXT REFERENCES team_members(id) ON DELETE SET NULL,\
parent_task_id TEXT REFERENCES team_tasks(id) ON DELETE SET NULL,\
dependencies TEXT,\
result TEXT,\
due_at TEXT,\
created_at TEXT NOT NULL,\
updated_at TEXT NOT NULL,\
completed_at TEXT\
);\
CREATE INDEX idx_tasks_instance_status ON team_tasks(team_instance_id,
status);\
CREATE INDEX idx_tasks_claimed ON team_tasks(claimed_by);\
CREATE INDEX idx_tasks_parent ON team_tasks(parent_task_id);\
CREATE INDEX idx_tasks_priority ON team_tasks(priority);

## **Table 9: team_messages / 团队消息 (TeamBus)**

**Purpose: Persists all inter-agent communication messages with routing
metadata for the TeamBus system.**

**目的：持久化所有智能体间通信消息，包含TeamBus系统的路由元数据。**

**Column Specification / 列规范**

  ------------------------------------------------------------------------------------------------------
  **Column**            **Type**          **Constraints**           **Description / 描述**
  --------------------- ----------------- ------------------------- ------------------------------------
  id                    TEXT              PK                        Unique message identifier

  team_instance_id      TEXT              FK-\>team_instances.id,   Parent team instance
                                          NN                        

  sender_member_id      TEXT              FK-\>team_members.id, NN  Sender member

  recipient_member_id   TEXT              FK-\>team_members.id,     Recipient (NULL=broadcast)
                                          NULLABLE                  

  recipient_role        TEXT              NULLABLE                  Target role (NULL=direct/broadcast)

  message_type          TEXT              NN                        direct/broadcast/role_group/system

  content               TEXT              NN                        Message content

  metadata              TEXT              NULLABLE                  JSON metadata (tool calls,
                                                                    references)

  delivery_status       TEXT              NN, DEFAULT delivered     delivered/read/failed

  created_at            TEXT              NN                        ISO 8601 timestamp
  ------------------------------------------------------------------------------------------------------

**Indexes / 索引**

  --------------------------------------------------------------------------------------
  **Index Name**               **Columns**           **Type**          **Description**
  ---------------------------- --------------------- ----------------- -----------------
  idx_messages_instance_time   team_instance_id,     INDEX             Query messages by
                               created_at                              team and time

  idx_messages_sender          sender_member_id      INDEX             Find messages by
                                                                       sender

  idx_messages_recipient       recipient_member_id   INDEX             Find messages by
                                                                       recipient
  --------------------------------------------------------------------------------------

**Foreign Keys / 外键**

  -----------------------------------------------------------------------
  **Column**              **References**          **On Delete**
  ----------------------- ----------------------- -----------------------
  team_instance_id        team_instances(id)      CASCADE

  sender_member_id        team_members(id)        CASCADE

  recipient_member_id     team_members(id)        SET NULL
  -----------------------------------------------------------------------

**Sample Data / 示例数据**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**    **team_instance_id**   **sender_member_id**   **recipient_member_id**   **recipient_role**   **message_type**   **content**   **metadata**   **delivery_status**   **created_at**
  --------- ---------------------- ---------------------- ------------------------- -------------------- ------------------ ------------- -------------- --------------------- ----------------------
  msg-001   ti-001                 mb-001                 None                      None                 broadcast          Auth schema   None           delivered             2026-04-07T10:00:00Z
                                                                                                                            design is                                          
                                                                                                                            complete.                                          
                                                                                                                            Ready for                                          
                                                                                                                            review.                                            

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE team_messages (\
id TEXT PRIMARY KEY,\
team_instance_id TEXT NOT NULL REFERENCES team_instances(id) ON DELETE
CASCADE,\
sender_member_id TEXT NOT NULL REFERENCES team_members(id) ON DELETE
CASCADE,\
recipient_member_id TEXT REFERENCES team_members(id) ON DELETE SET
NULL,\
recipient_role TEXT,\
message_type TEXT NOT NULL,\
content TEXT NOT NULL,\
metadata TEXT,\
delivery_status TEXT NOT NULL DEFAULT \'delivered\',\
created_at TEXT NOT NULL\
);\
CREATE INDEX idx_messages_instance_time ON
team_messages(team_instance_id, created_at);\
CREATE INDEX idx_messages_sender ON team_messages(sender_member_id);\
CREATE INDEX idx_messages_recipient ON
team_messages(recipient_member_id);

## **Table 10: conversations / 对话消息**

**Purpose: Stores structured conversation turns within sessions,
including tool calls, results, and token counts.**

**目的：存储会话中的结构化对话轮次，包括工具调用、结果和token计数。**

**Column Specification / 列规范**

  ------------------------------------------------------------------------------------
  **Column**        **Type**          **Constraints**     **Description / 描述**
  ----------------- ----------------- ------------------- ----------------------------
  id                TEXT              PK                  Unique message identifier

  session_id        TEXT              FK-\>sessions.id,   Parent session
                                      NN                  

  turn_number       INTEGER           NN                  Sequential turn number

  role              TEXT              NN                  user/assistant/system/tool

  content           TEXT              NULLABLE            Message content

  tool_calls        TEXT              NULLABLE            JSON array of tool
                                                          invocations

  tool_results      TEXT              NULLABLE            JSON array of tool results

  tokens_in         INTEGER           DEFAULT 0           Input tokens for this turn

  tokens_out        INTEGER           DEFAULT 0           Output tokens for this turn

  duration_ms       INTEGER           NULLABLE            Response duration in
                                                          milliseconds

  model             TEXT              NULLABLE            Model identifier used

  created_at        TEXT              NN                  ISO 8601 timestamp
  ------------------------------------------------------------------------------------

**Indexes / 索引**

  -----------------------------------------------------------------------------
  **Index Name**          **Columns**       **Type**          **Description**
  ----------------------- ----------------- ----------------- -----------------
  idx_conv_session_turn   session_id,       UNIQUE            Ordered
                          turn_number                         conversation
                                                              retrieval

  idx_conv_session        session_id        INDEX             Find all turns in
                                                              a session
  -----------------------------------------------------------------------------

**Foreign Keys / 外键**

  -----------------------------------------------------------------------
  **Column**              **References**          **On Delete**
  ----------------------- ----------------------- -----------------------
  session_id              sessions(id)            CASCADE

  -----------------------------------------------------------------------

**Sample Data / 示例数据**

  -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**   **session_id**   **turn_number**   **role**   **content**      **tool_calls**   **tool_results**   **tokens_in**   **tokens_out**   **duration_ms**   **model**   **created_at**
  -------- ---------------- ----------------- ---------- ---------------- ---------------- ------------------ --------------- ---------------- ----------------- ----------- ----------------------
  cv-001   ss-001           1                 user       Please review    None             None               25              0                None              None        2026-04-07T10:00:00Z
                                                         the                                                                                                                 
                                                         authentication                                                                                                      
                                                         module design.                                                                                                      

  -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE conversations (\
id TEXT PRIMARY KEY,\
session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,\
turn_number INTEGER NOT NULL,\
role TEXT NOT NULL,\
content TEXT,\
tool_calls TEXT,\
tool_results TEXT,\
tokens_in INTEGER DEFAULT 0,\
tokens_out INTEGER DEFAULT 0,\
duration_ms INTEGER,\
model TEXT,\
created_at TEXT NOT NULL,\
UNIQUE(session_id, turn_number)\
);\
CREATE INDEX idx_conv_session ON conversations(session_id);

## **Table 11: usage_stats / 使用统计**

**Purpose: Tracks daily token usage, costs, API calls, and errors per
entity (session, agent, team, provider).**

**目的：跟踪每个实体（会话、智能体、团队、提供者）的每日token使用量、成本和错误。**

**Column Specification / 列规范**

  ---------------------------------------------------------------------------------------------
  **Column**         **Type**          **Constraints**   **Description / 描述**
  ------------------ ----------------- ----------------- --------------------------------------
  id                 TEXT              PK                Unique stat record identifier

  entity_type        TEXT              NN                session/agent/team_instance/provider

  entity_id          TEXT              NN                Reference ID

  date               TEXT              NN                YYYY-MM-DD

  tokens_in          INTEGER           DEFAULT 0         Daily input tokens

  tokens_out         INTEGER           DEFAULT 0         Daily output tokens

  total_cost         REAL              DEFAULT 0.0       Estimated cost in USD

  api_calls          INTEGER           DEFAULT 0         Number of API calls

  errors             INTEGER           DEFAULT 0         Number of errors

  duration_seconds   INTEGER           DEFAULT 0         Total active seconds

  created_at         TEXT              NN                ISO 8601 timestamp
  ---------------------------------------------------------------------------------------------

**Indexes / 索引**

  -----------------------------------------------------------------------------
  **Index Name**          **Columns**       **Type**          **Description**
  ----------------------- ----------------- ----------------- -----------------
  idx_usage_entity_date   entity_type,      UNIQUE            Upsert by entity
                          entity_id, date                     and date

  idx_usage_date          date              INDEX             Aggregate by date
  -----------------------------------------------------------------------------

**Sample Data / 示例数据**

  ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**   **entity_type**   **entity_id**   **date**     **tokens_in**   **tokens_out**   **total_cost**   **api_calls**   **errors**   **duration_seconds**   **created_at**
  -------- ----------------- --------------- ------------ --------------- ---------------- ---------------- --------------- ------------ ---------------------- ----------------------
  us-001   session           ss-001          2026-04-07   52000           18000            3.45             12              0            1800                   2026-04-07T23:59:59Z

  ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE usage_stats (\
id TEXT PRIMARY KEY,\
entity_type TEXT NOT NULL,\
entity_id TEXT NOT NULL,\
date TEXT NOT NULL,\
tokens_in INTEGER DEFAULT 0,\
tokens_out INTEGER DEFAULT 0,\
total_cost REAL DEFAULT 0.0,\
api_calls INTEGER DEFAULT 0,\
errors INTEGER DEFAULT 0,\
duration_seconds INTEGER DEFAULT 0,\
created_at TEXT NOT NULL,\
UNIQUE(entity_type, entity_id, date)\
);\
CREATE INDEX idx_usage_date ON usage_stats(date);

## **Table 12: knowledge_entries / 知识条目**

**Purpose: Stores structured knowledge entries with FTS5 full-text
search, Obsidian vault integration, and agent/team scoping.**

**目的：存储结构化知识条目，支持FTS5全文搜索、Obsidian仓库集成和智能体/团队范围划分。**

**Column Specification / 列规范**

  ----------------------------------------------------------------------------------------------------------
  **Column**           **Type**          **Constraints**   **Description / 描述**
  -------------------- ----------------- ----------------- -------------------------------------------------
  id                   TEXT              PK                Unique knowledge entry identifier

  title                TEXT              NN                Knowledge entry title

  content              TEXT              NN                Markdown content

  category             TEXT              NN                project/agent_memory/research/technical/generic

  tags                 TEXT              NULLABLE          JSON array of tags

  source_type          TEXT              NULLABLE          agent_interaction/manual/import/obsidian

  source_ref           TEXT              NULLABLE          Source reference

  agent_id             TEXT              FK-\>agents.id,   Associated agent
                                         NULLABLE          

  team_id              TEXT              FK-\>teams.id,    Associated team
                                         NULLABLE          

  obsidian_path        TEXT              NULLABLE          Path in Obsidian vault

  embedding_checksum   TEXT              NULLABLE          Checksum for change detection

  search_vector        TEXT              NULLABLE          FTS5 full-text search vector

  created_at           TEXT              NN                ISO 8601 timestamp

  updated_at           TEXT              NN                ISO 8601 timestamp
  ----------------------------------------------------------------------------------------------------------

**Indexes / 索引**

  ------------------------------------------------------------------------------
  **Index Name**           **Columns**       **Type**          **Description**
  ------------------------ ----------------- ----------------- -----------------
  idx_knowledge_category   category          INDEX             Filter by
                                                               category

  idx_knowledge_agent      agent_id          INDEX             Find knowledge by
                                                               agent

  idx_knowledge_team       team_id           INDEX             Find knowledge by
                                                               team

  idx_knowledge_obsidian   obsidian_path     INDEX             Obsidian sync
                                                               lookup
  ------------------------------------------------------------------------------

**Foreign Keys / 外键**

  -----------------------------------------------------------------------
  **Column**              **References**          **On Delete**
  ----------------------- ----------------------- -----------------------
  agent_id                agents(id)              SET NULL

  team_id                 teams(id)               CASCADE
  -----------------------------------------------------------------------

**Sample Data / 示例数据**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**   **title**   **content**   **category**   **tags**                               **source_type**     **source_ref**   **agent_id**   **team_id**   **obsidian_path**           **embedding_checksum**   **search_vector**   **created_at**         **updated_at**
  -------- ----------- ------------- -------------- -------------------------------------- ------------------- ---------------- -------------- ------------- --------------------------- ------------------------ ------------------- ---------------------- ----------------------
  kn-001   Auth Module \# Auth       technical      \[\"auth\",\"database\",\"design\"\]   agent_interaction   ss-001           ag-arch-001    None          /knowledge/auth-design.md   None                     auth module design  2026-04-07T10:30:00Z   2026-04-07T10:30:00Z
           Design      Module                                                                                                                                                                                     database schema                            
                       Design\                                                                                                                                                                                    overview                                   
                       \                                                                                                                                                                                                                                     
                       \## Overview\                                                                                                                                                                                                                         
                       Database                                                                                                                                                                                                                              
                       schema                                                                                                                                                                                                                                
                       for\...                                                                                                                                                                                                                               

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE knowledge_entries (\
id TEXT PRIMARY KEY,\
title TEXT NOT NULL,\
content TEXT NOT NULL,\
category TEXT NOT NULL,\
tags TEXT,\
source_type TEXT,\
source_ref TEXT,\
agent_id TEXT REFERENCES agents(id) ON DELETE SET NULL,\
team_id TEXT REFERENCES teams(id) ON DELETE CASCADE,\
obsidian_path TEXT,\
embedding_checksum TEXT,\
search_vector TEXT,\
created_at TEXT NOT NULL,\
updated_at TEXT NOT NULL\
);\
CREATE VIRTUAL TABLE knowledge_entries_fts USING fts5(\
title, content, tags,\
content=knowledge_entries,\
content_rowid=rowid\
);\
CREATE INDEX idx_knowledge_category ON knowledge_entries(category);\
CREATE INDEX idx_knowledge_agent ON knowledge_entries(agent_id);\
CREATE INDEX idx_knowledge_team ON knowledge_entries(team_id);\
CREATE INDEX idx_knowledge_obsidian ON knowledge_entries(obsidian_path);

## **Table 13: mcp_tools / MCP工具**

**Purpose: Registry of MCP (Model Context Protocol) tools available to
agents, including built-in, team-specific, and custom tools.**

**目的：注册MCP工具，包括内置工具、团队工具和自定义工具。**

**Column Specification / 列规范**

  ---------------------------------------------------------------------------------------------------------------
  **Column**            **Type**          **Constraints**   **Description / 描述**
  --------------------- ----------------- ----------------- -----------------------------------------------------
  id                    TEXT              PK                Unique tool identifier

  name                  TEXT              NN, UNIQUE        Tool name

  description           TEXT              NULLABLE          Tool description

  input_schema          TEXT              NULLABLE          JSON Schema for input parameters

  output_schema         TEXT              NULLABLE          JSON Schema for output

  tool_type             TEXT              NN                built_in/team/custom

  category              TEXT              NULLABLE          file_system/web_search/code_execution/communication

  permissions           TEXT              NULLABLE          JSON required permissions

  token_cost_estimate   INTEGER           DEFAULT 0         Estimated token cost per invocation

  is_enabled            INTEGER           DEFAULT 1         0=disabled, 1=enabled

  config                TEXT              NULLABLE          JSON tool configuration

  created_at            TEXT              NN                ISO 8601 timestamp

  updated_at            TEXT              NN                ISO 8601 timestamp
  ---------------------------------------------------------------------------------------------------------------

**Indexes / 索引**

  --------------------------------------------------------------------------
  **Index Name**       **Columns**       **Type**          **Description**
  -------------------- ----------------- ----------------- -----------------
  idx_tools_type       tool_type         INDEX             Filter tools by
                                                           type

  idx_tools_category   category          INDEX             Filter tools by
                                                           category

  idx_tools_enabled    is_enabled        INDEX             Filter enabled
                                                           tools
  --------------------------------------------------------------------------

**Sample Data / 示例数据**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**    **name**            **description**   **input_schema**                                                                                          **output_schema**   **tool_type**   **category**    **permissions**     **token_cost_estimate**   **is_enabled**   **config**   **created_at**         **updated_at**
  --------- ------------------- ----------------- --------------------------------------------------------------------------------------------------------- ------------------- --------------- --------------- ------------------- ------------------------- ---------------- ------------ ---------------------- ----------------------
  mcp-001   team_message_role   Send a message to {\"type\":\"object\",\"properties\":{\"role\":{\"type\":\"string\"},\"content\":{\"type\":\"string\"}}}   None                built_in        communication   \[\"team:read\"\]   50                        1                None         2026-04-01T00:00:00Z   2026-04-01T00:00:00Z
                                a specific team                                                                                                                                                                                                                                                                    
                                role                                                                                                                                                                                                                                                                               

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE mcp_tools (\
id TEXT PRIMARY KEY,\
name TEXT NOT NULL UNIQUE,\
description TEXT,\
input_schema TEXT,\
output_schema TEXT,\
tool_type TEXT NOT NULL,\
category TEXT,\
permissions TEXT,\
token_cost_estimate INTEGER DEFAULT 0,\
is_enabled INTEGER DEFAULT 1,\
config TEXT,\
created_at TEXT NOT NULL,\
updated_at TEXT NOT NULL\
);\
CREATE INDEX idx_tools_type ON mcp_tools(tool_type);\
CREATE INDEX idx_tools_category ON mcp_tools(category);\
CREATE INDEX idx_tools_enabled ON mcp_tools(is_enabled);

## **Table 14: audit_log / 审计日志**

**Purpose: Immutable audit trail recording all security-relevant events
across the system.**

**目的：不可变的审计跟踪，记录系统中所有安全相关事件。**

**Column Specification / 列规范**

  -----------------------------------------------------------------------------------------------------------------
  **Column**        **Type**          **Constraints**   **Description / 描述**
  ----------------- ----------------- ----------------- -----------------------------------------------------------
  id                TEXT              PK                Unique audit record identifier

  entity_type       TEXT              NN                agent/team/session/provider/user/config

  entity_id         TEXT              NN                Affected entity ID

  action            TEXT              NN                created/updated/deleted/started/stopped/claimed/completed

  actor_type        TEXT              NN                user/agent/system

  actor_id          TEXT              NN                Actor reference

  details           TEXT              NULLABLE          JSON change details

  ip_address        TEXT              NULLABLE          Client IP (if applicable)

  created_at        TEXT              NN                ISO 8601 timestamp (immutable)
  -----------------------------------------------------------------------------------------------------------------

**Indexes / 索引**

  ------------------------------------------------------------------------
  **Index Name**     **Columns**       **Type**          **Description**
  ------------------ ----------------- ----------------- -----------------
  idx_audit_entity   entity_type,      INDEX             Query audit by
                     entity_id,                          entity
                     created_at                          

  idx_audit_actor    actor_type,       INDEX             Query audit by
                     actor_id                            actor

  idx_audit_time     created_at        INDEX             Time-range
                                                         queries
  ------------------------------------------------------------------------

**Sample Data / 示例数据**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **id**   **entity_type**   **entity_id**   **action**   **actor_type**   **actor_id**   **details**                           **ip_address**   **created_at**
  -------- ----------------- --------------- ------------ ---------------- -------------- ------------------------------------- ---------------- ----------------------
  al-001   agent             ag-arch-001     created      user             user-admin     {\"name\":\"Chief                     None             2026-04-01T00:00:00Z
                                                                                          Architect\",\"role\":\"Architect\"}                    

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------

**CREATE TABLE SQL / 建表语句**

CREATE TABLE audit_log (\
id TEXT PRIMARY KEY,\
entity_type TEXT NOT NULL,\
entity_id TEXT NOT NULL,\
action TEXT NOT NULL,\
actor_type TEXT NOT NULL,\
actor_id TEXT NOT NULL,\
details TEXT,\
ip_address TEXT,\
created_at TEXT NOT NULL\
);\
CREATE INDEX idx_audit_entity ON audit_log(entity_type, entity_id,
created_at);\
CREATE INDEX idx_audit_actor ON audit_log(actor_type, actor_id);\
CREATE INDEX idx_audit_time ON audit_log(created_at);

# **6. Index Strategy / 索引策略**

## **6.1 Index Types / 索引类型**

  -------------------------------------------------------------------------------
  **Type**                **Description**         **Usage in AgentForge**
  ----------------------- ----------------------- -------------------------------
  Primary Key             Automatic unique index  All 14 tables use TEXT PK (UUID
                          on PK column            v4)

  Unique Constraint       Enforces uniqueness     agents.name,
                          across one or more      team_members(instance+agent),
                          columns                 usage_stats(entity+date)

  Secondary Index         B-tree index on         Status filters, foreign key
                          frequently queried      lookups, time-range queries
                          columns                 

  Composite Index         Multi-column index for  team_tasks(instance+status) for
                          complex WHERE clauses   atomic claim,
                                                  messages(instance+time)

  FTS5 Virtual Table      Full-text search index  knowledge_entries_fts for
                          on content columns      knowledge base search
  -------------------------------------------------------------------------------

## **6.2 Index Naming Convention / 索引命名约定**

All indexes follow the pattern: idx\_{table}\_{column(s)}

idx_sessions_status \-- Single column index\
idx_tasks_instance_status \-- Composite index\
idx_conv_session_turn \-- Unique composite index\
knowledge_entries_fts \-- FTS5 virtual table

## **6.3 Complete Index Inventory / 完整索引清单**

Total indexes across all tables: 35 (including 1 FTS5 virtual table)

-   providers: 1 index

-   agents: 2 indexes

-   sessions: 4 indexes

-   teams: 1 index

-   team_roles: 1 index

-   team_instances: 2 indexes

-   team_members: 3 indexes (including 1 UNIQUE)

-   team_tasks: 4 indexes

-   team_messages: 3 indexes

-   conversations: 2 indexes (including 1 UNIQUE)

-   usage_stats: 2 indexes (including 1 UNIQUE)

-   knowledge_entries: 4 indexes + 1 FTS5

-   mcp_tools: 3 indexes

-   audit_log: 3 indexes

# **7. Data Access Layer / 数据访问层**

## **7.1 Repository Trait Pattern / Repository特征模式**

Each entity type has a corresponding Rust trait defining the data access
contract:

pub trait Repository\<T, Filter, Update\> {\
fn create(&self, entity: &T) -\> Result\<T\>;\
fn get_by_id(&self, id: &str) -\> Result\<Option\<T\>\>;\
fn list(&self, filter: &Filter) -\> Result\<Vec\<T\>\>;\
fn update(&self, id: &str, update: &Update) -\> Result\<()\>;\
fn delete(&self, id: &str) -\> Result\<()\>;\
fn count(&self, filter: &Filter) -\> Result\<i64\>;\
}

## **7.2 Specialized Repository Methods / 专用Repository方法**

Certain repositories have specialized methods for domain-specific
operations:

-   TaskRepository::claim_next(team_instance_id) \-- Atomic task claim:
    UPDATE team_tasks SET status=claimed, claimed_by=? WHERE id IN
    (SELECT id FROM team_tasks WHERE team_instance_id=? AND
    status=pending ORDER BY priority LIMIT 1)

-   TaskRepository::complete(task_id, member_id, result) \-- Mark task
    as completed with result

-   KnowledgeRepository::search(query) \-- Full-text search via FTS5:
    SELECT \* FROM knowledge_entries_fts WHERE knowledge_entries_fts
    MATCH ?

-   UsageRepository::upsert_daily(entity_type, entity_id, date,
    tokens_in, tokens_out) \-- INSERT OR REPLACE for daily stats

-   AuditLogRepository::append(entity_type, entity_id, action,
    actor_type, actor_id, details) \-- Immutable append-only

## **7.3 Error Handling / 错误处理**

Database errors are mapped to domain-specific error types:

pub enum DbError {\
NotFound(String),\
Conflict(String), // Unique constraint violation\
Constraint(String), // Foreign key violation\
Busy(String), // SQLITE_BUSY after retries\
Corrupted(String), // Database corruption detected\
Migration(String), // Migration failure\
}

All rusqlite errors are caught at the repository boundary and converted
to DbError. The application layer handles DbError with user-friendly
messages and appropriate recovery actions.

## **7.4 Connection Lifecycle / 连接生命周期**

pub struct Database {\
write_conn: Mutex\<Connection\>, // Single writer connection\
read_pool: Vec\<Connection\>, // Pool of read connections\
path: PathBuf, // Database file path\
}\
\
impl Database {\
pub fn open(path: &Path) -\> Result\<Self\> {\
let conn = Connection::open(path)?;\
conn.execute_batch(\"PRAGMA journal_mode=WAL;\
PRAGMA synchronous=NORMAL;\
PRAGMA foreign_keys=ON;\
PRAGMA busy_timeout=5000;\")?;\
Ok(Self { write_conn: Mutex::new(conn), read_pool: vec\![\], path:
path.into() })\
}\
}

# **8. Migration Strategy / 迁移策略**

## **8.1 Versioned Migration Files / 版本化迁移文件**

Database schema changes are managed through versioned SQL migration
files stored in the migrations/ directory:

migrations/\
001_initial_schema.up.sql\
001_initial_schema.down.sql\
002_add_knowledge_fts.up.sql\
002_add_knowledge_fts.down.sql\
003_add_usage_stats.up.sql\
003_add_usage_stats.down.sql

## **8.2 Naming Convention / 命名约定**

-   Format: {NNN}\_{description}.{up\|down}.sql

-   NNN: Zero-padded sequential number (001, 002, \...)

-   description: snake_case description of the change

-   up.sql: Forward migration (apply schema change)

-   down.sql: Reverse migration (rollback schema change)

## **8.3 Schema Version Tracking / 模式版本跟踪**

A schema_versions table tracks applied migrations:

CREATE TABLE IF NOT EXISTS schema_versions (\
version INTEGER PRIMARY KEY,\
description TEXT NOT NULL,\
applied_at TEXT NOT NULL DEFAULT (datetime(\'now\')),\
checksum TEXT\
);

## **8.4 Migration Execution / 迁移执行**

Migrations are executed at application startup:

-   Check current schema version from schema_versions table

-   Apply all pending migrations in order (001, 002, \...)

-   Each migration runs within a transaction

-   On failure, rollback the transaction and report the error

-   Record successful migration in schema_versions

# **9. Backup & Recovery / 备份与恢复**

## **9.1 Automated Backup Schedule / 自动备份计划**

AgentForge implements automated database backups:

-   Frequency: Configurable, minimum daily (default: every 6 hours)

-   Method: SQLite backup API (conn.backup(target_path))

-   Retention: Last 7 daily backups, 4 weekly backups, 12 monthly
    backups

-   Verification: SHA-256 checksum comparison after backup completion

-   Storage: {APP_DATA}/agentforge/backups/ directory

## **9.2 Backup API Usage / 备份API使用**

fn create_backup(db_path: &Path, backup_dir: &Path) -\>
Result\<PathBuf\> {\
let timestamp = chrono::Utc::now().format(\"%Y%m%d\_%H%M%S\");\
let backup_name = format!(\"agentforge\_{}.db\", timestamp);\
let backup_path = backup_dir.join(&backup_name);\
let conn = Connection::open(db_path)?;\
conn.execute(&format!(\"VACUUM INTO \'{}\'\", backup_path.display()),
\[\])?;\
Ok(backup_path)\
}

## **9.3 Recovery / 恢复**

Recovery process:

-   1\. Locate the most recent valid backup file

-   2\. Verify backup integrity: PRAGMA integrity_check

-   3\. Replace the current database file with the backup

-   4\. Replay WAL file if available

-   5\. Verify application functionality

## **9.4 RPO and RTO Targets / RPO和RTO目标**

  -----------------------------------------------------------------------
  **Metric**              **Target**              **Description**
  ----------------------- ----------------------- -----------------------
  RPO (Recovery Point     \< 5 minutes            Maximum acceptable data
  Objective)                                      loss window

  RTO (Recovery Time      \< 2 minutes            Maximum acceptable
  Objective)                                      downtime for recovery

  Backup Verification     Every backup            SHA-256 checksum
                                                  validation after each
                                                  backup

  Full Backup Frequency   Every 6 hours           Automated backup
                                                  interval (configurable)
  -----------------------------------------------------------------------

# **10. Performance Optimization / 性能优化**

## **10.1 Performance Targets / 性能目标**

  -----------------------------------------------------------------------
  **Operation**           **Target**              **Notes**
  ----------------------- ----------------------- -----------------------
  Single row read         \< 1 ms                 Indexed lookup by
                                                  primary key

  List query (100 rows)   \< 5 ms                 With WHERE clause on
                                                  indexed column

  Full-text search        \< 50 ms                FTS5 search across
                                                  knowledge entries

  INSERT (single row)     \< 1 ms                 Simple insert with
                                                  auto-generated UUID

  Batch INSERT (100 rows) \< 20 ms                Transaction-wrapped
                                                  batch insert

  Atomic task claim       \< 5 ms                 UPDATE with subquery on
                                                  indexed columns

  Dashboard aggregation   \< 100 ms               SUM/COUNT across
                                                  usage_stats with date
                                                  filter

  Database file size (1   \< 500 MB               With periodic VACUUM
  year)                                           and data archival
  -----------------------------------------------------------------------

## **10.2 Optimization Strategies / 优化策略**

-   WAL Mode: Enables concurrent reads during writes, essential for
    responsive UI

-   Prepared Statement Caching: Reuse prepared statements across
    repository calls to avoid SQL compilation overhead

-   Batch Operations: Wrap bulk inserts in a single transaction to
    minimize disk sync operations

-   Connection Pool: Maintain a pool of read connections for parallel
    dashboard queries

-   Appropriate Indexing: Index all WHERE, JOIN, and ORDER BY columns;
    avoid over-indexing write-heavy tables

-   Query Optimization: Use EXPLAIN QUERY PLAN to identify slow queries;
    avoid SELECT \*

-   VACUUM Schedule: Run PRAGMA incremental_vacuum weekly; full VACUUM
    monthly during idle time

-   ANALYZE: Run PRAGMA optimize after bulk data imports to update query
    planner statistics

-   Memory-Mapped I/O: Enable PRAGMA mmap_size for large read operations
    on knowledge entries

# **11. Security Considerations / 安全考虑**

## **11.1 SQL Injection Prevention / SQL注入防护**

All database queries use parameterized statements via rusqlite, which
automatically escapes user input:

-   Never concatenate user input into SQL strings

-   Use conn.execute(\"SELECT \* FROM agents WHERE id = ?\",
    \[user_id\])

-   rusqlite binds parameters as BLOB/TEXT/INTEGER/REAL with proper type
    checking

-   All repository methods accept typed parameters, not raw SQL strings

## **11.2 Data Encryption / 数据加密**

-   Encryption at Rest: Optional SQLCipher integration for encrypting
    the entire database file

-   Encryption in Transit: N/A for local SQLite; relevant for any future
    network replication

-   Sensitive Data: API keys stored in OS keychain (keyring crate),
    referenced by api_key_ref in providers table

-   JSON Fields: Sensitive fields within JSON columns (e.g.,
    system_prompt) are not encrypted at the DB level but protected by OS
    file permissions

## **11.3 Access Control / 访问控制**

Access control is enforced at the application layer, not the database
layer:

-   RBAC roles (Admin, Team Lead, Operator, Viewer) determine which
    operations are permitted

-   Repository methods check permissions before executing database
    operations

-   Audit log records all access decisions for compliance review

## **11.4 Audit Trail Immutability / 审计跟踪不可变性**

The audit_log table is strictly append-only:

-   No UPDATE or DELETE operations are permitted on audit_log records

-   The created_at column is set at insert time and never modified

-   Application code only calls AuditLogRepository::append(), never
    update or delete

-   Retention period: Configurable, minimum 12 months; export to
    external archive before deletion

# **12. Data Lifecycle / 数据生命周期**

## **12.1 Retention Policies / 保留策略**

  -----------------------------------------------------------------------
  **Table**               **Retention Period**    **Cleanup Strategy**
  ----------------------- ----------------------- -----------------------
  conversations           90 days after session   Archive to compressed
                          completion              file, then delete

  usage_stats             24 months               Aggregate to monthly
                                                  summaries, delete raw
                                                  daily records

  team_messages           30 days after team      Export to JSON archive,
                          instance completion     then delete

  audit_log               12 months minimum       Export to external
                                                  archive, then delete

  knowledge_entries       No automatic deletion   Manual review and
                                                  cleanup; Obsidian is
                                                  source of truth

  sessions                90 days after           Archive conversation
                          completion              data, delete session
                                                  record

  team_tasks              Until team instance     Archive with team
                          archived                instance, then delete
  -----------------------------------------------------------------------

## **12.2 Archival Strategy / 归档策略**

-   Archive Format: Compressed JSON files stored in
    {APP_DATA}/agentforge/archives/

-   Naming: {table}\_{date_range}.json.gz

-   Contents: Full table rows matching the retention filter

-   Metadata: Archive manifest with row counts, date range, and checksum

-   Restoration: Archive files can be re-imported via a dedicated
    restore command

## **12.3 Maintenance Jobs / 维护作业**

  -----------------------------------------------------------------------
  **Job**                 **Frequency**           **Description**
  ----------------------- ----------------------- -----------------------
  Data Cleanup            Daily at 02:00          Delete expired records
                                                  per retention policy

  Usage Rollup            Monthly on 1st          Aggregate daily stats
                                                  to monthly summaries

  VACUUM                  Weekly on Sunday        PRAGMA
                                                  incremental_vacuum to
                                                  reclaim space

  Full VACUUM             Monthly on 1st          Full VACUUM to optimize
                                                  database file

  ANALYZE                 After bulk imports      PRAGMA optimize to
                                                  update statistics

  Backup Verification     After each backup       SHA-256 checksum and
                                                  integrity check

  Index Rebuild           Quarterly               REINDEX for fragmented
                                                  indexes
  -----------------------------------------------------------------------
