use parser::TxKind;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SIMILARITY_THRESHOLD: f64 = 0.92;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum RuleKind { Exact, Merchant, CounterpartyAccount, Seed }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct Rule { pub id: i64, pub kind: RuleKind, pub key: String, pub place: Option<String>, pub category_id: i64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum Status { Transfer, Confirmed, Suggested, Unassigned }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum Source { OwnAccount, AccountRule, ExactRule, MerchantRule, Similar, Seed, KindRule, None }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct Assignment { pub status: Status, pub category_id: Option<i64>, pub rule_id: Option<i64>, pub source: Source }

pub struct Facts<'a> { pub kind: TxKind, pub merchant_norm: &'a str, pub place_norm: Option<&'a str>, pub counterparty_iban: Option<&'a str> }
pub struct Context<'a> { pub own_ibans: &'a HashSet<String>, pub rules: &'a [Rule], pub cash_category: Option<i64>, pub refund_lookup: &'a dyn Fn(&str) -> Option<i64> }

fn hit(status: Status, r: &Rule, source: Source) -> Assignment { Assignment { status, category_id: Some(r.category_id), rule_id: Some(r.id), source } }
fn kind_hit(category_id: Option<i64>) -> Option<Assignment> { category_id.map(|c| Assignment { status: Status::Suggested, category_id: Some(c), rule_id: None, source: Source::KindRule }) }

pub fn classify(f: &Facts, ctx: &Context) -> Assignment {
    if let Some(iban) = f.counterparty_iban { if ctx.own_ibans.contains(iban) { return Assignment { status: Status::Transfer, category_id: None, rule_id: None, source: Source::OwnAccount }; } }
    let by = |k: RuleKind, pred: &dyn Fn(&Rule) -> bool| ctx.rules.iter().find(|r| r.kind == k && pred(r));
    if let Some(r) = f.counterparty_iban.and_then(|i| by(RuleKind::CounterpartyAccount, &|r| r.key == i)) { return hit(Status::Confirmed, r, Source::AccountRule); }
    if let Some(r) = by(RuleKind::Exact, &|r| r.key == f.merchant_norm && r.place.as_deref() == f.place_norm) { return hit(Status::Confirmed, r, Source::ExactRule); }
    if let Some(r) = by(RuleKind::Merchant, &|r| r.key == f.merchant_norm) { return hit(Status::Suggested, r, Source::MerchantRule); }
    if let Some(r) = most_similar(f.merchant_norm, ctx.rules) { return hit(Status::Suggested, r, Source::Similar); }
    // Single-word seed keys match by whole token only (a `contains` on `dm` or `of` would hit random merchants); multi-word keys use contains.
    if let Some(r) = by(RuleKind::Seed, &|r| if r.key.contains(' ') { f.merchant_norm.contains(&r.key) } else { f.merchant_norm.split(' ').any(|t| t == r.key) }) { return hit(Status::Suggested, r, Source::Seed); }
    kind_rule(f, ctx).unwrap_or(Assignment { status: Status::Unassigned, category_id: None, rule_id: None, source: Source::None })
}

fn most_similar<'a>(merchant: &str, rules: &'a [Rule]) -> Option<&'a Rule> {
    if merchant.is_empty() { return None; }
    rules.iter().filter(|r| matches!(r.kind, RuleKind::Exact | RuleKind::Merchant))
        .map(|r| (strsim::jaro_winkler(merchant, &r.key), r)).filter(|(s, _)| *s >= SIMILARITY_THRESHOLD)
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)).map(|(_, r)| r)
}

fn kind_rule(f: &Facts, ctx: &Context) -> Option<Assignment> {
    match f.kind { TxKind::Atm => kind_hit(ctx.cash_category), TxKind::Refund => kind_hit((ctx.refund_lookup)(f.merchant_norm)), _ => None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::TxKind;
    use std::collections::HashSet;

    fn rule(id: i64, kind: RuleKind, key: &str, place: Option<&str>, cat: i64) -> Rule { Rule { id, kind, key: key.into(), place: place.map(str::to_string), category_id: cat } }
    fn facts<'a>(kind: TxKind, m: &'a str, place: Option<&'a str>, iban: Option<&'a str>) -> Facts<'a> { Facts { kind, merchant_norm: m, place_norm: place, counterparty_iban: iban } }
    fn ctx<'a>(own: &'a HashSet<String>, rules: &'a [Rule], lookup: &'a dyn Fn(&str) -> Option<i64>) -> Context<'a> { Context { own_ibans: own, rules, cash_category: Some(99), refund_lookup: lookup } }
    fn none(_: &str) -> Option<i64> { None }

    #[test] fn own_account_is_transfer_before_everything() {
        let own: HashSet<String> = ["SK4411000000000012345678".to_string()].into();
        let rules = [rule(1, RuleKind::CounterpartyAccount, "SK4411000000000012345678", None, 5)];
        let a = classify(&facts(TxKind::TransferOut, "jana vzorova", None, Some("SK4411000000000012345678")), &ctx(&own, &rules, &none));
        assert_eq!(a.status, Status::Transfer); assert_eq!(a.category_id, None); assert_eq!(a.source, Source::OwnAccount);
    }
    #[test] fn account_rule_is_confirmed() {
        let own = HashSet::new(); let rules = [rule(1, RuleKind::CounterpartyAccount, "SK0281800000007000000001", None, 5)];
        let a = classify(&facts(TxKind::StandingOrder, "tpp 8180", None, Some("SK0281800000007000000001")), &ctx(&own, &rules, &none));
        assert_eq!((a.status, a.category_id, a.rule_id, a.source), (Status::Confirmed, Some(5), Some(1), Source::AccountRule));
    }
    #[test] fn exact_rule_beats_merchant_rule() {
        let own = HashSet::new(); let rules = [rule(1, RuleKind::Merchant, "netto marken discount", None, 7), rule(2, RuleKind::Exact, "netto marken discount", Some("juechen"), 8)];
        let a = classify(&facts(TxKind::Card, "netto marken discount", Some("juechen"), None), &ctx(&own, &rules, &none));
        assert_eq!((a.status, a.category_id, a.rule_id), (Status::Confirmed, Some(8), Some(2)));
    }
    #[test] fn merchant_rule_with_new_place_is_suggested() {
        let own = HashSet::new(); let rules = [rule(1, RuleKind::Merchant, "netto marken discount", None, 7), rule(2, RuleKind::Exact, "netto marken discount", Some("juechen"), 8)];
        let a = classify(&facts(TxKind::Card, "netto marken discount", Some("neuss"), None), &ctx(&own, &rules, &none));
        assert_eq!((a.status, a.category_id, a.rule_id, a.source), (Status::Suggested, Some(7), Some(1), Source::MerchantRule));
    }
    #[test] fn similar_merchant_is_suggested_above_threshold() {
        let own = HashSet::new(); let rules = [rule(1, RuleKind::Merchant, "lidl sagt danke", None, 7)];
        let a = classify(&facts(TxKind::Card, "lidl sagt dank", None, None), &ctx(&own, &rules, &none));
        assert_eq!((a.status, a.category_id, a.source), (Status::Suggested, Some(7), Source::Similar));
    }
    #[test] fn dissimilar_merchant_is_unassigned() {
        let own = HashSet::new(); let rules = [rule(1, RuleKind::Merchant, "lidl sagt danke", None, 7)];
        let a = classify(&facts(TxKind::Card, "bauhaus", None, None), &ctx(&own, &rules, &none));
        assert_eq!((a.status, a.category_id, a.source), (Status::Unassigned, None, Source::None));
    }
    #[test] fn seed_matches_a_token_and_is_suggested() {
        let own = HashSet::new(); let rules = [rule(1, RuleKind::Seed, "aldi", None, 3)];
        let a = classify(&facts(TxKind::Card, "aldi sued", None, None), &ctx(&own, &rules, &none));
        assert_eq!((a.status, a.category_id, a.source), (Status::Suggested, Some(3), Source::Seed));
    }
    #[test] fn atm_goes_to_cash_when_nothing_else_matches() {
        let own = HashSet::new(); let rules: [Rule; 0] = [];
        let a = classify(&facts(TxKind::Atm, "sparkasse neuss", None, None), &ctx(&own, &rules, &none));
        assert_eq!((a.status, a.category_id, a.source), (Status::Suggested, Some(99), Source::KindRule));
    }
    #[test] fn refund_takes_the_category_of_the_last_confirmed_purchase() {
        let own = HashSet::new(); let rules: [Rule; 0] = [];
        let lookup = |m: &str| (m == "amazon").then_some(11);
        let a = classify(&facts(TxKind::Refund, "amazon", None, None), &ctx(&own, &rules, &lookup));
        assert_eq!((a.status, a.category_id, a.source), (Status::Suggested, Some(11), Source::KindRule));
    }
}
