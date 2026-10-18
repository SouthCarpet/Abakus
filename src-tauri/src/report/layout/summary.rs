//! Summary PDFs (plan 084). Fixed geometry: text never wraps and a page never
//! continues on another page. Every table has a row cap and says how many
//! rows it left out. See docs/plan-084-summary-pdf.md.
use super::*;
use crate::report::SummaryLength;
use chrono::Datelike;
use store::report::{ratio_tenths, ReportAccount, ReportTotals, SummarySnapshot};

pub(super) const SUMMARY_TITLE: &str = "Abakus · Sumár";
const CONTENT_WIDTH: f32 = PAGE_WIDTH - 2.0 * MARGIN;
const RIGHT_EDGE: f32 = PAGE_WIDTH - MARGIN;
/// Space kept free at the bottom of the last page for the character note.
const NOTE_RESERVE: f32 = 14.0;
const HEADER_ACCOUNTS: usize = 3;
const TOP_CATEGORIES: usize = 8;
const CHART_BUCKETS: usize = 36;
const UNAVAILABLE: &str = "nedostupné";

pub(crate) fn layout_summary(
    summary: &SummarySnapshot,
    length: SummaryLength,
    limits: PdfReportLimits,
) -> Result<RenderedReport, String> {
    let mut document = Layout::new(limits.max_pages)?;
    document.continuation_context = range_context(&summary.snapshot);
    document.summary_first_page(summary, length)?;
    if length == SummaryLength::ThreePages {
        document.new_page()?;
        document.summary_comparison_page(summary)?;
        document.new_page()?;
        document.summary_detail_page(summary)?;
    }
    document.summary_replacement_note()?;
    document.footers()?;
    Ok(RenderedReport {
        pages: document.pages,
        title: SUMMARY_TITLE,
    })
}

/// One table column. `x` is the left edge, or the right edge when `right`.
#[derive(Clone, Copy)]
pub(super) struct Column {
    pub x: f32,
    pub width: f32,
    pub right: bool,
}

pub(super) const fn left(x: f32, width: f32) -> Column {
    Column {
        x,
        width,
        right: false,
    }
}

pub(super) const fn right(x: f32, width: f32) -> Column {
    Column {
        x,
        width,
        right: true,
    }
}

impl Layout {
    pub(super) fn fit(
        &mut self,
        text: &str,
        kind: FontKind,
        size: f32,
        max_width: f32,
    ) -> Result<String, String> {
        let prepared = font::prepare(text, kind)?;
        self.replacements = self
            .replacements
            .checked_add(prepared.replacements)
            .ok_or_else(|| "Počet nahradených znakov pretiekol.".to_string())?;
        let flat = prepared.text.replace('\n', " ");
        let face = font::face(kind)?;
        let units = f32::from(face.units_per_em());
        fit_width(&flat, &face, units, size, max_width)
    }

    pub(super) fn put(
        &mut self,
        text: &str,
        column: Column,
        size: f32,
        kind: FontKind,
        color: Color,
    ) -> Result<(), String> {
        let fitted = self.fit(text, kind, size, column.width)?;
        let x = if column.right {
            column.x - font::width(&fitted, kind, size)?
        } else {
            column.x
        };
        self.draw_prepared_text(fitted, x, self.cursor + size, size, kind, color);
        Ok(())
    }

    fn put_full(
        &mut self,
        text: &str,
        size: f32,
        kind: FontKind,
        color: Color,
    ) -> Result<(), String> {
        self.put(text, left(MARGIN, CONTENT_WIDTH), size, kind, color)
    }

    /// One muted line at body size, then the cursor moves down.
    pub(super) fn note(&mut self, text: &str) -> Result<(), String> {
        self.put_full(text, 8.5, FontKind::Regular, MUTED)?;
        self.cursor += 11.5;
        Ok(())
    }

    pub(super) fn warning(&mut self, text: &str) -> Result<(), String> {
        self.put_full(text, 8.5, FontKind::Bold, DANGER)?;
        self.cursor += 11.5;
        Ok(())
    }

    pub(super) fn block_heading(&mut self, title: &str) -> Result<(), String> {
        self.cursor += 6.0;
        self.put_full(title, 11.0, FontKind::Bold, INK)?;
        self.line(
            MARGIN,
            self.cursor + 15.0,
            RIGHT_EDGE,
            self.cursor + 15.0,
            0.7,
            BORDER,
        );
        self.cursor += 20.0;
        Ok(())
    }

    pub(super) fn grid_header(
        &mut self,
        columns: &[Column],
        labels: &[&str],
    ) -> Result<(), String> {
        self.rect(
            MARGIN,
            self.cursor,
            CONTENT_WIDTH,
            13.0,
            Some(SOFT),
            Some(BORDER),
        );
        self.cursor += 1.5;
        for (column, label) in columns.iter().zip(labels) {
            let shifted = Column {
                x: if column.right {
                    column.x - 3.0
                } else {
                    column.x + 3.0
                },
                ..*column
            };
            self.put(label, shifted, 7.7, FontKind::Bold, MUTED)?;
        }
        self.cursor += 12.5;
        Ok(())
    }

    pub(super) fn grid_row(&mut self, columns: &[Column], cells: &[String]) -> Result<(), String> {
        for (column, cell) in columns.iter().zip(cells) {
            let shifted = Column {
                x: if column.right {
                    column.x - 3.0
                } else {
                    column.x + 3.0
                },
                ..*column
            };
            self.put(cell, shifted, 8.0, FontKind::Regular, INK)?;
        }
        self.cursor += 12.5;
        Ok(())
    }

    pub(super) fn omitted(&mut self, text: &str) -> Result<(), String> {
        self.put(
            text,
            left(MARGIN + 3.0, CONTENT_WIDTH - 3.0),
            8.0,
            FontKind::Regular,
            MUTED,
        )?;
        self.cursor += 12.5;
        Ok(())
    }

    /// A page is full when the cursor passes the bottom margin or the room
    /// kept for the character note. The row caps keep every page below it.
    pub(super) fn page_guard(&self) -> Result<(), String> {
        if PAGE_HEIGHT - CONTENT_BOTTOM - NOTE_RESERVE < self.cursor {
            return Err("Sumár sa nezmestil na stranu.".into());
        }
        Ok(())
    }

    pub(super) fn page_header(
        &mut self,
        summary: &SummarySnapshot,
        title: &str,
    ) -> Result<(), String> {
        self.put_full(title, 12.0, FontKind::Bold, INK)?;
        self.cursor += 16.0;
        self.put_full(
            &range_context(&summary.snapshot),
            8.5,
            FontKind::Regular,
            MUTED,
        )?;
        self.cursor += 12.0;
        Ok(())
    }

    fn summary_first_page(
        &mut self,
        summary: &SummarySnapshot,
        length: SummaryLength,
    ) -> Result<(), String> {
        let title = match length {
            SummaryLength::OnePage => "Abakus · Sumár na 1 stranu",
            SummaryLength::ThreePages => "Abakus · Sumár do 3 strán",
        };
        self.put_full(title, 17.0, FontKind::Bold, INK)?;
        self.cursor += 22.0;
        self.summary_header(summary)?;
        self.summary_kpis(&summary.snapshot.totals)?;
        self.summary_chart(summary)?;
        self.summary_top_categories(summary)?;
        self.summary_coverage(summary)?;
        self.page_guard()
    }

    fn summary_header(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        let snapshot = &summary.snapshot;
        self.put_full(&range_context(snapshot), 9.5, FontKind::Bold, INK)?;
        self.cursor += 13.0;
        self.note(&format!("Údaje zachytené {}", snapshot.preview.captured_at))?;
        self.summary_accounts(&snapshot.preview.accounts)?;
        if snapshot.preview.unfinished_period {
            self.warning("Obdobie ešte neskončilo. Údaje sa môžu zmeniť.")?;
        }
        if coverage_incomplete(&snapshot.preview) {
            self.warning("Pokrytie výpismi je neúplné. Podrobnosti sú dole.")?;
        }
        self.cursor += 6.0;
        Ok(())
    }

    fn summary_accounts(&mut self, accounts: &[ReportAccount]) -> Result<(), String> {
        if accounts.is_empty() {
            return self.note("V zvolenom rozsahu nie je žiadny účet.");
        }
        let mut parts = Vec::new();
        for account in accounts.iter().take(HEADER_ACCOUNTS) {
            let label = self.fit(&account.label, FontKind::Regular, 8.5, 80.0)?;
            parts.push(format!("{label} •••• {}", account.iban_suffix));
        }
        let mut text = format!("Účty ({}): {}", accounts.len(), parts.join(", "));
        if accounts.len() > HEADER_ACCOUNTS {
            text.push_str(&format!(
                ", a ďalších {} účtov",
                accounts.len() - HEADER_ACCOUNTS
            ));
        }
        self.note(&text)
    }

    fn summary_kpis(&mut self, totals: &ReportTotals) -> Result<(), String> {
        let top = self.cursor;
        let tiles = [
            (0.0, 100.0),
            (106.0, 100.0),
            (212.0, 132.0),
            (350.0, 161.28),
        ];
        for (offset, width) in tiles {
            self.rect(MARGIN + offset, top, width, 40.0, Some(SOFT), Some(BORDER));
        }
        self.kpi_tile(0, top, "Príjmy", totals.income_cents)?;
        self.kpi_tile(1, top, "Výdavky", totals.expense_cents)?;
        self.kpi_tile(2, top, "Rozdiel príjmov a výdavkov", totals.net_cents)?;
        self.transfer_tile(top, totals)?;
        self.cursor = top + 48.0;
        Ok(())
    }

    fn kpi_tile(&mut self, index: usize, top: f32, label: &str, cents: i64) -> Result<(), String> {
        let (offset, width) = [(0.0, 100.0), (106.0, 100.0), (212.0, 132.0)][index];
        self.cursor = top + 3.0;
        self.put(
            label,
            left(MARGIN + offset + 6.0, width - 12.0),
            7.5,
            FontKind::Regular,
            MUTED,
        )?;
        self.cursor = top + 17.0;
        let value = format_money(cents);
        let size = self.largest_fitting(&value, width - 12.0, &[12.0, 10.0, 8.5, 7.0])?;
        self.put(
            &value,
            left(MARGIN + offset + 6.0, width - 12.0),
            size,
            FontKind::Bold,
            INK,
        )
    }

    fn transfer_tile(&mut self, top: f32, totals: &ReportTotals) -> Result<(), String> {
        let (x, width) = (MARGIN + 350.0, 161.28);
        for (index, (label, cents)) in [
            ("Odchádzajúce prevody", totals.transfer_out_cents),
            ("Prichádzajúce prevody", totals.transfer_in_cents),
        ]
        .into_iter()
        .enumerate()
        {
            self.cursor = top + 4.0 + index as f32 * 17.0;
            self.put(label, left(x + 6.0, 92.0), 7.5, FontKind::Regular, MUTED)?;
            self.put(
                &format_money(cents),
                right(x + width - 6.0, width - 108.0),
                9.0,
                FontKind::Bold,
                INK,
            )?;
        }
        Ok(())
    }

    fn largest_fitting(&mut self, text: &str, width: f32, sizes: &[f32]) -> Result<f32, String> {
        for size in sizes {
            if font::width(text, FontKind::Bold, *size)? <= width {
                return Ok(*size);
            }
        }
        Ok(sizes[sizes.len() - 1])
    }

    fn summary_chart(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        let (title, buckets) = chart_series(summary)?;
        self.block_heading(&format!("Príjmy a výdavky po {title}"))?;
        if buckets.is_empty() {
            self.note("Bez transakcií. Graf nemá žiadne obdobie.")?;
            return Ok(());
        }
        let skipped = buckets.len().saturating_sub(CHART_BUCKETS);
        self.chart_bars(&buckets[skipped..])?;
        let mut legend = String::from("Zelená: príjmy · Modrá: výdavky · os zobrazuje nulu");
        if skipped > 0 {
            legend.push_str(&format!(
                " · graf ukazuje posledných {CHART_BUCKETS} z {}",
                buckets.len()
            ));
        }
        self.note(&legend)
    }

    fn chart_bars(&mut self, buckets: &[ChartBucket]) -> Result<(), String> {
        let top = self.cursor;
        let zero_y = top + 60.0;
        self.line(MARGIN, zero_y, RIGHT_EDGE, zero_y, 0.8, MUTED);
        let maximum = buckets
            .iter()
            .flat_map(|item| [item.income, item.expense])
            .map(i64::unsigned_abs)
            .max()
            .unwrap_or(1)
            .max(1) as f64;
        let slot = CONTENT_WIDTH / buckets.len() as f32;
        let width = (slot * 0.3).clamp(1.5, 14.0);
        for (index, item) in buckets.iter().enumerate() {
            let x = MARGIN + index as f32 * slot + (slot - 2.0 * width - 2.0) / 2.0;
            self.chart_bar(x, zero_y, width, item.income, maximum, ACCENT);
            self.chart_bar(x + width + 2.0, zero_y, width, item.expense, maximum, BLUE);
        }
        self.chart_labels(buckets, slot, zero_y + 56.0 + 10.0)?;
        self.cursor = top + 132.0;
        Ok(())
    }

    fn chart_bar(
        &mut self,
        x: f32,
        zero_y: f32,
        width: f32,
        cents: i64,
        maximum: f64,
        color: Color,
    ) {
        let height = (cents as f64 / maximum).abs() as f32 * 56.0;
        let top = if cents >= 0 { zero_y - height } else { zero_y };
        self.rect(x, top, width, height.max(0.8), Some(color), None);
    }

    fn chart_labels(
        &mut self,
        buckets: &[ChartBucket],
        slot: f32,
        baseline: f32,
    ) -> Result<(), String> {
        let widest = buckets
            .iter()
            .map(|item| item.label.chars().count())
            .max()
            .unwrap_or(1) as f32;
        let every = ((widest * 3.6 + 4.0) / slot).ceil().max(1.0) as usize;
        for (index, item) in buckets.iter().enumerate().step_by(every) {
            let label = self.fit(&item.label, FontKind::Regular, 7.0, slot * every as f32)?;
            let width = font::width(&label, FontKind::Regular, 7.0)?;
            let x = MARGIN + index as f32 * slot + (slot - width) / 2.0;
            self.draw_prepared_text(
                label,
                x.max(MARGIN),
                baseline,
                7.0,
                FontKind::Regular,
                MUTED,
            );
        }
        Ok(())
    }

    fn summary_top_categories(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        self.block_heading("Výdavky podľa kategórie")?;
        let snapshot = &summary.snapshot;
        if snapshot.categories.is_empty() {
            return self.note("Bez výdavkových kategórií.");
        }
        let grouped = grouped_categories(&snapshot.categories, TOP_CATEGORIES)?;
        let maximum = grouped
            .iter()
            .map(|item| item.cents.unsigned_abs())
            .max()
            .unwrap_or(1)
            .max(1) as f64;
        let total = snapshot.totals.expense_cents;
        for item in &grouped {
            self.category_row(item, total, maximum)?;
        }
        self.put_full(
            &format!("Spolu výdavky {}", format_money(total)),
            8.5,
            FontKind::Bold,
            MUTED,
        )?;
        self.cursor += 14.0;
        Ok(())
    }

    fn category_row(
        &mut self,
        item: &GroupedCategory,
        total: i64,
        maximum: f64,
    ) -> Result<(), String> {
        self.put(
            &item.label,
            left(MARGIN, 215.0),
            8.5,
            FontKind::Regular,
            INK,
        )?;
        self.put(
            &format_money(item.cents),
            right(350.0, 85.0),
            8.5,
            FontKind::Regular,
            INK,
        )?;
        self.put(
            &share_text(item.cents, total),
            right(402.0, 48.0),
            8.5,
            FontKind::Regular,
            MUTED,
        )?;
        let width = (item.cents.unsigned_abs() as f64 / maximum * 140.0) as f32;
        let color = if item.cents >= 0 { BLUE } else { DANGER };
        self.rect(
            RIGHT_EDGE - 140.0,
            self.cursor + 1.5,
            width.max(1.0),
            7.0,
            Some(color),
            None,
        );
        self.cursor += 14.5;
        Ok(())
    }

    fn summary_coverage(&mut self, summary: &SummarySnapshot) -> Result<(), String> {
        let snapshot = &summary.snapshot;
        let preview = &snapshot.preview;
        let statements: u64 = snapshot
            .coverage
            .iter()
            .map(|item| u64::from(item.statement_count))
            .sum();
        let gaps: usize = snapshot.coverage.iter().map(|item| item.gaps.len()).sum();
        let counts = &summary.extras.status_counts;
        self.block_heading("Pokrytie výpismi a stav transakcií")?;
        self.note(&format!(
            "Výpisy v období: {statements}. Medzery v pokrytí: {gaps}. Účty bez výpisov: {}. Účty s medzerami: {}.",
            preview.accounts_without_statements, preview.accounts_with_gaps
        ))?;
        self.note(&format!(
            "Neoverené alebo nesediace kontrolné súčty: {}. Neplatné rozsahy výpisov: {}.",
            preview.unverified_statement_count, preview.invalid_statement_range_count
        ))?;
        self.note(&format!(
            "Transakcie: {} · potvrdené {} · navrhnuté {} · nezaradené {} · prevody {}",
            preview.transaction_count,
            counts.confirmed,
            counts.suggested,
            counts.unassigned,
            counts.transfer
        ))?;
        self.note("Súčty vychádzajú len zo zaznamenaných transakcií. Pokrytie výpismi nedokazuje, že appka má všetky transakcie.")
    }

    fn summary_replacement_note(&mut self) -> Result<(), String> {
        if self.replacements == 0 {
            return Ok(());
        }
        self.cursor = PAGE_HEIGHT - CONTENT_BOTTOM - NOTE_RESERVE + 2.0;
        let text = format!(
            "Poznámka k znakom: {} nepodporovaných alebo riadiacich znakov bolo zapísaných viditeľne v tvare [U+XXXX].",
            self.replacements
        );
        self.put_full(&text, 7.5, FontKind::Regular, DANGER)
    }
}

fn coverage_incomplete(preview: &store::report::ReportPreview) -> bool {
    preview.accounts_without_statements > 0
        || preview.accounts_with_gaps > 0
        || preview.unverified_statement_count > 0
        || preview.invalid_statement_range_count > 0
}

/// Daily buckets for a single month, otherwise the buckets of the full report.
fn chart_series(summary: &SummarySnapshot) -> Result<(&'static str, Vec<ChartBucket>), String> {
    if !summary.extras.days.is_empty() {
        let buckets = summary
            .extras
            .days
            .iter()
            .map(|item| ChartBucket {
                label: item.date.day().to_string(),
                income: item.income_cents,
                expense: item.expense_cents,
                transaction_count: 0,
            })
            .collect();
        return Ok(("dňoch", buckets));
    }
    let snapshot = &summary.snapshot;
    let buckets = chart_buckets(&snapshot.months, &snapshot.transactions)?;
    let title = if snapshot.months.len() > 24 {
        "rokoch"
    } else {
        "mesiacoch"
    };
    Ok((title, buckets))
}

fn fit_width(
    text: &str,
    face: &ttf_parser::Face<'_>,
    units: f32,
    size: f32,
    maximum: f32,
) -> Result<String, String> {
    if measured_width(text, face, units, size)? <= maximum {
        return Ok(text.to_owned());
    }
    let ellipsis = scalar_width(face, '…', units, size)?;
    let mut kept = String::new();
    let mut width = 0.0_f32;
    for scalar in text.chars() {
        let next = scalar_width(face, scalar, units, size)?;
        if width + next + ellipsis > maximum {
            break;
        }
        kept.push(scalar);
        width += next;
    }
    kept.push('…');
    Ok(kept)
}

pub(super) fn format_signed_money(cents: i64) -> String {
    if cents > 0 {
        format!("+{}", format_money(cents))
    } else {
        format_money(cents)
    }
}

fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    digits
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Tenths of a percent as `45,8 %`; `signed` adds `+` to a positive value.
pub(super) fn format_tenths(tenths: i64, signed: bool) -> String {
    let sign = if tenths < 0 {
        "-"
    } else if signed && tenths > 0 {
        "+"
    } else {
        ""
    };
    let magnitude = tenths.unsigned_abs();
    format!(
        "{sign}{},{} %",
        group_thousands(magnitude / 10),
        magnitude % 10
    )
}

pub(super) fn change_text(percent_tenths: Option<i64>) -> String {
    percent_tenths.map_or_else(
        || UNAVAILABLE.to_string(),
        |value| format_tenths(value, true),
    )
}

/// Share of the expense total. Not available when the total is not positive.
pub(super) fn share_text(cents: i64, total: i64) -> String {
    if total <= 0 {
        return UNAVAILABLE.into();
    }
    ratio_tenths(i128::from(cents), i128::from(total)).map_or_else(
        || UNAVAILABLE.to_string(),
        |value| format_tenths(value, false),
    )
}
