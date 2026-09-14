use parser::{parse_text, TxKind};

const STATEMENT: &str = "Osobný účet     SK44 1100 0000 0000 1234 5678          Mena  EUR       BIC (SWIFT)   TATRSKBX
IBAN SK44 1100 0000 0000 1234 5678
Dialog  0800 00 1100              ID:   00               Výpis číslo:        7
Osobný účet     SK44 1100 0000 0000 1234 5678     Majiteľ Jana Vzorová      Dátum 30.06.2026
Dátum sprac.  Popis                                     Dátum zúčt.       Suma
--------------------------------------------------------------------------------------------------
              Posledný výpis  30.05.2026                                      100.00
--------------------------------------------------------------------------------------------------
01.06.2026    Poplatok za vedenie účtu                                          2.00-
--------------------------------------------------------------------------------------------------
02.06.2026    Neznámy bankový záznam                                             1.00-
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                        97.00
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        7        Strana:        1
";

#[test]
fn recognized_fee_is_fee_and_unknown_shape_stays_other_with_warning() {
    let statement = parse_text(STATEMENT).unwrap();

    assert_eq!(statement.transactions[0].kind, TxKind::Fee, "the plan 091 fee contract defines descriptions starting with poplat as recognized fees");
    assert_eq!(statement.transactions[1].kind, TxKind::Other, "a truly unknown description must keep the safe Other kind");
    assert_eq!(statement.warnings, vec!["Neznámy typ transakcie: Neznámy bankový záznam"]);
}

#[test]
fn raw_fee_recognition_folds_prefix_and_rejects_later_or_invalid_text() {
    // Plan 091 oracle: only a valid first-line description beginning with
    // folded poplat qualifies. Body lines and a merchant mention do not.
    assert!(parser::raw_block_is_fee("01.06.2026    PÓPLATKY za účet                       2.00-"));
    assert!(!parser::raw_block_is_fee("01.06.2026    Obchod poplatok                       2.00-"));
    assert!(!parser::raw_block_is_fee("Poplatok za účet"));
    assert!(!parser::raw_block_is_fee("01.06.2026    Obchod                       2.00-\nPoplatok za účet"));
}
