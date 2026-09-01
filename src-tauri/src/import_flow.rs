//! Turns a parsed (or failed) statement into a UI-facing `ImportReport`, and
//! drives the retry-with-stored-password loop. Pure and unit tested; file and
//! pdfium I/O live in `import_path` only.
use chrono::NaiveDate;
use parser::{AccountKind, Checksum, ParseError, Statement};
use serde::{Deserialize, Serialize};
use std::path::Path;
use store::{Store, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus { Imported, AlreadyImported, Locked, UnknownAccount, Error }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub path: String,
    pub status: ImportStatus,
    pub account_label: Option<String>,
    pub account_kind: Option<AccountKind>,
    pub iban_masked: Option<String>,
    pub iban: Option<String>,
    pub statement_number: Option<u32>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub inserted: usize,
    pub duplicates: usize,
    pub checksum: Option<Checksum>,
    pub warnings: Vec<String>,
    pub message: Option<String>,
    /// A17/F5: the id a successful import wrote to, so the UI can navigate
    /// straight to the rows that were just imported. `None` for every status
    /// that never reaches a stored statement (locked, unknown_account, error).
    pub statement_id: Option<i64>,
}

impl ImportReport {
    fn bare(path: &str, status: ImportStatus) -> Self {
        Self { path: path.into(), status, account_label: None, account_kind: None, iban_masked: None, iban: None, statement_number: None, period_start: None, period_end: None, inserted: 0, duplicates: 0, checksum: None, warnings: Vec::new(), message: None, statement_id: None }
    }
}

pub fn mask(iban: &str) -> String {
    if iban.len() < 8 { return iban.into() }
    format!("{}...{}", &iban[..4], &iban[iban.len() - 4..])
}

pub fn import_parsed(store: &mut Store, path: &str, parsed: Result<Statement, ParseError>, file_hash: &str) -> ImportReport {
    let st = match parsed {
        Ok(st) => st,
        Err(ParseError::Encrypted) => return ImportReport::bare(path, ImportStatus::Locked),
        Err(e) => return ImportReport { message: Some(e.to_string()), ..ImportReport::bare(path, ImportStatus::Error) },
    };
    let mut r = ImportReport { account_kind: Some(st.account_kind), iban_masked: Some(mask(&st.iban)), iban: Some(st.iban.clone()), statement_number: Some(st.number), period_start: Some(st.period_start), period_end: Some(st.period_end), warnings: st.warnings.clone(), checksum: Some(st.checksum()), ..ImportReport::bare(path, ImportStatus::Imported) };
    match store.import_statement(&st, file_hash) {
        Ok(o) => { r.status = if o.already_imported { ImportStatus::AlreadyImported } else { ImportStatus::Imported }; r.inserted = o.inserted; r.duplicates = o.duplicates; r.statement_id = Some(o.statement_id); r.account_label = store.account_by_iban(&st.iban).ok().flatten().map(|a| a.label); }
        Err(StoreError::UnknownAccount { .. }) => r.status = ImportStatus::UnknownAccount,
        Err(e) => { r.status = ImportStatus::Error; r.message = Some(e.to_string()); }
    }
    r
}

pub fn file_hash(path: &Path) -> Result<String, ParseError> {
    use sha2::Digest;
    let bytes = std::fs::read(path).map_err(|e| ParseError::Io(e.to_string()))?;
    Ok(hex::encode(sha2::Sha256::digest(bytes)))
}

/// `stored` returns the saved password for an IBAN (keyring in the app, a closure in tests).
///
/// Note: `parser::parse_pdf` serializes every call through an internal lock
/// (pdfium's C++ engine is not reentrant), so concurrent imports from Tauri
/// commands never run their pdfium calls in parallel; they just queue.
pub fn import_path(store: &mut Store, path: &Path, password: Option<&str>, stored: &dyn Fn(&str) -> Option<String>) -> ImportReport {
    let p = path.to_string_lossy().to_string();
    let hash = match file_hash(path) { Ok(h) => h, Err(e) => return ImportReport { message: Some(e.to_string()), ..ImportReport::bare(&p, ImportStatus::Error) } };
    let mut attempt = parser::parse_pdf(path, password);
    if matches!(attempt, Err(ParseError::Encrypted)) && password.is_none() {
        let candidates: Vec<String> = store.list_accounts().unwrap_or_default().into_iter().filter(|a| a.has_password).filter_map(|a| stored(&a.iban)).collect();
        for pw in candidates { attempt = parser::parse_pdf(path, Some(&pw)); if !matches!(attempt, Err(ParseError::Encrypted)) { break; } }
    }
    import_parsed(store, &p, attempt, &hash)
}

/// Amendment A5: remembering a password happens only after a successful
/// import (`imported`/`already_imported`), never for `unknown_account` or any
/// other status, so a wrong-but-parseable password is never saved against the
/// wrong account. `set_secret` is injected so a test can pass a recording
/// fake instead of touching the real keyring.
pub fn remember_password(store: &mut Store, report: &ImportReport, password: &str, set_secret: &dyn Fn(&str, &str) -> Result<(), String>) -> Result<(), String> {
    if !matches!(report.status, ImportStatus::Imported | ImportStatus::AlreadyImported) { return Ok(()); }
    let Some(iban) = &report.iban else { return Ok(()) };
    set_secret(iban, password)?;
    if let Some(a) = store.account_by_iban(iban).map_err(|e| e.to_string())? {
        store.set_has_password(a.id, true).map_err(|e| e.to_string())?;
    }
    Ok(())
}
