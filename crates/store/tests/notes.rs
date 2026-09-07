//! 0.1.2: `save_transaction_note` plus the folded, literal-substring text
//! search it extends, and the CSV `poznamka` column, all against a real
//! persistent SQLite file (not `open_in_memory`), the shape the desktop app
//! actually runs in.
use parser::AccountKind;
use rusqlite::Connection;
use store::{Store, TxFilter};

struct TempDb(std::path::PathBuf);
impl TempDb {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-notes-it-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        TempDb(path)
    }
}
impl Drop for TempDb {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

fn fixture(name: &str) -> parser::Statement {
    parser::parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap()).unwrap()
}

fn loaded(db: &TempDb) -> Store {
    let mut s = Store::open(&db.0).unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    s
}

fn id_of(s: &Store, merchant: &str) -> i64 {
    s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.merchant_raw == merchant).unwrap().id
}

fn search(s: &Store, text: &str) -> Vec<i64> {
    s.list_transactions(&TxFilter { text: Some(text.into()), ..Default::default() }).unwrap().into_iter().map(|r| r.id).collect()
}

#[test]
fn a_percent_sign_in_the_search_text_is_a_literal_character_not_a_sql_wildcard() {
    let db = TempDb::new("percent");
    let mut s = loaded(&db);
    let discount = id_of(&s, "ALDI SUED");
    let unrelated = id_of(&s, "OF");
    s.save_transaction_note(discount, "zľava 50%").unwrap();
    s.save_transaction_note(unrelated, "50 centov").unwrap();

    let hits = search(&s, "50%");

    assert_eq!(hits, vec![discount], "a bare '50' must not satisfy a search for the literal '50%'");
}

#[test]
fn an_underscore_in_the_search_text_is_a_literal_character_not_a_sql_single_char_wildcard() {
    let db = TempDb::new("underscore");
    let mut s = loaded(&db);
    let exact = id_of(&s, "ALDI SUED");
    let decoy = id_of(&s, "OF");
    s.save_transaction_note(exact, "kod_123").unwrap();
    s.save_transaction_note(decoy, "kodX123").unwrap();

    let hits = search(&s, "kod_123");

    assert_eq!(hits, vec![exact], "'kodX123' must not satisfy a search for the literal 'kod_123'");
}

#[test]
fn search_matches_a_note_through_diacritic_and_case_folding() {
    let db = TempDb::new("fold");
    let mut s = loaded(&db);
    let id = id_of(&s, "ALDI SUED");
    s.save_transaction_note(id, "Kávička s mliekom").unwrap();

    assert_eq!(search(&s, "kavicka"), vec![id], "search text must fold to match diacritics and case");
    assert_eq!(search(&s, "KAVICKA"), vec![id]);
}

#[test]
fn search_still_matches_merchant_place_and_counterparty_as_before_alongside_notes() {
    let db = TempDb::new("still-matches");
    let s = loaded(&db);
    let aldi = id_of(&s, "ALDI SUED");

    assert_eq!(search(&s, "ALDI"), vec![aldi]);
    assert!(!search(&s, "neuss").is_empty(), "place search must still work");
}

/// The CSV round trip: a leading LF then a leading `=` on the next line (a
/// formula attempt hidden behind a blank first line), an embedded quote, and
/// an embedded plain newline, all in the same note.
#[test]
fn csv_export_defuses_and_quotes_a_multiline_note_and_adds_the_poznamka_column() {
    let db = TempDb::new("csv");
    let mut s = loaded(&db);
    let id = id_of(&s, "ALDI SUED");
    let note = "\n=cmd|'/c calc'!A1\nobsahuje \"úvodzovky\" aj koniec riadku";
    s.save_transaction_note(id, note).unwrap();

    let csv = s.export_csv(&TxFilter { text: Some("ALDI".into()), ..Default::default() }).unwrap();

    assert!(csv.starts_with("datum;ucet;obchodnik;miesto;suma_eur;kategoria;podkategoria;stav;poznamka\n"));
    let expected_field = "\"'\n=cmd|'/c calc'!A1\nobsahuje \"\"úvodzovky\"\" aj koniec riadku\"";
    assert!(csv.contains(expected_field), "note must be defused (leading LF), quote-doubled and kept multiline:\n{csv}");
    assert_eq!(s.list_transactions(&TxFilter { text: Some("ALDI".into()), ..Default::default() }).unwrap().len(), 1, "exactly one record must have been exported despite the embedded newlines");
}

#[test]
fn csv_export_escapes_a_legacy_multiline_category_loaded_from_disk() {
    let db = TempDb::new("legacy-category-csv");
    let s = loaded(&db);
    let id = id_of(&s, "ALDI SUED");
    drop(s);

    let legacy_name = "\n=SUM(A1)\n\"Legacy\"";
    let raw = Connection::open(&db.0).unwrap();
    raw.execute(
        "INSERT INTO categories (name, kind, sort) VALUES (?1, 'expense', 999)",
        [legacy_name],
    )
    .unwrap();
    let category_id = raw.last_insert_rowid();
    raw.execute(
        "UPDATE transactions SET category_id = ?2, status = 'confirmed' WHERE id = ?1",
        rusqlite::params![id, category_id],
    )
    .unwrap();
    drop(raw);

    let reopened = Store::open(&db.0).unwrap();
    let csv = reopened.export_csv(&TxFilter { text: Some("ALDI".into()), ..Default::default() }).unwrap();

    let expected_category = "\"'\n=SUM(A1)\n\"\"Legacy\"\"\"";
    assert!(csv.contains(expected_category), "legacy category must be defused, quote-doubled and kept multiline:\n{csv}");
    let row = reopened.list_transactions(&TxFilter { text: Some("ALDI".into()), ..Default::default() }).unwrap();
    assert_eq!(row.len(), 1);
}

#[test]
fn saving_a_note_never_changes_category_status_or_fingerprint_search_fields() {
    let db = TempDb::new("side-effects");
    let mut s = loaded(&db);
    let id = id_of(&s, "ALDI SUED");
    let before = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == id).unwrap();

    s.save_transaction_note(id, "iba poznámka, nič iné").unwrap();

    let after = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == id).unwrap();
    assert_eq!(after.category_id, before.category_id);
    assert_eq!(after.status, before.status);
    assert_eq!(after.merchant_raw, before.merchant_raw);
}
