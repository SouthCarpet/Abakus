//! Summary PDFs (plan 084): literal text oracle and page-count bounds.
//! The numbers are the August 2026 profile of docs/plan-084-summary-pdf.md.
use abakus_lib::report::{
    render_summary_pdf_to, write_summary_pdf_report, PdfReportLimits, ReportDocumentScope,
    SummaryLength,
};
use chrono::NaiveDate;
use parser::AccountKind;
use rules::Status;
use store::recurring::{
    AmountBasis, Cadence, RecurringAmounts, RecurringDirection, RecurringExcluded, RecurringRow,
    RecurringScope, RecurringState, RowDecision,
};
use store::report::{
    ReportAccount, ReportAccountTotal, ReportCategoryDelta, ReportCategoryTotal, ReportComparison,
    ReportCoverage, ReportDateRange, ReportDay, ReportDelta, ReportMonth, ReportMonthRow,
    ReportPreview, ReportPrevious, ReportRecurring, ReportSnapshot, ReportStatusCounts,
    ReportTotals, ReportTransaction, SummaryExtras, SummarySnapshot,
};
use store::TxRow;

fn day(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

fn range(from: &str, to: &str) -> ReportDateRange {
    ReportDateRange {
        from: day(from),
        to: day(to),
    }
}

fn row(
    id: i64,
    date: &str,
    cents: i64,
    merchant: &str,
    category: Option<&str>,
) -> ReportTransaction {
    ReportTransaction {
        transaction: TxRow {
            id,
            account_id: 1,
            account_kind: AccountKind::Personal,
            statement_number: 1,
            posted_date: day(date),
            tx_date: day(date),
            kind: "card".into(),
            amount_cents: cents,
            orig_amount_cents: None,
            orig_currency: None,
            merchant_raw: merchant.into(),
            place: None,
            counterparty_name: None,
            counterparty_iban: None,
            category_id: None,
            category_name: category.map(str::to_owned),
            parent_name: None,
            status: Status::Confirmed,
            source: "none".into(),
            raw_block: "must never appear in PDF".into(),
            note: String::new(),
        },
        account_label: "Osobný".into(),
        category_kind: None,
    }
}

fn delta(current: i64, previous: i64, percent: Option<i64>) -> ReportDelta {
    ReportDelta {
        current_cents: current,
        previous_cents: previous,
        delta_cents: current - previous,
        percent_tenths: percent,
    }
}

fn category_delta(
    id: Option<i64>,
    label: &str,
    current: i64,
    previous: i64,
    percent: Option<i64>,
) -> ReportCategoryDelta {
    ReportCategoryDelta {
        category_id: id,
        label: label.into(),
        delta: delta(current, previous, percent),
    }
}

fn category(id: Option<i64>, label: &str, cents: i64) -> ReportCategoryTotal {
    ReportCategoryTotal {
        category_id: id,
        label: label.into(),
        cents,
    }
}

fn account(id: i64, label: &str, kind: AccountKind, suffix: &str) -> ReportAccount {
    ReportAccount {
        id,
        label: label.into(),
        kind,
        iban_suffix: suffix.into(),
    }
}

fn recurring_row(name: &str, decision: RowDecision, monthly: i64) -> RecurringRow {
    RecurringRow {
        series_key: format!("g:{name}"),
        group_key: name.into(),
        decision_id: None,
        scope: RecurringScope::Group,
        decision,
        account_id: 1,
        account_label: "Osobný".into(),
        account_kind: AccountKind::Personal,
        direction: RecurringDirection::Expense,
        name: name.into(),
        cadence: Some(Cadence::Monthly),
        anchor_date: Some(day("2026-06-10")),
        subscription_category: false,
        category_id: None,
        evidence_count: 3,
        last_paid: Some(day("2026-08-10")),
        next_due: Some(day("2026-09-10")),
        state: RecurringState::Upcoming,
        unknown_reason: None,
        grace_until: None,
        currency: None,
        original_amount_cents: None,
        amount_cents: Some(monthly),
        amount_basis: Some(AmountBasis::StablePair),
        monthly_cents: Some(monthly),
        annual_cents: Some(monthly * 12),
        price_change: None,
        manual_membership: false,
        foreign_eur_estimate: false,
    }
}

fn amounts(income: i64, expense: i64) -> RecurringAmounts {
    RecurringAmounts {
        monthly_income_cents: income,
        monthly_expense_cents: expense,
        monthly_net_cents: income - expense,
        ..RecurringAmounts::default()
    }
}

fn coverage(account_id: i64) -> ReportCoverage {
    ReportCoverage {
        account_id,
        known_range: Some(range("2026-08-01", "2026-08-31")),
        gaps: Vec::new(),
        statement_count: 1,
        unverified_statement_count: 0,
        invalid_range_count: 0,
    }
}

/// The August 2026 profile with the literal numbers of the contract.
fn august() -> SummarySnapshot {
    let largest = vec![
        row(1, "2026-08-02", 250_000, "Zamestnávateľ", Some("Plat")),
        row(17, "2026-08-04", 100_000, "Faktúra", None),
        row(4, "2026-08-07", -60_000, "Prenájom", Some("Bývanie")),
        row(
            18,
            "2026-08-10",
            -20_000,
            "Nájom kancelárie",
            Some("Bývanie"),
        ),
        row(2, "2026-08-03", -12_000, "Tesco", Some("Potraviny")),
        row(9, "2026-08-16", -9_000, "Hotel", Some("Cestovanie")),
        row(3, "2026-08-05", -8_000, "Tesco", Some("Potraviny")),
        row(5, "2026-08-09", -5_000, "ZSSK", Some("Doprava")),
        row(16, "2026-08-31", -4_500, "Kino návrh", Some("Zábava")),
        row(6, "2026-08-11", -4_000, "Kino", Some("Zábava")),
    ];
    let mut days = (1..=31)
        .map(|number| ReportDay {
            date: day(&format!("2026-08-{number:02}")),
            income_cents: 0,
            expense_cents: 0,
        })
        .collect::<Vec<_>>();
    days[0].expense_cents = 500;
    days[1].income_cents = 250_000;
    days[3].income_cents = 100_000;
    days[26].expense_cents = -2_000;
    days[30].expense_cents = 4_500;
    SummarySnapshot {
        snapshot: ReportSnapshot {
            preview: ReportPreview {
                range: Some(range("2026-08-01", "2026-08-31")),
                scope_label: "Všetky účty".into(),
                accounts: vec![
                    account(1, "Osobný", AccountKind::Personal, "5678"),
                    account(2, "Firemný", AccountKind::Business, "5432"),
                ],
                transaction_count: 19,
                latest_transaction_date: Some(day("2026-08-31")),
                captured_at: "2026-09-07T14:30:00+02:00".into(),
                unfinished_period: false,
                accounts_without_statements: 0,
                accounts_with_gaps: 0,
                unverified_statement_count: 0,
                invalid_statement_range_count: 0,
            },
            totals: ReportTotals {
                income_cents: 350_000,
                expense_cents: 132_500,
                net_cents: 217_500,
                transfer_out_cents: 30_000,
                transfer_in_cents: 30_000,
                suggested_count: 1,
                unassigned_count: 1,
            },
            months: vec![ReportMonth {
                month: "2026-08".into(),
                income_cents: 350_000,
                expense_cents: 132_500,
            }],
            categories: vec![
                category(Some(902), "Bývanie", 80_000),
                category(Some(901), "Potraviny", 18_000),
                category(Some(907), "Cestovanie", 9_000),
                category(Some(904), "Zábava", 8_500),
                category(Some(903), "Doprava", 5_000),
                category(None, "Nezaradené", 3_500),
                category(Some(905), "Zdravie", 3_000),
                category(Some(906), "Oblečenie", 2_500),
                category(Some(908), "Vzdelanie", 1_500),
                category(Some(909), "Darčeky", 1_000),
                category(Some(910), "Poistenie", 500),
            ],
            coverage: vec![coverage(1), coverage(2)],
            transactions: Vec::new(),
        },
        extras: SummaryExtras {
            days,
            month_rows: vec![ReportMonthRow {
                month: "2026-08".into(),
                income_cents: 350_000,
                expense_cents: 132_500,
                net_cents: 217_500,
                cumulative_net_cents: 217_500,
            }],
            account_totals: vec![
                ReportAccountTotal {
                    account_id: 1,
                    label: "Osobný".into(),
                    income_cents: 250_000,
                    expense_cents: 112_500,
                    net_cents: 137_500,
                    transaction_count: 16,
                },
                ReportAccountTotal {
                    account_id: 2,
                    label: "Firemný".into(),
                    income_cents: 100_000,
                    expense_cents: 20_000,
                    net_cents: 80_000,
                    transaction_count: 3,
                },
            ],
            status_counts: ReportStatusCounts {
                confirmed: 15,
                suggested: 1,
                unassigned: 1,
                transfer: 2,
            },
            largest,
            comparison: ReportComparison::Ready(Box::new(ReportPrevious {
                range: range("2026-07-01", "2026-07-31"),
                income: delta(350_000, 240_000, Some(458)),
                expense: delta(132_500, 93_500, Some(417)),
                net: delta(217_500, 146_500, Some(485)),
                categories: vec![
                    category_delta(Some(902), "Bývanie", 80_000, 60_000, Some(333)),
                    category_delta(Some(901), "Potraviny", 18_000, 10_000, Some(800)),
                    category_delta(Some(904), "Zábava", 8_500, 3_000, Some(1_833)),
                    category_delta(Some(905), "Zdravie", 3_000, 0, None),
                    category_delta(Some(907), "Cestovanie", 9_000, 12_000, Some(-250)),
                    category_delta(Some(903), "Doprava", 5_000, 7_000, Some(-286)),
                ],
                accounts_without_statements: 1,
                accounts_with_gaps: 1,
            })),
            recurring: ReportRecurring {
                as_of: day("2026-08-31"),
                confirmed: amounts(0, 0),
                estimates: amounts(0, 1_299),
                excluded: RecurringExcluded::default(),
                rows: vec![recurring_row("NETFLIX.COM", RowDecision::Estimate, 1_299)],
            },
        },
    }
}

fn render(summary: &SummarySnapshot, length: SummaryLength, name: &str) -> Vec<Vec<String>> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(name);
    let outcome = write_summary_pdf_report(summary, length, &path).unwrap();
    let pages = parser::extract_pages(&path, None).unwrap();
    assert_eq!(usize::try_from(outcome.pages).unwrap(), pages.len());
    if let Some(output) = std::env::var_os("ABAKUS_PDF_TEST_OUTPUT") {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir_all(&output).unwrap();
        std::fs::copy(&path, output.join(name)).unwrap();
    }
    pages
}

fn page_text(page: &[String]) -> String {
    page.join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn assert_contains(text: &str, literals: &[&str]) {
    for literal in literals {
        assert!(text.contains(literal), "missing {literal:?} in:\n{text}");
    }
}

#[test]
fn d01_scope_names_and_default_are_the_wire_contract() {
    assert_eq!(ReportDocumentScope::default(), ReportDocumentScope::Full);
    assert_eq!(
        serde_json::to_value([
            ReportDocumentScope::Full,
            ReportDocumentScope::SummaryOnePage,
            ReportDocumentScope::SummaryThreePages
        ])
        .unwrap(),
        serde_json::json!(["full", "summary_one_page", "summary_three_pages"])
    );
    assert_eq!(ReportDocumentScope::Full.summary_length(), None);
    assert_eq!(
        ReportDocumentScope::SummaryOnePage.summary_length(),
        Some(SummaryLength::OnePage)
    );
    assert_eq!(
        ReportDocumentScope::SummaryThreePages.summary_length(),
        Some(SummaryLength::ThreePages)
    );
}

#[test]
fn d02_one_page_summary_prints_header_kpis_chart_categories_and_coverage() {
    let pages = render(&august(), SummaryLength::OnePage, "one.pdf");
    assert_eq!(pages.len(), 1);
    let text = page_text(&pages[0]);
    assert_contains(
        &text,
        &[
            "Abakus · Sumár na 1 stranu",
            "Obdobie 01.08.2026 až 31.08.2026 · Všetky účty · najnovšia transakcia 31.08.2026",
            "Údaje zachytené 2026-09-07T14:30:00+02:00",
            "Účty (2): Osobný •••• 5678, Firemný •••• 5432",
            "Príjmy",
            "3 500,00 €",
            "Výdavky",
            "1 325,00 €",
            "Rozdiel príjmov a výdavkov",
            "2 175,00 €",
            "Odchádzajúce prevody 300,00 €",
            "Prichádzajúce prevody 300,00 €",
            "Príjmy a výdavky po dňoch",
            "Výdavky podľa kategórie",
            "Bývanie",
            "800,00 €",
            "60,4 %",
            "Potraviny",
            "180,00 €",
            "13,6 %",
            "Oblečenie",
            "25,00 €",
            "1,9 %",
            "Ostatné (3 kategórií)",
            "30,00 €",
            "2,3 %",
            "Spolu výdavky 1 325,00 €",
            "Výpisy v období: 2",
            "Medzery v pokrytí: 0",
            "Transakcie: 19 · potvrdené 15 · navrhnuté 1 · nezaradené 1 · prevody 2",
            "Súčty vychádzajú len zo zaznamenaných transakcií.",
            "Strana 1 z 1",
        ],
    );
    assert!(!text.contains("Vzdelanie"), "only the top 8 are listed");
    assert!(!text.contains("Obdobie ešte neskončilo"));
    assert!(!text.contains("Porovnanie"));
}

#[test]
fn d03_three_page_summary_has_the_comparison_and_the_detail_pages() {
    let pages = render(&august(), SummaryLength::ThreePages, "three.pdf");
    assert_eq!(pages.len(), 3);
    let first = page_text(&pages[0]);
    assert_contains(
        &first,
        &["Abakus · Sumár do 3 strán", "3 500,00 €", "Strana 1 z 3"],
    );
    let second = page_text(&pages[1]);
    assert_contains(
        &second,
        &[
            "Porovnanie s predchádzajúcim obdobím",
            "Predchádzajúce obdobie 01.07.2026 až 31.07.2026 (rovnaký počet dní).",
            "Predchádzajúce obdobie nie je celé pokryté výpismi. Účty bez výpisov: 1. Účty s medzerami: 1.",
            "2 400,00 €",
            "+1 100,00 €",
            "+45,8 %",
            "935,00 €",
            "+390,00 €",
            "+41,7 %",
            "1 465,00 €",
            "+710,00 €",
            "+48,5 %",
            "Rozdiel výdavkov po kategóriách",
            "+200,00 €",
            "+33,3 %",
            "+55,00 €",
            "+183,3 %",
            "-30,00 €",
            "-25,0 %",
            "-28,6 %",
            "nedostupné",
            "Všetky kategórie výdavkov",
            "Vzdelanie",
            "15,00 €",
            "Strana 2 z 3",
        ],
    );
    let third = page_text(&pages[2]);
    assert_contains(
        &third,
        &[
            "Súčty po účtoch",
            "Firemný",
            "800,00 €",
            "Vývoj po mesiacoch",
            "2026-08",
            "Kumulatívne",
            "2 175,00 €",
            "10 najväčších transakcií",
            "Zamestnávateľ",
            "2 500,00 €",
            "Faktúra",
            "Nájom kancelárie",
            "-600,00 €",
            "Kino",
            "-40,00 €",
            "Pravidelné platby (odhady)",
            "Odhady z importov.",
            "NETFLIX.COM",
            "12,99 €",
            "10.09.2026",
            "Strana 3 z 3",
        ],
    );
    assert!(
        !third.contains("Prevod von"),
        "the transfer is not among the largest"
    );
}

#[test]
fn d04_all_time_has_no_comparison_and_says_so() {
    let mut summary = august();
    summary.extras.comparison = ReportComparison::AllTime;
    summary.snapshot.preview.range = Some(range("2026-06-30", "2026-09-01"));
    let pages = render(&summary, SummaryLength::ThreePages, "all-time.pdf");
    assert_eq!(pages.len(), 3);
    let second = page_text(&pages[1]);
    assert_contains(
        &second,
        &[
            "Celé obdobie nemá porovnanie s predchádzajúcim obdobím.",
            "Všetky kategórie výdavkov",
        ],
    );
    assert!(!second.contains("Predchádzajúce obdobie 0"));
    assert!(!second.contains("Rozdiel výdavkov po kategóriách"));
}

#[test]
fn d05_out_of_range_and_unfinished_period_are_stated() {
    let mut summary = august();
    summary.extras.comparison = ReportComparison::OutOfRange;
    summary.snapshot.preview.unfinished_period = true;
    summary.snapshot.preview.accounts_with_gaps = 1;
    let pages = render(&summary, SummaryLength::ThreePages, "range.pdf");
    let first = page_text(&pages[0]);
    assert_contains(
        &first,
        &[
            "Obdobie ešte neskončilo. Údaje sa môžu zmeniť.",
            "Pokrytie výpismi je neúplné.",
        ],
    );
    let second = page_text(&pages[1]);
    assert_contains(
        &second,
        &["Predchádzajúce obdobie by začínalo pred rokom 0001. Porovnanie nie je dostupné."],
    );
    let mut unfinished = august();
    unfinished.snapshot.preview.unfinished_period = true;
    let pages = render(&unfinished, SummaryLength::ThreePages, "unfinished.pdf");
    assert_contains(
        &page_text(&pages[1]),
        &["Obdobie ešte neskončilo. Porovnanie je predbežné."],
    );
}

#[test]
fn d06_empty_profile_still_renders_the_fixed_pages_without_invalid_numbers() {
    let mut summary = august();
    summary.snapshot.preview.range = None;
    summary.snapshot.preview.accounts = Vec::new();
    summary.snapshot.preview.transaction_count = 0;
    summary.snapshot.preview.latest_transaction_date = None;
    summary.snapshot.totals = ReportTotals {
        income_cents: 0,
        expense_cents: 0,
        net_cents: 0,
        transfer_out_cents: 0,
        transfer_in_cents: 0,
        suggested_count: 0,
        unassigned_count: 0,
    };
    summary.snapshot.months = Vec::new();
    summary.snapshot.categories = Vec::new();
    summary.snapshot.coverage = Vec::new();
    summary.extras.days = Vec::new();
    summary.extras.month_rows = Vec::new();
    summary.extras.account_totals = Vec::new();
    summary.extras.status_counts = ReportStatusCounts {
        confirmed: 0,
        suggested: 0,
        unassigned: 0,
        transfer: 0,
    };
    summary.extras.largest = Vec::new();
    summary.extras.comparison = ReportComparison::AllTime;
    summary.extras.recurring.rows = Vec::new();
    for length in [SummaryLength::OnePage, SummaryLength::ThreePages] {
        let pages = render(&summary, length, "empty.pdf");
        let text = pages
            .iter()
            .map(|page| page_text(page))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!text.contains("NaN"));
        assert!(!text.contains("Infinity"));
        assert_contains(&text, &["Bez transakcií", "Bez výdavkových kategórií."]);
    }
}

fn busy(period: &str) -> SummarySnapshot {
    let mut summary = august();
    let long = "Veľmi dlhý názov účtu s emoji 😀 a ďalšími slovami ".repeat(8);
    summary.snapshot.preview.accounts = (1..=14)
        .map(|id| account(id, &format!("{long}{id}"), AccountKind::Personal, "9999"))
        .collect();
    summary.snapshot.preview.unfinished_period = true;
    summary.snapshot.preview.accounts_without_statements = 4;
    summary.snapshot.preview.accounts_with_gaps = 5;
    summary.snapshot.preview.unverified_statement_count = 6;
    summary.snapshot.preview.invalid_statement_range_count = 7;
    summary.snapshot.categories = (1..=150)
        .map(|id| category(Some(id), &format!("{long} {id}"), 1_000 + id * 7))
        .collect();
    summary.snapshot.totals.expense_cents =
        summary.snapshot.categories.iter().map(|c| c.cents).sum();
    let months: usize = match period {
        "month" => 1,
        "six" => 6,
        "year" => 12,
        _ => 80,
    };
    summary.snapshot.months = (0..months)
        .map(|index| ReportMonth {
            month: format!("{}-{:02}", 2020 + index / 12, index % 12 + 1),
            income_cents: 100_000 + index as i64,
            expense_cents: 50_000 + index as i64,
        })
        .collect();
    if period != "month" {
        summary.extras.days = Vec::new();
    }
    summary.extras.month_rows = summary
        .snapshot
        .months
        .iter()
        .scan(0_i64, |running, month| {
            let net = month.income_cents - month.expense_cents;
            *running += net;
            Some(ReportMonthRow {
                month: month.month.clone(),
                income_cents: month.income_cents,
                expense_cents: month.expense_cents,
                net_cents: net,
                cumulative_net_cents: *running,
            })
        })
        .collect();
    summary.extras.account_totals = (1..=14)
        .map(|id| ReportAccountTotal {
            account_id: id,
            label: format!("{long}{id}"),
            income_cents: 1_000_000_000_000,
            expense_cents: 900_000_000_000,
            net_cents: 100_000_000_000,
            transaction_count: 12_345,
        })
        .collect();
    summary.extras.largest = (1..=10)
        .map(|id| {
            row(
                id,
                "2026-08-15",
                -(i64::MAX / 4),
                &format!("{long} 😀 {id}"),
                Some(&long),
            )
        })
        .collect();
    summary.extras.comparison = ReportComparison::Ready(Box::new(ReportPrevious {
        range: range("2026-07-01", "2026-07-31"),
        income: delta(1, i64::MAX / 8, Some(-1_000)),
        expense: delta(i64::MAX / 8, 1, None),
        net: delta(-5, 5, Some(-2_000)),
        categories: (1..=60)
            .map(|id| category_delta(Some(id), &format!("{long} {id}"), id * 1_000, id, Some(id)))
            .collect(),
        accounts_without_statements: 14,
        accounts_with_gaps: 14,
    }));
    summary.extras.recurring.rows = (1..=30)
        .map(|id| recurring_row(&format!("{long} {id}"), RowDecision::Confirmed, 4_000 + id))
        .collect();
    summary.extras.recurring.confirmed = amounts(i64::MAX / 4, i64::MAX / 4);
    summary
}

#[test]
fn d07_one_page_summary_is_exactly_one_page_for_all_four_periods_on_a_busy_profile() {
    for period in ["month", "six", "year", "all"] {
        let pages = render(
            &busy(period),
            SummaryLength::OnePage,
            &format!("busy-one-{period}.pdf"),
        );
        assert_eq!(pages.len(), 1, "period {period}");
    }
}

#[test]
fn d08_three_page_summary_is_at_most_three_pages_for_all_four_periods_on_a_busy_profile() {
    for period in ["month", "six", "year", "all"] {
        let pages = render(
            &busy(period),
            SummaryLength::ThreePages,
            &format!("busy-three-{period}.pdf"),
        );
        assert!(pages.len() <= 3, "period {period}: {} pages", pages.len());
        assert_eq!(pages.len(), 3, "period {period}");
        for (index, page) in pages.iter().enumerate() {
            assert!(
                page_text(page).contains(&format!("Strana {} z 3", index + 1)),
                "footer of page {}",
                index + 1
            );
        }
    }
}

#[test]
fn d09_capped_tables_say_how_many_rows_are_left_out() {
    let pages = render(&busy("all"), SummaryLength::ThreePages, "caps.pdf");
    let first = page_text(&pages[0]);
    assert_contains(&first, &["Účty (14)", "a ďalších 11 účtov"]);
    let second = page_text(&pages[1]);
    assert_contains(
        &second,
        &["a ďalších 48 kategórií", "a ďalších 128 kategórií"],
    );
    let third = page_text(&pages[2]);
    assert_contains(
        &third,
        &[
            "a ďalších 8 účtov",
            "a ďalších 68 skorších mesiacov",
            "a ďalších 24",
        ],
    );
    assert!(
        !first.contains("must never appear in PDF"),
        "raw blocks never reach a summary"
    );
}

#[test]
fn d10_a_page_limit_below_the_fixed_page_count_is_an_error_and_leaves_no_file() {
    let summary = august();
    let mut sink = Vec::new();
    let limits = PdfReportLimits {
        max_pages: 2,
        max_bytes: u64::MAX,
    };
    assert!(render_summary_pdf_to(&summary, SummaryLength::ThreePages, limits, &mut sink).is_err());
    let pages = render_summary_pdf_to(&summary, SummaryLength::OnePage, limits, &mut sink).unwrap();
    assert_eq!(pages, 1);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("taken.pdf");
    std::fs::write(&path, b"existing").unwrap();
    assert!(write_summary_pdf_report(&summary, SummaryLength::OnePage, &path).is_err());
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"existing",
        "no-clobber publish"
    );
}
