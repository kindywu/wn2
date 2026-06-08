import wn
from pathlib import Path

wn.config.data_directory = Path(__file__).parent / "data"


def lookup_bilingual(word: str, en: wn.Wordnet, cmn: wn.Wordnet):
    """查询一个词，优先英文词库，并尝试找出对应的中文释义。"""
    results = []

    # 1) 英文词库查询
    for w in en.words(word):
        for synset in w.synsets():
            entry = {
                "pos": synset.pos,
                "definition": synset.definition(),
                "english_lemmas": [lw.lemma() for lw in synset.words()],
                "chinese_lemmas": [],
            }
            # 尝试通过 ILI (Inter-Lingual Index) 找对应的中文 synset
            if synset.ili:
                for c_synset in cmn.synsets(ili=synset.ili):
                    entry["chinese_lemmas"] = [lw.lemma() for lw in c_synset.words()]
            results.append(entry)

    # 2) 如果英文没查到，尝试直接查中文词库
    if not results:
        for w in cmn.words(word):
            for synset in w.synsets():
                results.append({
                    "pos": synset.pos,
                    "definition": synset.definition(),
                    "english_lemmas": [],
                    "chinese_lemmas": [lw.lemma() for lw in synset.words()],
                })

    return results


def main():
    print("Downloading / verifying lexicons ...")
    wn.download("oewn:2025+")
    wn.download("omw-cmn:1.4")

    print("Loading into SQLite ...")
    en = wn.Wordnet("oewn:2025+")
    cmn = wn.Wordnet("omw-cmn:1.4")

    print()
    print("=" * 60)
    print("Bilingual WordNet Lookup  —  oewn:2025+  +  omw-cmn:1.4")
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

        results = lookup_bilingual(query, en, cmn)

        if not results:
            print(f"  (no results for '{query}')")
            continue

        for i, r in enumerate(results, 1):
            print(f"\n  [{i}]  pos={r['pos']}")
            print(f"       definition: {r['definition']}")
            if r["english_lemmas"]:
                print(f"       English:  {', '.join(r['english_lemmas'])}")
            if r["chinese_lemmas"]:
                print(f"       Chinese:  {', '.join(r['chinese_lemmas'])}")


if __name__ == "__main__":
    main()
