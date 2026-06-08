# AGENTS.md

本文档面向 AI 编程助手，帮助其快速理解本项目的结构、技术栈与开发约定。

## 项目概述

`wn-demo` 是一个基于 Python 的双语 WordNet 探索工具，支持：
- **Open English WordNet 2025+**（英文词库，标识符 `oewn:2025+`）
- **Chinese Open WordNet**（中文词库，标识符 `omw-cmn:1.4`）

项目核心功能是通过命令行交互式查询单词，返回其词性、英文释义、英文同义词以及对应的中文同义词。中英文之间的概念映射通过 **ILI（Inter-Lingual Index，跨语言索引）** 实现。

## 技术栈

| 组件 | 说明 |
|------|------|
| Python | >= 3.13（见 `.python-version`） |
| 包管理器 | [uv](https://docs.astral.sh/uv/) |
| 核心依赖 | `wn>=1.1.0`（WordNet 访问库） |
| 数据库（开发） | SQLite 3（`data/wn.db`，~133 MB，26 张表） |
| 数据库（生产） | PostgreSQL（通过 `PG_URL` 环境变量连接） |
| PG 驱动 | `psycopg2-binary` |
| 环境配置 | `python-dotenv`（读取 `.env` 文件） |

依赖声明在 `pyproject.toml` 中，锁文件为 `uv.lock`。

## 项目结构

```
.
├── main.py              # 入口：使用 wn 库的高级 API 实现交互查询
├── test.py              # 备用实现：直接对 SQLite 写 SQL 查询（非单元测试！）
├── data/
│   ├── wn.db            #   SQLite 本地数据库（首次运行自动下载）
│   ├── downloads/       #   wn 库下载缓存
│   └── README.md        #   数据来源说明
├── docs/
│   ├── wn.md            #   wn.db 数据库结构详解（含 ER 图、贯穿示例、核心查询）
│   └── test.md          #   SQL 查询示例
├── schema/
│   ├── wn.sqlite.sql    #   WN-LMF SQLite 建表语句
│   ├── wn.pg.sql        #   WN-LMF PostgreSQL 建表语句
│   └── stardict.sqlite.sql  # ECDICT 英中词典建表语句（预留）
├── sync_to_pg/
│   └── migrate.py       #   data/wn.db → PostgreSQL 迁移脚本
├── pyproject.toml       # Python 项目配置与依赖
└── .env.example         # 环境变量模板
```

## 运行方式

### 开发环境（SQLite）

```bash
# 使用 wn 库 API 查询
uv run python main.py

# 使用原生 SQL 查询（功能相同，实现不同）
uv run python test.py
```

首次运行 `main.py` 时，`wn.download("oewn:2025+")` 和 `wn.download("omw-cmn:1.4")` 会自动将词库数据下载到 `data/wn.db`（约 107 MB）。

### 生产环境（PostgreSQL）

1. 复制 `.env.example` 为 `.env`，填入 `PG_URL`：
   ```
   PG_URL=postgresql://user:password@localhost:5432/dict
   ```
2. 执行迁移：
   ```bash
   uv run python sync_to_pg/migrate.py
   ```
   该脚本会读取 `data/wn.db`，在 PostgreSQL 的 `wn` schema 中重建表结构、导入数据并创建索引。

## 代码组织

项目目前有两个并行的交互式查询实现，逻辑完全一致：

| 文件 | 实现方式 | 特点 |
|------|---------|------|
| `main.py` | `wn` 库高级 API | `wn.Wordnet("oewn:2025+")`、`word.synsets()`、`synset.definition()` 等 |
| `test.py` | 原生 `sqlite3` SQL | 直接 JOIN `forms` / `entries` / `senses` / `synsets` / `definitions` / `ilis` 等表 |

两者都实现了相同的函数签名风格：
- `lookup_bilingual(word, en, cmn)`（`main.py`）
- `lookup_word(conn, word)`（`test.py`）

查询逻辑：
1. 先查英文词库（`lexicon_rowid = 1`）
2. 若查到 synset，通过 `synset.ili` / `synsets.ili_rowid` 找到对应中文 synset
3. 若英文无结果，回退查中文词库（`lexicon_rowid = 2`），反向通过 ILI 找英文

## 数据库架构要点

`data/wn.db` 遵循 **WN-LMF**（WordNet Lexical Markup Framework）规范，包含 26 张表。核心实体关系：

- **lexicons**：词库元信息。当前 2 行：英文（rowid=1）、中文（rowid=2）
- **entries**：词条（如 `oewn-bank-n`）
- **forms**：词形（`form` 为表面文字，`rank=0` 为首选 lemma）
- **synsets**：同义词集（概念单元）
- **senses**：义项，连接 entry 与 synset（多对多）
- **definitions**：synset 的文字定义
- **ilis**：跨语言索引（独立于语言的唯一概念 ID）
- **synset_relations**：synset 间的语义关系（上下位、整体部分等，共 28 种）
- **sense_relations**：义项间的细粒度关系（派生、反义等）

### 中英文数据差异

| 数据项 | 英文 (oewn) | 中文 (omw-cmn) |
|--------|------------|---------------|
| entries | 161,875 | 63,347 |
| synsets | 120,564 | 42,312 |
| definitions | 有 | **无** |
| synset_examples | 有 | **无** |
| synset_relations | 有 | **无** |

中文 synset 不含定义和关系，全部通过 ILI 复用英文侧数据。

## 代码风格与约定

- **注释语言**：中文。函数 docstring、代码注释、文档均以中文为主。
- **代码风格**：简单直接的过程式风格，无类封装、无 Web 框架。
- **变量命名**：英文 snake_case，与 `wn` 库的 API 命名保持一致。
- **字符串**：单引号用于代码内字符串，双引号用于文档或输出格式化，无严格限制。

## 测试说明

⚠️ **本项目没有单元测试套件**。`test.py` 不是测试文件，而是 `main.py` 的 SQL 原生实现版本。

验证功能的方式是运行两个文件，输入相同单词，对比输出是否一致。

## 安全与配置

- **敏感信息**：数据库连接字符串 `PG_URL` 放在 `.env` 中，`.env` 已加入 `.gitignore`。
- **数据文件**：`data/*` 已加入 `.gitignore`，仅保留 `data/README.md`。
- **虚拟环境**：`.venv` 已加入 `.gitignore`。

## 修改注意事项

1. **修改 schema**：`schema/wn.sqlite.sql` 和 `schema/wn.pg.sql` 是权威建表语句。若修改表结构，需同步更新 `sync_to_pg/migrate.py` 中的类型映射和 `TABLES` 顺序。
2. **修改查询逻辑**：`main.py` 和 `test.py` 逻辑必须保持一致，否则说明实现有偏差。
3. **wn 库升级**：`wn>=1.1.0` 的 API 与 WN-LMF schema 紧密绑定，升级大版本前需验证 schema 兼容性。
4. **数据目录**：`wn.config.data_directory` 在 `main.py` 中被设为 `./data`，确保下载的数据落在正确位置。
