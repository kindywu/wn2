# dict.db 设计文档

## 概述

`dict.db` 是词典应用的自主规范数据源，整合三类来源：

| 来源 | 特质 | 角色 |
|------|------|------|
| 外部数据源 (stardict, wn) | 快速、免费、规模化 | 数据基底，批量导入，覆盖大部分常用词 |
| LLM | 灵活、按需、费钱 | 空缺填充，外部源覆盖不到的地方 |
| 人工 | 最可靠、不可规模化 | 质量闸门，审核/修正/新建 |

### 三层递进关系

```
外部数据源 (基底)     →  批量导入，confidence=1.0
      ↓ 空缺
LLM (填充层)         →  按需补全，confidence=0.5~0.7
      ↓ 未审核
人工 (质量闸门)       →  审核/修正/新建，confidence=1.0
```

**核心关系：**

- **外部源是基础，不依赖 LLM。** 外部源有数据就直接用，LLM 只在外部源都缺失时才介入。
- **LLM 是填充，不覆盖外部源。** LLM 生成的内容 `confidence < 1.0`，优先级低于外部源。查询排序时外部源在前，LLM 在后。
- **人工是闸门，覆盖一切。** 人工修改（`modified=1`）或人工创建（`source='human'`）的数据优先级最高。人工审核通过（`reviewed=1`）的数据即使源是 LLM 也视为可用。
- **外部源更新时，人工说了算。** 新版本外部源的数据变化，如果与人工修改冲突，进入 `pending_changes` 等待审批，而不是自动覆盖。

**查询优先级（ORDER BY）：**

```
人工创建/修改  >  外部源+已审核  >  外部源+未审核  >  LLM+已审核  >  LLM+未审核
```

**核心原则：**
- **所有数据标注来源和置信度**，查询时不关心数据来自哪个上游。
- **每个值都可追溯**——谁创建的、是否审核过、是否被人工修改过。

---

## 表结构

### words — 单词主表

一条记录 = 一个词形 + 可选词性的组合。词性为非必填，stardict 中有部分词条不标注词性。

```sql
CREATE TABLE words (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL,           -- 词形 (lemma)，如 "bank"
    pos TEXT,                     -- 词性: n/v/a/r/s，可为 NULL
    pos_source TEXT,              -- 'stardict' / 'wn' / 'human'
    phonetic TEXT,                -- 音标 (IPA)
    phonetic_source TEXT,         -- 'stardict' / 'wn' / 'human'
    audio TEXT,                   -- 音频文件路径 (来自 stardict)
    collins INTEGER DEFAULT 0,    -- 柯林斯星级 0-5
    oxford INTEGER DEFAULT 0,     -- 牛津核心词 0/1
    bnc INTEGER,                  -- BNC 词频排名，越小越常用
    frq INTEGER,                  -- COCA 词频排名，越小越常用
    tag TEXT,                     -- 考试分类标签，逗号分隔: "CET4,CET6,TOEFL"
    exchange TEXT,                -- JSON: {"pl":"banks","past":"banked","pp":"banked","ing":"banking","3rd":"banks","comp":"","super":""}
    detail TEXT,                  -- JSON: stardict detail 原始扩展数据
    sources TEXT,                 -- 贡献来源，逗号分隔: 'stardict,wn' / 'stardict' / 'wn' / 'human'
    curated INTEGER NOT NULL DEFAULT 0,  -- 人工是否审核过本条词条 (0/1)
    curated_at TEXT,              -- 审核时间
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),

    UNIQUE(word, pos)
);
CREATE INDEX idx_words_word ON words(word);
CREATE INDEX idx_words_pos ON words(pos);
CREATE INDEX idx_words_collins ON words(collins);
CREATE INDEX idx_words_bnc ON words(bnc);
CREATE INDEX idx_words_frq ON words(frq);
CREATE INDEX idx_words_curated ON words(curated);
```

**字段来源说明：**

| 字段 | 来源 | 说明 |
|------|------|------|
| word | 所有源共有 | 词形本身，作为主键的一部分 |
| pos | stardict / wn / human | stardict 部分有，wn 全覆盖，人工可修正 |
| phonetic | wn > stardict > human | wn IPA 更规范，人工可修正或填写 |
| audio | stardict > human | |
| collins | stardict > human | 人工可修正 |
| oxford | stardict > human | 人工可修正 |
| bnc | stardict > human | 人工可修正 |
| frq | stardict > human | 人工可修正 |
| tag | stardict > human | 人工可添加或修正 |
| exchange | stardict + wn > human | stardict 为主，wn forms 补充，人工可修正 |
| detail | stardict | 原始数据，不做人工修改 |

**curated 语义：**
- 0 = 未审核。各字段值来自自动 ETL 导入，人工未确认。
- 1 = 已审核。人工已逐字段检查并通过，或人工手动创建/编辑过本条记录。

curated 更新规则：
- ETL 新插入时 `curated=0`。
- 人工编辑 words 表任意字段时，自动将 `curated` 置为 1，`curated_at` 置为当前时间。
- 人工可通过 UI 将 `curated` 复位为 0（表示需要重新审核）。

### definitions — 英文定义

一对多，一个词可有多个来源的定义。每行独立追踪来源和审核状态。

```sql
CREATE TABLE definitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,   -- 人工审核通过 (0/1)
    reviewed_at TEXT,                      -- 审核时间
    modified INTEGER NOT NULL DEFAULT 0,   -- 人工修改过文本 (0/1)
    modified_at TEXT,                      -- 最后修改时间
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),

    UNIQUE(word_id, text)
);
CREATE INDEX idx_definitions_word ON definitions(word_id);
CREATE INDEX idx_definitions_source ON definitions(source);
CREATE INDEX idx_definitions_reviewed ON definitions(reviewed);
```

**审核状态机：**

```
ETL 插入                     人工审核              人工修改文本
source=wn/stardict/llm  →   reviewed=0       →   reviewed=1
confidence=0.5~1.0          modified=0            modified=1 (如果改了)
reviewed=0                  reviewed_at=NULL      reviewed_at=now
modified=0                                        modified_at=now
                            ↓ 人工点击"通过"
                            reviewed=1
                            reviewed_at=now
                            modified=0
```

**查询时按可信度排序：**

```sql
-- 人工数据最优先，其次已审核，再按 confidence 降序
ORDER BY
    CASE WHEN source='human' OR modified=1 THEN 0 ELSE 1 END,
    reviewed DESC,
    confidence DESC
```

**设计要点：**
- `modified=1` 表示人工修改过文本内容。此时 `source` 保持不变（保留原始来源记录），但 `confidence` 应置为 1.0。
- `reviewed=1` 且 `modified=0` 表示人工看过原文并确认无误。
- `source='human'` 表示人工从零创建的定义。

### translations — 中文翻译

与 definitions 相同的审核模型。

```sql
CREATE TABLE translations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    language TEXT NOT NULL DEFAULT 'zh',
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),

    UNIQUE(word_id, text, language)
);
CREATE INDEX idx_translations_word ON translations(word_id);
CREATE INDEX idx_translations_source ON translations(source);
CREATE INDEX idx_translations_reviewed ON translations(reviewed);
```

**设计要点：**
- stardict.translation 是中文翻译主来源，confidence=1.0。
- wn 无直接中文翻译，通过 ILI 映射 omw-cmn 中文 lemma，source='wn'，confidence=0.8（因为是词级映射而非翻译）。
- llm 仅在两源都无时填充，confidence=0.7。

### examples — 例句

例句文本和它的中文翻译有**独立**的来源和审核状态。

```sql
CREATE TABLE examples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,                       -- 英文例句文本
    translation TEXT,                         -- 例句的中文翻译
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,       -- 例句文本审核
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,       -- 例句文本被人工修改
    modified_at TEXT,
    -- 翻译的独立来源追踪
    trans_source TEXT CHECK(trans_source IN ('stardict', 'wn', 'llm', 'human')),
    trans_confidence REAL DEFAULT 1.0 CHECK(trans_confidence >= 0 AND trans_confidence <= 1),
    trans_reviewed INTEGER NOT NULL DEFAULT 0, -- 翻译审核
    trans_reviewed_at TEXT,
    trans_modified INTEGER NOT NULL DEFAULT 0, -- 翻译被人修改
    trans_modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),

    UNIQUE(word_id, text)
);
CREATE INDEX idx_examples_word ON examples(word_id);
CREATE INDEX idx_examples_source ON examples(source);
CREATE INDEX idx_examples_reviewed ON examples(reviewed);
```

**设计要点：**
- `source/confidence/reviewed/modified` 追踪英文例句文本。
- `trans_source/trans_confidence/trans_reviewed/trans_modified` 独立追踪中文翻译。
- 例句和翻译可来自不同源。例如：例句来自 wn（source='wn'），翻译来自 LLM（trans_source='llm', trans_confidence=0.7）。
- `translation IS NULL` 时，trans_* 字段无意义（忽略）。

### word_relations — 语义关系

```sql
CREATE TABLE word_relations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    related_word TEXT NOT NULL,   -- 关联词形 (lemma)，非 words.id
    relation_type TEXT NOT NULL,  -- synonym/antonym/hypernym/hyponym/derivation/similar/...
    source TEXT NOT NULL DEFAULT 'wn' CHECK(source IN ('wn', 'stardict', 'llm', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),

    UNIQUE(word_id, related_word, relation_type)
);
CREATE INDEX idx_relations_word ON word_relations(word_id);
CREATE INDEX idx_relations_related ON word_relations(related_word);
CREATE INDEX idx_relations_type ON word_relations(relation_type);
CREATE INDEX idx_relations_reviewed ON word_relations(reviewed);
```

**设计要点：**
- `related_word` 存词形文本而非 words.id，因为关联词可能不在本地词库中。
- wn synset_relations 导入时展开：synset A {bank, depository} —hypernym→ synset B {institution}，展开为 bank→institution、depository→institution 两条关系。
- 人工可添加关系（source='human'），或删除关系（物理删除行）。

### word_forms — 词形变化

```sql
CREATE TABLE word_forms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    form TEXT NOT NULL,
    form_type TEXT NOT NULL CHECK(form_type IN (
        'plural', 'past', 'past_participle', 'present_participle',
        'third_person', 'comparative', 'superlative'
    )),
    source TEXT NOT NULL DEFAULT 'stardict' CHECK(source IN ('stardict', 'wn', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),

    UNIQUE(word_id, form, form_type)
);
CREATE INDEX idx_wordforms_word ON word_forms(word_id);
```

### change_log — 变更审计

记录所有人工操作的审计日志。ETL 自动导入不记日志（避免膨胀）。

```sql
CREATE TABLE change_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,     -- 'words' / 'definitions' / 'translations' / 'examples' / 'word_relations' / 'word_forms'
    row_id INTEGER NOT NULL,      -- 该表中的行 ID
    field TEXT,                   -- 被修改的字段名；NULL 表示整行操作
    old_value TEXT,               -- 旧值；NULL 表示新建
    new_value TEXT,               -- 新值；NULL 表示删除
    action TEXT NOT NULL CHECK(action IN ('create', 'update', 'delete', 'review', 'unreview')),
    changed_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_changelog_table_row ON change_log(table_name, row_id);
CREATE INDEX idx_changelog_changed_at ON change_log(changed_at);
```

**action 含义：**
- `create` — 人工新建一行（source='human'）
- `update` — 人工修改了某个字段的值
- `delete` — 人工删除了一行
- `review` — 人工将 reviewed 置为 1
- `unreview` — 人工将 reviewed 置为 0

### pending_changes — 数据源更新冲突队列

当外部数据源更新后，如果某行已被人工修改过，新数据不直接覆写，而是生成一条待审批记录。

```sql
CREATE TABLE pending_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,     -- 目标表: 'words' / 'definitions' / 'translations' / 'examples' / 'word_relations' / 'word_forms'
    row_id INTEGER,               -- 目标表中已有行的 ID；NULL 表示源新增了整行待确认
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    field TEXT,                   -- 字段名 (仅 words 表使用，附属表为 NULL 表示整行)
    old_value TEXT,               -- dict.db 当前值
    new_value TEXT,               -- 新数据源建议值
    source TEXT NOT NULL,         -- 哪个数据源触发的: 'stardict' / 'wn'
    import_log_id INTEGER REFERENCES import_log(id),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'rejected')),
    resolved_at TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_pending_word ON pending_changes(word_id);
CREATE INDEX idx_pending_status ON pending_changes(status);
```

**与附属表的对应关系：**

| table_name | row_id | field | 含义 |
|------------|--------|-------|------|
| definitions | 123 | NULL | 定义行 id=123 的 text 被源更新，但人工曾修改过 |
| translations | 456 | NULL | 翻译行 id=456 的 text 被源更新，但人工曾修改过 |
| examples | 789 | NULL | 例句行 id=789 的 text 被源更新，但人工曾修改过 |
| words | 10 | collins | 词条 id=10 的 collins 从 3 变为 4 |
| words | 10 | phonetic | 词条 id=10 的 phonetic 从 /a/ 变为 /b/ |
| word_relations | 55 | NULL | 关系行 id=55 被源更新 |

### 完整建表 SQL

```sql
-- 单词主表
CREATE TABLE words (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL,
    pos TEXT,
    pos_source TEXT,
    phonetic TEXT,
    phonetic_source TEXT,
    audio TEXT,
    collins INTEGER DEFAULT 0,
    oxford INTEGER DEFAULT 0,
    bnc INTEGER,
    frq INTEGER,
    tag TEXT,
    exchange TEXT,
    detail TEXT,
    sources TEXT,
    curated INTEGER NOT NULL DEFAULT 0,
    curated_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(word, pos)
);
CREATE INDEX idx_words_word ON words(word);
CREATE INDEX idx_words_pos ON words(pos);
CREATE INDEX idx_words_collins ON words(collins);
CREATE INDEX idx_words_bnc ON words(bnc);
CREATE INDEX idx_words_frq ON words(frq);
CREATE INDEX idx_words_curated ON words(curated);

-- 英文定义
CREATE TABLE definitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(word_id, text)
);
CREATE INDEX idx_definitions_word ON definitions(word_id);
CREATE INDEX idx_definitions_source ON definitions(source);
CREATE INDEX idx_definitions_reviewed ON definitions(reviewed);

-- 中文翻译
CREATE TABLE translations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    language TEXT NOT NULL DEFAULT 'zh',
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(word_id, text, language)
);
CREATE INDEX idx_translations_word ON translations(word_id);
CREATE INDEX idx_translations_source ON translations(source);
CREATE INDEX idx_translations_reviewed ON translations(reviewed);

-- 例句
CREATE TABLE examples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    translation TEXT,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    trans_source TEXT CHECK(trans_source IN ('stardict', 'wn', 'llm', 'human')),
    trans_confidence REAL DEFAULT 1.0 CHECK(trans_confidence >= 0 AND trans_confidence <= 1),
    trans_reviewed INTEGER NOT NULL DEFAULT 0,
    trans_reviewed_at TEXT,
    trans_modified INTEGER NOT NULL DEFAULT 0,
    trans_modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(word_id, text)
);
CREATE INDEX idx_examples_word ON examples(word_id);
CREATE INDEX idx_examples_source ON examples(source);
CREATE INDEX idx_examples_reviewed ON examples(reviewed);

-- 语义关系
CREATE TABLE word_relations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    related_word TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'wn' CHECK(source IN ('wn', 'stardict', 'llm', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(word_id, related_word, relation_type)
);
CREATE INDEX idx_relations_word ON word_relations(word_id);
CREATE INDEX idx_relations_related ON word_relations(related_word);
CREATE INDEX idx_relations_type ON word_relations(relation_type);
CREATE INDEX idx_relations_reviewed ON word_relations(reviewed);

-- 词形变化
CREATE TABLE word_forms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    form TEXT NOT NULL,
    form_type TEXT NOT NULL CHECK(form_type IN (
        'plural', 'past', 'past_participle', 'present_participle',
        'third_person', 'comparative', 'superlative'
    )),
    source TEXT NOT NULL DEFAULT 'stardict' CHECK(source IN ('stardict', 'wn', 'human')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0 AND confidence <= 1),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(word_id, form, form_type)
);
CREATE INDEX idx_wordforms_word ON word_forms(word_id);

-- 变更审计
CREATE TABLE change_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    row_id INTEGER NOT NULL,
    field TEXT,
    old_value TEXT,
    new_value TEXT,
    action TEXT NOT NULL CHECK(action IN ('create', 'update', 'delete', 'review', 'unreview')),
    changed_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_changelog_table_row ON change_log(table_name, row_id);
CREATE INDEX idx_changelog_changed_at ON change_log(changed_at);

-- 导入运行记录
CREATE TABLE import_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn')),
    source_version TEXT,          -- 源数据版本标识 (如 stardict 的文件 hash, wn 的版本号)
    mode TEXT NOT NULL CHECK(mode IN ('full', 'incremental')),
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    new_words INTEGER DEFAULT 0,  -- 新增词条数
    updated_words INTEGER DEFAULT 0,
    new_definitions INTEGER DEFAULT 0,
    updated_definitions INTEGER DEFAULT 0,
    new_translations INTEGER DEFAULT 0,
    updated_translations INTEGER DEFAULT 0,
    new_examples INTEGER DEFAULT 0,
    updated_examples INTEGER DEFAULT 0,
    new_relations INTEGER DEFAULT 0,
    updated_relations INTEGER DEFAULT 0,
    new_forms INTEGER DEFAULT 0,
    updated_forms INTEGER DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed')),
    error TEXT                   -- 错误信息
);

-- 数据源更新冲突队列
CREATE TABLE pending_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    row_id INTEGER,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    field TEXT,
    old_value TEXT,
    new_value TEXT,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn')),
    import_log_id INTEGER REFERENCES import_log(id),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'rejected')),
    resolved_at TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_pending_word ON pending_changes(word_id);
CREATE INDEX idx_pending_status ON pending_changes(status);
```

---

## ETL 流程

### 总体顺序

```
1. import_stardict.py   → words + definitions + translations + examples + word_forms
2. import_wn.py         → 合并 words + definitions + translations + examples + word_forms + word_relations
3. fill_gaps.py         → LLM 补全 translations + examples.translation + definitions
```

第二步是"合并"而非"插入"——wn 中已存在于 words 的词形，更新补充字段，不新增行。wn 独有的词形（stardict 没有的），新增行。

### 字段合并优先级

| 字段 | 优先级 | 说明 |
|------|--------|------|
| pos | wn > stardict | wn pos 全覆盖且准确 |
| phonetic | wn > stardict | wn IPA 更规范 |
| audio | stardict only | |
| collins | stardict only | |
| oxford | stardict only | |
| bnc | stardict only | |
| frq | stardict only | |
| tag | stardict only | |
| exchange | stardict > wn | stardict 为主，wn forms 补充缺失 |
| detail | stardict only | |

合并时遵循规则：**已有值不被覆盖，除非新源优先级更高。** 已人工审核（curated=1）的 words 行，ETL 不再修改任何字段。

### import_stardict.py 逻辑

```
for each row in stardict:
    1. 解析 exchange JSON → 暂存
    2. 确定 pos:
       - stardict.pos 有值 → pos = stardict.pos, pos_source = 'stardict'
       - stardict.pos 为空 → pos = NULL, pos_source = NULL
    3. INSERT OR IGNORE INTO words (
         word, pos, pos_source,
         phonetic, phonetic_source='stardict',
         audio,
         collins, oxford, bnc, frq, tag,
         exchange, detail
       )
       ON CONFLICT(word, pos) DO NOTHING  -- ETL 阶段不覆盖已有数据
    4. SELECT id FROM words WHERE word=? AND (pos=? OR pos IS NULL) → word_id
    5. definition 非空:
       INSERT OR IGNORE INTO definitions (word_id, text, source='stardict', reviewed=0, modified=0)
    6. translation 非空:
       INSERT OR IGNORE INTO translations (word_id, text, source='stardict', reviewed=0, modified=0)
    7. 解析 detail JSON 中的例句:
       - 如有例句文本:
         INSERT OR IGNORE INTO examples (word_id, text, translation=例句中文翻译或NULL,
           source='stardict',
           trans_source='stardict' IF translation NOT NULL ELSE NULL)
    8. 展开 exchange JSON:
       for each key in exchange:
         INSERT OR IGNORE INTO word_forms (word_id, form=value, form_type=mapped, source='stardict')
```

### import_wn.py 逻辑

```
# 阶段 1: 导入词形
for each lexicon in wn (oewn, omw-cmn):
    for each entry e in lexicon.entries:
        for each form f in e.forms where f.rank = 0 (只取 lemma):
            word = f.form
            pos = e.pos

            existing = SELECT id, curated, pos, pos_source, phonetic
                       FROM words WHERE word = ? AND (pos = ? OR pos IS NULL)

            if existing:
                # 不覆盖已人工审核的行
                if existing.curated: skip to next

                # 补全 pos (如果原为空)
                if existing.pos IS NULL:
                    UPDATE words SET pos=?, pos_source='wn' WHERE id=?

                # 补全 phonetic (如果原为空，且 wn 有发音)
                if existing.phonetic IS NULL:
                    pronunciation = lookup pronunciations for this form
                    if pronunciation:
                        UPDATE words SET phonetic=?, phonetic_source='wn' WHERE id=?
            else:
                pronunciation = lookup pronunciations for this form
                INSERT INTO words (word, pos, pos_source='wn',
                    phonetic=pronunciation.value, phonetic_source='wn')
            word_id = resolved

# 阶段 2: 导入定义和例句
for each synset s in oewn:
    for each word w in s.words:
        word_id = resolve words row for w
        if s.definition:
            INSERT OR IGNORE INTO definitions (word_id, text=s.definition, source='wn')
        for each example in s.examples:
            INSERT OR IGNORE INTO examples (word_id, text=example, source='wn')

# 阶段 3: 导入中文翻译 (通过 ILI 跨语言映射)
for each synset s_cmn in omw-cmn:
    ili_id = s_cmn.ili_rowid
    s_en = synset where ili_rowid = ili_id and lexicon = oewn
    if s_en:
        chinese_words = s_cmn.words (中文 lemma 列表)
        for each word w in s_en.words:
            word_id = resolve words row for w
            for each cw in chinese_words:
                INSERT OR IGNORE INTO translations (word_id, text=cw, source='wn', confidence=0.8)

# 阶段 4: 导入语义关系
for each synset_relation sr:
    type = relation_types[sr.type_rowid].type
    source_words = all lemmas in source synset
    target_words = all lemmas in target synset
    for each sw in source_words:
        for each tw in target_words:
            INSERT OR IGNORE INTO word_relations (word_id, related_word=tw, relation_type=type, source='wn')

for each sense_relation sr:
    type = relation_types[sr.type_rowid].type
    source_word = lemma of source sense's entry
    target_word = lemma of target sense's entry
    INSERT OR IGNORE INTO word_relations (word_id, related_word=target_word, relation_type=type, source='wn')

# 阶段 5: 补充词形变化
for each form f where f.rank > 0:
    lemma = f.entry's rank-0 form
    word_id = resolve words row for lemma
    form_type = infer from form text and entry.pos
    INSERT OR IGNORE INTO word_forms (word_id, form=f.form, form_type, source='wn')
```

### fill_gaps.py 逻辑 (LLM 补全)

```
BATCH_SIZE = 50

for each word_id in words:
    # 1. 补翻译
    existing_tr = SELECT COUNT(*) FROM translations WHERE word_id = ?
    if existing_tr == 0:
        enqueue_llm_task(
            type='translate',
            word_id=word_id,
            prompt="Translate '{word}' into Chinese. Return only the Chinese translation."
        )

    # 2. 补例句中文翻译
    for each example in SELECT * FROM examples WHERE word_id = ? AND translation IS NULL:
        enqueue_llm_task(
            type='example_translation',
            example_id=example.id,
            prompt="Translate to Chinese: '{example.text}'"
        )

    # 3. 补定义 (仅当完全无定义时)
    existing_def = SELECT COUNT(*) FROM definitions WHERE word_id = ?
    if existing_def == 0:
        enqueue_llm_task(
            type='define',
            word_id=word_id,
            prompt="Define '{word}' in English. One concise sentence."
        )

# 批量执行 LLM 任务
for batch in chunks(tasks, BATCH_SIZE):
    responses = call_llm_batch(batch)
    for resp in responses:
        if resp.type == 'translate':
            INSERT OR IGNORE INTO translations (word_id, text, source='llm', confidence=0.7)
        elif resp.type == 'example_translation':
            UPDATE examples SET
                translation = resp.text,
                trans_source = 'llm',
                trans_confidence = 0.7
            WHERE id = resp.example_id
        elif resp.type == 'define':
            INSERT OR IGNORE INTO definitions (word_id, text, source='llm', confidence=0.5)
```

**LLM 调用策略：**
- 批量执行，每批 50 条，减少 API 往返。
- `confidence`：翻译 0.7，例句翻译 0.7，定义 0.5（定义质量不如翻译可靠）。
- `reviewed=0, modified=0` 插入，等待人工审核。
- 同一词条的重复任务自动去重（已有数据后不再补全）。
- fill_gaps 幂等：可重复运行，已有数据的行自动跳过。
- 前端展示：`reviewed=0 AND source='llm'` 的内容标注 "(AI 未审核)"；`reviewed=1 AND source='llm'` 标注 "(AI 已审核)"。

### ETL 幂等性保证

所有 ETL 脚本可重复运行，不会产生重复数据：
- words: `UNIQUE(word, pos)` + `INSERT OR IGNORE`
- definitions/translations/examples: `UNIQUE(word_id, text)` + `INSERT OR IGNORE`
- word_relations: `UNIQUE(word_id, related_word, relation_type)` + `INSERT OR IGNORE`
- word_forms: `UNIQUE(word_id, form, form_type)` + `INSERT OR IGNORE`
- ETL 不修改 `curated=1` 的 words 行
- ETL 不修改 `reviewed=1` 或 `modified=1` 的附属表行

---

## 数据源更新与冲突处理

### 核心规则

| 行状态 | 行为 |
|--------|------|
| 未人工修改 (`modified=0`, `reviewed=0`, `curated=0`) | **直接覆写** |
| 已人工修改 (`modified=1` 或 `reviewed=1` 或 `curated=1`) | **生成 pending_change，等待人工审批** |
| 源中新增的数据 | **直接插入** |

### 场景

外部数据源（stardict.db、wn.db）可能以两种方式更新：

| 场景 | 触发条件 | 策略 |
|------|---------|------|
| 全量替换 | 源 DB 文件被新版替换（如 stardict 新版发布） | 全量重导 |
| 增量追加 | 源 DB 在原基础上新增词条（如 wn 版本升级新增 synset） | 增量导入 |

两种模式通过导入脚本的命令行参数控制：

```
python import_stardict.py --mode full
python import_stardict.py --mode incremental
python import_wn.py --mode full --lexicon oewn:2026
python import_wn.py --mode incremental --lexicon oewn:2026
```

### 版本追踪

每次导入在 `import_log` 中记录一条运行记录。

stardict 的版本标识：
- 计算文件的 SHA256 作为 `source_version`
- SHA256 与上次导入相同 → 跳过
- SHA256 不同 → 执行导入

wn 的版本标识：
- wn.db 中 `lexicons` 表有 `version` 字段（如 `2025+`）
- 导入时指定 `--lexicon oewn:2026`，版本号写入 `source_version`

### 附属表冲突检测与处理

附属表（definitions/translations/examples/word_relations/word_forms）的冲突检测粒度是**行级**——源中同一 (word_id, source) 对应的文本发生了变化。

```
for each source_row in source:
    # 1. 精确匹配：同 word_id + 同 source + 同 text → 无变化，跳过
    exact_match = SELECT * FROM {table}
                  WHERE word_id=? AND source=? AND text=?
    if exact_match:
        continue

    # 2. 同源匹配：同 word_id + 同 source，但 text 不同 → 源数据变了
    same_source = SELECT * FROM {table}
                  WHERE word_id=? AND source=?

    if same_source:
        if same_source.modified == 0 AND same_source.reviewed == 0:
            # 未人工改动 → 直接覆写
            UPDATE {table} SET text=?, updated_at=now() WHERE id=same_source.id
            stats.updated += 1
        else:
            # 已人工改动 → 生成冲突，不覆写
            INSERT OR IGNORE INTO pending_changes (
                table_name, row_id, word_id, field=NULL,
                old_value=same_source.text, new_value=source_row.text,
                source=source_name, import_log_id=current_import_id
            )
            stats.conflicts += 1

    else:
        # 3. 无同源匹配 → 全新数据，直接插入
        # 但需检查是否有其他源的同文本行（去重）
        INSERT OR IGNORE INTO {table} (word_id, text, source, ...)
        stats.new += 1
```

**设计要点：**
- 冲突不产生新的数据行，而是写入 `pending_changes` 队列。
- 查询时只查数据表，不 join pending_changes。待审批的修改完全隔离，不影响正常查询。
- 同一条源行多次导入不会生成重复的 pending_change（`INSERT OR IGNORE` + 唯一约束）。

### words 表冲突检测与处理

words 表的冲突检测粒度是**字段级**——curated=1 时，逐字段对比。

```
for each source_row in source:
    existing = SELECT * FROM words WHERE word=? AND (pos=? OR ...)

    if existing:
        if sources does not contain source_name:
            UPDATE words SET sources = sources + ',' + source_name WHERE id=existing.id

        if existing.curated == 0:
            # 未人工审核 → 逐字段覆写（按优先级）
            for each field in [pos, phonetic, collins, oxford, bnc, frq, tag, exchange]:
                if source_has_field AND current_source_priority >= field_source_priority:
                    UPDATE words SET {field}=? WHERE id=existing.id
            stats.updated += 1
        else:
            # 已人工审核 → 逐字段对比，差异生成 pending_change
            for each field in [pos, phonetic, collins, oxford, bnc, frq, tag, exchange]:
                old_val = getattr(existing, field)
                new_val = getattr(source_row, field)
                if old_val != new_val and new_val is not None:
                    INSERT OR IGNORE INTO pending_changes (
                        table_name='words', row_id=existing.id, word_id=existing.id,
                        field=field, old_value=old_val, new_value=new_val,
                        source=source_name, import_log_id=current_import_id
                    )
                    stats.conflicts += 1
    else:
        INSERT INTO words (word, pos, ..., sources=source_name)
        stats.new += 1
```

### 源中删除的词条处理

外部源删除数据时，dict.db **不做级联删除**。只更新 `words.sources` 字段（移除该源名）。

```
# 全量导入完成后，检查本次源中缺失的词
words_from_this_source = SELECT word FROM words WHERE sources LIKE '%{source_name}%'
words_in_current_source = {all words from source}
deleted_words = words_from_this_source - words_in_current_source

for word in deleted_words:
    UPDATE words SET sources = REPLACE(sources, '{source_name}', '') WHERE word = ?
    # 清理 sources 中的空逗号
    UPDATE words SET sources = REPLACE(sources, ',,', ',') WHERE word = ?
    # 清理首尾逗号
```

词条本身保留，附属数据保留。

### 冲突审批流程

人工在 UI 中查看 `pending_changes`，逐条决定：

```
审批通过 (status='approved'):
    if table_name == 'words':
        UPDATE words SET {field} = new_value, updated_at = now()
        WHERE id = row_id
        INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action='update')
    else:
        UPDATE {table_name} SET text = new_value,
               modified = 0, reviewed = 1, reviewed_at = now(), updated_at = now()
        WHERE id = row_id
        INSERT INTO change_log (table_name, row_id, field='text', old_value, new_value, action='update')
    UPDATE pending_changes SET status = 'approved', resolved_at = now() WHERE id = ?

审批通过但人工调整 (先改值再通过):
    # 人工将 new_value 改为自定义值 Z
    UPDATE pending_changes SET new_value = 'Z' WHERE id = ?
    # 然后走上述审批通过流程

审批拒绝 (status='rejected'):
    # 不修改数据表，仅标记 pending_change 状态
    UPDATE pending_changes SET status = 'rejected', resolved_at = now() WHERE id = ?
```

**审批通过的语义：** 接受新数据源的值，替代人工修改过的旧值。`modified` 复位为 0（因为现在是源数据），`reviewed` 置为 1（因为人工确认过这个新版本）。

**审批拒绝的语义：** 保留人工修改的版本，丢弃新数据源的建议。

### 增量导入

增量导入仅处理源中**新增**的数据，已存在的数据不触发冲突检测。

```
def import_stardict(mode='full'):
    current_hash = sha256('data/stardict.db')
    last = get_last_import('stardict')

    if last and last.source_version == current_hash:
        print("Source unchanged, skipping.")
        return

    if mode == 'incremental' and last:
        existing_words = SELECT word FROM words WHERE sources LIKE '%stardict%'
        new_rows = [r for r in source_rows if r.word not in existing_words]
        process(new_rows, detect_conflicts=False)  # 只插入，不检测冲突
    else:
        process(source_rows, detect_conflicts=True)  # 全量检测

    record_import_log('stardict', current_hash, mode, stats)
```

**增量模式下不检测冲突**的原因：增量意味着源只追加了新词条，不会修改已有词条的数据。如果源确实修改了已有数据，应该用全量模式。

### 导入流水线

```
1. import_stardict.py   ─┐
2. import_wn.py         ─┤ 全量或增量
3. fill_gaps.py         ─┘ 补全新产生的空缺
4. 人工审批 pending_changes
```

导入操作幂等：重复运行不会产生重复数据或重复冲突。

### 导入失败处理

全量重导如果中途失败，`import_log` 中留下 `status='failed'` 的记录，但已写入的数据和 pending_changes 不回滚。修复问题后重新运行即可，幂等机制保证不会重复。

---

## 查询示例

### 查单词完整信息（含审核状态）

```sql
SELECT w.word, w.pos, w.phonetic, w.phonetic_source,
       w.collins, w.bnc, w.frq, w.tag,
       w.curated, w.curated_at,
       d.text AS definition, d.source AS def_source, d.confidence AS def_conf,
       d.reviewed AS def_reviewed, d.modified AS def_modified,
       t.text AS translation, t.source AS tr_source, t.confidence AS tr_conf,
       t.reviewed AS tr_reviewed, t.modified AS tr_modified
FROM words w
LEFT JOIN definitions d ON d.word_id = w.id
LEFT JOIN translations t ON t.word_id = w.id
WHERE w.word = 'bank'
ORDER BY
    CASE WHEN d.source='human' OR d.modified=1 THEN 0 ELSE 1 END,
    d.reviewed DESC, d.confidence DESC,
    CASE WHEN t.source='human' OR t.modified=1 THEN 0 ELSE 1 END,
    t.reviewed DESC, t.confidence DESC;
```

### 查待审核项

```sql
-- 待审核的定义
SELECT w.word, d.text, d.source, d.confidence
FROM definitions d
JOIN words w ON w.id = d.word_id
WHERE d.reviewed = 0
ORDER BY d.source, d.confidence DESC;

-- 待审核的翻译
SELECT w.word, t.text, t.source, t.confidence
FROM translations t
JOIN words w ON w.id = t.word_id
WHERE t.reviewed = 0
ORDER BY t.source, t.confidence DESC;

-- 待审核的 LLM 例句翻译
SELECT w.word, e.text, e.translation
FROM examples e
JOIN words w ON w.id = e.word_id
WHERE e.trans_source = 'llm' AND e.trans_reviewed = 0;
```

### 查同义词（优先人工/已审核）

```sql
SELECT wr.related_word, wr.relation_type, wr.source,
       wr.reviewed, wr.modified
FROM word_relations wr
JOIN words w ON w.id = wr.word_id
WHERE w.word = 'happy' AND wr.relation_type = 'synonym'
ORDER BY
    CASE WHEN wr.source='human' OR wr.modified=1 THEN 0 ELSE 1 END,
    wr.reviewed DESC;
```

### 过滤低质量数据

```sql
-- 只看人类确认过的数据
SELECT * FROM definitions WHERE word_id = ?
  AND (source = 'human' OR reviewed = 1);

-- 排除 AI 未审核
SELECT * FROM translations WHERE word_id = ?
  AND NOT (source = 'llm' AND reviewed = 0);
```

### 查待审批冲突

```sql
-- 按来源和词分组查看所有待审批冲突
SELECT w.word, pc.table_name, pc.field,
       pc.old_value, pc.new_value, pc.source, pc.created_at
FROM pending_changes pc
JOIN words w ON w.id = pc.word_id
WHERE pc.status = 'pending'
ORDER BY pc.source, pc.created_at;
```

### 审核操作 SQL

```sql
-- 审核通过一笔翻译
UPDATE translations SET reviewed = 1, reviewed_at = datetime('now'), updated_at = datetime('now')
WHERE id = ?;
INSERT INTO change_log (table_name, row_id, action) VALUES ('translations', ?, 'review');

-- 人工修改定义文本
UPDATE definitions SET text = ?, modified = 1, modified_at = datetime('now'),
       reviewed = 1, reviewed_at = datetime('now'), confidence = 1.0, updated_at = datetime('now')
WHERE id = ?;
INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action)
VALUES ('definitions', ?, 'text', old_text, new_text, 'update');

-- 人工新建翻译
INSERT INTO translations (word_id, text, source, confidence, reviewed, modified)
VALUES (?, ?, 'human', 1.0, 1, 0);
INSERT INTO change_log (table_name, row_id, action) VALUES ('translations', last_insert_rowid(), 'create');

-- 审批通过数据源更新冲突 (附属表)
UPDATE definitions SET text = (SELECT new_value FROM pending_changes WHERE id = ?),
       modified = 0, reviewed = 1, reviewed_at = datetime('now'), updated_at = datetime('now')
WHERE id = (SELECT row_id FROM pending_changes WHERE id = ?);
UPDATE pending_changes SET status = 'approved', resolved_at = datetime('now') WHERE id = ?;

-- 审批通过数据源更新冲突 (words 表字段)
UPDATE words SET collins = (SELECT new_value FROM pending_changes WHERE id = ?),
       updated_at = datetime('now')
WHERE id = (SELECT row_id FROM pending_changes WHERE id = ?);
UPDATE pending_changes SET status = 'approved', resolved_at = datetime('now') WHERE id = ?;
INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action)
SELECT 'words', row_id, field, old_value, new_value, 'update'
FROM pending_changes WHERE id = ?;

-- 审批拒绝冲突
UPDATE pending_changes SET status = 'rejected', resolved_at = datetime('now') WHERE id = ?;
```

---

## 数据统计估算

| 表 | 预估行数 | 来源 |
|----|---------|------|
| words | ~200,000 | stardict 全量 + wn 补充 |
| definitions | ~250,000 | wn ~120k + stardict ~130k (去重后) + LLM |
| translations | ~180,000 | stardict 全量 + wn ILI 中文映射 + LLM |
| examples | ~80,000 | wn ~50k + stardict detail 解析 + LLM 翻译 |
| word_relations | ~400,000 | wn synset_relations 展开 + sense_relations |
| word_forms | ~100,000 | stardict exchange 展开 + wn forms 补充 |
| change_log | ~0 起步 | 仅人工操作时写入 |
| import_log | ~10/年 | 每次导入一条记录 |
| pending_changes | ~0 起步 | 仅在源更新且有人工修改时产生 |
