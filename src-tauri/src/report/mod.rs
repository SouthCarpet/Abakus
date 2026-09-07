//! Bounded vector PDF rendering and no-clobber publication.
mod font;
mod layout;
mod pdf;
mod publish;

use serde::Serialize;
use std::path::Path;

pub use layout::PdfReportLimits;
pub use publish::PdfIoFault;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PdfReportOutcome {
    pub path: String,
    pub bytes: u64,
    pub pages: u32,
    pub report: store::report::ReportPreview,
}

pub fn write_pdf_report(
    snapshot: &store::report::ReportSnapshot,
    destination: &Path,
) -> Result<PdfReportOutcome, String> {
    write_pdf_report_with_limits(snapshot, destination, PdfReportLimits::default())
}

#[doc(hidden)]
pub fn write_pdf_report_with_limits(
    snapshot: &store::report::ReportSnapshot,
    destination: &Path,
    limits: PdfReportLimits,
) -> Result<PdfReportOutcome, String> {
    publish::write(snapshot, destination, limits, PdfIoFault::None)
}

#[doc(hidden)]
pub fn write_pdf_report_with_fault(
    snapshot: &store::report::ReportSnapshot,
    destination: &Path,
    limits: PdfReportLimits,
    fault: PdfIoFault,
) -> Result<PdfReportOutcome, String> {
    publish::write(snapshot, destination, limits, fault)
}

#[doc(hidden)]
pub fn render_pdf_to<W: std::io::Write>(
    snapshot: &store::report::ReportSnapshot,
    limits: PdfReportLimits,
    target: &mut W,
) -> Result<u32, String> {
    let rendered = layout::layout(snapshot, limits)?;
    let pages = u32::try_from(rendered.pages.len())
        .map_err(|_| "Počet strán prekročil podporovaný rozsah.".to_string())?;
    pdf::write(rendered, target)?;
    Ok(pages)
}
