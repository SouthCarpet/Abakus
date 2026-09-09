mod gen;
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
    // fixtures_txt.rs (personal_checksum_is_ok_and_the_known_fee_produces_no_warning) says this
    // fixture's fee record ("Poplatok za vedenie účtu") is now a recognized shape and carries no
    // warning (2026-09-09 continuation fix); the PDF path must agree.
    assert_eq!(from_pdf.warnings, Vec::<String>::new(), "{:?}", from_pdf.warnings);
    assert_eq!(from_txt.warnings, Vec::<String>::new(), "{:?}", from_txt.warnings);
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
