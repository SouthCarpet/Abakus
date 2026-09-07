"""Hold a synthetic profile's SQLite write lock until stdin closes."""

from __future__ import annotations

import argparse
import json
import sqlite3
import sys
from pathlib import Path


ACCEPTANCE_DIR = Path(__file__).resolve().parent


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile-root", required=True, type=Path)
    args = parser.parse_args()
    root = args.profile_root.resolve()
    try:
        root.relative_to(ACCEPTANCE_DIR)
    except ValueError as error:
        raise SystemExit(f"profile must be inside {ACCEPTANCE_DIR}: {root}") from error
    if "private" in {part.casefold() for part in root.parts}:
        raise SystemExit(f"refusing private path: {root}")
    oracle = json.loads((root / "oracle.json").read_text(encoding="utf-8"))
    assert oracle["profile"] == "v3-lock"
    assert oracle["safety"]["synthetic_only"] is True
    db_path = Path(oracle["db_path"]).resolve()
    assert db_path == (root / "LocalAppData" / "Abakus" / "abakus.db").resolve()
    connection = sqlite3.connect(db_path)
    try:
        connection.execute("BEGIN IMMEDIATE")
        print(json.dumps({"ready": True, "profile": "v3-lock", "db_path": str(db_path)}, ensure_ascii=False), flush=True)
        sys.stdin.readline()
        connection.rollback()
    finally:
        connection.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
