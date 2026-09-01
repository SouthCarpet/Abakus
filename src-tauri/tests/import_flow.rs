use abakus_lib::import_flow::{import_parsed, remember_password, ImportStatus};
use parser::{parse_text, AccountKind, Checksum, ParseError};
use std::cell::RefCell;
use store::Store;

fn fixture(name: &str) -> parser::Statement { parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/synthetic/").to_string() + name).unwrap()).unwrap() }

#[test]
fn unknown_account_report_carries_iban_and_kind_for_the_dialog() {
    let mut s = Store::open_in_memory().unwrap();
    let r = import_parsed(&mut s, "x.pdf", Ok(fixture("personal-2026-06.txt")), "h1");
    assert_eq!(r.status, ImportStatus::UnknownAccount);
    assert_eq!(r.iban.as_deref(), Some("SK4411000000000012345678"));
    assert_eq!(r.account_kind, Some(AccountKind::Personal));
    assert_eq!(r.iban_masked.as_deref(), Some("SK44...5678"));
}

#[test]
fn imported_report_has_counts_and_checksum() {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    let r = import_parsed(&mut s, "x.pdf", Ok(fixture("personal-2026-06.txt")), "h1");
    assert_eq!((r.status, r.inserted, r.duplicates, r.checksum), (ImportStatus::Imported, 8, 0, Some(Checksum::Ok)));
    assert_eq!(r.statement_number, Some(6));
    assert_eq!(r.account_label.as_deref(), Some("Osobný"));
}

/// A17/F5: the report carries the statement id a successful import wrote,
/// for both a fresh import and a re-import of the same file.
#[test]
fn imported_report_carries_the_statement_id_for_fresh_and_repeat_imports() {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    let first = import_parsed(&mut s, "x.pdf", Ok(fixture("personal-2026-06.txt")), "h1");
    assert_eq!(first.status, ImportStatus::Imported);
    let id = first.statement_id.expect("a successful import must carry a statement id");

    let repeat = import_parsed(&mut s, "x.pdf", Ok(fixture("personal-2026-06.txt")), "h1");
    assert_eq!(repeat.status, ImportStatus::AlreadyImported);
    assert_eq!(repeat.statement_id, Some(id), "the repeat import points at the same statement");
}

#[test]
fn encrypted_becomes_locked_and_other_errors_become_error() {
    let mut s = Store::open_in_memory().unwrap();
    assert_eq!(import_parsed(&mut s, "x.pdf", Err(ParseError::Encrypted), "h").status, ImportStatus::Locked);
    let r = import_parsed(&mut s, "x.pdf", Err(ParseError::NotAStatement("no IBAN line".into())), "h");
    assert_eq!(r.status, ImportStatus::Error);
    assert!(r.message.unwrap().contains("no IBAN line"));
}

/// Amendment A5: for an `unknown_account` report the remember-password write
/// never fires, proved with a recording fake instead of the real keyring.
#[test]
fn remember_password_is_not_called_for_an_unknown_account() {
    let mut s = Store::open_in_memory().unwrap();
    let r = import_parsed(&mut s, "x.pdf", Ok(fixture("personal-2026-06.txt")), "h1");
    assert_eq!(r.status, ImportStatus::UnknownAccount);

    let calls: RefCell<Vec<(String, String)>> = RefCell::new(Vec::new());
    let set_secret = |iban: &str, pw: &str| -> Result<(), String> { calls.borrow_mut().push((iban.into(), pw.into())); Ok(()) };
    let outcome = remember_password(&mut s, &r, "hunter2", &set_secret);

    assert!(outcome.is_ok());
    assert!(calls.borrow().is_empty(), "setter must not be called for unknown_account");
}

/// A12: a statement whose IBAN is on no registered account reports
/// `unknown_account` even when it is the only fixture on the store (the store
/// starts with zero accounts, mirroring a first-ever import).
#[test]
fn unknown_account_fixture_reports_unknown_account() {
    let mut s = Store::open_in_memory().unwrap();
    let unknown = parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/synthetic/edge-cases/unknown-account-2026-07.txt")).unwrap()).unwrap();
    let r = import_parsed(&mut s, "x.pdf", Ok(unknown), "h1");
    assert_eq!(r.status, ImportStatus::UnknownAccount);
    assert_eq!(r.iban.as_deref(), Some("SK8911000000000055555555"));
    assert_eq!(r.account_kind, Some(AccountKind::Personal));
}

/// Successful import does write through the injected setter and flips `has_password`.
#[test]
fn remember_password_writes_and_flags_the_account_on_success() {
    let mut s = Store::open_in_memory().unwrap();
    let id = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    let r = import_parsed(&mut s, "x.pdf", Ok(fixture("personal-2026-06.txt")), "h1");
    assert_eq!(r.status, ImportStatus::Imported);

    let calls: RefCell<Vec<(String, String)>> = RefCell::new(Vec::new());
    let set_secret = |iban: &str, pw: &str| -> Result<(), String> { calls.borrow_mut().push((iban.into(), pw.into())); Ok(()) };
    remember_password(&mut s, &r, "hunter2", &set_secret).unwrap();

    assert_eq!(calls.borrow().as_slice(), &[("SK4411000000000012345678".to_string(), "hunter2".to_string())]);
    let acc = s.list_accounts().unwrap().into_iter().find(|a| a.id == id).unwrap();
    assert!(acc.has_password);
}
