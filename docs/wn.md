# wn.db 数据库结构分析

## 概述

`wn.db` 是一个 SQLite 3.x 数据库，存储了 **Open English WordNet 2025+** 和 **Chinese Open WordNet (omw-cmn 1.4)** 两个词库。该数据库遵循 **WN-LMF**（WordNet Lexical Markup Framework）规范，通过 Python 库 `wn`（>=1.1.0）进行访问。

- **文件大小**: ~133 MB
- **词库 1**: Open English WordNet (oewn:2025+) — 英语 (en), CC BY 4.0
- **词库 2**: Chinese Open WordNet (omw-cmn:1.4) — 简体中文 (cmn-Hans), wordnet
- **表数量**: 26 张表
- **行数规模**: 英文约 120 万行，中文约 25 万行，合计约 145 万行

---

## 核心实体

### lexicon（词库）

一个完整的词汇数据库实例。本项目使用 **Open English WordNet 2025+**（标识符 `oewn:2025+`）。一个数据库可以包含多个词库，通过 `lexicon_dependencies` 和 `lexicon_extensions` 建立词库间的依赖/扩展关系。

### entry（词条）

一个独立的词或短语单元，是词汇的基本载体。如 `bank`、`give up`、`run` 均为独立的词条。每个词条有一个唯一 ID（如 `oewn-bank-n`）和一个词性（n/v/a/r/s，分别对应名词/动词/形容词/副词/形容词卫星）。

### form（词形）

词条的表面文字形式。一个词条可以有多个词形：

- 首选词形（lemma）：rank=0，如 `bank`
- 变体词形：如复数 `banks`、过去式 `ran` 等

`normalized_form` 是标准化后的形式（通常是小写），用于搜索匹配。

### synset（同义词集）

一组表达**同一概念**的同义词集合。这是 WordNet 的核心组织单元。例如 {bank, depository financial institution, banking concern} 构成一个同义词集，表达"银行-金融机构"这个概念。

每个 synset 关联：定义（definition）、例句（synset_examples）、词性（pos）、语义域分类（lexfile）、与其他 synset 的关系（上下位、整体部分等）、跨语言索引（ILI）。

### sense（义项）

连接 **entry** 和 **synset** 的桥梁，表示一个词条在**某个特定含义**下的用法。例如 `bank` 至少有 3 个义项：银行（金融机构）、河岸、倾斜（动词）。每个 sense 对应一个 synset。

### ILI（Inter-Lingual Index，跨语言索引）

一个独立于语言的概念标识符，用于在不同语言的 WordNet 之间建立概念映射。例如英文 synset {dog} 和中文 synset {狗} 指向同一个 ILI，从而可以跨语言查询。

ILI 有两种状态：
- **presupposed**（预设）：已验证的稳定 ILI
- **proposed**（提议）：待审核的新 ILI 映射（存放在 `proposed_ilis` 表中）

---

## ER 图

```mermaid
erDiagram
    entries["entries (225,222 rows)"] {
        int rowid PK
        string id
        int lexicon_rowid FK
        string pos
        json metadata
    }

    forms["forms (229,696 rows)"] {
        int rowid PK
        string id
        int lexicon_rowid FK
        int entry_rowid FK
        string form
        string normalized_form
        string script
        int rank
    }

    senses["senses (292,468 rows)"] {
        int rowid PK
        string id
        int lexicon_rowid FK
        int entry_rowid FK
        int entry_rank
        int synset_rowid FK
        int synset_rank
        json metadata
    }

    synsets["synsets (162,876 rows)"] {
        int rowid PK
        string id
        int lexicon_rowid FK
        int ili_rowid FK
        string pos
        int lexfile_rowid FK
        json metadata
    }

    definitions["definitions (120,569 rows)"] {
        int rowid PK
        int lexicon_rowid FK
        int synset_rowid FK
        text definition
        string language
        int sense_rowid FK
        json metadata
    }

    ilis["ilis (117,351 rows)"] {
        int rowid PK
        string id UK
        int status_rowid FK
        text definition
        json metadata
    }

    ili_statuses["ili_statuses (2 rows)"] {
        int rowid PK
        string status UK
    }

    lexfiles["lexfiles (45 rows)"] {
        int rowid PK
        string name UK
    }

    relation_types["relation_types (28 rows)"] {
        int rowid PK
        string type UK
    }

    synset_relations["synset_relations (297,172 rows)"] {
        int rowid PK
        int lexicon_rowid FK
        int source_rowid FK
        int target_rowid FK
        int type_rowid FK
        json metadata
    }

    sense_relations["sense_relations (122,054 rows)"] {
        int rowid PK
        int lexicon_rowid FK
        int source_rowid FK
        int target_rowid FK
        int type_rowid FK
        json metadata
    }

    sense_synset_relations["sense_synset_relations (0 rows)"] {
        int rowid PK
        int lexicon_rowid FK
        int source_rowid FK
        int target_rowid FK
        int type_rowid FK
        json metadata
    }

    pronunciations["pronunciations (44,638 rows)"] {
        int form_rowid FK
        int lexicon_rowid FK
        text value
        string variety
        string notation
        bool phonemic
        text audio
    }

    synset_examples["synset_examples (49,724 rows)"] {
        int rowid PK
        int lexicon_rowid FK
        int synset_rowid FK
        text example
        string language
        json metadata
    }

    syntactic_behaviours["syntactic_behaviours (39 rows)"] {
        int rowid PK
        string id
        int lexicon_rowid FK
        string frame UK
    }

    syntactic_behaviour_senses["syntactic_behaviour_senses (41,650 rows)"] {
        int syntactic_behaviour_rowid FK
        int sense_rowid FK
    }

    adjpositions["adjpositions (1,052 rows)"] {
        int sense_rowid FK
        string adjposition
    }

    proposed_ilis["proposed_ilis (3,213 rows)"] {
        int rowid PK
        int synset_rowid FK
        text definition
        json metadata
    }

    entries ||--o{ forms : word_forms
    entries ||--o{ senses : has_sense

    forms ||--o{ pronunciations : pronounced_as

    senses ||--o{ sense_relations : source
    senses ||--o{ sense_relations : target
    senses ||--o{ sense_synset_relations : source
    senses ||--o{ syntactic_behaviour_senses : has_behaviour
    senses ||--o{ adjpositions : positioned_as
    senses }o--|| synsets : belongs_to
    senses }o--|| definitions : defined_by

    synsets ||--o{ synset_relations : source
    synsets ||--o{ synset_relations : target
    synsets ||--o{ sense_synset_relations : target
    synsets ||--o{ definitions : defined_by
    synsets ||--o{ synset_examples : example_of
    synsets ||--o{ proposed_ilis : proposes
    synsets }o--|| ilis : maps_to
    synsets }o--|| lexfiles : categorized_as

    ilis }o--|| ili_statuses : has_status

    synset_relations }o--|| relation_types : typed_as
    sense_relations }o--|| relation_types : typed_as
    sense_synset_relations }o--|| relation_types : typed_as

    syntactic_behaviour_senses }o--|| syntactic_behaviours : references
```

---

## 贯穿示例：以 "bank"（银行）为主线

下面用 **bank** 的一个具体义项 ——"银行，金融机构"—— 串起 26 张表之间的完整关系链路。

### 起点：lexicon

一切数据的容器是词库 `oewn:2025+`：

```
lexicons.rowid = 1, specifier = 'oewn:2025+', language = 'en'
```

### entry → form：词条与词形

`bank` 作为一个名词词条存储于 `entries`，其首选词形（lemma）存储在 `forms`：

| 表 | 字段 | 值 |
|----|------|-----|
| entries | id | `oewn-bank-n` |
| entries | pos | `n` |
| forms | form | `bank` |
| forms | rank | `0`（首选词形/lemma） |

`bank` 还有一个动词词条 `oewn-bank-v`（pos=`v`），同样通过 `forms.entry_rowid` 关联到词形 `bank`。

### sense → synset → definition：义项、同义词集与定义

名词 `bank` 有 10 个义项（senses），每个义项属于一个 synset。选取"银行"义项：

| 表 | 字段 | 值 |
|----|------|-----|
| senses | id | `oewn-bank__1.14.00..` |
| senses | entry_rank | `2`（该词条的第 2 个义项） |
| senses | synset_rank | `1`（该 synset 中的排序位置） |
| synsets | id | `oewn-08437235-n` |
| definitions | definition | *a financial institution that accepts deposits and channels the money into lending activities* |

同时，这个 synset 还包含另外 3 个词条的义项，组成同义词集：

| synset_rank | form | entry_id |
|-------------|------|----------|
| 0 | depository financial institution | `oewn-depository_financial_institution-n` |
| 1 | **bank** | `oewn-bank-n` |
| 2 | banking concern | `oewn-banking_concern-n` |
| 3 | banking company | `oewn-banking_company-n` |

> 这展示了 **entry → sense → synset** 的多对多模式：一个 synset 通过多行 senses 聚合多个 entry，一个 entry 通过多行 senses 分散到多个 synset。

### lexfile：语义域分类

synset `oewn-08437235-n` 通过 `lexfile_rowid` 归入 `noun.group`（集合名词），表示"银行"在语义上属于一种机构/集合。45 个 lexfile 构成了 WordNet 的顶层语义分类。

### ILI：跨语言索引

synset 通过 `ili_rowid` 指向跨语言索引：

| 表 | 字段 | 值 |
|----|------|-----|
| ilis | id | `i81364` |
| ili_statuses | status | `presupposed`（已确认的稳定 ILI） |

未来如果中文 WordNet 中有一个 synset 也指向 `i81364`，就能实现"bank ↔ 银行"的跨语言映射。

### synset_relations：同义词集关系（层级结构）

基于 `oewn-08437235-n`，通过 `synset_relations` 构建上下位层级：

```
synset_relations: source_rowid → target_rowid (via relation_types)
```

| 关系类型 | 目标 synset | 目标定义 |
|----------|------------|---------|
| **hypernym** (上位词) | `oewn-08071473-n` | an institution that collects funds and invests them in financial assets |
| **holo_member** (整体-成员) | `oewn-08083327-n` | banks collectively |
| **hyponym** (下位词) | `oewn-08251549-n` | credit union |
| **hyponym** | `oewn-08435377-n` | a financial institution that accepts demand deposits and provides other services for the public |
| **hyponym** | …还有 10+ 个下位词 | (Federal Reserve bank, commercial bank, etc.) |

`relation_types` 表定义了 28 种关系类型，这三个关系使用了其中的 `hypernym`、`holo_member`、`hyponym`。

### sense_relations：义项关系（细粒度语义）

名词"银行"义项通过 `sense_relations` 派生了三个动词用法：

| 关系类型 | 目标 sense | 目标词 | 含义 |
|----------|-----------|--------|------|
| **derivation** (派生) | `oewn-bank__2.40.02..` | bank (v) | do business with a bank |
| **derivation** | `oewn-bank__2.40.01..` | bank (v) | be in the banking business |
| **derivation** | `oewn-bank__2.40.00..` | bank (v) | put into a bank account |

> `synset_relations` 连接 synset → synset（概念层级）；`sense_relations` 连接 sense → sense（具体词义间的关系），粒度更细。

### synset_examples：例句

每个 synset 可附带使用例句：

| example | language |
|---------|----------|
| *he cashed a check at the bank* | (null=en) |
| *that bank holds the mortgage on my home* | (null=en) |

### syntactic_behaviours：句法行为（动词）

动词 `bank` 的 8 个义项各自关联句法模式。通过 `syntactic_behaviour_senses`（sense → syntactic_behaviour 的关联表）：

| sense_id | 句法框架 | sb_id |
|----------|---------|-------|
| bank 的存钱义项 | Somebody ----s something | `vtai` (及物-有灵主语-无灵宾语) |
| bank 的开银行义项 | Somebody ----s | `via` (不及物-有灵主语) |
| bank 的倾斜义项 | Somebody ----s something | `vtai` |

### pronunciations：发音（补充示例）

并非所有词都有发音数据。以 **give** 为例：

| form | IPA | variety | phonemic |
|------|-----|---------|----------|
| give | ɡɪv | (null) | 0 |

`pronunciations` 通过 `form_rowid` 关联到具体的词形，支持 US/GB 等多种变体。

### adjpositions：形容词位置（补充示例）

形容词通过 `adjpositions` 标记句法位置。以 **able** 为例：

| form | adjposition |
|------|-------------|
| able | predicative |

### 空表的位置

`counts`、`entry_index`、`tags`、`sense_synset_relations`、`lexicon_dependencies`、`lexicon_extensions` 等表在当前数据库中为空，但它们是 WN-LMF 规范预留的结构，供扩展使用（词频统计、词形标签、跨词库依赖等）。

### 完整关系链路图

```
lexicons (1) ──── oewn:2025+
  │
  ├── entries ──── oewn-bank-n (pos=n)
  │     │
  │     ├── forms ──── "bank" (rank=0) ──→ [pronunciations: IPA, 变体]
  │     │
  │     └── senses ──── oewn-bank__1.14.00.. (entry_rank=2, synset_rank=1)
  │           │
  │           ├── belongs_to → synsets ──── oewn-08437235-n (pos=n)
  │           │     │
  │           │     ├── lexfiles ──── noun.group
  │           │     ├── ilis ──── i81364 (status: presupposed)
  │           │     ├── definitions ──── "a financial institution that..."
  │           │     ├── synset_examples ──── "he cashed a check at the bank"
  │           │     ├── synset_relations ──→ hypernym → oewn-08071473-n
  │           │     │                    ──→ hyponym → oewn-08251549-n
  │           │     │                    ──→ holo_member → oewn-08083327-n
  │           │     │                    ──→ [relation_types: 28 种类型]
  │           │     │
  │           │     └── other senses (同义词集其他成员)
  │           │           └── depository financial institution / banking concern / banking company
  │           │
  │           ├── sense_relations ──→ derivation → bank (v) - "put into a bank account"
  │           │                   ──→ derivation → bank (v) - "do business with a bank"
  │           │
  │           └── syntactic_behaviour_senses ──→ vtai "Somebody ----s something"
  │                                           ──→ via  "Somebody ----s"
  │
  └── [其他表: counts, tags, entry_index, sense_synset_relations 等预留空表]
```

---

## 表结构详解

### 核心表

#### lexicons — 词库元信息

| 字段 | 类型 | 说明 |
|------|------|------|
| rowid | INTEGER PK | 内部 ID |
| specifier | TEXT UNIQUE | 词库标识符 `id:version` |
| id | TEXT | 用户面向 ID |
| label | TEXT | 显示名称 |
| language | TEXT | BCP-47 语言标签 |
| email | TEXT | 联系邮箱 |
| license | TEXT | 许可证 URL |
| version | TEXT | 版本号 |
| url | TEXT | 项目 URL |
| citation | TEXT | 引用信息 |
| logo | TEXT | Logo URL |
| metadata | META | 元数据 (JSON) |
| modified | BOOLEAN | 是否被修改 |

当前数据：2 行，oewn:2025+（英文）和 omw-cmn:1.4（中文）。

#### entries — 词条

| 字段 | 类型 | 说明 |
|------|------|------|
| rowid | INTEGER PK | 内部 ID |
| id | TEXT NOT NULL | 词条 ID，如 `oewn-bank-n` |
| lexicon_rowid | INTEGER FK → lexicons | 所属词库 |
| pos | TEXT | 词性 (n/v/a/r/s) |
| metadata | META | 元数据 |

唯一约束：`(id, lexicon_rowid)`。行数：225,222（英文 161,875 + 中文 63,347）。

#### forms — 词形

| 字段 | 类型 | 说明 |
|------|------|------|
| rowid | INTEGER PK | 内部 ID |
| id | TEXT | 词形 ID |
| lexicon_rowid | INTEGER FK → lexicons | 所属词库 |
| entry_rowid | INTEGER FK → entries | 所属词条 |
| form | TEXT NOT NULL | 词形文本 |
| normalized_form | TEXT | 标准化词形（小写等） |
| script | TEXT | 书写系统 |
| rank | INTEGER DEFAULT 1 | 排序，0 为首选词形（lemma） |

唯一约束：`(entry_rowid, form, script)`。行数：229,696（英文 166,349 + 中文 63,347）。

#### synsets — 同义词集

| 字段 | 类型 | 说明 |
|------|------|------|
| rowid | INTEGER PK | 内部 ID |
| id | TEXT NOT NULL | 同义词集 ID，如 `oewn-00001740-n` |
| lexicon_rowid | INTEGER FK → lexicons | 所属词库 |
| ili_rowid | INTEGER FK → ilis | 跨语言索引 |
| pos | TEXT | 词性 |
| lexfile_rowid | INTEGER FK → lexfiles | 语义域分类 |
| metadata | META | 元数据 |

行数：162,876（英文 120,564 + 中文 42,312）。ID 前缀：`oewn-`（英文）、`omw-cmn-`（中文）。

#### senses — 义项

| 字段 | 类型 | 说明 |
|------|------|------|
| rowid | INTEGER PK | 内部 ID |
| id | TEXT NOT NULL | 义项 ID |
| lexicon_rowid | INTEGER FK → lexicons | 所属词库 |
| entry_rowid | INTEGER FK → entries | 所属词条 |
| entry_rank | INTEGER DEFAULT 1 | 在词条中的排序 |
| synset_rowid | INTEGER FK → synsets | 所属同义词集 |
| synset_rank | INTEGER DEFAULT 1 | 在同义词集中的排序 |
| metadata | META | 元数据 |

行数：292,468（英文 212,659 + 中文 79,809）。Sense 是 Entry 和 Synset 之间的多对多关联表。

---

### 语义分类

#### lexfiles — 语义域

将 synset 按语义分成 45 个类别，继承自 Princeton WordNet 的分类体系：

| 分类名 | 含义 |
|------|------|
| adj.all | 所有形容词 |
| adj.pert | 关系形容词 |
| adj.ppl | 分词形容词 |
| adv.all | 所有副词 |
| noun.Tops | 顶级名词（最抽象的概念） |
| noun.act | 动作名词 |
| noun.animal | 动物名词 |
| noun.artifact | 人造物名词 |
| noun.attribute | 属性名词 |
| noun.body | 身体名词 |
| verb.body | 身体动作动词 |
| ... | （共 45 个） |

#### relation_types — 关系类型

定义了 28 种语义关系：

| 类别 | 类型 | 含义 |
|------|------|------|
| 上下位 | hypernym | 上位词（is-a 关系） |
| | hyponym | 下位词 |
| 实例 | instance_hypernym | 实例上位词 |
| | instance_hyponym | 实例下位词 |
| 整体部分 | holo_part / mero_part | 整体-部分 |
| | holo_member / mero_member | 整体-成员 |
| | holo_substance / mero_substance | 整体-物质 |
| 反义近似 | antonym | 反义词 |
| | similar | 近似词 |
| 因果 | causes / is_caused_by | 导致 / 被导致 |
| 蕴含 | entails / is_entailed_by | 蕴含 / 被蕴含 |
| 派生 | derivation | 派生关系（如 run → runner） |
| | participle | 分词关系 |
| | pertainym | 相关形容词（如 sun → solar） |
| 领域 | domain_region / has_domain_region | 地域领域 |
| | domain_topic / has_domain_topic | 主题领域 |
| 其他 | also, attribute, exemplifies, is_exemplified_by, other | 参见、属性、例证等 |

---

### 关系表

#### synset_relations — 同义词集关系

synset 之间的语义关系，构成 WordNet 的层级结构。例如：
`{dog}` —hypernym→ `{canine}` —hypernym→ `{carnivore}` —hypernym→ ... → `{entity}`

这是 WordNet 最核心的关系表，共 297,172 行。

#### sense_relations — 义项关系

义项之间的语义关系，如反义关系（antonym）、派生关系（derivation）。粒度比 synset_relations 更细，关联到具体词条的特定含义。共 122,054 行。

#### sense_synset_relations — 义项-同义词集关系

跨层级的关系，连接 sense 到 synset。当前表为空（预留结构）。

---

### 内容表

#### definitions — 定义

每个 synset 对应的文字定义。共 120,569 行。例如 synset {able} 的定义为：

> (usually followed by 'to') having the necessary means or skill or know-how or authority to do something

#### synset_examples — 同义词集例句

为 synset 提供的使用例句。共 49,724 行。

#### sense_examples — 义项例句

为具体义项提供的例句，粒度比 synset_examples 更细。当前为空。

---

### 跨语言索引 (ILI)

#### ili_statuses — ILI 状态

2 行：`presupposed`（预设）、`proposed`（提议）。

#### ilis — 跨语言索引

| 字段 | 类型 | 说明 |
|------|------|------|
| rowid | INTEGER PK | 内部 ID |
| id | TEXT UNIQUE NOT NULL | ILI ID |
| status_rowid | INTEGER FK → ili_statuses | 状态 |
| definition | TEXT | 多语言通用定义 |
| metadata | META | 元数据 |

共 117,351 行。

#### proposed_ilis — 提议的 ILI

待审核的 ILI 映射提议，共 3,213 行。

---

### 语音与语法

#### pronunciations — 发音

词形的 IPA 音标信息，支持不同口音变体（US/GB）。`phonemic=1` 表示音位标音（broad transcription）。共 44,638 行。

#### syntactic_behaviours — 句法行为

描述动词在句子中的搭配模式（句型框架），共 39 种。每种行为描述一个主语/宾语的句法模式：

| ID | 框架 | 含义 |
|------|------|------|
| vtai | Somebody ----s something | 及物-有灵主语-无灵宾语 |
| vtii | Something ----s something | 及物-无灵主语-无灵宾语 |
| vii | Something ----s | 不及物-无灵主语 |
| via | Somebody ----s | 不及物-有灵主语 |
| via-inf | Somebody ----s INFINITIVE | 后接不定式 |
| via-that | Somebody ----s that CLAUSE | 后接 that 从句 |
| vtaa | Somebody ----s somebody | 及物-双有灵 |

命名规则：`v`=verb，`t`=transitive，`i`=intransitive，`a`=animate（有灵），`i`=inanimate（无灵）。例如 `vtaa` = verb transitive animate-animate（及物动词，主语宾语均为有灵）。

每个 sense 通过 `syntactic_behaviour_senses`（41,650 行）关联到其适用的句法行为模式。

#### adjpositions — 形容词位置

记录形容词的句法位置（如 predicative/attributive），共 1,052 行。

---

### 预留空表

以下表是 WN-LMF 规范定义的结构，当前数据中为空：

| 表名 | 用途 |
|------|------|
| counts | 义项使用频次统计 |
| entry_index | 按 lemma 快速查找词条 |
| tags | 词形标签（如语体标注） |
| lexicon_dependencies | 词库间的依赖关系 |
| lexicon_extensions | 词库间的扩展关系 |
| unlexicalized_senses | 标记未词化的义项 |
| unlexicalized_synsets | 标记未词化的同义词集 |

---

## 核心查询示例

### 查询某个词的所有义项

```sql
SELECT e.id, f.form, s.id AS synset_id, d.definition
FROM entries e
JOIN forms f ON f.entry_rowid = e.rowid AND f.rank = 0
JOIN senses se ON se.entry_rowid = e.rowid
JOIN synsets s ON se.synset_rowid = s.rowid
JOIN definitions d ON d.synset_rowid = s.rowid
WHERE f.form = 'bank';
```

### 查询某个同义词集的上位词

```sql
SELECT ss.id, d.definition
FROM synset_relations sr
JOIN synsets ss ON sr.target_rowid = ss.rowid
JOIN relation_types rt ON sr.type_rowid = rt.rowid
JOIN definitions d ON d.synset_rowid = ss.rowid
WHERE sr.source_rowid = (
    SELECT rowid FROM synsets WHERE id = 'oewn-09200144-n'
)
AND rt.type = 'hypernym';
```

### 查询某个同义词集包含的所有词形

```sql
SELECT f.form
FROM senses se
JOIN forms f ON f.entry_rowid = se.entry_rowid AND f.rank = 0
WHERE se.synset_rowid = (
    SELECT rowid FROM synsets WHERE id = 'oewn-09200144-n'
);
```

---

## 与 Python `wn` 库的对应关系

| wn API | 涉及的表 |
|--------|---------|
| `wn.Wordnet("oewn:2025+")` | lexicons |
| `wordnet.words()` | entries + forms |
| `wordnet.synsets()` | synsets |
| `wordnet.senses()` | senses |
| `synset.definition()` | definitions |
| `synset.examples()` | synset_examples |
| `synset.hypernyms()` | synset_relations (type=hypernym) |
| `synset.hyponyms()` | synset_relations (type=hyponym) |
| `synset.lemmas()` | senses → entries → forms |
| `sense.relations()` | sense_relations |
| `wordnet.ilis` | ilis |

---

## 中文 WordNet (omw-cmn)

### 概述

wn.db 中包含 **两个词库**（通过 `lexicons` 表管理）：

| rowid | id | label | language | version |
|-------|-----|-------|----------|---------|
| 1 | oewn | Open English Wordnet | en | 2025+ |
| 2 | omw-cmn | Chinese Open Wordnet | cmn-Hans | 1.4 |

中文词库是 **Open Multilingual WordNet** 的一部分，与英文词库共享同一套 WN-LMF 表结构，通过 `lexicon_rowid` 区分。

### 数据规模对比

| 表 | 英文 (oewn) | 中文 (omw-cmn) |
|----|------------|---------------|
| entries | 161,875 | 63,347 |
| forms | 166,349 | 63,347 |
| senses | 212,659 | 79,809 |
| synsets | 120,564 | 42,312 |
| definitions | 120,569 | 0 |
| synset_examples | 49,724 | 0 |
| synset_relations | 297,172 | 0 |
| sense_relations | 122,054 | 0 |

### 关键差异

**中文词库不包含定义、例句和语义关系。** 中文 synset 通过 **ILI（跨语言索引）** 与英文 synset 共享概念，定义和关系全部复用英文侧数据。

```
中文 synset ──(ili_rowid)──→ ILI ←──(ili_rowid)── 英文 synset
                                                        │
                                                        ├── definitions (英文定义)
                                                        ├── synset_relations (上下位等)
                                                        └── synset_examples (英文例句)
```

### 数据特点

- **一对多映射**：一个中文 synset 通过 ILI 对应一个英文 synset，但一个英文 synset 可能对应多个中文词条的 sense
- **词性分布**：中文词条覆盖 n（名词）、v（动词）、a（形容词）、r（副词）
- **lemma 即词形**：中文词条的 form 和 entry 一一对应（63,347 entries = 63,347 forms），不存在复数/时态等变体
- **无句法行为**：中文侧没有 `syntactic_behaviours` 数据

### 中英文跨语言查询示例

查中文词"银行"对应的英文同义词集和定义：

```sql
SELECT f_en.form, d.definition
FROM entries e_cmn
JOIN forms f_cmn ON f_cmn.entry_rowid = e_cmn.rowid AND f_cmn.rank = 0
JOIN senses se_cmn ON se_cmn.entry_rowid = e_cmn.rowid
JOIN synsets s_cmn ON se_cmn.synset_rowid = s_cmn.rowid
JOIN ilis i ON s_cmn.ili_rowid = i.rowid
JOIN synsets s_en ON s_en.ili_rowid = i.rowid AND s_en.lexicon_rowid = 1
JOIN senses se_en ON se_en.synset_rowid = s_en.rowid
JOIN entries e_en ON se_en.entry_rowid = e_en.rowid
JOIN forms f_en ON f_en.entry_rowid = e_en.rowid AND f_en.rank = 0
JOIN definitions d ON d.synset_rowid = s_en.rowid
WHERE f_cmn.form = '银行';
```

### ILI 作为跨语言桥梁

ILI 表（117,435 行）本身独立于语言，中文和英文 synset 通过指向同一个 ILI 建立概念映射。中文词库 42,312 个 synset 各自指向唯一的 ILI，其中：

- 部分 ILI 同时被中英文 synset 引用（可直接跨语言映射）
- 部分 ILI 仅被中文 synset 引用（中文独有的概念）
- 英文侧 120,564 个 synset 中的 42,312 个有中文映射，其余暂无

ILI 状态仍为 `presupposed`（已确认），中文 ILI 映射来自 OMW 项目发布的稳定版本。
