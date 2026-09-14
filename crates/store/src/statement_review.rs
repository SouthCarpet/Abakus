//! Stored import evidence and live retained-row checks for one statement.
use crate::{Result, Store, StoreError};
use parser::Checksum;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementReviewStatus { NeedsAttention, EvidenceIncomplete, NoOpenChecks }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatementReview {
    pub statement_id: i64,
    pub checksum: Checksum,
    pub parser_warnings: Option<Vec<String>>,
    pub unassigned_count: i64,
    pub suggested_count: i64,
    pub status: StatementReviewStatus,
}

impl Store {
    /// One SELECT gives all evidence the same SQLite read snapshot. Unknown
    /// legacy warnings stay distinct from a known empty parser result.
    pub fn statement_review(&self, statement_id: i64) -> Result<StatementReview> {
        let (checksum_status, delta, warnings, unassigned_count, suggested_count) = self.conn.query_row(
            "SELECT checksum_status, checksum_off_by, parser_warnings_json,
                (SELECT COUNT(*) FROM transactions t WHERE t.statement_id=s.id AND t.status='unassigned'),
                (SELECT COUNT(*) FROM transactions t WHERE t.statement_id=s.id AND t.status='suggested')
             FROM statements s WHERE s.id=?1",
            [statement_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<String>>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?)),
        ).optional()?.ok_or(StoreError::UnknownStatement { id: statement_id })?;
        let checksum = strict_checksum(&checksum_status, delta)?;
        let parser_warnings = warnings.map(|json| serde_json::from_str::<Vec<String>>(&json)
            .map_err(|error| StoreError::Parse(format!("Neplatné upozornenia výpisu: {error}"))))
            .transpose()?;
        let status = review_status(checksum, parser_warnings.as_deref(), unassigned_count, suggested_count);
        Ok(StatementReview { statement_id, checksum, parser_warnings, unassigned_count, suggested_count, status })
    }
}

fn review_status(checksum: Checksum, warnings: Option<&[String]>, unassigned: i64, suggested: i64) -> StatementReviewStatus {
    if matches!(checksum, Checksum::OffBy(_))
        || warnings.is_some_and(|warnings| !warnings.is_empty())
        || unassigned > 0 || suggested > 0 {
        return StatementReviewStatus::NeedsAttention;
    }
    if checksum == Checksum::NotVerifiable || warnings.is_none() {
        return StatementReviewStatus::EvidenceIncomplete;
    }
    StatementReviewStatus::NoOpenChecks
}

fn strict_checksum(status: &str, delta: Option<i64>) -> Result<Checksum> {
    match (status, delta) {
        ("ok", None) => Ok(Checksum::Ok),
        ("not_verifiable", None) => Ok(Checksum::NotVerifiable),
        ("off", Some(delta)) => Ok(Checksum::OffBy(delta)),
        _ => Err(StoreError::Parse("Neplatná uložená kontrola zostatku výpisu.".into())),
    }
}
