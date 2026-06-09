# dict.db 设计文档

## 1. 概述

### 1.1 目标

`dict.db` 是词典应用的**自主规范数据源**。外部数据仅作为上游导入管道，应用层只读写这一个数据库，不关心数据最初来自哪里。

### 1.2 数据来源与关系

三类来源，分层递进：

| 来源 | 特质 | 角色 |
|------|------|------|
| 外部数据源 (stardict.db, wn.db) | 快速、免费、可规模化 | 数据基底，批量导入覆盖大部分常用词 |
| LLM | 灵活、按需、费钱 | 空缺填充 + 质量评价 |
| 人工 | 最可靠、不可规模化 | 质量闸门，审核、修正、新建 |

```
外部数据源 (基底)  ──→  批量导入全量数据
      ↓ 空缺
LLM (填充层)      ──→  按需补全缺失字段，不覆盖外部源已有数据
      ↓ 未审核
人工 (质量闸门)    ──→  审核、修正、新建，覆盖一切
```

**核心关系规则：**

1. 外部源是基础，LLM 不覆盖外部源已有数据。LLM 只在两外部源都缺失某字段时才介入填充。
2. 人工是闸门，覆盖一切。人工修改过的数据，外部源更新时进入审批队列而非自动覆写。
3. LLM 的第二角色是质量评价。对已有数据打分、标记问题，辅助人工审核优先级排序。

### 1.3 数据质量判定

不用浮点数置信度（0.0~1.0 无法操作）。用 `source` + `reviewed` + `modified` 三个离散字段判定：

| 条件 | 质量等级 | 含义 |
|------|---------|------|
| `source='human'` 或 `modified=1` | 最高 | 人工创建或修改过 |
| `source IN ('stardict','wn')` 且 `reviewed=1` | 高 | 外部源数据，人工已确认 |
| `source IN ('stardict','wn')` 且 `reviewed=0` | 中 | 外部源数据，未审核 |
| `source='llm'` 且 `reviewed=1` | 中 | AI 生成，人工已确认 |
| `source='llm'` 且 `reviewed=0` | 低 | AI 生成，未审核 |

查询排序 ORDER BY 规则：

```sql
ORDER BY
    CASE WHEN source='human' OR modified=1 THEN 0
         WHEN source IN ('stardict','wn') AND reviewed=1 THEN 1
         WHEN source IN ('stardict','wn') AND reviewed=0 THEN 2
         WHEN source='llm' AND reviewed=1 THEN 3
         ELSE 4
    END
```

---

## 2. 表结构

### 2.1 表总览

| 表 | 用途 | 预估行数 |
|----|------|---------|
| words | 单词主表，一个 (word, pos) 一行 | ~200,000 |
| definitions | 英文定义，一对多 | ~250,000 |
| translations | 中文翻译，一对多 | ~180,000 |
| examples | 例句，文本和翻译独立追踪 | ~80,000 |
| word_relations | 语义关系（同义/反义/上下位/派生） | ~400,000 |
| word_forms | 词形变化（复数/时态/比较级） | ~100,000 |
| evaluations | LLM 质量评价记录 | ~变动 |
| pending_changes | 外部源更新产生的冲突审批队列 | ~0 起步 |
| change_log | 人工操作审计日志 | ~0 起步 |
| import_log | 外部源导入运行记录 | ~10/年 |

### 2.2 words — 单词主表

一条记录 = 一个词形 + 可选词性。词性非必填（stardict 部分词条不标词性）。

```sql
CREATE TABLE words (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL,           -- 词形 (lemma)，如 "bank"
    pos TEXT,                     -- 词性: n/v/a/r/s，可为 NULL
    pos_source TEXT,              -- 该词性来自哪个源: 'stardict' / 'wn' / 'human'
    phonetic TEXT,                -- 音标 (IPA)
    phonetic_source TEXT,         -- 音标来源: 'stardict' / 'wn' / 'human'
    audio TEXT,                   -- 音频文件路径
    collins INTEGER DEFAULT 0,    -- 柯林斯星级 0-5
    oxford INTEGER DEFAULT 0,     -- 牛津核心词 0/1
    bnc INTEGER,                  -- BNC 词频排名，越小越常用
    frq INTEGER,                  -- COCA 词频排名，越小越常用
    tag TEXT,                     -- 考试分类标签，逗号分隔: "CET4,CET6,TOEFL,IELTS,GRE"
    exchange TEXT,                -- JSON: {"pl":"banks","past":"banked","pp":"banked","ing":"banking","3rd":"banks","comp":"","super":""}
    detail TEXT,                  -- JSON: stardict detail 原始扩展数据（保留备用）
    sources TEXT,                 -- 贡献来源，逗号分隔: 'stardict,wn' / 'stardict' / 'wn' / 'human'
    curated INTEGER NOT NULL DEFAULT 0,  -- 人工是否审核过本条 (0/1)
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
```

**字段来源说明：**

| 字段 | 来源 | 说明 |
|------|------|------|
| word | 所有源共有 | 词形本身 |
| pos | stardict(部分) / wn(全) / human | wn 的 pos 更完整 |
| phonetic | wn > stardict > human | wn IPA 更规范 |
| audio | stardict | wn 理论上也有但极少填充 |
| collins | stardict | 柯林斯词典独有 |
| oxford | stardict | 牛津词典独有 |
| bnc | stardict | 英国国家语料库词频 |
| frq | stardict | 美国当代英语语料库词频 |
| tag | stardict | 考试分类标签 |
| exchange | stardict + wn 补充 | stardict 为主，wn forms 表补充 |
| detail | stardict | 原始扩展 JSON，不展开 |
| sources | ETL 自动维护 | 记录哪些上游提供了此词条 |

**curated 语义：**
- 0 = 未审核，字段值来自 ETL 自动导入。
- 1 = 已审核，人工已逐字段检查并确认。人工编辑任意字段时自动置 1。

### 2.3 definitions — 英文定义

每个词可有多条定义（不同来源的措辞可能不同）。

```sql
CREATE TABLE definitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
    reviewed INTEGER NOT NULL DEFAULT 0,   -- 人工审核通过 (0/1)
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,   -- 人工修改过文本 (0/1)
    modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),

    UNIQUE(word_id, text)
);
CREATE INDEX idx_definitions_word ON definitions(word_id);
CREATE INDEX idx_definitions_source ON definitions(source);
CREATE INDEX idx_definitions_reviewed ON definitions(reviewed);
```

**来源说明：**

| source | 导入来源 | 说明 |
|--------|---------|------|
| stardict | stardict.definition 字段 | 英文定义，通常简洁 |
| wn | wn synset definition | 英文定义，通常较完整 |
| llm | fill_gaps.py | 仅在 stardict 和 wn 都没有时补全 |
| human | 人工新建 | 人工输入的定义 |

**modified 语义：**
- 0 = 文本未经人工修改，保持源导入时的原样
- 1 = 人工编辑过文本内容。`source` 保持不变（记录最初来源），但查询优先级提升到最高

**审核状态迁移：**

```
ETL 插入 (reviewed=0, modified=0)
    ├── 人工点击"通过" → reviewed=1, reviewed_at=now, modified=0
    ├── 人工修改文本   → reviewed=1, reviewed_at=now, modified=1, modified_at=now
    └── 人工撤回审核   → reviewed=0, reviewed_at=NULL
```

### 2.4 translations — 中文翻译

与 definitions 结构一致。

```sql
CREATE TABLE translations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    language TEXT NOT NULL DEFAULT 'zh',
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
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

**来源说明：**

| source | 导入来源 | 说明 |
|--------|---------|------|
| stardict | stardict.translation 字段 | 中文翻译主来源 |
| wn | omw-cmn ILI 映射 | 通过跨语言索引找到的中文 lemma，非直接翻译 |
| llm | fill_gaps.py | 仅在 stardict 和 wn 都没有时补全 |
| human | 人工新建 | 人工输入的翻译 |

### 2.5 examples — 例句

例句的英文文本和中文翻译有**独立**的审核追踪。

```sql
CREATE TABLE examples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,                       -- 英文例句文本
    translation TEXT,                         -- 例句的中文翻译（可为 NULL）
    -- 例句文本的追踪
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    -- 翻译的独立追踪
    trans_source TEXT CHECK(trans_source IN ('stardict', 'wn', 'llm', 'human')),
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
```

**独立追踪的含义：**

同一例句行，英文文本和中文翻译可能来自不同源、分别审核：

```
例句行: word_id=1
  text="He cashed a check at the bank"    source='wn',     reviewed=1  (人工已确认)
  translation="他在银行兑现了支票"          trans_source='llm', trans_reviewed=0  (AI翻译，未审核)
```

**来源说明：**

| source | 导入来源 |
|--------|---------|
| stardict | stardict detail JSON 中解析的例句 |
| wn | wn synset_examples 表 |
| llm | fill_gaps.py 生成的例句 |
| human | 人工新建的例句 |

### 2.6 word_relations — 语义关系

统一存储所有词间语义关系，不区分 synset 级和 sense 级。

```sql
CREATE TABLE word_relations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    related_word TEXT NOT NULL,   -- 关联词形 (lemma)，非 words.id
    relation_type TEXT NOT NULL,  -- 关系类型
    source TEXT NOT NULL DEFAULT 'wn' CHECK(source IN ('wn', 'stardict', 'llm', 'human')),
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

**关系类型（relation_type）完整列表：**

| 类别 | 类型 | 含义 |
|------|------|------|
| 同义/反义 | synonym | 同义词 |
| | antonym | 反义词 |
| | similar | 近似词 |
| 上下位 | hypernym | 上位词 (is-a) |
| | hyponym | 下位词 |
| | instance_hypernym | 实例上位词 |
| | instance_hyponym | 实例下位词 |
| 整体部分 | holo_part / mero_part | 整体-部分 |
| | holo_member / mero_member | 整体-成员 |
| | holo_substance / mero_substance | 整体-物质 |
| 因果 | causes / is_caused_by | 导致/被导致 |
| 蕴含 | entails / is_entailed_by | 蕴含/被蕴含 |
| 派生 | derivation | 派生关系 (run → runner) |
| | participle | 分词关系 |
| | pertainym | 相关形容词 (sun → solar) |
| 领域 | domain_region / has_domain_region | 地域领域 |
| | domain_topic / has_domain_topic | 主题领域 |
| 其他 | also, attribute, exemplifies, is_exemplified_by, other | 参见、属性、例证等 |

**设计要点：**
- `related_word` 存词形文本而非 words.id，因为关联词可能不在本地词库中。查询时 `JOIN words ON word_relations.related_word = words.word`。
- wn 的 synset_relations 导入时展开：synset A {bank, depository} —hypernym→ synset B {institution}，展开为 bank→institution 和 depository→institution 两条关系。
- 关系是单向的。双向关系（如 synonym）在导入时写两行（A→B 和 B→A）。

### 2.7 word_forms — 词形变化

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

**form_type 与 stardict exchange JSON key 的映射：**

| exchange key | form_type | 示例 |
|-------------|-----------|------|
| pl | plural | banks |
| past | past | banked |
| pp | past_participle | banked |
| ing | present_participle | banking |
| 3rd | third_person | banks |
| comp | comparative | bigger |
| super | superlative | biggest |

### 2.8 evaluations — LLM 质量评价

LLM 对已有数据打分，辅助人工审核优先级排序。LLM 不直接改数据，评价结果写入此表。

```sql
CREATE TABLE evaluations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- 评价目标
    target_table TEXT NOT NULL,    -- 'definitions' / 'translations' / 'examples' / 'word_relations'
    target_id INTEGER NOT NULL,    -- 目标表中的行 ID
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    -- 评价内容
    dimension TEXT NOT NULL,       -- 评价维度
    score INTEGER NOT NULL CHECK(score BETWEEN 1 AND 5),  -- 1=严重问题, 5=完美
    comment TEXT,                  -- 评价说明（LLM 给出的理由）
    -- 建议
    suggestion TEXT,               -- LLM 建议的修正值（如有）
    severity TEXT NOT NULL DEFAULT 'info' CHECK(severity IN ('critical', 'warning', 'info')),
    -- 状态
    reviewed INTEGER NOT NULL DEFAULT 0,  -- 人工是否已处理此评价 (0/1)
    reviewed_at TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_evaluations_target ON evaluations(target_table, target_id);
CREATE INDEX idx_evaluations_word ON evaluations(word_id);
CREATE INDEX idx_evaluations_score ON evaluations(score);
CREATE INDEX idx_evaluations_reviewed ON evaluations(reviewed);
```

**评价维度：**

| dimension | 适用表 | 评估内容 |
|-----------|--------|---------|
| translation_accuracy | translations | 中文翻译是否准确对应英文定义 |
| definition_completeness | definitions | 定义是否过简或过泛 |
| example_naturalness | examples | 例句是否地道 |
| cross_source_consistency | definitions + translations | stardict 和 wn 对同一词的定义/翻译是否矛盾 |
| relation_accuracy | word_relations | 语义关系是否正确 |
| form_correctness | word_forms | 词形变化是否正确 |

**severity 语义：**

| severity | 含义 | 人工处理建议 |
|----------|------|------------|
| critical | 明显错误，必须修正 | 优先处理 |
| warning | 可能有问题，建议复查 | 有空处理 |
| info | 参考信息 | 可选 |

**工作流：**

```
LLM 评价 → 写入 evaluations (reviewed=0)
    → 人工按 severity + score 排序，优先看 critical
    → 人工判断：同意评价 → 修改目标数据 + 标记 evaluations.reviewed=1
                 不同意   → 直接标记 evaluations.reviewed=1（跳过）
```

### 2.9 pending_changes — 数据源更新冲突队列

外部源更新时，与人工修改冲突的数据不自动覆写，而是进入此队列等待审批。

```sql
CREATE TABLE pending_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,     -- 目标表
    row_id INTEGER,               -- 目标表中已有行的 ID；NULL 表示源新增了整行待确认
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    field TEXT,                   -- 字段名 (仅 words 表使用，附属表为 NULL 表示整行)
    old_value TEXT,               -- dict.db 当前值
    new_value TEXT,               -- 新数据源建议值
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn')),
    import_log_id INTEGER REFERENCES import_log(id),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'rejected')),
    resolved_at TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_pending_word ON pending_changes(word_id);
CREATE INDEX idx_pending_status ON pending_changes(status);
```

**与各表的对应关系：**

| table_name | row_id | field | 含义示例 |
|------------|--------|-------|---------|
| definitions | 123 | NULL | 定义行 id=123 的 text 在源中变了，但人工曾改过 |
| translations | 456 | NULL | 翻译行 id=456 的 text 在源中变了 |
| examples | 789 | NULL | 例句行 id=789 的 text 在源中变了 |
| words | 10 | collins | 词条 id=10 的 collins 从 3 变为 4 |
| words | 10 | phonetic | 词条 id=10 的音标变了 |
| word_relations | 55 | NULL | 关系行 id=55 的文本在源中变了 |

**审批流程：**

```
pending (默认)
    ├── 人工通过     → approved,   applied to target table
    ├── 人工调整后通过 → approved,   applied with manual edit
    └── 人工拒绝     → rejected,   target table unchanged
```

### 2.10 change_log — 人工操作审计

```sql
CREATE TABLE change_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,     -- 目标表
    row_id INTEGER NOT NULL,      -- 目标行 ID
    field TEXT,                   -- 被修改的字段名；NULL 表示整行操作
    old_value TEXT,               -- 旧值；NULL 表示新建
    new_value TEXT,               -- 新值；NULL 表示删除
    action TEXT NOT NULL CHECK(action IN ('create', 'update', 'delete', 'review', 'unreview')),
    changed_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_changelog_table_row ON change_log(table_name, row_id);
CREATE INDEX idx_changelog_changed_at ON change_log(changed_at);
```

ETL 自动导入不记日志（避免膨胀）。仅人工操作时写入。

### 2.11 import_log — 外部源导入记录

```sql
CREATE TABLE import_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn')),
    source_version TEXT,          -- 源版本标识 (stardict=SHA256, wn=版本号如'2025+')
    mode TEXT NOT NULL CHECK(mode IN ('full', 'incremental')),
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    new_words INTEGER DEFAULT 0,
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
    conflicts INTEGER DEFAULT 0,  -- 产生的 pending_changes 数量
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed')),
    error TEXT
);
```

### 2.12 完整建表 SQL

```sql
-- ============================================================
-- 单词主表
-- ============================================================
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

-- ============================================================
-- 英文定义
-- ============================================================
CREATE TABLE definitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
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

-- ============================================================
-- 中文翻译
-- ============================================================
CREATE TABLE translations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    language TEXT NOT NULL DEFAULT 'zh',
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
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

-- ============================================================
-- 例句
-- ============================================================
CREATE TABLE examples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    translation TEXT,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn', 'llm', 'human')),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    trans_source TEXT CHECK(trans_source IN ('stardict', 'wn', 'llm', 'human')),
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

-- ============================================================
-- 语义关系
-- ============================================================
CREATE TABLE word_relations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    related_word TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'wn' CHECK(source IN ('wn', 'stardict', 'llm', 'human')),
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

-- ============================================================
-- 词形变化
-- ============================================================
CREATE TABLE word_forms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    form TEXT NOT NULL,
    form_type TEXT NOT NULL CHECK(form_type IN (
        'plural', 'past', 'past_participle', 'present_participle',
        'third_person', 'comparative', 'superlative'
    )),
    source TEXT NOT NULL DEFAULT 'stardict' CHECK(source IN ('stardict', 'wn', 'human')),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    modified INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(word_id, form, form_type)
);
CREATE INDEX idx_wordforms_word ON word_forms(word_id);

-- ============================================================
-- LLM 质量评价
-- ============================================================
CREATE TABLE evaluations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target_table TEXT NOT NULL,
    target_id INTEGER NOT NULL,
    word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    dimension TEXT NOT NULL,
    score INTEGER NOT NULL CHECK(score BETWEEN 1 AND 5),
    comment TEXT,
    suggestion TEXT,
    severity TEXT NOT NULL DEFAULT 'info' CHECK(severity IN ('critical', 'warning', 'info')),
    reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_at TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_evaluations_target ON evaluations(target_table, target_id);
CREATE INDEX idx_evaluations_word ON evaluations(word_id);
CREATE INDEX idx_evaluations_score ON evaluations(score);
CREATE INDEX idx_evaluations_reviewed ON evaluations(reviewed);

-- ============================================================
-- 数据源更新冲突队列
-- ============================================================
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

-- ============================================================
-- 人工操作审计
-- ============================================================
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

-- ============================================================
-- 导入运行记录
-- ============================================================
CREATE TABLE import_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL CHECK(source IN ('stardict', 'wn')),
    source_version TEXT,
    mode TEXT NOT NULL CHECK(mode IN ('full', 'incremental')),
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    new_words INTEGER DEFAULT 0,
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
    conflicts INTEGER DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed')),
    error TEXT
);
```

---

## 3. ETL 导入流程

### 3.1 总体流水线

```
1. import_stardict.py   →  words + definitions + translations + examples + word_forms
2. import_wn.py         →  合并 words + definitions + translations + examples + word_forms + word_relations
3. fill_gaps.py         →  LLM 补全 translations + examples.translation + definitions + examples.text
4. evaluate.py          →  LLM 评价已有数据质量，写入 evaluations
```

第二步是"合并"模式——wn 中已存在的词条更新补充字段，不新增重复行；wn 独有的词条新增。

### 3.2 字段导入优先级

合并时，**高优先级源可以覆盖低优先级源的数据**（仅限未人工审核的行）。

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
| exchange | stardict(主) + wn(补) | stardict 为主，wn forms 表补充缺失 |
| detail | stardict only | |

合并规则：
- **已有值不被覆盖**，除非新源优先级更高。
- **curated=1 的 words 行**，任何 ETL 都不修改其字段。
- **reviewed=1 或 modified=1 的附属表行**，ETL 不覆写，差异进入 pending_changes。

### 3.3 import_stardict.py

```
for each row in stardict:
    1. 确定 pos:
       - stardict.pos 有值 → pos = stardict.pos, pos_source = 'stardict'
       - stardict.pos 为空 → pos = NULL, pos_source = NULL

    2. 插入或获取 words:
       INSERT OR IGNORE INTO words (word, pos, pos_source, phonetic, phonetic_source='stardict',
           audio, collins, oxford, bnc, frq, tag, exchange, detail, sources='stardict')
       SELECT id FROM words WHERE word=? AND (pos=? OR pos IS NULL) → word_id

    3. definition 非空:
       INSERT OR IGNORE INTO definitions (word_id, text, source='stardict')

    4. translation 非空:
       INSERT OR IGNORE INTO translations (word_id, text, source='stardict')

    5. 解析 detail JSON 中的例句:
       - 如有例句:
         INSERT OR IGNORE INTO examples (word_id, text, translation=中文翻译或NULL, source='stardict',
           trans_source='stardict' IF translation NOT NULL ELSE NULL)

    6. 展开 exchange JSON:
       for each key in exchange:
         form_type = EXCHANGE_MAP[key]
         INSERT OR IGNORE INTO word_forms (word_id, form=value, form_type, source='stardict')
```

### 3.4 import_wn.py

```
# 阶段 1: 导入词形
for each lexicon in wn (oewn, omw-cmn):
    for each entry e where e.pos in (n, v, a, r, s):
        for each form f in e.forms where f.rank = 0 (只取 lemma):
            word = f.form
            pos = e.pos

            existing = SELECT * FROM words WHERE word=? AND (pos=? OR pos IS NULL)

            if existing:
                if existing.curated: continue   # 已审核，不碰

                # 补全 sources
                if 'wn' NOT IN existing.sources:
                    UPDATE words SET sources = sources || ',wn'

                # 补全 pos (如果原为空)
                if existing.pos IS NULL:
                    UPDATE words SET pos=?, pos_source='wn'

                # 补全 phonetic (如果原为空，且 wn 有发音)
                if existing.phonetic IS NULL:
                    prons = fetch pronunciations for this form
                    if prons:
                        UPDATE words SET phonetic=best_pron(prons), phonetic_source='wn'
            else:
                prons = fetch pronunciations for this form
                INSERT INTO words (word, pos, pos_source='wn',
                    phonetic=best_pron(prons), phonetic_source='wn' if prons else NULL,
                    sources='wn')

            word_id = resolved

# 阶段 2: 导入定义和例句 (仅英文 oewn)
for each synset s in oewn:
    words_in_synset = resolve all word_ids for s.words
    for each word_id in words_in_synset:
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
                INSERT OR IGNORE INTO translations (word_id, text=cw, source='wn')
    else:
        # 中文 synset 独有概念 (无英文对应)
        # 将中文 lemma 作为独立 words 行
        for each word w in s_cmn.words:
            INSERT OR IGNORE INTO words (word, pos=s_cmn.pos, pos_source='wn', sources='wn')
            if s_cmn.definition:
                INSERT OR IGNORE INTO definitions (word_id, text=s_cmn.definition, source='wn')

# 阶段 4: 导入语义关系
for each synset_relation sr:
    type = relation_types[sr.type_rowid].type
    source_words = all lemmas in source synset
    target_words = all lemmas in target synset
    for each sw in source_words:
        for each tw in target_words:
            if sw != tw:  # 避免自己指向自己
                INSERT OR IGNORE INTO word_relations (word_id, related_word=tw, relation_type=type, source='wn')

for each sense_relation sr:
    type = relation_types[sr.type_rowid].type
    source_word = lemma of source sense's entry
    target_word = lemma of target sense's entry
    if source_word != target_word:
        INSERT OR IGNORE INTO word_relations (word_id, related_word=target_word, relation_type=type, source='wn')

# 阶段 5: 补充词形变化
for each form f where f.rank > 0:
    lemma = f.entry's rank-0 form
    word_id = resolve words row for lemma
    form_type = infer from form text and entry.pos
    INSERT OR IGNORE INTO word_forms (word_id, form=f.form, form_type, source='wn')
```

### 3.5 导入模式：首次 vs 重导

`INSERT OR IGNORE` 的 UNIQUE 约束只能防止**完全相同的行**被重复插入，但数据源更新后同一 (word_id, source) 的文本可能变化（如定义从 "X" 变为 "Y"），此时 `INSERT OR IGNORE` 会同时保留新旧两行——这是错误的。

因此导入逻辑分三种情况处理，首次导入和重导使用同一套逻辑：

```
for each source_row in source:
    # 1. 同源匹配：找 dict.db 中同 word_id + 同 source 的行
    same_source = SELECT * FROM {table} WHERE word_id=? AND source=?

    if same_source:
        if same_source.text == source_row.text:
            # 情况1: 文本未变 → 跳过
            continue
        else:
            # 情况2: 文本变了 → 更新或冲突
            if same_source.modified == 0 AND same_source.reviewed == 0:
                UPDATE {table} SET text=?, updated_at=now() WHERE id=same_source.id
            else:
                INSERT OR IGNORE INTO pending_changes (...)
    else:
        # 情况3: 无同源匹配 → 全新数据，直接插入
        INSERT OR IGNORE INTO {table} (word_id, text, source, ...)
```

**首次导入时**，dict.db 为空，所有行都走情况3（无同源匹配），直接插入。

**重导时**，已存在的行走情况1（跳过）或情况2（更新/冲突）。三种情况覆盖了所有场景。

详细冲突检测逻辑见第 5 章。

---

## 4. LLM 角色

LLM 在系统中有两个独立角色：**内容生成**（填充数据）和 **质量评价**（审核数据）。两者不混在一条流水线中。

### 4.1 角色 A: 内容生成 (fill_gaps.py)

**触发时机：** ETL 导入完成后立即执行。

**原则：** 只在外部源都没提供数据时才生成。已有外部源数据的字段，LLM 不碰。

```python
def fill_gaps():
    for each word in words:
        # 1. 补中文翻译
        if SELECT COUNT(*) FROM translations WHERE word_id = word.id == 0:
            result = llm(f"Translate '{word.word}' ({word.pos}) into Chinese. "
                         "Provide only the Chinese translation, comma-separated if multiple.")
            INSERT INTO translations (word_id, text, source='llm', reviewed=0)
            # 如果生成了多个翻译，插多行

        # 2. 补英文定义
        if SELECT COUNT(*) FROM definitions WHERE word_id = word.id == 0:
            result = llm(f"Define '{word.word}' ({word.pos}) in English. "
                         "One concise sentence.")
            INSERT INTO definitions (word_id, text, source='llm', reviewed=0)

        # 3. 补例句
        if SELECT COUNT(*) FROM examples WHERE word_id = word.id == 0:
            result = llm(f"Write 2-3 example sentences using '{word.word}' ({word.pos}). "
                         "Return one sentence per line.")
            for each sentence in result:
                INSERT INTO examples (word_id, text=sentence, source='llm', reviewed=0)

        # 4. 补例句中文翻译
        for each example in SELECT * FROM examples WHERE word_id = word.id AND translation IS NULL:
            result = llm(f"Translate to Chinese: '{example.text}'")
            UPDATE examples SET translation=result, trans_source='llm', trans_reviewed=0
            WHERE id = example.id
```

**可选扩展（未来启用）：**

```python
        # 5. 补用法说明
        if not has_usage_note(word.id):
            result = llm(f"Describe the usage and common collocations of '{word.word}' ({word.pos}).")
            INSERT INTO usage_notes (word_id, text, source='llm', reviewed=0)

        # 6. 补近义词辨析
        synonyms = get_synonyms(word.id)
        if synonyms and not has_differentiation(word.id):
            result = llm(f"Differentiate '{word.word}' from: {', '.join(synonyms[:5])}. "
                         "Explain the nuances in Chinese.")
            INSERT INTO differentiations (word_id, text, source='llm', reviewed=0)

        # 7. 补词根词缀
        if not has_morphology(word.id):
            result = llm(f"Analyze the morphology of '{word.word}': prefix, root, suffix.")
            INSERT INTO morphologies (word_id, text, source='llm', reviewed=0)
```

### 4.2 角色 B: 质量评价 (evaluate.py)

**触发时机：** 可独立运行，也可在 fill_gaps.py 后运行，或定期运行。

**原则：** LLM 只评价，不修改数据。评价结果写入 `evaluations` 表，人工审核后决定是否采纳。

```python
def evaluate():
    for each word in words:
        # 1. 翻译一致性：中文翻译是否准确对应英文定义
        defs = SELECT text FROM definitions WHERE word_id = word.id AND source IN ('stardict','wn')
        trs = SELECT text FROM translations WHERE word_id = word.id
        if defs and trs:
            result = llm(f"""
                Evaluate the translation accuracy for '{word.word}'.
                English definitions: {defs}
                Chinese translations: {trs}
                Check: does the Chinese translation accurately reflect the English definition?
                Score 1-5 (1=completely wrong, 5=perfect).
                Respond in JSON: {{"score": N, "comment": "...", "suggestion": "..." or null}}
            """)
            if result.score < 4:
                INSERT INTO evaluations (target_table='translations', target_id=tr.id,
                    word_id=word.id, dimension='translation_accuracy',
                    score=result.score, comment=result.comment,
                    suggestion=result.suggestion,
                    severity='critical' if result.score <= 2 else 'warning')

        # 2. 定义完整度
        for each def in definitions WHERE word_id = word.id:
            result = llm(f"""
                Evaluate this definition for '{word.word}':
                "{def.text}"
                Is it too brief, too vague, or adequately complete?
                Score 1-5 (1=useless, 5=comprehensive).
                Respond in JSON.
            """)
            if result.score < 4:
                INSERT INTO evaluations (...)

        # 3. 例句自然度
        for each ex in examples WHERE word_id = word.id AND source IN ('stardict','wn','llm'):
            result = llm(f"""
                Evaluate this example sentence for '{word.word}':
                "{ex.text}"
                Is it natural, idiomatic English? Does it illustrate the word's meaning well?
                Score 1-5.
                Respond in JSON.
            """)
            if result.score < 4:
                INSERT INTO evaluations (...)

        # 4. 跨源一致性 (stardict vs wn)
        stardict_defs = SELECT text FROM definitions WHERE word_id=word.id AND source='stardict'
        wn_defs = SELECT text FROM definitions WHERE word_id=word.id AND source='wn'
        if stardict_defs and wn_defs:
            result = llm(f"""
                Compare these two definitions for '{word.word}':
                stardict: {stardict_defs}
                wn: {wn_defs}
                Are they consistent or contradictory?
                Score 1-5 (1=contradictory, 5=perfectly consistent).
                Respond in JSON.
            """)
            if result.score < 4:
                INSERT INTO evaluations (...)
```

**评价结果的使用方式：**

```
人工审核面板：
  1. 按 severity DESC, score ASC 排序 evaluations
  2. 优先看 critical (score 1-2)
  3. 人工判断：
     - 同意评价 → 修改目标数据 + UPDATE evaluations SET reviewed=1
     - 不同意   → UPDATE evaluations SET reviewed=1 (跳过)
```

---

## 5. 数据源更新与冲突处理

### 5.1 更新模式

| 模式 | 触发条件 | 行为 |
|------|---------|------|
| 全量重导 | 源 DB 被新版替换 | 所有行对比，检测冲突 |
| 增量导入 | 源 DB 在原基础上新增词条 | 只导入新增，不检测冲突 |

```bash
python import_stardict.py --mode full
python import_stardict.py --mode incremental
python import_wn.py --mode full --lexicon oewn:2026
python import_wn.py --mode incremental --lexicon oewn:2026
```

### 5.2 版本追踪

stardict: 计算文件 SHA256 作为 `source_version`。SHA256 不变则跳过导入。

wn: 从 `lexicons.version` 字段获取（如 `2025+`）。指定新版本号后执行导入。

### 5.3 冲突检测规则

| 行状态 | 行为 |
|--------|------|
| `modified=0 AND reviewed=0` | **直接覆写** |
| `modified=1 OR reviewed=1` (附属表) | **生成 pending_change** |
| `curated=1` (words 表) | **字段级对比，差异生成 pending_change** |
| 源中全新的数据 | **直接插入** |

### 5.4 附属表冲突检测

粒度：行级。源中同一 (word_id, source) 的文本发生了变化。

```
for each source_row in source:
    # 精确匹配：同 word_id + 同 source + 同 text → 跳过
    match = SELECT * FROM {table} WHERE word_id=? AND source=? AND text=?
    if match: continue

    # 同源匹配：同 word_id + 同 source，但 text 不同
    same_source = SELECT * FROM {table} WHERE word_id=? AND source=?

    if same_source:
        if same_source.modified == 0 AND same_source.reviewed == 0:
            # 未人工改动 → 直接覆写
            UPDATE {table} SET text=?, updated_at=now() WHERE id=same_source.id
        else:
            # 已人工改动 → 生成冲突
            INSERT OR IGNORE INTO pending_changes (
                table_name, row_id=same_source.id, word_id, field=NULL,
                old_value=same_source.text, new_value=source_row.text,
                source=source_name, import_log_id=current_import_id
            )
    else:
        # 无同源匹配 → 全新数据，直接插入
        INSERT OR IGNORE INTO {table} (word_id, text, source, ...)
```

### 5.5 words 表冲突检测

粒度：字段级。curated=1 时逐字段对比。

```
for each source_row in source:
    existing = SELECT * FROM words WHERE word=? AND (pos=? OR ...)

    if existing:
        # 追加 sources
        if source_name NOT IN existing.sources:
            UPDATE words SET sources = sources || ',' || source_name

        if existing.curated == 0:
            # 未审核 → 逐字段覆写（按优先级）
            for each field in [pos, phonetic, collins, oxford, bnc, frq, tag, exchange]:
                if source_has_field AND source_priority >= existing_priority:
                    UPDATE words SET {field}=? WHERE id=existing.id
        else:
            # 已审核 → 逐字段对比，差异生成 pending_change
            for each field in [pos, phonetic, collins, oxford, bnc, frq, tag, exchange]:
                old_val, new_val = existing.{field}, source_row.{field}
                if old_val != new_val and new_val is not None:
                    INSERT OR IGNORE INTO pending_changes (
                        table_name='words', row_id=existing.id, word_id=existing.id,
                        field=field, old_value=old_val, new_value=new_val,
                        source=source_name, import_log_id=current_import_id
                    )
    else:
        INSERT INTO words (word, pos, ..., sources=source_name)
```

### 5.6 源删除处理

外部源删除数据时，dict.db **不做级联删除**。只更新 `words.sources`：

```python
words_from_source = SELECT word FROM words WHERE sources LIKE '%{source}%'
words_in_current = {all words from current source}
deleted = words_from_source - words_in_current

for word in deleted:
    UPDATE words SET sources = REPLACE(REPLACE(sources, source, ''), ',,', ',')
    -- 清理首尾逗号
```

词条和附属数据保留。源删除不意味着数据失效——其他源可能仍提供同一词条。

### 5.7 冲突审批

人工在 UI 中逐条处理 `pending_changes`：

```
审批通过:
    if table_name == 'words':
        UPDATE words SET {field} = new_value, updated_at=now() WHERE id=row_id
    else:
        UPDATE {table_name} SET text = new_value,
               modified=0, reviewed=1, reviewed_at=now(), updated_at=now()
        WHERE id=row_id
    INSERT INTO change_log (...)
    UPDATE pending_changes SET status='approved', resolved_at=now()

审批通过但人工调整:
    # 人工先修改 pending_changes.new_value 为自定义值
    UPDATE pending_changes SET new_value=? WHERE id=?
    # 然后走审批通过流程

审批拒绝:
    # 不修改数据表
    UPDATE pending_changes SET status='rejected', resolved_at=now()
```

---

## 6. 查询示例

### 6.1 查单词完整信息

```sql
SELECT w.word, w.pos, w.phonetic, w.phonetic_source,
       w.collins, w.bnc, w.frq, w.tag, w.curated,
       d.text AS definition, d.source AS def_src, d.reviewed AS def_ok,
       t.text AS translation, t.source AS tr_src, t.reviewed AS tr_ok
FROM words w
LEFT JOIN definitions d ON d.word_id = w.id
LEFT JOIN translations t ON t.word_id = w.id
WHERE w.word = 'bank'
ORDER BY
    CASE WHEN d.source='human' OR d.modified=1 THEN 0
         WHEN d.source IN ('stardict','wn') AND d.reviewed=1 THEN 1
         WHEN d.source IN ('stardict','wn') AND d.reviewed=0 THEN 2
         WHEN d.source='llm' AND d.reviewed=1 THEN 3
         ELSE 4 END,
    CASE WHEN t.source='human' OR t.modified=1 THEN 0
         WHEN t.source IN ('stardict','wn') AND t.reviewed=1 THEN 1
         WHEN t.source IN ('stardict','wn') AND t.reviewed=0 THEN 2
         WHEN t.source='llm' AND t.reviewed=1 THEN 3
         ELSE 4 END;
```

### 6.2 查同义词

```sql
SELECT wr.related_word, wr.relation_type, wr.source, wr.reviewed
FROM word_relations wr
JOIN words w ON w.id = wr.word_id
WHERE w.word = 'happy' AND wr.relation_type = 'synonym'
ORDER BY
    CASE WHEN wr.source='human' OR wr.modified=1 THEN 0 ELSE 1 END,
    wr.reviewed DESC;
```

### 6.3 查词形变化

```sql
SELECT form, form_type
FROM word_forms
WHERE word_id = (SELECT id FROM words WHERE word = 'run');
```

### 6.4 查高频词

```sql
SELECT word, pos, frq, collins FROM words
WHERE frq IS NOT NULL
ORDER BY frq ASC
LIMIT 100;
```

### 6.5 查例句（含翻译状态）

```sql
SELECT e.text, e.translation,
       e.source, e.reviewed,
       e.trans_source, e.trans_reviewed
FROM examples e
JOIN words w ON w.id = e.word_id
WHERE w.word = 'bank'
ORDER BY
    CASE WHEN e.source='human' OR e.modified=1 THEN 0
         WHEN e.source IN ('stardict','wn') AND e.reviewed=1 THEN 1
         ELSE 2 END;
```

### 6.6 查待审核数据

```sql
-- 按优先级：LLM 未审核 > 外部源未审核
SELECT w.word, d.text, d.source, 'definition' AS type
FROM definitions d JOIN words w ON w.id = d.word_id
WHERE d.reviewed = 0 AND d.source = 'llm'
UNION ALL
SELECT w.word, d.text, d.source, 'definition' AS type
FROM definitions d JOIN words w ON w.id = d.word_id
WHERE d.reviewed = 0 AND d.source IN ('stardict','wn')
ORDER BY type, source;
```

### 6.7 查 LLM 评价

```sql
-- 按严重程度排序，优先看 critical
SELECT w.word, e.target_table, e.dimension,
       e.score, e.severity, e.comment, e.suggestion
FROM evaluations e
JOIN words w ON w.id = e.word_id
WHERE e.reviewed = 0
ORDER BY
    CASE e.severity WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,
    e.score ASC;
```

### 6.8 查待审批冲突

```sql
SELECT w.word, pc.table_name, pc.field,
       pc.old_value, pc.new_value, pc.source, pc.created_at
FROM pending_changes pc
JOIN words w ON w.id = pc.word_id
WHERE pc.status = 'pending'
ORDER BY pc.source, pc.created_at;
```

### 6.9 审核操作 SQL

```sql
-- 审核通过一笔翻译
UPDATE translations
SET reviewed = 1, reviewed_at = datetime('now'), updated_at = datetime('now')
WHERE id = ?;
INSERT INTO change_log (table_name, row_id, action) VALUES ('translations', ?, 'review');

-- 人工修改定义文本
UPDATE definitions
SET text = ?, modified = 1, modified_at = datetime('now'),
    reviewed = 1, reviewed_at = datetime('now'), updated_at = datetime('now')
WHERE id = ?;
INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action)
VALUES ('definitions', ?, 'text', ?, ?, 'update');

-- 人工新建翻译
INSERT INTO translations (word_id, text, source, reviewed, modified)
VALUES (?, ?, 'human', 1, 0);

-- 审批通过数据源冲突 (附属表)
UPDATE definitions SET
    text = (SELECT new_value FROM pending_changes WHERE id = ?),
    modified = 0, reviewed = 1, reviewed_at = datetime('now'), updated_at = datetime('now')
WHERE id = (SELECT row_id FROM pending_changes WHERE id = ?);
UPDATE pending_changes SET status = 'approved', resolved_at = datetime('now') WHERE id = ?;
INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action)
SELECT table_name, row_id, field, old_value, new_value, 'update'
FROM pending_changes WHERE id = ?;

-- 审批通过数据源冲突 (words 表字段)
UPDATE words SET
    collins = CAST((SELECT new_value FROM pending_changes WHERE id = ?) AS INTEGER),
    updated_at = datetime('now')
WHERE id = (SELECT row_id FROM pending_changes WHERE id = ?);
UPDATE pending_changes SET status = 'approved', resolved_at = datetime('now') WHERE id = ?;

-- 审批拒绝
UPDATE pending_changes SET status = 'rejected', resolved_at = datetime('now') WHERE id = ?;

-- 处理 LLM 评价 (同意并修改)
UPDATE definitions SET text = (SELECT suggestion FROM evaluations WHERE id = ?),
    modified = 1, modified_at = datetime('now'),
    reviewed = 1, reviewed_at = datetime('now'), updated_at = datetime('now')
WHERE id = (SELECT target_id FROM evaluations WHERE id = ?);
UPDATE evaluations SET reviewed = 1, reviewed_at = datetime('now') WHERE id = ?;

-- 处理 LLM 评价 (不同意，跳过)
UPDATE evaluations SET reviewed = 1, reviewed_at = datetime('now') WHERE id = ?;
```

---

## 7. 数据统计估算

| 表 | 预估行数 | 说明 |
|----|---------|------|
| words | ~200,000 | stardict 全量 + wn 补充 |
| definitions | ~250,000 | wn ~120k + stardict ~130k (去重后) + LLM |
| translations | ~180,000 | stardict 全量 + wn ILI 中文映射 + LLM |
| examples | ~80,000 | wn ~50k + stardict detail 解析 + LLM 生成 |
| word_relations | ~400,000 | wn synset_relations 展开 + sense_relations |
| word_forms | ~100,000 | stardict exchange 展开 + wn forms 补充 |
| evaluations | ~变动 | 取决于评价覆盖范围和阈值 |
| pending_changes | ~0 起步 | 仅在源更新且有人工修改时产生 |
| change_log | ~0 起步 | 仅人工操作时写入 |
| import_log | ~10/年 | 每次导入一条记录 |
