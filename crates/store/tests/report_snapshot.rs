//! Literal store-level oracle for PDF report capture (P01-P08, P13, P17).
use chrono::{FixedOffset, NaiveDate, TimeZone};
use parser::AccountKind;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use store::report::{ReportCaptureLimits, ReportClock, ReportPeriod, ReportRequest, ReportScope};
use store::Store;

const PERSONAL_IBAN: &str = "SK4411000000000012345678";
const SECOND_IBAN: &str = "SK3112000000198742637541";
const BUSINESS_IBAN: &str = "SK3711000000000098765432";

macro_rules! add_transaction {
    ($fixture:expr, $id:expr, $account_id:expr, $statement_id:expr, $posted:expr, $date:expr, $kind:expr, $cents:expr, $status:expr, $category_id:expr, $merchant:expr, $note:expr) => {
        $fixture.add_transaction(TransactionSeed {
            id: $id,
            account_id: $account_id,
            statement_id: $statement_id,
            posted: $posted,
            date: $date,
            kind: $kind,
            cents: $cents,
            status: $status,
            category_id: $category_id,
            merchant: $merchant,
            note: $note,
        })
    };
}

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

fn request(period: ReportPeriod, scope: ReportScope) -> ReportRequest {
    ReportRequest { period, scope }
}

struct Fixture {
    _directory: tempfile::TempDir,
    path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("report.db");
        Store::open(&path).unwrap();
        Self {
            _directory: directory,
            path,
        }
    }

    fn connection(&self) -> Connection {
        let connection = Connection::open(&self.path).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        connection
    }

    fn store(&self) -> Store {
        Store::open(&self.path).unwrap()
    }

    fn add_account(&self, id: i64, iban: &str, kind: &str, label: &str) {
        self.connection()
            .execute(
                "INSERT INTO accounts(id,iban,kind,label) VALUES(?1,?2,?3,?4)",
                params![id, iban, kind, label],
            )
            .unwrap();
    }

    fn add_statement(&self, id: i64, account_id: i64, from: &str, to: &str, checksum: &str) {
        self.connection().execute(
            "INSERT INTO statements(id,account_id,number,period_start,period_end,checksum_status,file_hash) VALUES(?1,?2,?1,?3,?4,?5,?6)",
            params![id, account_id, from, to, checksum, format!("statement-{id}")],
        ).unwrap();
    }

    fn add_transaction(&self, seed: TransactionSeed<'_>) {
        self.connection().execute(
            "INSERT INTO transactions(id,statement_id,account_id,fingerprint,posted_date,tx_date,kind,amount_cents,merchant_raw,merchant_norm,raw_block,status,source,category_id,note) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,'private source must not enter report',?10,'none',?11,?12)",
            params![seed.id, seed.statement_id, seed.account_id, format!("tx-{}", seed.id), seed.posted, seed.date, seed.kind, seed.cents, seed.merchant, seed.status, seed.category_id, seed.note],
        ).unwrap();
    }
}

struct TransactionSeed<'a> {
    id: i64,
    account_id: i64,
    statement_id: i64,
    posted: &'a str,
    date: &'a str,
    kind: &'a str,
    cents: i64,
    status: &'a str,
    category_id: Option<i64>,
    merchant: &'a str,
    note: &'a str,
}

#[test]
fn p01_calendar_ranges_are_exact_and_membership_uses_transaction_date() {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_statement(1, 1, "2024-01-01", "2026-12-31", "ok");
    add_transaction!(
        fixture,
        1,
        1,
        1,
        "2024-01-01",
        "2024-02-01",
        "card",
        -100,
        "confirmed",
        None,
        "first-edge",
        ""
    );
    add_transaction!(
        fixture,
        2,
        1,
        1,
        "2024-04-01",
        "2024-02-29",
        "card",
        -200,
        "confirmed",
        None,
        "last-edge",
        ""
    );
    add_transaction!(
        fixture,
        3,
        1,
        1,
        "2024-02-15",
        "2024-01-31",
        "card",
        -400,
        "confirmed",
        None,
        "outside-before",
        ""
    );
    add_transaction!(
        fixture,
        4,
        1,
        1,
        "2024-02-15",
        "2024-03-01",
        "card",
        -800,
        "confirmed",
        None,
        "outside-after",
        ""
    );
    let mut store = fixture.store();

    let leap = store
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2024-02".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    let ordinary = store
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2025-02".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    let six = store
        .report_snapshot(
            &request(
                ReportPeriod::SixMonths {
                    ending_month: "2026-02".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    let year = store
        .report_snapshot(
            &request(ReportPeriod::Year { year: 2024 }, ReportScope::All),
            &clock("2026-09-07"),
        )
        .unwrap();

    assert_eq!(leap.preview.range.unwrap().from, day("2024-02-01"));
    assert_eq!(leap.preview.range.unwrap().to, day("2024-02-29"));
    assert_eq!(
        leap.transactions
            .iter()
            .map(|row| row.transaction.merchant_raw.as_str())
            .collect::<Vec<_>>(),
        ["last-edge", "first-edge"]
    );
    assert_eq!(ordinary.preview.range.unwrap().to, day("2025-02-28"));
    assert_eq!(six.preview.range.unwrap().from, day("2025-09-01"));
    assert_eq!(six.preview.range.unwrap().to, day("2026-02-28"));
    assert_eq!(
        six.months
            .iter()
            .map(|month| month.month.as_str())
            .collect::<Vec<_>>(),
        ["2025-09", "2025-10", "2025-11", "2025-12", "2026-01", "2026-02"]
    );
    assert_eq!(
        year.preview.range.unwrap(),
        store::report::ReportDateRange {
            from: day("2024-01-01"),
            to: day("2024-12-31")
        }
    );
    assert_eq!(year.months.len(), 12);
}

#[test]
fn p02_current_period_keeps_calendar_end_and_invalid_or_future_periods_fail() {
    let mut store = Store::open_in_memory().unwrap();
    let september = store
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2026-09".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    let august = store
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2026-08".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    let six = store
        .report_snapshot(
            &request(
                ReportPeriod::SixMonths {
                    ending_month: "2026-09".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    let year = store
        .report_snapshot(
            &request(ReportPeriod::Year { year: 2026 }, ReportScope::All),
            &clock("2026-09-07"),
        )
        .unwrap();
    assert_eq!(september.preview.range.unwrap().to, day("2026-09-30"));
    assert!(september.preview.unfinished_period);
    assert!(!august.preview.unfinished_period);
    assert_eq!(
        six.preview.range.unwrap(),
        store::report::ReportDateRange {
            from: day("2026-04-01"),
            to: day("2026-09-30")
        }
    );
    assert!(six.preview.unfinished_period);
    assert_eq!(year.preview.range.unwrap().to, day("2026-12-31"));
    assert!(year.preview.unfinished_period);
    let upper_month = store
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "9999-12".into(),
                },
                ReportScope::All,
            ),
            &clock("9999-12-01"),
        )
        .unwrap();
    assert_eq!(upper_month.preview.range.unwrap().to, day("9999-12-31"));
    for period in [
        ReportPeriod::Month {
            month: "2026-13".into(),
        },
        ReportPeriod::Month {
            month: "2026-2".into(),
        },
        ReportPeriod::Month {
            month: "2026-10".into(),
        },
        ReportPeriod::Month {
            month: "0000-12".into(),
        },
        ReportPeriod::Year { year: 0 },
        ReportPeriod::Year { year: 10_000 },
        ReportPeriod::SixMonths {
            ending_month: "0001-04".into(),
        },
    ] {
        assert!(store
            .report_snapshot(&request(period, ReportScope::All), &clock("2026-09-07"))
            .is_err());
    }
}

#[test]
fn p03_scope_includes_every_matching_account_and_rejects_unknown_or_unsafe_ids() {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný 1");
    fixture.add_account(2, SECOND_IBAN, "personal", "Osobný 2 bez výpisov");
    fixture.add_account(3, BUSINESS_IBAN, "business", "Firemný");
    fixture.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    fixture.add_statement(2, 3, "2026-09-01", "2026-09-30", "ok");
    let mut store = fixture.store();
    let period = ReportPeriod::Month {
        month: "2026-09".into(),
    };

    let all = store
        .report_snapshot(
            &request(period.clone(), ReportScope::All),
            &clock("2026-09-07"),
        )
        .unwrap();
    let personal = store
        .report_snapshot(
            &request(
                period.clone(),
                ReportScope::Kind {
                    account_kind: AccountKind::Personal,
                },
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    let exact = store
        .report_snapshot(
            &request(period.clone(), ReportScope::Account { account_id: 3 }),
            &clock("2026-09-07"),
        )
        .unwrap();

    assert_eq!(all.preview.accounts.len(), 3);
    assert_eq!(
        personal
            .preview
            .accounts
            .iter()
            .map(|account| account.id)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(personal.preview.accounts_without_statements, 1);
    assert_eq!(exact.preview.accounts[0].id, 3);
    assert!(store
        .report_snapshot(
            &request(period.clone(), ReportScope::Account { account_id: -1 }),
            &clock("2026-09-07")
        )
        .is_err());
    assert!(store
        .report_snapshot(
            &request(
                period.clone(),
                ReportScope::Account {
                    account_id: 9_007_199_254_740_992
                }
            ),
            &clock("2026-09-07")
        )
        .is_err());
    assert!(store
        .report_snapshot(
            &request(period, ReportScope::Account { account_id: 99 }),
            &clock("2026-09-07")
        )
        .is_err());
}

fn money_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    let connection = fixture.connection();
    connection
        .execute(
            "INSERT INTO categories(id,name,kind,sort) VALUES(900,'Výdavok','expense',900)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO categories(id,name,kind,sort) VALUES(901,'Príjem','income',901)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO categories(id,name,parent_id,kind,sort) VALUES(902,'Jedlo',900,'expense',902)",
            [],
        )
        .unwrap();
    drop(connection);
    fixture
}

struct MoneyRowSeed {
    id: i64,
    kind: &'static str,
    cents: i64,
    status: &'static str,
    category: Option<i64>,
    merchant: &'static str,
}

fn money_row(
    id: i64,
    kind: &'static str,
    cents: i64,
    status: &'static str,
    category: Option<i64>,
    merchant: &'static str,
) -> MoneyRowSeed {
    MoneyRowSeed {
        id,
        kind,
        cents,
        status,
        category,
        merchant,
    }
}

fn add_money_rows(fixture: &Fixture, rows: &[MoneyRowSeed]) {
    for row in rows {
        add_transaction!(
            fixture,
            row.id,
            1,
            1,
            "2026-09-07",
            "2026-09-07",
            row.kind,
            row.cents,
            row.status,
            row.category,
            row.merchant,
            ""
        );
    }
}

fn september_report(fixture: &Fixture) -> store::report::ReportSnapshot {
    fixture
        .store()
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2026-09".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap()
}

fn assert_p04_money(report: &store::report::ReportSnapshot) {
    assert_eq!(report.preview.transaction_count, 8);
    assert_eq!(
        (
            report.totals.income_cents,
            report.totals.expense_cents,
            report.totals.net_cents
        ),
        (100_000, 10_000, 90_000)
    );
    assert_eq!(
        (
            report.totals.transfer_out_cents,
            report.totals.transfer_in_cents
        ),
        (20_000, 20_000)
    );
    assert_eq!(
        (
            report.totals.suggested_count,
            report.totals.unassigned_count
        ),
        (1, 1)
    );
    assert_eq!(
        report
            .categories
            .iter()
            .map(|category| category.cents)
            .sum::<i64>(),
        10_000
    );
    let assigned = report
        .categories
        .iter()
        .find(|category| category.category_id == Some(902))
        .unwrap();
    assert_eq!(assigned.cents, 7_500);
    assert_eq!(assigned.label, "Výdavok / Jedlo");
    assert_eq!(
        report
            .categories
            .iter()
            .find(|category| category.category_id.is_none())
            .map(|category| category.cents),
        Some(2_500)
    );
}

fn assert_p05_money(report: &store::report::ReportSnapshot) {
    assert_eq!(
        (
            report.totals.income_cents,
            report.totals.expense_cents,
            report.totals.net_cents
        ),
        (101_100, 9_300, 91_800)
    );
    assert_eq!(report.preview.transaction_count, 12);
    assert_eq!(
        (
            report.totals.suggested_count,
            report.totals.unassigned_count
        ),
        (2, 1),
        "a suggested row assigned to an expense category keeps its status badge"
    );
    for merchant in ["negative-income", "income-refund"] {
        assert!(report
            .transactions
            .iter()
            .any(|row| row.transaction.merchant_raw == merchant));
    }
}

#[test]
fn p04_p05_literal_money_semantics_keep_refunds_transfers_zero_and_mismatches_visible() {
    let fixture = money_fixture();
    add_money_rows(
        &fixture,
        &[
            money_row(1, "card", 100_000, "confirmed", Some(901), "income"),
            money_row(2, "card", -10_000, "confirmed", Some(902), "expense"),
            money_row(3, "refund", 2_500, "confirmed", Some(902), "refund"),
            money_row(4, "card", -3_000, "suggested", None, "suggested-null"),
            money_row(5, "refund", 500, "unassigned", None, "unassigned-refund"),
            money_row(6, "transfer_out", -20_000, "transfer", None, "transfer-out"),
            money_row(7, "transfer_in", 20_000, "transfer", None, "transfer-in"),
            money_row(8, "other", 0, "confirmed", None, "zero"),
        ],
    );
    assert_p04_money(&september_report(&fixture));
    add_money_rows(
        &fixture,
        &[
            money_row(9, "card", 700, "suggested", Some(900), "positive-expense"),
            money_row(10, "card", -900, "confirmed", Some(901), "negative-income"),
            money_row(11, "refund", 300, "confirmed", Some(901), "income-refund"),
            money_row(
                12,
                "card",
                1_100,
                "confirmed",
                None,
                "positive-uncategorized",
            ),
        ],
    );
    assert_p05_money(&september_report(&fixture));
}

#[test]
fn p06_refund_only_can_make_expense_negative_and_overflow_is_an_error() {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    let connection = fixture.connection();
    connection
        .execute(
            "INSERT INTO categories(id,name,kind,sort) VALUES(900,'Výdavok','expense',900)",
            [],
        )
        .unwrap();
    drop(connection);
    add_transaction!(
        fixture,
        1,
        1,
        1,
        "2026-09-01",
        "2026-09-01",
        "refund",
        2_500,
        "confirmed",
        Some(900),
        "refund-only",
        ""
    );
    let mut store = fixture.store();
    let report = store
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2026-09".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    assert_eq!(
        (
            report.totals.income_cents,
            report.totals.expense_cents,
            report.totals.net_cents
        ),
        (0, -2_500, 2_500)
    );
    drop(store);

    add_transaction!(
        fixture,
        2,
        1,
        1,
        "2026-09-02",
        "2026-09-02",
        "card",
        i64::MAX,
        "confirmed",
        None,
        "maximum-a",
        ""
    );
    add_transaction!(
        fixture,
        3,
        1,
        1,
        "2026-09-03",
        "2026-09-03",
        "card",
        i64::MAX,
        "confirmed",
        None,
        "maximum-b",
        ""
    );
    let error = fixture
        .store()
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2026-09".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap_err();
    assert!(error.to_string().contains("prekročil"));

    let minimum = Fixture::new();
    minimum.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    minimum.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    add_transaction!(
        minimum,
        1,
        1,
        1,
        "2026-09-01",
        "2026-09-01",
        "card",
        i64::MIN,
        "confirmed",
        None,
        "minimum",
        ""
    );
    let error = minimum
        .store()
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2026-09".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap_err();
    assert!(
        error.to_string().contains("prekročil"),
        "negating i64::MIN must be rejected, never clamped"
    );

    let net = Fixture::new();
    net.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    net.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    add_transaction!(
        net,
        1,
        1,
        1,
        "2026-09-01",
        "2026-09-01",
        "card",
        i64::MAX,
        "confirmed",
        None,
        "income-max",
        ""
    );
    add_transaction!(
        net,
        2,
        1,
        1,
        "2026-09-02",
        "2026-09-02",
        "refund",
        1,
        "confirmed",
        None,
        "negative-expense",
        ""
    );
    let error = net
        .store()
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2026-09".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap_err();
    assert!(
        error.to_string().contains("prekročil"),
        "income minus a negative expense must use checked conversion"
    );
}

#[test]
fn p05_original_currency_stays_signed_and_does_not_replace_booked_eur() {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    add_transaction!(
        fixture,
        1,
        1,
        1,
        "2026-09-01",
        "2026-09-01",
        "card",
        -923,
        "confirmed",
        None,
        "foreign",
        ""
    );
    fixture
        .connection()
        .execute(
            "UPDATE transactions SET orig_amount_cents=-1000,orig_currency='USD' WHERE id=1",
            [],
        )
        .unwrap();
    let report = fixture
        .store()
        .report_snapshot(
            &request(ReportPeriod::AllTime, ReportScope::All),
            &clock("2026-09-07"),
        )
        .unwrap();
    assert_eq!(report.totals.expense_cents, 923);
    assert_eq!(report.transactions[0].transaction.amount_cents, -923);
    assert_eq!(
        report.transactions[0].transaction.orig_amount_cents,
        Some(-1_000)
    );
    assert_eq!(
        report.transactions[0].transaction.orig_currency.as_deref(),
        Some("USD")
    );
}

#[test]
fn p07_statement_ranges_merge_per_account_and_keep_quality_separate() {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_account(2, SECOND_IBAN, "personal", "Bez výpisov");
    fixture.add_statement(1, 1, "2026-09-03", "2026-09-05", "ok");
    fixture.add_statement(2, 1, "2026-09-05", "2026-09-07", "off");
    fixture.add_statement(3, 1, "2026-09-09", "2026-09-10", "not_verifiable");
    fixture.add_statement(4, 1, "2026-09-12", "2026-09-11", "ok");
    fixture.add_statement(5, 1, "2026-09-03", "2026-09-05", "ok");
    fixture.add_statement(6, 1, "2026-09-11", "2026-09-11", "ok");
    let mut store = fixture.store();
    let report = store
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2026-09".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    let first = &report.coverage[0];
    assert_eq!(first.statement_count, 6);
    assert_eq!(first.unverified_statement_count, 2);
    assert_eq!(first.invalid_range_count, 1);
    assert_eq!(
        first.gaps,
        [
            store::report::ReportDateRange {
                from: day("2026-09-01"),
                to: day("2026-09-02")
            },
            store::report::ReportDateRange {
                from: day("2026-09-08"),
                to: day("2026-09-08")
            },
            store::report::ReportDateRange {
                from: day("2026-09-12"),
                to: day("2026-09-30")
            },
        ]
    );
    assert_eq!(report.coverage[1].statement_count, 0);

    let all_time = store
        .report_snapshot(
            &request(
                ReportPeriod::AllTime,
                ReportScope::Account { account_id: 1 },
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    assert!(all_time.preview.range.is_none());
    assert_eq!(
        all_time.coverage[0].known_range,
        Some(store::report::ReportDateRange {
            from: day("2026-09-03"),
            to: day("2026-09-11")
        })
    );
    assert_eq!(
        all_time.coverage[0].gaps,
        [store::report::ReportDateRange {
            from: day("2026-09-08"),
            to: day("2026-09-08")
        }],
        "all-time coverage has only internal gaps, without invented leading or trailing dates"
    );
}

#[test]
fn p07_coverage_at_year_9999_has_no_invented_trailing_gap() {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_statement(1, 1, "9999-12-01", "9999-12-31", "ok");
    let report = fixture
        .store()
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "9999-12".into(),
                },
                ReportScope::All,
            ),
            &clock("9999-12-01"),
        )
        .unwrap();
    assert!(report.coverage[0].gaps.is_empty());
}

fn spawn_snapshot_writer(
    path: PathBuf,
    start_receive: std::sync::mpsc::Receiver<()>,
    done_send: std::sync::mpsc::Sender<()>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        start_receive.recv().unwrap();
        let connection = Connection::open(path).unwrap();
        connection.execute_batch("PRAGMA journal_mode=WAL; BEGIN IMMEDIATE; UPDATE accounts SET label='Nový účet' WHERE id=1; UPDATE statements SET period_start='2026-09-02',checksum_status='off' WHERE id=1; UPDATE categories SET name='Nová kategória',kind='income' WHERE id=900; UPDATE transactions SET note='nová poznámka' WHERE id=1; INSERT INTO transactions(id,statement_id,account_id,fingerprint,posted_date,tx_date,kind,amount_cents,merchant_raw,merchant_norm,raw_block,status,source,note) VALUES(2,1,1,'tx-2','2026-09-02','2026-09-02','card',200,'new-row','new-row','raw','confirmed','none','nový riadok'); COMMIT;").unwrap();
        done_send.send(()).unwrap();
    })
}

fn assert_old_snapshot(report: &store::report::ReportSnapshot) {
    assert_eq!(report.preview.transaction_count, 1);
    assert_eq!(
        report.preview.latest_transaction_date,
        Some(day("2026-09-01"))
    );
    assert_eq!(report.preview.accounts[0].label, "Osobný");
    assert_eq!(
        report.coverage[0].known_range.unwrap().from,
        day("2026-09-01")
    );
    assert_eq!(report.coverage[0].unverified_statement_count, 0);
    assert_eq!(report.transactions[0].transaction.note, "stará poznámka");
    assert_eq!(
        report.transactions[0].transaction.category_name.as_deref(),
        Some("Stará kategória")
    );
    assert_eq!(
        (report.totals.income_cents, report.totals.expense_cents),
        (0, -100)
    );
}

fn assert_new_snapshot(report: &store::report::ReportSnapshot) {
    assert_eq!(report.preview.transaction_count, 2);
    assert_eq!(
        report.preview.latest_transaction_date,
        Some(day("2026-09-02"))
    );
    assert_eq!(report.preview.accounts[0].label, "Nový účet");
    assert_eq!(
        report.coverage[0].known_range.unwrap().from,
        day("2026-09-02")
    );
    assert_eq!(report.coverage[0].unverified_statement_count, 1);
    assert_eq!(
        report
            .transactions
            .iter()
            .find(|row| row.transaction.id == 1)
            .unwrap()
            .transaction
            .note,
        "nová poznámka"
    );
    assert_eq!(
        (report.totals.income_cents, report.totals.expense_cents),
        (300, 0)
    );
}

#[test]
fn p08_wal_writer_cannot_mix_new_category_note_or_row_into_an_established_snapshot() {
    let fixture = Fixture::new();
    fixture
        .connection()
        .execute_batch("PRAGMA journal_mode=WAL;")
        .unwrap();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    let connection = fixture.connection();
    connection
        .execute(
            "INSERT INTO categories(id,name,kind,sort) VALUES(900,'Stará kategória','expense',900)",
            [],
        )
        .unwrap();
    drop(connection);
    add_transaction!(
        fixture,
        1,
        1,
        1,
        "2026-09-01",
        "2026-09-01",
        "card",
        100,
        "confirmed",
        Some(900),
        "old-row",
        "stará poznámka"
    );
    let path = fixture.path.clone();
    let (start_send, start_receive) = std::sync::mpsc::channel();
    let (done_send, done_receive) = std::sync::mpsc::channel();
    let writer = spawn_snapshot_writer(path, start_receive, done_send);
    let mut store = fixture.store();
    let query = request(
        ReportPeriod::Month {
            month: "2026-09".into(),
        },
        ReportScope::All,
    );
    let old = store
        .report_snapshot_with_hook(
            &query,
            &clock("2026-09-07"),
            ReportCaptureLimits::default(),
            || {
                start_send.send(()).unwrap();
                done_receive.recv().unwrap();
            },
        )
        .unwrap();
    writer.join().unwrap();
    let new = store.report_snapshot(&query, &clock("2026-09-07")).unwrap();

    assert_old_snapshot(&old);
    assert_new_snapshot(&new);
}

#[test]
fn p08_read_error_rolls_back_and_leaves_the_store_usable() {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    add_transaction!(
        fixture,
        1,
        1,
        1,
        "2026-09-01",
        "not-a-date",
        "card",
        -100,
        "confirmed",
        None,
        "corrupt",
        ""
    );
    let mut store = fixture.store();
    let query = request(ReportPeriod::AllTime, ReportScope::All);

    assert!(store.report_snapshot(&query, &clock("2026-09-07")).is_err());
    fixture
        .connection()
        .execute("DELETE FROM transactions WHERE id=1", [])
        .unwrap();
    assert_eq!(
        store
            .report_snapshot(&query, &clock("2026-09-07"))
            .unwrap()
            .preview
            .transaction_count,
        0,
        "a failed read must not leave a transaction open"
    );
}

#[test]
fn p09_capture_returns_every_row_in_descending_transaction_date_and_id_order() {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    for id in 1..=31 {
        add_transaction!(
            fixture,
            id,
            1,
            1,
            "2026-09-07",
            "2026-09-07",
            "card",
            -100,
            "confirmed",
            None,
            &format!("SENTINEL-{id:02}"),
            ""
        );
    }
    let report = fixture
        .store()
        .report_snapshot(
            &request(
                ReportPeriod::Month {
                    month: "2026-09".into(),
                },
                ReportScope::All,
            ),
            &clock("2026-09-07"),
        )
        .unwrap();
    assert_eq!(report.preview.transaction_count, 31);
    assert_eq!(report.totals.expense_cents, 3_100);
    assert_eq!(report.transactions.first().unwrap().transaction.id, 31);
    assert_eq!(report.transactions.last().unwrap().transaction.id, 1);
    assert_eq!(
        report
            .transactions
            .iter()
            .map(|row| row.transaction.id)
            .collect::<Vec<_>>(),
        (1..=31).rev().collect::<Vec<_>>()
    );
}

#[test]
fn p13_capture_limits_accept_the_boundary_and_reject_one_over_without_partial_data() {
    assert_eq!(ReportCaptureLimits::default().max_transactions, 100_000);
    assert_eq!(
        ReportCaptureLimits::default().max_text_bytes,
        32 * 1024 * 1024
    );
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "A");
    fixture.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    add_transaction!(
        fixture,
        1,
        1,
        1,
        "2026-09-01",
        "2026-09-01",
        "card",
        1,
        "confirmed",
        None,
        "M",
        ""
    );
    let mut store = fixture.store();
    let query = request(
        ReportPeriod::Month {
            month: "2026-09".into(),
        },
        ReportScope::All,
    );
    let boundary = ReportCaptureLimits {
        max_transactions: 1,
        max_text_bytes: 7,
    };
    assert_eq!(
        store
            .report_snapshot_with_hook(&query, &clock("2026-09-07"), boundary, || {})
            .unwrap()
            .preview
            .transaction_count,
        1
    );
    assert!(store
        .report_snapshot_with_hook(
            &query,
            &clock("2026-09-07"),
            ReportCaptureLimits {
                max_transactions: 0,
                max_text_bytes: 7
            },
            || {}
        )
        .is_err());
    assert!(store
        .report_snapshot_with_hook(
            &query,
            &clock("2026-09-07"),
            ReportCaptureLimits {
                max_transactions: 1,
                max_text_bytes: 6
            },
            || {}
        )
        .is_err());
}

#[test]
fn p17_report_reads_actual_rows_without_mutating_unrelated_v4_state() {
    let fixture = Fixture::new();
    fixture.add_account(1, PERSONAL_IBAN, "personal", "Osobný");
    fixture.add_statement(1, 1, "2026-09-01", "2026-09-30", "ok");
    fixture.connection().execute_batch("INSERT INTO categories(id,name,kind,sort) VALUES(900,'Upravená kategória','expense',900); INSERT INTO settings(key,value) VALUES('report_test_setting','retained');").unwrap();
    add_transaction!(
        fixture,
        1,
        1,
        1,
        "2026-09-01",
        "2026-09-01",
        "card",
        -500,
        "confirmed",
        Some(900),
        "actual-only",
        "durable note"
    );
    fixture.connection().execute_batch("CREATE TABLE report_test_recurring_state(id INTEGER PRIMARY KEY, decision TEXT NOT NULL); INSERT INTO report_test_recurring_state VALUES(1,'ignored');").unwrap();
    let before = logical_state(&fixture.path);
    let mut store = fixture.store();
    let report = store
        .report_snapshot(
            &request(ReportPeriod::AllTime, ReportScope::All),
            &clock("2026-09-07"),
        )
        .unwrap();

    assert_eq!(report.preview.transaction_count, 1);
    assert_eq!(report.transactions[0].transaction.note, "durable note");
    assert_eq!(
        report.transactions[0].transaction.category_name.as_deref(),
        Some("Upravená kategória")
    );
    assert!(report.transactions[0].transaction.raw_block.is_empty());
    drop(store);
    let restarted_report = fixture
        .store()
        .report_snapshot(
            &request(ReportPeriod::AllTime, ReportScope::All),
            &clock("2026-09-07"),
        )
        .unwrap();
    assert_eq!(restarted_report.preview.transaction_count, 1);
    assert_eq!(
        restarted_report.transactions[0].transaction.note,
        "durable note"
    );
    let after = logical_state(&fixture.path);
    assert_eq!(
        before, after,
        "capture must be read-only, including unrelated recurring decision state"
    );
}

fn logical_state(path: &Path) -> (i64, String, i64, String, String, String) {
    let connection = Connection::open(path).unwrap();
    let version = connection
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key='schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let decision = connection
        .query_row(
            "SELECT decision FROM report_test_recurring_state WHERE id=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let transaction_count = connection
        .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))
        .unwrap();
    let note = connection
        .query_row("SELECT note FROM transactions WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    let category = connection
        .query_row("SELECT name FROM categories WHERE id=900", [], |row| {
            row.get(0)
        })
        .unwrap();
    let setting = connection
        .query_row(
            "SELECT value FROM settings WHERE key='report_test_setting'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    (
        version,
        decision,
        transaction_count,
        note,
        category,
        setting,
    )
}
