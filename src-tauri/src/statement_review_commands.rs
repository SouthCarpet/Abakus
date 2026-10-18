//! Read-only stored statement checklist. The UI consumes a separate DTO.
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn statement_review(state: State<AppState>, statement_id: i64) -> Result<store::StatementReview, String> {
    state.store.lock().map_err(|_| "store busy".to_string())?
        .statement_review(statement_id).map_err(|error| error.to_string())
}
