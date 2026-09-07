//! Tauri command layer: thin wrappers that lock the store and delegate to
//! `store`/`import_flow`. Every command returns `Result<T, String>`.
use crate::import_flow::{self, import_path, ImportReport};
use crate::secrets;
use crate::state::AppState;
use crate::{net, update};
use std::sync::Mutex;
use tauri::State;

fn lock<'a>(state: &'a State<AppState>) -> Result<std::sync::MutexGuard<'a, store::Store>, String> {
    state.store.lock().map_err(|_| "store busy".to_string())
}

/// Note: `parser::parse_pdf` (via `import_flow::import_path`) serializes on
/// pdfium's internal lock, so a batch import runs its PDF parsing one file at
/// a time even though the loop below is sequential anyway.
#[tauri::command]
pub fn import_statements(state: State<AppState>, paths: Vec<String>) -> Result<Vec<ImportReport>, String> {
    let mut s = lock(&state)?;
    Ok(paths.iter().map(|p| import_path(&mut s, std::path::Path::new(p), None, &secrets::get)).collect())
}

#[tauri::command]
pub fn import_with_password(state: State<AppState>, path: String, password: String, remember: bool) -> Result<ImportReport, String> {
    let mut s = lock(&state)?;
    let r = import_path(&mut s, std::path::Path::new(&path), Some(&password), &secrets::get);
    if remember { import_flow::remember_password(&mut s, &r, &password, &secrets::set)?; }
    Ok(r)
}

#[tauri::command]
pub fn list_accounts(state: State<AppState>) -> Result<Vec<store::Account>, String> {
    lock(&state)?.list_accounts().map_err(|e| e.to_string())
}

/// Spec D4: reclassifies open/transfer rows against the new account set right
/// after the upsert, so a newly-added own IBAN flips its own transfers immediately.
#[tauri::command]
pub fn save_account(state: State<AppState>, iban: String, kind: parser::AccountKind, label: String) -> Result<store::Account, String> {
    if !parser::iban::is_valid(&iban) { return Err("IBAN nie je platný".into()); }
    let mut s = lock(&state)?;
    let id = s.upsert_account(&iban, kind, &label).map_err(account_error)?;
    s.reclassify_after_account_change().map_err(|e| e.to_string())?;
    s.list_accounts().map_err(|e| e.to_string())?.into_iter().find(|a| a.id == id).ok_or_else(|| "account vanished".into())
}

/// A17/F2: edits an existing account. The IBAN is not a parameter, so an edit
/// can never move an account's history onto another IBAN. The label is free.
/// The kind is refused while the account has imports until the UI sends
/// `acknowledgeKindChange`, and an acknowledged change reclassifies (spec D4).
#[tauri::command]
pub fn update_account(state: State<AppState>, id: i64, label: String, kind: parser::AccountKind, acknowledge_kind_change: bool) -> Result<store::Account, String> {
    lock(&state)?.update_account(id, &label, kind, acknowledge_kind_change).map_err(account_error)
}

/// The refusal names what would be recast, so the UI can show a truthful
/// warning instead of a generic failure.
fn account_error(e: store::StoreError) -> String {
    match e {
        store::StoreError::AccountKindLocked { statements, transactions, .. } => format!(
            "Počet výpisov: {statements}. Počet transakcií: {transactions}. Zmena typu účtu zmení ich zaradenie v Prehľade aj v exporte. Ak chcete pokračovať, potvrďte zmenu."
        ),
        other => other.to_string(),
    }
}

#[tauri::command]
pub fn clear_password(state: State<AppState>, account_id: i64) -> Result<(), String> {
    let mut s = lock(&state)?;
    let acc = s.list_accounts().map_err(|e| e.to_string())?.into_iter().find(|a| a.id == account_id).ok_or("no such account")?;
    secrets::clear(&acc.iban)?;
    s.set_has_password(account_id, false).map_err(|e| e.to_string())
}

/// 078: what a delete of this account would remove, for the confirmation
/// text. Runs the same rule query the delete runs (mirrors
/// `statement_delete_preview`), so the numbers the user confirms are the
/// numbers the delete produces. Read-only: never touches the database.
#[tauri::command]
pub fn account_delete_preview(state: State<AppState>, account_id: i64) -> Result<store::AccountDeletePreview, String> {
    lock(&state)?.account_delete_preview(account_id).map_err(account_delete_error)
}

/// 078: deletes one account with its statements, transactions, and the
/// learned rules only it produced. A keyring and SQLite have no shared
/// commit, so this orders the two steps to protect the user's data: the
/// credential removal runs FIRST, while the account still exists, and the
/// database delete only runs once that has actually succeeded. A keyring
/// failure (locked vault, backend error) then leaves the account and all its
/// data exactly as they were, so the user has a working retry, not a gone
/// account and an orphaned credential.
///
/// Residual boundary, undocumented no further: the two systems still cannot
/// share one atomic commit, so if the keyring clear succeeds but the
/// database delete then fails (rolled back internally, see
/// `crates/store/src/delete_account.rs`'s own commit-failure handling), the
/// account survives with `has_password` still true while the credential is
/// already gone. This is not silently lost: `secrets::clear` is idempotent
/// (a missing credential is `Ok`, see its doc comment), so the user's retry
/// calls the keyring again, finds nothing there, and proceeds straight to a
/// successful database delete. No account is ever left with a stale
/// `has_password` flag AND a live credential removed out from under it.
#[tauri::command]
pub fn delete_account(state: State<AppState>, account_id: i64) -> Result<store::AccountDeleteOutcome, String> {
    let mut s = lock(&state)?;
    delete_account_inner(&mut s, account_id, &secrets::clear)
}

/// The testable body of `delete_account`: `clear_secret` stands in for the
/// real keyring removal (same seam shape as `import_flow::remember_password`'s
/// `set_secret`), so a test can inject a failing fake and prove the
/// credential-cleanup failure boundary without ever touching a real
/// credential. An account with no stored password never touches the keyring
/// at all.
fn delete_account_inner(s: &mut store::Store, account_id: i64, clear_secret: &dyn Fn(&str) -> Result<(), String>) -> Result<store::AccountDeleteOutcome, String> {
    let account = s
        .account_by_id(account_id)
        .map_err(|e| e.to_string())?
        .ok_or(store::StoreError::UnknownAccountId { id: account_id })
        .map_err(account_delete_error)?;
    if account.has_password {
        clear_secret(&account.iban).map_err(|e| format!("Odstránenie hesla zlyhalo, účet nebol vymazaný: {e}"))?;
    }
    s.delete_account(account_id).map_err(account_delete_error)
}

fn account_delete_error(e: store::StoreError) -> String {
    match e {
        store::StoreError::UnknownAccountId { id } => format!("Účet s id {id} neexistuje."),
        other => other.to_string(),
    }
}

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> Result<Vec<store::Category>, String> {
    lock(&state)?.list_categories().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_category(state: State<AppState>, id: Option<i64>, parent_id: Option<i64>, name: String, kind: store::CategoryKind) -> Result<store::Category, String> {
    lock(&state)?.save_category(id, parent_id, &name, kind).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn archive_category(state: State<AppState>, id: i64) -> Result<(), String> {
    lock(&state)?.archive_category(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_rules(state: State<AppState>) -> Result<Vec<store::RuleView>, String> {
    lock(&state)?.list_rules_view().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_rule(state: State<AppState>, id: i64) -> Result<(), String> {
    lock(&state)?.delete_rule(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_transactions(state: State<AppState>, filter: store::TxFilter) -> Result<Vec<store::TxRow>, String> {
    lock(&state)?.list_transactions(&filter).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn assign(state: State<AppState>, ids: Vec<i64>, category_id: i64, apply_to_matching: bool) -> Result<store::AssignOutcome, String> {
    lock(&state)?.assign(&ids, category_id, apply_to_matching).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn confirm(state: State<AppState>, ids: Vec<i64>) -> Result<usize, String> {
    lock(&state)?.confirm(&ids).map_err(|e| e.to_string())
}

/// A17/F3: `accountKind` covers EVERY account of that kind. The screen used to
/// pass the id of the first account of a kind, which silently dropped a second
/// personal or business account out of the totals.
#[tauri::command]
pub fn summary(state: State<AppState>, from: Option<chrono::NaiveDate>, to: Option<chrono::NaiveDate>, account_id: Option<i64>, account_kind: Option<parser::AccountKind>) -> Result<store::Summary, String> {
    lock(&state)?.summary_filtered(from, to, account_id, account_kind).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_csv(state: State<AppState>, filter: store::TxFilter, path: String) -> Result<usize, String> {
    let s = lock(&state)?;
    export_csv_inner(&s, &filter, &path)
}

/// The testable body of `export_csv`, same seam shape as
/// `delete_account_inner`. The exported row count comes from the same
/// filtered query `export_csv` itself runs, not from counting `\n` in the
/// finished text: a field can legitimately contain an embedded newline (it
/// is CSV-quoted, per `csv_quote`), and `.lines().count()` then reports more
/// rows than were actually exported. Regression:
/// `export_csv_counts_records_not_lines_when_a_field_has_an_embedded_newline`.
fn export_csv_inner(s: &store::Store, filter: &store::TxFilter, path: &str) -> Result<usize, String> {
    let record_count = s.list_transactions(filter).map_err(|e| e.to_string())?.len();
    let csv = s.export_csv(filter).map_err(|e| e.to_string())?;
    std::fs::write(path, &csv).map_err(|e| e.to_string())?;
    Ok(record_count)
}

#[tauri::command]
pub fn bad_checksums(state: State<AppState>) -> Result<Vec<store::BadChecksum>, String> {
    lock(&state)?.statements_with_bad_checksum().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn data_dir(state: State<AppState>) -> String {
    state.data_dir.to_string_lossy().to_string()
}

#[tauri::command]
pub fn recent_statements(state: State<AppState>, limit: usize) -> Result<Vec<store::RecentStatement>, String> {
    lock(&state)?.recent_statements(limit).map_err(|e| e.to_string())
}

/// 0.1.2: every statement matching BOTH optional filters, oldest first per
/// account. Distinct from `recent_statements` (newest-first, capped, for the
/// Import screen's own history section).
#[tauri::command]
pub fn statement_history(state: State<AppState>, account_id: Option<i64>, account_kind: Option<parser::AccountKind>) -> Result<Vec<store::StatementHistoryRow>, String> {
    lock(&state)?.statement_history(account_id, account_kind).map_err(|e| e.to_string())
}

/// 0.1.2: exact text including newlines, up to `store::NOTE_MAX_CHARS`
/// Unicode code points; an empty string clears the note. No learning,
/// category or fingerprint change.
#[tauri::command]
pub fn save_transaction_note(state: State<AppState>, id: i64, note: String) -> Result<(), String> {
    lock(&state)?.save_transaction_note(id, &note).map_err(|e| e.to_string())
}

/// 0.1.2: a consistent snapshot of the live database at a user-chosen local
/// path. Never touches `secrets`/the OS keyring: the store alone decides
/// what a backup contains.
#[tauri::command]
pub fn backup_database(state: State<AppState>, path: String) -> Result<store::BackupOutcome, String> {
    lock(&state)?.backup_to(std::path::Path::new(&path)).map_err(|e| e.to_string())
}

/// A17/F1: what a delete of this statement would remove, for the confirmation
/// text. It runs the same rule query the delete runs, so the numbers the user
/// confirms are the numbers the delete produces.
#[tauri::command]
pub fn statement_delete_preview(state: State<AppState>, statement_id: i64) -> Result<store::StatementDeletePreview, String> {
    lock(&state)?.statement_delete_preview(statement_id).map_err(|e| e.to_string())
}

/// A17/F1: deletes one imported statement with its transactions and the rules
/// only it produced. All or nothing; returns what was removed.
#[tauri::command]
pub fn delete_statement(state: State<AppState>, statement_id: i64) -> Result<store::StatementDeleteOutcome, String> {
    lock(&state)?.delete_statement(statement_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_check_updates(state: State<AppState>) -> Result<bool, String> {
    lock(&state)?.get_check_updates().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_check_updates(state: State<AppState>, on: bool) -> Result<(), String> {
    lock(&state)?.set_check_updates(on).map_err(|e| e.to_string())
}

/// Every failure (offline, rate limit, malformed body, nothing newer)
/// collapses to `None`; the UI never distinguishes them (spec A14).
#[tauri::command]
pub async fn check_update_now(state: State<'_, AppState>) -> Result<Option<update::Release>, String> {
    check_update_now_inner(&state.store, env!("CARGO_PKG_VERSION"), net::send_request).await
}

/// The testable body of `check_update_now`: reads the opt-in setting itself
/// rather than trusting the caller to have checked it first, so any command
/// invocation (not only the one path the UI happens to use) is gated at the
/// same place. `send` stands in for the real network transport, so a test
/// can prove the gate with a recording fake that must never fire while the
/// setting is off (Grok review t15b).
pub async fn check_update_now_inner<F, Fut>(
    store: &Mutex<store::Store>,
    version: &str,
    send: F,
) -> Result<Option<update::Release>, String>
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Result<(u16, Vec<u8>), String>>,
{
    let check_updates = store.lock().map_err(|_| "store busy".to_string())?.get_check_updates().map_err(|e| e.to_string())?;
    if !update::wants_check(check_updates) {
        return Ok(None);
    }
    Ok(update::check_with(version, store, send).await.ok().flatten())
}

#[tauri::command]
pub fn net_log(state: State<AppState>, limit: usize) -> Result<Vec<store::NetLogRow>, String> {
    lock(&state)?.net_log(limit).map_err(|e| e.to_string())
}

/// A17/F7: audit rows that could not be written. Empty is the normal answer;
/// anything else means the network log is incomplete and says so.
#[tauri::command]
pub fn net_audit_failures() -> Vec<net::AuditFailure> {
    net::audit_failures()
}

#[tauri::command]
pub fn run_net_audit(state: State<AppState>) -> Result<usize, String> {
    let mut s = lock(&state)?;
    net::sample_connections(&mut s)
}

/// 078: `delete_account_inner`'s credential-cleanup failure boundary, tested
/// with a fake `clear_secret` (never the real keyring, per the audit's
/// constraint). `account_delete_error`'s Slovak wording is tested here too:
/// it is a pure function, so no `State`/fixture is needed for it.
#[cfg(test)]
mod tests {
    use super::*;
    use parser::AccountKind;
    use std::cell::RefCell;
    use store::Store;

    fn fixture(name: &str) -> parser::Statement {
        parser::parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/synthetic/").to_string() + name).unwrap()).unwrap()
    }

    #[test]
    fn account_delete_error_states_the_unknown_id_in_slovak() {
        let msg = account_delete_error(store::StoreError::UnknownAccountId { id: 42 });
        assert_eq!(msg, "Účet s id 42 neexistuje.");
    }

    #[test]
    fn account_delete_error_passes_other_errors_through() {
        let msg = account_delete_error(store::StoreError::Parse("čokoľvek".into()));
        assert_eq!(msg, "čokoľvek");
    }

    #[test]
    fn deleting_an_unknown_account_is_a_clear_slovak_error() {
        let mut s = Store::open_in_memory().unwrap();
        let never = |_: &str| -> Result<(), String> { panic!("must not be called: no account was found") };

        let e = delete_account_inner(&mut s, 9_999, &never).unwrap_err();

        assert_eq!(e, "Účet s id 9999 neexistuje.");
    }

    #[test]
    fn deleting_an_account_without_a_password_never_touches_the_keyring() {
        let mut s = Store::open_in_memory().unwrap();
        let id = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        let calls: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let record = |iban: &str| -> Result<(), String> { calls.borrow_mut().push(iban.into()); Ok(()) };

        let outcome = delete_account_inner(&mut s, id, &record).unwrap();

        assert_eq!(outcome.account_id, id);
        assert!(calls.borrow().is_empty(), "an account with no stored password must never call the keyring");
        assert!(s.account_by_id(id).unwrap().is_none());
    }

    /// The exact boundary the 078 review correction asked for named and
    /// tested: a keyring failure now runs BEFORE the database delete, so the
    /// account and every one of its rows survive it, giving the user a
    /// working retry instead of a gone account and an orphaned credential.
    #[test]
    fn a_credential_cleanup_failure_leaves_the_account_and_its_data_intact_for_a_retry() {
        let mut s = Store::open_in_memory().unwrap();
        let id = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        s.set_has_password(id, true).unwrap();
        s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
        let failing = |_: &str| -> Result<(), String> { Err("keyring locked".into()) };

        let e = delete_account_inner(&mut s, id, &failing).unwrap_err();

        assert!(e.contains("keyring locked"), "the real cause must reach the caller: {e}");
        assert!(!e.to_lowercase().contains("success"), "a credential failure must never read as a success: {e}");
        assert!(s.account_by_id(id).unwrap().is_some(), "a keyring failure must not delete the account: the user needs a handle to retry");
        assert_eq!(s.list_transactions(&store::TxFilter { account_id: Some(id), ..Default::default() }).unwrap().len(), 8, "no data may be lost when only the credential step failed");
    }

    /// The keyring step runs and succeeds; only then does the database
    /// delete run and actually remove the data.
    /// Recurring/category contract §9: category names are now validated
    /// (trimmed, non-empty, ≤100 scalar values, no control characters), so
    /// `\n` in a category name is rejected and can no longer stand in for the
    /// CSV escaping test's multiline field. `note` (0.1.2) is deliberately
    /// exempt from that validation and already documented in
    /// `summary::csv_quote` as "the first multiline field this function ever
    /// sees" (its own `\n`-in-`DANGEROUS_LEADING` addition), so it is the
    /// real, currently-reachable multiline field and replaces the category
    /// name here through the real public API, no raw SQL needed. The old
    /// `csv.lines().count() - 1` would have reported one row too many for
    /// this exact case.
    #[test]
    fn export_csv_counts_records_not_lines_when_a_field_has_an_embedded_newline() {
        let mut s = Store::open_in_memory().unwrap();
        s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
        let ids: Vec<i64> = s.list_transactions(&store::TxFilter::default()).unwrap().into_iter().map(|r| r.id).collect();
        let expected = ids.len();
        s.save_transaction_note(ids[0], "Multi\nline").unwrap();
        let path = std::env::temp_dir().join(format!("abakus-test-export-{}.csv", std::process::id()));
        let path = path.to_str().unwrap();

        let count = export_csv_inner(&s, &store::TxFilter::default(), path).unwrap();

        let written = std::fs::read_to_string(path).unwrap();
        std::fs::remove_file(path).ok();
        assert!(written.contains("Multi\nline"), "the embedded newline must reach the file, quoted, not stripped");
        assert_eq!(count, expected, "the count must be the number of exported records, not the number of text lines");
        assert_ne!(count, written.lines().count().saturating_sub(1), "this fixture must actually exercise the bug: line-counting must disagree with the real record count");
    }

    #[test]
    fn a_successful_credential_cleanup_runs_before_the_database_delete_and_clears_the_right_iban() {
        let mut s = Store::open_in_memory().unwrap();
        let id = s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        s.set_has_password(id, true).unwrap();
        s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
        let calls: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let record = |iban: &str| -> Result<(), String> { calls.borrow_mut().push(iban.into()); Ok(()) };

        let outcome = delete_account_inner(&mut s, id, &record).unwrap();

        assert_eq!((outcome.statements_deleted, outcome.transactions_deleted), (1, 8));
        assert_eq!(calls.borrow().as_slice(), &["SK4411000000000012345678".to_string()]);
        assert!(s.account_by_id(id).unwrap().is_none(), "the database delete runs once the credential step has actually succeeded");
    }
}
