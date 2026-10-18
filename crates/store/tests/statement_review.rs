//! Oracles: plan 091 review21-contract.md and review21-test-plan.md.
#[path = "support/review.rs"]
mod support;
use parser::Checksum;
use store::{StatementReviewStatus, Store, StoreError};
use support::*;

#[test]
fn balanced_empty_import_has_known_empty_warnings_and_no_open_checks() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let id = store.import_statement(&statement(), "clean").unwrap().statement_id;
    let review = store.statement_review(id).unwrap();
    assert_eq!(review.statement_id, id);
    assert_eq!(review.checksum, Checksum::Ok);
    assert_eq!(review.parser_warnings, Some(vec![]));
    assert_eq!((review.unassigned_count, review.suggested_count), (0, 0));
    assert_eq!(review.status, StatementReviewStatus::NoOpenChecks);
}

#[test]
fn warning_order_duplicates_unicode_and_newlines_survive_reopen() {
    let db = Database::new("warnings");
    let mut store = db.open();
    account(&mut store);
    let mut input = statement();
    input.warnings = vec!["Žltý riadok\nďalší".into(), "druhé".into(), "Žltý riadok\nďalší".into()];
    let id = store.import_statement(&input, "warnings").unwrap().statement_id;
    drop(store);
    let review = db.open().statement_review(id).unwrap();
    assert_eq!(review.parser_warnings, Some(vec!["Žltý riadok\nďalší".into(), "druhé".into(), "Žltý riadok\nďalší".into()]));
    assert_eq!(review.status, StatementReviewStatus::NeedsAttention);
}

#[test]
fn absent_positive_zero_and_negative_ids_return_typed_unknown_statement() {
    let store = Store::open_in_memory().unwrap();
    assert!(matches!(store.statement_review(987), Err(StoreError::UnknownStatement { id: 987 })));
    assert!(matches!(store.statement_review(0), Err(StoreError::UnknownStatement { id: 0 })));
    assert!(matches!(store.statement_review(-1), Err(StoreError::UnknownStatement { id: -1 })));
}

#[test]
fn positive_checksum_mismatch_keeps_exact_cents_and_needs_attention() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let mut input = statement();
    input.closing_cents = Some(877);
    let id = store.import_statement(&input, "positive").unwrap().statement_id;
    let review = store.statement_review(id).unwrap();
    assert_eq!(review.checksum, Checksum::OffBy(123));
    assert_eq!(review.status, StatementReviewStatus::NeedsAttention);
}

#[test]
fn negative_checksum_mismatch_keeps_exact_cents_and_needs_attention() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let mut input = statement();
    input.closing_cents = Some(1123);
    let id = store.import_statement(&input, "negative").unwrap().statement_id;
    let review = store.statement_review(id).unwrap();
    assert_eq!(review.checksum, Checksum::OffBy(-123));
    assert_eq!(review.status, StatementReviewStatus::NeedsAttention);
}

#[test]
fn missing_balance_with_no_open_items_reports_incomplete_evidence() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let mut input = statement();
    input.opening_cents = None;
    let id = store.import_statement(&input, "unverifiable").unwrap().statement_id;
    let review = store.statement_review(id).unwrap();
    assert_eq!(review.checksum, Checksum::NotVerifiable);
    assert_eq!(review.parser_warnings, Some(vec![]));
    assert_eq!(review.status, StatementReviewStatus::EvidenceIncomplete);
}

#[test]
fn mixed_open_row_unknown_warnings_and_unverifiable_checksum_keep_all_evidence() {
    let db = Database::new("mixed");
    let mut store = db.open();
    account(&mut store);
    let mut input = statement();
    input.closing_cents = None;
    input.transactions = vec![transaction("Unknown test merchant")];
    let id = store.import_statement(&input, "mixed").unwrap().statement_id;
    db.raw().execute("UPDATE statements SET parser_warnings_json=NULL", []).unwrap();
    let review = store.statement_review(id).unwrap();
    assert_eq!(review.checksum, Checksum::NotVerifiable);
    assert_eq!(review.parser_warnings, None);
    assert_eq!((review.unassigned_count, review.suggested_count), (1, 0));
    assert_eq!(review.status, StatementReviewStatus::NeedsAttention);
}

#[test]
fn checksum_mismatch_with_unknown_warnings_needs_attention_and_preserves_unknown() {
    let db = Database::new("off-unknown");
    let mut store = db.open();
    account(&mut store);
    let mut input = statement();
    input.closing_cents = Some(877);
    let id = store.import_statement(&input, "off-unknown").unwrap().statement_id;
    db.raw().execute("UPDATE statements SET parser_warnings_json=NULL", []).unwrap();
    let review = store.statement_review(id).unwrap();
    assert_eq!(review.checksum, Checksum::OffBy(123));
    assert_eq!(review.parser_warnings, None);
    assert_eq!(review.status, StatementReviewStatus::NeedsAttention);
}

#[test]
fn counts_include_only_current_unassigned_and_suggested_owned_rows() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    store.upsert_account(OTHER_IBAN, parser::AccountKind::Business, "Druhý").unwrap();
    let mut transfer = transaction("Vlastný prevod");
    transfer.kind = parser::TxKind::TransferOut;
    transfer.counterparty_iban = Some(OTHER_IBAN.into());
    let mut input = statement();
    input.transactions = vec![transaction("Unknown test merchant"), transaction("ALDI SUED"), transaction("LIDL"), transaction("Confirm this merchant"), transfer];
    let id = store.import_statement(&input, "counts").unwrap().statement_id;
    let to_confirm = store.list_transactions(&store::TxFilter::default()).unwrap().into_iter().find(|row| row.merchant_raw == "Confirm this merchant").unwrap().id;
    let category = store.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    store.assign(&[to_confirm], category, false).unwrap();
    let review = store.statement_review(id).unwrap();
    assert_eq!((review.unassigned_count, review.suggested_count), (1, 2));
    assert_eq!(review.status, StatementReviewStatus::NeedsAttention);
}

#[test]
fn public_assignment_and_rule_deletion_reclassification_change_live_counts() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let mut input = statement();
    input.transactions = vec![transaction("Learn review merchant")];
    let first = store.import_statement(&input, "learn-first").unwrap().statement_id;
    let row_id = store.list_transactions(&store::TxFilter::default()).unwrap()[0].id;
    let category = store.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    store.assign(&[row_id], category, false).unwrap();
    assert_eq!(store.statement_review(first).unwrap().status, StatementReviewStatus::NoOpenChecks);
    input.transactions[0].raw_block = "second owned transaction".into();
    // A new place uses the learned merchant suggestion, not an exact match.
    input.transactions[0].place = Some("Nové miesto".into());
    let second = store.import_statement(&input, "learn-second").unwrap().statement_id;
    assert_eq!(store.statement_review(second).unwrap().suggested_count, 1);
    let learned = store.list_rules().unwrap().into_iter().filter(|rule| rule.kind != rules::RuleKind::Seed).collect::<Vec<_>>();
    for rule in learned { store.delete_rule(rule.id).unwrap(); }
    store.reclassify_open().unwrap();
    let review = store.statement_review(second).unwrap();
    assert_eq!((review.unassigned_count, review.suggested_count), (1, 0));
    assert_eq!(store.statement_review(first).unwrap().status, StatementReviewStatus::NoOpenChecks);
}

#[test]
fn statement_and_account_scopes_exclude_other_open_rows() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    store.upsert_account(OTHER_IBAN, parser::AccountKind::Business, "Druhý").unwrap();
    let mut input = statement();
    input.transactions = vec![transaction("Unknown one")];
    let first = store.import_statement(&input, "scope-first").unwrap().statement_id;
    input.transactions = vec![transaction("ALDI SUED"), transaction("LIDL")];
    let second = store.import_statement(&input, "scope-second").unwrap().statement_id;
    input.iban = OTHER_IBAN.into();
    input.account_kind = parser::AccountKind::Business;
    let other = store.import_statement(&input, "scope-other").unwrap().statement_id;
    let first_review = store.statement_review(first).unwrap();
    let second_review = store.statement_review(second).unwrap();
    let other_review = store.statement_review(other).unwrap();
    assert_eq!((first_review.unassigned_count, first_review.suggested_count), (1, 0));
    assert_eq!((second_review.unassigned_count, second_review.suggested_count), (0, 2));
    assert_eq!((other_review.unassigned_count, other_review.suggested_count), (0, 2));
}

#[test]
fn same_file_retry_reports_current_input_but_preserves_first_warning_evidence_and_rows() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let mut input = statement();
    input.warnings = vec!["first".into()];
    input.transactions = vec![transaction("Unknown retained")];
    let first = store.import_statement(&input, "retry").unwrap();
    input.warnings = vec!["retry".into()];
    input.closing_cents = Some(877);
    input.transactions = vec![transaction("ALDI SUED"), transaction("LIDL")];
    let retry = store.import_statement(&input, "retry").unwrap();
    assert_eq!((retry.statement_id, retry.inserted, retry.duplicates, retry.already_imported), (first.statement_id, 0, 2, true));
    assert_eq!(retry.warnings, vec!["retry"]);
    assert_eq!(retry.checksum, Checksum::OffBy(123));
    let review = store.statement_review(first.statement_id).unwrap();
    assert_eq!(review.parser_warnings, Some(vec!["first".into()]));
    assert_eq!(review.checksum, Checksum::Ok);
    assert_eq!((review.unassigned_count, review.suggested_count), (1, 0));
}

#[test]
fn reexport_saves_own_warnings_but_deduplicated_rows_keep_original_owner() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let mut input = statement();
    input.transactions = vec![transaction("Unknown retained")];
    input.warnings = vec!["original".into()];
    let first = store.import_statement(&input, "original").unwrap().statement_id;
    input.warnings = vec![];
    let reexport = store.import_statement(&input, "reexport").unwrap();
    assert_eq!((reexport.inserted, reexport.duplicates), (0, 1));
    let review = store.statement_review(reexport.statement_id).unwrap();
    assert_eq!(review.parser_warnings, Some(vec![]));
    assert_eq!((review.unassigned_count, review.suggested_count), (0, 0));
    assert_eq!(review.status, StatementReviewStatus::NoOpenChecks);
    assert_eq!(store.statement_review(first).unwrap().unassigned_count, 1);
    assert_eq!(store.statement_review(first).unwrap().parser_warnings, Some(vec!["original".into()]));
}

fn corrupt_warning_error(name: &str, json: &str) {
    let db = Database::new(name);
    let mut store = db.open();
    account(&mut store);
    let id = store.import_statement(&statement(), name).unwrap().statement_id;
    db.raw().execute("UPDATE statements SET parser_warnings_json=?1", [json]).unwrap();
    assert!(matches!(store.statement_review(id), Err(StoreError::Parse(_))), "invalid warning evidence must fail");
}

#[test]
fn malformed_warning_json_returns_parse_error() { corrupt_warning_error("malformed", "["); }
#[test]
fn scalar_warning_json_returns_parse_error() { corrupt_warning_error("scalar", "42"); }
#[test]
fn non_string_warning_array_returns_parse_error() { corrupt_warning_error("non-string", "[\"valid\",42]"); }
#[test]
fn json_null_is_invalid_evidence_instead_of_sql_null_unknown() { corrupt_warning_error("json-null", "null"); }

#[test]
fn missing_transaction_table_returns_database_error() {
    let db = Database::new("read-failure");
    let mut store = db.open();
    account(&mut store);
    let id = store.import_statement(&statement(), "read-failure").unwrap().statement_id;
    db.raw().execute_batch("DROP TABLE transactions").unwrap();
    assert!(matches!(store.statement_review(id), Err(StoreError::Db(_))));
}

#[test]
fn off_without_delta_returns_error_while_history_keeps_legacy_conversion() {
    let db = Database::new("off-missing");
    let mut store = db.open();
    account(&mut store);
    let id = store.import_statement(&statement(), "off-missing").unwrap().statement_id;
    db.raw().execute("UPDATE statements SET checksum_status='off',checksum_off_by=NULL", []).unwrap();
    assert!(matches!(store.statement_review(id), Err(StoreError::Parse(_))));
    assert_eq!(store.statement_history(None, None).unwrap()[0].checksum, Checksum::OffBy(0));
}

#[test]
fn unknown_checksum_status_returns_error_while_history_keeps_legacy_conversion() {
    let db = Database::new("checksum-unknown");
    let mut store = db.open();
    account(&mut store);
    let id = store.import_statement(&statement(), "checksum-unknown").unwrap().statement_id;
    db.raw().execute("UPDATE statements SET checksum_status='future'", []).unwrap();
    assert!(matches!(store.statement_review(id), Err(StoreError::Parse(_))));
    assert_eq!(store.statement_history(None, None).unwrap()[0].checksum, Checksum::NotVerifiable);
}

#[test]
fn text_checksum_delta_returns_database_error_instead_of_coercion() {
    let db = Database::new("checksum-text");
    let mut store = db.open();
    account(&mut store);
    let id = store.import_statement(&statement(), "checksum-text").unwrap().statement_id;
    db.raw().execute("UPDATE statements SET checksum_status='off',checksum_off_by='abc'", []).unwrap();
    assert!(matches!(store.statement_review(id), Err(StoreError::Db(_))));
}

#[test]
fn late_classification_failure_rolls_back_warning_row_transactions_and_rule_touches() {
    let db = Database::new("import-rollback");
    let mut store = db.open();
    account(&mut store);
    let before_rules = rows(&db.raw(), "SELECT * FROM rules ORDER BY id");
    db.raw().execute_batch("CREATE TRIGGER reject_second_classification BEFORE UPDATE OF status ON transactions WHEN NEW.merchant_raw='LIDL' AND EXISTS (SELECT 1 FROM transactions WHERE merchant_raw='ALDI SUED' AND status='suggested') BEGIN SELECT RAISE(ABORT,'synthetic late classification failure'); END;").unwrap();
    let mut input = statement();
    input.warnings = vec!["must roll back".into()];
    input.transactions = vec![transaction("ALDI SUED"), transaction("LIDL")];
    let error = store.import_statement(&input, "atomic").unwrap_err();
    assert!(error.to_string().contains("synthetic late classification failure"));
    assert_eq!(store.statement_count().unwrap(), 0);
    assert_eq!(rows(&db.raw(), "SELECT parser_warnings_json FROM statements"), Vec::<Vec<rusqlite::types::Value>>::new());
    assert!(store.list_transactions(&store::TxFilter::default()).unwrap().is_empty());
    assert_eq!(rows(&db.raw(), "SELECT * FROM rules ORDER BY id"), before_rules);
    db.raw().execute_batch("DROP TRIGGER reject_second_classification").unwrap();
    let retry = store.import_statement(&input, "atomic").unwrap();
    assert_eq!(retry.inserted, 2);
    assert_eq!(store.statement_review(retry.statement_id).unwrap().parser_warnings, Some(vec!["must roll back".into()]));
}

#[test]
fn statement_deletion_removes_its_warning_evidence_and_preserves_survivor() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let mut input = statement();
    input.warnings = vec!["removed".into()];
    let removed = store.import_statement(&input, "removed").unwrap().statement_id;
    input.warnings = vec!["survives".into()];
    let survivor = store.import_statement(&input, "survivor").unwrap().statement_id;
    store.delete_statement(removed).unwrap();
    assert!(matches!(store.statement_review(removed), Err(StoreError::UnknownStatement { id }) if id == removed));
    assert_eq!(store.statement_count().unwrap(), 1);
    assert_eq!(store.statement_review(survivor).unwrap().parser_warnings, Some(vec!["survives".into()]));
}

#[test]
fn account_deletion_removes_all_its_warning_evidence_and_preserves_other_account() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    store.upsert_account(OTHER_IBAN, parser::AccountKind::Business, "Druhý").unwrap();
    let mut input = statement();
    input.warnings = vec!["removed".into()];
    let first = store.import_statement(&input, "delete-account-first").unwrap().statement_id;
    let second = store.import_statement(&input, "delete-account-second").unwrap().statement_id;
    input.iban = OTHER_IBAN.into();
    input.account_kind = parser::AccountKind::Business;
    input.warnings = vec!["survivor".into()];
    let survivor = store.import_statement(&input, "delete-account-survivor").unwrap().statement_id;
    let account_id = store.account_by_iban(IBAN).unwrap().unwrap().id;
    store.delete_account(account_id).unwrap();
    assert!(matches!(store.statement_review(first), Err(StoreError::UnknownStatement { id }) if id == first));
    assert!(matches!(store.statement_review(second), Err(StoreError::UnknownStatement { id }) if id == second));
    assert_eq!(store.statement_count().unwrap(), 1);
    assert_eq!(store.statement_review(survivor).unwrap().parser_warnings, Some(vec!["survivor".into()]));
}

#[test]
fn known_empty_evidence_stays_empty_after_same_file_warning_retry() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let mut input = statement();
    let id = store.import_statement(&input, "clean-retry").unwrap().statement_id;
    input.warnings = vec!["retry warning".into()];
    store.import_statement(&input, "clean-retry").unwrap();
    assert_eq!(store.statement_review(id).unwrap().parser_warnings, Some(vec![]));
    assert_eq!(store.statement_review(id).unwrap().status, StatementReviewStatus::NoOpenChecks);
}

#[test]
fn known_warnings_take_precedence_over_unverifiable_checksum_without_open_rows() {
    let mut store = Store::open_in_memory().unwrap();
    account(&mut store);
    let mut input = statement();
    input.opening_cents = None;
    input.warnings = vec!["warning".into()];
    let id = store.import_statement(&input, "warning-unverifiable").unwrap().statement_id;
    let review = store.statement_review(id).unwrap();
    assert_eq!(review.checksum, Checksum::NotVerifiable);
    assert_eq!(review.parser_warnings, Some(vec!["warning".into()]));
    assert_eq!((review.unassigned_count, review.suggested_count), (0, 0));
    assert_eq!(review.status, StatementReviewStatus::NeedsAttention);
}

#[test]
fn ok_checksum_with_unexpected_delta_returns_error() {
    let db = Database::new("ok-delta");
    let mut store = db.open();
    account(&mut store);
    let id = store.import_statement(&statement(), "ok-delta").unwrap().statement_id;
    db.raw().execute("UPDATE statements SET checksum_off_by=123", []).unwrap();
    assert!(matches!(store.statement_review(id), Err(StoreError::Parse(_))));
}

#[test]
fn repeated_review_reads_leave_all_stored_rows_and_schema_unchanged() {
    let db = Database::new("readonly");
    let mut store = db.open();
    account(&mut store);
    let mut input = statement();
    input.warnings = vec!["stored warning".into()];
    input.transactions = vec![transaction("ALDI SUED")];
    let id = store.import_statement(&input, "readonly").unwrap().statement_id;
    let statements_before = rows(&db.raw(), "SELECT * FROM statements");
    let transactions_before = rows(&db.raw(), "SELECT * FROM transactions");
    let rules_before = rows(&db.raw(), "SELECT * FROM rules ORDER BY id");
    let schema_before = rows(&db.raw(), "SELECT * FROM sqlite_master ORDER BY type,name");
    assert_eq!(store.statement_review(id).unwrap(), store.statement_review(id).unwrap());
    assert_eq!(rows(&db.raw(), "SELECT * FROM statements"), statements_before);
    assert_eq!(rows(&db.raw(), "SELECT * FROM transactions"), transactions_before);
    assert_eq!(rows(&db.raw(), "SELECT * FROM rules ORDER BY id"), rules_before);
    assert_eq!(rows(&db.raw(), "SELECT * FROM sqlite_master ORDER BY type,name"), schema_before);
}
