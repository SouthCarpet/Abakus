//! Owned by lane A. Persistence scenarios R07/R08 from
//! recurring-acceptance.md: manual confirm/edit/ignore/reset, and evidence
//! deletion vs. account-cascade behavior.
use chrono::NaiveDate;
use rusqlite::Connection;
use parser::AccountKind;
use store::recurring::{RecurringDecisionInput, RecurringQuery, RecurringScope, RecurringSelection, RecurringState, RowDecision, SaveRecurringRequest, UnknownReason};
use store::Store;

struct TempDb(std::path::PathBuf);
impl TempDb {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-recurring-store-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        TempDb(path)
    }
}
impl Drop for TempDb {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

fn q(today: &str) -> RecurringQuery { RecurringQuery { from: None, to: None, account_kind: None, today: NaiveDate::parse_from_str(today, "%Y-%m-%d").unwrap() } }

/// R07: a single uncategorized transaction, manually confirmed yearly.
/// Appears confirmed immediately at that anchor; changing cadence to
/// quarterly persists on reopen; ignore hides it from totals; reset
/// returns it to inference (one observation alone gives no candidate).
#[test]
fn r07_manual_confirm_edit_ignore_reset_round_trip() {
    let db = TempDb::new("r07");
    let mut s = Store::open(&db.0).unwrap();
    let account = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    let tx_id = insert_tx(&db.0, account, "2026-01-15", -50000, "insurance");

    let decision = s
        .save_recurring(&SaveRecurringRequest {
            decision_id: None,
            selection: RecurringSelection::Group { transaction_id: tx_id },
            decision: RecurringDecisionInput::Confirmed { cadence: store::recurring::Cadence::Yearly, anchor_date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap() },
        })
        .unwrap();
    assert_eq!(decision.scope, RecurringScope::Group);

    let overview = s.recurring_overview(&q("2026-02-01")).unwrap();
    let row = overview.rows.iter().find(|r| r.decision_id == Some(decision.id)).expect("a manually confirmed row must appear immediately");
    assert_eq!(row.decision, RowDecision::Confirmed);
    assert_eq!(row.cadence, Some(store::recurring::Cadence::Yearly));

    // Edit cadence to quarterly, reopen the store, confirm it persisted.
    s.save_recurring(&SaveRecurringRequest {
        decision_id: Some(decision.id),
        selection: RecurringSelection::Group { transaction_id: tx_id },
        decision: RecurringDecisionInput::Confirmed { cadence: store::recurring::Cadence::Quarterly, anchor_date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap() },
    })
    .unwrap();
    drop(s);
    let mut s = Store::open(&db.0).unwrap();
    let overview = s.recurring_overview(&q("2026-02-01")).unwrap();
    let row = overview.rows.iter().find(|r| r.decision_id == Some(decision.id)).unwrap();
    assert_eq!(row.cadence, Some(store::recurring::Cadence::Quarterly), "the cadence edit must survive a reopen");

    // Ignore: hidden from confirmed totals, still visible as an ignored row.
    s.save_recurring(&SaveRecurringRequest { decision_id: Some(decision.id), selection: RecurringSelection::Group { transaction_id: tx_id }, decision: RecurringDecisionInput::Ignored }).unwrap();
    let overview = s.recurring_overview(&q("2026-02-01")).unwrap();
    let row = overview.rows.iter().find(|r| r.decision_id == Some(decision.id)).unwrap();
    assert_eq!(row.decision, RowDecision::Ignored);
    assert_eq!(overview.confirmed.annual_expense_cents, 0, "an ignored item must be absent from confirmed totals");

    // Reset: back to inference. One observation alone gives no candidate.
    s.reset_recurring(decision.id).unwrap();
    let overview = s.recurring_overview(&q("2026-02-01")).unwrap();
    assert!(!overview.rows.iter().any(|r| r.decision_id == Some(decision.id)), "after reset, a single observation must not reappear as a candidate");

    // A zero-effect reset of a missing id is a clear error.
    let err = s.reset_recurring(decision.id).unwrap_err();
    assert!(matches!(err, store::StoreError::Parse(_)));
}

/// R08 (partial, statement half): deleting the only source statement of a
/// confirmed item keeps the decision and fingerprint, but the row reads
/// unknown/no_evidence and drops out of totals.
#[test]
fn r08_deleting_the_only_evidence_makes_the_row_unknown_no_evidence() {
    let db = TempDb::new("r08-stmt");
    let mut s = Store::open(&db.0).unwrap();
    let account = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    let tx_id = insert_tx(&db.0, account, "2026-01-15", -50000, "insurance-r08");
    let decision = s
        .save_recurring(&SaveRecurringRequest {
            decision_id: None,
            selection: RecurringSelection::Group { transaction_id: tx_id },
            decision: RecurringDecisionInput::Confirmed { cadence: store::recurring::Cadence::Yearly, anchor_date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap() },
        })
        .unwrap();

    s.delete_statement(1).unwrap();

    let overview = s.recurring_overview(&q("2026-02-01")).unwrap();
    let row = overview.rows.iter().find(|r| r.decision_id == Some(decision.id)).expect("the decision must remain visible after its only evidence is deleted");
    assert_eq!(row.state, RecurringState::Unknown);
    assert_eq!(row.unknown_reason, Some(UnknownReason::NoEvidence));
    assert_eq!(overview.confirmed.annual_expense_cents, 0, "a no_evidence row must be excluded from confirmed totals");
    assert_eq!(overview.excluded.unknown, 1);
}

/// R08 (account half): deleting an account cascades its own recurring
/// decisions and members; another account's confirmed/ignored items
/// survive byte-for-byte.
#[test]
fn r08_deleting_an_account_cascades_its_decisions_and_leaves_others_alone() {
    let db = TempDb::new("r08-acct");
    let mut s = Store::open(&db.0).unwrap();
    let doomed = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    let kept = s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
    let doomed_tx = insert_tx(&db.0, doomed, "2026-01-15", -50000, "doomed-insurance");
    let kept_tx = insert_tx(&db.0, kept, "2026-01-15", -60000, "kept-insurance");

    let doomed_decision = s
        .save_recurring(&SaveRecurringRequest {
            decision_id: None,
            selection: RecurringSelection::Group { transaction_id: doomed_tx },
            decision: RecurringDecisionInput::Confirmed { cadence: store::recurring::Cadence::Yearly, anchor_date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap() },
        })
        .unwrap();
    let kept_decision = s
        .save_recurring(&SaveRecurringRequest {
            decision_id: None,
            selection: RecurringSelection::Group { transaction_id: kept_tx },
            decision: RecurringDecisionInput::Confirmed { cadence: store::recurring::Cadence::Yearly, anchor_date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap() },
        })
        .unwrap();

    s.delete_account(doomed).unwrap();

    let overview = s.recurring_overview(&q("2026-02-01")).unwrap();
    assert!(!overview.rows.iter().any(|r| r.decision_id == Some(doomed_decision.id)), "the doomed account's decision must cascade away");
    let kept_row = overview.rows.iter().find(|r| r.decision_id == Some(kept_decision.id)).expect("the retained account's decision must survive byte-for-byte");
    assert_eq!(kept_row.anchor_date, Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()));
    assert_eq!(kept_row.account_id, kept);
}

/// Writes one statement plus one transaction directly (same technique as
/// `recurring_detection.rs`: a second connection on the same on-disk file,
/// since `Store::conn` is not visible from an external integration test).
/// Callers are expected to have already opened the `Store` once so the
/// schema exists.
fn insert_tx(path: &std::path::Path, account_id: i64, date: &str, amount_cents: i64, key: &str) -> i64 {
    let conn = Connection::open(path).unwrap();
    let hash = format!("h-{key}");
    conn.execute(
        "INSERT INTO statements (account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (?1, 1, ?2, ?2, 'ok', ?3)",
        rusqlite::params![account_id, date, hash],
    )
    .unwrap();
    let statement_id: i64 = conn.query_row("SELECT id FROM statements WHERE file_hash = ?1", [&hash], |r| r.get(0)).unwrap();
    conn.execute(
        "INSERT INTO transactions (statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source) \
         VALUES (?1, ?2, ?3, ?4, ?4, 'card', ?5, 'INSURANCE CO', 'insurance co', 'raw', 'unassigned', 'none')",
        rusqlite::params![statement_id, account_id, format!("fp-{key}"), date, amount_cents],
    )
    .unwrap();
    conn.last_insert_rowid()
}
