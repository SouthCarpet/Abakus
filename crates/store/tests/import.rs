use parser::{parse_text, AccountKind, Checksum};
use rules::Status;
use store::Store;

fn fixture(name: &str) -> parser::Statement { parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap()).unwrap() }
fn store_with_accounts() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
    s
}

#[test] fn seeds_every_category_and_every_seed_rule_resolves() {
    let s = Store::open_in_memory().unwrap();
    let cats = s.list_categories().unwrap();
    assert!(cats.iter().any(|c| c.name == "Nezaradené" && c.system)); assert!(cats.iter().any(|c| c.name == "potraviny" && c.parent_id.is_some()));
    assert!(s.category_by_path("Jedlo/potraviny").unwrap().is_some());
    assert!(s.list_rules().unwrap().iter().all(|r| r.kind == rules::RuleKind::Seed)); assert!(s.list_rules().unwrap().len() >= 40);
}
#[test] fn unknown_account_is_refused_with_iban_and_kind() {
    let mut s = Store::open_in_memory().unwrap();
    let e = s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap_err();
    assert!(matches!(e, store::StoreError::UnknownAccount { ref iban, kind: AccountKind::Personal } if iban == "SK4411000000000012345678"));
}
#[test] fn import_inserts_all_and_reports_checksum() {
    let mut s = store_with_accounts();
    let o = s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    assert_eq!((o.inserted, o.duplicates, o.checksum, o.already_imported), (8, 0, Checksum::Ok, false));
}
#[test] fn same_file_twice_is_already_imported() {
    let mut s = store_with_accounts();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    let o = s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    assert!(o.already_imported); assert_eq!(o.inserted, 0);
}
#[test] fn same_transactions_from_another_file_are_duplicates() {
    let mut s = store_with_accounts();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    let o = s.import_statement(&fixture("personal-2026-06.txt"), "h2").unwrap();
    assert_eq!((o.inserted, o.duplicates), (0, 8));
    // A2: no UNIQUE(account_id, number, period_end) on `statements`, so the re-export under a
    // second file hash gets its own statements row; only the transactions dedupe by fingerprint.
    assert_eq!(s.statement_count().unwrap(), 2);
    assert_eq!(o.checksum, Checksum::Ok);
}
#[test] fn own_account_transfer_and_seed_suggestions_are_classified_on_import() {
    let mut s = store_with_accounts();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    // TxFilter is a unit struct today; Task 10 gives it fields, so keep the
    // `::default()` call site stable instead of collapsing it to a bare value.
    #[allow(clippy::default_constructed_unit_structs)]
    let rows = s.list_transactions(&store::TxFilter::default()).unwrap();
    let by = |m: &str| rows.iter().find(|r| r.merchant_raw == m).unwrap();
    assert_eq!(by("Jana Vzorová").status, Status::Transfer);
    assert_eq!(by("ALDI SUED").status, Status::Suggested); assert_eq!(by("ALDI SUED").category_name.as_deref(), Some("potraviny"));
    assert_eq!(by("Sparkasse Neuss").category_name.as_deref(), Some("bankomat"));
    assert_eq!(by("Poplatok za vedenie účtu").status, Status::Suggested);
    assert_eq!(by("OF").status, Status::Unassigned);
}
