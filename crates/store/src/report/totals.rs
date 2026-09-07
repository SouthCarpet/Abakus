use super::{
    dates::month_keys, ReportCategoryTotal, ReportDateRange, ReportMonth, ReportTotals,
    ReportTransaction,
};
use crate::{CategoryKind, Result, StoreError};
use rules::Status;
use std::collections::BTreeMap;

type CategoryKey = (Option<i64>, String);

pub(super) fn calculate(
    rows: &[ReportTransaction],
    range: Option<ReportDateRange>,
) -> Result<(ReportTotals, Vec<ReportMonth>, Vec<ReportCategoryTotal>)> {
    let mut sums = Sums::default();
    let mut months: BTreeMap<String, (i128, i128)> = initial_months(range)?;
    let mut categories: BTreeMap<CategoryKey, i128> = BTreeMap::new();
    for row in rows {
        let contribution = contribution(row);
        sums.add(row, contribution)?;
        add_month(&mut months, row, contribution);
        add_category(&mut categories, row, contribution.expense);
    }
    let totals = sums.finish()?;
    let months = finish_months(months)?;
    let categories = finish_categories(categories)?;
    Ok((totals, months, categories))
}

#[derive(Default)]
struct Sums {
    income: i128,
    expense: i128,
    transfer_out: i128,
    transfer_in: i128,
    suggested: u32,
    unassigned: u32,
}

impl Sums {
    fn add(&mut self, row: &ReportTransaction, contribution: Contribution) -> Result<()> {
        self.income += contribution.income;
        self.expense += contribution.expense;
        self.transfer_out += contribution.transfer_out;
        self.transfer_in += contribution.transfer_in;
        self.suggested = self
            .suggested
            .checked_add(u32::from(row.transaction.status == Status::Suggested))
            .ok_or_else(count_overflow)?;
        self.unassigned = self
            .unassigned
            .checked_add(u32::from(row.transaction.status == Status::Unassigned))
            .ok_or_else(count_overflow)?;
        Ok(())
    }

    fn finish(self) -> Result<ReportTotals> {
        let net = self.income.checked_sub(self.expense).ok_or_else(overflow)?;
        Ok(ReportTotals {
            income_cents: narrow(self.income)?,
            expense_cents: narrow(self.expense)?,
            net_cents: narrow(net)?,
            transfer_out_cents: narrow(self.transfer_out)?,
            transfer_in_cents: narrow(self.transfer_in)?,
            suggested_count: self.suggested,
            unassigned_count: self.unassigned,
        })
    }
}

#[derive(Clone, Copy, Default)]
struct Contribution {
    income: i128,
    expense: i128,
    transfer_out: i128,
    transfer_in: i128,
}

fn contribution(row: &ReportTransaction) -> Contribution {
    let tx = &row.transaction;
    let amount = i128::from(tx.amount_cents);
    if tx.status == Status::Transfer {
        return Contribution {
            transfer_out: if amount < 0 { -amount } else { 0 },
            transfer_in: if amount > 0 { amount } else { 0 },
            ..Contribution::default()
        };
    }
    let income = amount > 0
        && tx.kind != "refund"
        && matches!(row.category_kind, Some(CategoryKind::Income) | None);
    let expense = row.category_kind == Some(CategoryKind::Expense)
        || (row.category_kind.is_none() && (amount < 0 || tx.kind == "refund"));
    Contribution {
        income: if income { amount } else { 0 },
        expense: if expense { -amount } else { 0 },
        ..Contribution::default()
    }
}

fn initial_months(range: Option<ReportDateRange>) -> Result<BTreeMap<String, (i128, i128)>> {
    let mut months = BTreeMap::new();
    if let Some(range) = range {
        for key in month_keys(range)? {
            months.insert(key, (0, 0));
        }
    }
    Ok(months)
}

fn add_month(
    months: &mut BTreeMap<String, (i128, i128)>,
    row: &ReportTransaction,
    contribution: Contribution,
) {
    let key = row.transaction.tx_date.format("%Y-%m").to_string();
    let entry = months.entry(key).or_default();
    entry.0 += contribution.income;
    entry.1 += contribution.expense;
}

fn category_label(row: &ReportTransaction) -> String {
    match (&row.transaction.parent_name, &row.transaction.category_name) {
        (Some(parent), Some(child)) => format!("{parent} / {child}"),
        (_, Some(category)) => category.clone(),
        _ => "Nezaradené".into(),
    }
}

fn add_category(
    categories: &mut BTreeMap<CategoryKey, i128>,
    row: &ReportTransaction,
    cents: i128,
) {
    if cents != 0 {
        *categories
            .entry((row.transaction.category_id, category_label(row)))
            .or_default() += cents;
    }
}

fn finish_months(months: BTreeMap<String, (i128, i128)>) -> Result<Vec<ReportMonth>> {
    months
        .into_iter()
        .map(|(month, (income, expense))| {
            Ok(ReportMonth {
                month,
                income_cents: narrow(income)?,
                expense_cents: narrow(expense)?,
            })
        })
        .collect()
}

fn finish_categories(categories: BTreeMap<CategoryKey, i128>) -> Result<Vec<ReportCategoryTotal>> {
    categories
        .into_iter()
        .filter(|(_, cents)| *cents != 0)
        .map(|((category_id, label), cents)| {
            Ok(ReportCategoryTotal {
                category_id,
                label,
                cents: narrow(cents)?,
            })
        })
        .collect()
}

fn narrow(value: i128) -> Result<i64> {
    i64::try_from(value).map_err(|_| overflow())
}

fn overflow() -> StoreError {
    StoreError::Parse("Súčet reportu prekročil podporovaný rozsah centov.".into())
}

fn count_overflow() -> StoreError {
    StoreError::Parse("Počet stavov transakcií prekročil podporovaný rozsah.".into())
}
