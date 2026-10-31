//! The "radar": a snapshot of where the current budget cycle is heading.

use chrono::NaiveDate;

use crate::model::{CategoryTotal, Kind};
use crate::money::Money;
use crate::period::Period;
use crate::store::{Result, Store};

#[derive(Debug, Clone, PartialEq)]
pub struct Radar {
    pub today: NaiveDate,
    pub period: Period,
    pub total_balance: Money,
    pub income: Money,
    pub expense: Money,
    pub prev_income: Money,
    pub prev_expense: Money,
    /// How much can be spent per remaining day without going below zero for
    /// the cycle. Based on this cycle's income when there is any, otherwise on
    /// the total balance across accounts.
    pub safe_per_day: Money,
    pub days_left: u32,
    /// Spending extrapolated to the end of the cycle at the current pace.
    pub projected_expense: Money,
    /// Expenses for the last seven days, oldest first, ending today.
    pub last_7_days: Vec<(NaiveDate, Money)>,
    pub top_expenses: Vec<CategoryTotal>,
    pub income_sources: Vec<CategoryTotal>,
}

impl Radar {
    pub fn compute(store: &Store, today: NaiveDate) -> Result<Radar> {
        let settings = store.settings()?;
        let period = Period::cycle_containing(today, settings.cycle_start_day);
        let (income, expense) = store.totals(period)?;
        let (prev_income, prev_expense) = store.totals(period.previous())?;
        let total_balance = store.total_balance()?;

        let days_left = period.days_left(today);
        let budget_base = if income.0 > 0 {
            income - expense
        } else {
            total_balance
        };
        let safe_per_day = Money(budget_base.0.max(0) / days_left as i64);

        let elapsed = period.days_elapsed(today) as i64;
        let projected_expense = Money(expense.0 * period.total_days() as i64 / elapsed);

        let week_start = today - chrono::Days::new(6);
        Ok(Radar {
            today,
            period,
            total_balance,
            income,
            expense,
            prev_income,
            prev_expense,
            safe_per_day,
            days_left,
            projected_expense,
            last_7_days: store.daily_expenses(week_start, today)?,
            top_expenses: store.category_totals(Kind::Expense, period)?,
            income_sources: store.category_totals(Kind::Income, period)?,
        })
    }

    pub fn net(&self) -> Money {
        self.income - self.expense
    }

    /// Share of income kept this cycle, 0.0..=1.0, or `None` without income.
    pub fn savings_rate(&self) -> Option<f32> {
        (self.income.0 > 0).then(|| (self.net().0.max(0) as f32 / self.income.0 as f32).min(1.0))
    }

    /// Share of income already spent, clamped to 0.0..=1.0.
    pub fn spent_ratio(&self) -> f32 {
        if self.income.0 <= 0 {
            return if self.expense.0 > 0 { 1.0 } else { 0.0 };
        }
        (self.expense.0 as f32 / self.income.0 as f32).clamp(0.0, 1.0)
    }

    pub fn income_change_pct(&self) -> Option<i64> {
        change_pct(self.prev_income, self.income)
    }

    pub fn expense_change_pct(&self) -> Option<i64> {
        change_pct(self.prev_expense, self.expense)
    }
}

/// Whole-percent change from `before` to `after`; `None` when there is no base.
pub fn change_pct(before: Money, after: Money) -> Option<i64> {
    if before.0 <= 0 {
        return None;
    }
    let diff = (after.0 - before.0) as i128 * 100;
    Some((diff / before.0 as i128) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_pct_handles_zero_base() {
        assert_eq!(change_pct(Money(0), Money(500)), None);
        assert_eq!(change_pct(Money(1000), Money(1120)), Some(12));
        assert_eq!(change_pct(Money(1000), Money(500)), Some(-50));
    }
}
