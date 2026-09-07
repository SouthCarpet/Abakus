use super::{render_pdf_to, PdfReportLimits, PdfReportOutcome};
use std::io::{self, ErrorKind, Write};
use std::path::Path;

pub(crate) fn write(
    snapshot: &store::report::ReportSnapshot,
    destination: &Path,
    limits: PdfReportLimits,
    fault: PdfIoFault,
) -> Result<PdfReportOutcome, String> {
    validate_destination(destination)?;
    let parent = destination
        .parent()
        .ok_or_else(|| "Cieľ PDF nemá nadradený priečinok.".to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        format!("Do cieľového priečinka sa nedá vytvoriť dočasný súbor: {error}")
    })?;
    let pages = {
        let mut bounded = ByteLimitedWriter::new(&mut temporary, limits.max_bytes);
        render_pdf_to(snapshot, limits, &mut bounded)?
    };
    let bytes = finish_file(&mut temporary, limits, fault)?;
    publish_file(temporary, destination, fault)?;
    Ok(PdfReportOutcome {
        path: destination.to_string_lossy().into_owned(),
        bytes,
        pages,
        report: snapshot.preview.clone(),
    })
}

struct ByteLimitedWriter<W> {
    inner: W,
    maximum: u64,
    written: u64,
}

impl<W: Write> ByteLimitedWriter<W> {
    fn new(inner: W, maximum: u64) -> Self {
        Self {
            inner,
            maximum,
            written: 0,
        }
    }
}

impl<W: Write> Write for ByteLimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = self.maximum.saturating_sub(self.written);
        if remaining == 0 && !bytes.is_empty() {
            return Err(io::Error::other("PDF prekročilo povolený počet bajtov"));
        }
        let available = usize::try_from(remaining).unwrap_or(usize::MAX);
        let count = self.inner.write(&bytes[..bytes.len().min(available)])?;
        self.written = self
            .written
            .checked_add(u64::try_from(count).map_err(io::Error::other)?)
            .ok_or_else(|| io::Error::other("Počet zapísaných bajtov PDF pretiekol"))?;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn finish_file(
    temporary: &mut tempfile::NamedTempFile,
    limits: PdfReportLimits,
    fault: PdfIoFault,
) -> Result<u64, String> {
    inject_fault(
        fault,
        PdfIoFault::Flush,
        "Dokončenie zápisu PDF zlyhalo pri flush.",
    )?;
    temporary
        .as_file_mut()
        .flush()
        .map_err(|error| format!("Uloženie PDF na disk zlyhalo pri dokončení zápisu: {error}"))?;
    inject_fault(
        fault,
        PdfIoFault::Sync,
        "Synchronizácia PDF na disk zlyhala.",
    )?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| format!("Synchronizácia PDF na disk zlyhala: {error}"))?;
    let bytes = temporary
        .as_file()
        .metadata()
        .map_err(|error| format!("Veľkosť dokončeného PDF sa nedá zistiť: {error}"))?
        .len();
    enforce_byte_limit(bytes, limits.max_bytes)?;
    Ok(bytes)
}

fn publish_file(
    temporary: tempfile::NamedTempFile,
    destination: &Path,
    fault: PdfIoFault,
) -> Result<(), String> {
    inject_fault(
        fault,
        PdfIoFault::Persist,
        "Publikovanie dokončeného PDF zlyhalo.",
    )?;
    temporary
        .persist_noclobber(destination)
        .map_err(|error| persist_error(error, destination))?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfIoFault {
    None,
    Flush,
    Sync,
    Persist,
}

fn inject_fault(actual: PdfIoFault, stage: PdfIoFault, message: &str) -> Result<(), String> {
    if actual == stage {
        return Err(format!("{message} Injektovaná testovacia chyba."));
    }
    Ok(())
}

fn validate_destination(destination: &Path) -> Result<(), String> {
    let text = destination
        .to_str()
        .ok_or_else(|| "Cesta PDF nie je platný text Unicode.".to_string())?;
    if text.contains('\0') {
        return Err("Cesta PDF obsahuje zakázaný znak NUL.".into());
    }
    if !destination.is_absolute() {
        return Err("Zvoľte úplnú lokálnu cestu pre PDF.".into());
    }
    validate_windows_path(text)?;
    let extension = destination.extension().and_then(|value| value.to_str());
    if !extension.is_some_and(|value| value.eq_ignore_ascii_case("pdf")) {
        return Err("Cieľový súbor musí mať príponu .pdf.".into());
    }
    match std::fs::symlink_metadata(destination) {
        Ok(_) => Err(format!(
            "Cieľový súbor už existuje: {}. Zvoľte iný názov.",
            destination.display()
        )),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Cieľový súbor sa nedá skontrolovať: {error}")),
    }
}

fn validate_windows_path(text: &str) -> Result<(), String> {
    if text.starts_with("\\\\")
        || text.starts_with("//")
        || text.starts_with("\\\\?\\")
        || text.starts_with("\\\\.\\")
    {
        return Err("Sieťové cesty a cesty zariadení nie sú povolené pre PDF.".into());
    }
    #[cfg(windows)]
    {
        let bytes = text.as_bytes();
        let drive_path = bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/');
        if !drive_path {
            return Err("Zvoľte obyčajnú absolútnu cestu na lokálnom disku.".into());
        }
        if text[3..].contains(':') {
            return Err("Alternatívne dátové prúdy nie sú povolené pre PDF.".into());
        }
        let leaf = text.rsplit(['\\', '/']).next().unwrap_or_default();
        if is_windows_device_name(leaf) {
            return Err("Názov cieľového súboru je vyhradený pre zariadenie Windows.".into());
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_windows_device_name(leaf: &str) -> bool {
    let stem = leaf
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "CLOCK$"
            | "COM¹"
            | "COM²"
            | "COM³"
            | "LPT¹"
            | "LPT²"
            | "LPT³"
    ) {
        return true;
    }
    let bytes = stem.as_bytes();
    bytes.len() == 4 && matches!(&bytes[..3], b"COM" | b"LPT") && matches!(bytes[3], b'1'..=b'9')
}

fn enforce_byte_limit(bytes: u64, maximum: u64) -> Result<(), String> {
    if bytes > maximum {
        return Err(format!("Report prekročil limit {maximum} bajtov PDF. Zvoľte kratšie obdobie alebo menej účtov."));
    }
    Ok(())
}

fn persist_error(error: tempfile::PersistError, destination: &Path) -> String {
    if error.error.kind() == ErrorKind::AlreadyExists {
        format!(
            "Cieľový súbor už existuje: {}. Zvoľte iný názov.",
            destination.display()
        )
    } else {
        format!("Publikovanie dokončeného PDF zlyhalo: {}", error.error)
    }
}
