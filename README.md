# wn-demo

Open English WordNet 2025+ 探索工具，基于 Python [`wn`](https://github.com/goodmami/wn) 库。

## 环境

- Python >= 3.13
- 包管理器: [uv](https://docs.astral.sh/uv/)

## 快速开始

```bash
uv run python main.py
```

首次运行会自动下载 Open English WordNet 2025+ 数据库到 `data/wn.db`（~107 MB）。

## 项目结构

```
.
├── main.py          # 入口
├── data/            # wn.db（SQLite，26 张表）
├── docs/
│   └── wn.md        # 数据库 schema 详解（含贯穿示例）
└── sync_to_pg/      # PG 同步工具（预留）
```

## 依赖

| 包 | 用途 |
|----|------|
| `wn` | WordNet 访问与查询 |
| `psycopg2-binary` | PostgreSQL 同步（预留） |

## 参考

- [WN-LMF 规范](https://globalwordnet.github.io/schemas/)
- [Open English WordNet](https://github.com/globalwordnet/english-wordnet)
