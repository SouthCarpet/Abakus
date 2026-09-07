"""Inspect one synthetic migration profile without application helpers."""

from __future__ import annotations

import argparse
import hashlib
import json
import sqlite3
from pathlib import Path
from typing import Any


ACCEPTANCE_DIR = Path(__file__).resolve().parent


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def table_state(connection: sqlite3.Connection, table: str) -> dict[str, Any]:
    columns = [row[1] for row in connection.execute(f'PRAGMA table_info("{table}")')]
    order = ", ".join(f'"{column}"' for column in columns)
    rows = connection.execute(f'SELECT * FROM "{table}" ORDER BY {order}').fetchall()
    payload = json.dumps(rows, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    return {"columns": columns, "count": len(rows), "sha256": sha256_bytes(payload)}


def schema_state(connection: sqlite3.Connection) -> dict[str, Any]:
    rows = connection.execute(
        "SELECT type,name,tbl_name,COALESCE(sql,'') FROM sqlite_master "
        "WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name"
    ).fetchall()
    payload = json.dumps(rows, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    return {"count": len(rows), "sha256": sha256_bytes(payload), "objects": [row[1] for row in rows]}


def scalar(connection: sqlite3.Connection, sql: str, params: tuple[Any, ...] = ()) -> Any:
    row = connection.execute(sql, params).fetchone()
    if row is None:
        raise AssertionError(f"query returned no row: {sql}")
    return row[0]


def inspect(connection: sqlite3.Connection) -> dict[str, Any]:
    tables = [
        row[0]
        for row in connection.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )
    ]
    return {
        "integrity": scalar(connection, "PRAGMA integrity_check"),
        "foreign_key_errors": connection.execute("PRAGMA foreign_key_check").fetchall(),
        "schema_version": int(scalar(connection, "SELECT value FROM settings WHERE key='schema_version'")),
        "schema": schema_state(connection),
        "tables": {table: table_state(connection, table) for table in tables},
    }


def assert_initial(connection: sqlite3.Connection, oracle: dict[str, Any], phase: str) -> None:
    current = inspect(connection)
    assert current["schema_version"] == oracle["schema_version_before"]
    assert current["schema"] == oracle["initial_schema"]
    for table, expected in oracle["initial_tables"].items():
        assert current["tables"][table] == expected, (table, current["tables"][table], expected)
    note_columns = current["tables"]["transactions"]["columns"]
    assert ("note" in note_columns) is oracle["has_note_before"]
    if phase == "failed":
        assert oracle["profile"] in {"v2-failure", "v3-lock", "future-v5"}


def category_id(connection: sqlite3.Connection, parent: str, child: str) -> int | None:
    row = connection.execute(
        "SELECT c.id FROM categories c JOIN categories p ON p.id=c.parent_id "
        "WHERE p.name=?1 AND c.name=?2 ORDER BY c.id",
        (parent, child),
    ).fetchall()
    assert len(row) <= 1, row
    return row[0][0] if row else None


def assert_migrated(connection: sqlite3.Connection, oracle: dict[str, Any]) -> None:
    current = inspect(connection)
    assert current["schema_version"] == 4
    assert "note" in current["tables"]["transactions"]["columns"]
    assert {"recurring_decisions", "recurring_members"}.issubset(current["tables"])
    expected = oracle["counts_before"]
    for table in ("accounts", "statements", "transactions", "rules", "rule_sources"):
        assert current["tables"][table]["count"] == expected[table], (table, current["tables"][table]["count"], expected[table])
    assert scalar(connection, "SELECT value FROM settings WHERE key='migration_marker'") == oracle["profile"]
    preserved = connection.execute(
        "SELECT category_id,status,rule_id,source,note FROM transactions WHERE id=1003"
    ).fetchone()
    assert preserved == (12, "confirmed", 21, "learned", oracle["expectation"]["note_1003_after"]), preserved
    repair = oracle["expectation"]["spotify_repair"]
    if repair is None:
        assert scalar(connection, "SELECT COUNT(*) FROM categories WHERE name='Spotify'") == 0
        assert scalar(connection, "SELECT COUNT(*) FROM rules WHERE id=20") == 0
        return
    spotify_id = category_id(connection, "Predplatné", "Spotify")
    assert spotify_id is not None
    assert scalar(connection, "SELECT category_id FROM rules WHERE id=20") == spotify_id
    open_row = connection.execute("SELECT category_id,status,rule_id,source FROM transactions WHERE id=1001").fetchone()
    confirmed_row = connection.execute("SELECT category_id,status,rule_id,source FROM transactions WHERE id=1002").fetchone()
    assert open_row == (spotify_id, "suggested", 20, "seed"), open_row
    assert confirmed_row == (11, "confirmed", 20, "seed"), confirmed_row
    assert scalar(
        connection,
        "SELECT COUNT(*) FROM categories c JOIN categories p ON p.id=c.parent_id "
        "WHERE p.name='Predplatné' AND c.name='Spotify'",
    ) == 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile-root", required=True, type=Path)
    parser.add_argument("--phase", required=True, choices=("before", "migrated", "failed"))
    parser.add_argument("--record", type=Path)
    parser.add_argument("--compare", type=Path)
    args = parser.parse_args()
    if args.record and args.compare:
        raise SystemExit("--record and --compare are mutually exclusive")
    root = args.profile_root.resolve()
    try:
        root.relative_to(ACCEPTANCE_DIR)
    except ValueError as error:
        raise SystemExit(f"profile must be inside {ACCEPTANCE_DIR}: {root}") from error
    if "private" in {part.casefold() for part in root.parts}:
        raise SystemExit(f"refusing private path: {root}")
    oracle = json.loads((root / "oracle.json").read_text(encoding="utf-8"))
    assert oracle["safety"]["synthetic_only"] is True
    db_path = Path(oracle["db_path"]).resolve()
    assert db_path == (root / "LocalAppData" / "Abakus" / "abakus.db").resolve()
    connection = sqlite3.connect(f"file:{db_path.as_posix()}?mode=ro", uri=True)
    try:
        assert scalar(connection, "PRAGMA integrity_check") == "ok"
        assert connection.execute("PRAGMA foreign_key_check").fetchall() == []
        if args.phase == "migrated":
            assert oracle["expectation"]["migration_should_succeed"]
            assert_migrated(connection, oracle)
        else:
            assert_initial(connection, oracle, args.phase)
        state = inspect(connection)
    finally:
        connection.close()
    if args.record:
        try:
            args.record.resolve().relative_to(ACCEPTANCE_DIR)
        except ValueError as error:
            raise SystemExit(f"snapshot must be inside {ACCEPTANCE_DIR}: {args.record.resolve()}") from error
        if args.record.exists():
            raise SystemExit(f"refusing existing snapshot: {args.record}")
        args.record.parent.mkdir(parents=True, exist_ok=True)
        args.record.write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if args.compare:
        try:
            args.compare.resolve().relative_to(ACCEPTANCE_DIR)
        except ValueError as error:
            raise SystemExit(f"comparison snapshot must be inside {ACCEPTANCE_DIR}: {args.compare.resolve()}") from error
        assert state == json.loads(args.compare.read_text(encoding="utf-8")), "database changed across reopen"
    result = {
        "profile": oracle["profile"],
        "phase": args.phase,
        "schema_version": state["schema_version"],
        "integrity": state["integrity"],
        "foreign_key_errors": state["foreign_key_errors"],
        "state_recorded": str(args.record.resolve()) if args.record else None,
        "state_compared": str(args.compare.resolve()) if args.compare else None,
    }
    print(json.dumps(result, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
