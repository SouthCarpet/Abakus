//! 078: delete one account and everything only it owns, all or nothing.
//!
//! Same transaction shape as `delete_statement` (`BEGIN IMMEDIATE` /
//! `COMMIT` / `ROLLBACK`), scaled up to every statement of the account at
//! once rather than one. A retained account's own rows, including a
//! historical transfer that used to point at the deleted account, are never
//! touched: this module only ever deletes rows whose `account_id` (directly,
//! or via `statements.account_id`) is the account being removed.
use crate::{Result, Store, StoreError};
use serde::{Deserialize, Serialize};

/// What the confirmation dialog reads before the user decides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountDeletePreview {
    pub account_id: i64,
    pub label: String,
    pub statement_count: i64,
    pub transaction_count: i64,
    pub confirmed_count: i64,
    pub rules_deleted: i64,
    pub has_password: bool,
}

/// What the delete actually removed. Credential removal is a separate,
/// non-transactional step the Tauri command layer runs after this commits
/// (see `src-tauri/src/commands.rs::delete_account`); it is not part of this
/// outcome because a keyring failure must never look like a database failure
/// or vice versa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountDeleteOutcome {
    pub account_id: i64,
    pub statements_deleted: usize,
    pub transactions_deleted: usize,
    pub rules_deleted: usize,
    pub open_rows_reclassified: usize,
}

/// The learned rules an account delete removes: rules produced ONLY by this
/// account's statements, and used by no transaction outside this account.
///
/// This cannot reuse `delete_statement`'s single-statement `DOOMED_RULES`
/// query one statement at a time: that query treats a rule as "not doomed"
/// the moment ANY other statement produced it too, even when that other
/// statement belongs to the SAME account and is also being deleted right
/// now. Scoping the whole query to the account, instead of looping per
/// statement, is what lets a rule two of this account's own statements both
/// taught still get removed together.
const DOOMED_RULES_FOR_ACCOUNT: &str = "SELECT DISTINCT rs.rule_id FROM rule_sources rs \
     JOIN rules r ON r.id = rs.rule_id \
     JOIN statements st ON st.id = rs.statement_id \
     WHERE st.account_id = ?1 AND r.match_kind <> 'seed' \
       AND NOT EXISTS (SELECT 1 FROM rule_sources o JOIN statements os ON os.id = o.statement_id WHERE o.rule_id = rs.rule_id AND os.account_id <> ?1) \
       AND NOT EXISTS (SELECT 1 FROM transactions t WHERE t.rule_id = rs.rule_id AND t.account_id <> ?1)";

impl Store {
    pub fn account_delete_preview(&self, account_id: i64) -> Result<AccountDeletePreview> {
        let account = self.account_by_id(account_id)?.ok_or(StoreError::UnknownAccountId { id: account_id })?;
        let count = |sql: &str| -> Result<i64> { Ok(self.conn.query_row(sql, [account_id], |r| r.get(0))?) };
        Ok(AccountDeletePreview {
            account_id,
            label: account.label,
            statement_count: count("SELECT COUNT(*) FROM statements WHERE account_id = ?1")?,
            transaction_count: count("SELECT COUNT(*) FROM transactions WHERE account_id = ?1")?,
            confirmed_count: count("SELECT COUNT(*) FROM transactions WHERE account_id = ?1 AND status = 'confirmed'")?,
            rules_deleted: count(&format!("SELECT COUNT(*) FROM ({DOOMED_RULES_FOR_ACCOUNT})"))?,
            has_password: account.has_password,
        })
    }

    /// A failure of the `COMMIT` statement itself (not just the delete body)
    /// must roll back too: `execute_batch("COMMIT")?` alone would propagate
    /// the error and leave the transaction open on the connection, so every
    /// later query on it would fail with "cannot start a transaction within
    /// a transaction". Regression:
    /// `a_failed_commit_rolls_back_too_and_leaves_the_connection_usable`.
    pub fn delete_account(&mut self, account_id: i64) -> Result<AccountDeleteOutcome> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.delete_account_tx(account_id) {
            Ok(outcome) => match self.conn.execute_batch("COMMIT") {
                Ok(()) => Ok(outcome),
                Err(e) => {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    Err(e.into())
                }
            },
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    /// Children before parents, same reasoning as `delete_statement`: an
    /// unknown id fails before anything is touched; the doomed rules are read
    /// while their evidence still exists; `rule_sources` goes before the
    /// transactions and statements it points at; transactions go before
    /// statements (releases `transactions.fingerprint` first); the account
    /// row goes last, after nothing still references it. A transfer row on a
    /// RETAINED account is never in any of these tables' `WHERE account_id =
    /// ?1`, so it is never touched, and `reclassify_open` only ever revisits
    /// `suggested`/`unassigned` rows, never `transfer` or `confirmed` ones.
    fn delete_account_tx(&mut self, account_id: i64) -> Result<AccountDeleteOutcome> {
        self.account_by_id(account_id)?.ok_or(StoreError::UnknownAccountId { id: account_id })?;
        let doomed = self.doomed_rule_ids_for_account(account_id)?;
        self.conn.execute(
            "DELETE FROM rule_sources WHERE statement_id IN (SELECT id FROM statements WHERE account_id = ?1)",
            [account_id],
        )?;
        let transactions_deleted = self.conn.execute("DELETE FROM transactions WHERE account_id = ?1", [account_id])?;
        let statements_deleted = self.conn.execute("DELETE FROM statements WHERE account_id = ?1", [account_id])?;
        for rule_id in &doomed {
            self.conn.execute("DELETE FROM rules WHERE id = ?1", [rule_id])?;
        }
        self.conn.execute("DELETE FROM accounts WHERE id = ?1", [account_id])?;
        let open_rows_reclassified = self.reclassify_open()?;
        self.recompute_hit_counts()?;
        Ok(AccountDeleteOutcome { account_id, statements_deleted, transactions_deleted, rules_deleted: doomed.len(), open_rows_reclassified })
    }

    fn doomed_rule_ids_for_account(&self, account_id: i64) -> Result<Vec<i64>> {
        let mut st = self.conn.prepare(DOOMED_RULES_FOR_ACCOUNT)?;
        let rows = st.query_map([account_id], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::AccountKind;
    use rules::RuleKind;

    /// Same technique as `accounts.rs`'s own rollback test: engineer a state
    /// the app itself can never reach (a rule pointing at a category that no
    /// longer exists, via raw SQL with `foreign_keys` briefly off) so that
    /// `delete_account`'s own `reclassify_open` call, which re-scans EVERY
    /// suggested/unassigned row in the database (not just the deleted
    /// account's), hits the dangling reference and fails partway through the
    /// transaction. This proves the whole delete rolls back as one unit: the
    /// account being deleted, its own already-doomed rules, AND a completely
    /// unrelated retained account's row are all still exactly as they were.
    #[test]
    fn a_failed_reclassification_rolls_back_the_whole_account_delete() {
        let mut s = Store::open_in_memory().unwrap();
        let doomed_account = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        let retained_account = s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
        let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();

        // The account being deleted: one row, assigned so its rule is
        // genuinely produced and used by this account alone (a real doomed
        // rule, found the normal way).
        s.insert_rule(RuleKind::Merchant, "widget", None, cat).unwrap();
        s.conn.execute_batch(&format!(
            "INSERT INTO statements (id, account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (1, 1, 1, '2026-06-01', '2026-06-30', 'ok', 'h1'); \
             INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source, rule_id, category_id) \
               VALUES (1, 1, 1, 'fp-widget', '2026-06-01', '2026-06-01', 'card', -500, 'WIDGET', 'widget', 'raw', 'confirmed', 'merchant_rule', (SELECT id FROM rules WHERE key = 'widget'), {cat}); \
             INSERT INTO rule_sources (rule_id, transaction_id, statement_id) SELECT id, 1, 1 FROM rules WHERE key = 'widget';"
        )).unwrap();

        // A completely unrelated rule pointing at the SAME category, used by
        // an UNASSIGNED row on the RETAINED account: `reclassify_open` will
        // try to re-run this row through `classify` during the delete.
        s.insert_rule(RuleKind::Merchant, "acme", None, cat).unwrap();
        s.conn.execute_batch(
            "INSERT INTO statements (id, account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (2, 2, 1, '2026-06-01', '2026-06-30', 'ok', 'h2'); \
             INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source) \
               VALUES (2, 2, 2, 'fp-acme', '2026-06-01', '2026-06-01', 'card', -700, 'ACME', 'acme', 'raw', 'unassigned', 'none');",
        ).unwrap();

        // Delete the category out from under the "acme" rule: the app itself
        // can never reach this (categories are only archived, never deleted),
        // so it needs `foreign_keys` off to set up.
        s.conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
        s.conn.execute("DELETE FROM categories WHERE id = ?1", [cat]).unwrap();
        s.conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();

        let retained_row_before = s.list_transactions(&crate::TxFilter { account_id: Some(retained_account), ..Default::default() }).unwrap();
        let doomed_account_before = s.account_by_id(doomed_account).unwrap();
        let rules_before = s.list_rules().unwrap();

        let e = s.delete_account(doomed_account).unwrap_err();

        assert!(matches!(e, StoreError::Db(_)), "got {e:?}");
        assert_eq!(s.account_by_id(doomed_account).unwrap(), doomed_account_before, "a rolled-back delete must leave the account in place");
        assert_eq!(s.list_rules().unwrap(), rules_before, "no rule may be removed by a rolled-back delete, including the one genuinely doomed");
        assert_eq!(
            s.list_transactions(&crate::TxFilter { account_id: Some(retained_account), ..Default::default() }).unwrap(),
            retained_row_before,
            "an unrelated retained account's row must be untouched by a rolled-back delete"
        );
    }

    /// A REFUND's suggested category comes from `refund_lookup`, i.e. the
    /// most recent CONFIRMED purchase for the same merchant, wherever it
    /// lives (see `crate::import::classify_one`). The confirmed purchase here
    /// is written directly with SQL rather than through `assign`, because
    /// `assign` always also teaches a rule for the merchant, and that rule
    /// would then match the refund directly (Exact/Merchant beat the
    /// `kind_rule` fallback in `classify::classify`), never reaching
    /// `refund_lookup` at all: this test is specifically about the
    /// rule-free evidence path, which `crate::TxFilter`-based tests in
    /// `tests/delete_account.rs` cannot set up without a rule attached.
    #[test]
    fn a_retained_accounts_suggested_refund_is_reclassified_once_its_confirmed_evidence_is_gone() {
        let mut s = Store::open_in_memory().unwrap();
        let business_id = s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
        let personal_id = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
        s.conn.execute_batch(&format!(
            "INSERT INTO statements (id, account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (1, {business_id}, 1, '2026-06-01', '2026-06-30', 'ok', 'h-business'); \
             INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source, category_id) \
               VALUES (1, 1, {business_id}, 'fp-purchase', '2026-05-15', '2026-05-15', 'card', -2000, 'ZUNIVERSUM', 'zuniversum', 'raw', 'confirmed', 'none', {cat});"
        )).unwrap();

        let personal_refund_text = "Osobný účet     SK44 1100 0000 0000 1234 5678          Mena  EUR                          BIC (SWIFT)   TATRSKBX
IBAN SK44 1100 0000 0000 1234 5678
Číslo klienta:  1234567
Majiteľ účtu:   JANA VZOROVÁ
Tatra banka, a.s., Hodžovo nám. 3
811 06 Bratislava
Dialog  0800 00 1100              ID:   00                                    Výpis číslo:        1
Osobný účet     SK44 1100 0000 0000 1234 5678     Majiteľ Jana Vzorová                  Dátum 30.06.2026
Dátum sprac.  Popis                                     Dátum zúčt.                              Suma
--------------------------------------------------------------------------------------------------
              Posledný výpis  30.06.2026                                                       500.00
02.06.2026    EUR NÁVRAT POS                          29.05.2026                              20.00
              Miesto platby:    SEATTLE               ZUNIVERSUM
              Dátum:  29.05.26  Čas:  00:00:00        Suma:         20.00  EUR
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                                       520.00
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        1        Strana:        1
";
        // Imported AFTER the confirmed purchase exists, so the refund row's
        // classify pass at import time actually finds it.
        s.import_statement(&parser::parse_text(personal_refund_text).unwrap(), "h-personal-refund").unwrap();
        let refund = s.list_transactions(&crate::TxFilter { account_id: Some(personal_id), ..Default::default() }).unwrap().into_iter().next().unwrap();
        assert_eq!((refund.status, refund.category_id), (rules::Status::Suggested, Some(cat)), "the refund's category came from the confirmed business purchase, not from any rule");

        let outcome = s.delete_account(business_id).unwrap();

        assert!(outcome.open_rows_reclassified >= 1);
        let after = s.list_transactions(&crate::TxFilter { account_id: Some(personal_id), ..Default::default() }).unwrap().into_iter().find(|r| r.id == refund.id).unwrap();
        assert_eq!((after.status, after.category_id), (rules::Status::Unassigned, None), "no confirmed purchase for this merchant survives anywhere, so the refund falls back to unassigned");
    }

    /// PARENT-NOTE 078 correction: a COMMIT that fails must roll back too,
    /// not just a failure inside `delete_account_tx`. Same dangling-category
    /// setup as the rollback test above, but with `defer_foreign_keys` on:
    /// the `reclassify_open` write that would otherwise violate the FK
    /// immediately (and take the ordinary `Err` branch already covered
    /// above) instead succeeds inside the transaction and only surfaces as a
    /// violation when SQLite validates deferred constraints at `COMMIT`.
    /// This is the one path that exercises the new `COMMIT`-failure branch.
    #[test]
    fn a_failed_commit_rolls_back_too_and_leaves_the_connection_usable() {
        let mut s = Store::open_in_memory().unwrap();
        let doomed_account = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        let retained_account = s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
        let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();

        s.insert_rule(RuleKind::Merchant, "widget", None, cat).unwrap();
        s.conn.execute_batch(&format!(
            "INSERT INTO statements (id, account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (1, {doomed_account}, 1, '2026-06-01', '2026-06-30', 'ok', 'h1'); \
             INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source, rule_id, category_id) \
               VALUES (1, 1, {doomed_account}, 'fp-widget', '2026-06-01', '2026-06-01', 'card', -500, 'WIDGET', 'widget', 'raw', 'confirmed', 'merchant_rule', (SELECT id FROM rules WHERE key = 'widget'), {cat}); \
             INSERT INTO rule_sources (rule_id, transaction_id, statement_id) SELECT id, 1, 1 FROM rules WHERE key = 'widget';"
        )).unwrap();

        s.insert_rule(RuleKind::Merchant, "acme", None, cat).unwrap();
        s.conn.execute_batch(&format!(
            "INSERT INTO statements (id, account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (2, {retained_account}, 1, '2026-06-01', '2026-06-30', 'ok', 'h2'); \
             INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source) \
               VALUES (2, 2, {retained_account}, 'fp-acme', '2026-06-01', '2026-06-01', 'card', -700, 'ACME', 'acme', 'raw', 'unassigned', 'none');"
        )).unwrap();

        s.conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
        s.conn.execute("DELETE FROM categories WHERE id = ?1", [cat]).unwrap();
        s.conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA defer_foreign_keys = ON").unwrap();

        let doomed_account_before = s.account_by_id(doomed_account).unwrap();
        let retained_row_before = s.list_transactions(&crate::TxFilter { account_id: Some(retained_account), ..Default::default() }).unwrap();

        let e = s.delete_account(doomed_account).unwrap_err();

        assert!(matches!(e, StoreError::Db(_)), "got {e:?}");
        assert_eq!(s.account_by_id(doomed_account).unwrap(), doomed_account_before, "a rolled-back COMMIT must leave the account in place, same as a rolled-back body");
        assert_eq!(
            s.list_transactions(&crate::TxFilter { account_id: Some(retained_account), ..Default::default() }).unwrap(),
            retained_row_before,
            "a rolled-back COMMIT must leave a retained account's row untouched"
        );
        // Connection usability: `BEGIN IMMEDIATE` would itself fail with
        // "cannot start a transaction within a transaction" if the earlier
        // ROLLBACK had not actually closed the open one. A second, unrelated
        // write proves the connection recovered.
        let new_account = s.upsert_account("SK8911000000000055555555", AccountKind::Personal, "Nový").unwrap();
        assert!(s.account_by_id(new_account).unwrap().is_some(), "the connection must accept a fresh write after a rolled-back COMMIT");
    }
}
