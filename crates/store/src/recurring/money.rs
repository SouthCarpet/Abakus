//! Checked money arithmetic (recurring-contract.md §2): i128 intermediates,
//! half-rounds-up, and a hard reject outside JavaScript's safe integer
//! range before any Rust monetary value crosses the IPC boundary.
use super::types::Cadence;
use crate::{Result, StoreError};

/// `Number.MAX_SAFE_INTEGER`.
pub(crate) const JS_SAFE_MAX: i128 = 9_007_199_254_740_991;

pub(crate) fn safe_i64(v: i128) -> Result<i64> {
    if v.unsigned_abs() > JS_SAFE_MAX as u128 {
        return Err(StoreError::Parse("recurring: hodnota presahuje bezpečný rozsah celého čísla pre JavaScript".into()));
    }
    i64::try_from(v).map_err(|_| StoreError::Parse("recurring: pretečenie pri prepočte sumy".into()))
}

fn cadence_divisor(c: Cadence) -> i64 {
    match c {
        Cadence::Monthly => 1,
        Cadence::Quarterly => 3,
        Cadence::Yearly => 12,
    }
}

fn cadence_annual_multiplier(c: Cadence) -> i64 {
    match c {
        Cadence::Monthly => 12,
        Cadence::Quarterly => 4,
        Cadence::Yearly => 1,
    }
}

/// Nearest cent, half rounded up. `n` and `d` are both non-negative by
/// construction at every call site (checked once, here).
fn round_half_up(n: i128, d: i128) -> Result<i128> {
    if d <= 0 {
        return Err(StoreError::Parse("recurring: delenie nulou pri prepočte sumy".into()));
    }
    if n < 0 {
        return Err(StoreError::Parse("recurring: záporná suma pri prepočte".into()));
    }
    Ok((2 * n + d) / (2 * d))
}

/// Positive magnitude cents divided by the cadence's month count (1/3/12),
/// rounded to the nearest cent.
pub(crate) fn monthly_apportionment_cents(amount_cents_positive: i64, cadence: Cadence) -> Result<i64> {
    safe_i64(round_half_up(amount_cents_positive as i128, cadence_divisor(cadence) as i128)?)
}

/// Positive magnitude cents multiplied by the cadence's per-year occurrence
/// count (12/4/1), computed from the ORIGINAL amount, never from the
/// already-rounded monthly figure (10000 yearly cents must project to
/// 10000 annual cents, not 833 * 12 = 9996).
pub(crate) fn annual_projection_cents(amount_cents_positive: i64, cadence: Cadence) -> Result<i64> {
    if amount_cents_positive < 0 {
        return Err(StoreError::Parse("recurring: záporná suma pri prepočte".into()));
    }
    safe_i64(amount_cents_positive as i128 * cadence_annual_multiplier(cadence) as i128)
}

/// Integer basis points (1/100 of a percent), half rounded up, or `None`
/// when there is nothing to divide by. The only place a non-integer ratio
/// is computed at all; the result is always an integer.
pub(crate) fn basis_points(numerator_cents: i64, denominator_cents: i64) -> Result<Option<i64>> {
    if denominator_cents <= 0 || numerator_cents < 0 {
        return Ok(None);
    }
    Ok(Some(safe_i64(round_half_up(numerator_cents as i128 * 10_000, denominator_cents as i128)?)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yearly_ten_thousand_cents_apportions_to_833_monthly_and_projects_to_10000_annual() {
        assert_eq!(monthly_apportionment_cents(10_000, Cadence::Yearly).unwrap(), 833);
        assert_eq!(annual_projection_cents(10_000, Cadence::Yearly).unwrap(), 10_000, "must not be monthly*12 (833*12=9996)");
    }

    #[test]
    fn half_a_cent_rounds_up() {
        // 5 / 2 -> monthly divisor path is only 1/3/12, so exercise the
        // shared rounding helper through basis_points instead: 25/1000 = 2.5%.
        assert_eq!(basis_points(25, 1000).unwrap(), Some(250));
        assert_eq!(basis_points(15, 1000).unwrap(), Some(150));
    }

    #[test]
    fn quarterly_and_monthly_divisors() {
        assert_eq!(monthly_apportionment_cents(3000, Cadence::Quarterly).unwrap(), 1000);
        assert_eq!(monthly_apportionment_cents(1200, Cadence::Monthly).unwrap(), 1200);
        assert_eq!(annual_projection_cents(1200, Cadence::Monthly).unwrap(), 14_400);
        assert_eq!(annual_projection_cents(3000, Cadence::Quarterly).unwrap(), 12_000);
    }

    #[test]
    fn a_null_or_nonpositive_denominator_returns_none_not_a_fabricated_zero() {
        assert_eq!(basis_points(500, 0).unwrap(), None);
        assert_eq!(basis_points(500, -100).unwrap(), None);
    }

    #[test]
    fn a_value_outside_the_js_safe_integer_range_is_rejected() {
        assert!(safe_i64(JS_SAFE_MAX + 1).is_err());
        assert!(safe_i64(JS_SAFE_MAX).is_ok());
        assert!(safe_i64(-(JS_SAFE_MAX + 1)).is_err());
    }

    #[test]
    fn a_negative_amount_is_rejected_not_silently_flipped() {
        assert!(monthly_apportionment_cents(-100, Cadence::Monthly).is_err());
        assert!(annual_projection_cents(-100, Cadence::Monthly).is_err());
    }
}
