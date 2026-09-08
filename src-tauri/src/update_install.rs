//! Verified download and launch of a GitHub release installer (0.1.4,
//! Michal 2026-09-08: "viac ako odkaz, nikdy automaticky"). Two Tauri
//! commands: `download_update` fetches the latest release again, downloads
//! the installer asset and `SHA256SUMS.txt`, and refuses anything that does
//! not check out; `launch_update` re-verifies the file on disk and starts
//! it. Every request goes through `net::download_with_redirects`, so it
//! lands in `net_log` like every other request this app makes. Neither
//! command runs on its own; both are one deliberate click away, behind the
//! `Aktualizovať na <tag>` button in Nastavenia.
use crate::net;
use crate::state::AppState;
use crate::update::{self, Release};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use store::Store;
use tauri::State;

/// Shared with `net::download_with_redirects`'s own cap: the installer and
/// `SHA256SUMS.txt` are each capped at 64 MiB, so neither fetch can hold an
/// unbounded amount of memory even if a redirect points somewhere unexpected.
const MAX_DOWNLOAD_BYTES: usize = net::UPDATE_DOWNLOAD_MAX_BYTES;

/// What `download_update` hands back to Nastavenia: a path on disk and the
/// SHA-256 that was actually verified against `SHA256SUMS.txt`.
/// `launch_update` takes both back so it can re-verify before it runs
/// anything, rather than trusting that the file on disk is still the one
/// this process just checked.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DownloadedUpdate {
    pub path: String,
    pub sha256: String,
}

/// Every way `download_update` can refuse to hand back a verified installer.
/// Each variant's message is the exact Slovak line Nastavenia shows.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DownloadUpdateError {
    #[error("Kontrola aktualizácie zlyhala: {0}")]
    Check(String),
    #[error("Táto verzia nemá inštalátor na stiahnutie.")]
    NoInstaller,
    #[error("Najnovšie vydanie sa medzičasom zmenilo, obnovte Nastavenia a skúste to znova.")]
    TagMismatch,
    #[error("Stiahnutie zlyhalo: {0}")]
    Download(String),
    #[error("Súbor SHA256SUMS.txt neobsahuje riadok pre {0}.")]
    ChecksumLineMissing(String),
    #[error("Kontrolný súčet inštalátora sa nezhoduje, stiahnutý súbor bol odstránený.")]
    ChecksumMismatch,
    #[error("V priečinku už existuje iný súbor {0}. Zmažte ho a skúste to znova.")]
    ExistingFileMismatch(String),
    #[error("Zápis na disk zlyhal: {0}")]
    Io(String),
}

/// Every way `launch_update` can refuse to start the installer it was
/// pointed at.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LaunchUpdateError {
    #[error("Súbor sa nedá prečítať: {0}")]
    Io(String),
    #[error("Kontrolný súčet inštalátora sa nezhoduje, spustenie bolo zamietnuté.")]
    ChecksumMismatch(String),
    #[error("Inštalátor sa nepodarilo spustiť: {0}")]
    Spawn(String),
}

/// The two asset URLs plus the installer's real GitHub filename (the name
/// `SHA256SUMS.txt` lists it under, which is not necessarily the same as
/// the local filename `installer_dest_path` writes to).
struct InstallerAssets {
    installer_url: String,
    installer_name: String,
    checksums_url: String,
}

fn installer_assets(release: &Release) -> Option<InstallerAssets> {
    Some(InstallerAssets {
        installer_url: release.installer_url.clone()?,
        installer_name: release.installer_name.clone()?,
        checksums_url: release.checksums_url.clone()?,
    })
}

fn ensure_tag_matches(actual: &str, expected: &str) -> Result<(), DownloadUpdateError> {
    if actual == expected { Ok(()) } else { Err(DownloadUpdateError::TagMismatch) }
}

/// `%TEMP%\abakus-update\abakus-setup-<tag, without a leading v>.exe`
/// (spec 0.1.4). The tag drives the local filename so two different
/// releases downloaded in a row never collide; the GitHub asset's own name
/// (possibly different) is what `SHA256SUMS.txt` is keyed on instead, see
/// `InstallerAssets::installer_name`.
fn installer_dest_path(dest_dir: &Path, tag: &str) -> PathBuf {
    let normalized = tag.strip_prefix('v').or_else(|| tag.strip_prefix('V')).unwrap_or(tag);
    dest_dir.join(format!("abakus-setup-{normalized}.exe"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Standard `sha256sum`-style line: `<hex hash>  <filename>`, one or two
/// spaces, an optional leading `*` on the filename (binary mode). Returns
/// the lowercase hex hash for the first line naming `filename`.
fn find_checksum_line(text: &str, filename: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == filename).then(|| hash.to_lowercase())
    })
}

/// If `dest_path` already holds a file, it must already be exactly the
/// verified installer (idempotent re-click) or the download is refused
/// outright rather than silently overwritten (spec 0.1.4: "refuse to
/// overwrite an existing file with a different hash").
fn existing_download(dest_path: &Path, expected_hex: &str) -> Result<Option<DownloadedUpdate>, DownloadUpdateError> {
    if !dest_path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(dest_path).map_err(|e| DownloadUpdateError::Io(e.to_string()))?;
    let actual = sha256_hex(&bytes);
    if actual == expected_hex {
        return Ok(Some(DownloadedUpdate { path: dest_path.display().to_string(), sha256: actual }));
    }
    Err(DownloadUpdateError::ExistingFileMismatch(dest_path.display().to_string()))
}

/// Writes the freshly downloaded bytes, then verifies: a mismatch deletes
/// what was just written rather than leaving an unverified installer on
/// disk under a name the user might double-click.
fn save_verified(dest_path: &Path, bytes: &[u8], expected_hex: &str) -> Result<DownloadedUpdate, DownloadUpdateError> {
    std::fs::write(dest_path, bytes).map_err(|e| DownloadUpdateError::Io(e.to_string()))?;
    let actual = sha256_hex(bytes);
    if actual != expected_hex {
        let _ = std::fs::remove_file(dest_path);
        return Err(DownloadUpdateError::ChecksumMismatch);
    }
    Ok(DownloadedUpdate { path: dest_path.display().to_string(), sha256: actual })
}

async fn download_checksums<FDown, FutDown>(store: &Mutex<Store>, url: &str, send: &FDown) -> Result<String, DownloadUpdateError>
where
    FDown: Fn(String) -> FutDown,
    FutDown: std::future::Future<Output = Result<net::RawResponse, String>>,
{
    let bytes = net::download_with_redirects(store, url, MAX_DOWNLOAD_BYTES, send).await.map_err(|e| DownloadUpdateError::Download(e.to_string()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Re-fetches the release (instead of trusting whatever Nastavenia already
/// had in memory, so a click always verifies against GitHub's current
/// answer), confirms it is still the tag the click was for
/// (`ensure_tag_matches` catches the release changing underneath the click),
/// and returns its installer asset URLs or refuses.
async fn fetch_and_validate_release<FJson, FutJson>(store: &Mutex<Store>, tag: &str, send_json: FJson) -> Result<InstallerAssets, DownloadUpdateError>
where
    FJson: FnOnce(String) -> FutJson,
    FutJson: std::future::Future<Output = Result<(u16, Vec<u8>), String>>,
{
    let release = update::fetch_latest(store, send_json).await.map_err(|e| DownloadUpdateError::Check(e.to_string()))?;
    let release = release.ok_or(DownloadUpdateError::NoInstaller)?;
    ensure_tag_matches(&release.tag, tag)?;
    installer_assets(&release).ok_or(DownloadUpdateError::NoInstaller)
}

/// Downloads `SHA256SUMS.txt` and returns the hash it lists for the
/// installer's own name, or refuses when that line is missing.
async fn verified_expected_hash<FDown, FutDown>(store: &Mutex<Store>, assets: &InstallerAssets, send_download: &FDown) -> Result<String, DownloadUpdateError>
where
    FDown: Fn(String) -> FutDown,
    FutDown: std::future::Future<Output = Result<net::RawResponse, String>>,
{
    let checksums = download_checksums(store, &assets.checksums_url, send_download).await?;
    find_checksum_line(&checksums, &assets.installer_name).ok_or_else(|| DownloadUpdateError::ChecksumLineMissing(assets.installer_name.clone()))
}

/// The testable body of the `download_update` command. `send_json` fetches
/// the release JSON again (same shape as `update::check_with`'s seam);
/// `send_download` fetches the installer and checksums assets, following
/// redirects (same shape as `net::download_with_redirects`'s seam).
pub(crate) async fn download_update_with<FJson, FutJson, FDown, FutDown>(
    store: &Mutex<Store>,
    tag: &str,
    dest_dir: &Path,
    send_json: FJson,
    send_download: FDown,
) -> Result<DownloadedUpdate, DownloadUpdateError>
where
    FJson: FnOnce(String) -> FutJson,
    FutJson: std::future::Future<Output = Result<(u16, Vec<u8>), String>>,
    FDown: Fn(String) -> FutDown,
    FutDown: std::future::Future<Output = Result<net::RawResponse, String>>,
{
    let assets = fetch_and_validate_release(store, tag, send_json).await?;
    let expected_hex = verified_expected_hash(store, &assets, &send_download).await?;

    let dest_path = installer_dest_path(dest_dir, tag);
    if let Some(existing) = existing_download(&dest_path, &expected_hex)? {
        return Ok(existing);
    }

    std::fs::create_dir_all(dest_dir).map_err(|e| DownloadUpdateError::Io(e.to_string()))?;
    let bytes = net::download_with_redirects(store, &assets.installer_url, MAX_DOWNLOAD_BYTES, &send_download).await.map_err(|e| DownloadUpdateError::Download(e.to_string()))?;
    save_verified(&dest_path, &bytes, &expected_hex)
}

/// The testable body of the `launch_update` command: reads `path`, hashes
/// it again, and only calls `spawn` when that hash still matches
/// `expected_sha256`, the value `download_update` returned. `spawn` stands
/// in for actually starting the installer, so a test can prove the
/// checksum gate without ever launching a real process.
pub(crate) fn launch_update_with(path: &Path, expected_sha256: &str, spawn: &dyn Fn(&Path) -> std::io::Result<()>) -> Result<(), LaunchUpdateError> {
    let bytes = std::fs::read(path).map_err(|e| LaunchUpdateError::Io(e.to_string()))?;
    let actual = sha256_hex(&bytes);
    if actual != expected_sha256.to_lowercase() {
        return Err(LaunchUpdateError::ChecksumMismatch(actual));
    }
    spawn(path).map_err(|e| LaunchUpdateError::Spawn(e.to_string()))
}

/// Starts the installer with no silent flags: the Inno wizard runs and asks
/// the user, exactly as if they had double-clicked it themselves. Never
/// waited on; `launch_update` exits the app right after this returns.
fn spawn_installer(path: &Path) -> std::io::Result<()> {
    std::process::Command::new(path).spawn().map(|_child| ())
}

/// Only a link under this exact prefix is ever opened, so `open_release_page`
/// can never be pointed at an arbitrary local or remote command.
const ALLOWED_RELEASE_PREFIX: &str = "https://github.com/SouthCarpet/Abakus/";

fn is_allowed_release_url(url: &str) -> bool {
    url.starts_with(ALLOWED_RELEASE_PREFIX)
}

/// The testable body of `open_release_page`: `open` stands in for actually
/// asking Windows to open the URL, so a test can prove the prefix guard
/// without spawning a browser.
pub(crate) fn open_release_page_with(url: &str, open: &dyn Fn(&str) -> std::io::Result<()>) -> Result<(), String> {
    if !is_allowed_release_url(url) {
        return Err("Neplatná adresa vydania.".to_string());
    }
    open(url).map_err(|e| e.to_string())
}

/// `cmd /C start "" <url>` is the standard way to hand a URL to the user's
/// default browser on Windows without adding a Tauri shell/opener plugin
/// (`no_network.rs` bans both); the empty `""` is the window-title argument
/// `start` expects before the URL, otherwise a URL in quotes is misread as
/// the title itself.
fn open_in_default_browser(url: &str) -> std::io::Result<()> {
    std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn().map(|_child| ())
}

#[tauri::command]
pub async fn download_update(state: State<'_, AppState>, tag: String) -> Result<DownloadedUpdate, String> {
    let dest_dir = std::env::temp_dir().join("abakus-update");
    download_update_with(&state.store, &tag, &dest_dir, net::send_request, net::send_download_request).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn launch_update(app: tauri::AppHandle, path: String, sha256: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    launch_update_with(&p, &sha256, &spawn_installer).map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn open_release_page(url: String) -> Result<(), String> {
    open_release_page_with(&url, &open_in_default_browser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn dest_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("abakus-test-update-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn release_json(tag: &str, assets: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "tag_name": tag,
            "html_url": format!("https://github.com/SouthCarpet/Abakus/releases/tag/{tag}"),
            "body": "poznamky",
            "assets": assets,
        }))
        .unwrap()
    }

    fn full_assets() -> serde_json::Value {
        serde_json::json!([
            {"name": "abakus-setup-0.1.4.exe", "browser_download_url": "https://api.github.com/redirect/exe", "size": 9},
            {"name": "SHA256SUMS.txt", "browser_download_url": "https://api.github.com/redirect/sums", "size": 90},
        ])
    }

    const INSTALLER_BYTES: &[u8] = b"installer";

    fn checksums_text() -> String {
        format!("{}  abakus-setup-0.1.4.exe\n", sha256_hex(INSTALLER_BYTES))
    }

    #[tokio::test]
    async fn download_update_happy_path_writes_the_verified_installer_and_returns_its_hash() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let dir = dest_dir("happy");
        let send_json = |_: String| async { Ok((200, release_json("0.1.4", full_assets()))) };
        let send_download = |url: String| async move {
            match url.as_str() {
                "https://api.github.com/redirect/exe" => Ok(net::RawResponse { status: 302, location: Some("https://objects.githubusercontent.com/exe".into()), body: Vec::new() }),
                "https://objects.githubusercontent.com/exe" => Ok(net::RawResponse { status: 200, location: None, body: INSTALLER_BYTES.to_vec() }),
                "https://api.github.com/redirect/sums" => Ok(net::RawResponse { status: 302, location: Some("https://objects.githubusercontent.com/sums".into()), body: Vec::new() }),
                "https://objects.githubusercontent.com/sums" => Ok(net::RawResponse { status: 200, location: None, body: checksums_text().into_bytes() }),
                other => panic!("unexpected download url: {other}"),
            }
        };

        let result = download_update_with(&store, "0.1.4", &dir, send_json, send_download).await.unwrap();

        assert_eq!(result.sha256, sha256_hex(INSTALLER_BYTES));
        assert_eq!(std::fs::read(&result.path).unwrap(), INSTALLER_BYTES);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn download_update_refuses_a_checksum_mismatch_and_deletes_the_file() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let dir = dest_dir("mismatch");
        let send_json = |_: String| async { Ok((200, release_json("0.1.4", full_assets()))) };
        let send_download = |url: String| async move {
            match url.as_str() {
                "https://api.github.com/redirect/exe" => Ok(net::RawResponse { status: 200, location: None, body: b"not-the-real-installer".to_vec() }),
                "https://api.github.com/redirect/sums" => Ok(net::RawResponse { status: 200, location: None, body: checksums_text().into_bytes() }),
                other => panic!("unexpected download url: {other}"),
            }
        };

        let err = download_update_with(&store, "0.1.4", &dir, send_json, send_download).await.unwrap_err();

        assert_eq!(err, DownloadUpdateError::ChecksumMismatch);
        assert!(!installer_dest_path(&dir, "0.1.4").exists(), "a mismatched download must not be left on disk");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn download_update_refuses_a_release_with_no_installer_asset() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let dir = dest_dir("no-installer");
        let send_json = |_: String| async { Ok((200, release_json("0.1.4", serde_json::json!([])))) };
        let send_download = |_: String| async { unreachable!("must not be called: there is nothing to download") };

        let err = download_update_with(&store, "0.1.4", &dir, send_json, send_download).await.unwrap_err();

        assert_eq!(err, DownloadUpdateError::NoInstaller);
    }

    #[tokio::test]
    async fn download_update_refuses_when_the_latest_tag_changed_underneath_the_click() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let dir = dest_dir("tag-mismatch");
        let send_json = |_: String| async { Ok((200, release_json("0.1.5", full_assets()))) };
        let send_download = |_: String| async { unreachable!("must not be called: the tag mismatch is caught first") };

        let err = download_update_with(&store, "0.1.4", &dir, send_json, send_download).await.unwrap_err();

        assert_eq!(err, DownloadUpdateError::TagMismatch);
    }

    #[tokio::test]
    async fn download_update_reports_an_oversized_installer_body() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let dir = dest_dir("oversized");
        let send_json = |_: String| async { Ok((200, release_json("0.1.4", full_assets()))) };
        let oversized = vec![0u8; MAX_DOWNLOAD_BYTES + 1];
        let send_download = move |url: String| {
            let oversized = oversized.clone();
            async move {
                match url.as_str() {
                    "https://api.github.com/redirect/sums" => Ok(net::RawResponse { status: 200, location: None, body: checksums_text().into_bytes() }),
                    "https://api.github.com/redirect/exe" => Ok(net::RawResponse { status: 200, location: None, body: oversized }),
                    other => panic!("unexpected download url: {other}"),
                }
            }
        };

        let err = download_update_with(&store, "0.1.4", &dir, send_json, send_download).await.unwrap_err();

        assert!(matches!(err, DownloadUpdateError::Download(_)), "an oversized body must surface as a download failure: {err:?}");
    }

    #[tokio::test]
    async fn download_update_reports_a_redirect_limit_failure() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let dir = dest_dir("redirect-limit");
        let send_json = |_: String| async { Ok((200, release_json("0.1.4", full_assets()))) };
        let send_download = move |url: String| async move {
            match url.as_str() {
                "https://api.github.com/redirect/sums" => Ok(net::RawResponse { status: 200, location: None, body: checksums_text().into_bytes() }),
                _ => Ok(net::RawResponse { status: 302, location: Some(format!("{url}+")), body: Vec::new() }),
            }
        };

        let err = download_update_with(&store, "0.1.4", &dir, send_json, send_download).await.unwrap_err();

        assert!(matches!(err, DownloadUpdateError::Download(_)), "exceeding the redirect limit must surface as a download failure: {err:?}");
    }

    #[tokio::test]
    async fn download_update_short_circuits_when_the_file_already_matches() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let dir = dest_dir("already-there");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(installer_dest_path(&dir, "0.1.4"), INSTALLER_BYTES).unwrap();
        let send_json = |_: String| async { Ok((200, release_json("0.1.4", full_assets()))) };
        let send_download = move |url: String| async move {
            match url.as_str() {
                "https://api.github.com/redirect/sums" => Ok(net::RawResponse { status: 200, location: None, body: checksums_text().into_bytes() }),
                other => panic!("must not re-download the installer once the existing file already matches: {other}"),
            }
        };

        let result = download_update_with(&store, "0.1.4", &dir, send_json, send_download).await.unwrap();

        assert_eq!(result.sha256, sha256_hex(INSTALLER_BYTES));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn download_update_refuses_to_overwrite_an_existing_file_with_a_different_hash() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let dir = dest_dir("stale-file");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(installer_dest_path(&dir, "0.1.4"), b"some other file").unwrap();
        let send_json = |_: String| async { Ok((200, release_json("0.1.4", full_assets()))) };
        let send_download = move |url: String| async move {
            match url.as_str() {
                "https://api.github.com/redirect/sums" => Ok(net::RawResponse { status: 200, location: None, body: checksums_text().into_bytes() }),
                other => panic!("must not download when an unrelated file already occupies the destination: {other}"),
            }
        };

        let err = download_update_with(&store, "0.1.4", &dir, send_json, send_download).await.unwrap_err();

        assert!(matches!(err, DownloadUpdateError::ExistingFileMismatch(_)));
        assert_eq!(std::fs::read(installer_dest_path(&dir, "0.1.4")).unwrap(), b"some other file", "the unrelated existing file must be left untouched");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_checksum_line_matches_the_exact_filename_and_lowercases_the_hash() {
        let text = "DEADBEEF  abakus-setup-0.1.4.exe\ncafef00d abakus-setup-0.1.4.exe.sig\n";
        assert_eq!(find_checksum_line(text, "abakus-setup-0.1.4.exe").as_deref(), Some("deadbeef"));
        assert_eq!(find_checksum_line(text, "missing.exe"), None);
    }

    #[test]
    fn find_checksum_line_strips_the_binary_mode_asterisk() {
        let text = "abc123 *abakus-setup-0.1.4.exe\n";
        assert_eq!(find_checksum_line(text, "abakus-setup-0.1.4.exe").as_deref(), Some("abc123"));
    }

    #[test]
    fn launch_update_with_calls_spawn_only_when_the_hash_still_matches() {
        let path = std::env::temp_dir().join(format!("abakus-test-launch-ok-{}.exe", std::process::id()));
        std::fs::write(&path, INSTALLER_BYTES).unwrap();
        let calls: RefCell<Vec<PathBuf>> = RefCell::new(Vec::new());
        let spawn = |p: &Path| -> std::io::Result<()> { calls.borrow_mut().push(p.to_path_buf()); Ok(()) };

        launch_update_with(&path, &sha256_hex(INSTALLER_BYTES), &spawn).unwrap();

        assert_eq!(calls.borrow().as_slice(), std::slice::from_ref(&path));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn launch_update_with_refuses_and_never_spawns_when_the_file_changed_since_download() {
        let path = std::env::temp_dir().join(format!("abakus-test-launch-tampered-{}.exe", std::process::id()));
        std::fs::write(&path, b"tampered contents").unwrap();
        let never = |_: &Path| -> std::io::Result<()> { panic!("must not be called: the hash no longer matches") };

        let err = launch_update_with(&path, &sha256_hex(INSTALLER_BYTES), &never).unwrap_err();

        assert!(matches!(err, LaunchUpdateError::ChecksumMismatch(_)));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn open_release_page_with_allows_only_the_abakus_releases_prefix() {
        let calls: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let record = |url: &str| -> std::io::Result<()> { calls.borrow_mut().push(url.to_string()); Ok(()) };

        open_release_page_with("https://github.com/SouthCarpet/Abakus/releases/tag/0.1.4", &record).unwrap();

        assert_eq!(calls.borrow().as_slice(), ["https://github.com/SouthCarpet/Abakus/releases/tag/0.1.4".to_string()]);
    }

    #[test]
    fn open_release_page_with_refuses_a_url_outside_the_allowed_prefix() {
        let never = |_: &str| -> std::io::Result<()> { panic!("must not be called: the url is outside the allowed prefix") };

        let err = open_release_page_with("https://evil.example.com/steal", &never).unwrap_err();

        assert_eq!(err, "Neplatná adresa vydania.");
    }
}
