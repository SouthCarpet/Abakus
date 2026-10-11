//! One consistent, bounded database snapshot for a PDF transaction report.
mod capture;
mod coverage;
mod dates;
mod digest;
mod previous;
mod summary;
mod totals;
mod types;

pub use capture::ReportCaptureLimits;
pub use digest::{previous_range, ratio_tenths, LARGEST_TRANSACTION_COUNT};
pub use types::*;
