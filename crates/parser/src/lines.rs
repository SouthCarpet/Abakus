#[derive(Debug, Clone, PartialEq)]
pub struct Char { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub ch: char }
#[derive(Debug, Clone, PartialEq)]
pub struct Line { pub x: f32, pub y: f32, pub text: String }

pub fn group_lines(chars: &[Char]) -> Vec<Line> {
    let mut sorted: Vec<&Char> = chars.iter().filter(|c| c.ch != '\n' && c.ch != '\r').collect();
    sorted.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal));
    let mut rows: Vec<Vec<&Char>> = Vec::new();
    for c in sorted {
        match rows.last_mut() {
            Some(row) if (row[0].y - c.y).abs() <= row[0].h / 2.0 => row.push(c),
            _ => rows.push(vec![c]),
        }
    }
    rows.into_iter().map(row_to_line).collect()
}

fn row_to_line(mut row: Vec<&Char>) -> Line {
    row.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    let mut text = String::new();
    let mut prev_end: Option<f32> = None;
    for c in &row {
        if let Some(end) = prev_end {
            let gap = c.x - end;
            if gap > c.w * 0.5 { text.push_str(&" ".repeat(((gap / c.w).round() as usize).max(1))); }
        }
        text.push(c.ch);
        prev_end = Some(c.x + c.w);
    }
    Line { x: row[0].x, y: row[0].y, text: text.trim_end().to_string() }
}

pub fn from_text(text: &str) -> Vec<Vec<String>> {
    text.split('\u{0c}').map(|p| p.lines().map(|l| l.trim_end().to_string()).collect()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn c(x: f32, y: f32, ch: char) -> Char { Char { x, y, w: 6.0, h: 10.0, ch } }

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
}
