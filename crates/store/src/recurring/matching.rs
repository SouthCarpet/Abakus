//! Occurrence matching, coverage-based state derivation and stable-price
//! detection (recurring-contract.md §4-6). Pure functions over an already
//! loaded evidence slice and a set of trusted coverage ranges; nothing here
//! touches the database.
use super::coverage::{is_covered, Range};
use super::detect::{comparison_amount, comparison_currency, Evidence};
use super::schedule;
use super::types::{AmountBasis, Cadence, PriceChange, RecurringState, UnknownReason};
use crate::Result;
use chrono::{Duration, NaiveDate};

const GRACE_DAYS: i64 = 10;
/// A generous ceiling on how many occurrences a schedule can generate
/// between its anchor and `as_of`; real inputs terminate in well under a
/// few hundred iterations (a monthly cadence run for 1000 years is 12000).
const MAX_OCCURRENCES: i64 = 120_000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScheduleOutcome {
    pub last_paid: Option<NaiveDate>,
    pub next_due: Option<NaiveDate>,
    pub state: RecurringState,
    pub unknown_reason: Option<UnknownReason>,
    pub grace_until: Option<NaiveDate>,
    pub matched_transaction_ids: Vec<i64>,
}

struct Scan<'a> {
    evidence: &'a [Evidence],
    used: Vec<bool>,
    ranges: &'a [Range],
    as_of: NaiveDate,
    ambiguous: bool,
    consecutive_missed: i64,
    ended: bool,
    grace_until: Option<NaiveDate>,
    next_due: Option<NaiveDate>,
    unknown_reason: Option<UnknownReason>,
    last_paid: Option<NaiveDate>,
    matched_ids: Vec<i64>,
}

impl<'a> Scan<'a> {
    fn window(due: NaiveDate) -> Result<(NaiveDate, NaiveDate)> {
        let start = due.checked_sub_signed(Duration::days(GRACE_DAYS)).ok_or_else(|| crate::StoreError::Parse("recurring: okno je mimo podporovaného rozsahu".into()))?;
        let end = due.checked_add_signed(Duration::days(GRACE_DAYS)).ok_or_else(|| crate::StoreError::Parse("recurring: okno je mimo podporovaného rozsahu".into()))?;
        Ok((start, end))
    }

    fn candidates(&self, due: NaiveDate) -> Result<Vec<usize>> {
        let (start, end) = Self::window(due)?;
        Ok(self.evidence.iter().enumerate().filter(|(i, e)| !self.used[*i] && e.tx_date >= start && e.tx_date <= end).map(|(i, _)| i).collect())
    }

    fn match_one(&mut self, idx: usize) {
        self.used[idx] = true;
        self.consecutive_missed = 0;
        self.last_paid = Some(self.evidence[idx].tx_date);
        self.matched_ids.push(self.evidence[idx].transaction_id);
    }

    /// `true` once nothing further can change the verdict: a future
    /// occurrence (recorded as `next_due` and nothing else) or an
    /// unresolved coverage gap (highest-priority `unknown`).
    fn step_unmatched(&mut self, due: NaiveDate) -> Result<bool> {
        if due > self.as_of {
            if self.next_due.is_none() {
                self.next_due = Some(due);
            }
            return Ok(true);
        }
        let (elapsed_start, window_end) = Self::window(due)?;
        let elapsed_end = self.as_of.min(window_end);
        if !is_covered(self.ranges, elapsed_start, elapsed_end) {
            self.unknown_reason = Some(UnknownReason::MissingCoverage);
            return Ok(true);
        }
        self.record_proven_absence(due)
    }

    fn record_proven_absence(&mut self, due: NaiveDate) -> Result<bool> {
        // Strictly less than: `as_of == due+10` is still the last day of the
        // window (recurring-contract.md §5, "missing only when as_of >
        // due+10"), so it must read as grace/active, not yet proven.
        let window_end = due.checked_add_signed(Duration::days(GRACE_DAYS)).ok_or_else(|| crate::StoreError::Parse("recurring: okno je mimo podporovaného rozsahu".into()))?;
        if window_end < self.as_of {
            self.consecutive_missed += 1;
            if self.consecutive_missed >= 2 {
                self.ended = true;
                return Ok(true);
            }
        } else {
            self.grace_until = Some(window_end);
        }
        Ok(false)
    }

    fn run(&mut self, anchor: NaiveDate, cadence: Cadence) -> Result<()> {
        let first_date = self.evidence.first().map_or(anchor, |item| item.tx_date);
        let month_offset = schedule::calendar_months_between(anchor, first_date);
        let mut n = month_offset.div_euclid(schedule::cadence_months(cadence)) - 1;
        while n < MAX_OCCURRENCES {
            let due = match schedule::occurrence(anchor, cadence, n) {
                Err(_) if n < 0 => {
                    n += 1;
                    continue;
                }
                result => result?,
            };
            let (_, window_end) = Self::window(due)?;
            if self.evidence.first().is_some_and(|first| window_end < first.tx_date) {
                n += 1;
                continue;
            }
            let candidates = self.candidates(due)?;
            let done = match candidates.len() {
                1 => {
                    self.match_one(candidates[0]);
                    false
                }
                0 => self.step_unmatched(due)?,
                _ => {
                    self.ambiguous = true;
                    true
                }
            };
            if done {
                break;
            }
            n += 1;
        }
        Ok(())
    }

    fn final_state(&self) -> (RecurringState, Option<UnknownReason>) {
        if self.ambiguous {
            return (RecurringState::Unknown, Some(UnknownReason::AmbiguousMembership));
        }
        if let Some(reason) = self.unknown_reason {
            return (RecurringState::Unknown, Some(reason));
        }
        if self.ended {
            return (RecurringState::Ended, None);
        }
        if self.consecutive_missed == 1 {
            return (RecurringState::Missing, None);
        }
        if self.is_upcoming() {
            return (RecurringState::Upcoming, None);
        }
        (RecurringState::Active, None)
    }

    fn is_upcoming(&self) -> bool {
        let (Some(due), Ok(end)) = (self.next_due, schedule::month_end(self.as_of)) else { return false };
        due >= self.as_of && due <= end
    }

    fn finish(mut self) -> ScheduleOutcome {
        if !self.ambiguous && self.unknown_reason.is_none() && self.used.iter().any(|used| !used) {
            self.unknown_reason = Some(UnknownReason::UnmatchedHistory);
        }
        let (state, unknown_reason) = self.final_state();
        let Scan { last_paid, next_due, grace_until, matched_ids, .. } = self;
        ScheduleOutcome { last_paid, next_due, state, unknown_reason, grace_until, matched_transaction_ids: matched_ids }
    }
}

/// Matches `evidence` (already this group/selection's exact members,
/// oldest first) against the occurrence schedule of `anchor`/`cadence`
/// through `as_of`, and derives the row's `RecurringState` from trusted
/// coverage. Empty evidence (a confirmed decision whose only source
/// statement was deleted) is `unknown/no_evidence` before any schedule
/// arithmetic runs.
pub(crate) fn derive_schedule(evidence: &[Evidence], anchor: NaiveDate, cadence: Cadence, ranges: &[Range], as_of: NaiveDate) -> Result<ScheduleOutcome> {
    if evidence.is_empty() {
        return Ok(ScheduleOutcome { last_paid: None, next_due: None, state: RecurringState::Unknown, unknown_reason: Some(UnknownReason::NoEvidence), grace_until: None, matched_transaction_ids: Vec::new() });
    }
    let mut scan = Scan {
        evidence,
        used: vec![false; evidence.len()],
        ranges,
        as_of,
        ambiguous: false,
        consecutive_missed: 0,
        ended: false,
        grace_until: None,
        next_due: None,
        unknown_reason: None,
        last_paid: None,
        matched_ids: Vec::new(),
    };
    scan.run(anchor, cadence)?;
    Ok(scan.finish())
}

struct AmountRun {
    amount: i64,
    start: usize,
    end: usize,
}

fn amount_runs(evidence: &[Evidence]) -> Vec<AmountRun> {
    let mut runs: Vec<AmountRun> = Vec::new();
    for (i, e) in evidence.iter().enumerate() {
        let amount = comparison_amount(e);
        match runs.last_mut() {
            Some(last) if last.amount == amount => last.end = i,
            _ => runs.push(AmountRun { amount, start: i, end: i }),
        }
    }
    runs
}

/// The latest pair of adjacent stable runs (>=2 consecutive equal amounts
/// each, differing from each other): recurring-contract.md §6. Evidence
/// from a `foreign_unknown` group must never reach here (no price badge),
/// enforced by the caller.
pub(crate) fn detect_price_change(evidence: &[Evidence]) -> Option<PriceChange> {
    let runs = amount_runs(evidence);
    runs.windows(2).rev().find_map(|w| {
        let (old, new) = (&w[0], &w[1]);
        let old_len = old.end - old.start + 1;
        let new_len = new.end - new.start + 1;
        if old_len < 2 || new_len < 2 || old.amount == new.amount {
            return None;
        }
        Some(PriceChange {
            currency: comparison_currency(&evidence[new.start]),
            previous_cents: old.amount,
            current_cents: new.amount,
            delta_cents: new.amount - old.amount,
            effective_from: evidence[new.start].tx_date,
        })
    })
}

/// Index of the amount to project (recurring-contract.md §6): the last
/// point of the most recent run of >=2 consecutive equal amounts, or the
/// single latest observation when no stable pair exists yet.
pub(crate) fn stable_or_latest_index(evidence: &[Evidence]) -> Option<(usize, AmountBasis)> {
    if evidence.is_empty() {
        return None;
    }
    let runs = amount_runs(evidence);
    if let Some(last) = runs.last() {
        if last.end - last.start + 1 >= 2 {
            return Some((last.end, AmountBasis::StablePair));
        }
    }
    Some((evidence.len() - 1, AmountBasis::LatestObservation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recurring::key::{CurrencyBasis, Identity};
    use crate::recurring::types::RecurringDirection;

    fn ev(date: &str, amount: i64) -> Evidence {
        Evidence {
            transaction_id: date.len() as i64,
            account_id: 1,
            account_kind: parser::AccountKind::Personal,
            account_label: "Osobný".into(),
            fingerprint: date.to_string(),
            tx_date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            amount_cents: -amount,
            orig_amount_cents: None,
            orig_currency: None,
            merchant_raw: "MERCHANT".into(),
            category_id: None,
            subscription_category: false,
            group_key: "g".into(),
            direction: RecurringDirection::Expense,
            currency_basis: CurrencyBasis::Eur,
            identity: Identity::Merchant("merchant".into()),
        }
    }

    fn d(s: &str) -> NaiveDate { NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap() }
    fn full_year_coverage() -> Vec<Range> { vec![Range { start: d("2020-01-01"), end: d("2030-12-31") }] }

    #[test]
    fn three_monthly_observations_with_full_coverage_are_upcoming_at_mid_month() {
        let evidence = vec![ev("2026-01-31", 1200), ev("2026-02-28", 1200), ev("2026-03-31", 1200)];
        let anchor = evidence[0].tx_date;
        let out = derive_schedule(&evidence, anchor, Cadence::Monthly, &full_year_coverage(), d("2026-04-15")).unwrap();
        assert_eq!(out.state, RecurringState::Upcoming);
        assert_eq!(out.next_due, Some(d("2026-04-30")));
        assert_eq!(out.last_paid, Some(d("2026-03-31")));
    }

    #[test]
    fn a_missed_then_a_second_missed_window_reaches_ended_not_before() {
        // Last paid June 15, monthly. July window [05-25] covered and empty: at July25 grace/active, July26 missing (1 miss). August window [05-25] covered and empty too: Aug25 not ended yet, Aug26 ended.
        let evidence = vec![ev("2026-06-15", 1000)];
        let anchor = d("2026-06-15");
        let ranges = vec![Range { start: d("2026-01-01"), end: d("2026-08-31") }];

        let at_jul_25 = derive_schedule(&evidence, anchor, Cadence::Monthly, &ranges, d("2026-07-25")).unwrap();
        assert_eq!(at_jul_25.state, RecurringState::Active, "still within the 10-day grace on July 25");
        assert_eq!(at_jul_25.grace_until, Some(d("2026-07-25")));

        let at_jul_26 = derive_schedule(&evidence, anchor, Cadence::Monthly, &ranges, d("2026-07-26")).unwrap();
        assert_eq!(at_jul_26.state, RecurringState::Missing);

        let at_aug_25 = derive_schedule(&evidence, anchor, Cadence::Monthly, &ranges, d("2026-08-25")).unwrap();
        assert_eq!(at_aug_25.state, RecurringState::Missing, "still only one proven-missed window, not ended");

        let at_aug_26 = derive_schedule(&evidence, anchor, Cadence::Monthly, &ranges, d("2026-08-26")).unwrap();
        assert_eq!(at_aug_26.state, RecurringState::Ended);
    }

    #[test]
    fn removing_coverage_over_the_missed_window_makes_it_unknown_never_ended() {
        let evidence = vec![ev("2026-06-15", 1000)];
        let anchor = d("2026-06-15");
        // July 20 gap: [07-05..07-25] not fully covered.
        let ranges = vec![Range { start: d("2026-01-01"), end: d("2026-07-19") }, Range { start: d("2026-07-21"), end: d("2026-08-31") }];
        let out = derive_schedule(&evidence, anchor, Cadence::Monthly, &ranges, d("2026-08-26")).unwrap();
        assert_eq!(out.state, RecurringState::Unknown);
        assert_eq!(out.unknown_reason, Some(UnknownReason::MissingCoverage));
    }

    #[test]
    fn overlapping_and_gapped_evidence_reaching_ended_two_windows_later_still_matches_r10() {
        // R10's "Partial/unverifiable or off-by statement cannot close gap" is exercised by the coverage module's own tests; this covers the state machine's month-by-month walk directly.
        let evidence = vec![ev("2026-06-15", 1000)];
        let anchor = d("2026-06-15");
        let ranges = vec![Range { start: d("2026-01-01"), end: d("2026-12-31") }];
        let out = derive_schedule(&evidence, anchor, Cadence::Monthly, &ranges, d("2026-09-01")).unwrap();
        assert_eq!(out.state, RecurringState::Ended);
    }

    #[test]
    fn two_observations_in_one_occurrence_window_are_ambiguous_not_summed() {
        // Both Jan 3 and Jan 8 fall within the Jan 1 occurrence's ±10-day
        // window: neither is the sole eligible observation, so the whole
        // series reads as ambiguous rather than silently picking one.
        let evidence = vec![ev("2026-01-01", 1000), ev("2026-01-03", 1000), ev("2026-01-08", 1000)];
        let anchor = d("2025-12-01");
        let out = derive_schedule(&evidence, anchor, Cadence::Monthly, &full_year_coverage(), d("2026-02-01")).unwrap();
        assert_eq!(out.state, RecurringState::Unknown);
        assert_eq!(out.unknown_reason, Some(UnknownReason::AmbiguousMembership));
    }

    #[test]
    fn an_extra_observation_between_occurrence_windows_is_unmatched_history() {
        let evidence = vec![
            ev("2026-01-31", 1000),
            ev("2026-02-28", 1000),
            ev("2026-03-15", 1000),
            ev("2026-03-31", 1000),
        ];
        let out = derive_schedule(&evidence, d("2026-01-31"), Cadence::Monthly, &full_year_coverage(), d("2026-04-01")).unwrap();
        assert_eq!(out.state, RecurringState::Unknown);
        assert_eq!(out.unknown_reason, Some(UnknownReason::UnmatchedHistory));
    }

    #[test]
    fn a_manual_anchor_before_known_history_does_not_invent_earlier_misses() {
        let evidence = vec![ev("2026-06-15", 1000)];
        let out = derive_schedule(&evidence, d("2026-01-15"), Cadence::Monthly, &full_year_coverage(), d("2026-07-01")).unwrap();
        assert_eq!(out.state, RecurringState::Upcoming);
        assert_eq!(out.last_paid, Some(d("2026-06-15")));
        assert_eq!(out.next_due, Some(d("2026-07-15")));
    }

    #[test]
    fn empty_evidence_is_unknown_no_evidence() {
        let out = derive_schedule(&[], d("2026-01-01"), Cadence::Monthly, &full_year_coverage(), d("2026-06-01")).unwrap();
        assert_eq!(out.state, RecurringState::Unknown);
        assert_eq!(out.unknown_reason, Some(UnknownReason::NoEvidence));
    }

    #[test]
    fn price_change_badge_needs_two_stable_runs_and_survives_a_later_one_off() {
        let stable = vec![ev("2026-01-31", 1000), ev("2026-02-28", 1000), ev("2026-03-31", 1200), ev("2026-04-30", 1200)];
        let change = detect_price_change(&stable).unwrap();
        assert_eq!((change.previous_cents, change.current_cents, change.delta_cents), (1000, 1200, 200));
        assert_eq!(change.effective_from, d("2026-03-31"));

        let with_one_off = vec![ev("2026-01-31", 1000), ev("2026-02-28", 1000), ev("2026-03-31", 1200), ev("2026-04-30", 1200), ev("2026-05-31", 1200), ev("2026-06-30", 1300)];
        let change2 = detect_price_change(&with_one_off).unwrap();
        assert_eq!((change2.previous_cents, change2.current_cents, change2.effective_from), (1000, 1200, d("2026-03-31")), "a one-off differing payment must not replace the stable badge");
    }

    #[test]
    fn a_single_differing_payment_is_not_a_price_change_badge() {
        let evidence = vec![ev("2026-01-31", 1000), ev("2026-02-28", 1000), ev("2026-03-31", 1000), ev("2026-04-30", 1300)];
        assert!(detect_price_change(&evidence).is_none());
    }
}
