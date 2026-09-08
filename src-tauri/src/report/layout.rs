use super::font::{self, FontKind};
use rules::Status;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use store::report::{
    ReportCategoryTotal, ReportCoverage, ReportMonth, ReportSnapshot, ReportTransaction,
};

pub(crate) const PAGE_WIDTH: f32 = 595.28;
pub(crate) const PAGE_HEIGHT: f32 = 841.89;
const MARGIN: f32 = 42.0;
const CONTENT_BOTTOM: f32 = 56.0;
const BODY_SIZE: f32 = 9.0;
const LINE_HEIGHT: f32 = 11.5;
const MAX_PAGES: usize = 2_000;
const MAX_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfReportLimits {
    pub max_pages: usize,
    pub max_bytes: u64,
}

impl Default for PdfReportLimits {
    fn default() -> Self {
        Self {
            max_pages: MAX_PAGES,
            max_bytes: MAX_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Color(pub f32, pub f32, pub f32);

pub(crate) const INK: Color = Color(0.13, 0.14, 0.16);
pub(crate) const MUTED: Color = Color(0.36, 0.38, 0.42);
pub(crate) const BORDER: Color = Color(0.77, 0.79, 0.82);
pub(crate) const SOFT: Color = Color(0.94, 0.97, 0.94);
pub(crate) const ACCENT: Color = Color(0.10, 0.43, 0.22);
pub(crate) const BLUE: Color = Color(0.15, 0.35, 0.67);
pub(crate) const DANGER: Color = Color(0.70, 0.22, 0.16);

#[derive(Debug)]
pub(crate) enum DrawOp {
    Text {
        x: f32,
        y: f32,
        size: f32,
        font: FontKind,
        color: Color,
        text: String,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        color: Color,
    },
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill: Option<Color>,
        stroke: Option<Color>,
    },
}

#[derive(Debug, Default)]
pub(crate) struct Page {
    pub ops: Vec<DrawOp>,
}

#[derive(Debug)]
pub(crate) struct RenderedReport {
    pub pages: Vec<Page>,
}

pub(crate) fn layout(
    snapshot: &ReportSnapshot,
    limits: PdfReportLimits,
) -> Result<RenderedReport, String> {
    let mut document = Layout::new(limits.max_pages)?;
    document.summary(snapshot)?;
    document.transactions(snapshot)?;
    document.coverage(snapshot)?;
    document.replacement_note()?;
    document.footers()?;
    Ok(RenderedReport {
        pages: document.pages,
    })
}

struct Layout {
    pages: Vec<Page>,
    cursor: f32,
    max_pages: usize,
    replacements: u32,
    continuation_context: String,
}

impl Layout {
    fn new(max_pages: usize) -> Result<Self, String> {
        if max_pages == 0 {
            return Err(page_limit_message(max_pages));
        }
        Ok(Self {
            pages: vec![Page::default()],
            cursor: MARGIN,
            max_pages,
            replacements: 0,
            continuation_context: String::new(),
        })
    }

    fn summary(&mut self, snapshot: &ReportSnapshot) -> Result<(), String> {
        self.continuation_context = range_context(snapshot);
        self.text_line(
            "Abakus · Prehľad transakcií",
            MARGIN,
            20.0,
            FontKind::Bold,
            INK,
        )?;
        self.cursor += 4.0;
        self.paragraph(
            &range_context(snapshot),
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            10.5,
            FontKind::Bold,
            INK,
        )?;
        self.paragraph(
            &format!("Údaje zachytené {}", snapshot.preview.captured_at),
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            BODY_SIZE,
            FontKind::Regular,
            MUTED,
        )?;
        self.paragraph(
            "Všetky transakcie vo zvolenom období a účtoch. Filtre tabuľky sa nepoužijú.",
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            BODY_SIZE,
            FontKind::Regular,
            MUTED,
        )?;
        if snapshot.preview.unfinished_period {
            self.paragraph(
                "Obdobie ešte neskončilo.",
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                BODY_SIZE,
                FontKind::Bold,
                DANGER,
            )?;
        }
        self.account_summary(snapshot)?;
        self.coverage_summary(snapshot)?;
        self.kpis(snapshot)?;
        self.month_charts(&snapshot.months, &snapshot.transactions)?;
        self.category_chart(&snapshot.categories, snapshot.totals.expense_cents)?;
        Ok(())
    }

    fn coverage_summary(&mut self, snapshot: &ReportSnapshot) -> Result<(), String> {
        self.section("Stav pokrytia")?;
        self.paragraph(
            &format!(
                "Účty bez výpisov: {}. Účty s medzerami: {}. Výpisy s odchýlkou alebo neoveriteľným kontrolným súčtom: {}. Neplatné rozsahy výpisov: {}.",
                snapshot.preview.accounts_without_statements,
                snapshot.preview.accounts_with_gaps,
                snapshot.preview.unverified_statement_count,
                snapshot.preview.invalid_statement_range_count,
            ),
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            BODY_SIZE,
            FontKind::Regular,
            MUTED,
        )
    }

    fn account_summary(&mut self, snapshot: &ReportSnapshot) -> Result<(), String> {
        self.section("Účty")?;
        self.paragraph(
            &format!(
                "{} · Počet účtov: {}",
                snapshot.preview.scope_label,
                snapshot.preview.accounts.len()
            ),
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            BODY_SIZE,
            FontKind::Bold,
            INK,
        )?;
        if snapshot.preview.accounts.is_empty() {
            return self.paragraph(
                "V zvolenom rozsahu nie je žiadny účet.",
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                BODY_SIZE,
                FontKind::Regular,
                MUTED,
            );
        }
        for account in &snapshot.preview.accounts {
            let kind = if account.kind == parser::AccountKind::Business {
                "firemný"
            } else {
                "osobný"
            };
            self.paragraph(
                &format!(
                    "{} · {kind} · IBAN •••• {}",
                    account.label, account.iban_suffix
                ),
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                BODY_SIZE,
                FontKind::Regular,
                INK,
            )?;
        }
        Ok(())
    }

    fn kpis(&mut self, snapshot: &ReportSnapshot) -> Result<(), String> {
        self.section("Súhrn zaznamenaných transakcií")?;
        let items = [
            ("Príjmy", format_money(snapshot.totals.income_cents)),
            ("Výdavky", format_money(snapshot.totals.expense_cents)),
            (
                "Rozdiel príjmov a výdavkov",
                format_money(snapshot.totals.net_cents),
            ),
            (
                "Odchádzajúce prevody",
                format_money(snapshot.totals.transfer_out_cents),
            ),
            (
                "Prichádzajúce prevody",
                format_money(snapshot.totals.transfer_in_cents),
            ),
            ("Transakcie", snapshot.preview.transaction_count.to_string()),
            ("Navrhnuté", snapshot.totals.suggested_count.to_string()),
            ("Nezaradené", snapshot.totals.unassigned_count.to_string()),
        ];
        for pair in items.chunks(2) {
            self.ensure_space(34.0, false)?;
            for (index, (label, value)) in pair.iter().enumerate() {
                let x = MARGIN + index as f32 * 255.0;
                self.rect(x, self.cursor, 242.0, 29.0, Some(SOFT), Some(BORDER));
                self.draw_text(
                    label,
                    x + 8.0,
                    self.cursor + 9.0,
                    8.0,
                    FontKind::Regular,
                    MUTED,
                )?;
                self.draw_text(
                    value,
                    x + 8.0,
                    self.cursor + 22.0,
                    11.0,
                    FontKind::Bold,
                    INK,
                )?;
            }
            self.cursor += 35.0;
        }
        Ok(())
    }

    fn month_charts(
        &mut self,
        months: &[ReportMonth],
        transactions: &[ReportTransaction],
    ) -> Result<(), String> {
        self.section("Príjmy a výdavky podľa obdobia")?;
        if months.is_empty() {
            return self.paragraph(
                "Bez transakcií. Graf nemá žiadne obdobie.",
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                BODY_SIZE,
                FontKind::Regular,
                MUTED,
            );
        }
        let buckets = chart_buckets(months, transactions)?;
        for (panel_index, panel) in buckets.chunks(24).enumerate() {
            if buckets.len() > 24 {
                self.paragraph(
                    &format!("Panel {} · ročné súčty", panel_index + 1),
                    MARGIN,
                    PAGE_WIDTH - 2.0 * MARGIN,
                    BODY_SIZE,
                    FontKind::Bold,
                    MUTED,
                )?;
            }
            self.bar_panel(panel)?;
        }
        Ok(())
    }

    fn bar_panel(&mut self, buckets: &[ChartBucket]) -> Result<(), String> {
        let key_lines = buckets.len();
        self.ensure_space(120.0 + key_lines as f32 * LINE_HEIGHT, true)?;
        let chart_top = self.cursor;
        let zero_y = chart_top + 42.0;
        self.line(MARGIN, zero_y, PAGE_WIDTH - MARGIN, zero_y, 0.8, MUTED);
        let max_abs = buckets
            .iter()
            .flat_map(|item| [item.income, item.expense])
            .map(i64::unsigned_abs)
            .max()
            .unwrap_or(1)
            .max(1) as f64;
        let slot = (PAGE_WIDTH - 2.0 * MARGIN) / buckets.len().max(1) as f32;
        for (index, item) in buckets.iter().enumerate() {
            let x = MARGIN + index as f32 * slot + slot * 0.18;
            let width = (slot * 0.27).max(2.0);
            self.bar(x, zero_y, width, item.income, max_abs, ACCENT);
            self.bar(x + width + 2.0, zero_y, width, item.expense, max_abs, BLUE);
        }
        self.cursor += 88.0;
        self.paragraph(
            "Zelená: príjmy · Modrá: výdavky · os zobrazuje nulu",
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            8.0,
            FontKind::Regular,
            MUTED,
        )?;
        for item in buckets {
            self.paragraph(
                &format!(
                    "{}: príjmy {}, výdavky {} · zaznamenané transakcie: {}",
                    item.label,
                    format_money(item.income),
                    format_money(item.expense),
                    item.transaction_count,
                ),
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                8.5,
                FontKind::Regular,
                INK,
            )?;
        }
        Ok(())
    }

    fn bar(&mut self, x: f32, zero_y: f32, width: f32, cents: i64, maximum: f64, color: Color) {
        let signed = cents as f64 / maximum;
        let height = (signed.abs() * 36.0) as f32;
        let top = if cents >= 0 { zero_y - height } else { zero_y };
        self.rect(x, top, width, height.max(0.8), Some(color), None);
    }

    fn category_chart(
        &mut self,
        categories: &[ReportCategoryTotal],
        expense_total: i64,
    ) -> Result<(), String> {
        self.section("Výdavky podľa kategórie")?;
        if categories.is_empty() {
            return self.paragraph(
                "Bez výdavkových kategórií.",
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                BODY_SIZE,
                FontKind::Regular,
                MUTED,
            );
        }
        let grouped = grouped_categories(categories)?;
        let maximum = grouped
            .iter()
            .map(|item| item.cents.unsigned_abs())
            .max()
            .unwrap_or(1)
            .max(1) as f64;
        for item in &grouped {
            self.ensure_space(29.0, false)?;
            self.paragraph(
                &format!("{} · {}", item.label, format_money(item.cents)),
                MARGIN,
                330.0,
                8.5,
                FontKind::Regular,
                INK,
            )?;
            let x = 390.0;
            let width = (item.cents.unsigned_abs() as f64 / maximum * 150.0) as f32;
            self.line(
                x,
                self.cursor - 6.0,
                x + 150.0,
                self.cursor - 6.0,
                0.5,
                BORDER,
            );
            self.rect(
                x,
                self.cursor - 11.0,
                width.max(1.0),
                7.0,
                Some(if item.cents >= 0 { BLUE } else { DANGER }),
                None,
            );
        }
        let total = grouped
            .iter()
            .map(|item| i128::from(item.cents))
            .sum::<i128>();
        let total = i64::try_from(total)
            .map_err(|_| "Súčet zobrazených kategórií pretiekol.".to_string())?;
        self.paragraph(
            &format!(
                "Súčet kategórií: {} · KPI výdavkov: {}",
                format_money(total),
                format_money(expense_total)
            ),
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            8.5,
            FontKind::Bold,
            MUTED,
        )
    }

    fn transactions(&mut self, snapshot: &ReportSnapshot) -> Result<(), String> {
        self.new_page()?;
        self.section("Úplný výpis transakcií")?;
        if snapshot.transactions.is_empty() {
            return self.paragraph(
                "Bez transakcií.",
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                BODY_SIZE,
                FontKind::Regular,
                MUTED,
            );
        }
        self.table_header()?;
        for (index, row) in snapshot.transactions.iter().enumerate() {
            self.transaction_row(index + 1, row)?;
        }
        Ok(())
    }

    fn table_header(&mut self) -> Result<(), String> {
        self.ensure_space(24.0, false)?;
        self.rect(
            MARGIN,
            self.cursor,
            PAGE_WIDTH - 2.0 * MARGIN,
            18.0,
            Some(SOFT),
            Some(BORDER),
        );
        for (x, label) in [
            (MARGIN + 3.0, "Dátum"),
            (130.0, "Účet"),
            (190.0, "Popis"),
            (315.0, "Kategória / stav"),
            (510.0, "Suma EUR"),
        ] {
            self.draw_text(label, x, self.cursor + 12.0, 7.7, FontKind::Bold, MUTED)?;
        }
        self.cursor += 21.0;
        Ok(())
    }

    fn transaction_row(&mut self, ordinal: usize, row: &ReportTransaction) -> Result<(), String> {
        let columns = transaction_columns(ordinal, row);
        let widths = [79.0, 54.0, 119.0, 84.0, 145.0];
        let mut lines = Vec::new();
        for (column, width) in columns.iter().zip(widths) {
            lines.push(self.prepared_lines(column, FontKind::Regular, BODY_SIZE, width)?);
        }
        let line_count = lines.iter().map(Vec::len).max().unwrap_or(1);
        let mut offset = 0_usize;
        let mut first_fragment = true;
        while offset < line_count {
            let continuation_height = if first_fragment { 0.0 } else { LINE_HEIGHT };
            if self.available_height() < LINE_HEIGHT + 7.0 + continuation_height {
                self.table_continuation_page()?;
            }
            if !first_fragment {
                self.draw_text(
                    &format!("Pokračovanie položky {ordinal}"),
                    MARGIN + 3.0,
                    self.cursor + 8.0,
                    7.5,
                    FontKind::Bold,
                    MUTED,
                )?;
                self.cursor += LINE_HEIGHT;
            }
            let available_lines = ((self.available_height() - 7.0) / LINE_HEIGHT)
                .floor()
                .max(1.0) as usize;
            let fragment_lines = available_lines.min(line_count - offset);
            let height = fragment_lines as f32 * LINE_HEIGHT + 6.0;
            self.rect(
                MARGIN,
                self.cursor,
                PAGE_WIDTH - 2.0 * MARGIN,
                height,
                None,
                Some(BORDER),
            );
            self.draw_row_lines(&lines, offset, fragment_lines, self.cursor + 10.0)?;
            self.cursor += height;
            offset += fragment_lines;
            first_fragment = false;
        }
        Ok(())
    }

    fn draw_row_lines(
        &mut self,
        columns: &[Vec<String>],
        offset: usize,
        count: usize,
        top: f32,
    ) -> Result<(), String> {
        let xs = [MARGIN + 3.0, 130.0, 190.0, 315.0, 405.0];
        for (column_index, column) in columns.iter().enumerate() {
            for line_index in 0..count {
                if let Some(line) = column.get(offset + line_index) {
                    let x = if column_index == 4 {
                        PAGE_WIDTH - MARGIN - 3.0 - font::width(line, FontKind::Regular, BODY_SIZE)?
                    } else {
                        xs[column_index]
                    };
                    self.draw_text(
                        line,
                        x,
                        top + line_index as f32 * LINE_HEIGHT,
                        BODY_SIZE,
                        FontKind::Regular,
                        INK,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn table_continuation_page(&mut self) -> Result<(), String> {
        self.new_page()?;
        self.draw_text(
            "Abakus · Prehľad transakcií",
            MARGIN,
            self.cursor + 11.0,
            10.0,
            FontKind::Bold,
            INK,
        )?;
        self.cursor += 17.0;
        self.paragraph(
            &self.continuation_context.clone(),
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            8.0,
            FontKind::Regular,
            MUTED,
        )?;
        self.table_header()
    }

    fn coverage(&mut self, snapshot: &ReportSnapshot) -> Result<(), String> {
        self.new_page()?;
        self.section("Pokrytie výpismi")?;
        self.paragraph("Pokrytie opisuje prítomnosť platných rozsahov výpisov. Kvalita kontrolného súčtu je uvedená osobitne.", MARGIN, PAGE_WIDTH - 2.0 * MARGIN, BODY_SIZE, FontKind::Regular, MUTED)?;
        for item in &snapshot.coverage {
            self.coverage_item(item, snapshot)?;
        }
        Ok(())
    }

    fn coverage_item(
        &mut self,
        item: &ReportCoverage,
        snapshot: &ReportSnapshot,
    ) -> Result<(), String> {
        let label = snapshot
            .preview
            .accounts
            .iter()
            .find(|account| account.id == item.account_id)
            .map(|account| account.label.as_str())
            .unwrap_or("Neznámy účet");
        self.ensure_space(45.0, true)?;
        self.paragraph(
            label,
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            10.0,
            FontKind::Bold,
            INK,
        )?;
        if item.statement_count == 0 {
            return self.paragraph(
                "Bez výpisov.",
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                BODY_SIZE,
                FontKind::Regular,
                DANGER,
            );
        }
        let known = item
            .known_range
            .map(|range| format!("Známy rozsah: {} až {}.", date(range.from), date(range.to)))
            .unwrap_or_else(|| "Bez platného rozsahu výpisu.".into());
        self.paragraph(
            &format!(
                "{known} Výpisy: {}. Neoverené kontrolné súčty: {}. Neplatné rozsahy: {}.",
                item.statement_count, item.unverified_statement_count, item.invalid_range_count
            ),
            MARGIN,
            PAGE_WIDTH - 2.0 * MARGIN,
            BODY_SIZE,
            FontKind::Regular,
            INK,
        )?;
        if item.gaps.is_empty() {
            self.paragraph(
                "Bez medzier v známom rozsahu.",
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                BODY_SIZE,
                FontKind::Regular,
                MUTED,
            )?;
        } else {
            for gap in &item.gaps {
                self.paragraph(
                    &format!("Medzera: {} až {}", date(gap.from), date(gap.to)),
                    MARGIN + 10.0,
                    PAGE_WIDTH - 2.0 * MARGIN - 10.0,
                    BODY_SIZE,
                    FontKind::Regular,
                    DANGER,
                )?;
            }
        }
        Ok(())
    }

    fn replacement_note(&mut self) -> Result<(), String> {
        if self.replacements == 0 {
            return Ok(());
        }
        self.section("Poznámka k znakom")?;
        self.paragraph(&format!("{0} nepodporovaných alebo riadiacich znakov bolo zapísaných viditeľne v tvare [U+XXXX].", self.replacements), MARGIN, PAGE_WIDTH - 2.0 * MARGIN, BODY_SIZE, FontKind::Regular, DANGER)
    }

    fn footers(&mut self) -> Result<(), String> {
        let count = self.pages.len();
        for index in 0..count {
            let text = format!("Strana {} z {}", index + 1, count);
            let prepared = font::prepare(&text, FontKind::Regular)?;
            let width = font::width(&prepared.text, FontKind::Regular, 8.0)?;
            self.pages[index].ops.push(DrawOp::Line {
                x1: MARGIN,
                y1: 45.0,
                x2: PAGE_WIDTH - MARGIN,
                y2: 45.0,
                width: 0.5,
                color: BORDER,
            });
            self.pages[index].ops.push(DrawOp::Text {
                x: PAGE_WIDTH - MARGIN - width,
                y: 31.0,
                size: 8.0,
                font: FontKind::Regular,
                color: MUTED,
                text: prepared.text,
            });
        }
        Ok(())
    }

    fn section(&mut self, title: &str) -> Result<(), String> {
        self.ensure_space(31.0, true)?;
        self.cursor += 8.0;
        self.text_line(title, MARGIN, 12.0, FontKind::Bold, INK)?;
        self.line(
            MARGIN,
            self.cursor + 1.0,
            PAGE_WIDTH - MARGIN,
            self.cursor + 1.0,
            0.7,
            BORDER,
        );
        self.cursor += 4.0;
        Ok(())
    }

    fn paragraph(
        &mut self,
        text: &str,
        x: f32,
        width: f32,
        size: f32,
        kind: FontKind,
        color: Color,
    ) -> Result<(), String> {
        let lines = self.prepared_lines(text, kind, size, width)?;
        let step = size * 1.3;
        for line in lines {
            self.ensure_space(step, true)?;
            self.draw_prepared_text(line, x, self.cursor + size, size, kind, color);
            self.cursor += step;
        }
        Ok(())
    }

    fn text_line(
        &mut self,
        text: &str,
        x: f32,
        size: f32,
        kind: FontKind,
        color: Color,
    ) -> Result<(), String> {
        self.ensure_space(size * 1.4, true)?;
        self.draw_text(text, x, self.cursor + size, size, kind, color)?;
        self.cursor += size * 1.4;
        Ok(())
    }

    fn prepared_lines(
        &mut self,
        text: &str,
        kind: FontKind,
        size: f32,
        width: f32,
    ) -> Result<Vec<String>, String> {
        let prepared = font::prepare(text, kind)?;
        self.replacements = self
            .replacements
            .checked_add(prepared.replacements)
            .ok_or_else(|| "Počet nahradených znakov pretiekol.".to_string())?;
        wrap(&prepared.text, kind, size, width)
    }

    fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        baseline_from_top: f32,
        size: f32,
        kind: FontKind,
        color: Color,
    ) -> Result<(), String> {
        let prepared = font::prepare(text, kind)?;
        self.replacements = self
            .replacements
            .checked_add(prepared.replacements)
            .ok_or_else(|| "Počet nahradených znakov pretiekol.".to_string())?;
        self.draw_prepared_text(prepared.text, x, baseline_from_top, size, kind, color);
        Ok(())
    }

    fn draw_prepared_text(
        &mut self,
        text: String,
        x: f32,
        baseline_from_top: f32,
        size: f32,
        font: FontKind,
        color: Color,
    ) {
        let y = PAGE_HEIGHT - baseline_from_top;
        self.page_mut().ops.push(DrawOp::Text {
            x,
            y,
            size,
            font,
            color,
            text,
        });
    }

    fn ensure_space(&mut self, height: f32, continuation_header: bool) -> Result<(), String> {
        if self.available_height() >= height {
            return Ok(());
        }
        self.new_page()?;
        if continuation_header {
            self.draw_text(
                "Abakus · Prehľad transakcií",
                MARGIN,
                self.cursor + 11.0,
                10.0,
                FontKind::Bold,
                INK,
            )?;
            self.cursor += 17.0;
            self.paragraph(
                &self.continuation_context.clone(),
                MARGIN,
                PAGE_WIDTH - 2.0 * MARGIN,
                8.0,
                FontKind::Regular,
                MUTED,
            )?;
        }
        Ok(())
    }

    fn available_height(&self) -> f32 {
        PAGE_HEIGHT - CONTENT_BOTTOM - self.cursor
    }

    fn new_page(&mut self) -> Result<(), String> {
        if self.pages.len() >= self.max_pages {
            return Err(page_limit_message(self.max_pages));
        }
        self.pages.push(Page::default());
        self.cursor = MARGIN;
        Ok(())
    }

    fn line(&mut self, x1: f32, top1: f32, x2: f32, top2: f32, width: f32, color: Color) {
        self.page_mut().ops.push(DrawOp::Line {
            x1,
            y1: PAGE_HEIGHT - top1,
            x2,
            y2: PAGE_HEIGHT - top2,
            width,
            color,
        });
    }

    fn rect(
        &mut self,
        x: f32,
        top: f32,
        width: f32,
        height: f32,
        fill: Option<Color>,
        stroke: Option<Color>,
    ) {
        self.page_mut().ops.push(DrawOp::Rect {
            x,
            y: PAGE_HEIGHT - top - height,
            width,
            height,
            fill,
            stroke,
        });
    }

    fn page_mut(&mut self) -> &mut Page {
        self.pages.last_mut().expect("layout always has one page")
    }
}

fn page_limit_message(limit: usize) -> String {
    format!("Report prekročil limit {limit} strán. Zvoľte kratšie obdobie alebo menej účtov.")
}

fn wrap(text: &str, kind: FontKind, size: f32, maximum: f32) -> Result<Vec<String>, String> {
    let face = font::face(kind)?;
    let units = f32::from(face.units_per_em());
    let mut lines = Vec::new();
    for source_line in text.split('\n') {
        if source_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        lines.extend(wrap_source_line(source_line, &face, units, size, maximum)?);
    }
    Ok(lines)
}

fn wrap_source_line(
    source: &str,
    face: &ttf_parser::Face<'_>,
    units: f32,
    size: f32,
    maximum: f32,
) -> Result<Vec<String>, String> {
    let space_width = scalar_width(face, ' ', units, size)?;
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut line_width = 0.0_f32;
    for word in source.split_whitespace() {
        let word_width = measured_width(word, face, units, size)?;
        if !line.is_empty() && line_width + space_width + word_width <= maximum {
            line.push(' ');
            line.push_str(word);
            line_width += space_width + word_width;
        } else {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            let mut fragments = hard_wrap(word, face, units, size, maximum)?;
            line = fragments.pop().unwrap_or_default();
            lines.extend(fragments);
            line_width = measured_width(&line, face, units, size)?;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    Ok(lines)
}

fn hard_wrap(
    word: &str,
    face: &ttf_parser::Face<'_>,
    units: f32,
    size: f32,
    maximum: f32,
) -> Result<Vec<String>, String> {
    let mut fragments = Vec::new();
    let mut fragment = String::new();
    let mut width = 0.0_f32;
    for scalar in word.chars() {
        let next_width = scalar_width(face, scalar, units, size)?;
        if !fragment.is_empty() && width + next_width > maximum {
            fragments.push(std::mem::take(&mut fragment));
            width = 0.0;
        }
        fragment.push(scalar);
        width += next_width;
    }
    if !fragment.is_empty() {
        fragments.push(fragment);
    }
    Ok(fragments)
}

fn measured_width(
    text: &str,
    face: &ttf_parser::Face<'_>,
    units: f32,
    size: f32,
) -> Result<f32, String> {
    text.chars()
        .map(|scalar| scalar_width(face, scalar, units, size))
        .try_fold(0.0_f32, |total, width| Ok(total + width?))
}

fn scalar_width(
    face: &ttf_parser::Face<'_>,
    scalar: char,
    units: f32,
    size: f32,
) -> Result<f32, String> {
    let glyph = face.glyph_index(scalar).ok_or_else(|| {
        format!(
            "Font neobsahuje vykresľovaný znak U+{:04X}.",
            u32::from(scalar)
        )
    })?;
    Ok(f32::from(face.glyph_hor_advance(glyph).unwrap_or(0)) * size / units)
}

fn range_context(snapshot: &ReportSnapshot) -> String {
    match snapshot.preview.range {
        Some(range) => format!(
            "Obdobie {} až {} · {} · najnovšia transakcia {}",
            date(range.from),
            date(range.to),
            snapshot.preview.scope_label,
            snapshot
                .preview
                .latest_transaction_date
                .map(date)
                .unwrap_or_else(|| "bez transakcií".into())
        ),
        None => format!(
            "Celé obdobie · {} · Bez transakcií",
            snapshot.preview.scope_label
        ),
    }
}

fn date(value: chrono::NaiveDate) -> String {
    value.format("%d.%m.%Y").to_string()
}

/// Formats cents as a signed, space-grouped, comma-decimal amount without a
/// currency suffix, e.g. `-1 000,00`. Shared by [`format_money`] (which adds
/// ` €`) and [`format_original`] (which the caller suffixes with the foreign
/// currency code), so the two amounts never drift into different locales.
fn format_grouped(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let magnitude = cents.unsigned_abs();
    let digits = magnitude.div_euclid(100).to_string();
    let grouped = digits
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{sign}{grouped},{:02}", magnitude.rem_euclid(100))
}

fn format_money(cents: i64) -> String {
    format!("{} €", format_grouped(cents))
}

#[derive(Debug)]
struct ChartBucket {
    label: String,
    income: i64,
    expense: i64,
    transaction_count: usize,
}

fn chart_buckets(
    months: &[ReportMonth],
    transactions: &[ReportTransaction],
) -> Result<Vec<ChartBucket>, String> {
    if months.len() <= 24 {
        let counts = transaction_counts(transactions, false)?;
        return Ok(months
            .iter()
            .map(|item| ChartBucket {
                label: item.month.clone(),
                income: item.income_cents,
                expense: item.expense_cents,
                transaction_count: counts.get(&item.month).copied().unwrap_or(0),
            })
            .collect());
    }
    let counts = transaction_counts(transactions, true)?;
    let mut years: BTreeMap<String, (i128, i128)> = BTreeMap::new();
    for month in months {
        let year = month
            .month
            .get(0..4)
            .ok_or_else(|| "Mesiac reportu nemá tvar RRRR-MM.".to_string())?;
        let entry = years.entry(year.into()).or_default();
        entry.0 += i128::from(month.income_cents);
        entry.1 += i128::from(month.expense_cents);
    }
    years
        .into_iter()
        .map(|(label, (income, expense))| {
            let transaction_count = counts.get(&label).copied().unwrap_or(0);
            Ok(ChartBucket {
                label,
                income: i64::try_from(income)
                    .map_err(|_| "Ročný súčet príjmov pretiekol.".to_string())?,
                expense: i64::try_from(expense)
                    .map_err(|_| "Ročný súčet výdavkov pretiekol.".to_string())?,
                transaction_count,
            })
        })
        .collect()
}

fn transaction_counts(
    transactions: &[ReportTransaction],
    annual: bool,
) -> Result<BTreeMap<String, usize>, String> {
    let mut counts = BTreeMap::new();
    for row in transactions {
        let key = if annual {
            row.transaction.tx_date.format("%Y").to_string()
        } else {
            row.transaction.tx_date.format("%Y-%m").to_string()
        };
        let entry = counts.entry(key).or_insert(0_usize);
        *entry = entry
            .checked_add(1)
            .ok_or_else(|| "Počet transakcií v grafe pretiekol.".to_string())?;
    }
    Ok(counts)
}

#[derive(Debug)]
struct GroupedCategory {
    label: String,
    cents: i64,
}

fn grouped_categories(categories: &[ReportCategoryTotal]) -> Result<Vec<GroupedCategory>, String> {
    let mut sorted = categories.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| compare_category(left, right));
    let mut result = sorted
        .iter()
        .take(10)
        .map(|item| GroupedCategory {
            label: item.label.clone(),
            cents: item.cents,
        })
        .collect::<Vec<_>>();
    let hidden = sorted.iter().skip(10).collect::<Vec<_>>();
    if !hidden.is_empty() {
        let sum = hidden
            .iter()
            .map(|item| i128::from(item.cents))
            .sum::<i128>();
        let cents =
            i64::try_from(sum).map_err(|_| "Súčet ostatných kategórií pretiekol.".to_string())?;
        result.push(GroupedCategory {
            label: format!("Ostatné ({} kategórií)", hidden.len()),
            cents,
        });
    }
    Ok(result)
}

fn compare_category(left: &ReportCategoryTotal, right: &ReportCategoryTotal) -> Ordering {
    right
        .cents
        .unsigned_abs()
        .cmp(&left.cents.unsigned_abs())
        .then_with(|| {
            left.category_id
                .unwrap_or(i64::MAX)
                .cmp(&right.category_id.unwrap_or(i64::MAX))
        })
        .then_with(|| left.label.cmp(&right.label))
}

fn transaction_columns(ordinal: usize, row: &ReportTransaction) -> [String; 5] {
    let tx = &row.transaction;
    let date_text = if tx.posted_date == tx.tx_date {
        format!("{ordinal}. {}", date(tx.tx_date))
    } else {
        format!(
            "{ordinal}. {}\nZaúčtované: {}",
            date(tx.tx_date),
            date(tx.posted_date)
        )
    };
    let description = description(row);
    let category = format!("{} · {}", category_label(row), status_label(tx.status));
    [
        date_text,
        row.account_label.clone(),
        description,
        category,
        format_money(tx.amount_cents),
    ]
}

fn description(row: &ReportTransaction) -> String {
    let tx = &row.transaction;
    let mut lines = vec![if tx.merchant_raw.trim().is_empty() {
        tx.counterparty_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("Bez popisu")
            .to_owned()
    } else {
        tx.merchant_raw.clone()
    }];
    if let Some(place) = tx.place.as_deref().filter(|place| !place.is_empty()) {
        lines.push(place.to_owned());
    }
    if !tx.note.is_empty() {
        lines.push(format!("Poznámka: {}", tx.note));
    }
    if let (Some(cents), Some(currency)) = (tx.orig_amount_cents, &tx.orig_currency) {
        lines.push(format!(
            "Pôvodná suma: {} {currency}",
            format_original(cents)
        ));
    }
    lines.join("\n")
}

fn category_label(row: &ReportTransaction) -> String {
    match (&row.transaction.parent_name, &row.transaction.category_name) {
        (Some(parent), Some(child)) => format!("{parent} / {child}"),
        (_, Some(category)) => category.clone(),
        _ => "Bez kategórie".into(),
    }
}

fn status_label(status: Status) -> &'static str {
    match status {
        Status::Confirmed => "Potvrdené",
        Status::Suggested => "Navrhnuté",
        Status::Unassigned => "Nezaradené",
        Status::Transfer => "Prevod",
    }
}

fn format_original(cents: i64) -> String {
    format_grouped(cents)
}
