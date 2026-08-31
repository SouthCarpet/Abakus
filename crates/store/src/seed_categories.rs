//! Spec §5 default categories, seeded once when `categories` is empty.
//! `system` marks Nezaradené and Hotovosť as undeletable.

pub struct SeedCat { pub name: &'static str, pub kind: &'static str, pub subs: &'static [&'static str], pub system: bool }

pub const SEED: &[SeedCat] = &[
    SeedCat { name: "Bývanie", kind: "expense", subs: &["nájom SK", "ubytovanie DE"], system: false },
    SeedCat { name: "Internet/telefón", kind: "expense", subs: &["mobil", "internet"], system: false },
    SeedCat { name: "Odvody a poistenie", kind: "expense", subs: &["sociálka", "zdravotka", "poistenie", "dane"], system: false },
    SeedCat { name: "Jedlo", kind: "expense", subs: &["potraviny", "reštaurácia", "objednávky"], system: false },
    SeedCat { name: "Auto/doprava", kind: "expense", subs: &["tankovanie", "parkovanie", "MHD", "servis", "diaľnica"], system: false },
    SeedCat { name: "Nákupy", kind: "expense", subs: &["obchod", "drogéria", "oblečenie", "domácnosť", "elektronika", "alkohol"], system: false },
    SeedCat { name: "Zdravie", kind: "expense", subs: &["lekáreň", "lekár"], system: false },
    SeedCat { name: "Predplatné", kind: "expense", subs: &["Netflix", "Max HBO", "Voyo", "YouTube", "Apple"], system: false },
    SeedCat { name: "Investovanie", kind: "expense", subs: &["XTB", "TAM", "Finax", "Binance"], system: false },
    SeedCat { name: "Život", kind: "expense", subs: &["posilka", "suplementy", "zábava", "darčeky", "jednorazové"], system: false },
    SeedCat { name: "Hotovosť", kind: "expense", subs: &["bankomat"], system: true },
    SeedCat { name: "Poplatky", kind: "expense", subs: &["banka"], system: false },
    SeedCat { name: "Nezaradené", kind: "expense", subs: &[], system: true },
    SeedCat { name: "Faktúry", kind: "income", subs: &[], system: false },
    SeedCat { name: "Ostatný príjem", kind: "income", subs: &["od ľudí", "cashback", "úroky"], system: false },
];
