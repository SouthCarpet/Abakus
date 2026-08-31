use crate::Cents;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind { Personal, Business }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxKind { Card, CardForeign, Refund, Atm, TransferIn, TransferOut, StandingOrder, Other }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub posted_date: NaiveDate, pub tx_date: NaiveDate, pub kind: TxKind, pub amount_cents: Cents,
    pub orig_amount_cents: Option<Cents>, pub orig_currency: Option<String>, pub rate_micros: Option<i64>,
    pub merchant_raw: String, pub place: Option<String>, pub counterparty_name: Option<String>,
    pub counterparty_iban: Option<String>, pub reference: Option<String>, pub card_last4: Option<String>, pub raw_block: String,
}

impl Transaction {
    /// A blank record for `posted` with `amount`; parsers fill the rest.
    pub fn blank(posted: NaiveDate, amount: Cents, kind: TxKind, raw_block: String) -> Self {
        Self { posted_date: posted, tx_date: posted, kind, amount_cents: amount, orig_amount_cents: None, orig_currency: None, rate_micros: None,
            merchant_raw: String::new(), place: None, counterparty_name: None, counterparty_iban: None, reference: None, card_last4: None, raw_block }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header { pub iban: String, pub account_kind: AccountKind, pub number: u32, pub date: NaiveDate }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Statement {
    pub iban: String, pub account_kind: AccountKind, pub number: u32, pub period_start: NaiveDate, pub period_end: NaiveDate,
    /// None when the `Posledný výpis` line was not found; the checksum is then not verifiable (review blocker 2026-08-30: a defaulted 0 could sum to a false Ok).
    pub opening_cents: Option<Cents>, pub closing_cents: Option<Cents>, pub transactions: Vec<Transaction>, pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "off_by", rename_all = "snake_case")]
pub enum Checksum { Ok, OffBy(Cents), NotVerifiable }

impl Statement {
    pub fn checksum(&self) -> Checksum {
        let (Some(opening), Some(closing)) = (self.opening_cents, self.closing_cents) else { return Checksum::NotVerifiable };
        let sum: Cents = self.transactions.iter().map(|t| t.amount_cents).sum();
        let diff = opening + sum - closing;
        if diff == 0 { Checksum::Ok } else { Checksum::OffBy(diff) }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error("not a Tatra banka statement: {0}")] NotAStatement(String),
    #[error("the PDF is encrypted")] Encrypted,
    #[error("pdf error: {0}")] Pdf(String),
    #[error("io error: {0}")] Io(String),
}
