//! D9: no workspace crate declares a network dependency, and no Tauri
//! network plugin enters the dependency tree. Tauri's own transitive
//! reqwest/hyper internals are outside the app's reach and not banned.
use std::fs;

const BANNED_DIRECT: &[&str] = &[
    "reqwest", "hyper", "ureq", "curl", "isahc", "attohttpc", "tokio-tungstenite",
    "tauri-plugin-http", "tauri-plugin-updater", "tauri-plugin-shell", "tauri-plugin-opener",
];
const BANNED_PLUGINS: &[&str] = &[
    "tauri-plugin-http", "tauri-plugin-updater", "tauri-plugin-shell", "tauri-plugin-opener",
];
const MANIFESTS: &[&str] = &[
    "/../Cargo.toml", "/../crates/parser/Cargo.toml", "/../crates/rules/Cargo.toml",
    "/../crates/store/Cargo.toml", "/Cargo.toml",
];

#[test]
fn no_manifest_declares_a_network_crate() {
    for m in MANIFESTS {
        let text = fs::read_to_string(format!("{}{m}", env!("CARGO_MANIFEST_DIR"))).expect(m);
        let hits: Vec<&&str> = BANNED_DIRECT.iter().filter(|n| {
            let re = regex::Regex::new(&format!(r#"(?m)^\s*"?{}"?\s*="#, regex::escape(n))).unwrap();
            re.is_match(&text)
        }).collect();
        assert!(hits.is_empty(), "network dependency declared in {m}: {hits:?}");
    }
}

#[test]
fn lockfile_has_no_network_plugin() {
    let lock = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.lock")).expect("Cargo.lock exists");
    let re = regex::Regex::new(r#"(?m)^name = "([^"]+)""#).unwrap();
    let found: Vec<&str> = re.captures_iter(&lock).map(|c| c.get(1).unwrap().as_str()).filter(|n| BANNED_PLUGINS.contains(n)).collect();
    assert!(found.is_empty(), "network plugin in Cargo.lock: {found:?}");
}
