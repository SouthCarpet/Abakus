use crate::report::{write_pdf_report, PdfReportOutcome};
use crate::state::AppState;
use chrono::Local;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn preview_pdf_report(
    state: State<AppState>,
    request: store::report::ReportRequest,
) -> Result<store::report::ReportPreview, String> {
    let clock = report_clock();
    let mut store = state
        .store
        .lock()
        .map_err(|_| "Databáza je zaneprázdnená.".to_string())?;
    Ok(store
        .report_snapshot(&request, &clock)
        .map_err(|error| error.to_string())?
        .preview)
}

#[tauri::command]
pub async fn export_pdf_report(
    state: State<'_, AppState>,
    request: store::report::ReportRequest,
    path: String,
) -> Result<PdfReportOutcome, String> {
    let snapshot = {
        let clock = report_clock();
        let mut store = state
            .store
            .lock()
            .map_err(|_| "Databáza je zaneprázdnená.".to_string())?;
        store
            .report_snapshot(&request, &clock)
            .map_err(|error| error.to_string())?
    };
    let destination = PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || write_pdf_report(&snapshot, &destination))
        .await
        .map_err(|error| format!("Vytvorenie PDF sa neočakávane skončilo: {error}"))?
}

fn report_clock() -> store::report::ReportClock {
    let now = Local::now().fixed_offset();
    store::report::ReportClock {
        today: now.date_naive(),
        captured_at: now,
    }
}
