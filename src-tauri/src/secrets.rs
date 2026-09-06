//! Statement passwords in the Windows Credential Manager, via `keyring` (v1
//! API: `Entry::new`/`get_password`/`set_password`/`delete_credential`). The
//! `windows-native` credential store ships as part of the crate's default
//! `v1` feature on Windows targets (verified against the crate source: keyring
//! 4.x dropped the standalone `windows-native` cargo feature that existed only
//! in the yanked `4.0.0-alpha.1`), so no extra feature flag is needed.
const SERVICE: &str = "abakus";

pub fn get(iban: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, iban).ok()?.get_password().ok()
}

pub fn set(iban: &str, pw: &str) -> Result<(), String> {
    keyring::Entry::new(SERVICE, iban).and_then(|e| e.set_password(pw)).map_err(|e| e.to_string())
}

/// A missing credential is not a failure: the caller wants it gone, and it
/// already is. Anything else (locked vault, backend error) must reach the
/// caller as `Err`, not be swallowed here: a credential-cleanup failure must
/// never look like a completed cleanup (078 audit).
pub fn clear(iban: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, iban).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
