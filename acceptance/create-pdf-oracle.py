"""Create the independent synthetic Abakus v4 PDF/native acceptance profile.

The fixture values and expected results come from the accepted P01-P17 and
R01-R16 rubrics. The script uses only Python's standard sqlite3 module. It
refuses every existing output path and every path outside this worktree's
acceptance directory.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sqlite3
import unicodedata
from pathlib import Path
from typing import Any


ACCEPTANCE_DIR = Path(__file__).resolve().parent

SCHEMA_V4 = """
PRAGMA foreign_keys = ON;
CREATE TABLE accounts (id INTEGER PRIMARY KEY, iban TEXT NOT NULL UNIQUE, kind TEXT NOT NULL CHECK (kind IN ('personal','business')), label TEXT NOT NULL DEFAULT '', has_password INTEGER NOT NULL DEFAULT 0);
CREATE TABLE statements (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id), number INTEGER NOT NULL, period_start TEXT NOT NULL, period_end TEXT NOT NULL, opening_cents INTEGER, closing_cents INTEGER, checksum_status TEXT NOT NULL, checksum_off_by INTEGER, file_hash TEXT NOT NULL UNIQUE, imported_at TEXT NOT NULL DEFAULT (datetime('now')));
CREATE TABLE categories (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES categories(id), name TEXT NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('expense','income')), sort INTEGER NOT NULL DEFAULT 0, system INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0, UNIQUE (parent_id, name));
CREATE TABLE rules (id INTEGER PRIMARY KEY, match_kind TEXT NOT NULL CHECK (match_kind IN ('exact','merchant','counterparty_account','seed')), key TEXT NOT NULL, place TEXT NOT NULL DEFAULT '', category_id INTEGER NOT NULL REFERENCES categories(id), created_at TEXT NOT NULL DEFAULT (datetime('now')), hit_count INTEGER NOT NULL DEFAULT 0, UNIQUE (match_kind, key, place));
CREATE TABLE transactions (id INTEGER PRIMARY KEY, statement_id INTEGER NOT NULL REFERENCES statements(id), account_id INTEGER NOT NULL REFERENCES accounts(id), fingerprint TEXT NOT NULL UNIQUE, posted_date TEXT NOT NULL, tx_date TEXT NOT NULL, kind TEXT NOT NULL, amount_cents INTEGER NOT NULL, orig_amount_cents INTEGER, orig_currency TEXT, rate_micros INTEGER, merchant_raw TEXT NOT NULL, merchant_norm TEXT NOT NULL, place TEXT, place_norm TEXT, counterparty_name TEXT, counterparty_iban TEXT, reference TEXT, card_last4 TEXT, raw_block TEXT NOT NULL, category_id INTEGER REFERENCES categories(id), status TEXT NOT NULL CHECK (status IN ('transfer','confirmed','suggested','unassigned')), rule_id INTEGER REFERENCES rules(id), source TEXT NOT NULL DEFAULT 'none', note TEXT NOT NULL DEFAULT '');
CREATE INDEX tx_date_idx ON transactions(tx_date);
CREATE INDEX tx_status_idx ON transactions(status);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE UNIQUE INDEX categories_top_uniq ON categories(name) WHERE parent_id IS NULL;
CREATE TABLE rule_sources (rule_id INTEGER NOT NULL REFERENCES rules(id) ON DELETE CASCADE, transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE, statement_id INTEGER NOT NULL REFERENCES statements(id) ON DELETE CASCADE, created_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY (rule_id, transaction_id));
CREATE INDEX rule_sources_stmt_idx ON rule_sources(statement_id);
CREATE TABLE net_log (id INTEGER PRIMARY KEY, started_at TEXT NOT NULL, url TEXT NOT NULL, status TEXT NOT NULL, duration_ms INTEGER NOT NULL, bytes_in INTEGER NOT NULL);
CREATE TABLE recurring_decisions (
 id INTEGER PRIMARY KEY,
 account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 group_key TEXT NOT NULL,
 scope TEXT NOT NULL CHECK(scope IN ('group','selected')),
 mode TEXT NOT NULL CHECK(mode IN ('confirmed','ignored')),
 cadence TEXT CHECK(cadence IN ('monthly','quarterly','yearly')),
 anchor_date TEXT,
 identity_json TEXT NOT NULL DEFAULT '{}',
 updated_at TEXT NOT NULL DEFAULT (datetime('now')),
 CHECK((mode='confirmed' AND cadence IS NOT NULL AND anchor_date IS NOT NULL)
    OR (mode='ignored' AND cadence IS NULL AND anchor_date IS NULL))
);
CREATE UNIQUE INDEX recurring_group_unique ON recurring_decisions(group_key) WHERE scope='group';
CREATE TABLE recurring_members (
 decision_id INTEGER NOT NULL REFERENCES recurring_decisions(id) ON DELETE CASCADE,
 fingerprint TEXT NOT NULL UNIQUE,
 PRIMARY KEY(decision_id,fingerprint)
);
"""

ACCOUNTS = [
    (1, "SK4411000000000012345678", "personal", "PDF mesačný účet", 0),
    (2, "SK3112000000198742637541", "personal", "Osobný bez výpisov", 0),
    (3, "SK3711000000000098765432", "business", "Firemný vzor", 0),
    (4, "SK3212000000198742637541", "personal", "Prázdny PDF účet", 0),
    (5, "SK0281800000007000000001", "personal", "Dlhý text a refundácie", 0),
    (6, "SK0902000000005230000001", "business", "Veľký PDF účet", 0),
    (7, "SK8911000000000055555555", "personal", "Pravidelné platby", 0),
]

CATEGORIES = [
    (10, None, "Výdavky PDF", "expense", 10, 0, 0),
    (11, None, "Príjmy PDF", "income", 20, 0, 0),
    (12, None, "Predplatné", "expense", 30, 0, 0),
    (13, 12, "Spotify", "expense", 31, 0, 0),
    (14, 12, "Apple", "expense", 32, 0, 0),
    *[(category_id, 10, f"Kategória {category_id - 200:02d}", "expense", category_id, 0, 0) for category_id in range(201, 213)],
    (301, 10, "Veľký výpis", "expense", 301, 0, 0),
    (401, 10, "Presmerovaný seed", "expense", 401, 0, 0),
]

STATEMENTS = [
    (101, 1, 1, "2024-01-01", "2024-02-14", 500_000, 490_000, "ok", None, "pdf-s101", "2024-02-15 08:00:00"),
    (102, 1, 2, "2024-02-15", "2024-06-30", 490_000, 478_700, "ok", None, "pdf-s102", "2024-07-01 08:00:00"),
    (103, 1, 3, "2024-02-01", "2024-02-29", 490_000, None, "not_verifiable", None, "pdf-s103", "2024-03-01 08:00:00"),
    (104, 1, 4, "2024-08-01", "2024-08-31", 478_700, 478_600, "off", 1, "pdf-s104", "2024-09-01 08:00:00"),
    (105, 1, 5, "2025-02-05", "2025-02-01", 478_600, 478_500, "ok", None, "pdf-s105-invalid", "2025-02-06 08:00:00"),
    (301, 3, 1, "2024-02-01", "2024-02-29", 1_000_000, 1_475_000, "ok", None, "pdf-s301", "2024-03-01 09:00:00"),
    (401, 4, 1, "2024-01-01", "2024-12-31", 100_000, 100_000, "ok", None, "pdf-s401", "2025-01-01 08:00:00"),
    (501, 5, 1, "2025-06-01", "2025-06-30", 300_000, 294_227, "ok", None, "pdf-s501", "2025-07-01 08:00:00"),
    (601, 6, 1, "2023-01-01", "2023-12-31", 700_000, 695_000, "ok", None, "pdf-s601", "2024-01-01 08:00:00"),
    (701, 7, 1, "2026-09-01", "2026-09-30", 200_000, 200_000, "ok", None, "pdf-s701", "2026-10-01 08:00:00"),
    (801, 7, 1, "2026-01-01", "2026-07-31", 900_000, 882_000, "ok", None, "pdf-s801", "2026-08-01 08:00:00"),
    (802, 7, 2, "2026-08-01", "2026-09-30", 882_000, 1_630_000, "ok", None, "pdf-s802", "2026-10-01 08:00:00"),
]

RULES = [
    (401, "exact", "pdf-oracle-expense", "", 201, "2024-02-27 12:00:00", 1),
    (402, "seed", "spotify", "", 13, "2024-01-01 00:00:00", 3),
    (403, "merchant", "learned oracle", "", 202, "2024-02-20 12:00:00", 2),
]


def transaction(
    tx_id: int,
    statement_id: int,
    account_id: int,
    tx_date: str,
    kind: str,
    amount_cents: int,
    merchant: str,
    category_id: int | None,
    status: str = "confirmed",
    *,
    rule_id: int | None = None,
    source: str = "manual",
    note: str = "",
    posted_date: str | None = None,
    orig_amount_cents: int | None = None,
    orig_currency: str | None = None,
    place: str | None = "Bratislava",
    card_last4: str | None = None,
) -> tuple[Any, ...]:
    normalized = unicodedata.normalize("NFC", merchant).casefold()
    return (
        tx_id,
        statement_id,
        account_id,
        f"pdf-oracle-fp-{tx_id}",
        posted_date or tx_date,
        tx_date,
        kind,
        amount_cents,
        orig_amount_cents,
        orig_currency,
        None,
        merchant,
        normalized,
        place,
        place.casefold() if place else None,
        None,
        None,
        f"SYNTH-{tx_id}",
        card_last4,
        f"SYNTHETIC PDF ORACLE ROW {tx_id}",
        category_id,
        status,
        rule_id,
        source,
        note,
    )


def month_transactions() -> list[tuple[Any, ...]]:
    return [
        transaction(1001, 102, 1, "2024-02-28", "incoming", 100_000, "Príjem 1000", 11),
        transaction(1002, 102, 1, "2024-02-27", "card", -10_000, "Výdavok 100", 201, rule_id=401, source="exact_rule"),
        transaction(1003, 102, 1, "2024-02-26", "refund", 2_500, "Refundácia 25", 201),
        transaction(1004, 102, 1, "2024-02-25", "card", -3_000, "Navrhnuté bez kategórie", None, "suggested"),
        transaction(1005, 102, 1, "2024-02-24", "refund", 500, "Nezaradená refundácia", None, "unassigned"),
        transaction(1006, 102, 1, "2024-02-23", "transfer_out", -20_000, "Odchádzajúci prevod", None, "transfer"),
        transaction(1007, 102, 1, "2024-02-22", "transfer_in", 20_000, "Prichádzajúci prevod", None, "transfer"),
        transaction(1008, 102, 1, "2024-02-21", "card", 0, "Nulová položka", None),
    ]


def account_one_transactions() -> list[tuple[Any, ...]]:
    rows = month_transactions()
    rows.append(transaction(1010, 101, 1, "2024-01-15", "card", -1_200, "Bežný návrh s kategóriou", 202, "suggested", rule_id=403, source="merchant_rule"))
    for offset, year in enumerate(range(1990, 2026)):
        rows.append(transaction(2000 + offset, 101, 1, f"{year:04d}-12-31", "card", -100, f"YEAR-SENTINEL-{year}", 203))
    return rows


def long_text_transactions() -> list[tuple[Any, ...]]:
    final_sentinel = "FINAL-NOTE-SENTINEL"
    long_note = "A" * 1980 + "\n" + final_sentinel
    assert len(long_note) == 2000
    phrase = "Ľúbim čučoriedky. Kŕdeľ ďatľov učí koňa žrať kôru. 1 234,56 €"
    rows = []
    negative_amounts = [-1000, -900, -800, -700, -600, -500, -400, -300, -200, -100, -50]
    for offset, amount in enumerate(negative_amounts):
        merchant = phrase + " (\\) " + "NEPRERUŠOVANÝTEXT" * 18 if offset == 0 else f"Dlhá kategória {offset + 1:02d}"
        note = long_note if offset == 0 else ("Cafe\u0301 NFD" if offset == 2 else "")
        rows.append(transaction(5001 + offset, 501, 5, f"2025-06-{offset + 1:02d}", "card", amount, merchant, 201 + offset, note=note))
    rows.extend(
        [
            transaction(5012, 501, 5, "2025-06-12", "refund", 500, "Refundácia kategórie", 212),
            transaction(5013, 501, 5, "2025-06-13", "card", -700, "Nezaradený výdavok", None, "unassigned"),
            transaction(5014, 501, 5, "2025-06-14", "refund", 200, "Nezaradená refundácia", None),
            transaction(5015, 501, 5, "2025-06-15", "card", 700, "Kladný výdavok", 204),
            transaction(5016, 501, 5, "2025-06-16", "card", -923, "Foreign USD", 205, orig_amount_cents=-1000, orig_currency="USD"),
            transaction(5017, 501, 5, "2025-06-17", "incoming", -900, "Záporný príjem", 11),
            transaction(5018, 501, 5, "2025-06-18", "refund", 300, "Refundácia príjmu 🙂", 11),
            transaction(5019, 501, 5, "2025-06-19", "incoming", 1100, "Kladný nezaradený príjem", None),
        ]
    )
    return rows


def large_transactions() -> list[tuple[Any, ...]]:
    rows = []
    for offset in range(5000):
        day = offset % 365
        month = day // 31 + 1
        safe_month = min(month, 12)
        safe_day = day - (safe_month - 1) * 31 + 1
        if safe_month == 2:
            safe_day = min(safe_day, 28)
        elif safe_month in {4, 6, 9, 11}:
            safe_day = min(safe_day, 30)
        else:
            safe_day = min(safe_day, 31)
        tx_id = 10_000 + offset
        rows.append(transaction(tx_id, 601, 6, f"2023-{safe_month:02d}-{safe_day:02d}", "card", -1, f"LARGE-SENTINEL-{offset + 1:05d}", 301, place=None))
    return rows


def recurring_transactions() -> list[tuple[Any, ...]]:
    rows = [
        transaction(8001, 801, 7, "2026-01-31", "card", -1200, "Mesačný kotviaci obchod", 13, card_last4="1111"),
        transaction(8002, 801, 7, "2026-02-28", "card", -1200, "Mesačný kotviaci obchod", 13, card_last4="1111"),
        transaction(8003, 801, 7, "2026-03-31", "card", -1200, "Mesačný kotviaci obchod", 13, card_last4="1111"),
    ]
    for tx_id, date, cents in [
        (8011, "2026-01-15", -1000),
        (8012, "2026-02-15", -1000),
        (8013, "2026-03-15", -1000),
        (8014, "2026-04-15", -1200),
        (8015, "2026-05-15", -1200),
        (8016, "2026-06-15", -1200),
        (8017, "2026-07-15", -1300),
    ]:
        rows.append(transaction(tx_id, 801, 7, date, "card", cents, "Cena služby", 13, card_last4="2222"))
    rows.extend(
        [
            transaction(8021, 801, 7, "2026-01-10", "card", -10000, "Ručné poistenie", None),
            transaction(8022, 801, 7, "2026-02-10", "card", -500, "Ignorovaná platba", 13),
            transaction(8031, 801, 7, "2026-07-20", "incoming", 250000, "Mesačný príjem", 11),
            transaction(8032, 802, 7, "2026-08-20", "incoming", 250000, "Mesačný príjem", 11),
            transaction(8033, 802, 7, "2026-09-20", "incoming", 250000, "Mesačný príjem", 11),
            transaction(8041, 802, 7, "2026-09-05", "card", -899, "Spotify", 13, "suggested", rule_id=402, source="seed"),
        ]
    )
    return rows


def all_transactions() -> list[tuple[Any, ...]]:
    business = [
        transaction(3001, 301, 3, "2024-02-20", "incoming", 500_000, "Firemný príjem", 11),
        transaction(3002, 301, 3, "2024-02-19", "card", -25_000, "Firemný výdavok", 201),
    ]
    return account_one_transactions() + business + long_text_transactions() + large_transactions() + recurring_transactions()


RECURRING_DECISIONS = [
    (501, 7, "selected-insurance-oracle", "selected", "confirmed", "yearly", "2026-01-10", '{"direction":"expense","currency":null,"name":"Ručné poistenie"}', "2026-09-07 10:00:00"),
    (502, 7, "selected-ignored-oracle", "selected", "ignored", None, None, '{"direction":"expense","currency":null,"name":"Ignorovaná platba"}', "2026-09-07 10:05:00"),
]

RECURRING_MEMBERS = [
    (501, "pdf-oracle-fp-8021"),
    (502, "pdf-oracle-fp-8022"),
]

FAMILIES = {
    "month": {
        "request": {"period": {"kind": "month", "month": "2024-02"}, "scope": {"kind": "account", "account_id": 1}},
        "range": {"from": "2024-02-01", "to": "2024-02-29"},
        "transaction_count": 8,
        "ordered_ids": [1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008],
        "totals": {"income_cents": 100000, "expense_cents": 10000, "net_cents": 90000, "transfer_out_cents": 20000, "transfer_in_cents": 20000, "suggested_count": 1, "unassigned_count": 1},
    },
    "six_months": {
        "request": {"period": {"kind": "six_months", "ending_month": "2024-06"}, "scope": {"kind": "account", "account_id": 1}},
        "range": {"from": "2024-01-01", "to": "2024-06-30"},
        "transaction_count": 9,
        "totals": {"income_cents": 100000, "expense_cents": 11200, "net_cents": 88800, "transfer_out_cents": 20000, "transfer_in_cents": 20000, "suggested_count": 2, "unassigned_count": 1},
    },
    "year": {
        "request": {"period": {"kind": "year", "year": 2024}, "scope": {"kind": "account", "account_id": 1}},
        "range": {"from": "2024-01-01", "to": "2024-12-31"},
        "transaction_count": 10,
        "totals": {"income_cents": 100000, "expense_cents": 11300, "net_cents": 88700, "transfer_out_cents": 20000, "transfer_in_cents": 20000, "suggested_count": 2, "unassigned_count": 1},
    },
    "all_time": {
        "request": {"period": {"kind": "all_time"}, "scope": {"kind": "account", "account_id": 1}},
        "range": {"from": "1990-12-31", "to": "2025-12-31"},
        "transaction_count": 45,
        "totals": {"income_cents": 100000, "expense_cents": 14800, "net_cents": 85200, "transfer_out_cents": 20000, "transfer_in_cents": 20000, "suggested_count": 2, "unassigned_count": 1},
        "year_sentinels": {"first": "YEAR-SENTINEL-1990", "middle": "YEAR-SENTINEL-2008", "last": "YEAR-SENTINEL-2025", "count": 36},
    },
    "empty": {
        "request": {"period": {"kind": "year", "year": 2024}, "scope": {"kind": "account", "account_id": 4}},
        "range": {"from": "2024-01-01", "to": "2024-12-31"},
        "transaction_count": 0,
        "totals": {"income_cents": 0, "expense_cents": 0, "net_cents": 0, "transfer_out_cents": 0, "transfer_in_cents": 0, "suggested_count": 0, "unassigned_count": 0},
    },
    "refunds_long_text": {
        "request": {"period": {"kind": "month", "month": "2025-06"}, "scope": {"kind": "account", "account_id": 5}},
        "range": {"from": "2025-06-01", "to": "2025-06-30"},
        "transaction_count": 19,
        "totals": {"income_cents": 1100, "expense_cents": 5773, "net_cents": -4673, "transfer_out_cents": 0, "transfer_in_cents": 0, "suggested_count": 0, "unassigned_count": 1},
        "required_text": ["Ľúbim čučoriedky", "Café NFD", "FINAL-NOTE-SENTINEL", "[U+1F642]", "-10,00 USD"],
    },
    "large": {
        "request": {"period": {"kind": "year", "year": 2023}, "scope": {"kind": "account", "account_id": 6}},
        "range": {"from": "2023-01-01", "to": "2023-12-31"},
        "transaction_count": 5000,
        "totals": {"income_cents": 0, "expense_cents": 5000, "net_cents": -5000, "transfer_out_cents": 0, "transfer_in_cents": 0, "suggested_count": 0, "unassigned_count": 0},
        "sentinel": {"prefix": "LARGE-SENTINEL-", "first": "LARGE-SENTINEL-00001", "last": "LARGE-SENTINEL-05000", "count": 5000},
    },
}

TABLES = ["accounts", "statements", "categories", "rules", "transactions", "settings", "rule_sources", "recurring_decisions", "recurring_members"]


def table_state(connection: sqlite3.Connection, table: str) -> dict[str, Any]:
    columns = [row[1] for row in connection.execute(f'PRAGMA table_info("{table}")')]
    order = ", ".join(f'"{column}"' for column in columns)
    rows = connection.execute(f'SELECT * FROM "{table}" ORDER BY {order}').fetchall()
    payload = json.dumps(rows, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    return {"count": len(rows), "sha256": hashlib.sha256(payload).hexdigest()}


def populate(connection: sqlite3.Connection) -> None:
    connection.executescript(SCHEMA_V4)
    connection.executemany("INSERT INTO accounts VALUES (?,?,?,?,?)", ACCOUNTS)
    connection.executemany("INSERT INTO categories VALUES (?,?,?,?,?,?,?)", CATEGORIES)
    connection.executemany("INSERT INTO statements VALUES (?,?,?,?,?,?,?,?,?,?,?)", STATEMENTS)
    connection.executemany("INSERT INTO rules VALUES (?,?,?,?,?,?,?)", RULES)
    transactions = all_transactions()
    connection.executemany("INSERT INTO transactions VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", transactions)
    connection.executemany(
        "INSERT INTO settings(key,value) VALUES (?,?)",
        [("schema_version", "4"), ("check_updates", "false"), ("fixture_marker", "pdf-native-v4")],
    )
    connection.executemany(
        "INSERT INTO rule_sources VALUES (?,?,?,?)",
        [(401, 1002, 102, "2024-02-27 12:00:00"), (403, 1010, 101, "2024-01-15 12:00:00")],
    )
    connection.executemany("INSERT INTO recurring_decisions VALUES (?,?,?,?,?,?,?,?,?)", RECURRING_DECISIONS)
    connection.executemany("INSERT INTO recurring_members VALUES (?,?)", RECURRING_MEMBERS)
    connection.commit()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    output = args.output.resolve()
    try:
        output.relative_to(ACCEPTANCE_DIR)
    except ValueError as error:
        raise SystemExit(f"output must be inside {ACCEPTANCE_DIR}: {output}") from error
    if output.exists():
        raise SystemExit(f"refusing existing output: {output}")
    if "private" in {part.casefold() for part in output.parts}:
        raise SystemExit(f"refusing private path: {output}")

    data_dir = output / "LocalAppData" / "Abakus"
    db_path = data_dir / "abakus.db"
    data_dir.mkdir(parents=True)
    connection = sqlite3.connect(db_path)
    try:
        connection.execute("PRAGMA journal_mode = WAL")
        connection.execute("PRAGMA wal_autocheckpoint = 0")
        populate(connection)
        integrity = connection.execute("PRAGMA integrity_check").fetchone()
        foreign_keys = connection.execute("PRAGMA foreign_key_check").fetchall()
        assert integrity == ("ok",), integrity
        assert foreign_keys == [], foreign_keys
        assert connection.execute("SELECT value FROM settings WHERE key='schema_version'").fetchone() == ("4",)
        initial_state = {table: table_state(connection, table) for table in TABLES}
    finally:
        connection.close()

    oracle = {
        "fixture": "independent synthetic Abakus v4 PDF/native oracle",
        "rubric": "P01-P17 and R01-R16, accepted 2026-09-07",
        "today": "2026-09-07",
        "output": str(output),
        "localappdata": str(output / "LocalAppData"),
        "data_dir": str(data_dir),
        "db_path": str(db_path),
        "db_sha256_after_close": hashlib.sha256(db_path.read_bytes()).hexdigest(),
        "schema_version": 4,
        "families": FAMILIES,
        "expected_accounts": [{"id": row[0], "kind": row[2], "label": row[3]} for row in ACCOUNTS],
        "expected_counts": {
            "accounts": 7,
            "statements": 12,
            "categories": 19,
            "rules": 3,
            "transactions": 5082,
            "rule_sources": 2,
            "recurring_decisions": 2,
            "recurring_members": 2,
        },
        "initial_state": initial_state,
        "recurring": {
            "anchor_candidate": {"name": "Mesačný kotviaci obchod", "cadence": "monthly", "anchor_date": "2026-01-31", "next_due": "2026-04-30", "state_at_2026_04_15": "upcoming"},
            "stable_price": {"name": "Cena služby", "previous_cents": 1000, "current_cents": 1200, "delta_cents": 200, "effective_from": "2026-04-15"},
            "persisted_decision_ids": [501, 502],
        },
        "safety": {"synthetic_only": True, "private_fixtures_read": False, "network_used": False, "pid": os.getpid()},
    }
    oracle_path = output / "oracle.json"
    oracle_path.write_text(json.dumps(oracle, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(output), "data_dir": str(data_dir), "db_path": str(db_path), "oracle_path": str(oracle_path)}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
