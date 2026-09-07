//! Thin Tauri wrappers for the recurring feature (recurring-contract.md §7).
//! Same shape as `commands.rs`: lock the store, delegate, map errors to
//! `String`.
use crate::state::AppState;
use store::recurring::{RecurringDetail, RecurringDetailRequest, RecurringDecision, RecurringOverview, RecurringQuery, SaveRecurringRequest, TransactionRecurringContext};
use tauri::State;

fn lock<'a>(state: &'a State<AppState>) -> Result<std::sync::MutexGuard<'a, store::Store>, String> {
    state.store.lock().map_err(|_| "store busy".to_string())
}

#[tauri::command]
pub fn recurring_overview(state: State<AppState>, query: RecurringQuery) -> Result<RecurringOverview, String> {
    lock(&state)?.recurring_overview(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn recurring_detail(state: State<AppState>, request: RecurringDetailRequest) -> Result<RecurringDetail, String> {
    lock(&state)?.recurring_detail(&request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn transaction_recurring_context(state: State<AppState>, transaction_id: i64, as_of: chrono::NaiveDate) -> Result<TransactionRecurringContext, String> {
    lock(&state)?.transaction_recurring_context(transaction_id, as_of).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_recurring(state: State<AppState>, request: SaveRecurringRequest) -> Result<RecurringDecision, String> {
    lock(&state)?.save_recurring(&request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_recurring(state: State<AppState>, decision_id: i64) -> Result<(), String> {
    lock(&state)?.reset_recurring(decision_id).map_err(|e| e.to_string())
}
