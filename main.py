import wn
from pathlib import Path
wn.config.data_directory = Path(__file__).parent / "data"

def main():
    wn.download("oewn:2025+")
    oewn = wn.Wordnet("oewn:2025+")

    print("WordNet Lookup  —  Open English WordNet 2025+")
    print("Ctrl+C to exit")
    print("=" * 60)


if __name__ == "__main__":
    main()
