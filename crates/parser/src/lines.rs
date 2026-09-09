#[derive(Debug, Clone, PartialEq)]
pub struct Char {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub ch: char,
    /// PDFium's text origin x (distinct from `x`, the loose-bounds left edge): on the generator
    /// that leaves loose and tight bounds equal, this is the only coordinate whose consecutive
    /// deltas hold a constant monospace pitch.
    pub origin_x: f32,
    /// Scaled font size (`0.0` when the caller does not know it; row grouping then falls back to `h / 2`).
    pub size: f32,
    /// The loose-bounds width, kept only when it is a real advance box (loose bounds measurably
    /// wider than tight/ink bounds). `None` means the generator reports identical loose and
    /// tight bounds for this glyph, so its width says nothing about spacing.
    pub advance: Option<f32>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Line { pub x: f32, pub y: f32, pub text: String }

/// A real `' '` codepoint with a degenerate zero box (`w`, `h` both effectively 0): PDFium
/// synthesises these on the generator behind the 2026-06-30 Tatra banka statement, and they sit
/// at whatever x PDFium put them, never between the words they are supposed to separate
/// (`abakus-cli geometry`: e.g. index 6 at x=137.68, past the whole word it follows). Every
/// space in the output comes from measured glyph geometry instead, never from a character
/// PDFium invented.
const ZERO_BOX: f32 = 0.01;
fn is_synthetic_whitespace(c: &Char) -> bool { c.ch.is_whitespace() && c.w <= ZERO_BOX && c.h <= ZERO_BOX }

pub fn group_lines(chars: &[Char]) -> Vec<Line> {
    let mut sorted: Vec<&Char> = chars.iter().filter(|c| c.ch != '\n' && c.ch != '\r' && !is_synthetic_whitespace(c)).collect();
    sorted.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal));
    let mut rows: Vec<Vec<&Char>> = Vec::new();
    for c in sorted {
        match rows.last_mut() {
            Some(row) if (row[0].y - c.y).abs() <= baseline_threshold(row[0]) => row.push(c),
            _ => rows.push(vec![c]),
        }
    }
    rows.into_iter().map(row_to_line).collect()
}

/// Row membership goes by the baseline (`Char.y`, PDFium's text origin y), not the ink box: a
/// descender's own box sits lower than its baseline, but its baseline is the same as the rest of
/// its row. Half the font size is the threshold; `size` is `0.0` for callers that do not know it
/// (mostly tests), so `h / 2` keeps their old behaviour.
fn baseline_threshold(c: &Char) -> f32 { if c.size > 0.0 { c.size * 0.5 } else { c.h / 2.0 } }

fn row_to_line(mut row: Vec<&Char>) -> Line {
    row.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    let text = if row.iter().all(|c| c.advance.is_some()) { spaced_by_advance(&row) } else { spaced_by_origin(&row) };
    Line { x: row[0].x, y: row[0].y, text: text.trim_end().to_string() }
}

/// Old rule, unchanged: on these PDFs PDFium's loose bounds are a real advance box, so the gap
/// between consecutive boxes divided by the next glyph's own advance gives the space count.
fn spaced_by_advance(row: &[&Char]) -> String {
    let mut text = String::new();
    let mut prev_end: Option<f32> = None;
    for c in row {
        let advance = c.advance.unwrap();
        if let Some(end) = prev_end {
            let gap = c.x - end;
            if gap > advance * 0.5 { text.push_str(&" ".repeat(((gap / advance).round() as usize).max(1))); }
        }
        text.push(c.ch);
        prev_end = Some(c.x + advance);
    }
    text
}

/// New generator: loose bounds equal tight bounds, so a glyph's own width says nothing about
/// spacing (a narrow '1' is still one monospace cell wide). Use the origin-to-origin pitch
/// instead: on the 2026-06-30 statement it is a constant 4.80pt within a word, so the median of
/// a row's small deltas is the cell width, and anything past 1.5 cells is a real gap.
/// `round(delta / unit) - 1` turns a 2-cell gap into one space.
fn spaced_by_origin(row: &[&Char]) -> String {
    let unit = row_unit(row);
    let mut text = String::new();
    let mut prev_origin: Option<f32> = None;
    for c in row {
        if let Some(prev) = prev_origin {
            let delta = c.origin_x - prev;
            if delta > unit * 1.5 { text.push_str(&" ".repeat((((delta / unit).round() as isize) - 1).max(1) as usize)); }
        }
        text.push(c.ch);
        prev_origin = Some(c.origin_x);
    }
    text
}

/// The row's monospace cell width: the median of the origin-to-origin deltas that are close to
/// the smallest one seen (the intra-word pitch, 4.80pt on the reference statement), which drops
/// the larger deltas that are themselves real word gaps. Fewer than 3 glyphs give no reliable
/// cluster, so fall back to 0.6 of the font size (the same 4.80pt at 8pt).
fn row_unit(row: &[&Char]) -> f32 {
    let fallback = 0.6 * row.iter().map(|c| c.size).fold(0.0_f32, f32::max).max(1.0);
    if row.len() < 3 { return fallback; }
    let deltas: Vec<f32> = row.windows(2).map(|w| w[1].origin_x - w[0].origin_x).filter(|d| *d > 0.0).collect();
    let Some(min) = deltas.iter().cloned().reduce(f32::min) else { return fallback };
    let mut cell: Vec<f32> = deltas.into_iter().filter(|d| *d <= min * 1.5).collect();
    cell.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    cell[cell.len() / 2]
}

pub fn from_text(text: &str) -> Vec<Vec<String>> {
    text.split('\u{0c}').map(|p| p.lines().map(|l| l.trim_end().to_string()).collect()).collect()
}

/// Test-only geometry fixtures for the new (2026-06-30) generator, shared with
/// `header::tests` so both prove the fix from the same numbers.
#[cfg(test)]
pub(crate) mod fixtures {
    use super::Char;

    /// Real glyph on the 2026-06-30 Tatra banka statement (Michal's `abakus-cli geometry` run):
    /// loose bounds equal tight bounds (`advance: None`), scaled font size 8pt, monospace pitch
    /// 4.80pt origin-to-origin.
    pub(crate) fn mono(origin_x: f32, y: f32, ch: char, w: f32, h: f32) -> Char {
        Char { x: origin_x + 0.3, y, w, h, ch, origin_x, size: 8.0, advance: None }
    }
    /// PDFium's own synthetic whitespace on that statement: a real `' '` codepoint with a
    /// degenerate zero box, sitting at whatever x PDFium put it, never between the words it is
    /// supposed to separate.
    pub(crate) fn synth_space(x: f32, y: f32) -> Char { Char { x, y, w: 0.0, h: 0.0, ch: ' ', origin_x: x, size: 1.0, advance: None } }

    /// Page 1, row 1 of the diagnostic (`abakus-cli lines`/`geometry`), digits replaced by the
    /// real synthetic account number and date. Indices and coordinates copied verbatim.
    pub(crate) fn header_line1() -> Vec<Char> {
        vec![
            mono(73.68, 817.68, 'O', 4.00, 5.90), mono(78.48, 817.68, 's', 3.44, 4.33),
            mono(83.28, 817.68, 'o', 3.89, 4.33), mono(88.08, 817.68, 'b', 3.60, 5.82),
            mono(92.88, 817.68, 'n', 3.38, 4.24), mono(97.68, 817.68, 'ý', 3.97, 7.50),
            synth_space(137.68, 817.68),
            mono(107.28, 817.68, 'ú', 3.37, 5.91), mono(112.08, 817.68, 'č', 3.78, 5.91),
            mono(116.88, 817.68, 'e', 3.82, 4.33), mono(121.68, 817.68, 't', 3.50, 5.65),
            synth_space(161.68, 817.68),
            mono(164.88, 817.68, '2', 3.82, 5.75), mono(169.68, 817.68, '9', 3.77, 5.84),
            mono(174.48, 817.68, '3', 3.75, 5.84), mono(179.28, 817.68, '5', 3.79, 5.73),
            mono(184.08, 817.68, '3', 3.75, 5.84), mono(188.88, 817.68, '0', 3.73, 5.84),
            mono(193.68, 817.68, '1', 2.11, 5.75), // the narrow digit from the diagnostic
            mono(198.48, 817.68, '8', 3.77, 5.84), mono(203.28, 817.68, '8', 3.77, 5.84),
            mono(208.08, 817.68, '7', 3.70, 5.64),
            synth_space(248.08, 817.68),
            mono(284.88, 817.68, 'M', 3.60, 5.73), mono(289.68, 817.68, 'e', 3.82, 4.33),
            mono(294.48, 817.68, 'n', 3.38, 4.24), mono(299.28, 817.68, 'a', 3.82, 4.33),
            synth_space(339.28, 817.68),
            mono(313.68, 817.68, 'E', 3.56, 5.73), mono(318.48, 817.68, 'U', 3.52, 5.82),
            mono(323.28, 817.68, 'R', 4.14, 5.73),
            synth_space(363.28, 817.68),
            mono(443.28, 817.68, 'D', 3.86, 5.73), mono(448.08, 817.68, 'á', 3.82, 5.91),
            mono(452.88, 817.68, 't', 3.50, 5.65), mono(457.68, 817.68, 'u', 3.37, 4.23),
            mono(462.48, 817.68, 'm', 3.87, 4.24),
            synth_space(502.48, 817.68),
            mono(472.08, 817.68, '3', 3.75, 5.84), mono(476.88, 817.68, '0', 3.73, 5.84),
            mono(481.68, 817.68, '.', 0.80, 0.96), mono(486.48, 817.68, '0', 3.73, 5.84),
            mono(491.28, 817.68, '6', 3.78, 5.84), mono(496.08, 817.68, '.', 0.80, 0.96),
            mono(500.88, 817.68, '2', 3.82, 5.75), mono(505.68, 817.68, '0', 3.73, 5.84),
            mono(510.48, 817.68, '2', 3.82, 5.75), mono(515.28, 817.68, '6', 3.78, 5.84),
        ]
    }

    /// Row 2 of the same page (`IBAN SK97 ...`): indices 50..59 are verbatim from the
    /// diagnostic, the rest extends the same 4.80pt monospace pitch to the full IBAN and BIC.
    /// Two synthetic spaces sit at the diagnostic's own odd x positions (128.08, 152.08).
    pub(crate) fn header_line2() -> Vec<Char> {
        let words = ["IBAN", "SK97", "1100", "0000", "0029", "3530", "1887", "BIC", "(SWIFT)", "TATRSKBX"];
        let y = 808.80;
        let mut out = Vec::new();
        let mut origin = 73.68_f32;
        for (i, word) in words.iter().enumerate() {
            for ch in word.chars() {
                let w = if ch == '1' { 2.11 } else { 3.8 };
                out.push(mono(origin, y, ch, w, 5.8));
                origin += 4.80;
            }
            // The diagnostic (`abakus-cli lines`) shows "BIC" sitting in a far-right column, a
            // gap of about 40 cells past the IBAN digits ("<~40 spaces>B IC"); a single-cell gap
            // here would let the header regex swallow "BIC" into the IBAN capture.
            origin += if i == 6 { 150.0 } else { 4.80 };
            if i == 0 { out.push(synth_space(128.08, y)); }
            if i == 1 { out.push(synth_space(152.08, y)); }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixtures::{header_line1, header_line2, mono, synth_space};
    fn c(x: f32, y: f32, ch: char) -> Char { Char { x, y, w: 6.0, h: 10.0, ch, origin_x: x, size: 0.0, advance: Some(6.0) } }

    #[test] fn chars_on_one_baseline_form_one_line_sorted_by_x() {
        let l = group_lines(&[c(12.0, 700.0, 'b'), c(6.0, 700.0, 'a'), c(18.0, 700.4, 'c')]);
        assert_eq!(l.len(), 1); assert_eq!(l[0].text, "abc"); assert_eq!(l[0].x, 6.0);
    }
    #[test] fn top_of_page_comes_first() {
        let l = group_lines(&[c(0.0, 100.0, 'l'), c(0.0, 700.0, 'h')]);
        assert_eq!(l.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), vec!["h", "l"]);
    }
    #[test] fn a_gap_becomes_proportional_spaces() {
        let l = group_lines(&[c(0.0, 50.0, 'a'), c(6.0, 50.0, 'b'), c(36.0, 50.0, 'c')]);
        assert_eq!(l[0].text, "ab    c");
    }
    #[test] fn text_pages_split_on_form_feed() {
        let p = from_text("h1\nx\n\x0ch2\ny\n");
        assert_eq!(p, vec![vec!["h1".to_string(), "x".into()], vec!["h2".into(), "y".into()]]);
    }

    #[test] fn mono_generator_with_tight_bounds_keeps_words_and_iban_groups() {
        let mut chars = header_line1();
        chars.extend(header_line2());
        let l = group_lines(&chars);
        assert_eq!(l.len(), 2, "{l:?}");
        assert_eq!(l[0].text.split_whitespace().collect::<Vec<_>>(), vec!["Osobný", "účet", "2935301887", "Mena", "EUR", "Dátum", "30.06.2026"]);
        assert_eq!(l[1].text.split_whitespace().collect::<Vec<_>>(), vec!["IBAN", "SK97", "1100", "0000", "0029", "3530", "1887", "BIC", "(SWIFT)", "TATRSKBX"]);
    }

    #[test] fn narrow_glyph_in_a_mono_word_does_not_split_it() {
        let row = vec![
            mono(0.0, 800.0, '1', 2.11, 5.75), mono(4.80, 800.0, '1', 2.11, 5.75),
            mono(9.60, 800.0, '0', 3.8, 5.84), mono(14.40, 800.0, '0', 3.8, 5.84),
        ];
        let l = group_lines(&row);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].text, "1100");
    }

    #[test] fn synthetic_pdfium_spaces_are_ignored_for_geometry() {
        let row = vec![
            mono(0.0, 800.0, 'a', 3.8, 5.8), mono(4.80, 800.0, 'b', 3.8, 5.8),
            synth_space(10.0, 800.0), // sits between b and c; must add nothing
            mono(14.40, 800.0, 'c', 3.8, 5.8), mono(19.20, 800.0, 'd', 3.8, 5.8),
        ];
        let l = group_lines(&row);
        assert_eq!(l[0].text, "ab cd");
    }

    #[test] fn descender_does_not_split_the_row() {
        // row[0] has a small ink-box height (h/2 == 0.5) but a real font size (8pt, threshold
        // 4.0); the baseline is 0.6pt off, which the size-based threshold accepts and a bare
        // h/2 fallback would reject.
        let a = Char { x: 0.0, y: 800.0, w: 3.8, h: 1.0, ch: 'a', origin_x: 0.0, size: 8.0, advance: None };
        let y_char = Char { x: 4.8, y: 799.4, w: 3.8, h: 7.5, ch: 'ý', origin_x: 4.8, size: 8.0, advance: None };
        let l = group_lines(&[a, y_char]);
        assert_eq!(l.len(), 1, "{l:?}");
        assert_eq!(l[0].text, "aý");
    }
}
