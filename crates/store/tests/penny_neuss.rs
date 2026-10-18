//! Plan 091 p3 follow-up (ledger seq 5433): the synthetic Penny Neuss acceptance case, end to
//! end from statement text to a persistent SQLite file. The fixture
//! `fixtures/synthetic/penny-neuss-2026-07.txt` holds three PENNY payments in Neuss, one PENNY
//! payment in Berlin, one PENNY MARKT payment in Neuss and one REWE payment in Neuss.
//! Oracles: `docs/plan-091-transaction-search.md` (the phrase spans merchant and place, a
//! literal folded substring) and `docs/plan-091-bulk-confirmation.md` (confirm similar needs the
//! same merchant and the same place; a confirmed choice is never rewritten).
use rules::Status;
use rusqlite::{params, Connection};
use store::{Store, TxFilter};
use tempfile::TempDir;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/penny-neuss-2026-07.txt");

struct Loaded {
    _dir: TempDir,
    path: std::path::PathBuf,
}

fn load() -> Loaded {
    let statement = parser::parse_text(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("penny-neuss.db");
    let mut store = Store::open(&path).unwrap();
    store.upsert_account(&statement.iban, statement.account_kind, "Osobný").unwrap();
    store.import_statement(&statement, "penny-neuss").unwrap();
    Loaded { _dir: dir, path }
}

fn rows(store: &Store) -> Vec<store::TxRow> {
    store.list_transactions(&TxFilter::default()).unwrap()
}

fn ids_where(store: &Store, merchant: &str, place: &str) -> Vec<i64> {
    let mut ids: Vec<i64> = rows(store)
        .into_iter()
        .filter(|row| row.merchant_raw == merchant && row.place.as_deref() == Some(place))
        .map(|row| row.id)
        .collect();
    ids.sort();
    ids
}

fn search(store: &Store, text: &str) -> Vec<i64> {
    let mut ids: Vec<i64> = store
        .list_transactions(&TxFilter { text: Some(text.into()), ..Default::default() })
        .unwrap()
        .into_iter()
        .map(|row| row.id)
        .collect();
    ids.sort();
    ids
}

#[test]
fn the_fixture_parses_into_six_card_payments_with_a_matching_closing_balance() {
    let statement = parser::parse_text(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();

    let pairs: Vec<(String, Option<String>, i64)> = statement
        .transactions
        .iter()
        .map(|tx| (tx.merchant_raw.clone(), tx.place.clone(), tx.amount_cents))
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("PENNY".to_string(), Some("Neuss".to_string()), -1240),
            ("PENNY".to_string(), Some("Neuss".to_string()), -2310),
            ("PENNY".to_string(), Some("Neuss".to_string()), -875),
            ("PENNY".to_string(), Some("Berlin".to_string()), -1500),
            ("PENNY MARKT".to_string(), Some("Neuss".to_string()), -620),
            ("REWE".to_string(), Some("Neuss".to_string()), -980),
        ]
    );
    assert_eq!(statement.opening_cents, Some(50_000));
    assert_eq!(statement.closing_cents, Some(42_475));
}

#[test]
fn the_exact_phrase_finds_only_the_three_penny_payments_in_neuss() {
    let db = load();
    let store = Store::open(&db.path).unwrap();
    let neuss = ids_where(&store, "PENNY", "Neuss");
    assert_eq!(neuss.len(), 3);

    assert_eq!(search(&store, "Penny Neuss"), neuss, "PENNY Berlin, PENNY MARKT Neuss and REWE Neuss stay out");
    assert_eq!(search(&store, "  pénny   NEUSS "), neuss, "case, diacritics and spaces fold the same way");
    assert_eq!(search(&store, "Penny Berlin"), ids_where(&store, "PENNY", "Berlin"));
    assert_eq!(search(&store, "Penny").len(), 5, "the merchant alone also finds Berlin and PENNY MARKT");
}

#[test]
fn confirming_one_suggestion_with_similar_confirms_the_neuss_peers_and_nothing_else() {
    let db = load();
    let store = Store::open(&db.path).unwrap();
    let neuss = ids_where(&store, "PENNY", "Neuss");
    let food = store.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    drop(store);
    Connection::open(&db.path)
        .unwrap()
        .execute("UPDATE transactions SET status = 'suggested', category_id = ?1 WHERE id = ?2", params![food, neuss[0]])
        .unwrap();
    let mut store = Store::open(&db.path).unwrap();
    let before: std::collections::HashMap<i64, (Status, Option<i64>)> =
        rows(&store).into_iter().map(|row| (row.id, (row.status, row.category_id))).collect();

    let outcome = store.confirm(&[neuss[0]], true).unwrap();

    assert_eq!(outcome.updated, 3, "the selected row and its two Neuss peers");
    for row in rows(&store) {
        let same = row.merchant_raw == "PENNY" && row.place.as_deref() == Some("Neuss");
        // Another merchant or place keeps the state the import gave it (REWE may carry a
        // built-in suggestion; it is never confirmed by this action).
        let expected = if same { (Status::Confirmed, Some(food)) } else { before[&row.id] };
        assert_eq!((row.status, row.category_id), expected, "{} {:?}", row.merchant_raw, row.place);
        if !same {
            assert_ne!(row.status, Status::Confirmed);
        }
    }
}

#[test]
fn a_bulk_confirmation_of_the_search_result_never_reaches_another_merchant() {
    let db = load();
    let store = Store::open(&db.path).unwrap();
    let neuss = search(&store, "Penny Neuss");
    let food = store.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    drop(store);
    let connection = Connection::open(&db.path).unwrap();
    for id in &neuss {
        connection
            .execute("UPDATE transactions SET status = 'suggested', category_id = ?1 WHERE id = ?2", params![food, id])
            .unwrap();
    }
    drop(connection);
    let mut store = Store::open(&db.path).unwrap();

    let outcome = store.confirm(&neuss, true).unwrap();

    assert_eq!(outcome.updated, 3);
    let confirmed: Vec<i64> = {
        let mut ids: Vec<i64> = rows(&store).into_iter().filter(|row| row.status == Status::Confirmed).map(|row| row.id).collect();
        ids.sort();
        ids
    };
    assert_eq!(confirmed, neuss, "PENNY MARKT in the same place and PENNY in Berlin stay open");
}
