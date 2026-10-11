//! Bounded vector PDF rendering and no-clobber publication.
mod font;
mod layout;
mod pdf;
mod publish;

use serde::{Deserialize, Serialize};
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
    write_pdf_report_with_fault(snapshot, destination, limits, PdfIoFault::None)
}

#[doc(hidden)]
pub fn write_pdf_report_with_fault(
    snapshot: &store::report::ReportSnapshot,
    destination: &Path,
    limits: PdfReportLimits,
    fault: PdfIoFault,
) -> Result<PdfReportOutcome, String> {
    publish::write(&snapshot.preview, destination, limits, fault, |target| {
        render_pdf_to(snapshot, limits, target)
    })
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

/// Which document an export produces. `Full` is the transaction listing that
/// exists since 0.1.2. The summaries have a fixed number of pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportDocumentScope {
    #[default]
    Full,
    SummaryOnePage,
    SummaryThreePages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryLength {
    OnePage,
    ThreePages,
}

impl ReportDocumentScope {
    pub fn summary_length(self) -> Option<SummaryLength> {
        match self {
            Self::Full => None,
            Self::SummaryOnePage => Some(SummaryLength::OnePage),
            Self::SummaryThreePages => Some(SummaryLength::ThreePages),
        }
    }
}

pub fn write_summary_pdf_report(
    summary: &store::report::SummarySnapshot,
    length: SummaryLength,
    destination: &Path,
) -> Result<PdfReportOutcome, String> {
    let limits = PdfReportLimits::default();
    publish::write(
        &summary.snapshot.preview,
        destination,
        limits,
        PdfIoFault::None,
        |target| render_summary_pdf_to(summary, length, limits, target),
    )
}

#[doc(hidden)]
pub fn render_summary_pdf_to<W: std::io::Write>(
    summary: &store::report::SummarySnapshot,
    length: SummaryLength,
    limits: PdfReportLimits,
    target: &mut W,
) -> Result<u32, String> {
    let rendered = layout::layout_summary(summary, length, limits)?;
    let pages = u32::try_from(rendered.pages.len())
        .map_err(|_| "Počet strán prekročil podporovaný rozsah.".to_string())?;
    pdf::write(rendered, target)?;
    Ok(pages)
}
