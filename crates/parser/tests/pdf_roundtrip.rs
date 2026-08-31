mod gen;
use parser::fold::fold;
use parser::lines::from_text;
use parser::{extract_pages, parse_pages, parse_text, Checksum, ParseError};

fn fixture(name: &str) -> String { std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap() }

fn require_pdfium() {
    let dir = parser::pdfium::library_dir();
    assert!(dir.join("pdfium.dll").exists(), "pdfium.dll missing in {dir:?}: run scripts/fetch-pdfium.ps1 and set ABAKUS_PDFIUM_DIR");
}

#[test]
fn synthetic_pdf_parses_like_its_text_source() {
    require_pdfium();
    let text = fixture("personal-2026-06.txt");
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("personal.pdf");
    gen::write_pdf(&from_text(&text), &pdf);
    let from_pdf = parse_pages(&extract_pages(&pdf, None).unwrap()).unwrap();
    let from_txt = parse_text(&text).unwrap();
    assert_eq!(from_pdf.transactions.len(), from_txt.transactions.len());
    assert_eq!(from_pdf.transactions.iter().map(|t| t.amount_cents).collect::<Vec<_>>(), from_txt.transactions.iter().map(|t| t.amount_cents).collect::<Vec<_>>());
    assert_eq!(from_pdf.checksum(), Checksum::Ok);
    assert_eq!(from_pdf.transactions[0].merchant_raw, "ALDI SUED");
    assert_eq!(from_pdf.transactions[1].counterparty_iban, from_txt.transactions[1].counterparty_iban);
    // fixtures_txt.rs (personal_checksum_is_ok_and_the_only_warning_is_the_unknown_fee_kind) says
    // this fixture carries exactly one warning, the unclassified fee record; the PDF path keeps
    // it too. Compared through `fold` because the synthetic PDF's labels are ASCII-folded
    // (`strip_marks`, Courier/WinAnsi cannot encode the diacritics in "účtu").
    assert_eq!(from_pdf.warnings.len(), from_txt.warnings.len(), "{:?} vs {:?}", from_pdf.warnings, from_txt.warnings);
    assert_eq!(fold(&from_pdf.warnings[0]), fold(&from_txt.warnings[0]));
}

#[test]
fn a_non_statement_pdf_is_rejected() {
    require_pdfium();
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("x.pdf");
    gen::write_pdf(&[vec!["hello".into(), "world".into()]], &pdf);
    assert!(matches!(parser::parse_pdf(&pdf, None), Err(ParseError::NotAStatement(_))));
}

#[test]
fn missing_file_is_a_pdf_error() {
    require_pdfium();
    assert!(matches!(parser::parse_pdf(std::path::Path::new("does-not-exist.pdf"), None), Err(ParseError::Pdf(_))));
}
