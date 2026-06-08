import sqlite3
import sys

db_path = "data/stardict.db"

def lookup(word):
    conn = sqlite3.connect(db_path)
    cur = conn.execute("SELECT * FROM stardict WHERE word = ?", (word,))
    row = cur.fetchone()
    conn.close()

    if not row:
        print(f"not found: {word}")
        return

    col_names = [desc[0] for desc in cur.description]

    for c in col_names:
        if c in ("id", "sw"):
            continue
        v = row[col_names.index(c)]
        if v is None:
            continue
        print(f"{c}: {v}")


if __name__ == "__main__":
    word = " ".join(sys.argv[1:]) if len(sys.argv) > 1 else input("word: ")
    lookup(word.strip())
