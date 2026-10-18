//! Literal oracle for the summary PDF data (plan 084, docs/plan-084-summary-pdf.md).
//! Every expected number below is worked out by hand in that document.
use chrono::{FixedOffset, NaiveDate, TimeZone};
use parser::AccountKind;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use store::report::{
    previous_range, ratio_tenths, ReportCaptureLimits, ReportClock, ReportComparison,
    ReportDateRange, ReportPeriod, ReportPrevious, ReportRequest, ReportScope, SummarySnapshot,
};
use store::Store;

const PERSONAL_IBAN: &str = "SK4411000000000012345678";
const BUSINESS_IBAN: &str = "SK3711000000000098765432";

fn day(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

fn clock(today: &str) -> ReportClock {
    let offset = FixedOffset::east_opt(2 * 3600).unwrap();
    ReportClock {
        today: day(today),
        captured_at: offset.with_ymd_and_hms(2026, 9, 7, 14, 30, 0).unwrap(),
    }
}

fn month(value: &str) -> ReportRequest {
    ReportRequest {
        period: ReportPeriod::Month {
            month: value.into(),
        },
        scope: ReportScope::All,
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("summary.db");
        Store::open(&path).unwrap();
        let fixture = Self {
            _directory: directory,
            path,
        };
        // The synthetic profile owns its categories, not the seeded ones.
        fixture
            .connection()
            .execute_batch("DELETE FROM rules; DELETE FROM categories;")
            .unwrap();
        fixture
    }

    fn connection(&self) -> Connection {
        let connection = Connection::open(&self.path).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        connection
    }

    fn store(&self) -> Store {
        Store::open(&self.path).unwrap()
    }

    fn account(&self, id: i64, iban: &str, kind: &str, label: &str) {
        self.connection()
            .execute(
                "INSERT INTO accounts(id,iban,kind,label) VALUES(?1,?2,?3,?4)",
                params![id, iban, kind, label],
            )
            .unwrap();
    }

    fn statement(&self, id: i64, account_id: i64, from: &str, to: &str) {
        self.connection()
            .execute(
                "INSERT INTO statements(id,account_id,number,period_start,period_end,checksum_status,file_hash) VALUES(?1,?2,?1,?3,?4,'ok',?5)",
                params![id, account_id, from, to, format!("statement-{id}")],
            )
            .unwrap();
    }

    fn category(&self, id: i64, name: &str, kind: &str) {
        self.connection()
            .execute(
                "INSERT INTO categories(id,name,kind,sort) VALUES(?1,?2,?3,?1)",
                params![id, name, kind],
            )
            .unwrap();
    }

    fn tx(&self, seed: Tx) {
        self.connection().execute(
            "INSERT INTO transactions(id,statement_id,account_id,fingerprint,posted_date,tx_date,kind,amount_cents,merchant_raw,merchant_norm,raw_block,status,source,category_id,note)              VALUES(?1,?2,?2,?3,?4,?4,?5,?6,?7,?7,'raw',?8,'none',?9,'')",
            params![seed.id, seed.account, format!("tx-{}", seed.id), seed.date, seed.kind, seed.cents, seed.merchant, seed.status, seed.category],
        ).unwrap();
    }
}

struct Tx {
    id: i64,
    account: i64,
    date: &'static str,
    kind: &'static str,
    cents: i64,
    status: &'static str,
    category: Option<i64>,
    merchant: &'static str,
}

fn tx(
    id: i64,
    account: i64,
    date: &'static str,
    cents: i64,
    category: Option<i64>,
    merchant: &'static str,
) -> Tx {
    Tx {
        id,
        account,
        date,
        kind: "card",
        cents,
        status: "confirmed",
        category,
        merchant,
    }
}

/// The synthetic profile of the contract document, section "Syntetický profil".
fn busy_profile() -> Fixture {
    let fixture = Fixture::new();
    fixture.account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.account(2, BUSINESS_IBAN, "business", "Firemný");
    fixture.statement(1, 1, "2026-06-01", "2026-08-31");
    fixture.statement(2, 2, "2026-08-01", "2026-08-31");
    for (id, name, kind) in [
        (901, "Potraviny", "expense"),
        (902, "Bývanie", "expense"),
        (903, "Doprava", "expense"),
        (904, "Zábava", "expense"),
        (905, "Zdravie", "expense"),
        (906, "Oblečenie", "expense"),
        (907, "Cestovanie", "expense"),
        (908, "Vzdelanie", "expense"),
        (909, "Darčeky", "expense"),
        (910, "Poistenie", "expense"),
        (911, "Plat", "income"),
    ] {
        fixture.category(id, name, kind);
    }
    for row in august_rows().into_iter().chain(earlier_rows()) {
        fixture.tx(row);
    }
    fixture
}

fn august_rows() -> Vec<Tx> {
    let mut refund = tx(15, 1, "2026-08-27", 2_000, Some(901), "Tesco vrátenie");
    refund.kind = "refund";
    let mut unassigned = tx(13, 1, "2026-08-25", -3_500, None, "Neznámy");
    unassigned.status = "unassigned";
    let mut transfer_out = tx(14, 1, "2026-08-26", -30_000, None, "Prevod von");
    transfer_out.status = "transfer";
    let mut suggested = tx(16, 1, "2026-08-31", -4_500, Some(904), "Kino návrh");
    suggested.status = "suggested";
    let mut transfer_in = tx(19, 2, "2026-08-29", 30_000, None, "Prevod dnu");
    transfer_in.status = "transfer";
    vec![
        tx(1, 1, "2026-08-02", 250_000, Some(911), "Zamestnávateľ"),
        tx(2, 1, "2026-08-03", -12_000, Some(901), "Tesco"),
        tx(3, 1, "2026-08-05", -8_000, Some(901), "Tesco"),
        tx(4, 1, "2026-08-07", -60_000, Some(902), "Prenájom"),
        tx(5, 1, "2026-08-09", -5_000, Some(903), "ZSSK"),
        tx(6, 1, "2026-08-11", -4_000, Some(904), "Kino"),
        tx(7, 1, "2026-08-12", -3_000, Some(905), "Lekáreň"),
        tx(8, 1, "2026-08-14", -2_500, Some(906), "Textil"),
        tx(9, 1, "2026-08-16", -9_000, Some(907), "Hotel"),
        tx(10, 1, "2026-08-18", -1_500, Some(908), "Kurz"),
        tx(11, 1, "2026-08-20", -1_000, Some(909), "Darček"),
        tx(12, 1, "2026-08-01", -500, Some(910), "Poisťovňa"),
        unassigned,
        transfer_out,
        refund,
        suggested,
        tx(17, 2, "2026-08-04", 100_000, None, "Faktúra"),
        tx(18, 2, "2026-08-10", -20_000, Some(902), "Nájom kancelárie"),
        transfer_in,
    ]
}

fn earlier_rows() -> Vec<Tx> {
    let mut unassigned = tx(25, 1, "2026-07-24", -1_000, None, "Neznámy");
    unassigned.status = "unassigned";
    let mut transfer = tx(28, 1, "2026-07-28", -5_000, None, "Prevod");
    transfer.status = "transfer";
    vec![
        tx(20, 1, "2026-07-02", 240_000, Some(911), "Zamestnávateľ"),
        tx(21, 1, "2026-07-05", -10_000, Some(901), "Tesco"),
        tx(22, 1, "2026-07-07", -60_000, Some(902), "Prenájom"),
        tx(23, 1, "2026-07-15", -7_000, Some(903), "ZSSK"),
        tx(24, 1, "2026-07-20", -3_000, Some(904), "Kino"),
        unassigned,
        tx(26, 1, "2026-07-31", -12_000, Some(907), "Hotel"),
        tx(27, 1, "2026-07-01", -500, Some(910), "Poisťovňa"),
        transfer,
        tx(29, 1, "2026-06-30", -999, Some(901), "Tesco"),
        tx(30, 1, "2026-09-01", -111, Some(903), "ZSSK"),
    ]
}

fn summary(fixture: &Fixture, request: &ReportRequest, today: &str) -> SummarySnapshot {
    fixture
        .store()
        .report_summary_snapshot(request, &clock(today))
        .unwrap()
}

fn ready(summary: &SummarySnapshot) -> &ReportPrevious {
    match &summary.extras.comparison {
        ReportComparison::Ready(previous) => previous,
        other => panic!("comparison must be ready, got {other:?}"),
    }
}

fn delta_tuple(delta: store::report::ReportDelta) -> (i64, i64, i64, Option<i64>) {
    (
        delta.current_cents,
        delta.previous_cents,
        delta.delta_cents,
        delta.percent_tenths,
    )
}

#[test]
fn s01_ratio_tenths_rounds_half_away_from_zero_and_never_divides_by_zero() {
    assert_eq!(ratio_tenths(110_000, 240_000), Some(458));
    assert_eq!(ratio_tenths(-3_000, 12_000), Some(-250));
    assert_eq!(ratio_tenths(1, 3), Some(333));
    assert_eq!(ratio_tenths(5, 2_000), Some(3));
    assert_eq!(ratio_tenths(-5, 2_000), Some(-3));
    assert_eq!(ratio_tenths(-131_889, 132_000), Some(-999));
    assert_eq!(ratio_tenths(-218_111, 218_000), Some(-1001));
    assert_eq!(
        ratio_tenths(7, -2),
        Some(3500),
        "denominator is taken as abs"
    );
    assert_eq!(ratio_tenths(7, 0), None);
    assert_eq!(ratio_tenths(0, 5), Some(0));
}

#[test]
fn s02_previous_range_has_the_same_day_count_and_ends_the_day_before() {
    let range = |from: &str, to: &str| ReportDateRange {
        from: day(from),
        to: day(to),
    };
    assert_eq!(
        previous_range(range("2026-08-01", "2026-08-31")),
        Some(range("2026-07-01", "2026-07-31"))
    );
    assert_eq!(
        previous_range(range("2026-09-01", "2026-09-30")),
        Some(range("2026-08-02", "2026-08-31"))
    );
    assert_eq!(
        previous_range(range("2026-02-01", "2026-02-28")),
        Some(range("2026-01-04", "2026-01-31"))
    );
    assert_eq!(
        previous_range(range("2026-03-01", "2026-08-31")),
        Some(range("2025-08-29", "2026-02-28"))
    );
    assert_eq!(
        previous_range(range("2024-01-01", "2024-12-31")),
        Some(range("2022-12-31", "2023-12-31"))
    );
    assert_eq!(
        previous_range(range("2026-06-15", "2026-06-15")),
        Some(range("2026-06-14", "2026-06-14"))
    );
    assert_eq!(previous_range(range("0001-01-01", "0001-01-31")), None);
    assert_eq!(previous_range(range("0001-01-10", "0001-01-20")), None);
}

#[test]
fn s03_month_comparison_kpis_in_cents_and_tenths_of_percent() {
    let profile = busy_profile();
    let report = summary(&profile, &month("2026-08"), "2026-09-07");
    assert_eq!(
        (
            report.snapshot.totals.income_cents,
            report.snapshot.totals.expense_cents,
            report.snapshot.totals.net_cents
        ),
        (350_000, 132_500, 217_500)
    );
    let previous = ready(&report);
    assert_eq!(previous.range.from, day("2026-07-01"));
    assert_eq!(previous.range.to, day("2026-07-31"));
    assert_eq!(
        delta_tuple(previous.income),
        (350_000, 240_000, 110_000, Some(458))
    );
    assert_eq!(
        delta_tuple(previous.expense),
        (132_500, 93_500, 39_000, Some(417))
    );
    assert_eq!(
        delta_tuple(previous.net),
        (217_500, 146_500, 71_000, Some(485))
    );
}

#[test]
fn s04_category_comparison_unions_both_periods_and_orders_by_absolute_delta() {
    let profile = busy_profile();
    let report = summary(&profile, &month("2026-08"), "2026-09-07");
    let rows = ready(&report)
        .categories
        .iter()
        .map(|row| {
            (
                row.label.as_str(),
                row.category_id,
                row.delta.current_cents,
                row.delta.previous_cents,
                row.delta.delta_cents,
                row.delta.percent_tenths,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        vec![
            ("Bývanie", Some(902), 80_000, 60_000, 20_000, Some(333)),
            ("Potraviny", Some(901), 18_000, 10_000, 8_000, Some(800)),
            ("Zábava", Some(904), 8_500, 3_000, 5_500, Some(1_833)),
            ("Zdravie", Some(905), 3_000, 0, 3_000, None),
            ("Cestovanie", Some(907), 9_000, 12_000, -3_000, Some(-250)),
            ("Oblečenie", Some(906), 2_500, 0, 2_500, None),
            ("Nezaradené", None, 3_500, 1_000, 2_500, Some(2_500)),
            ("Doprava", Some(903), 5_000, 7_000, -2_000, Some(-286)),
            ("Vzdelanie", Some(908), 1_500, 0, 1_500, None),
            ("Darčeky", Some(909), 1_000, 0, 1_000, None),
            ("Poistenie", Some(910), 500, 500, 0, Some(0)),
        ]
    );
}

#[test]
fn s05_previous_period_coverage_is_counted_per_account() {
    let profile = busy_profile();
    let report = summary(&profile, &month("2026-08"), "2026-09-07");
    let previous = ready(&report);
    assert_eq!(
        (
            previous.accounts_without_statements,
            previous.accounts_with_gaps
        ),
        (1, 1),
        "the business account has no statement in July"
    );
    assert_eq!(report.snapshot.preview.accounts_without_statements, 0);
}

#[test]
fn s06_account_totals_status_counts_and_ten_largest_transactions() {
    let profile = busy_profile();
    let report = summary(&profile, &month("2026-08"), "2026-09-07");
    let accounts = report
        .extras
        .account_totals
        .iter()
        .map(|item| {
            (
                item.account_id,
                item.label.as_str(),
                item.income_cents,
                item.expense_cents,
                item.net_cents,
                item.transaction_count,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        accounts,
        vec![
            (1, "Osobný", 250_000, 112_500, 137_500, 16),
            (2, "Firemný", 100_000, 20_000, 80_000, 3),
        ]
    );
    let counts = report.extras.status_counts;
    assert_eq!(
        (
            counts.confirmed,
            counts.suggested,
            counts.unassigned,
            counts.transfer
        ),
        (15, 1, 1, 2)
    );
    let largest = report
        .extras
        .largest
        .iter()
        .map(|row| (row.transaction.id, row.transaction.amount_cents))
        .collect::<Vec<_>>();
    assert_eq!(
        largest,
        vec![
            (1, 250_000),
            (17, 100_000),
            (4, -60_000),
            (18, -20_000),
            (2, -12_000),
            (9, -9_000),
            (3, -8_000),
            (5, -5_000),
            (16, -4_500),
            (6, -4_000),
        ],
        "the 30 000 transfer is not listed, ranking is by absolute amount"
    );
}

#[test]
fn s07_month_has_one_entry_per_day_from_the_same_money_rule() {
    let profile = busy_profile();
    let report = summary(&profile, &month("2026-08"), "2026-09-07");
    let days = &report.extras.days;
    assert_eq!(days.len(), 31);
    assert_eq!(days[0].date, day("2026-08-01"));
    assert_eq!(days[30].date, day("2026-08-31"));
    let pair = |index: usize| (days[index].income_cents, days[index].expense_cents);
    assert_eq!(pair(0), (0, 500));
    assert_eq!(pair(1), (250_000, 0));
    assert_eq!(pair(25), (0, 0), "a transfer is neither income nor expense");
    assert_eq!(pair(26), (0, -2_000), "a refund reduces the expense");
    assert_eq!(pair(30), (0, 4_500));
    assert_eq!(
        days.iter().map(|item| item.income_cents).sum::<i64>(),
        350_000
    );
    assert_eq!(
        days.iter().map(|item| item.expense_cents).sum::<i64>(),
        132_500
    );
}

#[test]
fn s08_month_table_accumulates_net_from_the_first_month() {
    let profile = busy_profile();
    let request = ReportRequest {
        period: ReportPeriod::SixMonths {
            ending_month: "2026-08".into(),
        },
        scope: ReportScope::All,
    };
    let report = summary(&profile, &request, "2026-09-07");
    let rows = report
        .extras
        .month_rows
        .iter()
        .map(|row| {
            (
                row.month.as_str(),
                row.income_cents,
                row.expense_cents,
                row.net_cents,
                row.cumulative_net_cents,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        vec![
            ("2026-03", 0, 0, 0, 0),
            ("2026-04", 0, 0, 0, 0),
            ("2026-05", 0, 0, 0, 0),
            ("2026-06", 0, 999, -999, -999),
            ("2026-07", 240_000, 93_500, 146_500, 145_501),
            ("2026-08", 350_000, 132_500, 217_500, 363_001),
        ]
    );
    assert!(
        report.extras.days.is_empty(),
        "days exist only for one month"
    );
    let previous = ready(&report);
    assert_eq!(previous.range.from, day("2025-08-29"));
    assert_eq!(previous.range.to, day("2026-02-28"));
    assert_eq!(
        delta_tuple(previous.income),
        (590_000, 0, 590_000, None),
        "no percent against zero"
    );
    assert_eq!(
        (
            previous.accounts_without_statements,
            previous.accounts_with_gaps
        ),
        (2, 2)
    );
}

#[test]
fn s09_all_time_has_no_comparison() {
    let profile = busy_profile();
    let request = ReportRequest {
        period: ReportPeriod::AllTime,
        scope: ReportScope::All,
    };
    let report = summary(&profile, &request, "2026-09-07");
    assert_eq!(report.extras.comparison, ReportComparison::AllTime);
    assert!(report.extras.days.is_empty());
    let months = report
        .extras
        .month_rows
        .iter()
        .map(|row| (row.month.as_str(), row.cumulative_net_cents))
        .collect::<Vec<_>>();
    assert_eq!(
        months,
        vec![
            ("2026-06", -999),
            ("2026-07", 145_501),
            ("2026-08", 363_001),
            ("2026-09", 362_890),
        ]
    );
}

#[test]
fn s10_unfinished_month_compares_with_the_equal_length_range_before_it() {
    let profile = busy_profile();
    let report = summary(&profile, &month("2026-09"), "2026-09-07");
    assert!(report.snapshot.preview.unfinished_period);
    let previous = ready(&report);
    assert_eq!(previous.range.from, day("2026-08-02"));
    assert_eq!(previous.range.to, day("2026-08-31"));
    assert_eq!(
        delta_tuple(previous.income),
        (0, 350_000, -350_000, Some(-1_000))
    );
    assert_eq!(
        delta_tuple(previous.expense),
        (111, 132_000, -131_889, Some(-999))
    );
    assert_eq!(
        delta_tuple(previous.net),
        (-111, 218_000, -218_111, Some(-1_001))
    );
}

#[test]
fn s11_february_compares_with_the_last_28_days_of_january() {
    let fixture = Fixture::new();
    fixture.account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.statement(1, 1, "2026-01-01", "2026-02-28");
    fixture.category(901, "Potraviny", "expense");
    fixture.tx(tx(1, 1, "2026-01-03", -700, Some(901), "mimo"));
    fixture.tx(tx(2, 1, "2026-01-04", -300, Some(901), "zaciatok"));
    fixture.tx(tx(3, 1, "2026-01-31", -200, Some(901), "koniec"));
    fixture.tx(tx(4, 1, "2026-02-01", -100, Some(901), "teraz"));
    fixture.tx(tx(5, 1, "2026-02-28", 500, None, "prijem"));
    let report = summary(&fixture, &month("2026-02"), "2026-09-07");
    let previous = ready(&report);
    assert_eq!(previous.range.from, day("2026-01-04"));
    assert_eq!(previous.range.to, day("2026-01-31"));
    assert_eq!(delta_tuple(previous.expense), (100, 500, -400, Some(-800)));
    assert_eq!(delta_tuple(previous.income), (500, 0, 500, None));
}

#[test]
fn s12_a_period_whose_predecessor_starts_before_year_one_has_no_comparison() {
    let fixture = Fixture::new();
    fixture.account(1, PERSONAL_IBAN, "personal", "Osobný");
    let request = ReportRequest {
        period: ReportPeriod::Month {
            month: "0001-01".into(),
        },
        scope: ReportScope::All,
    };
    let report = summary(&fixture, &request, "2026-09-07");
    assert_eq!(report.extras.comparison, ReportComparison::OutOfRange);
}

#[test]
fn s13_scope_limits_every_extra_to_the_selected_accounts() {
    let profile = busy_profile();
    let request = ReportRequest {
        period: ReportPeriod::Month {
            month: "2026-08".into(),
        },
        scope: ReportScope::Kind {
            account_kind: AccountKind::Business,
        },
    };
    let report = summary(&profile, &request, "2026-09-07");
    assert_eq!(report.extras.account_totals.len(), 1);
    assert_eq!(report.extras.account_totals[0].account_id, 2);
    assert_eq!(
        report
            .extras
            .largest
            .iter()
            .map(|row| row.transaction.id)
            .collect::<Vec<_>>(),
        vec![17, 18]
    );
    let previous = ready(&report);
    assert_eq!(delta_tuple(previous.income), (100_000, 0, 100_000, None));
}

#[test]
fn s14_extras_come_from_the_snapshot_of_the_report_not_from_a_later_read() {
    let profile = busy_profile();
    profile
        .connection()
        .execute_batch("PRAGMA journal_mode=WAL;")
        .unwrap();
    let path = profile.path.clone();
    let (start_send, start_receive) = std::sync::mpsc::channel::<()>();
    let (done_send, done_receive) = std::sync::mpsc::channel::<()>();
    let writer = std::thread::spawn(move || {
        start_receive.recv().unwrap();
        let connection = Connection::open(path).unwrap();
        connection.execute_batch(
            "BEGIN IMMEDIATE; \
             INSERT INTO transactions(id,statement_id,account_id,fingerprint,posted_date,tx_date,kind,amount_cents,merchant_raw,merchant_norm,raw_block,status,source,category_id,note) VALUES(90,1,1,'tx-90','2026-07-10','2026-07-10','card',-77700,'new-july','new-july','raw','confirmed','none',901,''); \
             INSERT INTO transactions(id,statement_id,account_id,fingerprint,posted_date,tx_date,kind,amount_cents,merchant_raw,merchant_norm,raw_block,status,source,category_id,note) VALUES(91,1,1,'tx-91','2026-08-10','2026-08-10','card',1000000,'new-august','new-august','raw','confirmed','none',911,''); \
             COMMIT;",
        ).unwrap();
        done_send.send(()).unwrap();
    });
    let mut store = profile.store();
    let old = store
        .report_summary_snapshot_with_hook(
            &month("2026-08"),
            &clock("2026-09-07"),
            ReportCaptureLimits::default(),
            || {
                start_send.send(()).unwrap();
                done_receive.recv().unwrap();
            },
        )
        .unwrap();
    writer.join().unwrap();
    let previous = ready(&old);
    assert_eq!(
        delta_tuple(previous.expense),
        (132_500, 93_500, 39_000, Some(417))
    );
    assert_eq!(old.extras.largest[0].transaction.id, 1);
    assert_eq!(old.extras.account_totals[0].transaction_count, 16);
    let fresh = store
        .report_summary_snapshot(&month("2026-08"), &clock("2026-09-07"))
        .unwrap();
    assert_eq!(ready(&fresh).expense.previous_cents, 171_200);
    assert_eq!(fresh.extras.largest[0].transaction.id, 91);
}

fn recurring_profile() -> Fixture {
    let fixture = Fixture::new();
    fixture.account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.account(2, BUSINESS_IBAN, "business", "Firemný");
    fixture.statement(1, 1, "2026-01-01", "2026-04-30");
    fixture.statement(2, 2, "2026-01-01", "2026-04-30");
    let mut id = 0;
    for (account, merchant, cents) in [(1, "NETFLIX.COM", -1_200), (2, "ADOBE", -3_000)] {
        for date in ["2026-01-31", "2026-02-28", "2026-03-31"] {
            id += 1;
            let mut row = tx(id, account, date, cents, None, merchant);
            row.status = "unassigned";
            fixture.tx(row);
        }
    }
    fixture
}

fn recurring_names(report: &SummarySnapshot) -> Vec<(String, Option<i64>)> {
    report
        .extras
        .recurring
        .rows
        .iter()
        .map(|row| (row.name.clone(), row.monthly_cents))
        .collect()
}

#[test]
fn s15_recurring_payments_follow_the_report_accounts_and_stay_estimates() {
    let profile = recurring_profile();
    let all = summary(&profile, &month("2026-04"), "2026-04-15");
    assert_eq!(
        recurring_names(&all),
        vec![
            ("ADOBE".to_string(), Some(3_000)),
            ("NETFLIX.COM".to_string(), Some(1_200))
        ],
        "same order as the Prehľad rows"
    );
    assert_eq!(all.extras.recurring.estimates.monthly_expense_cents, 4_200);
    assert_eq!(
        all.extras.recurring.estimates.remaining_expense_cents,
        4_200
    );
    assert_eq!(all.extras.recurring.confirmed.monthly_expense_cents, 0);
    assert_eq!(
        all.extras.recurring.rows[1].next_due,
        Some(day("2026-04-30"))
    );
    assert_eq!(all.extras.recurring.as_of, day("2026-04-15"));

    let personal = ReportRequest {
        period: ReportPeriod::Month {
            month: "2026-04".into(),
        },
        scope: ReportScope::Kind {
            account_kind: AccountKind::Personal,
        },
    };
    let narrowed = summary(&profile, &personal, "2026-04-15");
    assert_eq!(
        recurring_names(&narrowed),
        vec![("NETFLIX.COM".to_string(), Some(1_200))]
    );
    assert_eq!(
        narrowed.extras.recurring.estimates.monthly_expense_cents,
        1_200
    );
}
