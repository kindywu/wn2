# wn-demo

Open English WordNet 2025+ 探索工具，基于 Python [`wn`](https://github.com/goodmami/wn) 库。支持英文 WordNet 和中文 WordNet。

## 环境

- Python >= 3.13
- 包管理器: [uv](https://docs.astral.sh/uv/)

## 快速开始

```bash
uv run python main.py
```

首次运行会自动下载 Open English WordNet 2025+ 数据库到 `data/wn.db`（~107 MB）。

中文 WordNet 数据 `data/omw-cmn.db` 通过 `wn.download("omw-cmn:1.4")` 获取。

## 项目结构

```
.
├── main.py            # 入口
├── data/              # 原始数据（只读来源）
│   ├── wn.db          #   Open English WordNet 2025+（SQLite，26 表）
│   ├── stardict.db    #   ECDICT 英中词典
│   └── downloads/     #   wn 库下载缓存
├── docs/
│   ├── wn.md          #   WN-LMF schema 详解（含贯穿示例）
│   └── pg_sql.md      #   PostgreSQL 查询示例
└── sync_to_pg/        # PG 同步工具
    └── migrate.py     #   data/wn.db → PostgreSQL 迁移脚本
```

PostgreSQL 为生产数据库，连接信息通过 `.env` 中的 `PG_URL` 配置。

## 依赖

| 包 | 用途 |
|----|------|
| `wn` | WordNet 访问与查询 |
| `psycopg2-binary` | PostgreSQL 同步（sync_to_pg/migrate.py） |

## 参考

- [WN-LMF 规范](https://globalwordnet.github.io/schemas/)
- [Open English WordNet](https://github.com/globalwordnet/english-wordnet)
