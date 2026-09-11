use chrono::NaiveDate;
use parser::{AccountKind, Statement, Transaction, TxKind};
use rules::{RuleKind, Status};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use store::{Store, StoreError, TxFilter};
use tempfile::TempDir;

const IBAN: &str = "SK4411000000000012345678";

struct TestDb {
    _dir: TempDir,
    path: PathBuf,
}

impl TestDb {
    fn new(rows: Vec<Transaction>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bulk-confirmation.db");
        let mut store = Store::open(&path).unwrap();
        store
            .upsert_account(IBAN, AccountKind::Personal, "Osobný")
            .unwrap();
        store
            .import_statement(&statement(rows), "bulk-confirmation")
            .unwrap();
        drop(store);
        Self { _dir: dir, path }
    }

    fn set_state(&self, merchant: &str, status: &str, category_id: Option<i64>) {
        Connection::open(&self.path)
            .unwrap()
            .execute(
                "UPDATE transactions SET status = ?2, category_id = ?3, rule_id = NULL, source = 'none' WHERE merchant_raw = ?1",
                params![merchant, status, category_id],
            )
            .unwrap();
    }

    fn open(&self) -> Store {
        Store::open(&self.path).unwrap()
    }
}

fn statement(rows: Vec<Transaction>) -> Statement {
    Statement {
        iban: IBAN.into(),
        account_kind: AccountKind::Personal,
        number: 7,
        period_start: NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
        period_end: NaiveDate::from_ymd_opt(2026, 7, 31).unwrap(),
        opening_cents: None,
        closing_cents: None,
        transactions: rows,
        warnings: Vec::new(),
    }
}

fn row(
    day: u32,
    kind: TxKind,
    merchant: &str,
    place: Option<&str>,
    counterparty_iban: Option<&str>,
) -> Transaction {
    let date = NaiveDate::from_ymd_opt(2026, 7, day).unwrap();
    let mut row = Transaction::blank(date, -i64::from(day) * 100, kind, format!("raw-{day}"));
    row.merchant_raw = merchant.into();
    row.place = place.map(str::to_string);
    row.counterparty_iban = counterparty_iban.map(str::to_string);
    row
}

fn id(store: &Store, merchant: &str) -> i64 {
    store
        .list_transactions(&TxFilter::default())
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.merchant_raw == merchant)
        .unwrap()
        .id
}

fn state(store: &Store, merchant: &str) -> (Status, Option<i64>) {
    let row = store
        .list_transactions(&TxFilter::default())
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.merchant_raw == merchant)
        .unwrap();
    (row.status, row.category_id)
}

#[test]
fn confirm_deduplicates_selected_ids_and_reports_each_transfer_once() {
    let db = TestDb::new(vec![
        row(1, TxKind::Card, "SHOP", Some("Nitra"), None),
        row(2, TxKind::TransferOut, "OWN", None, Some(IBAN)),
    ]);
    let store = db.open();
    let shop = id(&store, "SHOP");
    let transfer = id(&store, "OWN");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    drop(store);
    db.set_state("SHOP", "suggested", Some(category));
    let mut store = db.open();

    let outcome = store
        .confirm(&[shop, shop, transfer, transfer], false)
        .unwrap();

    assert_eq!(outcome.updated, 1, "one suggested row is confirmed once");
    assert_eq!(
        outcome.skipped_transfers, 1,
        "one transfer id is skipped once"
    );
}

#[test]
fn confirm_matching_requires_the_same_nonempty_merchant_and_place() {
    let db = TestDb::new(vec![
        row(1, TxKind::Card, "ALFA", Some("Nitra"), None),
        row(2, TxKind::Card, "alfa", Some("nitra"), None),
        row(3, TxKind::Card, "alfa", Some("Žilina"), None),
        row(4, TxKind::Card, "alfa", None, None),
        row(5, TxKind::Card, "", Some("Nitra"), None),
        row(6, TxKind::Card, "", Some("Nitra"), None),
    ]);
    let store = db.open();
    let source = store
        .list_transactions(&TxFilter::default())
        .unwrap()
        .into_iter()
        .find(|candidate| {
            candidate.merchant_raw == "ALFA" && candidate.place.as_deref() == Some("Nitra")
        })
        .unwrap()
        .id;
    let food = store.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    let home = store.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    drop(store);
    let connection = Connection::open(&db.path).unwrap();
    connection
        .execute(
            "UPDATE transactions SET status = 'suggested', category_id = ?1 WHERE id = ?2",
            params![food, source],
        )
        .unwrap();
    connection.execute("UPDATE transactions SET category_id = ?1 WHERE merchant_raw = 'alfa' AND place = 'Žilina'", [home]).unwrap();
    drop(connection);
    let mut store = db.open();

    let outcome = store.confirm(&[source], true).unwrap();

    assert_eq!(
        outcome.updated, 2,
        "the selected row and its exact Nitra peer are confirmed"
    );
    let rows = store.list_transactions(&TxFilter::default()).unwrap();
    let nitra_count = rows
        .iter()
        .filter(|candidate| {
            candidate
                .place
                .as_deref()
                .is_some_and(|place| place.eq_ignore_ascii_case("Nitra"))
                && candidate.status == Status::Confirmed
        })
        .count();
    assert_eq!(nitra_count, 2);
    assert!(
        rows.iter().any(|candidate| candidate.merchant_raw == "alfa"
            && candidate.place.as_deref() == Some("Žilina")
            && candidate.status == Status::Unassigned),
        "a different place and category stays open"
    );
    assert!(
        rows.iter().any(|candidate| candidate.merchant_raw == "alfa"
            && candidate.place.is_none()
            && candidate.status == Status::Unassigned),
        "missing place does not equal a concrete place"
    );
    assert_eq!(
        rows.iter()
            .filter(|candidate| candidate.merchant_raw.is_empty()
                && candidate.status == Status::Unassigned)
            .count(),
        2,
        "an empty merchant is never a match key"
    );
}

#[test]
fn confirm_matching_uses_iban_only_for_bank_transaction_kinds() {
    const COUNTERPARTY: &str = "SK0281800000007000000001";
    let db = TestDb::new(vec![
        row(1, TxKind::TransferOut, "RENT A", None, Some(COUNTERPARTY)),
        row(2, TxKind::StandingOrder, "RENT B", None, Some(COUNTERPARTY)),
        row(3, TxKind::Card, "CARD", Some("Nitra"), Some(COUNTERPARTY)),
    ]);
    let store = db.open();
    let source = id(&store, "RENT A");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    drop(store);
    db.set_state("RENT A", "suggested", Some(category));
    let mut store = db.open();

    let outcome = store.confirm(&[source], true).unwrap();

    assert_eq!(
        outcome.updated, 2,
        "the selected transfer and standing order share the exact account"
    );
    assert_eq!(state(&store, "RENT B"), (Status::Confirmed, Some(category)));
    assert_eq!(
        state(&store, "CARD"),
        (Status::Unassigned, None),
        "a card row does not use an incidental account as identity"
    );
    assert!(store
        .list_rules()
        .unwrap()
        .iter()
        .any(|rule| rule.kind == RuleKind::CounterpartyAccount
            && rule.key == COUNTERPARTY
            && rule.category_id == category));
}

#[test]
fn confirm_matching_preserves_confirmed_and_different_concrete_categories() {
    let db = TestDb::new(vec![
        row(1, TxKind::Card, "BETA", Some("Nitra"), None),
        row(2, TxKind::Card, "BETA", Some("Nitra"), None),
        row(3, TxKind::Card, "BETA", Some("Nitra"), None),
    ]);
    let store = db.open();
    let source = id(&store, "BETA");
    let food = store.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    let home = store.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    drop(store);
    let connection = Connection::open(&db.path).unwrap();
    connection
        .execute(
            "UPDATE transactions SET status = 'suggested', category_id = ?1 WHERE id = ?2",
            params![food, source],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE transactions SET status = 'confirmed', category_id = ?1 \
         WHERE id = (SELECT MIN(id) FROM transactions WHERE merchant_raw = 'BETA' AND id <> ?2)",
            params![home, source],
        )
        .unwrap();
    connection.execute("UPDATE transactions SET status = 'unassigned', category_id = ?1 WHERE merchant_raw = 'BETA' AND id <> ?2 AND status <> 'confirmed'", params![home, source]).unwrap();
    drop(connection);
    let mut store = db.open();

    let outcome = store.confirm(&[source], true).unwrap();

    assert_eq!(outcome.updated, 1);
    let rows = store.list_transactions(&TxFilter::default()).unwrap();
    assert_eq!(
        rows.iter()
            .filter(|candidate| candidate.status == Status::Confirmed
                && candidate.category_id == Some(home))
            .count(),
        1,
        "the older confirmed row stays unchanged"
    );
    assert_eq!(
        rows.iter()
            .filter(|candidate| candidate.status == Status::Unassigned
                && candidate.category_id == Some(home))
            .count(),
        1,
        "an open row with another concrete category stays unchanged"
    );
}

#[test]
fn conflicting_selected_categories_have_order_independent_results() {
    #[derive(Debug, PartialEq)]
    struct OrderResult {
        states: Vec<(String, Status, Option<i64>)>,
        rules: Vec<(RuleKind, String, Option<String>, i64)>,
    }

    fn run(reverse: bool) -> OrderResult {
        let db = TestDb::new(vec![
            row(1, TxKind::Card, "GAMMA", Some("Nitra"), None),
            row(2, TxKind::Card, "GAMMA", Some("Žilina"), None),
            row(3, TxKind::Card, "GAMMA", Some("Nitra"), None),
            row(4, TxKind::Card, "GAMMA", Some("Žilina"), None),
        ]);
        let store = db.open();
        let rows = store.list_transactions(&TxFilter::default()).unwrap();
        let first = rows
            .iter()
            .find(|candidate| candidate.place.as_deref() == Some("Nitra"))
            .unwrap()
            .id;
        let second = rows
            .iter()
            .find(|candidate| candidate.place.as_deref() == Some("Žilina"))
            .unwrap()
            .id;
        let food = store.category_by_path("Jedlo/potraviny").unwrap().unwrap();
        let home = store.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
        drop(store);
        let connection = Connection::open(&db.path).unwrap();
        connection
            .execute(
                "UPDATE transactions SET status = 'suggested', category_id = ?1 WHERE id = ?2",
                params![food, first],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE transactions SET status = 'suggested', category_id = ?1 WHERE id = ?2",
                params![home, second],
            )
            .unwrap();
        drop(connection);
        let mut store = db.open();
        let ids = if reverse {
            [second, first]
        } else {
            [first, second]
        };
        let outcome = store.confirm(&ids, true).unwrap();
        assert_eq!(
            outcome.updated, 4,
            "both selected rows and both exact-place peers are confirmed"
        );
        let mut states: Vec<_> = store
            .list_transactions(&TxFilter::default())
            .unwrap()
            .into_iter()
            .map(|candidate| {
                (
                    candidate.place.unwrap(),
                    candidate.status,
                    candidate.category_id,
                )
            })
            .collect();
        states.sort_by(|left, right| left.0.cmp(&right.0).then(left.2.cmp(&right.2)));
        let mut rules: Vec<_> = store
            .list_rules()
            .unwrap()
            .into_iter()
            .filter(|rule| rule.key == "gamma")
            .map(|rule| (rule.kind, rule.key, rule.place, rule.category_id))
            .collect();
        rules.sort_by(|left, right| left.2.cmp(&right.2));
        OrderResult { states, rules }
    }

    let forward = run(false);
    let reverse = run(true);

    assert_eq!(
        forward, reverse,
        "selection order cannot choose the final category"
    );
    assert_eq!(
        forward
            .rules
            .iter()
            .filter(|rule| rule.0 == RuleKind::Merchant)
            .count(),
        0,
        "conflicting categories do not create one broad merchant rule"
    );
}

#[test]
fn confirm_rolls_back_rules_sources_and_rows_then_retries_on_the_same_store() {
    let db = TestDb::new(vec![
        row(1, TxKind::Card, "ROLLBACK A", Some("Nitra"), None),
        row(2, TxKind::Card, "ROLLBACK B", Some("Nitra"), None),
    ]);
    let store = db.open();
    let first = id(&store, "ROLLBACK A");
    let second = id(&store, "ROLLBACK B");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    drop(store);
    db.set_state("ROLLBACK A", "suggested", Some(category));
    db.set_state("ROLLBACK B", "suggested", Some(category));
    let connection = Connection::open(&db.path).unwrap();
    connection.execute_batch(&format!("CREATE TRIGGER fail_second_confirmation BEFORE UPDATE OF status ON transactions WHEN OLD.id = {second} BEGIN SELECT RAISE(ABORT, 'forced second-row failure'); END;")).unwrap();
    drop(connection);
    let mut store = db.open();
    let rules_before = store.list_rules().unwrap().len();

    let error = store.confirm(&[first, second], false).unwrap_err();

    assert!(error.to_string().contains("forced second-row failure"));
    assert_eq!(
        state(&store, "ROLLBACK A"),
        (Status::Suggested, Some(category)),
        "the first successful row write is rolled back"
    );
    assert_eq!(
        state(&store, "ROLLBACK B"),
        (Status::Suggested, Some(category)),
        "the failing row stays unchanged"
    );
    assert_eq!(
        store.list_rules().unwrap().len(),
        rules_before,
        "learned rules are rolled back too"
    );
    Connection::open(&db.path)
        .unwrap()
        .execute_batch("DROP TRIGGER fail_second_confirmation")
        .unwrap();

    let retry = store.confirm(&[first, second], false).unwrap();

    assert_eq!(
        retry.updated, 2,
        "the same Store accepts a valid write after rollback"
    );
    drop(store);
    let mut reopened = db.open();
    reopened.reclassify_open().unwrap();
    assert_eq!(
        state(&reopened, "ROLLBACK A"),
        (Status::Confirmed, Some(category))
    );
    assert!(reopened
        .list_rules()
        .unwrap()
        .iter()
        .any(|rule| rule.kind == RuleKind::Exact && rule.key == "rollback a"));
    let source_count: i64 = Connection::open(&db.path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM rule_sources", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        source_count, 4,
        "two rows persist provenance for exact and merchant rules"
    );
}

#[test]
fn unknown_id_aborts_the_full_confirmation_batch() {
    let db = TestDb::new(vec![row(1, TxKind::Card, "KNOWN", Some("Nitra"), None)]);
    let store = db.open();
    let known = id(&store, "KNOWN");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    drop(store);
    db.set_state("KNOWN", "suggested", Some(category));
    let mut store = db.open();

    let error = store.confirm(&[known, 999_999], false).unwrap_err();

    assert!(matches!(
        error,
        StoreError::UnknownTransaction { id: 999_999 }
    ));
    assert_eq!(state(&store, "KNOWN"), (Status::Suggested, Some(category)));
}

#[test]
fn assign_reports_selected_and_legacy_merchant_matching_rows() {
    let db = TestDb::new(vec![
        row(1, TxKind::Card, "DELTA", Some("Nitra"), None),
        row(2, TxKind::Card, "DELTA", Some("Nitra"), None),
        row(3, TxKind::Card, "DELTA", Some("Žilina"), None),
    ]);
    let mut store = db.open();
    let source = store
        .list_transactions(&TxFilter::default())
        .unwrap()
        .into_iter()
        .find(|candidate| {
            candidate.merchant_raw == "DELTA" && candidate.place.as_deref() == Some("Nitra")
        })
        .unwrap()
        .id;
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    assert_eq!(
        store
            .list_transactions(&TxFilter::default())
            .unwrap()
            .iter()
            .filter(|candidate| candidate.merchant_raw == "DELTA"
                && candidate.place.as_deref() == Some("Nitra"))
            .count(),
        2
    );

    let outcome = store.assign(&[source], category, true).unwrap();

    assert_eq!(
        outcome.updated, 3,
        "updated includes the selected row and both legacy merchant matches"
    );
    assert_eq!(
        store
            .list_transactions(&TxFilter::default())
            .unwrap()
            .iter()
            .filter(|candidate| candidate.status == Status::Confirmed)
            .count(),
        3
    );
}

#[test]
fn empty_confirmation_is_a_no_op() {
    let db = TestDb::new(vec![row(1, TxKind::Card, "NOOP", Some("Nitra"), None)]);
    let mut store = db.open();
    let rules_before = store.list_rules().unwrap();

    let outcome = store.confirm(&[], true).unwrap();

    assert_eq!(
        (
            outcome.updated,
            outcome.rules_created,
            outcome.skipped_transfers
        ),
        (0, 0, 0)
    );
    assert_eq!(store.list_rules().unwrap(), rules_before);
    assert_eq!(state(&store, "NOOP"), (Status::Unassigned, None));
}

#[test]
fn confirmed_and_unassigned_selected_rows_are_not_reconfirmed() {
    let db = TestDb::new(vec![
        row(1, TxKind::Card, "OLD", Some("Nitra"), None),
        row(2, TxKind::Card, "OPEN", Some("Nitra"), None),
    ]);
    let store = db.open();
    let old = id(&store, "OLD");
    let open = id(&store, "OPEN");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    drop(store);
    db.set_state("OLD", "confirmed", Some(category));
    let mut store = db.open();

    let outcome = store.confirm(&[old, open], true).unwrap();

    assert_eq!(outcome.updated, 0);
    assert_eq!(state(&store, "OLD"), (Status::Confirmed, Some(category)));
    assert_eq!(state(&store, "OPEN"), (Status::Unassigned, None));
}
