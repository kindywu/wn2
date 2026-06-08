import sqlite3
from pathlib import Path

DB_PATH = Path(__file__).parent / "data" / "wn.db"
LEXICON_EN = 1   # oewn:2025+
LEXICON_CMN = 2  # omw-cmn:1.4


def lookup_word(conn: sqlite3.Connection, word: str):
    """查询一个词，优先英文词库，并尝试通过 ILI 找对应中文释义。"""
    c = conn.cursor()
    results = []

    # 1) 英文词库查询
    c.execute(
        """
        SELECT DISTINCT s.synset_rowid, sy.pos, d.definition, p.value, p.variety
        FROM forms f
        JOIN entries e ON f.entry_rowid = e.rowid
        JOIN senses s ON s.entry_rowid = e.rowid
        JOIN synsets sy ON s.synset_rowid = sy.rowid
        LEFT JOIN definitions d ON d.synset_rowid = sy.rowid AND d.sense_rowid IS NULL
        LEFT JOIN pronunciations p ON f.rowid = p.form_rowid
        WHERE f.form = ? AND e.lexicon_rowid = ?
        """,
        (word, LEXICON_EN),
    )
    for synset_rowid, pos, definition, pron_value, _ in c.fetchall():
        # 英文 lemmas
        c.execute(
            """
            SELECT DISTINCT f.form
            FROM senses s
            JOIN entries e ON s.entry_rowid = e.rowid
            JOIN forms f ON f.entry_rowid = e.rowid
            WHERE s.synset_rowid = ? AND e.lexicon_rowid = ?
            """,
            (synset_rowid, LEXICON_EN),
        )
        english_lemmas = [row[0] for row in c.fetchall()]

        # 通过 ILI 找中文 lemmas
        chinese_lemmas = []
        c.execute(
            "SELECT ili_rowid FROM synsets WHERE rowid = ?",
            (synset_rowid,),
        )
        row = c.fetchone()
        if row and row[0]:
            ili_rowid = row[0]
            c.execute(
                """
                SELECT DISTINCT f.form
                FROM synsets sy
                JOIN senses s ON s.synset_rowid = sy.rowid
                JOIN entries e ON s.entry_rowid = e.rowid
                JOIN forms f ON f.entry_rowid = e.rowid
                WHERE sy.ili_rowid = ? AND sy.lexicon_rowid = ?
                """,
                (ili_rowid, LEXICON_CMN),
            )
            chinese_lemmas = [r[0] for r in c.fetchall()]

        results.append({
            "pos": pos,
            "definition": definition,
            "pronunciation": pron_value,
            "english_lemmas": english_lemmas,
            "chinese_lemmas": chinese_lemmas,
        })
    return results


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

        results = lookup_word(conn, query)

        if not results:
            print(f"  (no results for '{query}')")
            continue

        for i, r in enumerate(results, 1):
            print(f"\n  [{i}]  pos={r['pos']}")
            print(f"       definition: {r['definition']}")
            print(f"       pronunciation: {r['pronunciation']}")
            if r["english_lemmas"]:
                print(f"       english:  {', '.join(r['english_lemmas'])}")
            if r["chinese_lemmas"]:
                print(f"       chinese:  {', '.join(r['chinese_lemmas'])}")

    conn.close()


if __name__ == "__main__":
    main()
