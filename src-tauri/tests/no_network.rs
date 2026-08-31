//! D9/A14: no workspace crate other than the app itself declares a network
//! dependency, and no Tauri network plugin enters the dependency tree.
//! Tauri's own transitive reqwest/hyper internals are outside the app's
//! reach and not banned. Spec D14 carves out one exception: `reqwest` is
//! allowed as a *direct* dependency of `src-tauri/Cargo.toml` only, for the
//! opt-in update check (`update::check`, via `net::audited_get`).
use std::fs;

const BANNED_EVERYWHERE: &[&str] = &[
    "hyper", "ureq", "curl", "isahc", "attohttpc", "tokio-tungstenite",
    "tauri-plugin-http", "tauri-plugin-updater", "tauri-plugin-shell", "tauri-plugin-opener",
];
const BANNED_PLUGINS: &[&str] = &[
    "tauri-plugin-http", "tauri-plugin-updater", "tauri-plugin-shell", "tauri-plugin-opener",
];
const MANIFESTS: &[&str] = &[
    "/../Cargo.toml", "/../crates/parser/Cargo.toml", "/../crates/rules/Cargo.toml",
    "/../crates/store/Cargo.toml", "/Cargo.toml",
];
/// `src-tauri/Cargo.toml` (this crate's own manifest) is the one exception
/// where `reqwest` is allowed (spec D14).
const APP_MANIFEST: &str = "/Cargo.toml";

fn read_manifest(m: &str) -> String {
    fs::read_to_string(format!("{}{m}", env!("CARGO_MANIFEST_DIR"))).unwrap_or_else(|e| panic!("{m}: {e}"))
}

fn declares(text: &str, name: &str) -> bool {
    let re = regex::Regex::new(&format!(r#"(?m)^\s*"?{}"?\s*="#, regex::escape(name))).unwrap();
    re.is_match(text)
}

#[test]
fn no_manifest_declares_a_banned_network_crate() {
    for m in MANIFESTS {
        let text = read_manifest(m);
        let hits: Vec<&&str> = BANNED_EVERYWHERE.iter().filter(|n| declares(&text, n)).collect();
        assert!(hits.is_empty(), "network dependency declared in {m}: {hits:?}");
    }
}

#[test]
fn only_the_app_crate_declares_reqwest() {
    for m in MANIFESTS.iter().filter(|m| **m != APP_MANIFEST) {
        let text = read_manifest(m);
        assert!(!declares(&text, "reqwest"), "reqwest declared outside src-tauri in {m}");
    }
    let app_text = read_manifest(APP_MANIFEST);
    assert!(declares(&app_text, "reqwest"), "spec D14 expects src-tauri to declare reqwest directly");
}

/// A14b: `net::audited_get` must be the app's only reqwest callsite. Scans
/// every `.rs` file under `src/` except `net.rs` itself for any mention of
/// `reqwest`, not only `reqwest::`, so an alias (`use reqwest as r;`) or a
/// bare re-export can't slip the scan the narrower pattern missed. `net.rs`
/// is the one allowed callsite; this test file names itself too, in case
/// the scan is ever pointed at a wider tree that includes it.
const ALLOWED_REQWEST_FILES: &[&str] = &["net.rs", "no_network.rs"];

#[test]
fn reqwest_is_used_only_inside_net_rs() {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    collect_reqwest_offenders(&src_dir, &mut offenders);
    assert!(offenders.is_empty(), "reqwest used outside net.rs: {offenders:?}");
}

fn collect_reqwest_offenders(dir: &std::path::Path, offenders: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_reqwest_offenders(&path, offenders);
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| ALLOWED_REQWEST_FILES.contains(&n)) {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        if text.contains("reqwest") {
            offenders.push(path.display().to_string());
        }
    }
}

#[test]
fn lockfile_has_no_network_plugin() {
    let lock = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.lock")).expect("Cargo.lock exists");
    let re = regex::Regex::new(r#"(?m)^name = "([^"]+)""#).unwrap();
    let found: Vec<&str> = re.captures_iter(&lock).map(|c| c.get(1).unwrap().as_str()).filter(|n| BANNED_PLUGINS.contains(n)).collect();
    assert!(found.is_empty(), "network plugin in Cargo.lock: {found:?}");
}
