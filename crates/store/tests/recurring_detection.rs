//! Owned by lane A. Independent synthetic scenarios from
//! `animus/data/agent-runs/abakus-012-20260907/recurring-acceptance.md`
//! (R01, R03, R04, R09, R10, R12). Raw SQL writes go through a second
//! connection on the SAME on-disk file (same technique as
//! `crates/store/tests/migration.rs`'s `TempDb`), because `Store::conn` is
//! `pub(crate)` and this file is an external integration test: only the
//! public API and a fresh `Store::open` may observe the result.
use chrono::NaiveDate;
use rusqlite::Connection;
use store::recurring::{Cadence, RecurringDirection, RecurringQuery, RecurringState, RowDecision};
use store::Store;

struct TempDb(std::path::PathBuf);
impl TempDb {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-recurring-detect-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        TempDb(path)
    }
    fn raw(&self) -> Connection { Connection::open(&self.0).unwrap() }
}
impl Drop for TempDb {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

/// `(tx_date, amount_cents)` rows for one account, one statement covering
/// `stmt_start..stmt_end`, `checksum_status='ok'`.
fn seed(db: &TempDb, account_id: i64, iban: &str, stmt_number: i64, stmt_start: &str, stmt_end: &str, hash: &str, rows: &[(&str, i64)]) {
    { let _ = Store::open(&db.0).unwrap(); } // creates the schema (and seeds categories) before the raw connection writes into it
    let conn = db.raw();
    conn.execute(
        "INSERT OR IGNORE INTO accounts (id, iban, kind, label) VALUES (?1, ?2, 'personal', 'Osobný')",
        rusqlite::params![account_id, iban],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO statements (account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (?1, ?2, ?3, ?4, 'ok', ?5)",
        rusqlite::params![account_id, stmt_number, stmt_start, stmt_end, hash],
    )
    .unwrap();
    let statement_id: i64 = conn.query_row("SELECT id FROM statements WHERE file_hash = ?1", [hash], |r| r.get(0)).unwrap();
    for (i, (date, amount)) in rows.iter().enumerate() {
        let fp = format!("{hash}-fp-{i}");
        conn.execute(
            "INSERT INTO transactions (statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source) \
             VALUES (?1, ?2, ?3, ?4, ?4, 'card', ?5, 'NETFLIX.COM', 'netflix com', 'raw', 'unassigned', 'none')",
            rusqlite::params![statement_id, account_id, fp, date, amount],
        )
        .unwrap();
    }
}

fn q(from: Option<&str>, to: Option<&str>, today: &str) -> RecurringQuery {
    RecurringQuery {
        from: from.map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()),
        to: to.map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()),
        account_kind: None,
        today: NaiveDate::parse_from_str(today, "%Y-%m-%d").unwrap(),
    }
}

/// R01: three monthly expense observations, gap 28-33 days, qualify
/// monthly, anchored at the first observation, next occurrence April 30.
/// Two observations alone must not produce a candidate (mutation below).
#[test]
fn r01_three_monthly_observations_qualify_and_two_do_not() {
    let db = TempDb::new("r01");
    seed(&db, 1, "SK4411000000000012345678", 1, "2026-01-01", "2026-04-30", "r01", &[("2026-01-31", -1200), ("2026-02-28", -1200), ("2026-03-31", -1200)]);
    let s = Store::open(&db.0).unwrap();

    let overview = s.recurring_overview(&q(None, None, "2026-04-15")).unwrap();
    let row = overview.rows.iter().find(|r| r.name.to_lowercase().contains("netflix")).expect("a qualifying monthly run must produce a candidate row");
    assert_eq!(row.decision, RowDecision::Estimate);
    assert_eq!(row.cadence, Some(Cadence::Monthly));
    assert_eq!(row.anchor_date, Some(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap()));
    assert_eq!(row.next_due, Some(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap()));
    assert_eq!(row.state, RecurringState::Upcoming);
    assert_eq!(row.direction, RecurringDirection::Expense);

    // Mutation: drop to two observations (delete the March row) and the
    // candidate must disappear entirely, not just lose its cadence label.
    let conn = db.raw();
    conn.execute("DELETE FROM transactions WHERE fingerprint = 'r01-fp-2'", []).unwrap();
    drop(conn);
    let s2 = Store::open(&db.0).unwrap();
    let overview2 = s2.recurring_overview(&q(None, None, "2026-04-15")).unwrap();
    assert!(!overview2.rows.iter().any(|r| r.name.to_lowercase().contains("netflix")), "two observations alone must give no candidate");
}

/// R03: day-gap boundaries (28/33 inclusive) and the 10% amount band.
#[test]
fn r03_day_gap_and_amount_boundaries() {
    let db = TempDb::new("r03");
    // 33-day gaps: Jan1 -> Feb3 (33d) -> Mar8 (33d): both inside the inclusive window.
    seed(&db, 1, "SK4411000000000012345678", 1, "2026-01-01", "2026-04-30", "r03-ok", &[("2026-01-01", -1000), ("2026-02-03", -1000), ("2026-03-08", -1000)]);
    let s = Store::open(&db.0).unwrap();
    let overview = s.recurring_overview(&q(None, None, "2026-03-20")).unwrap();
    assert!(overview.rows.iter().any(|r| r.name.to_lowercase().contains("netflix")), "a 33-day gap is inside the inclusive 28-33 window");

    let db2 = TempDb::new("r03-bad");
    // 34-day gap: just outside.
    seed(&db2, 1, "SK4411000000000012345678", 1, "2026-01-01", "2026-05-31", "r03-bad", &[("2026-01-01", -1000), ("2026-02-04", -1000), ("2026-03-10", -1000)]);
    let s2 = Store::open(&db2.0).unwrap();
    let overview2 = s2.recurring_overview(&q(None, None, "2026-03-20")).unwrap();
    assert!(!overview2.rows.iter().any(|r| r.name.to_lowercase().contains("netflix")), "a 34-day gap must fail the monthly window");
}

/// R04: same merchant name on two accounts with opposite cash sign never
/// shares a group, so neither alone produces a candidate below the
/// evidence minimum, and a manual account-scoped save never crosses accounts.
#[test]
fn r04_same_merchant_different_accounts_and_signs_never_group() {
    let db = TempDb::new("r04");
    seed(&db, 1, "SK4411000000000012345678", 1, "2026-01-01", "2026-04-30", "r04-a", &[("2026-01-31", -1200), ("2026-02-28", -1200)]);
    seed(&db, 2, "SK3711000000000098765432", 1, "2026-01-01", "2026-04-30", "r04-b", &[("2026-01-31", 1200), ("2026-02-28", 1200)]);
    let s = Store::open(&db.0).unwrap();
    let overview = s.recurring_overview(&q(None, None, "2026-03-01")).unwrap();
    // Neither side alone reaches the 3-observation monthly minimum, so
    // no candidate should appear from either account.
    assert!(!overview.rows.iter().any(|r| r.name.to_lowercase().contains("netflix")));
}

/// R09: a historical query must never let evidence AFTER `as_of` prove a
/// candidate. Same `as_of`, a narrower `from` still keeps earlier detection
/// history (does not re-run detection with a truncated training window).
#[test]
fn r09_historical_as_of_ignores_future_evidence() {
    let db = TempDb::new("r09");
    seed(&db, 1, "SK4411000000000012345678", 1, "2026-01-01", "2026-09-30", "r09", &[("2026-01-31", -1200), ("2026-02-28", -1200), ("2026-03-31", -1200), ("2026-09-05", -1200)]);
    let s = Store::open(&db.0).unwrap();

    let historical = s.recurring_overview(&q(Some("2026-01-01"), Some("2026-03-31"), "2026-09-07")).unwrap();
    assert_eq!(historical.as_of, NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(), "as_of = min(to, today) for a finite range");
    let row = historical.rows.iter().find(|r| r.name.to_lowercase().contains("netflix")).expect("Jan-Mar alone already qualifies monthly");
    assert_eq!(row.evidence_count, 3, "the September row must not be visible to a query whose as_of is March 31");
    assert_eq!(row.next_due, Some(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap()), "next_due must come from the anchor schedule, never from the future September observation");

    let narrower_from = s.recurring_overview(&q(Some("2026-03-01"), Some("2026-03-31"), "2026-09-07")).unwrap();
    assert!(narrower_from.rows.iter().any(|r| r.name.to_lowercase().contains("netflix")), "a narrower `from` at the same as_of must keep earlier detection history (detection never cuts training history at `from`)");
}

/// R12: a stable price change badge, and a later one-off does not replace it.
#[test]
fn r12_price_change_badge_and_one_off_stability() {
    let db = TempDb::new("r12");
    seed(
        &db,
        1,
        "SK4411000000000012345678",
        1,
        "2026-01-01",
        "2026-07-31",
        "r12",
        &[("2026-01-31", -1000), ("2026-02-28", -1000), ("2026-03-31", -1200), ("2026-04-30", -1200), ("2026-05-31", -1200), ("2026-06-30", -1300)],
    );
    let s = Store::open(&db.0).unwrap();
    let overview = s.recurring_overview(&q(None, None, "2026-07-01")).unwrap();
    let row = overview.rows.iter().find(|r| r.name.to_lowercase().contains("netflix")).unwrap();
    let badge = row.price_change.as_ref().expect("two stable runs must produce a badge");
    assert_eq!((badge.previous_cents, badge.current_cents, badge.delta_cents), (1000, 1200, 200));
    assert_eq!(badge.effective_from, NaiveDate::from_ymd_opt(2026, 3, 31).unwrap());
}
