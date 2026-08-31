use crate::blocks::Block;
use crate::fields::{field, pieces, FirstLine};
use crate::fold::fold;
use crate::model::{Transaction, TxKind};
use crate::money::{parse_amount, parse_date_short, parse_rate};

pub fn is_card(description: &str) -> bool {
    let f = fold(description);
    f.contains("nakup pos") || f.contains("nakup zahr") || f.contains("navrat pos") || f.contains("bankomat") || f.contains("vyb.hotov")
}

fn kind_of(description: &str, block: &Block) -> TxKind {
    let f = fold(description);
    if f.contains("navrat") { TxKind::Refund }
    else if f.contains("bankomat") || f.contains("vyb.hotov") { TxKind::Atm }
    else if f.contains("zahr") || field(block, "orig. suma").is_some() { TxKind::CardForeign }
    else { TxKind::Card }
}

pub fn parse_card(first: &FirstLine, block: &Block) -> Transaction {
    let mut t = Transaction::blank(first.posted, first.amount, kind_of(&first.description, block), block.lines.join("\n"));
    t.card_last4 = field(block, "cislo karty").map(|c| c.chars().rev().take(4).collect::<String>().chars().rev().collect());
    let place_pieces = { let p = pieces(block, "miesto platby"); if p.is_empty() { pieces(block, "miesto vyberu") } else { p } };
    t.merchant_raw = place_pieces.last().cloned().unwrap_or_else(|| first.description.clone());
    t.place = (place_pieces.len() > 1).then(|| place_pieces[0].clone());
    t.tx_date = field(block, "datum").and_then(|d| parse_date_short(&d)).or(first.right_date).unwrap_or(first.posted);
    apply_foreign(&mut t, block);
    t
}

fn apply_foreign(t: &mut Transaction, block: &Block) {
    let Some(orig) = field(block, "orig. suma") else { return };
    let mut parts = orig.split_whitespace();
    t.orig_amount_cents = parts.next().and_then(parse_amount);
    t.orig_currency = parts.next().map(str::to_string);
    t.rate_micros = field(block, "kurz").and_then(|r| parse_rate(&r));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::Block;
    use crate::fields::first_line;
    use crate::model::TxKind;
    use chrono::NaiveDate;
    fn parse(lines: &[&str]) -> crate::model::Transaction {
        let blk = Block { lines: lines.iter().map(|s| s.to_string()).collect() };
        parse_card(&first_line(&blk).unwrap(), &blk)
    }
    #[test] fn card_purchase() {
        let t = parse(&["01.06.2026    EUR AP nákup POS                                     1.99-", "   Číslo karty:      440000******1111      Držiteľ: JANA VZOROVÁ", "   Miesto platby:    Neuss                 ALDI SUED", "   Dátum:  29.05.26  Čas:  12:13:57        Suma:          1.99- EUR"]);
        assert_eq!(t.kind, TxKind::Card); assert_eq!(t.amount_cents, -199); assert_eq!(t.merchant_raw, "ALDI SUED"); assert_eq!(t.place.as_deref(), Some("Neuss"));
        assert_eq!(t.tx_date, NaiveDate::from_ymd_opt(2026, 5, 29).unwrap()); assert_eq!(t.posted_date, NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()); assert_eq!(t.card_last4.as_deref(), Some("1111"));
    }
    #[test] fn refund_is_positive_refund() {
        let t = parse(&["02.06.2026    EUR NÁVRAT POS          29.05.2026        43.00", "   Miesto platby:    LUXEMBOURG            AMAZON* NQ97D00C4", "   Dátum:  29.05.26  Čas:  00:00:00        Suma:         43.00  EUR"]);
        assert_eq!(t.kind, TxKind::Refund); assert_eq!(t.amount_cents, 4300); assert_eq!(t.merchant_raw, "AMAZON* NQ97D00C4");
    }
    #[test] fn atm_uses_miesto_vyberu() {
        let t = parse(&["04.06.2026    EUR VYB.HOTOV. BANKOMAT                180.00-", "   Miesto výberu:    ERFTTAL               Sparkasse Neuss", "   Dátum:  02.06.26  Čas:  07:08:05        Suma:        180.00- EUR"]);
        assert_eq!(t.kind, TxKind::Atm); assert_eq!(t.place.as_deref(), Some("ERFTTAL")); assert_eq!(t.merchant_raw, "Sparkasse Neuss");
    }
    #[test] fn foreign_purchase_keeps_original_amount_and_rate() {
        let t = parse(&["08.06.2026    POS nákup zahr.                      13.69-", "   Miesto platby/výberu: London            OF", "   Dátum:  05.06.26  Čas:  04:14:43        Suma:         13.69- EUR", "   Orig. suma:       15.38- USD            Kurz:          1.1237"]);
        assert_eq!(t.kind, TxKind::CardForeign); assert_eq!(t.orig_amount_cents, Some(-1538)); assert_eq!(t.orig_currency.as_deref(), Some("USD")); assert_eq!(t.rate_micros, Some(1_123_700)); assert_eq!(t.merchant_raw, "OF");
    }
    #[test] fn missing_card_date_falls_back_to_posted() {
        let t = parse(&["01.06.2026    EUR NÁKUP POS                 5.00-", "   Miesto platby:    X     SHOP"]);
        assert_eq!(t.tx_date, t.posted_date);
    }
    #[test] fn is_card_matches_all_pos_forms() {
        for d in ["EUR AP nákup POS", "EUR NÁKUP POS", "AP nákup POS", "POS nákup zahr.", "EUR NÁVRAT POS", "EUR VYB.HOTOV. BANKOMAT"] { assert!(is_card(d), "{d}"); }
        assert!(!is_card("Platba 0200/000000-5230000001")); assert!(!is_card("TPP 1100/000000-0098765432"));
    }
}
