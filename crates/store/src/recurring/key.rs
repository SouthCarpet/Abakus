//! Group identity (recurring-contract.md §3): a versioned canonical
//! serialization hashed with SHA-256, never a raw concatenation of
//! user-controlled text. Every field is written into the hash with a
//! trailing separator byte, including the last one, so no ordering of
//! shorter/longer field values can ever collide by shifting a boundary.
use super::types::RecurringDirection;
use parser::fold::fold;
use sha2::{Digest, Sha256};

const KEY_VERSION: u8 = 1;
/// ASCII unit separator: never produced by `fold`/`normalize`/IBAN text, so
/// it cannot appear inside a field and be mistaken for the framing byte.
const FIELD_SEP: u8 = 0x1F;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CurrencyBasis {
    Eur,
    Original(String),
    ForeignUnknown,
}

impl CurrencyBasis {
    pub(crate) fn resolve(orig_currency: Option<&str>, orig_amount_cents: Option<i64>) -> Self {
        match (orig_currency, orig_amount_cents) {
            (None, _) => Self::Eur,
            (Some(c), Some(_)) => Self::Original(c.trim().to_uppercase()),
            (Some(_), None) => Self::ForeignUnknown,
        }
    }
    fn code(&self) -> String {
        match self {
            Self::Eur => "eur".to_string(),
            Self::Original(c) => format!("original:{c}"),
            Self::ForeignUnknown => "foreign_unknown".to_string(),
        }
    }
    pub(crate) fn is_foreign_unknown(&self) -> bool { matches!(self, Self::ForeignUnknown) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Identity {
    /// Normalized IBAN plus `fold`ed counterparty name: both dimensions
    /// together, so a shared intermediary IBAN never merges two differently
    /// named counterparties.
    Iban(String, String),
    Merchant(String),
    /// No usable identity. Never shared with any other blank row (see
    /// `group_key`'s `fingerprint` fallback below): eligible only through an
    /// explicit manual "selected" decision, never automatic inference.
    Blank,
}

pub(crate) fn resolve_identity(counterparty_iban: Option<&str>, counterparty_name: Option<&str>, merchant_norm: &str) -> Identity {
    let iban = counterparty_iban.map(str::trim).filter(|s| !s.is_empty());
    let name = counterparty_name.map(str::trim).filter(|s| !s.is_empty());
    if let (Some(iban), Some(name)) = (iban, name) {
        return Identity::Iban(parser::iban::normalize(iban), fold(name));
    }
    let merchant = merchant_norm.trim();
    if merchant.is_empty() { Identity::Blank } else { Identity::Merchant(merchant.to_string()) }
}

pub(crate) fn direction_of(amount_cents: i64) -> RecurringDirection {
    if amount_cents < 0 { RecurringDirection::Expense } else { RecurringDirection::Income }
}

fn direction_code(d: RecurringDirection) -> &'static str {
    match d {
        RecurringDirection::Expense => "expense",
        RecurringDirection::Income => "income",
    }
}

pub(crate) struct KeyInput<'a> {
    pub account_id: i64,
    pub direction: RecurringDirection,
    pub currency_basis: &'a CurrencyBasis,
    pub identity: &'a Identity,
    pub place_norm: Option<&'a str>,
    pub card_last4: Option<&'a str>,
    /// Only consulted for `Identity::Blank`, to keep every blank-identity
    /// row in a group of one.
    pub fingerprint: &'a str,
}

pub(crate) fn group_key(input: &KeyInput) -> String {
    let (identity_kind, identity_value) = match input.identity {
        Identity::Iban(iban, name) => ("iban", format!("{iban}|{name}")),
        Identity::Merchant(m) => ("merchant", m.clone()),
        Identity::Blank => ("blank", input.fingerprint.to_string()),
    };
    let fields = [
        KEY_VERSION.to_string(),
        input.account_id.to_string(),
        direction_code(input.direction).to_string(),
        input.currency_basis.code(),
        identity_kind.to_string(),
        identity_value,
        input.place_norm.unwrap_or("").to_string(),
        input.card_last4.unwrap_or("").to_string(),
    ];
    let mut hasher = Sha256::new();
    for f in &fields {
        hasher.update(f.as_bytes());
        hasher.update([FIELD_SEP]);
    }
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(account_id: i64, direction: RecurringDirection, basis: &CurrencyBasis, identity: &Identity, place: Option<&str>, card: Option<&str>, fp: &str) -> String {
        group_key(&KeyInput { account_id, direction, currency_basis: basis, identity, place_norm: place, card_last4: card, fingerprint: fp })
    }

    #[test]
    fn same_inputs_produce_the_same_key() {
        let identity = Identity::Merchant("netflix".into());
        let a = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &identity, None, None, "fp1");
        let b = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &identity, None, None, "fp2");
        assert_eq!(a, b, "the fingerprint must not affect a non-blank identity's key");
    }

    #[test]
    fn different_accounts_never_share_a_key() {
        let identity = Identity::Merchant("netflix".into());
        let a = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &identity, None, None, "fp");
        let b = key(2, RecurringDirection::Expense, &CurrencyBasis::Eur, &identity, None, None, "fp");
        assert_ne!(a, b);
    }

    #[test]
    fn opposite_cash_direction_never_shares_a_key() {
        let identity = Identity::Merchant("netflix".into());
        let a = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &identity, None, None, "fp");
        let b = key(1, RecurringDirection::Income, &CurrencyBasis::Eur, &identity, None, None, "fp");
        assert_ne!(a, b);
    }

    #[test]
    fn different_currency_basis_never_shares_a_key() {
        let identity = Identity::Merchant("netflix".into());
        let eur = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &identity, None, None, "fp");
        let usd = key(1, RecurringDirection::Expense, &CurrencyBasis::Original("USD".into()), &identity, None, None, "fp");
        let gbp = key(1, RecurringDirection::Expense, &CurrencyBasis::Original("GBP".into()), &identity, None, None, "fp");
        assert_ne!(eur, usd);
        assert_ne!(usd, gbp);
    }

    #[test]
    fn place_and_card_dimensions_split_the_key() {
        let identity = Identity::Merchant("shop".into());
        let base = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &identity, None, None, "fp");
        let with_place = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &identity, Some("bratislava"), None, "fp");
        let with_card = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &identity, None, Some("1234"), "fp");
        assert_ne!(base, with_place);
        assert_ne!(base, with_card);
        assert_ne!(with_place, with_card);
    }

    #[test]
    fn two_blank_identity_rows_never_share_a_key_even_on_the_same_account() {
        let a = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &Identity::Blank, None, None, "fp-a");
        let b = key(1, RecurringDirection::Expense, &CurrencyBasis::Eur, &Identity::Blank, None, None, "fp-b");
        assert_ne!(a, b, "a blank identity must never turn into one shared merchant");
    }

    #[test]
    fn iban_identity_requires_both_iban_and_a_nonblank_name_or_it_falls_back() {
        // IBAN present but blank name: falls back to merchant_norm, not a
        // silently blank-name IBAN identity.
        let identity = resolve_identity(Some("SK4411000000000012345678"), Some("  "), "acme");
        assert_eq!(identity, Identity::Merchant("acme".to_string()));
    }

    #[test]
    fn iban_and_name_together_form_an_iban_identity_distinct_from_a_merchant_with_the_same_name() {
        let iban_identity = resolve_identity(Some("SK44 1100 0000 0000 1234 5678"), Some("Ján Novák"), "");
        assert_eq!(iban_identity, Identity::Iban("SK4411000000000012345678".to_string(), "jan novak".to_string()));
    }

    #[test]
    fn no_iban_and_no_merchant_is_blank() {
        assert_eq!(resolve_identity(None, None, ""), Identity::Blank);
        assert_eq!(resolve_identity(Some(""), Some(""), "  "), Identity::Blank);
    }

    #[test]
    fn currency_basis_resolution_matches_the_contract_rule() {
        assert_eq!(CurrencyBasis::resolve(None, None), CurrencyBasis::Eur);
        assert_eq!(CurrencyBasis::resolve(Some("usd"), Some(1000)), CurrencyBasis::Original("USD".to_string()));
        assert_eq!(CurrencyBasis::resolve(Some("USD"), None), CurrencyBasis::ForeignUnknown);
        assert!(CurrencyBasis::resolve(Some("USD"), None).is_foreign_unknown());
    }
}
