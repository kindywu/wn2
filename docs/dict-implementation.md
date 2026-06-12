# dict 系统实现文档

> 本文档是 `dict-db-design.md` 的工程实现指南。技术栈为 **Rust 后端 + React/Ant Design 前端**，规定技术选型、代码结构、分阶段实施计划和每一步的验收标准。

---

## 1. 总体评估

### 1.1 设计文档质量

`dict-db-design.md` 的设计整体质量**优秀**，与具体编程语言无关，可直接作为工程实施的权威规范：

| 维度     | 评价  | 说明                                                             |
| -------- | ----- | ---------------------------------------------------------------- |
| 数据模型 | ★★★★★ | ENUM + `NULLS NOT DISTINCT` + JSONB 用法恰当，审计与来源追踪完备 |
| ETL 设计 | ★★★★★ | 冲突检测、优先级覆写、`pending_changes` 机制完整                 |
| API 设计 | ★★★★☆ | RESTful 路由完整，但缺少鉴权细节、分页规格、错误码规范           |
| 可运维性 | ★★★☆☆ | 缺少日志规范、监控指标、备份策略                                 |
| 测试策略 | ★★☆☆☆ | 未提及单元测试、集成测试、数据一致性校验                         |
| LLM 抽象 | ★★★☆☆ | 直接调用 `llm(...)` 过于笼统，缺少 Provider 抽象和容错设计       |

### 1.2 本实现文档补全的内容

1. **Rust 技术栈固化**：明确 crate 版本、框架选型、ORM 方式
2. **前端技术栈**：React + Ant Design + TypeScript + Vite
3. **工程结构**：Rust workspace + 前端目录的清晰分层
4. **分阶段计划**：从数据库到 ETL 到 Web Service 的递进式交付
5. **技术约束**：编码规范、事务边界、并发控制、安全要求
6. **测试与验收**：每阶段的可验证标准

---

## 2. 技术栈与约束

### 2.1 后端核心技术约束（不可更改）

| 层级           | 技术                                        | 版本/约束 | 理由                                                                          |
| -------------- | ------------------------------------------- | --------- | ----------------------------------------------------------------------------- |
| 语言           | Rust                                        | >= 1.80   | 性能、类型安全、并发模型适合 I/O 密集型服务                                   |
| Web 框架       | **Axum**                                    | >= 0.7    | Tokio 生态原生，中间件（Tower）成熟，与 `hyper` 深度集成                      |
| 数据库         | **PostgreSQL**                              | >= 16     | 设计文档要求；`citext` + `NULLS NOT DISTINCT` 为必需                          |
| ORM / Query    | **sqlx**                                    | >= 0.8    | 编译期 SQL 检查，零开销抽象；对 PG ENUM/Array/JSONB/CITEXT 直接原生支持       |
| Migration      | **sqlx-cli**                                | >= 0.8    | 与 sqlx 一体，纯 SQL migration（`up`/`down`），支持 `sqlx migrate run/revert` |
| 配置管理       | **config** + **dotenvy**                    | —         | 分层配置（default → file → env → cli args），支持 `.env`                      |
| 数据校验       | **validator** + **garde** (可选)            | —         | Request 参数校验；sqlx 模型配合 `validator` 做运行时校验                      |
| 序列化         | **serde** + **serde_json**                  | —         | Rust 标准                                                                     |
| 时间处理       | **chrono**                                  | —         | 与 sqlx PostgreSQL `TIMESTAMPTZ` 无缝映射                                     |
| HTTP Client    | **reqwest**                                 | —         | 基于 hyper，用于 LLM API 调用（DeepSeek 兼容 OpenAI API 格式）                |
| 任务调度       | **tokio-cron-scheduler** 或 **clokwerk**    | —         | 定时执行 ETL、清理 rejected pending_changes                                   |
| CLI 框架       | **clap**                                    | —         | ETL 命令行工具                                                                |
| 日志 / Tracing | **tracing** + **tracing-subscriber**        | —         | 结构化日志，支持 OpenTelemetry 扩展                                           |
| 错误处理       | **thiserror**（库）+ **anyhow**（应用/CLI） | —         | 清晰的分层错误处理                                                            |
| 重试与退避     | **backon** 或 **tokio-retry**               | —         | LLM 调用重试                                                                  |
| 限流           | **governor** 或 `tokio::sync::Semaphore`    | —         | LLM RPM 限制                                                                  |
| 构建工具       | **Cargo**                                   | —         | Rust 原生                                                                     |
| 容器化         | **Docker** + **Docker Compose**             | —         | 本地开发一键启动                                                              |

### 2.2 前端核心技术约束

| 层级        | 技术                                     | 说明                        |
| ----------- | ---------------------------------------- | --------------------------- |
| 框架        | **React 18**                             | —                           |
| 语言        | **TypeScript**                           | 严格模式 `strict: true`     |
| 构建工具    | **Vite**                                 | 快速 HMR，简洁配置          |
| UI 组件库   | **Ant Design (antd)**                    | >= 5.0，企业级后台组件丰富  |
| 路由        | **React Router v6**                      | —                           |
| HTTP 客户端 | **axios**                                | 拦截器统一处理 token 和错误 |
| 服务端状态  | **TanStack Query (React Query)**         | 缓存、轮询、乐观更新        |
| 客户端状态  | **Zustand** 或 Context                   | 轻量，无需 Redux            |
| 表格/列表   | **antd Table** + 自定义分页              | —                           |
| 表单        | **antd Form** + **ProComponents** (可选) | 复杂表单用 ProForm          |

### 2.3 重要技术决策记录 (ADR)

#### ADR-001：sqlx vs SeaORM

- **决策**：使用 **sqlx** 作为主体数据库工具包，所有查询以手写 SQL + `query!` / `query_as!` 宏为主，复杂场景辅以 `sqlx::query()` 动态构建。
- **理由**：
  - **编译期 SQL 检查**：`query!` 宏在编译时连接数据库校验语句，避免运行时 SQL 错误。
  - **直接控制 SQL**：不引入 ORM 的 DSL 抽象层，便于精确控制 `JOIN`、`CTE`、`WINDOW FUNCTION`、`UPSERT` 等 PostgreSQL 高级特性。
  - **对 PG 特有一等公民支持**：`ENUM`、`TEXT[]`、`JSONB`、`CITEXT` 可直接在 SQL 中使用，通过 `#[derive(sqlx::Type)]` 映射到 Rust enum / `Vec<T>` / `serde_json::Value`。
  - **生态简洁**：`sqlx` + `sqlx-cli` 一套工具即可覆盖连接池、迁移、查询；无需额外生成 Entity 代码，数据模型即普通 `#[derive(sqlx::FromRow)]` 结构体。
- **约束**：
  - 大规模 ETL 批量插入优先使用 `COPY FROM` 或 `UNNEST` + 参数化数组，而非逐条 `INSERT`。
  - 所有 Schema 变更通过 `sqlx migrate` 的纯 SQL 文件管理，禁止在业务代码中隐式建表。

#### ADR-002：Axum 而非 Actix-web

- **决策**：使用 **Axum**。
- **理由**：
  - Axum 与 `tokio`、`tower`（中间件、`Service` trait）生态无缝衔接。
  - 错误处理模型（`Result<impl IntoResponse, AppError>`）更简洁。
  - 与 sqlx 的 async runtime（基于 tokio）兼容无摩擦。

#### ADR-003：Rust Workspace 结构

- **决策**：使用 Cargo Workspace，拆分为 4 个 crate。**目录名（相对路径）** 与 **Cargo package name** 的对应关系如下：

  | 目录         | Cargo package name | 说明                     |
  | ------------ | ------------------ | ------------------------ |
  | `models/`    | `dict-models`      | 共享数据模型 + sqlx 类型 |
  | `migration/` | `dict-migration`   | sqlx Migration 管理      |
  | `api/`       | `dict-api`         | Axum Web Service         |
  | `etl/`       | `dict-etl`         | ETL 离线任务             |

  二进制名：`dict-api`（`cargo run --bin dict-api`）和 `dict-etl`（`cargo run --bin dict-etl`）。
- **理由**：ETL 和 API 共享数据模型定义，但依赖不同（ETL 不需要 Axum）。Workspace 避免代码重复。`dict-models` 仅包含纯数据结构（`#[derive(sqlx::FromRow, serde::Serialize)]`），无 Web 框架依赖。

#### ADR-004：ETL 与 Web Service 分离进程

- **决策**：ETL 编译为独立二进制 `dict-etl`，不内嵌在 API 进程内。
- **理由**：ETL 是离线批处理，可能耗时数小时；API 是在线服务。两者生命周期不同。生产环境 ETL 跑在 CronJob / 批量计算节点。

#### ADR-005-A：SQLite 读取策略（stardict.db / wn.db）

- **决策**：使用 **`rusqlite`**（同步阻塞）读取 SQLite 数据库，在 tokio 异步运行时中通过 `tokio::task::spawn_blocking` 包装调用。
- **理由**：
  - sqlx 的 `sqlite` feature 在 tokio runtime 下也支持 async，但需要额外在 `Cargo.toml` 启用 `sqlite` feature，且 sqlx 对 SQLite 的支持不如 postgres 成熟。
  - ETL 是离线批量任务，SQLite 读取是 CPU/IO 密集型同步操作，`spawn_blocking` 是 tokio 的标准模式。
  - `rusqlite` 成熟稳定，API 直观，不引入额外复杂性。
- **约束**：所有 `rusqlite` 调用必须在 `spawn_blocking` 闭包内。ETL 中不直接 `await` 同步阻塞操作。

#### ADR-005：LLM Provider 抽象层

- **决策**：所有 LLM 调用必须通过统一 `LlmClient` trait，禁止在业务代码中直接 `reqwest::post(api_url)`。
- **当前默认 Provider**：**DeepSeek API**（`https://api.deepseek.com`，兼容 OpenAI API 格式）。
- **理由**：
  - DeepSeek 中英双语能力强，价格极低，适合大规模词典生成任务。
  - 兼容 OpenAI SDK / `reqwest` + JSON 格式，切换成本低。
  - 统一抽象后，未来可无缝替换为 OpenAI / Claude / 本地模型。
- **当前可用模型名（截至 2025 年）**：
  - 非思考：`deepseek-v4-flash`（即 DeepSeek-V3，快速、低成本，适合 fill_gaps）
  - 思考：`deepseek-v4-pro`（即 DeepSeek-R1，深度推理，适合 evaluate）
  - **⚠️ 模型名以 [DeepSeek 官方文档](https://platform.deepseek.com/api-docs/) 为准，文档中所有 `deepseek-v4-flash` / `deepseek-v4-pro` 均为占位名，实际部署前需替换为当时最新的真实模型 ID。**

---

## 3. 工程目录结构

```
dict/
├── Cargo.toml                  # Workspace 定义
├── Cargo.lock
├── docker/
│   ├── Dockerfile.api          # 多阶段构建 Rust API 镜像
│   ├── Dockerfile.etl          # ETL 运行镜像
│   └── docker-compose.yml      # postgres + api + nginx
├── frontend/                   # === React + Ant Design 前端 ===
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── router.tsx          # React Router 配置
│       ├── api/
│       │   ├── client.ts       # axios 实例 + 拦截器
│       │   ├── words.ts        # /api/v1/words/*
│       │   ├── definitions.ts
│       │   ├── translations.ts
│       │   ├── examples.ts
│       │   ├── relations.ts
│       │   ├── forms.ts
│       │   ├── pending.ts      # pending-changes
│       │   ├── evaluations.ts
│       │   └── logs.ts         # import-logs, change-log
│       ├── pages/
│       │   ├── SearchPage.tsx          # 单词搜索/详情
│       │   ├── PendingChangesPage.tsx  # 冲突审批队列
│       │   ├── EvaluationsPage.tsx     # LLM 评价队列
│       │   ├── AuditLogPage.tsx        # 审计日志
│       │   └── ImportLogPage.tsx       # 导入记录
│       ├── components/
│       │   ├── WordDetailCard.tsx      # 单词信息卡片
│       │   ├── DefinitionList.tsx
│       │   ├── TranslationList.tsx
│       │   ├── ExampleList.tsx
│       │   ├── RelationList.tsx
│       │   ├── PendingChangeItem.tsx   # 审批卡片
│       │   └── EvaluationItem.tsx      # 评价卡片
│       ├── hooks/
│       │   ├── useWords.ts             # TanStack Query hooks
│       │   ├── usePendingChanges.ts
│       │   └── useEvaluations.ts
│       ├── types/
│       │   └── api.ts            # TypeScript 接口定义（与 Rust DTO 对应）
│       └── stores/
│           └── authStore.ts      # 简单全局状态（token 等）
├── models/                     # === 共享数据模型 + sqlx 类型映射 ===
│   └── src/
│       ├── lib.rs
│       ├── prelude.rs          # re-export 常用类型
│       ├── enums.rs            # PG ENUM → Rust enum（#[derive(sqlx::Type)]）
│       ├── words.rs
│       ├── definitions.rs
│       ├── translations.rs
│       ├── examples.rs
│       ├── word_relations.rs
│       ├── word_forms.rs
│       ├── evaluations.rs
│       ├── pending_changes.rs
│       ├── change_log.rs
│       └── import_log.rs
├── migration/                  # === sqlx Migration（纯 SQL） ===
│   ├── Cargo.toml
│   └── migrations/
│       ├── 20250101000000_init.sql   # 基线迁移：扩展、枚举、函数、表、索引、触发器
├── api/                        # === Web Service (Axum) ===
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # axum::serve 入口
│       ├── config.rs           # 配置结构（config crate）
│       ├── error.rs            # AppError + IntoResponse
│       ├── state.rs            # AppState（PgPool 等）
│       ├── middleware/
│       │   ├── auth.rs         # Bearer Token 校验
│       │   └── logging.rs      # tracing 请求日志
│       ├── routers/
│       │   ├── mod.rs          # 路由合并
│       │   ├── words.rs        # /api/v1/words/*
│       │   ├── definitions.rs
│       │   ├── translations.rs
│       │   ├── examples.rs
│       │   ├── relations.rs
│       │   ├── forms.rs
│       │   ├── pending.rs      # pending-changes
│       │   ├── evaluations.rs
│       │   └── logs.rs         # import-logs, change-log
│       ├── services/
│       │   ├── mod.rs
│       │   ├── word_service.rs       # 单词聚合查询
│       │   ├── audit_service.rs      # change_log 写入
│       │   ├── pending_service.rs    # pending_changes 审批
│       │   └── eval_service.rs       # evaluations 处置
│       └── dto/
│           ├── mod.rs
│           ├── word.rs
│           ├── definition.rs
│           ├── translation.rs
│           ├── example.rs
│           ├── relation.rs
│           ├── form.rs
│           ├── pending.rs
│           ├── evaluation.rs
│           └── log.rs
├── etl/                        # === ETL Pipeline ===
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # clap CLI 入口
│       ├── config.rs           # ETL 专用配置
│       ├── db.rs               # sqlx PgPool 连接
│       ├── common.rs           # POS_MAP, 优先级判断, 事务 helper
│       ├── llm_client.rs       # LLM Provider trait + 实现
│       ├── import_stardict.rs  # 步骤 1
│       ├── import_wn.rs        # 步骤 2
│       ├── fill_gaps.rs        # 步骤 3 (LLM 补全)
│       └── evaluate.rs         # 步骤 4 (LLM 评价)
└── .env.example
```

### 3.1 Workspace `Cargo.toml`

```toml
[workspace]
members = ["models", "migration", "api", "etl"]
resolver = "2"

[workspace.dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-native-tls", "postgres", "chrono", "json", "uuid", "migrate"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["clock"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

---

## 4. 分阶段实施计划

### 阶段 0：脚手架与基础设施（第 1 周）

**目标**：Rust workspace 可编译，数据库可创建，API 可启动，前端可运行。

#### 步骤 0.1 Rust Workspace 初始化

- [ ] 创建根目录 `dict/`，编写根 `Cargo.toml`（workspace 定义）
- [ ] 初始化 4 个 member crate：`models`, `migration`, `api`, `etl`
- [ ] 统一依赖到 workspace root，子 crate 使用 `workspace = true`

**验收标准**：`cargo check` 在 workspace root 成功，无编译错误。

#### 步骤 0.2 Docker 开发环境

- [ ] 编写 `docker-compose.yml`：PostgreSQL 16 容器，带 `POSTGRES_DB=dict`
- [ ] 编写 `Dockerfile.api`：多阶段构建（chef / cargo-chef 缓存依赖层，或使用标准 builder pattern）
- [ ] 本地验证：`docker compose up -d postgres` 后，可 `psql` 连接

**验收标准**：`docker compose up -d postgres` 后，`psql postgresql://postgres:postgres@localhost:5432/dict` 可连接。

#### 步骤 0.3 sqlx Migration 基线

- [ ] 安装 `sqlx-cli`：`cargo install sqlx-cli --no-default-features --features native-tls,postgres`
- [ ] 创建 migration 目录：`sqlx migrate add --source migration/migrations init`
- [ ] 将 `dict-db-design.md` 中的**完整建表 SQL**（第 5 节）写入 `migration/migrations/<timestamp>_init.sql`：
  - 先 `CREATE EXTENSION IF NOT EXISTS citext`
  - 再 `CREATE TYPE ... ENUM`
  - 再 `CREATE OR REPLACE FUNCTION set_updated_at()` / `check_evaluation_target()`
  - 再建表（注意 `import_log` 在 `pending_changes` 之前）
  - 再建索引
  - 再绑定触发器
- [ ] 在 `migration/migrations/<timestamp>_init_down.sql` 中按相反顺序清理

**技术约束**：
- 所有 schema 变更必须通过 `sqlx migrate` 管理的纯 SQL 文件，**禁止**在应用代码中隐式建表或改表。
- 开发时启用 `sqlx prepare`（`cargo sqlx prepare`）将查询元数据缓存到 `.sqlx/` 中，供 CI 无数据库编译检查。

**验收标准**：`cargo sqlx migrate run --source migration/migrations` 在空数据库成功执行；`cargo sqlx migrate revert --source migration/migrations` 可回滚。

#### 步骤 0.4 sqlx Model 定义与校验

- [ ] 在 `models/src/` 下手工定义所有表对应的 Rust 结构体（`#[derive(sqlx::FromRow)]`）和共享枚举（`#[derive(sqlx::Type)]`）。无需代码生成工具。
- [ ] 关键映射检查清单：
  - `CITEXT` → `String`（查询时 PG 自动大小写不敏感，Rust 侧无需特殊类型）
  - `ENUM` → 自定义 Rust enum + `#[derive(sqlx::Type)]`（见阶段 1.1）
  - `TEXT[]` → `Vec<String>`
  - `JSONB` → `serde_json::Value`
  - `TIMESTAMPTZ` → `chrono::DateTime<chrono::Utc>` 或 `chrono::DateTime<chrono::FixedOffset>`
  - `BIGINT` / `GENERATED ALWAYS AS IDENTITY` → `i64`
- [ ] `models/src/prelude.rs` 统一 re-export，供 `api` 和 `etl` 引用。

**验收标准**：`cargo check -p models` 通过。

#### 步骤 0.5 Axum 最小骨架

- [ ] `api/src/main.rs` 创建 Axum app，挂载 `/healthz` GET 路由
- [ ] `api/src/config.rs` 使用 `config` crate + `dotenvy` 读取 `.env`
- [ ] `api/src/state.rs` 定义 `AppState { db: sqlx::PgPool }`
- [ ] `api/src/error.rs` 定义 `AppError` 和 `IntoResponse` 实现
- [ ] 配置 `tracing-subscriber` 输出 JSON 格式日志

**验收标准**：`cargo run --bin dict-api` 启动后，`curl http://localhost:8000/healthz` 返回 `{"status":"ok"}`。

#### 步骤 0.6 前端脚手架

- [ ] `cd frontend && npm create vite@latest . -- --template react-ts`
- [ ] 安装依赖：`antd`, `axios`, `react-router-dom`, `@tanstack/react-query`, `zustand`
- [ ] 配置 `vite.config.ts` 代理 `/api` 到 `http://localhost:8000`
- [ ] 创建基础布局：`App.tsx` 引入 antd `ConfigProvider`，配置路由骨架

**验收标准**：`npm run dev` 启动后，浏览器访问 `http://localhost:5173` 看到基础页面。

---

### 阶段 1：数据库模型层完整实现（第 2 周）

**目标**：所有表、索引、触发器、枚举通过 sqlx Migration 完整落地，模型定义可被 Rust 代码引用。

#### 步骤 1.1 自定义类型映射完善

sqlx 对 PostgreSQL 特有类型通过 `#[derive(sqlx::Type)]` 直接映射，无需代码生成：

- [ ] **ENUM 映射**：
  ```rust
  // models/src/enums.rs
  #[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, serde::Serialize, serde::Deserialize)]
  #[sqlx(type_name = "data_source", rename_all = "snake_case")]
  pub enum DataSource {
      Stardict,
      Wn,
      Llm,
      Human,
  }
  ```
  - 同理定义 `PosType`, `FormType`, `ChangeAction`, `ChangeStatus`, `SeverityLevel`, `ImportMode`, `ImportStatus`
- [ ] **数组类型**：`TEXT[]` 在 Rust 中直接使用 `Vec<String>`，`query_as!` 自动映射
- [ ] **JSONB**：`serde_json::Value`，配合 `sqlx::types::Json<T>` 可实现强类型 JSON 字段

#### 步骤 1.2 数据质量排序函数封装

设计文档中的 `ORDER BY` 规则（第 1.3 节）在多处使用：

- [ ] 在 Migration 中创建 PostgreSQL 函数 `data_quality_priority(source text, reviewed bool, modified bool)`：
  ```sql
  CREATE OR REPLACE FUNCTION data_quality_priority(
      source data_source, reviewed boolean, modified boolean
  ) RETURNS integer AS $$
  BEGIN
      IF source = 'human' OR modified THEN RETURN 0; END IF;
      IF source IN ('stardict', 'wn') AND reviewed THEN RETURN 1; END IF;
      IF source IN ('stardict', 'wn') AND NOT reviewed THEN RETURN 2; END IF;
      IF source = 'llm' AND reviewed THEN RETURN 3; END IF;
      RETURN 4;
  END;
  $$ LANGUAGE plpgsql IMMUTABLE;
  ```
- [ ] 在 sqlx 查询中直接在 SQL 中使用：
  ```rust
  sqlx::query_as::<_, Definition>(
      "SELECT * FROM definitions WHERE word_id = $1 \
       ORDER BY data_quality_priority(source, reviewed, modified)"
  )
  .bind(word_id)
  .fetch_all(&pool)
  .await?;
  ```

**验收标准**：一个查询测试验证排序结果符合设计文档优先级。

#### 步骤 1.3 模型关系与查询模式

- [ ] 在 `models/src/prelude.rs` 中统一 re-export 所有结构体和枚举
- [ ] sqlx 无内置 `Related` 关系，所有关联查询通过显式 `JOIN` 或分离查询实现：
  - 简单一对多：先查主表，再按 `word_id IN (...)` 批量查从表
  - 聚合展示：手写 `JOIN` SQL 一次性取出（如 `word_service.rs` 中的单词详情聚合查询）
- [ ] 确保 `ON DELETE CASCADE` 在 migration SQL 中已声明，Rust 代码中无需额外处理

**验收标准**：`cargo check -p models` 通过；至少一个关联查询编译通过 `cargo sqlx prepare`。

---

### 阶段 2：ETL Pipeline 实现（第 3–4 周）

**目标**：实现 4 步 ETL，可将 `stardict.db` 和 `wn.db` 的数据完整导入 `dict` 数据库。

#### 步骤 2.1 ETL 共享基础设施

- [ ] `etl/src/config.rs`：ETL 配置结构（数据库 URL、LLM 配置、batch size）
- [ ] `etl/src/db.rs`：创建 `sqlx::PgPool::connect()`，返回 `PgPool`
- [ ] `etl/src/common.rs`：
  - `POS_MAP: HashMap<&str, &str>`（设计文档 7.3 节）
  - `FIELD_PRIORITY: HashMap<&str, Vec<&str>>`（设计文档 7.2 节）
  - `fn normalize_pos(raw: &str) -> Option<&str>`
  - `fn source_priority(field: &str, source: &str) -> i32`
  - `async fn in_transaction<F, Fut, T>(pool: &PgPool, f: F) -> Result<T, sqlx::Error>`

#### 步骤 2.2 import_stardict（步骤 1）

- [ ] 使用 `rusqlite` + `tokio::task::spawn_blocking` 读取 `data/stardict.db`（见 ADR-005-A）
- [ ] 按设计文档 7.4 节流程：
  - normalize_pos → upsert words → append sources → insert definitions / translations / examples / word_forms
- [ ] `examples` 解析：调研 `detail` JSON 的实际结构，写解析函数
- [ ] `word_forms` 解析：`exchange` JSON 按映射表展开
- [ ] 批量插入：使用 `sqlx::query!` + `UNNEST` 批量执行，或 `COPY FROM` 做超大批量写入

**技术约束**：
- ETL 必须记录 `import_log`。启动时先插入 running 记录，获取 `import_log_id`。
- 结束或异常时更新为 completed/failed。
- 单条词内的所有附属数据（definitions + translations + examples + forms）原子写入；不同词之间每 1000 词 commit 一次。

#### 步骤 2.3 import_wn（步骤 2）

**wn.db 数据来源说明**：

`wn.db` 是使用 [`wn` Python 库](https://wn.readthedocs.io/)（或 `wn export`）从 Open English WordNet（OEWN）和 Open Multilingual Wordnet（OMW）生成的 SQLite 文件。其核心表结构如下（简化）：

| 表                | 关键字段                           | 说明                           |
| ----------------- | ---------------------------------- | ------------------------------ |
| `entry`           | `id`, `lemma`, `pos`, `lexicon_id` | 词目（lemma + pos）            |
| `form`            | `entry_id`, `written_form`, `rank` | 词形（rank=0 是原形）          |
| `sense`           | `id`, `entry_id`, `synset_id`      | 词目↔synset 映射               |
| `synset`          | `id`, `ili`, `pos`, `definition`   | 概念集                         |
| `synset_example`  | `synset_id`, `text`                | 例句                           |
| `synset_relation` | `source_id`, `target_id`, `type`   | synset 间关系                  |
| `sense_relation`  | `source_id`, `target_id`, `type`   | sense 间关系（同义/反义）      |
| `pronunciation`   | `entry_id`, `value`                | IPA 音标                       |
| `lexicon`         | `id`, `language`, `version`        | 词典元数据（en=OEWN, cmn=OMW） |

**中文翻译来源**：`lexicon.language = 'cmn'`（OMW 普通话）的 `entry` 表，通过 `sense.synset_id → synset.ili → OEWN synset` 跨语言映射。

**如果直接访问 wn.db 太复杂**：可先用 Python 脚本导出中间 JSON 文件（每词一行），再由 Rust ETL 读取 JSON。此方法隔离了 wn 内部格式变化，但增加了一个中间步骤。

- [ ] 使用 `rusqlite` + `tokio::task::spawn_blocking` 读取 `data/wn.db`（见 ADR-005-A）
- [ ] **阶段 1**：导入 lemmas → `words` 表。使用 `INSERT ... ON CONFLICT (word, pos) DO UPDATE`（UPSERT）
- [ ] **阶段 2**：从 oewn synsets 导入 definitions + examples
- [ ] **阶段 3**：通过 ILI 映射导入中文 translations
  - 内存中建立 `HashMap<ili, Vec<en_word_id>>` 和 `HashMap<ili, Vec<cmn_lemma>>`
- [ ] **阶段 4**：展开 synset_relations → `word_relations`。对称关系（synonym/antonym/similar）写双向
- [ ] **阶段 5**：补充 `word_forms`

**技术约束**：
- wn 数据量较大，ILI 映射和关系展开需先在内存聚合，避免 N+1。
- ETL 末尾运行校验 SQL：确认 synonym/antonym/similar 的双向对称性。
- 全量导入目标耗时 **< 30 分钟**。

#### 步骤 2.4 fill_gaps（步骤 3）

- [ ] 遍历缺失 translations / definitions / examples 的词
- [ ] 按设计文档 8.1 节顺序：补翻译 → 补定义 → 补例句 → 补例句翻译
- [ ] 调用 `llm_client.generate(prompt, task_type).await`

**模型选型**：
- **fill_gaps 使用非思考模式**（对应 `deepseek-v4-flash` / DeepSeek-V3，或当时最新的快速模型，见 ADR-005 中的模型名说明）。任务为翻译/定义/例句生成，量大且标准，非思考模式速度快、成本低，中英双语表现优秀。

**技术约束**：
- **严格幂等**：同一词多次运行不重复生成。查询条件 `WHERE NOT EXISTS (SELECT 1 FROM translations t WHERE t.word_id = w.id AND t.source = 'llm')`。
- **限流与重试**：使用 `tokio::sync::Semaphore` 控制并发，`backon` 实现指数退避（只重试 429 / 5xx）。
- **批量处理**：每 100 词 commit 一次（`sqlx::Transaction::commit()`），失败记录到日志而不中断。

#### 步骤 2.5 evaluate（步骤 4）

- [ ] 遍历已有数据，按 8.2 节维度评分
- [ ] LLM 返回严格 JSON，`serde_json::from_str()` 解析
- [ ] `score < 4` 写入 `evaluations`

**模型选型**：
- **evaluate 使用思考模式**（对应 `deepseek-v4-pro` / DeepSeek-R1，见 ADR-005 中的模型名说明）。
- 评价任务需要批判性推理（尤其是跨源一致性检查），思考模式推理深度更可靠。
- 评价量远小于生成量，成本可控。

**技术约束**：
- Prompt 必须要求 `"返回严格 JSON，不要 markdown 代码块"`。
- 解析失败跳过，记录 error。
- 幂等：同一 `(target_table, target_id, dimension)` 不重复写入未审核记录。

#### 步骤 2.6 ETL CLI 与测试

- [ ] `etl/src/main.rs` 使用 `clap` 定义子命令：
  ```
  dict-etl stardict --mode full
  dict-etl wn --mode full
  dict-etl fill-gaps
  dict-etl evaluate
  ```
- [ ] 编写测试：
  - `common::normalize_pos` 单元测试
  - `import_stardict` 在测试数据库上运行，断言行数
  - 幂等性测试：同一数据运行两次，行数不变

**验收标准**：
- `cargo run --bin dict-etl -- stardict --mode full` 成功，words 表有数据。
- 重复运行不产生重复行。

---

### 阶段 3：REST API 实现（第 5–6 周）

**目标**：实现设计文档第 9 节所有 API，支持查询、审核、编辑、新建、审批、日志。

#### 步骤 3.1 DTO (Request/Response) 定义

- [ ] `api/src/dto/*.rs` 使用 `serde` + `validator` 定义：
  - `WordOut`, `WordSearchQuery`（`q`, `pos`, `limit`, `offset`）
  - `DefinitionOut`, `DefinitionCreate`, `DefinitionUpdate`
  - `TranslationOut`, `ExampleOut`, `ExampleUpdate`
  - `PendingChangeOut`, `PendingChangeApproveRequest`（可选 `value: Option<String>`）
  - `EvaluationOut`, `EvaluationAction`
  - `ChangeLogOut`, `ImportLogOut`
- [ ] 统一包装响应：`ApiResponse<T>` 和 `ApiError`

#### 步骤 3.2 核心查询接口

- [ ] `GET /api/v1/words?q=bank&pos=n&limit=20&offset=0&mode=exact`
  - 搜索模式（`mode` 参数，默认 `exact`）：
    - `exact`：`word = $q`（CITEXT 大小写不敏感精确匹配）
    - `prefix`：`word LIKE $q || '%'`（前缀匹配，不区分大小写）
  - 两种模式均命中 `idx_words_word`（CITEXT B-tree 索引支持前缀 LIKE）
  - 返回 `Vec<WordOut>` + `meta.total`（总匹配数，用于前端分页）
- [ ] `GET /api/v1/words/{id}` → 单词基础信息
- [ ] `GET /api/v1/words/{id}/definitions` → 按 `data_quality_priority` 排序
- [ ] `GET /api/v1/words/{id}/translations`
- [ ] `GET /api/v1/words/{id}/examples`
- [ ] `GET /api/v1/words/{id}/relations?type=synonym`
- [ ] `GET /api/v1/words/{id}/forms`

**技术约束**：
- 所有附属数据查询应用质量排序规则。
- 列表接口必须分页：`limit` (default 20, max 100)，`offset` (default 0)。
- 使用手写 `JOIN` SQL 或 `IN (...)` 批量查询避免 N+1。

#### 步骤 3.3 审核与编辑接口

- [ ] `PATCH /api/v1/definitions/{id}/review` → `reviewed=true`, 写 `change_log`
- [ ] `PATCH /api/v1/definitions/{id}` → 修改 `text`，自动 `modified=true`, `reviewed=true`
- [ ] 同理实现 translations / examples / word_relations / word_forms
- [ ] `PATCH /api/v1/words/{id}/curate` → `curated=true`

**技术约束**：
- 所有写操作必须在**同一数据库事务**中完成。
- Axum handler 中使用 `pool.begin().await?` 开启 `sqlx::Transaction<'_, Postgres>`，在 service 层传入 `&mut Transaction<'_, Postgres>`。
- 事务末尾统一 `commit().await?`。
- `change_log.operator` 从 HTTP Header `X-Operator-ID` 读取，无则使用 `"system"` 作为默认值（数据库层已设 `DEFAULT 'system' NOT NULL`，API 层显式传值仍为佳）。

#### 步骤 3.4 新建接口（source=human）

- [ ] `POST /api/v1/words/{id}/definitions`
- [ ] `POST /api/v1/words/{id}/translations`
- [ ] `POST /api/v1/words/{id}/examples`
- [ ] `POST /api/v1/words/{id}/relations`

**技术约束**：
- 新建前检查唯一性约束，冲突返回 `409 CONFLICT`。
- 必须 `source='human'`, `reviewed=true`, `modified=false`。
- 必须写 `change_log`，`action='create'`。

#### 步骤 3.5 pending_changes 审批接口

- [ ] `GET /api/v1/pending-changes?status=pending&source=wn`
- [ ] `PATCH /api/v1/pending-changes/{id}/approve`
  - body 可选 `{ "value": "edited text" }`
  - 事务：UPDATE 目标表 → INSERT change_log → UPDATE pending_changes.status='approved'
- [ ] `PATCH /api/v1/pending-changes/{id}/reject`

**技术约束**：
- `pending_service.apply()` 需处理目标字段类型转换（如 `collins` → `SMALLINT`）。
- 拒绝审批只更新 `pending_changes`，不触碰目标表。
- sqlx 查询返回 `Option<T>`，目标行不存在时返回 `404`。

#### 步骤 3.6 evaluations 处置接口

- [ ] `GET /api/v1/evaluations?reviewed=false&severity=critical`
- [ ] `PATCH /api/v1/evaluations/{id}/agree` → UPDATE 目标表 + UPDATE evaluations
- [ ] `PATCH /api/v1/evaluations/{id}/dismiss` → 仅 UPDATE evaluations

#### 步骤 3.7 运维接口

- [ ] `GET /api/v1/import-logs`
- [ ] `GET /api/v1/import-logs/{id}`
- [ ] `GET /api/v1/change-log?operator=alice&from=2025-01-01&limit=50`

#### 步骤 3.8 API 测试

- [ ] `api/tests/` 下编写集成测试：
  - 使用 `axum::Router` + `tower::ServiceExt` 做内存测试，或启动测试数据库
  - 每个写接口断言：数据库状态 + change_log 记录
- [ ] 使用 `sqlx::test` 宏（`#[sqlx::test]`）自动管理测试数据库事务回滚

**验收标准**：`cargo test -p api` 全部通过；Swagger UI（如有）或路由表完整。

---

### 阶段 4：LLM 客户端与调度（第 7 周）

#### 步骤 4.1 LlmClient trait 与实现

**System Prompt 规范**：

| 任务               | System Prompt 参考                                                                                                                           |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `fill_gaps` / 翻译 | `You are a professional lexicographer. Output only the requested data in the exact format specified. Do not add explanations or markdown.`   |
| `fill_gaps` / 定义 | （同上）+ 约束词：`One concise English sentence.`                                                                                            |
| `fill_gaps` / 例句 | （同上）+ 约束词：`Output one sentence per line, no numbering.`                                                                              |
| `evaluate`         | `You are a strict dictionary quality reviewer. Evaluate the given data and respond ONLY with a valid JSON object. No markdown, no preamble.` |

所有 evaluate 任务的 User Prompt 必须以 `Return ONLY valid JSON, no markdown code blocks.` 结尾。

- [ ] `etl/src/llm_client.rs`：
  ```rust
  #[derive(Clone, Copy, Debug)]
  pub enum LlmTaskType {
      Generate,   // fill_gaps：翻译/定义/例句生成
      Evaluate,   // evaluate：质量打分、批判性推理
  }

  #[async_trait]
  pub trait LlmClient: Send + Sync {
      /// 根据任务类型自动选择模型和模式
      async fn generate(&self, prompt: &str, task: LlmTaskType) -> anyhow::Result<String>;
      async fn generate_json<T: DeserializeOwned>(&self, prompt: &str, task: LlmTaskType) -> anyhow::Result<T>;
  }
  ```
- [ ] `DeepSeekClient` 实现（兼容 OpenAI API）：
  - `base_url = "https://api.deepseek.com"`
  - 支持 `thinking` 模式切换：`fill_gaps` → 关闭；`evaluate` → 开启（`reasoning_effort = medium`）
  - 支持 `system` prompt 注入
- [ ] 配置项（`.env`）：
  - `LLM_PROVIDER=deepseek`
  - `LLM_API_KEY=sk-...`
  - `LLM_MODEL_GENERATE=deepseek-v4-flash`（非思考模型，对应 fill_gaps；以官方文档为准）
  - `LLM_MODEL_EVALUATE=deepseek-v4-pro`（思考模型，对应 evaluate；以官方文档为准）
  - `LLM_RPM=60`
  - `LLM_MAX_RETRIES=3`
  - `LLM_TIMEOUT_SECONDS=60`
- [ ] **兼容性**：实现内部使用 OpenAI API 格式（`POST /chat/completions`），未来切换 Provider 只需改 `base_url` 和 `model` 名。

#### 步骤 4.2 重试与限流

- [ ] 使用 `backon` 实现指数退避重试（只重试 429 / 5xx，不重试 4xx）
- [ ] 使用 `tokio::sync::Semaphore` 或 `governor` 实现 RPM 限流

#### 步骤 4.3 ETL 调度

- [ ] `scripts/run_etl.sh`：
  ```bash
  cargo run --bin dict-etl -- stardict --mode full
  cargo run --bin dict-etl -- wn --mode full
  cargo run --bin dict-etl -- fill-gaps
  cargo run --bin dict-etl -- evaluate
  ```
- [ ] （可选）引入 `tokio-cron-scheduler` 做定时调度，或保持外部 cron

---

### 阶段 5：前端审核 UI（第 8–9 周）

#### 步骤 5.1 API 客户端封装

- [ ] `frontend/src/api/client.ts`：axios 实例，统一 baseURL `/api/v1`，注入 Bearer Token
- [ ] 统一错误处理：后端返回 `{ error: { code, message } }` 时，用 antd `message.error()` 提示
- [ ] 按资源分文件：`words.ts`, `pending.ts`, `evaluations.ts` 等

#### 步骤 5.2 类型定义

- [ ] `frontend/src/types/api.ts` 定义 TypeScript 接口，与 Rust DTO 保持字段名一致：
  ```typescript
  export interface WordOut {
    id: number;
    word: string;
    pos: 'n' | 'v' | 'a' | 'r' | 's' | null;
    phonetic: string | null;
    // ...
  }
  ```

#### 步骤 5.3 页面实现

- [ ] **搜索页 (`SearchPage.tsx`)**：
  - 顶部搜索框（antd `Input.Search`）
  - 结果列表点击展开详情
  - 详情页展示 WordDetailCard（含 definitions / translations / examples / relations / forms）
  - 每条记录可操作：Mark Reviewed / Edit
- [ ] **Pending Changes 审批页 (`PendingChangesPage.tsx`)**：
  - antd `List` 或 `Table` 展示冲突项
  - 每项展示 `old_value` vs `new_value`
  - 操作按钮：Approve / Edit & Approve / Reject
  - Edit & Approve 使用 antd `Modal` + `Form` + `Input`
- [ ] **Evaluations 处置页 (`EvaluationsPage.tsx`)**：
  - 按 severity 分组或排序（critical 红色高亮）
  - 展示 score + comment + suggestion
  - 操作：Apply Suggestion / Dismiss
- [ ] **Audit Log 页 (`AuditLogPage.tsx`)**：
  - antd `Table` 展示 change_log，支持按 operator 和日期筛选
- [ ] **Import Log 页 (`ImportLogPage.tsx`)**：
  - 展示导入历史列表

#### 步骤 5.4 路由与布局

- [ ] `frontend/src/router.tsx` 配置路由：
  - `/` → SearchPage
  - `/pending` → PendingChangesPage
  - `/evaluations` → EvaluationsPage
  - `/audit-log` → AuditLogPage
  - `/import-logs` → ImportLogPage
- [ ] 基础布局：antd `Layout`（Sider + Content），Sider 放菜单导航

#### 步骤 5.5 状态管理

- [ ] 使用 TanStack Query 管理服务端状态：
  - `useWords(query)`, `usePendingChanges(filters)`, `useEvaluations(filters)`
- [ ] 使用 Zustand 管理客户端全局状态：
  - `authStore`：存储 Bearer Token（简单字符串）

#### 步骤 5.6 前端测试

- [ ] 使用 Vitest 编写单元测试（工具函数）
- [ ] 使用 React Testing Library 测试组件交互（可选）

**验收标准**：
- 前端可搜索单词并查看详情
- 可对 pending_change 执行 Approve / Reject
- 可对 evaluation 执行 Agree / Dismiss

---

### 阶段 6：生产部署与运维（第 10 周）

#### 步骤 6.1 容器化与编排

- [ ] 完善 `docker-compose.yml`：postgres + api (Rust binary) + nginx
- [ ] `Dockerfile.api` 多阶段构建：
  - Stage 1: `rust:1.80-slim` 编译 release binary
  - Stage 2: `debian:bookworm-slim` 运行，只复制 binary 和 CA 证书
- [ ] `Dockerfile.etl` 可复用 builder 阶段，ENTRYPOINT 不同
- [ ] nginx 反向代理，处理 HTTPS 和静态前端文件

#### 步骤 6.2 前端构建与集成

- [ ] `npm run build` 产出 `frontend/dist/`
- [ ] nginx 配置：`location /` Serve 静态文件；`location /api/` proxy_pass 到 Rust 服务

#### 步骤 6.3 监控与日志

- [ ] Rust 侧：`tracing` + `tracing-subscriber`（JSON 格式）
- [ ] 关键指标：请求延迟、ETL 状态、数据库连接池状态
- [ ] （可选）`/metrics` 端点暴露 Prometheus 格式指标（`metrics` crate）

#### 步骤 6.4 备份策略

- [ ] PostgreSQL 逻辑备份：`pg_dump` 每日定时执行
- [ ] `import_log` 和 `change_log` 与主库同周期备份

---

## 5. 关键编码约束汇总

### 5.1 数据库事务边界

| 场景                | 事务范围                                                   | 说明                                                                                |
| ------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| API 单次写入        | 单请求 = 单事务                                            | Axum handler 中 `pool.begin().await`，service 传入 `&mut Transaction<'_, Postgres>` |
| ETL 单条词          | 单词 = 单事务                                              | words + definitions + translations + examples + forms 原子写入                      |
| ETL 批量            | 每 1000 词 commit                                          | 防止单事务持有锁过久                                                                |
| pending_change 审批 | 目标表 UPDATE + change_log INSERT + pending_changes UPDATE | 必须原子                                                                            |
| evaluation agree    | 目标表 UPDATE + evaluations UPDATE                         | 必须原子                                                                            |

### 5.2 数据写入铁律

1. **ETL 永不直接修改 `curated=true` 的 words 行**。
2. **ETL 永不覆写 `reviewed=true OR modified=true` 的附属表行**；差异进入 `pending_changes`。
3. **人工新建记录**必须 `source='human'`, `reviewed=true`, `modified=false`。
4. **人工编辑记录**必须 `modified=true`, `reviewed=true`, `reviewed_at=NOW()`。
5. **任何写操作**必须同步 `INSERT change_log`。

### 5.3 API 规范

- 前缀：`/api/v1`
- 响应格式统一：
  ```json
  {
    "data": {},
    "meta": {"total": 100, "limit": 20, "offset": 0}
  }
  ```
- 错误格式统一：
  ```json
  {
    "error": {"code": "CONFLICT", "message": "Duplicate definition for this word and source"}
  }
  ```
- 分页：所有列表接口支持 `limit` (default 20, max 100) 和 `offset` (default 0)。

### 5.4 安全约束

- PostgreSQL 连接字符串通过环境变量注入，禁止 hardcode。
- API 暴露在互联网时必须 HTTPS + Bearer Token 鉴权。Token 通过 `Authorization: Bearer <token>` 传递，在 `api/src/middleware/auth.rs` 校验。
- `change_log.operator` 必须可追溯到具体人员（HTTP Header `X-Operator-ID`）。
- `pending_changes` 和 `evaluations` 审批接口需校验目标行是否存在（sqlx `query_as!` 返回 `Option<T>` 为 `None` 时返回 `404`）。

### 5.5 性能约束

- 所有 `WHERE word = $1` 查询命中 `idx_words_word`（CITEXT 索引）。
- sqlx 查询避免 N+1：聚合展示用手写 `JOIN`，列表展示用 `IN (...)` 批量查从表。
- ETL 批量插入优先使用 `COPY FROM` 或 `UNNEST` + 参数化数组。

---

## 6. Rust 特有注意事项

### 6.1 sqlx 与 PostgreSQL 特有类型

| PG 类型                        | Rust 类型                                     | 备注                                                          |
| ------------------------------ | --------------------------------------------- | ------------------------------------------------------------- |
| `CITEXT`                       | `String`                                      | 查询时利用 PG 的 `citext` 自动大小写不敏感                    |
| `data_source` (ENUM)           | `DataSource` (自定义 enum)                    | `#[derive(sqlx::Type)]`，`#[sqlx(type_name = "data_source")]` |
| `TEXT[]`                       | `Vec<String>`                                 | `query!` / `query_as!` 自动映射                               |
| `JSONB`                        | `serde_json::Value` 或 `sqlx::types::Json<T>` | 强类型 JSON 推荐 `Json<MyStruct>`                             |
| `TIMESTAMPTZ`                  | `chrono::DateTime<chrono::Utc>`               | 也可使用 `chrono::DateTime<chrono::FixedOffset>`              |
| `GENERATED ALWAYS AS IDENTITY` | `i64`                                         | `BIGINT` 对应 `i64`                                           |

### 6.2 错误处理分层

```
底层 (sqlx)              → sqlx::Error
  ↓
Service 层               → thiserror 定义的 DomainError (NotFound, Conflict, ValidationFailed)
  ↓
Router 层                → impl IntoResponse，映射为 HTTP Status Code + JSON
```

- `sqlx::Error::RowNotFound` → `404 NotFound`
- `sqlx::Error::Database(db_err)` 且 `db_err.constraint()` 命中唯一约束 → `409 Conflict`
- 其他 `sqlx::Error` → `500 INTERNAL_SERVER_ERROR`（日志记录详细错误）
- `DomainError::NotFound` → `404`
- `DomainError::Conflict` → `409`
- `DomainError::Validation` → `400`

### 6.3 异步与阻塞操作

- ETL 中的 SQLite 读取（`rusqlite`）是同步阻塞的，若放在 async runtime 中，需使用 `tokio::task::spawn_blocking`。
- LLM HTTP 调用（`reqwest`）是原生 async，可直接 await。

---

## 7. 附录

### 7.1 常用命令

```bash
# 启动开发数据库
docker compose -f docker/docker-compose.yml up -d postgres

# 运行数据库迁移
cargo sqlx migrate run --source migration/migrations

# 启动 API 服务
cargo run --bin dict-api

# 运行 ETL
cargo run --bin dict-etl -- stardict --mode full
cargo run --bin dict-etl -- wn --mode full
cargo run --bin dict-etl -- fill-gaps
cargo run --bin dict-etl -- evaluate

# 测试
cargo test --workspace

# 前端开发
cd frontend && npm run dev

# 前端构建
cd frontend && npm run build

# 格式化 + 检查
cargo fmt
cargo clippy --workspace --all-targets --all-features

# 编译期 SQL 检查缓存（用于 CI 无数据库编译）
cargo sqlx prepare --workspace
```

### 7.2 环境变量模板 (.env.example)

```bash
# Database
DATABASE_URL=postgres://dict:dict@localhost:5432/dict

# API
API_HOST=127.0.0.1
API_PORT=8000
API_BEARER_TOKEN=changeme_in_production

# LLM (DeepSeek API，兼容 OpenAI 格式)
LLM_PROVIDER=deepseek
LLM_BASE_URL=https://api.deepseek.com
LLM_API_KEY=sk-...
# fill_gaps 用非思考模式（快速、低成本）；当前对应 deepseek-v4-flash，以官方文档为准
LLM_MODEL_GENERATE=deepseek-v4-flash
# evaluate 用思考模式（批判性推理）；当前对应 deepseek-v4-pro，以官方文档为准
LLM_MODEL_EVALUATE=deepseek-v4-pro
LLM_RPM=60
LLM_MAX_RETRIES=3
LLM_TIMEOUT_SECONDS=60

# ETL
ETL_BATCH_SIZE=1000
ETL_SQLITE_STARDICT_PATH=./data/stardict.db
ETL_SQLITE_WN_PATH=./data/wn.db
```

### 7.3 与上游设计文档的对应关系

| 设计文档章节         | 实现位置                                                                                         |
| -------------------- | ------------------------------------------------------------------------------------------------ |
| 第 2 节 枚举类型     | `migration/migrations/*_init.sql` + `models/src/enums.rs`                                        |
| 第 3 节 触发器       | Migration raw SQL（`CREATE FUNCTION set_updated_at / set_curated_at / check_evaluation_target`） |
| 第 4 节 表结构       | `models/src/*.rs` + Migration                                                                    |
| 第 5 节 完整建表 SQL | Migration baseline                                                                               |
| 第 7 节 ETL 流程     | `etl/src/import_*.rs`                                                                            |
| 第 8 节 LLM 角色     | `etl/src/fill_gaps.rs`, `evaluate.rs`, `llm_client.rs`                                           |
| 第 9 节 Web Service  | `api/src/routers/*.rs`, `services/*.rs`                                                          |
| 第 10 节 查询示例    | `api/src/services/word_service.rs`                                                               |
| 前端 UI              | `frontend/src/pages/*.tsx`                                                                       |
