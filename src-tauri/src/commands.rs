//! Tauri command layer: thin wrappers that lock the store and delegate to
//! `store`/`import_flow`. Every command returns `Result<T, String>`.
use crate::import_flow::{self, import_path, ImportReport};
use crate::secrets;
use crate::state::AppState;
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
    let id = s.upsert_account(&iban, kind, &label).map_err(|e| e.to_string())?;
    s.reclassify_after_account_change().map_err(|e| e.to_string())?;
    s.list_accounts().map_err(|e| e.to_string())?.into_iter().find(|a| a.id == id).ok_or_else(|| "account vanished".into())
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

#[tauri::command]
pub fn summary(state: State<AppState>, from: Option<chrono::NaiveDate>, to: Option<chrono::NaiveDate>, account_id: Option<i64>) -> Result<store::Summary, String> {
    lock(&state)?.summary(from, to, account_id).map_err(|e| e.to_string())
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
