"""Create fresh, independent Abakus v2/v3 migration acceptance profiles."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sqlite3
from pathlib import Path
from typing import Any


ACCEPTANCE_DIR = Path(__file__).resolve().parent
PROFILE_NAMES = ("v2", "v3", "v3-minimal", "v2-failure", "v3-lock", "future-v5")
BASE_TABLES = (
    "accounts",
    "statements",
    "categories",
    "rules",
    "transactions",
    "settings",
    "rule_sources",
    "net_log",
)
PRESERVED_NOTE = "Migrácia zachová Ľúbim čučoriedky.\nDruhý riadok 🙂"

SCHEMA_V2 = """
PRAGMA foreign_keys = ON;
CREATE TABLE accounts (id INTEGER PRIMARY KEY, iban TEXT NOT NULL UNIQUE, kind TEXT NOT NULL CHECK (kind IN ('personal','business')), label TEXT NOT NULL DEFAULT '', has_password INTEGER NOT NULL DEFAULT 0);
CREATE TABLE statements (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id), number INTEGER NOT NULL, period_start TEXT NOT NULL, period_end TEXT NOT NULL, opening_cents INTEGER, closing_cents INTEGER, checksum_status TEXT NOT NULL, checksum_off_by INTEGER, file_hash TEXT NOT NULL UNIQUE, imported_at TEXT NOT NULL DEFAULT (datetime('now')));
CREATE TABLE categories (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES categories(id), name TEXT NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('expense','income')), sort INTEGER NOT NULL DEFAULT 0, system INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0, UNIQUE (parent_id, name));
CREATE TABLE rules (id INTEGER PRIMARY KEY, match_kind TEXT NOT NULL CHECK (match_kind IN ('exact','merchant','counterparty_account','seed')), key TEXT NOT NULL, place TEXT NOT NULL DEFAULT '', category_id INTEGER NOT NULL REFERENCES categories(id), created_at TEXT NOT NULL DEFAULT (datetime('now')), hit_count INTEGER NOT NULL DEFAULT 0, UNIQUE (match_kind, key, place));
CREATE TABLE transactions (id INTEGER PRIMARY KEY, statement_id INTEGER NOT NULL REFERENCES statements(id), account_id INTEGER NOT NULL REFERENCES accounts(id), fingerprint TEXT NOT NULL UNIQUE, posted_date TEXT NOT NULL, tx_date TEXT NOT NULL, kind TEXT NOT NULL, amount_cents INTEGER NOT NULL, orig_amount_cents INTEGER, orig_currency TEXT, rate_micros INTEGER, merchant_raw TEXT NOT NULL, merchant_norm TEXT NOT NULL, place TEXT, place_norm TEXT, counterparty_name TEXT, counterparty_iban TEXT, reference TEXT, card_last4 TEXT, raw_block TEXT NOT NULL, category_id INTEGER REFERENCES categories(id), status TEXT NOT NULL CHECK (status IN ('transfer','confirmed','suggested','unassigned')), rule_id INTEGER REFERENCES rules(id), source TEXT NOT NULL DEFAULT 'none');
CREATE INDEX tx_date_idx ON transactions(tx_date);
CREATE INDEX tx_status_idx ON transactions(status);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE UNIQUE INDEX categories_top_uniq ON categories(name) WHERE parent_id IS NULL;
CREATE TABLE rule_sources (
 rule_id INTEGER NOT NULL REFERENCES rules(id) ON DELETE CASCADE,
 transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
 statement_id INTEGER NOT NULL REFERENCES statements(id) ON DELETE CASCADE,
 created_at TEXT NOT NULL DEFAULT (datetime('now')),
 PRIMARY KEY (rule_id, transaction_id)
);
CREATE INDEX rule_sources_stmt_idx ON rule_sources(statement_id);
CREATE TABLE net_log (id INTEGER PRIMARY KEY, started_at TEXT NOT NULL, url TEXT NOT NULL, status TEXT NOT NULL, duration_ms INTEGER NOT NULL, bytes_in INTEGER NOT NULL);
"""

ACCOUNTS = [
    (1, "SK4411000000000012345678", "personal", "Migračný osobný účet", 0),
    (2, "SK3711000000000098765432", "business", "Migračný firemný účet", 0),
]

STATEMENTS = [
    (1, 1, 1, "2026-01-01", "2026-03-31", 100_000, 92_000, "ok", None, "migration-s1", "2026-04-01 08:00:00"),
    (2, 2, 1, "2026-01-01", "2026-03-31", 200_000, 196_000, "ok", None, "migration-s2", "2026-04-01 09:00:00"),
]

LEGACY_CATEGORIES = [
    (10, None, "Predplatné", "expense", 10, 0, 0),
    (11, 10, "Apple", "expense", 11, 0, 0),
    (12, None, "Iné", "expense", 20, 0, 0),
]
MINIMAL_CATEGORIES = [(12, None, "Iné", "expense", 20, 0, 0)]
LEGACY_RULES = [
    (20, "seed", "spotify", "", 11, "2025-01-01 00:00:00", 2),
    (21, "merchant", "learned migration", "", 12, "2026-01-01 00:00:00", 1),
]
MINIMAL_RULES = [(21, "merchant", "learned migration", "", 12, "2026-01-01 00:00:00", 1)]


def transaction(
    tx_id: int,
    statement_id: int,
    account_id: int,
    date: str,
    merchant: str,
    amount_cents: int,
    category_id: int | None,
    status: str,
    rule_id: int | None,
    source: str,
) -> tuple[Any, ...]:
    return (
        tx_id,
        statement_id,
        account_id,
        f"migration-fp-{tx_id}",
        date,
        date,
        "card",
        amount_cents,
        None,
        None,
        None,
        merchant,
        merchant.casefold(),
        "Bratislava",
        "bratislava",
        None,
        None,
        f"MIGRATION-{tx_id}",
        "4242",
        f"SYNTHETIC MIGRATION ROW {tx_id}",
        category_id,
        status,
        rule_id,
        source,
    )


LEGACY_TRANSACTIONS = [
    transaction(1001, 1, 1, "2026-01-10", "Spotify", -999, 11, "suggested", 20, "seed"),
    transaction(1002, 1, 1, "2026-02-10", "Spotify", -999, 11, "confirmed", 20, "seed"),
    transaction(1003, 1, 1, "2026-03-10", "Learned migration", -2500, 12, "confirmed", 21, "learned"),
    transaction(2001, 2, 2, "2026-03-11", "Unassigned migration", -4000, None, "unassigned", None, "none"),
]
MINIMAL_TRANSACTIONS = [LEGACY_TRANSACTIONS[2], LEGACY_TRANSACTIONS[3]]


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


def populate_profile(connection: sqlite3.Connection, profile: str) -> dict[str, Any]:
    minimal = profile == "v3-minimal"
    version = 2 if profile in {"v2", "v2-failure"} else 3
    if profile == "future-v5":
        version = 5
    connection.executescript(SCHEMA_V2)
    if version >= 3:
        connection.execute("ALTER TABLE transactions ADD COLUMN note TEXT NOT NULL DEFAULT ''")
    connection.executemany("INSERT INTO accounts VALUES (?,?,?,?,?)", ACCOUNTS)
    connection.executemany("INSERT INTO statements VALUES (?,?,?,?,?,?,?,?,?,?,?)", STATEMENTS)
    categories = MINIMAL_CATEGORIES if minimal else LEGACY_CATEGORIES
    rules = MINIMAL_RULES if minimal else LEGACY_RULES
    transactions = MINIMAL_TRANSACTIONS if minimal else LEGACY_TRANSACTIONS
    connection.executemany("INSERT INTO categories VALUES (?,?,?,?,?,?,?)", categories)
    connection.executemany("INSERT INTO rules VALUES (?,?,?,?,?,?,?)", rules)
    placeholders = ",".join("?" for _ in range(25 if version >= 3 else 24))
    rows = [(*row, PRESERVED_NOTE if row[0] == 1003 else "") for row in transactions] if version >= 3 else transactions
    connection.executemany(f"INSERT INTO transactions VALUES ({placeholders})", rows)
    connection.execute("INSERT INTO rule_sources VALUES (21,1003,1,'2026-03-10 12:00:00')")
    connection.executemany(
        "INSERT INTO settings(key,value) VALUES (?,?)",
        [("schema_version", str(version)), ("check_updates", "false"), ("migration_marker", profile)],
    )
    if profile == "v2-failure":
        connection.executescript(
            """
            CREATE TRIGGER migration_oracle_reject_version
            BEFORE UPDATE OF value ON settings
            WHEN OLD.key='schema_version'
            BEGIN
              SELECT RAISE(ABORT, 'synthetic migration failure oracle');
            END;
            """
        )
    return {
        "version": version,
        "minimal": minimal,
        "categories": len(categories),
        "rules": len(rules),
        "transactions": len(transactions),
    }


def create_profile(root: Path, profile: str) -> dict[str, Any]:
    profile_root = root / profile
    data_dir = profile_root / "LocalAppData" / "Abakus"
    data_dir.mkdir(parents=True)
    db_path = data_dir / "abakus.db"
    connection = sqlite3.connect(db_path)
    try:
        connection.execute("PRAGMA journal_mode=WAL")
        shape = populate_profile(connection, profile)
        connection.commit()
        assert connection.execute("PRAGMA integrity_check").fetchone() == ("ok",)
        assert connection.execute("PRAGMA foreign_key_check").fetchall() == []
        initial_tables = {table: table_state(connection, table) for table in BASE_TABLES}
        initial_schema = schema_state(connection)
    finally:
        connection.close()
    expectation = {
        "post_version": 4 if profile not in {"v2-failure", "future-v5"} else shape["version"],
        "migration_should_succeed": profile not in {"v2-failure", "future-v5"},
        "note_1003_after": PRESERVED_NOTE if shape["version"] >= 3 else "",
        "spotify_repair": None if shape["minimal"] else {
            "new_category": "Predplatné/Spotify",
            "seed_rule_id": 20,
            "open_transaction_id": 1001,
            "confirmed_transaction_id": 1002,
            "apple_category_id": 11,
        },
        "minimal_has_no_spotify_signature": shape["minimal"],
    }
    oracle = {
        "fixture": "independent synthetic migration oracle",
        "profile": profile,
        "data_dir": str(data_dir.resolve()),
        "db_path": str(db_path.resolve()),
        "schema_version_before": shape["version"],
        "has_note_before": shape["version"] >= 3,
        "counts_before": {
            "accounts": len(ACCOUNTS),
            "statements": len(STATEMENTS),
            "categories": shape["categories"],
            "rules": shape["rules"],
            "transactions": shape["transactions"],
            "rule_sources": 1,
        },
        "initial_schema": initial_schema,
        "initial_tables": initial_tables,
        "db_sha256_before": sha256_bytes(db_path.read_bytes()),
        "expectation": expectation,
        "safety": {"synthetic_only": True, "private_fixtures_read": False, "network_used": False, "pid": os.getpid()},
    }
    oracle_path = profile_root / "oracle.json"
    oracle_path.write_text(json.dumps(oracle, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return {"profile": profile, "data_dir": oracle["data_dir"], "db_path": oracle["db_path"], "oracle_path": str(oracle_path.resolve())}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    root = args.output.resolve()
    try:
        root.relative_to(ACCEPTANCE_DIR)
    except ValueError as error:
        raise SystemExit(f"output must be inside {ACCEPTANCE_DIR}: {root}") from error
    if root.exists():
        raise SystemExit(f"refusing existing output: {root}")
    if "private" in {part.casefold() for part in root.parts}:
        raise SystemExit(f"refusing private path: {root}")
    root.mkdir()
    profiles = [create_profile(root, profile) for profile in PROFILE_NAMES]
    manifest = {"fixture": "Abakus v2/v3/failure migration profiles", "profiles": profiles}
    (root / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
