import os
import sqlite3

import psycopg2
from dotenv import load_dotenv

load_dotenv()

PG_URL = os.getenv("PG_URL")
SCHEMA = "wn"

# WN-LMF tables in dependency order (parents before children)
TABLES = [
    "ili_statuses",
    "relation_types",
    "lexfiles",
    "lexicons",
    "entries",
    "ilis",
    "synsets",
    "forms",
    "senses",
    "definitions",
    "synset_relations",
    "sense_relations",
    "sense_synset_relations",
    "pronunciations",
    "synset_examples",
    "sense_examples",
    "syntactic_behaviours",
    "syntactic_behaviour_senses",
    "adjpositions",
    "proposed_ilis",
    "counts",
    "entry_index",
    "tags",
    "lexicon_dependencies",
    "lexicon_extensions",
    "unlexicalized_senses",
    "unlexicalized_synsets",
]

# Tables without PRIMARY KEY — need row_number or composite approach
NO_PK_TABLES = {
    "adjpositions",
    "pronunciations",
    "tags",
    "syntactic_behaviour_senses",
    "lexicon_dependencies",
    "lexicon_extensions",
    "unlexicalized_senses",
    "unlexicalized_synsets",
}

# SQLite → PostgreSQL type mapping for known columns
TYPE_MAP = {
    "META": "JSONB",
    "BOOLEAN": "BOOLEAN",
}


def get_sqlite_tables(cursor):
    """Return {table_name: CREATE TABLE sql}."""
    cursor.execute(
        "SELECT name, sql FROM sqlite_master WHERE type='table' ORDER BY name"
    )
    return {row[0]: row[1] for row in cursor.fetchall()}


def get_sqlite_indexes(cursor):
    """Return list of CREATE INDEX sql statements."""
    cursor.execute(
        "SELECT sql FROM sqlite_master WHERE type='index' AND sql IS NOT NULL ORDER BY name"
    )
    return [row[0] for row in cursor.fetchall()]


def sqlite_to_pg(sqlite_sql: str) -> str:
    """Convert a SQLite CREATE TABLE statement to PostgreSQL."""
    import re

    sql = sqlite_sql

    # INTEGER PRIMARY KEY → BIGINT PRIMARY KEY
    sql = re.sub(r"INTEGER PRIMARY KEY", "BIGINT PRIMARY KEY", sql)

    # META → JSONB (as a column type)
    sql = re.sub(r"\bMETA\b", "JSONB", sql)

    # BOOLEAN CHECK(x IN (0,1)) with DEFAULT → BOOLEAN, and fix defaults
    # Pattern: col_name BOOLEAN CHECK( col_name IN (0, 1) ) DEFAULT 0 NOT NULL
    def fix_boolean(m):
        col = m.group(1)
        default_val = m.group(2)
        pg_default = "TRUE" if default_val == "1" else "FALSE"
        not_null = m.group(3) or ""
        return f"{col} BOOLEAN DEFAULT {pg_default}{not_null}"

    sql = re.sub(
        r"(\w+)\s+BOOLEAN\s+CHECK\s*\(\s*\1\s+IN\s*\(\s*0\s*,\s*1\s*\)\s*\)\s+DEFAULT\s+([01])(\s+NOT\s+NULL)?",
        fix_boolean,
        sql,
    )

    # Remove -- comments (from column inline comments)
    sql = re.sub(r"\s*--\s*.*$", "", sql, flags=re.MULTILINE)

    # Normalize whitespace
    sql = re.sub(r"\t", " ", sql)

    return sql


def create_schema(pg_cur):
    pg_cur.execute(f"CREATE SCHEMA IF NOT EXISTS {SCHEMA}")


def create_tables(pg_cur, sqlite_tables):
    for tname in TABLES:
        if tname not in sqlite_tables:
            continue
        sql = sqlite_to_pg(sqlite_tables[tname])
        # Set search_path so tables land in wn schema
        pg_cur.execute(f"SET search_path TO {SCHEMA}")
        pg_cur.execute(sql)
        print(f"  created {SCHEMA}.{tname}")


def copy_table(sq_cur, pg_cur, tname):
    """Copy all rows from SQLite to PostgreSQL for a given table."""
    sq_cur.execute(f"SELECT * FROM [{tname}]")
    rows = sq_cur.fetchall()
    if not rows:
        print(f"  {tname}: 0 rows")
        return

    col_names = [d[0] for d in sq_cur.description]
    cols = ", ".join(f'"{c}"' for c in col_names)
    placeholders = ", ".join(["%s"] * len(col_names))

    # Convert rows for PostgreSQL type compatibility
    boolean_cols = {"modified", "phonemic"}
    jsonb_cols = {"metadata"}

    def convert(row):
        import json

        new_row = []
        for i, val in enumerate(row):
            col = col_names[i]
            if val is None:
                new_row.append(None)
            elif col in boolean_cols:
                new_row.append(bool(val))
            elif col in jsonb_cols:
                # SQLite stores JSON as bytes → decode, parse, re-serialize for psycopg2
                if isinstance(val, bytes):
                    new_row.append(val.decode("utf-8"))
                elif isinstance(val, str):
                    new_row.append(val)
                else:
                    new_row.append(json.dumps(val) if val is not None else None)
            else:
                new_row.append(val)
        return tuple(new_row)

    converted_rows = [convert(r) for r in rows]

    pg_cur.execute(f"SET search_path TO {SCHEMA}")
    pg_cur.executemany(
        f'INSERT INTO "{tname}" ({cols}) VALUES ({placeholders})',
        converted_rows,
    )
    pg_cur.connection.commit()
    print(f"  {tname}: {len(rows)} rows")


def create_indexes(pg_cur, sqlite_indexes):
    for idx_sql in sqlite_indexes:
        pg_cur.execute(f"SET search_path TO {SCHEMA}")
        try:
            pg_cur.execute(idx_sql)
        except Exception as e:
            print(f"  SKIP index: {e}")


def main():
    sq = sqlite3.connect("data/wn.db")
    sq_cur = sq.cursor()

    pg = psycopg2.connect(PG_URL)
    pg_cur = pg.cursor()

    sqlite_tables = get_sqlite_tables(sq_cur)
    sqlite_indexes = get_sqlite_indexes(sq_cur)

    print("Creating schema...")
    create_schema(pg_cur)
    pg.commit()

    print("Creating tables...")
    create_tables(pg_cur, sqlite_tables)
    pg.commit()

    print("Copying data...")
    for tname in TABLES:
        if tname not in sqlite_tables:
            continue
        copy_table(sq_cur, pg_cur, tname)

    print("Creating indexes...")
    create_indexes(pg_cur, sqlite_indexes)
    pg.commit()

    sq.close()
    pg.close()
    print("Done.")


if __name__ == "__main__":
    main()
