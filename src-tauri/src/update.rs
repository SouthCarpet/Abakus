//! Opt-in update check against GitHub Releases (spec D14/A14).
//!
//! One unauthenticated GET, run only when the user has turned it on in
//! Nastavenia. No token, no credentials, no other request. Every failure
//! (offline, DNS failure, rate limiting, a body that is not the JSON we
//! expect) comes back as `Ok(None)` or a typed `UpdateError`; nothing here
//! panics, blocks, or opens a dialog on its own. The caller decides what,
//! if anything, to show.
use crate::net;
use std::sync::Mutex;
use store::Store;

/// The one endpoint this module ever calls. Kept as a named constant, not
/// folded into a format string, so the owner/repo it points at stays
/// visible and easy to change.
const RELEASES_URL: &str = "https://api.github.com/repos/SouthCarpet/Abakus/releases/latest";

/// A release the user can choose to download by hand. `check` never
/// downloads or installs it. `url` is shown in Nastavenia as plain text for
/// the user to copy; nothing in this app opens it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Release {
    pub tag: String,
    pub url: String,
    pub notes: String,
}

/// Every way the check can fail to produce a `Release`. Each variant is
/// meant to be shown, if at all, as a single quiet line in Nastavenia:
/// never as a blocking dialog.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// The request itself did not complete: offline, DNS failure, or it ran
    /// past `net::REQUEST_TIMEOUT`.
    #[error("update check request failed: {0}")]
    Request(String),
    /// GitHub answered but not with 200. This is how rate limiting (403,
    /// with the `x-ratelimit-*` headers) surfaces, and also how a redirect
    /// response comes back now that the audited HTTP client never follows one.
    #[error("update check got HTTP {0}")]
    Status(u16),
    /// The 200 body was not valid JSON.
    #[error("update check could not read GitHub's response")]
    MalformedResponse,
}

/// The subset of GitHub's release JSON this module reads. Every field is
/// optional on purpose: a response that parses as JSON but is missing or
/// nulls out a field (for example a release published with no tag) is a
/// normal, quiet "nothing to report", not a `MalformedResponse` error. Only
/// a body that fails to parse as JSON at all reaches that error.
#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: Option<String>,
    html_url: Option<String>,
    #[serde(default)]
    body: Option<String>,
}

/// Whether `App::setup` should schedule the update check at all, given only
/// the persisted opt-in flag. Kept separate from the async plumbing so the
/// on/off decision is unit-testable without a network call (Alidade 064 R8).
pub fn wants_check(check_updates_setting: bool) -> bool {
    check_updates_setting
}

/// Asks GitHub for the latest published release and, if it is newer than
/// `current`, returns it. Goes through `net::audited_get`, the app's only
/// production HTTP callsite, so this request lands in `net_log` like every other.
///
/// Takes `&Mutex<Store>` rather than `&mut Store` so the store is never
/// locked across the request's own `.await` (see `net::audited_get`).
pub async fn check(current: &str, store: &Mutex<Store>) -> Result<Option<Release>, UpdateError> {
    let (status, body) = net::audited_get(store, RELEASES_URL).await.map_err(UpdateError::Request)?;
    parse_release(current, status, &body)
}

/// The testable half of `check`: `send` stands in for the real network
/// transport (mirrors `net::fetch_and_log`'s own test seam), so a test can
/// inject a recording fake and assert it is never called when the caller
/// has already decided not to check.
pub(crate) async fn check_with<F, Fut>(current: &str, store: &Mutex<Store>, send: F) -> Result<Option<Release>, UpdateError>
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Result<(u16, Vec<u8>), String>>,
{
    let (status, body) = net::fetch_and_log(store, RELEASES_URL, send).await.map_err(UpdateError::Request)?;
    parse_release(current, status, &body)
}

/// Shared by `check` and `check_with`: turns a raw response into the same
/// `Ok(None)` / `Ok(Some(Release))` / `Err(UpdateError)` decision either way.
fn parse_release(current: &str, status: u16, body: &[u8]) -> Result<Option<Release>, UpdateError> {
    if !(200..300).contains(&status) {
        return Err(UpdateError::Status(status));
    }

    let release: GithubRelease = serde_json::from_slice(body).map_err(|_| UpdateError::MalformedResponse)?;

    let Some(tag) = release.tag_name.filter(|tag| !tag.trim().is_empty()) else {
        // A release with no usable tag can't be compared or linked to.
        return Ok(None);
    };

    if !is_newer(current, &tag) {
        return Ok(None);
    }

    Ok(Some(Release {
        url: release.html_url.unwrap_or_default(),
        notes: release.body.unwrap_or_default(),
        tag,
    }))
}

/// `(major, minor, patch)`.
type Core = (u64, u64, u64);

/// Parses a version string into its numeric core plus whether it carries a
/// pre-release suffix (`-` and anything after it, e.g. `-rc1`). Tolerant of
/// a leading `v`/`V`. Returns `None` for anything that isn't exactly three
/// dot-separated non-negative integers: that is the one "malformed" signal
/// both callers below rely on.
fn parse_version(input: &str) -> Option<(Core, bool)> {
    let input = input.strip_prefix('v').or_else(|| input.strip_prefix('V')).unwrap_or(input);
    let core_str = match input.split_once('-') {
        Some((core, _prerelease)) => core,
        None => input,
    };
    let mut parts = core_str.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let is_prerelease = core_str.len() < input.len();
    Some(((major, minor, patch), is_prerelease))
}

/// Is `tag` a release the user should be told about, given they are running
/// `current`? Tolerant of a leading `v`/`V` on either side (GitHub tag
/// convention). A malformed tag (fails to parse into exactly three
/// non-negative integers, for example `"latest"`, `""`, garbage) is never
/// newer: this backs a purely informational, opt-in prompt, so "no update"
/// is always the safe answer when the input can't be understood. The same
/// applies if `current` itself fails to parse.
///
/// Pre-release decision: a `tag` carrying a pre-release suffix (`0.2.0-rc1`,
/// `v0.2.0-beta.1`, and so on) is never reported as newer, even when its numeric
/// core is greater than `current`. This drives a single quiet "update
/// available" line aimed at ordinary users, not testers.
pub fn is_newer(current: &str, tag: &str) -> bool {
    let Some((current_core, _)) = parse_version(current) else { return false };
    let Some((tag_core, tag_is_prerelease)) = parse_version(tag) else { return false };
    if tag_is_prerelease {
        return false;
    }
    tag_core > current_core
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_tag_is_never_newer() {
        assert!(!is_newer("1.0.0", ""));
    }

    #[test]
    fn equal_versions_are_not_newer() {
        assert!(!is_newer("1.2.3", "1.2.3"));
        assert!(!is_newer("1.2.3", "v1.2.3"));
    }

    #[test]
    fn older_tag_is_not_newer() {
        assert!(!is_newer("1.2.3", "1.2.2"));
        assert!(!is_newer("1.2.3", "1.1.9"));
    }

    #[test]
    fn newer_tag_is_newer() {
        assert!(is_newer("1.2.3", "1.2.4"));
        assert!(is_newer("1.2.3", "1.3.0"));
        assert!(is_newer("0.1.0", "v0.2.0"));
    }

    #[test]
    fn malformed_tag_is_never_newer() {
        assert!(!is_newer("1.0.0", "latest"));
        assert!(!is_newer("1.0.0", "1.0"));
        assert!(!is_newer("1.0.0", "1.0.0.1"));
        assert!(!is_newer("garbage", "1.0.0"));
    }

    #[test]
    fn prerelease_tag_is_never_reported_as_newer() {
        assert!(!is_newer("0.1.0", "0.2.0-rc1"));
        assert!(!is_newer("0.1.0", "v0.2.0-beta.1"));
    }

    #[test]
    fn wants_check_follows_only_the_persisted_flag() {
        assert!(!wants_check(false), "the default settings must be opted out");
        assert!(wants_check(true), "turning the setting on must schedule the check");
    }
}
