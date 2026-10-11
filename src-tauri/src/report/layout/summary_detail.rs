//! Pages 2 and 3 of the three-page summary (plan 084).
use super::summary::{change_text, format_signed_money, left, right, share_text, Column};
use super::*;
use store::recurring::{RecurringDirection, RecurringRow, RowDecision};
use store::report::{ReportCategoryDelta, ReportComparison, ReportPrevious, SummarySnapshot};

const DELTA_ROWS: usize = 12;
const CATEGORY_ROWS: usize = 22;
const ACCOUNT_ROWS: usize = 6;
const MONTH_ROWS: usize = 12;
const RECURRING_ROWS: usize = 6;
const UNAVAILABLE: &str = "nedostupné";

const LABEL: Column = left(MARGIN, 190.0);
const FIGURES: [Column; 4] = [
    right(330.0, 78.0),
    right(410.0, 78.0),
    right(490.0, 78.0),
    right(PAGE_WIDTH - MARGIN, 70.0),
];

fn five_columns() -> [Column; 5] {
    [LABEL, FIGURES[0], FIGURES[1], FIGURES[2], FIGURES[3]]
}

impl Layout {
    pub(super) fn summary_comparison_page(
        &mut self,
        summary: &SummarySnapshot,
    ) -> Result<(), String> {
        self.page_header(
            summary,
            "Abakus · Sumár do 3 strán · porovnanie a kategórie",
        )?;
        self.block_heading("Porovnanie s predchádzajúcim obdobím")?;
        match &summary.extras.comparison {
            ReportComparison::AllTime => {
                self.note("Celé obdobie nemá porovnanie s predchádzajúcim obdobím.")?;
            }
            ReportComparison::OutOfRange => {
                self.note("Predchádzajúce obdobie by začínalo pred rokom 0001. Porovnanie nie je dostupné.")?;
            }
            ReportComparison::Ready(previous) => self.comparison_block(summary, previous)?,
        }
        self.all_categories(summary)?;
        self.page_guard()
    }

    fn comparison_block(
        &mut self,
        summary: &SummarySnapshot,
        previous: &ReportPrevious,
    ) -> Result<(), String> {
        self.note(&format!(
            "Predchádzajúce obdobie {} až {} (rovnaký počet dní).",
            date(previous.range.from),
            date(previous.range.to)
        ))?;
        if summary.snapshot.preview.unfinished_period {
            self.warning("Obdobie ešte neskončilo. Porovnanie je predbežné.")?;
        }
        if previous.accounts_without_statements > 0 || previous.accounts_with_gaps > 0 {
            self.warning(&format!(
                "Predchádzajúce obdobie nie je celé pokryté výpismi. Účty bez výpisov: {}. Účty s medzerami: {}.",
                previous.accounts_without_statements, previous.accounts_with_gaps
            ))?;
        }
        self.grid_header(
            &five_columns(),
            &["Ukazovateľ", "Teraz", "Predtým", "Rozdiel", "Zmena"],
        )?;
        for (label, delta) in [
            ("Príjmy", previous.income),
            ("Výdavky", previous.expense),
            ("Rozdiel príjmov a výdavkov", previous.net),
        ] {
            self.grid_row(&five_columns(), &delta_cells(label, delta))?;
        }
        self.category_comparison(previous)
    }

    fn category_comparison(&mut self, previous: &ReportPrevious) -> Result<(), String> {
        self.block_heading("Rozdiel výdavkov po kategóriách")?;
        if previous.categories.is_empty() {
            return self.note("Bez výdavkových kategórií.");
        }
        self.grid_header(
            &five_columns(),
            &["Kategória", "Teraz", "Predtým", "Rozdiel", "Zmena"],
        )?;
        for item in previous.categories.iter().take(DELTA_ROWS) {
            self.grid_row(&five_columns(), &category_delta_cells(item))?;
        }
        let hidden = previous.categories.len().saturating_sub(DELTA_ROWS);
        if hidden > 0 {
            self.omitted(&format!("a ďalších {hidden} kategórií"))?;
        }
        Ok(())
    }

    fn all_categories(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        self.block_heading("Všetky kategórie výdavkov")?;
        let snapshot = &summary.snapshot;
        if snapshot.categories.is_empty() {
            return self.note("Bez výdavkových kategórií.");
        }
        let mut sorted = snapshot.categories.iter().collect::<Vec<_>>();
        sorted.sort_by(|left, right| compare_category(left, right));
        let columns = [LABEL, FIGURES[0], FIGURES[1]];
        self.grid_header(&columns, &["Kategória", "Suma", "Podiel"])?;
        let total = snapshot.totals.expense_cents;
        for item in sorted.iter().take(CATEGORY_ROWS) {
            self.grid_row(
                &columns,
                &[
                    item.label.clone(),
                    format_money(item.cents),
                    share_text(item.cents, total),
                ],
            )?;
        }
        let hidden = &sorted[sorted.len().min(CATEGORY_ROWS)..];
        if !hidden.is_empty() {
            let sum = hidden
                .iter()
                .map(|item| i128::from(item.cents))
                .sum::<i128>();
            let sum = i64::try_from(sum)
                .map_err(|_| "Súčet skrytých kategórií pretiekol.".to_string())?;
            self.omitted(&format!(
                "a ďalších {} kategórií, spolu {}",
                hidden.len(),
                format_money(sum)
            ))?;
        }
        Ok(())
    }

    pub(super) fn summary_detail_page(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        self.page_header(
            summary,
            "Abakus · Sumár do 3 strán · účty, vývoj a transakcie",
        )?;
        self.account_totals_block(summary)?;
        self.month_table_block(summary)?;
        self.largest_block(summary)?;
        self.recurring_block(summary)?;
        self.page_guard()
    }

    fn account_totals_block(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        self.block_heading("Súčty po účtoch")?;
        let accounts = &summary.extras.account_totals;
        if accounts.is_empty() {
            return self.note("V zvolenom rozsahu nie je žiadny účet.");
        }
        self.grid_header(
            &five_columns(),
            &["Účet", "Príjmy", "Výdavky", "Rozdiel", "Transakcie"],
        )?;
        for item in accounts.iter().take(ACCOUNT_ROWS) {
            self.grid_row(
                &five_columns(),
                &[
                    item.label.clone(),
                    format_money(item.income_cents),
                    format_money(item.expense_cents),
                    format_money(item.net_cents),
                    item.transaction_count.to_string(),
                ],
            )?;
        }
        let hidden = accounts.len().saturating_sub(ACCOUNT_ROWS);
        if hidden > 0 {
            self.omitted(&format!("a ďalších {hidden} účtov"))?;
        }
        Ok(())
    }

    fn month_table_block(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        self.block_heading("Vývoj po mesiacoch")?;
        let months = &summary.extras.month_rows;
        if months.is_empty() {
            return self.note("Bez transakcií. Tabuľka nemá žiadny mesiac.");
        }
        self.grid_header(
            &five_columns(),
            &["Mesiac", "Príjmy", "Výdavky", "Rozdiel", "Kumulatívne"],
        )?;
        let hidden = months.len().saturating_sub(MONTH_ROWS);
        for item in &months[hidden..] {
            self.grid_row(
                &five_columns(),
                &[
                    item.month.clone(),
                    format_money(item.income_cents),
                    format_money(item.expense_cents),
                    format_money(item.net_cents),
                    format_money(item.cumulative_net_cents),
                ],
            )?;
        }
        if hidden > 0 {
            self.omitted(&format!(
                "a ďalších {hidden} skorších mesiacov. Kumulatívny rozdiel ich zahŕňa."
            ))?;
        }
        Ok(())
    }

    fn largest_block(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        self.block_heading("10 najväčších transakcií")?;
        let largest = &summary.extras.largest;
        if largest.is_empty() {
            return self.note("Bez transakcií.");
        }
        let columns = [
            left(MARGIN, 56.0),
            left(98.0, 82.0),
            left(184.0, 160.0),
            left(348.0, 110.0),
            right(PAGE_WIDTH - MARGIN, 86.0),
        ];
        self.grid_header(&columns, &["Dátum", "Účet", "Popis", "Kategória", "Suma"])?;
        for item in largest {
            self.grid_row(&columns, &largest_cells(item))?;
        }
        Ok(())
    }

    fn recurring_block(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        let recurring = &summary.extras.recurring;
        self.block_heading("Pravidelné platby (odhady)")?;
        self.note("Odhady z importov. Chýbajúce pokrytie nepreukazuje vynechanú platbu.")?;
        self.note(&format!(
            "Potvrdené mesačne: príjmy {}, výdavky {} · Odhady mesačne: príjmy {}, výdavky {}",
            format_money(recurring.confirmed.monthly_income_cents),
            format_money(recurring.confirmed.monthly_expense_cents),
            format_money(recurring.estimates.monthly_income_cents),
            format_money(recurring.estimates.monthly_expense_cents),
        ))?;
        if recurring.rows.is_empty() {
            return self.note("Bez pravidelných platieb.");
        }
        let columns = [
            left(MARGIN, 180.0),
            left(226.0, 100.0),
            left(330.0, 60.0),
            right(460.0, 66.0),
            right(PAGE_WIDTH - MARGIN, 85.0),
        ];
        self.grid_header(
            &columns,
            &["Platba", "Účet", "Stav", "Mesačne", "Ďalšia splatnosť"],
        )?;
        for row in recurring.rows.iter().take(RECURRING_ROWS) {
            self.grid_row(&columns, &recurring_cells(row))?;
        }
        let hidden = recurring.rows.len().saturating_sub(RECURRING_ROWS);
        if hidden > 0 {
            self.omitted(&format!("a ďalších {hidden} platieb"))?;
        }
        Ok(())
    }
}

fn delta_cells(label: &str, delta: store::report::ReportDelta) -> [String; 5] {
    [
        label.to_owned(),
        format_money(delta.current_cents),
        format_money(delta.previous_cents),
        format_signed_money(delta.delta_cents),
        change_text(delta.percent_tenths),
    ]
}

fn category_delta_cells(item: &ReportCategoryDelta) -> [String; 5] {
    delta_cells(&item.label, item.delta)
}

fn largest_cells(item: &ReportTransaction) -> [String; 5] {
    let tx = &item.transaction;
    let description = description(item);
    [
        date(tx.tx_date),
        item.account_label.clone(),
        description.lines().next().unwrap_or_default().to_owned(),
        category_label(item),
        format_money(tx.amount_cents),
    ]
}

fn recurring_cells(row: &RecurringRow) -> [String; 5] {
    let state = if row.decision == RowDecision::Confirmed {
        "Potvrdené"
    } else {
        "Odhad"
    };
    let monthly = row.monthly_cents.map_or_else(
        || UNAVAILABLE.to_string(),
        |cents| match row.direction {
            RecurringDirection::Expense => format_money(-cents),
            RecurringDirection::Income => format_signed_money(cents),
        },
    );
    [
        row.name.clone(),
        row.account_label.clone(),
        state.to_owned(),
        monthly,
        row.next_due.map_or_else(|| UNAVAILABLE.to_string(), date),
    ]
}
