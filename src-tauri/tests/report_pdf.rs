//! Renderer, Unicode, pagination, bounds and no-clobber publication oracle.
use abakus_lib::report::{
    render_pdf_to, write_pdf_report, write_pdf_report_with_fault, write_pdf_report_with_limits,
    PdfIoFault, PdfReportLimits,
};
use chrono::NaiveDate;
use parser::AccountKind;
use rules::Status;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use store::report::{
    ReportAccount, ReportCategoryTotal, ReportCoverage, ReportDateRange, ReportMonth,
    ReportPreview, ReportSnapshot, ReportTotals, ReportTransaction,
};
use store::{CategoryKind, TxRow};

fn day(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

fn transaction(id: i64, merchant: &str, note: &str, cents: i64) -> ReportTransaction {
    ReportTransaction {
        transaction: TxRow {
            id,
            account_id: 1,
            account_kind: AccountKind::Personal,
            statement_number: 1,
            posted_date: day("2026-09-07"),
            tx_date: day("2026-09-07"),
            kind: "card".into(),
            amount_cents: cents,
            orig_amount_cents: None,
            orig_currency: None,
            merchant_raw: merchant.into(),
            place: Some("Bratislava".into()),
            counterparty_name: None,
            counterparty_iban: None,
            category_id: None,
            category_name: None,
            parent_name: None,
            status: Status::Confirmed,
            source: "none".into(),
            raw_block: "must never appear in PDF".into(),
            note: note.into(),
        },
        account_label: "Domácnosť".into(),
        category_kind: None,
    }
}

fn snapshot(rows: Vec<ReportTransaction>) -> ReportSnapshot {
    let count = u32::try_from(rows.len()).unwrap();
    let income = rows
        .iter()
        .filter(|row| row.transaction.amount_cents > 0)
        .map(|row| row.transaction.amount_cents)
        .sum();
    let expense = rows
        .iter()
        .filter(|row| row.transaction.amount_cents < 0)
        .map(|row| -row.transaction.amount_cents)
        .sum();
    ReportSnapshot {
        preview: ReportPreview {
            range: Some(ReportDateRange {
                from: day("2026-09-01"),
                to: day("2026-09-30"),
            }),
            scope_label: "Všetky účty".into(),
            accounts: vec![ReportAccount {
                id: 1,
                label: "Domácnosť".into(),
                kind: AccountKind::Personal,
                iban_suffix: "5678".into(),
            }],
            transaction_count: count,
            latest_transaction_date: if rows.is_empty() {
                None
            } else {
                Some(day("2026-09-07"))
            },
            captured_at: "2026-09-07T14:30:00+02:00".into(),
            unfinished_period: true,
            accounts_without_statements: 0,
            accounts_with_gaps: 0,
            unverified_statement_count: 0,
            invalid_statement_range_count: 0,
        },
        totals: ReportTotals {
            income_cents: income,
            expense_cents: expense,
            net_cents: income - expense,
            transfer_out_cents: 0,
            transfer_in_cents: 0,
            suggested_count: 0,
            unassigned_count: 0,
        },
        months: vec![ReportMonth {
            month: "2026-09".into(),
            income_cents: income,
            expense_cents: expense,
        }],
        categories: Vec::new(),
        coverage: vec![ReportCoverage {
            account_id: 1,
            known_range: Some(ReportDateRange {
                from: day("2026-09-01"),
                to: day("2026-09-30"),
            }),
            gaps: Vec::new(),
            statement_count: 1,
            unverified_statement_count: 0,
            invalid_range_count: 0,
        }],
        transactions: rows,
    }
}

fn write_and_extract(
    report: &ReportSnapshot,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf, Vec<Vec<String>>) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(name);
    write_pdf_report(report, &path).unwrap();
    let pages = parser::extract_pages(&path, None).unwrap();
    if let Some(output) = std::env::var_os("ABAKUS_PDF_TEST_OUTPUT") {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir_all(&output).unwrap();
        let retained = output.join(name);
        assert!(!retained.exists(), "test output must be a new path");
        std::fs::copy(&path, retained).unwrap();
    }
    (directory, path, pages)
}

fn extracted_text(pages: &[Vec<String>]) -> String {
    let text = pages
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn transaction_text(pages: &[Vec<String>]) -> String {
    let first = pages
        .iter()
        .position(|page| {
            page.iter()
                .any(|line| line.contains("Úplný výpis transakcií"))
        })
        .expect("report must have a transaction section");
    let end = pages[first..]
        .iter()
        .position(|page| page.iter().any(|line| line.contains("Pokrytie výpismi")))
        .map_or(pages.len(), |offset| first + offset);
    extracted_text(&pages[first..end])
}

#[cfg(windows)]
fn assert_windows_device_leaves_are_rejected(report: &ReportSnapshot, directory: &std::path::Path) {
    for leaf in [
        "NUL.pdf",
        "CON.pdf",
        "COM¹.pdf",
        "COM².pdf",
        "COM³.pdf",
        "LPT¹.pdf",
        "LPT².pdf",
        "LPT³.pdf",
    ] {
        let destination = directory.join(leaf);
        assert!(write_pdf_report(report, &destination).is_err());
        assert!(!destination.exists());
    }
}

#[cfg(windows)]
fn assert_ordinary_windows_leaves_are_allowed(
    report: &ReportSnapshot,
    directory: &std::path::Path,
) {
    for leaf in ["COM10.pdf", "report.pdf"] {
        let destination = directory.join(leaf);
        assert!(write_pdf_report(report, &destination).is_ok());
        assert!(destination.is_file());
    }
}

#[test]
fn p06_empty_report_has_real_pages_and_no_invalid_numeric_text() {
    let mut report = snapshot(Vec::new());
    report.preview.accounts_without_statements = 1;
    report.preview.accounts_with_gaps = 1;
    report.coverage[0] = ReportCoverage {
        account_id: 1,
        known_range: None,
        gaps: vec![report.preview.range.unwrap()],
        statement_count: 0,
        unverified_statement_count: 0,
        invalid_range_count: 0,
    };
    let (_directory, _path, pages) = write_and_extract(&report, "empty.pdf");
    let text = extracted_text(&pages);
    assert!(text.contains("Bez transakcií"));
    assert!(text.contains("Bez výpisov"));
    assert!(!text.contains("Bez medzier v známom rozsahu"));
    assert!(text.contains("zaznamenané transakcie: 0"));
    assert!(!text.contains("NaN"));
    assert!(!text.contains("Infinity"));
    assert!(pages.iter().enumerate().all(|(index, page)| page
        .iter()
        .any(|line| line.contains(&format!("Strana {} z {}", index + 1, pages.len())))));
}

#[test]
fn p06_i64_minimum_transaction_formats_without_negation_overflow() {
    let mut row = transaction(1, "minimum-category-mismatch", "", i64::MIN);
    row.category_kind = Some(CategoryKind::Income);
    row.transaction.category_id = Some(40);
    row.transaction.category_name = Some("Príjem".into());
    let mut report = snapshot(Vec::new());
    report.preview.transaction_count = 1;
    report.preview.latest_transaction_date = Some(day("2026-09-07"));
    report.transactions = vec![row];
    let (_directory, _path, pages) = write_and_extract(&report, "minimum.pdf");
    assert!(extracted_text(&pages).contains("-92 233 720 368 547 758,08 €"));
}

#[test]
fn p06_refund_only_report_keeps_negative_expense_in_both_chart_keys() {
    let mut row = transaction(1, "refund-only", "", 2_500);
    row.transaction.kind = "refund".into();
    row.transaction.category_id = Some(20);
    row.transaction.category_name = Some("Potraviny".into());
    row.category_kind = Some(CategoryKind::Expense);
    let mut report = snapshot(vec![row]);
    report.totals = ReportTotals {
        income_cents: 0,
        expense_cents: -2_500,
        net_cents: 2_500,
        transfer_out_cents: 0,
        transfer_in_cents: 0,
        suggested_count: 0,
        unassigned_count: 0,
    };
    report.months[0].income_cents = 0;
    report.months[0].expense_cents = -2_500;
    report.categories = vec![ReportCategoryTotal {
        category_id: Some(20),
        label: "Potraviny".into(),
        cents: -2_500,
    }];
    let (_directory, _path, pages) = write_and_extract(&report, "refund-only.pdf");
    let text = extracted_text(&pages);
    assert!(text.contains("2026-09: príjmy 0,00 €, výdavky -25,00 €"));
    assert!(text.contains("Potraviny · -25,00 €"));
}

#[test]
fn p04_exact_money_transfers_categories_and_statuses_are_printed() {
    let mut income = transaction(8, "income", "", 100_000);
    income.category_kind = Some(CategoryKind::Income);
    income.transaction.category_id = Some(10);
    income.transaction.category_name = Some("Mzda".into());
    income.transaction.posted_date = day("2026-09-06");
    let mut expense = transaction(7, "expense", "", -10_000);
    expense.category_kind = Some(CategoryKind::Expense);
    expense.transaction.category_id = Some(20);
    expense.transaction.category_name = Some("Potraviny".into());
    let mut refund = transaction(6, "refund", "", 2_500);
    refund.category_kind = Some(CategoryKind::Expense);
    refund.transaction.category_id = Some(20);
    refund.transaction.category_name = Some("Potraviny".into());
    refund.transaction.kind = "refund".into();
    let mut suggested = transaction(5, "suggested", "", -3_000);
    suggested.transaction.status = Status::Suggested;
    let mut unassigned_refund = transaction(4, "unassigned-refund", "", 500);
    unassigned_refund.transaction.kind = "refund".into();
    unassigned_refund.transaction.status = Status::Unassigned;
    let mut transfer_out = transaction(3, "transfer-out", "", -20_000);
    transfer_out.transaction.status = Status::Transfer;
    let mut transfer_in = transaction(2, "transfer-in", "", 20_000);
    transfer_in.transaction.status = Status::Transfer;
    let zero = transaction(1, "zero", "", 0);
    let mut report = snapshot(vec![
        income,
        expense,
        refund,
        suggested,
        unassigned_refund,
        transfer_out,
        transfer_in,
        zero,
    ]);
    report.totals = ReportTotals {
        income_cents: 100_000,
        expense_cents: 10_000,
        net_cents: 90_000,
        transfer_out_cents: 20_000,
        transfer_in_cents: 20_000,
        suggested_count: 1,
        unassigned_count: 1,
    };
    report.months[0].income_cents = 100_000;
    report.months[0].expense_cents = 10_000;
    report.categories = vec![
        ReportCategoryTotal {
            category_id: Some(20),
            label: "Potraviny".into(),
            cents: 7_500,
        },
        ReportCategoryTotal {
            category_id: None,
            label: "Nezaradené".into(),
            cents: 2_500,
        },
    ];
    let (_directory, _path, pages) = write_and_extract(&report, "money-statuses.pdf");
    let text = extracted_text(&pages);
    for literal in [
        "1 000,00 €",
        "100,00 €",
        "900,00 €",
        "Navrhnuté",
        "Nezaradené",
        "Prevod",
        "Súčet kategórií: 100,00 €",
        "Nezaradené · 25,00 €",
        "2026-09: príjmy 1 000,00 €, výdavky 100,00 €",
    ] {
        assert!(text.contains(literal), "missing literal {literal}");
    }
    assert!(text.contains("Zaúčtované:"));
    assert!(text.contains("06.09.2026"));
}

#[test]
fn p05_category_sign_mismatches_remain_visible_with_signed_category_values() {
    let mut positive_expense = transaction(4, "positive-expense", "", 700);
    positive_expense.category_kind = Some(CategoryKind::Expense);
    positive_expense.transaction.category_id = Some(30);
    positive_expense.transaction.category_name = Some("Vrátenia".into());
    positive_expense.transaction.status = Status::Suggested;
    let mut negative_income = transaction(3, "negative-income", "", -900);
    negative_income.category_kind = Some(CategoryKind::Income);
    negative_income.transaction.category_id = Some(40);
    negative_income.transaction.category_name = Some("Príjem".into());
    let mut income_refund = transaction(2, "income-refund", "", 300);
    income_refund.category_kind = Some(CategoryKind::Income);
    income_refund.transaction.category_id = Some(40);
    income_refund.transaction.category_name = Some("Príjem".into());
    income_refund.transaction.kind = "refund".into();
    let uncategorized_income = transaction(1, "uncategorized-income", "", 1_100);
    let mut report = snapshot(vec![
        positive_expense,
        negative_income,
        income_refund,
        uncategorized_income,
    ]);
    report.totals = ReportTotals {
        income_cents: 1_100,
        expense_cents: -700,
        net_cents: 1_800,
        transfer_out_cents: 0,
        transfer_in_cents: 0,
        suggested_count: 1,
        unassigned_count: 0,
    };
    report.months[0].income_cents = 1_100;
    report.months[0].expense_cents = -700;
    report.categories = vec![ReportCategoryTotal {
        category_id: Some(30),
        label: "Vrátenia".into(),
        cents: -700,
    }];
    let (_directory, _path, pages) = write_and_extract(&report, "category-signs.pdf");
    let text = extracted_text(&pages);
    for literal in [
        "Príjmy",
        "11,00 €",
        "Výdavky",
        "-7,00 €",
        "Rozdiel príjmov a výdavkov",
        "18,00 €",
        "2026-09: príjmy 11,00 €, výdavky -7,00 €",
        "positive-expense",
        "negative-income",
        "income-refund",
    ] {
        assert!(text.contains(literal), "missing literal {literal}");
    }
    assert!(text.contains("Vrátenia"));
    assert!(text.contains("Navrhnuté"));
}

#[test]
fn p07_coverage_summary_and_detail_keep_gaps_and_quality_separate() {
    let mut report = snapshot(Vec::new());
    report.preview.accounts_with_gaps = 1;
    report.preview.unverified_statement_count = 2;
    report.preview.invalid_statement_range_count = 1;
    report.coverage[0] = ReportCoverage {
        account_id: 1,
        known_range: Some(ReportDateRange {
            from: day("2026-09-01"),
            to: day("2026-09-30"),
        }),
        gaps: vec![ReportDateRange {
            from: day("2026-09-12"),
            to: day("2026-09-12"),
        }],
        statement_count: 3,
        unverified_statement_count: 2,
        invalid_range_count: 1,
    };
    let (_directory, _path, pages) = write_and_extract(&report, "coverage.pdf");
    let text = extracted_text(&pages);
    assert!(text.contains("Účty s medzerami: 1"));
    assert!(text.contains("odchýlkou alebo neoveriteľným kontrolným súčtom: 2"));
    assert!(text.contains("Neplatné rozsahy výpisov: 1"));
    assert!(text.contains("Medzera: 12.09.2026 až 12.09.2026"));
}

#[test]
fn p09_complete_listing_and_more_than_twenty_four_years_keep_every_bucket() {
    let rows = (1..=31)
        .rev()
        .map(|id| transaction(id, &format!("SENTINEL-{id:02}"), "", -100))
        .collect();
    let mut report = snapshot(rows);
    report.preview.range = Some(ReportDateRange {
        from: day("1999-01-01"),
        to: day("2026-12-31"),
    });
    report.months = (1999..=2026)
        .flat_map(|year| (1..=12).map(move |month| (year, month)))
        .map(|(year, month)| ReportMonth {
            month: format!("{year}-{month:02}"),
            income_cents: 0,
            expense_cents: if (year, month) == (2026, 9) { 3_100 } else { 0 },
        })
        .collect();
    report.categories = vec![ReportCategoryTotal {
        category_id: None,
        label: "Nezaradené".into(),
        cents: 3_100,
    }];
    let (_directory, _path, pages) = write_and_extract(&report, "all-time.pdf");
    let text = extracted_text(&pages);
    for id in 1..=31 {
        assert_eq!(
            text.matches(&format!("SENTINEL-{id:02}")).count(),
            1,
            "each selected transaction must appear exactly once"
        );
    }
    assert!(
        text.find("SENTINEL-31") < text.find("SENTINEL-01"),
        "renderer must preserve the captured descending row order"
    );
    for year in 1999..=2026 {
        assert!(
            text.contains(&year.to_string()),
            "year bucket {year} must not be hidden"
        );
    }
    assert!(
        text.contains("Panel 2"),
        "more than 24 years must continue in another named panel"
    );
    assert!(text.contains("2026: príjmy 0,00 €, výdavky 31,00 € · zaznamenané transakcie: 31"));
    assert!(text.contains("1999: príjmy 0,00 €, výdavky 0,00 € · zaznamenané transakcie: 0"));
    assert!(!text.contains("must never appear in PDF"));
}

#[test]
fn p10_top_ten_plus_other_discloses_hidden_opposite_sign_categories_and_reconciles() {
    let mut report = snapshot(vec![transaction(1, "category-oracle", "", -10_000)]);
    report.totals.expense_cents = 550_000;
    report.totals.net_cents = -550_000;
    report.categories = (1..=12)
        .rev()
        .map(|id| ReportCategoryTotal {
            category_id: (id != 11).then_some(id),
            label: if id == 1 {
                "Domácnosť / Potraviny".into()
            } else if id == 11 {
                "Nezaradené".into()
            } else {
                format!("Kategória {id}")
            },
            cents: if id == 12 { -55_000 } else { 55_000 },
        })
        .collect();
    let (_directory, _path, pages) = write_and_extract(&report, "categories.pdf");
    let text = extracted_text(&pages);
    assert!(text.contains("Ostatné (2 kategórií)"));
    assert!(text.contains("Ostatné (2 kategórií) · 0,00 €"));
    assert!(text.contains("Domácnosť / Potraviny"));
    assert!(text.contains("Súčet kategórií: 5 500,00 €"));
    assert!(text.contains("KPI výdavkov: 5 500,00 €"));
}

#[test]
fn p11_slovak_nfc_nfd_controls_and_unsupported_emoji_are_selectable_and_visible() {
    let nfd = "L\u{030C}ubim c\u{030C}uc\u{030C}oriedky";
    let note_prefix = format!("{nfd} (zátvorky) \\ lomka\tTAB-END riadenie \u{0001}\n");
    let note_suffix = "\nFINAL-NOTE-SENTINEL 😀";
    let fill = 2_000_usize
        .checked_sub(note_prefix.chars().count() + note_suffix.chars().count())
        .unwrap();
    let note = format!("{note_prefix}{}{note_suffix}", "ž".repeat(fill));
    assert_eq!(note.chars().count(), 2_000);
    let merchant = format!(
        "Ľúbim čučoriedky. Kŕdeľ ďatľov učí koňa žrať kôru. 1 234,56 € {} MERCHANT-END",
        "M".repeat(400)
    );
    let row = transaction(1, &merchant, &note, -123_456);
    let (_directory, _path, pages) = write_and_extract(&snapshot(vec![row]), "unicode.pdf");
    let text = extracted_text(&pages);
    let selectable_text = text
        .chars()
        .filter(|scalar| !scalar.is_whitespace())
        .collect::<String>();
    for expected in [
        "Ľúbimčučoriedky",
        "Kŕdeľ",
        "ďatľov",
        "učí",
        "koňa",
        "žrať",
        "kôru",
        "Ľubimčučoriedky",
        "(zátvorky)",
        "\\lomka",
    ] {
        assert!(
            selectable_text.contains(expected),
            "extracted report text is missing {expected:?}"
        );
    }
    assert!(text.contains("MERCHANT-END"));
    assert!(text.contains("[U+1F600]"));
    assert!(text.contains("[U+0001]"));
    assert!(text.contains("TAB-END"));
    assert!(text.contains("FINAL-NOTE-SENTINEL"));
    assert!(text.contains("Poznámka k znakom"));
    assert!(text.contains("2 nepodporovaných alebo riadiacich znakov"));
}

fn pagination_snapshot() -> ReportSnapshot {
    let mut crossing = transaction(
        301,
        "one-row-crosses-a-page",
        &format!("{} FINAL-LONG-ROW", "wrap ".repeat(1_500)),
        -777,
    );
    crossing.account_label = format!("{} ACCTEND", "Dlhý účet ".repeat(20));
    crossing.transaction.category_name =
        Some(format!("{} CATEGORY-END", "Dlhá kategória ".repeat(20)));
    let mut rows = vec![crossing];
    rows.extend((1..=300).rev().map(|id| {
        transaction(
            id,
            &format!("ROW-{id:03}"),
            if id == 1 { "FINAL-REPORT-NOTE" } else { "" },
            -100,
        )
    }));
    snapshot(rows)
}

fn assert_pagination_content(pages: &[Vec<String>]) {
    let text = extracted_text(pages);
    let table_text = transaction_text(pages);
    assert!(pages.len() > 5);
    assert!(text.contains("Pokračovanie položky 1"));
    assert!(text.contains("FINAL-LONG-ROW"));
    assert!(text.contains("ACCTEND"));
    assert!(text.contains("CATEGORY-END"));
    assert!(text.contains("FINAL-REPORT-NOTE"));
    assert!(text.contains("1. 07.09.2026"));
    assert!(text.contains("301. 07.09.2026"));
    assert_eq!(
        table_text.matches("-7,77 €").count(),
        1,
        "a continued row prints its amount once"
    );
}

fn assert_pagination_headers_and_footers(pages: &[Vec<String>]) {
    for (index, page) in pages.iter().enumerate() {
        assert!(
            page.iter().any(|line| line.contains(&format!(
                "Strana {} z {}",
                index + 1,
                pages.len()
            ))),
            "every page has an exact footer"
        );
        let has_transaction_fragment = page.iter().any(|line| {
            line.contains("ROW-")
                || line.contains("one-row-crosses-a-page")
                || line.contains("Pokračovanie položky")
        });
        if has_transaction_fragment {
            assert!(
                page.iter().any(|line| line.contains("Dátum")),
                "every transaction continuation page repeats the table header"
            );
        }
    }
}

#[test]
fn p12_wrapped_row_and_three_hundred_rows_keep_ordinals_headers_footers_and_final_note() {
    let report = pagination_snapshot();
    let (_directory, _path, pages) = write_and_extract(&report, "pagination.pdf");
    assert_pagination_content(&pages);
    assert_pagination_headers_and_footers(&pages);
}

#[test]
fn p12_exact_fit_stays_on_one_table_page_and_one_more_line_continues() {
    let exact_note = (1..=58)
        .map(|line| format!("exact-{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let over_note = (1..=59)
        .map(|line| format!("over-{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut exact_row = transaction(1, "exact-fit", &exact_note, -100);
    exact_row.transaction.place = None;
    let mut over_row = transaction(1, "one-line-over", &over_note, -100);
    over_row.transaction.place = None;
    let (_directory, _path, exact_pages) =
        write_and_extract(&snapshot(vec![exact_row]), "exact-fit.pdf");
    let (_directory, _path, over_pages) =
        write_and_extract(&snapshot(vec![over_row]), "one-line-over.pdf");
    let exact_text = extracted_text(&exact_pages);
    let over_text = extracted_text(&over_pages);
    let exact_table_text = transaction_text(&exact_pages);
    let over_table_text = transaction_text(&over_pages);
    assert!(!exact_text.contains("Pokračovanie položky 1"));
    assert!(exact_text.contains("exact-58"));
    assert!(over_text.contains("Pokračovanie položky 1"));
    assert!(over_text.contains("over-59"));
    assert_eq!(exact_table_text.matches("-1,00 €").count(), 1);
    assert_eq!(over_table_text.matches("-1,00 €").count(), 1);
    assert_eq!(over_pages.len(), exact_pages.len() + 1);
}

#[test]
fn p13_page_and_byte_limits_accept_the_boundary_and_reject_one_over_without_a_final_file() {
    assert_eq!(PdfReportLimits::default().max_pages, 2_000);
    assert_eq!(PdfReportLimits::default().max_bytes, 100 * 1024 * 1024);
    let report = snapshot(vec![transaction(1, "bounds", "", -100)]);
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.pdf");
    let baseline = write_pdf_report(&report, &first).unwrap();
    let baseline_pages = usize::try_from(baseline.pages).unwrap();
    let exact = directory.path().join("exact.pdf");
    let exact_outcome = write_pdf_report_with_limits(
        &report,
        &exact,
        PdfReportLimits {
            max_pages: baseline_pages,
            max_bytes: baseline.bytes,
        },
    )
    .unwrap();
    assert_eq!(exact_outcome.bytes, baseline.bytes);
    let page_over = directory.path().join("page-over.pdf");
    assert!(write_pdf_report_with_limits(
        &report,
        &page_over,
        PdfReportLimits {
            max_pages: baseline_pages - 1,
            max_bytes: u64::MAX
        }
    )
    .is_err());
    assert!(!page_over.exists());
    let byte_over = directory.path().join("byte-over.pdf");
    assert!(write_pdf_report_with_limits(
        &report,
        &byte_over,
        PdfReportLimits {
            max_pages: usize::MAX,
            max_bytes: baseline.bytes - 1
        }
    )
    .is_err());
    assert!(!byte_over.exists());
}

#[test]
fn p13_real_five_thousand_row_export_contains_every_distinct_sentinel() {
    let rows = (1..=5_000)
        .rev()
        .map(|id| transaction(id, &format!("LARGE-{id:04}"), "", -1))
        .collect();
    let report = snapshot(rows);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("large.pdf");
    let started = std::time::Instant::now();
    let outcome = write_pdf_report(&report, &path).unwrap();
    let elapsed = started.elapsed();
    let pages = parser::extract_pages(&path, None).unwrap();
    assert_eq!(outcome.report.transaction_count, 5_000);
    assert_eq!(usize::try_from(outcome.pages).unwrap(), pages.len());
    assert_eq!(outcome.bytes, std::fs::metadata(&path).unwrap().len());
    eprintln!(
        "large report: rows={}, pages={}, bytes={}, elapsed_ms={}",
        outcome.report.transaction_count,
        outcome.pages,
        outcome.bytes,
        elapsed.as_millis()
    );
    let words = extracted_text(&pages)
        .split_whitespace()
        .filter(|word| word.starts_with("LARGE-"))
        .map(str::to_string)
        .collect::<HashSet<_>>();
    assert_eq!(words.len(), 5_000);
    assert!(words.contains("LARGE-0001"));
    assert!(words.contains("LARGE-5000"));
}

fn assert_basic_destination_rejections(report: &ReportSnapshot, directory: &Path) {
    let occupied = directory.join("occupied.pdf");
    std::fs::write(&occupied, b"sentinel").unwrap();
    assert!(write_pdf_report(report, &occupied).is_err());
    assert_eq!(std::fs::read(&occupied).unwrap(), b"sentinel");
    assert!(write_pdf_report(report, &directory.join("wrong.db")).is_err());
    assert!(write_pdf_report(report, Path::new("relative.pdf")).is_err());
    let directory_target = directory.join("directory.pdf");
    std::fs::create_dir(&directory_target).unwrap();
    assert!(write_pdf_report(report, &directory_target).is_err());
}

#[cfg(windows)]
fn assert_windows_destination_rules(report: &ReportSnapshot, directory: &Path) {
    use std::os::windows::ffi::OsStringExt;

    assert!(write_pdf_report(report, Path::new(r"\\.\NUL.pdf")).is_err());
    assert_windows_device_leaves_are_rejected(report, directory);
    assert_ordinary_windows_leaves_are_allowed(report, directory);
    assert!(write_pdf_report(report, &directory.join("stream.pdf:secret")).is_err());
    assert!(write_pdf_report(report, Path::new("A:\\nul\0.pdf")).is_err());
    let mut units = "A:\\nonunicode-".encode_utf16().collect::<Vec<_>>();
    units.push(0xD800);
    units.extend(".pdf".encode_utf16());
    let nonunicode = std::path::PathBuf::from(std::ffi::OsString::from_wide(&units));
    assert!(write_pdf_report(report, &nonunicode).is_err());
}

fn assert_source_alias_rejections(report: &ReportSnapshot, directory: &Path) {
    let source = directory.join("source.db");
    std::fs::write(&source, b"database-source").unwrap();
    assert!(write_pdf_report(report, &source).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), b"database-source");
    let sidecars: [(&str, &[u8]); 2] = [
        ("source.db-wal", b"wal-source"),
        ("source.db-shm", b"shm-source"),
    ];
    for (name, contents) in sidecars {
        let sidecar = directory.join(name);
        std::fs::write(&sidecar, contents).unwrap();
        assert!(write_pdf_report(report, &sidecar).is_err());
        assert_eq!(std::fs::read(&sidecar).unwrap(), contents);
    }
    let hardlink = directory.join("source-alias.pdf");
    std::fs::hard_link(&source, &hardlink).unwrap();
    assert!(write_pdf_report(report, &hardlink).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), b"database-source");
    #[cfg(windows)]
    {
        let symlink = directory.join("source-symlink.pdf");
        if std::os::windows::fs::symlink_file(&source, &symlink).is_ok() {
            assert!(write_pdf_report(report, &symlink).is_err());
            assert_eq!(std::fs::read(&source).unwrap(), b"database-source");
        }
    }
}

fn assert_competing_publication(report: &Arc<ReportSnapshot>, directory: &Path) {
    let destination = directory.join("race.pdf");
    let first_report = Arc::clone(report);
    let first_path = destination.clone();
    let second_report = Arc::clone(report);
    let second_path = destination.clone();
    let first = std::thread::spawn(move || write_pdf_report(&first_report, &first_path));
    let second = std::thread::spawn(move || write_pdf_report(&second_report, &second_path));
    let outcomes = [first.join().unwrap(), second.join().unwrap()];
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_err()).count(),
        1
    );
    assert!(outcomes
        .iter()
        .find_map(|outcome| outcome.as_ref().err())
        .is_some_and(|error| error.contains("existuje")));
    assert!(!parser::extract_pages(&destination, None)
        .unwrap()
        .is_empty());
}

#[test]
fn p14_existing_alias_invalid_and_competing_destinations_never_clobber() {
    let report = Arc::new(snapshot(vec![transaction(1, "publication", "", -100)]));
    let directory = tempfile::tempdir().unwrap();
    assert_basic_destination_rejections(&report, directory.path());
    #[cfg(windows)]
    assert_windows_destination_rules(&report, directory.path());
    assert_source_alias_rejections(&report, directory.path());
    assert_competing_publication(&report, directory.path());
}

#[test]
fn p15_late_writer_and_real_filesystem_failures_propagate_without_success() {
    let report = snapshot(vec![transaction(1, "failure", "", -100)]);
    let mut writer = FailingWriter { accepted: 128 };
    let error = render_pdf_to(&report, PdfReportLimits::default(), &mut writer).unwrap_err();
    assert!(error.contains("Zápis PDF zlyhal"));
    let directory = tempfile::tempdir().unwrap();
    let missing_parent = directory.path().join("missing").join("report.pdf");
    assert!(write_pdf_report(&report, &missing_parent).is_err());
    assert!(!missing_parent.exists());
    for (name, fault) in [
        ("flush.pdf", PdfIoFault::Flush),
        ("sync.pdf", PdfIoFault::Sync),
        ("persist.pdf", PdfIoFault::Persist),
    ] {
        let destination = directory.path().join(name);
        let error =
            write_pdf_report_with_fault(&report, &destination, PdfReportLimits::default(), fault)
                .unwrap_err();
        assert!(error.contains("Injektovaná testovacia chyba"));
        assert!(!destination.exists());
    }
    assert_eq!(
        std::fs::read_dir(directory.path()).unwrap().count(),
        0,
        "failed stages clean only their owned temporary files"
    );
}

struct FailingWriter {
    accepted: usize,
}

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.accepted == 0 {
            return Err(io::Error::other("injected late write failure"));
        }
        let count = self.accepted.min(bytes.len());
        self.accepted -= count;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn renderer_prints_original_currency_but_never_adds_it_to_eur_totals() {
    let mut row = transaction(1, "foreign", "", -923);
    row.transaction.orig_amount_cents = Some(-1_000);
    row.transaction.orig_currency = Some("USD".into());
    row.category_kind = Some(CategoryKind::Expense);
    let (_directory, _path, pages) = write_and_extract(&snapshot(vec![row]), "foreign.pdf");
    let text = extracted_text(&pages);
    assert!(text.contains("Pôvodná suma: -10.00 USD"));
    assert!(text.contains("-9,23 €"));
}
