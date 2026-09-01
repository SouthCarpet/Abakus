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
            "Účet už má importované výpisy: {statements}, transakcie: {transactions}. Zmena typu účtu prepíše ich zaradenie v prehľadoch aj v exporte. Zmenu treba potvrdiť."
        ),
        other => other.to_string(),
    }
}

#[tauri::command]
pub fn clear_password(state: State<AppState>, account_id: i64) -> Result<(), String> {
    let mut s = lock(&state)?;
    let acc = s.list_accounts().map_err(|e| e.to_string())?.into_iter().find(|a| a.id == account_id).ok_or("no such account")?;
    secrets::clear(&acc.iban);
    s.set_has_password(account_id, false).map_err(|e| e.to_string())
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
    let csv = lock(&state)?.export_csv(&filter).map_err(|e| e.to_string())?;
    std::fs::write(&path, &csv).map_err(|e| e.to_string())?;
    Ok(csv.lines().count().saturating_sub(1))
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

