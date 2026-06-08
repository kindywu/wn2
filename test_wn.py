import sqlite3
from pathlib import Path
import yaml

DB_PATH = Path(__file__).parent / "data" / "wn.db"


# ---------------------------------------------------------------------------
# 底层查询工具
# ---------------------------------------------------------------------------

def fetch_row(conn, table, rowid):
    c = conn.cursor()
    c.execute(f"SELECT rowid, * FROM {table} WHERE rowid = ?", (rowid,))
    row = c.fetchone()
    if not row:
        return None
    cols = [d[0] for d in c.description]
    return {col: val for col, val in zip(cols, row)}


def fetch_refs(conn, table, col, rowid):
    """反向引用：找出 table.col = rowid 的所有行 rowid。"""
    c = conn.cursor()
    c.execute(f"SELECT rowid FROM {table} WHERE {col} = ?", (rowid,))
    return [r[0] for r in c.fetchall()]


def make_node(table, data, children=None):
    return {
        "table": table,
        "rowid": data["rowid"],
        "data": {k: v for k, v in data.items() if v is not None},
        "children": children or [],
    }


def make_basic_node(conn, table, rowid):
    """只取基本信息，不展开子树。"""
    data = fetch_row(conn, table, rowid)
    if not data:
        return None
    node = make_node(table, data)
    node["note"] = "基本信息，不展开子树"
    return node


# ---------------------------------------------------------------------------
# 各表展开函数
# 每个函数只负责自己这一层，子节点调用对应的 expand_* 函数
# ---------------------------------------------------------------------------

def expand_ili_status(conn, rowid, visited):
    if (k := ("ili_statuses", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "ili_statuses", rowid)
    return make_node("ili_statuses", data) if data else None


def expand_ili(conn, rowid, visited):
    if (k := ("ilis", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "ilis", rowid)
    if not data:
        return None
    children = []

    # 正向 FK → ili_statuses
    if data.get("status_rowid"):
        child = expand_ili_status(conn, data["status_rowid"], visited)
        if child:
            child["via_fk"] = "status_rowid"
            children.append(child)

    # 反向引用 ilis → synsets（通过 ili_rowid）
    for ref_rowid in fetch_refs(conn, "synsets", "ili_rowid", rowid):
        child = expand_synset(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "ili_rowid"
            children.append(child)

    return make_node("ilis", data, children)


def expand_lexfile(conn, rowid, visited):
    if (k := ("lexfiles", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "lexfiles", rowid)
    return make_node("lexfiles", data) if data else None


def expand_relation_type(conn, rowid, visited):
    if (k := ("relation_types", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "relation_types", rowid)
    return make_node("relation_types", data) if data else None


def expand_definition(conn, rowid, visited):
    if (k := ("definitions", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "definitions", rowid)
    return make_node("definitions", data) if data else None


def expand_synset_example(conn, rowid, visited):
    if (k := ("synset_examples", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "synset_examples", rowid)
    return make_node("synset_examples", data) if data else None


def expand_sense_example(conn, rowid, visited):
    if (k := ("sense_examples", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "sense_examples", rowid)
    return make_node("sense_examples", data) if data else None


def expand_adjposition(conn, rowid, visited):
    if (k := ("adjpositions", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "adjpositions", rowid)
    return make_node("adjpositions", data) if data else None


def expand_count(conn, rowid, visited):
    if (k := ("counts", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "counts", rowid)
    return make_node("counts", data) if data else None


def expand_proposed_ili(conn, rowid, visited):
    if (k := ("proposed_ilis", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "proposed_ilis", rowid)
    return make_node("proposed_ilis", data) if data else None


def expand_syntactic_behaviour(conn, rowid, visited):
    """syntactic_behaviours 是共享资源，只取基本信息，不展开反向引用。"""
    if (k := ("syntactic_behaviours", rowid)) in visited:
        return None
    visited.add(k)
    return make_basic_node(conn, "syntactic_behaviours", rowid)


def expand_syntactic_behaviour_sense(conn, rowid, visited):
    if (k := ("syntactic_behaviour_senses", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "syntactic_behaviour_senses", rowid)
    if not data:
        return None
    children = []

    # 正向 FK → syntactic_behaviours（浅查，不展开反向引用）
    if data.get("syntactic_behaviour_rowid"):
        child = expand_syntactic_behaviour(conn, data["syntactic_behaviour_rowid"], visited)
        if child:
            child["via_fk"] = "syntactic_behaviour_rowid"
            children.append(child)

    return make_node("syntactic_behaviour_senses", data, children)


def expand_synset_relation(conn, rowid, visited):
    if (k := ("synset_relations", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "synset_relations", rowid)
    if not data:
        return None
    children = []

    # target_rowid：浅查，防止沿 hypernym/hyponym 链无限延伸
    if data.get("target_rowid"):
        child = make_basic_node(conn, "synsets", data["target_rowid"])
        if child:
            child["via_fk"] = "target_rowid"
            children.append(child)

    # type_rowid → relation_types
    if data.get("type_rowid"):
        child = expand_relation_type(conn, data["type_rowid"], visited)
        if child:
            child["via_fk"] = "type_rowid"
            children.append(child)

    return make_node("synset_relations", data, children)


def expand_sense_relation(conn, rowid, visited):
    if (k := ("sense_relations", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "sense_relations", rowid)
    if not data:
        return None
    children = []

    # target_rowid：浅查
    if data.get("target_rowid"):
        child = make_basic_node(conn, "senses", data["target_rowid"])
        if child:
            child["via_fk"] = "target_rowid"
            children.append(child)

    # type_rowid → relation_types
    if data.get("type_rowid"):
        child = expand_relation_type(conn, data["type_rowid"], visited)
        if child:
            child["via_fk"] = "type_rowid"
            children.append(child)

    return make_node("sense_relations", data, children)


def _synset_words(conn, synset_rowid):
    """查询同义词集包含的所有词形（lemma）。"""
    c = conn.cursor()
    c.execute("""
        SELECT DISTINCT f.form
        FROM senses s
        JOIN entries e ON e.rowid = s.entry_rowid
        JOIN forms f ON f.entry_rowid = e.rowid AND f.rank = 0
        WHERE s.synset_rowid = ?
        ORDER BY f.form
    """, (synset_rowid,))
    return [r[0] for r in c.fetchall()]


def _synset_translations(conn, ili_rowid):
    """通过 ILI 查询中文翻译。"""
    c = conn.cursor()
    c.execute("""
        SELECT DISTINCT f.form
        FROM synsets s
        JOIN senses se ON se.synset_rowid = s.rowid
        JOIN entries e ON e.rowid = se.entry_rowid
        JOIN forms f ON f.entry_rowid = e.rowid AND f.rank = 0
        WHERE s.ili_rowid = ? AND e.id LIKE 'omw-cmn-%'
        ORDER BY f.form
    """, (ili_rowid,))
    return [r[0] for r in c.fetchall()]


def expand_synset(conn, rowid, visited):
    if (k := ("synsets", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "synsets", rowid)
    if not data:
        return None
    data["words"] = _synset_words(conn, rowid)

    # 中文翻译（通过 ILI 关联）
    if ili_rowid := data.get("ili_rowid"):
        data["translations"] = _synset_translations(conn, ili_rowid)

    children = []

    # 正向 FK → ilis
    if data.get("ili_rowid"):
        child = expand_ili(conn, data["ili_rowid"], visited)
        if child:
            child["via_fk"] = "ili_rowid"
            children.append(child)

    # 正向 FK → lexfiles
    if data.get("lexfile_rowid"):
        child = expand_lexfile(conn, data["lexfile_rowid"], visited)
        if child:
            child["via_fk"] = "lexfile_rowid"
            children.append(child)

    # 反向引用 synsets → senses
    for ref_rowid in fetch_refs(conn, "senses", "synset_rowid", rowid):
        child = expand_sense(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "synset_rowid"
            children.append(child)

    # 反向引用 synsets → definitions
    for ref_rowid in fetch_refs(conn, "definitions", "synset_rowid", rowid):
        child = expand_definition(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "synset_rowid"
            children.append(child)

    # 反向引用 synsets → synset_examples
    for ref_rowid in fetch_refs(conn, "synset_examples", "synset_rowid", rowid):
        child = expand_synset_example(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "synset_rowid"
            children.append(child)

    # 反向引用 synsets → synset_relations (source)
    for ref_rowid in fetch_refs(conn, "synset_relations", "source_rowid", rowid):
        child = expand_synset_relation(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "source_rowid"
            children.append(child)

    # 反向引用 synsets → synset_relations (target)：浅查
    for ref_rowid in fetch_refs(conn, "synset_relations", "target_rowid", rowid):
        child = make_basic_node(conn, "synset_relations", ref_rowid)
        if child:
            child["via_reverse"] = "target_rowid"
            child["note"] = "作为 target 的同义词集关系 (基本信息)"
            children.append(child)

    # 反向引用 synsets → proposed_ilis
    for ref_rowid in fetch_refs(conn, "proposed_ilis", "synset_rowid", rowid):
        child = expand_proposed_ili(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "synset_rowid"
            children.append(child)

    return make_node("synsets", data, children)


def expand_sense(conn, rowid, visited):
    if (k := ("senses", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "senses", rowid)
    if not data:
        return None
    children = []

    # 正向 FK → synsets
    if data.get("synset_rowid"):
        child = expand_synset(conn, data["synset_rowid"], visited)
        if child:
            child["via_fk"] = "synset_rowid"
            children.append(child)

    # 反向引用 senses → sense_relations (source)
    for ref_rowid in fetch_refs(conn, "sense_relations", "source_rowid", rowid):
        child = expand_sense_relation(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "source_rowid"
            children.append(child)

    # 反向引用 senses → sense_relations (target)：浅查
    for ref_rowid in fetch_refs(conn, "sense_relations", "target_rowid", rowid):
        child = make_basic_node(conn, "sense_relations", ref_rowid)
        if child:
            child["via_reverse"] = "target_rowid"
            child["note"] = "作为 target 的义项关系 (基本信息)"
            children.append(child)

    # 反向引用 senses → sense_examples
    for ref_rowid in fetch_refs(conn, "sense_examples", "sense_rowid", rowid):
        child = expand_sense_example(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "sense_rowid"
            children.append(child)

    # 反向引用 senses → definitions (义项级定义)
    for ref_rowid in fetch_refs(conn, "definitions", "sense_rowid", rowid):
        child = expand_definition(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "sense_rowid"
            children.append(child)

    # 反向引用 senses → syntactic_behaviour_senses
    for ref_rowid in fetch_refs(conn, "syntactic_behaviour_senses", "sense_rowid", rowid):
        child = expand_syntactic_behaviour_sense(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "sense_rowid"
            children.append(child)

    # 反向引用 senses → adjpositions
    for ref_rowid in fetch_refs(conn, "adjpositions", "sense_rowid", rowid):
        child = expand_adjposition(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "sense_rowid"
            children.append(child)

    # 反向引用 senses → counts
    for ref_rowid in fetch_refs(conn, "counts", "sense_rowid", rowid):
        child = expand_count(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "sense_rowid"
            children.append(child)

    return make_node("senses", data, children)


def expand_entry(conn, rowid, visited):
    if (k := ("entries", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "entries", rowid)
    if not data:
        return None
    children = []

    # 反向引用 entries → forms (其他词形)
    for ref_rowid in fetch_refs(conn, "forms", "entry_rowid", rowid):
        child = expand_form(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "entry_rowid"
            children.append(child)

    # 反向引用 entries → senses
    for ref_rowid in fetch_refs(conn, "senses", "entry_rowid", rowid):
        child = expand_sense(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "entry_rowid"
            children.append(child)

    return make_node("entries", data, children)


def expand_pronunciation(conn, rowid, visited):
    if (k := ("pronunciations", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "pronunciations", rowid)
    return make_node("pronunciations", data) if data else None


def expand_form(conn, rowid, visited):
    if (k := ("forms", rowid)) in visited:
        return None
    visited.add(k)
    data = fetch_row(conn, "forms", rowid)
    if not data:
        return None
    children = []

    # 正向 FK → entries
    if data.get("entry_rowid"):
        child = expand_entry(conn, data["entry_rowid"], visited)
        if child:
            child["via_fk"] = "entry_rowid"
            children.append(child)

    # 反向引用 forms → pronunciations
    for ref_rowid in fetch_refs(conn, "pronunciations", "form_rowid", rowid):
        child = expand_pronunciation(conn, ref_rowid, visited)
        if child:
            child["via_reverse"] = "form_rowid"
            children.append(child)

    return make_node("forms", data, children)


# ---------------------------------------------------------------------------
# 友好化：将原始树转换为用户可读的扁平结构
# ---------------------------------------------------------------------------

def make_friendly(nodes):
    """将原始查询树转换为用户友好的结构，去除 FK 噪音、扁平化层级。"""

    def _get_relations(children):
        """从 children 中提取关系列表，每个关系含 type 和 target。"""
        rels = []
        for c in children:
            if c.get("table") not in ("synset_relations", "sense_relations"):
                continue
            type_name = None
            target_id = None
            for cc in c.get("children", []):
                if cc.get("table") == "relation_types":
                    type_name = cc.get("data", {}).get("type")
                if cc.get("via_fk") == "target_rowid":
                    target_id = cc.get("data", {}).get("id")
            if type_name:
                rel = {"type": type_name}
                if target_id:
                    rel["target"] = target_id
                rels.append(rel)
        return rels

    def _walk(node):
        table = node.get("table", "")
        data = node.get("data", {})
        children = node.get("children", [])

        if table == "forms":
            r = {"form": data.get("form")}
            if "rank" in data:
                r["rank"] = data["rank"]

            # 发音
            prons = {}
            for c in children:
                if c.get("table") == "pronunciations":
                    cd = c.get("data", {})
                    v = cd.get("variety") or "other"
                    if val := cd.get("value"):
                        prons.setdefault(v, []).append(val)
            if prons:
                r["pronunciations"] = {k: v if len(v) > 1 else v[0] for k, v in prons.items()}

            entries = [_walk(c) for c in children if c.get("table") == "entries"]
            if entries:
                r["entries"] = entries
            return r

        if table == "entries":
            r = {}
            for k in ("id", "pos"):
                if k in data:
                    r[k] = data[k]
            senses = [_walk(c) for c in children if c.get("table") == "senses"]
            if senses:
                r["senses"] = senses
            return r

        if table == "senses":
            r = {}
            for k in ("id", "entry_rank", "synset_rank"):
                if k in data:
                    r[k] = data[k]

            # 把 synset 的信息上提到 sense 层
            for c in children:
                if c.get("table") == "synsets":
                    r.update(_walk(c))
                    break

            # sense 层的关系（sense_relations）
            rels = _get_relations(children)
            if rels:
                r["relations"] = r.get("relations", []) + rels

            # sense 层的 examples
            exs = []
            for c in children:
                if c.get("table") == "sense_examples":
                    if e := c.get("data", {}).get("example"):
                        exs.append(e)
            if exs:
                r["examples"] = exs

            return r

        if table == "synsets":
            r = {}
            if "id" in data:
                r["synset"] = data["id"]
            if "words" in data:
                r["words"] = data["words"]
            if data.get("translations"):
                r["translations"] = data["translations"]

            # definitions → 纯文本
            defs = []
            for c in children:
                if c.get("table") == "definitions":
                    if d := c.get("data", {}).get("definition"):
                        defs.append(d)
            if defs:
                r["definition"] = defs[0] if len(defs) == 1 else defs

            # examples → 纯文本列表
            exs = []
            for c in children:
                if c.get("table") in ("synset_examples", "sense_examples"):
                    if e := c.get("data", {}).get("example"):
                        exs.append(e)
            if exs:
                r["examples"] = exs

            # relations
            rels = _get_relations(children)
            if rels:
                r["relations"] = rels

            return r

        return {}

    return [_walk(n) for n in nodes]


# ---------------------------------------------------------------------------
# 入口
# ---------------------------------------------------------------------------

def lookup_word(conn: sqlite3.Connection, word: str, output_file: str):
    print(f"\n{'='*60}")
    print(f"查询单词: '{word}'")
    print(f"{'='*60}")

    c = conn.cursor()
    c.execute("SELECT rowid FROM forms WHERE form = ?", (word,))
    form_rows = [r[0] for r in c.fetchall()]

    if not form_rows:
        print(f"在 forms 表中没有找到 '{word}'")
        return None

    print(f"找到 {len(form_rows)} 条 form 记录，rowids = {form_rows}")

    visited = set()
    trees = []

    for form_rowid in form_rows:
        print(f"\n从 forms(rowid={form_rowid}) 开始...")
        tree = expand_form(conn, form_rowid, visited)
        if tree:
            trees.append(tree)

    output_path = Path(output_file)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    friendly = make_friendly(trees)
    with open(output_path, "w", encoding="utf-8") as f:
        yaml.dump(friendly, f, allow_unicode=True, sort_keys=False, default_flow_style=False)

    print(f"\n结果已保存到: {output_path.absolute()}")

    # 原始树仍可通过返回值获取
    return friendly


def main():
    conn = sqlite3.connect(DB_PATH)

    print("=" * 60)
    print("Bilingual WordNet Lookup — direct SQL on data/wn.db")
    print("Type a word (English or Chinese) and press Enter.")
    print("Ctrl+C to exit")
    print("=" * 60)

    while True:
        try:
            query = input("\n>>> ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\nBye!")
            break
        if not query:
            continue
        lookup_word(conn, query, output_file=f"output/{query}.yaml")

    conn.close()


if __name__ == "__main__":
    main()
