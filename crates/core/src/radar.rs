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
    /// Daily allowance: what can be spent per remaining day (today included)
    /// without going below zero for the cycle. It is fixed at the start of the
    /// day, so spending today lowers `today_left`, not the allowance itself.
    ///
    /// Based on this cycle's income when there is any, otherwise on the total
    /// balance across accounts.
    pub safe_per_day: Money,
    pub spent_today: Money,
    /// `safe_per_day - spent_today`; negative when today is over the allowance.
    pub today_left: Money,
    pub days_left: u32,
    /// Spending expected by the end of the cycle: what is already spent plus a
    /// typical day for every remaining day. One-off spikes such as rent do not
    /// get multiplied.
    pub projected_expense: Money,
    /// Expenses for the last seven days, oldest first, ending today.
    pub last_7_days: Vec<(NaiveDate, Money)>,
    pub top_expenses: Vec<CategoryTotal>,
    pub income_sources: Vec<CategoryTotal>,
    pub has_entries: bool,
}

/// Mean of daily totals after dropping the single biggest day (when there are
/// at least three days), so one large bill does not define a "typical" day.
pub fn typical_day(days: &[Money]) -> Money {
    if days.is_empty() {
        return Money::ZERO;
    }
    let mut v: Vec<i64> = days.iter().map(|m| m.0).collect();
    v.sort_unstable();
    if v.len() >= 3 {
        v.pop();
    }
    Money(v.iter().sum::<i64>() / v.len() as i64)
}

impl Radar {
    pub fn compute(store: &Store, today: NaiveDate) -> Result<Radar> {
        let settings = store.settings()?;
        let period = Period::cycle_containing(today, settings.cycle_start_day);
        let (income, expense) = store.totals(period)?;
        let (prev_income, prev_expense) = store.totals(period.previous())?;
        let total_balance = store.total_balance()?;

        let cycle_days = store.daily_expenses(period.start, today)?;
        let spent_today = cycle_days.last().map(|(_, m)| *m).unwrap_or_default();
        let days_left = period.days_left(today);

        // what was available when today started
        let base = if income.0 > 0 {
            income - expense + spent_today
        } else {
            total_balance + spent_today
        };
        let safe_per_day = Money(base.0.max(0) / days_left as i64);
        let today_left = safe_per_day - spent_today;

        let before_today: Vec<Money> = cycle_days[..cycle_days.len().saturating_sub(1)]
            .iter()
            .map(|(_, m)| *m)
            .collect();
        let typical = if before_today.is_empty() {
            let prev = period.previous();
            let prev_days: Vec<Money> = store
                .daily_expenses(prev.start, prev.end.pred_opt().unwrap_or(prev.end))?
                .into_iter()
                .map(|(_, m)| m)
                .collect();
            if prev_expense.0 > 0 {
                typical_day(&prev_days)
            } else {
                Money::ZERO
            }
        } else {
            typical_day(&before_today)
        };
        let remaining_days = days_left.saturating_sub(1) as i64;
        let projected_expense = expense - spent_today
            + Money(spent_today.0.max(typical.0))
            + Money(typical.0 * remaining_days);

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
            spent_today,
            today_left,
            days_left,
            projected_expense,
            last_7_days: store.daily_expenses(week_start, today)?,
            top_expenses: store.category_totals(Kind::Expense, period)?,
            income_sources: store.category_totals(Kind::Income, period)?,
            has_entries: store.entry_count()? > 0,
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

    /// Share of today's allowance already used, 0.0..=1.0.
    pub fn today_ratio(&self) -> f32 {
        if self.safe_per_day.0 <= 0 {
            return if self.spent_today.0 > 0 { 1.0 } else { 0.0 };
        }
        (self.spent_today.0 as f32 / self.safe_per_day.0 as f32).clamp(0.0, 1.0)
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

    #[test]
    fn typical_day_ignores_one_spike() {
        assert_eq!(typical_day(&[]), Money(0));
        assert_eq!(typical_day(&[Money(100)]), Money(100));
        assert_eq!(
            typical_day(&[Money(450_000), Money(100), Money(300)]),
            Money(200)
        );
    }
}
