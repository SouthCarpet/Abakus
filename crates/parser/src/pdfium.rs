use crate::lines::{group_lines, Char};
use crate::model::{ParseError, Statement};
use crate::statement::parse_pages;
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub fn library_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("ABAKUS_PDFIUM_DIR") { return PathBuf::from(d); }
    // A17/F9: `cargo test` runs the test binary from `target/`, so the
    // current_exe()-relative fallback below never finds the dll there, and
    // the documented bare test command failed. The repo's own copy, baked in
    // at compile time via CARGO_MANIFEST_DIR, exists only on a dev machine
    // building from source; a real install has no such path, so this check
    // falls through to the packaged-app fallback unchanged.
    let repo_copy = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src-tauri/resources/pdfium"));
    if repo_copy.join("pdfium.dll").exists() { return repo_copy; }
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("resources").join("pdfium"))).unwrap_or_else(|| PathBuf::from("."))
}

/// Pdfium's own global bindings slot can be initialized once per process (`Pdfium::new` asserts
/// on a second call); a second `extract_pages` call in the same test binary would otherwise turn
/// `bind_to_library` into `Err(PdfiumLibraryBindingsAlreadyInitialized)`. Cache the bound
/// instance here so every caller in the process shares it.
static PDFIUM: OnceLock<Result<Pdfium, String>> = OnceLock::new();

fn bind() -> Result<&'static Pdfium, ParseError> {
    let dir = library_dir();
    PDFIUM
        .get_or_init(|| {
            let name = Pdfium::pdfium_platform_library_name_at_path(&dir);
            Pdfium::bind_to_library(name).map(Pdfium::new).map_err(|e| format!("pdfium not found in {dir:?}: {e}"))
        })
        .as_ref()
        .map_err(|e| ParseError::Pdf(e.clone()))
}

fn is_password_error(e: &PdfiumError) -> bool { format!("{e:?}").to_lowercase().contains("password") }

/// Pdfium's C++ engine is not reentrant: two calls in flight on different threads at once (as
/// `cargo test`'s default parallel runner does) crash the process (observed: `STATUS_ILLEGAL_INSTRUCTION`
/// on the second concurrent `extract_pages`, gone under `--test-threads=1`). The `thread_safe`
/// pdfium-render feature only makes the handle movable between threads; callers still have to
/// serialize their own calls, which this lock does.
static CALL_LOCK: Mutex<()> = Mutex::new(());

pub fn extract_pages(path: &Path, password: Option<&str>) -> Result<Vec<Vec<String>>, ParseError> {
    if !path.exists() { return Err(ParseError::Pdf(format!("no such file: {}", path.display()))); }
    let pdfium = bind()?;
    let _guard = CALL_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let doc = pdfium.load_pdf_from_file(path, password).map_err(|e| if is_password_error(&e) { ParseError::Encrypted } else { ParseError::Pdf(e.to_string()) })?;
    let mut pages = Vec::new();
    for page in doc.pages().iter() {
        let text = page.text().map_err(|e| ParseError::Pdf(e.to_string()))?;
        let chars: Vec<Char> = text
            .chars()
            .iter()
            .map(|c| {
                let b = c.loose_bounds().unwrap_or(PdfRect::new_from_values(0.0, 0.0, 0.0, 0.0));
                Char { x: b.left().value, y: b.bottom().value, w: b.width().value.max(0.1), h: b.height().value.max(1.0), ch: c.unicode_char().unwrap_or(' ') }
            })
            .collect();
        pages.push(group_lines(&chars).into_iter().map(|l| l.text).collect());
    }
    Ok(pages)
}

pub fn parse_pdf(path: &Path, password: Option<&str>) -> Result<Statement, ParseError> { parse_pages(&extract_pages(path, password)?) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn env_override_wins() { std::env::set_var("ABAKUS_PDFIUM_DIR", "X:/pdfium"); assert_eq!(library_dir(), PathBuf::from("X:/pdfium")); std::env::remove_var("ABAKUS_PDFIUM_DIR"); }

    /// A17/F9: without the env var, the bare `cargo test` command must still
    /// find the dll the repo already ships in `src-tauri/resources/pdfium`.
    #[test] fn falls_back_to_the_repo_resource_dir_when_the_env_var_is_absent() {
        std::env::remove_var("ABAKUS_PDFIUM_DIR");
        let dir = library_dir();
        assert!(dir.join("pdfium.dll").exists(), "expected the repo's src-tauri/resources/pdfium/pdfium.dll, got {dir:?}");
    }

    #[test] fn encrypted_is_detected_from_the_pdfium_error_debug_text() {
        // Unit coverage for the `Encrypted` mapping (spec: lopdf cannot write encrypted files,
        // so the real locked path is proved by Michal's local run, not a fixture).
        let e = PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError);
        assert!(is_password_error(&e));
        let e = PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FormatError);
        assert!(!is_password_error(&e));
    }
}
