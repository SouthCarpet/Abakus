//! One consistent, bounded database snapshot for a PDF transaction report.
mod capture;
mod coverage;
mod dates;
mod totals;
mod types;

pub use capture::ReportCaptureLimits;
pub use types::*;
