# stardict 词典数据库

`data/stardict.db` 是一个 SQLite 词典数据库，源于 StarDict（星际译王）词典数据。

## 表结构

### stardict

词典主表。

| 列 | 类型 | 约束 | 说明 |
|---|---|---|---|
| id | INTEGER | PK, AUTOINCREMENT, NOT NULL, UNIQUE | 自增主键，唯一标识 |
| word | VARCHAR(64) | NOT NULL, UNIQUE, COLLATE NOCASE | 单词/词条，大小写不敏感 |
| sw | VARCHAR(64) | NOT NULL, COLLATE NOCASE | 排序用单词（去除首字符干扰，用于字母排序），大小写不敏感 |
| phonetic | VARCHAR(64) | | 音标（IPA 音标） |
| definition | TEXT | | 英文释义 |
| translation | TEXT | | 中文翻译 |
| pos | VARCHAR(16) | | 词性（如 n. 名词, v. 动词, adj. 形容词） |
| collins | INTEGER | DEFAULT 0 | 柯林斯星级（0-5 星，5 星最高频） |
| oxford | INTEGER | DEFAULT 0 | 牛津 3000 核心词标记（1=是） |
| tag | VARCHAR(64) | | 标签（如 CET4, CET6, TOFEL, IELTS, GRE 等考试词汇分类） |
| bnc | INTEGER | | 英国国家语料库（BNC）词频排名，值越小越常用 |
| frq | INTEGER | | 当代美国英语语料库（COCA）词频排名，值越小越常用 |
| exchange | TEXT | | 词形变化（如过去式、过去分词、复数、比较级等） |
| detail | TEXT | | 详细释义/扩展信息（含例句等） |
| audio | TEXT | | 音频文件名或路径（用于播放发音） |

### 索引

| 索引名 | 列 | 说明 |
|---|---|---|
| sd_1 | word (COLLATE NOCASE) | 按单词快速查找 |
| stardict_1 | id | 按 id 查找 |
| stardict_2 | word | 单词唯一索引 |
| stardict_3 | sw, word (COLLATE NOCASE) | 按排序用词+单词复合查找 |
