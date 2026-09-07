CREATE TABLE IF NOT EXISTS accounts (id INTEGER PRIMARY KEY, iban TEXT NOT NULL UNIQUE, kind TEXT NOT NULL CHECK (kind IN ('personal','business')), label TEXT NOT NULL DEFAULT '', has_password INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS statements (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id), number INTEGER NOT NULL, period_start TEXT NOT NULL, period_end TEXT NOT NULL, opening_cents INTEGER, closing_cents INTEGER, checksum_status TEXT NOT NULL, checksum_off_by INTEGER, file_hash TEXT NOT NULL UNIQUE, imported_at TEXT NOT NULL DEFAULT (datetime('now')));
-- No UNIQUE (account_id, number, period_end): a re-export of the same statement under a new file hash must fall through to fingerprint dedup, not error (review blocker 2026-08-30).
CREATE TABLE IF NOT EXISTS categories (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES categories(id), name TEXT NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('expense','income')), sort INTEGER NOT NULL DEFAULT 0, system INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0, UNIQUE (parent_id, name));
-- place is NOT NULL with '' meaning "no place": SQLite treats NULLs as distinct in UNIQUE, so a NULL place would let ON CONFLICT insert endless duplicates (review blocker 2026-08-30). The repo layer maps '' <-> None.
CREATE TABLE IF NOT EXISTS rules (id INTEGER PRIMARY KEY, match_kind TEXT NOT NULL CHECK (match_kind IN ('exact','merchant','counterparty_account','seed')), key TEXT NOT NULL, place TEXT NOT NULL DEFAULT '', category_id INTEGER NOT NULL REFERENCES categories(id), created_at TEXT NOT NULL DEFAULT (datetime('now')), hit_count INTEGER NOT NULL DEFAULT 0, UNIQUE (match_kind, key, place));
-- note is free user text (0.1.2): exact bytes incl. newlines, max 2000 Unicode
-- code points, enforced in store::notes, never trimmed. classify/reclassify
-- never write this column, so a learned re-suggestion can never touch it.
CREATE TABLE IF NOT EXISTS transactions (id INTEGER PRIMARY KEY, statement_id INTEGER NOT NULL REFERENCES statements(id), account_id INTEGER NOT NULL REFERENCES accounts(id), fingerprint TEXT NOT NULL UNIQUE, posted_date TEXT NOT NULL, tx_date TEXT NOT NULL, kind TEXT NOT NULL, amount_cents INTEGER NOT NULL, orig_amount_cents INTEGER, orig_currency TEXT, rate_micros INTEGER, merchant_raw TEXT NOT NULL, merchant_norm TEXT NOT NULL, place TEXT, place_norm TEXT, counterparty_name TEXT, counterparty_iban TEXT, reference TEXT, card_last4 TEXT, raw_block TEXT NOT NULL, category_id INTEGER REFERENCES categories(id), status TEXT NOT NULL CHECK (status IN ('transfer','confirmed','suggested','unassigned')), rule_id INTEGER REFERENCES rules(id), source TEXT NOT NULL DEFAULT 'none', note TEXT NOT NULL DEFAULT '');
CREATE INDEX IF NOT EXISTS tx_date_idx ON transactions(tx_date);
CREATE INDEX IF NOT EXISTS tx_status_idx ON transactions(status);
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
-- Same NULL-distinct trap for top-level categories: UNIQUE (parent_id, name) does not fire when parent_id IS NULL.
CREATE UNIQUE INDEX IF NOT EXISTS categories_top_uniq ON categories(name) WHERE parent_id IS NULL;
-- A17/F1: provenance for LEARNED rules. One row per (rule, transaction that produced it);
-- statement_id is denormalized so "which rules did this statement produce" is answerable
-- before its transactions are deleted. Seed rules never get rows here, so they can never be
-- selected for deletion. ON DELETE CASCADE keeps the table clean if a caller ever deletes
-- transactions without going through delete_statement.
CREATE TABLE IF NOT EXISTS rule_sources (
  rule_id INTEGER NOT NULL REFERENCES rules(id) ON DELETE CASCADE,
  transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
  statement_id INTEGER NOT NULL REFERENCES statements(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (rule_id, transaction_id)
);
CREATE INDEX IF NOT EXISTS rule_sources_stmt_idx ON rule_sources(statement_id);
-- Spec A14b: every outbound request net::audited_get makes, plus net::sample_connections observations.
CREATE TABLE IF NOT EXISTS net_log (id INTEGER PRIMARY KEY, started_at TEXT NOT NULL, url TEXT NOT NULL, status TEXT NOT NULL, duration_ms INTEGER NOT NULL, bytes_in INTEGER NOT NULL);
