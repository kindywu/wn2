# 原始数据

本目录存放从外部下载的原始数据文件（只读来源），不作为生产数据库使用。生产环境使用 PostgreSQL。

## wn.db
Open English WordNet 2025+ SQLite 数据库，WN-LMF schema（26 张表）。

来源:
- https://en-word.net/static/english-wordnet-2025.xml.gz
- https://github.com/omwn/omw-data/releases/download/v1.4/omw-1.4.tar.xz

```python
wn.download("oewn:2025+")
wn.download("omw-cmn:1.4")
wn.Wordnet("oewn:2025+")
```

## stardict.db
ECDICT 英中词典（SQLite）。

来源: https://github.com/skywind3000/ECDICT