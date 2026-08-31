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

pub use model::*;
pub use statement::{parse_pages, parse_text};

pub type Cents = i64;
