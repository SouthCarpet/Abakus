//! Opt-in update check against GitHub Releases (spec D14/A14).
//!
//! One unauthenticated GET, run only when the user has turned it on in
//! Nastavenia. No token, no credentials, no other request. Every failure
//! (offline, DNS failure, rate limiting, a body that is not the JSON we
//! expect) comes back as `Ok(None)` or a typed `UpdateError`; nothing here
//! panics, blocks, or opens a dialog on its own. The caller decides what,
//! if anything, to show.
//!
//! 0.1.4 (Michal, 2026-09-08: "viac ako odkaz, nikdy automaticky"): a
//! `Release` also carries the installer asset's URL, name and size, plus the
//! `SHA256SUMS.txt` asset's URL, when the GitHub release has both. Nothing
//! here downloads them; that is `update_install::download_update`'s job, one
//! deliberate click away.
use crate::net;
use std::sync::Mutex;
use store::Store;

/// The one endpoint this module ever calls in a release build. Kept as a
/// named constant, not folded into a format string, so the owner/repo it
/// points at stays visible and easy to change.
const RELEASES_URL: &str = "https://api.github.com/repos/SouthCarpet/Abakus/releases/latest";

/// Debug-build-only test seam (Michal, 2026-09-08): a verifier proving the
/// update button end to end before v0.1.4 exists on GitHub needs to point
/// the check at a local static server instead of the real GitHub API. This
/// entire mechanism, including reading the environment variable, is
/// compiled out of a release build, so `RELEASES_URL` is the only URL this
/// module can ever reach once shipped; the override is not a hidden
/// production knob.
#[cfg(debug_assertions)]
fn effective_releases_url(override_value: Option<&str>) -> &str {
    override_value.unwrap_or(RELEASES_URL)
}

#[cfg(debug_assertions)]
fn releases_url() -> &'static str {
    effective_releases_url(option_env!("ABAKUS_RELEASES_URL_OVERRIDE"))
}

#[cfg(not(debug_assertions))]
fn releases_url() -> &'static str {
    RELEASES_URL
}

/// A release the user can choose to act on. `check` never downloads or
/// installs anything by itself. `url` is shown in Nastavenia as a link the
/// user opens deliberately (`open_release_page`); the installer fields stay
/// `None` unless the release has both an installer asset and a
/// `SHA256SUMS.txt` asset, so a release published without them still shows
/// the quiet informational line and never a broken button.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Release {
    pub tag: String,
    pub url: String,
    pub notes: String,
    pub installer_url: Option<String>,
    pub installer_name: Option<String>,
    pub installer_size: Option<u64>,
    pub checksums_url: Option<String>,
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

/// One release asset (spec 0.1.4): the installer executable or
/// `SHA256SUMS.txt`. Every field is optional and tolerant for the same
/// reason `GithubRelease`'s own fields are: an asset entry missing a name or
/// URL is skipped, never a hard `MalformedResponse` error.
#[derive(serde::Deserialize)]
struct GithubAsset {
    name: Option<String>,
    browser_download_url: Option<String>,
    size: Option<u64>,
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
    #[serde(default)]
    assets: Option<Vec<GithubAsset>>,
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
    let (status, body) = net::audited_get(store, releases_url()).await.map_err(UpdateError::Request)?;
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
    let (status, body) = net::fetch_and_log(store, releases_url(), send).await.map_err(UpdateError::Request)?;
    parse_release(current, status, &body)
}

/// `update_install::download_update` needs the exact asset URLs and name for
/// whatever GitHub currently calls "latest", not a currency decision against
/// some installed version: it re-fetches and re-parses the same endpoint,
/// through the same audit log, but skips the `is_newer` filter `check`/
/// `check_with` apply for their informational Nastavenia line.
pub(crate) async fn fetch_latest<F, Fut>(store: &Mutex<Store>, send: F) -> Result<Option<Release>, UpdateError>
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Result<(u16, Vec<u8>), String>>,
{
    let (status, body) = net::fetch_and_log(store, releases_url(), send).await.map_err(UpdateError::Request)?;
    parse_release_body(status, &body)
}

/// Shared by `check`/`check_with` (via `parse_release`) and `fetch_latest`:
/// turns a raw response into a `Release` regardless of whether it is newer
/// than anything, or `Ok(None)` when there is nothing usable to report.
fn parse_release_body(status: u16, body: &[u8]) -> Result<Option<Release>, UpdateError> {
    if !(200..300).contains(&status) {
        return Err(UpdateError::Status(status));
    }

    let release: GithubRelease = serde_json::from_slice(body).map_err(|_| UpdateError::MalformedResponse)?;

    let Some(tag) = release.tag_name.filter(|tag| !tag.trim().is_empty()) else {
        // A release with no usable tag can't be compared or linked to.
        return Ok(None);
    };

    let (installer_url, installer_name, installer_size, checksums_url) = installer_assets(release.assets);

    Ok(Some(Release {
        url: release.html_url.unwrap_or_default(),
        notes: release.body.unwrap_or_default(),
        tag,
        installer_url,
        installer_name,
        installer_size,
        checksums_url,
    }))
}

/// The installer asset (any name ending `.exe`) and the `SHA256SUMS.txt`
/// asset, if the release has both. Deliverable 1: "missing assets means the
/// update button is not offered ... never an error", so anything short of
/// both present returns four `None`s rather than a partial, unverifiable set.
fn installer_assets(assets: Option<Vec<GithubAsset>>) -> (Option<String>, Option<String>, Option<u64>, Option<String>) {
    let assets = assets.unwrap_or_default();
    let installer = assets.iter().find(|a| a.name.as_deref().is_some_and(|n| n.to_ascii_lowercase().ends_with(".exe")));
    let checksums = assets.iter().find(|a| a.name.as_deref() == Some("SHA256SUMS.txt"));
    match (installer, checksums) {
        (Some(installer), Some(checksums)) => match (&installer.browser_download_url, &installer.name, &checksums.browser_download_url) {
            (Some(installer_url), Some(installer_name), Some(checksums_url)) => {
                (Some(installer_url.clone()), Some(installer_name.clone()), installer.size, Some(checksums_url.clone()))
            }
            _ => (None, None, None, None),
        },
        _ => (None, None, None, None),
    }
}

/// Shared by `check` and `check_with`: turns a raw response into the same
/// `Ok(None)` / `Ok(Some(Release))` / `Err(UpdateError)` decision either way,
/// filtered through `is_newer` so only an actual update reaches the caller.
fn parse_release(current: &str, status: u16, body: &[u8]) -> Result<Option<Release>, UpdateError> {
    let Some(release) = parse_release_body(status, body)? else { return Ok(None) };
    if !is_newer(current, &release.tag) {
        return Ok(None);
    }
    Ok(Some(release))
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

    fn release_body(assets: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "tag_name": "0.1.4",
            "html_url": "https://github.com/SouthCarpet/Abakus/releases/tag/0.1.4",
            "body": "poznamky",
            "assets": assets,
        }))
        .unwrap()
    }

    #[test]
    fn parse_release_body_fills_installer_fields_when_both_assets_are_present() {
        let body = release_body(serde_json::json!([
            {"name": "abakus-setup-0.1.4.exe", "browser_download_url": "https://objects.githubusercontent.com/exe", "size": 12345},
            {"name": "SHA256SUMS.txt", "browser_download_url": "https://objects.githubusercontent.com/sums", "size": 90},
        ]));

        let release = parse_release_body(200, &body).unwrap().unwrap();

        assert_eq!(release.installer_url.as_deref(), Some("https://objects.githubusercontent.com/exe"));
        assert_eq!(release.installer_name.as_deref(), Some("abakus-setup-0.1.4.exe"));
        assert_eq!(release.installer_size, Some(12345));
        assert_eq!(release.checksums_url.as_deref(), Some("https://objects.githubusercontent.com/sums"));
    }

    #[test]
    fn parse_release_body_leaves_installer_fields_none_when_the_checksums_asset_is_missing() {
        let body = release_body(serde_json::json!([
            {"name": "abakus-setup-0.1.4.exe", "browser_download_url": "https://objects.githubusercontent.com/exe", "size": 12345},
        ]));

        let release = parse_release_body(200, &body).unwrap().unwrap();

        assert_eq!(release.installer_url, None);
        assert_eq!(release.installer_name, None);
        assert_eq!(release.installer_size, None);
        assert_eq!(release.checksums_url, None);
    }

    #[test]
    fn parse_release_body_leaves_installer_fields_none_when_there_are_no_assets_at_all() {
        let body = serde_json::to_vec(&serde_json::json!({"tag_name": "0.1.4", "html_url": "u", "body": "b"})).unwrap();

        let release = parse_release_body(200, &body).unwrap().unwrap();

        assert_eq!(release.installer_url, None);
        assert_eq!(release.checksums_url, None);
    }

    #[cfg(debug_assertions)]
    mod debug_override {
        use super::*;

        #[test]
        fn effective_releases_url_uses_the_override_when_set() {
            assert_eq!(effective_releases_url(Some("http://127.0.0.1:9999/releases/latest")), "http://127.0.0.1:9999/releases/latest");
        }

        #[test]
        fn effective_releases_url_falls_back_to_the_constant_when_unset() {
            assert_eq!(effective_releases_url(None), RELEASES_URL);
        }

        /// `ABAKUS_RELEASES_URL_OVERRIDE` is unset for this normal test run,
        /// so this exercises the real wiring (`option_env!` ->
        /// `effective_releases_url` -> `releases_url`), not just the pure
        /// function above, and shows it still lands on the constant.
        #[test]
        fn releases_url_reads_the_constant_when_no_override_was_compiled_in() {
            assert_eq!(releases_url(), RELEASES_URL);
        }
    }
}
