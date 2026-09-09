//! Bank statement PDF parsing.

pub mod money;
pub mod iban;
pub mod fold;
pub mod lines;
pub mod blocks;
pub mod model;
pub mod header;
pub mod fields;
pub mod card;
pub mod transfer;
pub mod statement;
pub mod pdfium;

pub use model::*;
pub use statement::{parse_pages, parse_text};
pub use pdfium::{char_geometry, extract_pages, parse_pdf};

pub type Cents = i64;
