use crate::report::{
    write_pdf_report, write_summary_pdf_report, PdfReportOutcome, ReportDocumentScope,
    SummaryLength,
};
use crate::state::AppState;
use chrono::Local;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn preview_pdf_report(
    state: State<AppState>,
    request: store::report::ReportRequest,
    scope: Option<ReportDocumentScope>,
) -> Result<store::report::ReportPreview, String> {
    let clock = report_clock();
    let mut store = state
        .store
        .lock()
        .map_err(|_| "Databáza je zaneprázdnená.".to_string())?;
    let captured = capture(&mut store, &request, &clock, scope.unwrap_or_default())?;
    Ok(captured.preview().clone())
}

#[tauri::command]
pub async fn export_pdf_report(
    state: State<'_, AppState>,
    request: store::report::ReportRequest,
    path: String,
    scope: Option<ReportDocumentScope>,
) -> Result<PdfReportOutcome, String> {
    let scope = scope.unwrap_or_default();
    let captured = {
        let clock = report_clock();
        let mut store = state
            .store
            .lock()
            .map_err(|_| "Databáza je zaneprázdnená.".to_string())?;
        capture(&mut store, &request, &clock, scope)?
    };
    let destination = PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || captured.write(&destination))
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

/// What one export captured under the store lock. The full report reads
/// only the snapshot, a summary also reads its extras in the same snapshot.
enum Captured {
    Full(Box<store::report::ReportSnapshot>),
    Summary(Box<store::report::SummarySnapshot>, SummaryLength),
}

impl Captured {
    fn preview(&self) -> &store::report::ReportPreview {
        match self {
            Self::Full(snapshot) => &snapshot.preview,
            Self::Summary(summary, _) => &summary.snapshot.preview,
        }
    }

    fn write(&self, destination: &std::path::Path) -> Result<PdfReportOutcome, String> {
        match self {
            Self::Full(snapshot) => write_pdf_report(snapshot, destination),
            Self::Summary(summary, length) => {
                write_summary_pdf_report(summary, *length, destination)
            }
        }
    }
}

fn capture(
    store: &mut store::Store,
    request: &store::report::ReportRequest,
    clock: &store::report::ReportClock,
    scope: ReportDocumentScope,
) -> Result<Captured, String> {
    match scope.summary_length() {
        None => store
            .report_snapshot(request, clock)
            .map(|snapshot| Captured::Full(Box::new(snapshot)))
            .map_err(|error| error.to_string()),
        Some(length) => store
            .report_summary_snapshot(request, clock)
            .map(|summary| Captured::Summary(Box::new(summary), length))
            .map_err(|error| error.to_string()),
    }
}
