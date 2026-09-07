//! Section 9 category workflow commands (Lane C): thin wrappers over the
//! validated `store::categories`/`store::rules_repo` implementations. Same
//! shape as the other command modules: lock, delegate, map the error to a
//! plain `String`.
use crate::state::AppState;
use std::sync::MutexGuard;
use tauri::State;

fn lock<'a>(state: &'a State<AppState>) -> Result<MutexGuard<'a, store::Store>, String> {
    state.store.lock().map_err(|_| "store busy".to_string())
}

#[tauri::command]
pub fn category_update_preview(state: State<AppState>, request: store::CategoryUpdateRequest) -> Result<store::CategoryUpdatePreview, String> {
    lock(&state)?.category_update_preview(&request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_category(state: State<AppState>, request: store::CategoryUpdateRequest) -> Result<store::Category, String> {
    lock(&state)?.update_category(&request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn seed_rule_for_transaction(state: State<AppState>, transaction_id: i64) -> Result<Option<store::RuleView>, String> {
    lock(&state)?.seed_rule_for_transaction(transaction_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_rule_category(state: State<AppState>, rule_id: i64, category_id: i64) -> Result<store::RuleRedirectOutcome, String> {
    lock(&state)?.update_rule_category(rule_id, category_id).map_err(|e| e.to_string())
}
