//! Advanced statistics: rates per hour and per minute, time-of-day patterns,
//! streaks, distributions and plain-language insights.

use chrono::{Datelike, Days, Months, NaiveDate, Weekday};

use crate::model::{Kind, Transaction, Weekend};
use crate::money::Money;
use crate::period::Period;
use crate::radar::change_pct;
use crate::store::{Result, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Range {
    #[default]
    ThisCycle,
    LastCycle,
    Last90Days,
    Last12Months,
}

impl Range {
    pub const ALL: [Range; 4] = [
        Range::ThisCycle,
        Range::LastCycle,
        Range::Last90Days,
        Range::Last12Months,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Range::ThisCycle => "This cycle",
            Range::LastCycle => "Last cycle",
            Range::Last90Days => "90 days",
            Range::Last12Months => "12 months",
        }
    }

    pub fn period(self, today: NaiveDate, cycle_start_day: u32) -> Period {
        let cycle = Period::cycle_containing(today, cycle_start_day);
        match self {
            Range::ThisCycle => cycle,
            Range::LastCycle => cycle.previous(),
            Range::Last90Days => Period {
                start: today - Days::new(89),
                end: today + Days::new(1),
            },
            Range::Last12Months => Period {
                start: today + Days::new(1) - Months::new(12),
                end: today + Days::new(1),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bucket {
    pub label: String,
    pub start: NaiveDate,
    pub income: Money,
    pub expense: Money,
    /// Bucket lies after today.
    pub future: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CategoryStat {
    pub category_id: i64,
    pub name: String,
    pub icon: String,
    pub total: Money,
    pub count: u32,
    /// Share of all spending in the range, 0..=1.
    pub share: f32,
    pub budget: Option<Money>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Insight {
    /// Icon key for the UI.
    pub icon: &'static str,
    pub text: String,
    /// Marks a warning (over budget, overspending).
    pub warning: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub range: Range,
    pub period: Period,
    pub label: String,
    /// Minutes of the range that have already happened (at least 1).
    pub elapsed_minutes: i64,
    pub elapsed_days: u32,
    pub income: Money,
    pub expense: Money,
    /// Same length of time immediately before the range.
    pub prev_income: Money,
    pub prev_expense: Money,

    pub income_per_hour: Money,
    pub income_per_minute: Money,
    /// Income divided by working hours (8 h on every non-weekend day).
    pub income_per_work_hour: Option<Money>,
    pub burn_per_day: Money,
    pub burn_per_hour: Money,
    pub burn_per_minute: Money,
    pub work_hours: u32,

    pub expense_count: u32,
    pub income_count: u32,
    pub entries_per_day: f32,
    pub avg_expense: Money,
    pub median_expense: Money,
    pub largest: Vec<Transaction>,
    /// Average time between two purchases.
    pub avg_gap_minutes: Option<i64>,
    /// What an average purchase costs in minutes of work.
    pub purchase_work_minutes: Option<i64>,
    /// Days the total balance lasts at the current daily burn.
    pub runway_days: Option<i64>,

    /// Spending by hour of day (0..24).
    pub by_hour: [Money; 24],
    /// Number of purchases by hour of day. Time patterns use counts, so one
    /// big bill does not decide when you "usually" spend.
    pub by_hour_count: [u32; 24],
    /// Spending by weekday, in display order (`week_labels`).
    pub by_weekday: [Money; 7],
    /// `heatmap[day][hour]` amounts, days in display order.
    pub heatmap: [[Money; 24]; 7],
    /// `heatmap_count[day][hour]` number of purchases.
    pub heatmap_count: [[u32; 24]; 7],
    pub week_labels: [&'static str; 7],
    pub peak_hour: Option<u8>,
    pub peak_weekday: Option<usize>,
    /// Share of spending between 20:00 and 05:59.
    pub night_share: f32,
    pub weekday_daily_avg: Money,
    pub weekend_daily_avg: Money,

    pub no_spend_days: u32,
    pub longest_no_spend: u32,
    pub current_no_spend: u32,

    pub buckets: Vec<Bucket>,
    pub categories: Vec<CategoryStat>,
    pub insights: Vec<Insight>,
}

fn per(amount: Money, minutes: i64, unit_minutes: i64) -> Money {
    if minutes <= 0 {
        return Money::ZERO;
    }
    Money(((amount.0 as i128 * unit_minutes as i128) / minutes as i128) as i64)
}

/// `"3 h 20 min"`, `"45 min"`, `"2 d 4 h"`.
pub fn format_minutes(minutes: i64) -> String {
    let m = minutes.max(0);
    if m < 60 {
        format!("{m} min")
    } else if m < 24 * 60 {
        let (h, r) = (m / 60, m % 60);
        if r == 0 {
            format!("{h} h")
        } else {
            format!("{h} h {r} min")
        }
    } else {
        let (d, h) = (m / 1440, (m % 1440) / 60);
        if h == 0 {
            format!("{d} d")
        } else {
            format!("{d} d {h} h")
        }
    }
}

fn week_order(weekend: Weekend) -> [Weekday; 7] {
    use Weekday::*;
    match weekend {
        // Week starts on Sunday where the weekend is Friday–Saturday.
        Weekend::FriSat => [Sun, Mon, Tue, Wed, Thu, Fri, Sat],
        Weekend::SatSun => [Mon, Tue, Wed, Thu, Fri, Sat, Sun],
    }
}

fn short_day(d: Weekday) -> &'static str {
    match d {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

fn hour_label(h: u8) -> String {
    format!("{:02}:00", h)
}

impl Stats {
    /// `now_minute` is the local minute of `today` (0..=1439).
    pub fn compute(
        store: &Store,
        today: NaiveDate,
        now_minute: u16,
        range: Range,
    ) -> Result<Stats> {
        let settings = store.settings()?;
        let period = range.period(today, settings.cycle_start_day);
        let order = week_order(settings.weekend);
        let day_index = |d: NaiveDate| order.iter().position(|w| *w == d.weekday()).unwrap_or(0);

        // how much of the range has happened
        let in_range = period.contains(today);
        let (elapsed_days, elapsed_minutes) = if in_range {
            let days = (today - period.start).num_days() as u32 + 1;
            (days, (days as i64 - 1) * 1440 + now_minute as i64 + 1)
        } else if period.start > today {
            (0, 1)
        } else {
            let days = period.total_days();
            (days, days as i64 * 1440)
        };
        let elapsed_minutes = elapsed_minutes.max(1);
        let last_day = if in_range {
            today
        } else {
            period.end.pred_opt().unwrap_or(period.end)
        };

        let txs = store.transactions_in(period)?;
        let mut income = Money::ZERO;
        let mut expense = Money::ZERO;
        let mut expenses: Vec<&Transaction> = Vec::new();
        let mut income_count = 0u32;
        let mut by_hour = [Money::ZERO; 24];
        let mut by_hour_count = [0u32; 24];
        let mut by_weekday = [Money::ZERO; 7];
        let mut heatmap = [[Money::ZERO; 24]; 7];
        let mut heatmap_count = [[0u32; 24]; 7];
        let mut daily = std::collections::BTreeMap::<NaiveDate, (Money, Money)>::new();

        for t in &txs {
            let entry = daily.entry(t.date).or_default();
            match t.kind {
                Kind::Income => {
                    income += t.amount;
                    income_count += 1;
                    entry.0 += t.amount;
                }
                Kind::Expense => {
                    expense += t.amount;
                    expenses.push(t);
                    entry.1 += t.amount;
                    let h = (t.minute / 60).min(23) as usize;
                    let d = day_index(t.date);
                    by_hour[h] += t.amount;
                    by_hour_count[h] += 1;
                    by_weekday[d] += t.amount;
                    heatmap[d][h] += t.amount;
                    heatmap_count[d][h] += 1;
                }
            }
        }

        // comparison with the same length of time just before the range
        let prev = Period {
            start: period.start - Days::new(elapsed_days.max(1) as u64),
            end: period.start,
        };
        let (prev_income, prev_expense) = store.totals(prev)?;

        // rates
        let work_days = period
            .start
            .iter_days()
            .take(elapsed_days as usize)
            .filter(|d| !settings.weekend.contains(d.weekday()))
            .count() as u32;
        let work_hours = work_days * 8;
        let income_per_work_hour =
            (work_hours > 0 && income.0 > 0).then(|| Money(income.0 / work_hours as i64));
        let burn_per_day = per(expense, elapsed_minutes, 1440);

        // distribution
        let mut amounts: Vec<i64> = expenses.iter().map(|t| t.amount.0).collect();
        amounts.sort_unstable();
        let expense_count = amounts.len() as u32;
        let avg_expense = if amounts.is_empty() {
            Money::ZERO
        } else {
            Money(expense.0 / amounts.len() as i64)
        };
        let median_expense = match amounts.len() {
            0 => Money::ZERO,
            n if n % 2 == 1 => Money(amounts[n / 2]),
            n => Money((amounts[n / 2 - 1] + amounts[n / 2]) / 2),
        };
        let mut largest: Vec<Transaction> = expenses.iter().map(|t| (*t).clone()).collect();
        largest.sort_by(|a, b| b.amount.cmp(&a.amount).then(b.stamp().cmp(&a.stamp())));
        largest.truncate(3);

        let avg_gap_minutes = if expenses.len() >= 2 {
            let first = expenses.first().map(|t| t.stamp()).unwrap_or(0);
            let last = expenses.last().map(|t| t.stamp()).unwrap_or(0);
            Some((last - first) / (expenses.len() as i64 - 1))
        } else {
            None
        };
        let purchase_work_minutes = income_per_work_hour
            .filter(|r| r.0 > 0 && avg_expense.0 > 0)
            .map(|r| (avg_expense.0 as i128 * 60 / r.0 as i128) as i64);
        let total_balance = store.total_balance()?;
        let runway_days =
            (burn_per_day.0 > 0 && total_balance.0 > 0).then(|| total_balance.0 / burn_per_day.0);

        // time of day
        let peak_hour = (expense_count_total(&by_hour_count) > 0).then(|| {
            (0..24u8)
                .max_by_key(|h| {
                    (
                        by_hour_count[*h as usize],
                        by_hour[*h as usize],
                        std::cmp::Reverse(*h),
                    )
                })
                .unwrap_or(0)
        });
        let day_counts: Vec<u32> = heatmap_count.iter().map(|r| r.iter().sum()).collect();
        let peak_weekday = (expense.0 > 0).then(|| {
            (0..7)
                .max_by_key(|d| (day_counts[*d], by_weekday[*d], std::cmp::Reverse(*d)))
                .unwrap_or(0)
        });
        let night: i64 = (20..24).chain(0..6).map(|h| by_hour[h].0).sum();
        let night_share = if expense.0 > 0 {
            night as f32 / expense.0 as f32
        } else {
            0.0
        };

        // days: streaks, weekday vs weekend
        let mut no_spend_days = 0;
        let mut longest_no_spend = 0;
        let mut run = 0;
        let (mut wd_total, mut wd_days, mut we_total, mut we_days) = (0i64, 0i64, 0i64, 0i64);
        for d in period.start.iter_days().take(elapsed_days as usize) {
            let spent = daily.get(&d).map(|v| v.1).unwrap_or_default();
            if spent.0 == 0 {
                no_spend_days += 1;
                run += 1;
                longest_no_spend = longest_no_spend.max(run);
            } else {
                run = 0;
            }
            if settings.weekend.contains(d.weekday()) {
                we_total += spent.0;
                we_days += 1;
            } else {
                wd_total += spent.0;
                wd_days += 1;
            }
        }
        // the streak still running today (today counts only when nothing is spent yet)
        let current_no_spend = if elapsed_days > 0 { run } else { 0 };
        let weekday_daily_avg = Money(if wd_days > 0 { wd_total / wd_days } else { 0 });
        let weekend_daily_avg = Money(if we_days > 0 { we_total / we_days } else { 0 });

        // chart buckets
        let buckets = make_buckets(period, today, &daily);

        // categories
        let limits = store.budget_limits()?;
        let mut cats: std::collections::HashMap<i64, CategoryStat> = Default::default();
        for t in &expenses {
            let c = cats.entry(t.category_id).or_insert_with(|| CategoryStat {
                category_id: t.category_id,
                name: t.category_name.clone(),
                icon: t.category_icon.clone(),
                total: Money::ZERO,
                count: 0,
                share: 0.0,
                budget: limits.get(&t.category_id).copied(),
            });
            c.total += t.amount;
            c.count += 1;
        }
        let mut categories: Vec<CategoryStat> = cats.into_values().collect();
        for c in &mut categories {
            c.share = if expense.0 > 0 {
                c.total.0 as f32 / expense.0 as f32
            } else {
                0.0
            };
        }
        categories.sort_by(|a, b| b.total.cmp(&a.total).then(a.name.cmp(&b.name)));

        let mut stats = Stats {
            range,
            label: period_label(range, period),
            period,
            elapsed_minutes,
            elapsed_days,
            income,
            expense,
            prev_income,
            prev_expense,
            income_per_hour: per(income, elapsed_minutes, 60),
            income_per_minute: per(income, elapsed_minutes, 1),
            income_per_work_hour,
            burn_per_day,
            burn_per_hour: per(expense, elapsed_minutes, 60),
            burn_per_minute: per(expense, elapsed_minutes, 1),
            work_hours,
            expense_count,
            income_count,
            entries_per_day: if elapsed_days > 0 {
                txs.len() as f32 / elapsed_days as f32
            } else {
                0.0
            },
            avg_expense,
            median_expense,
            largest,
            avg_gap_minutes,
            purchase_work_minutes,
            runway_days,
            by_hour,
            by_hour_count,
            by_weekday,
            heatmap,
            heatmap_count,
            week_labels: order.map(short_day),
            peak_hour,
            peak_weekday,
            night_share,
            weekday_daily_avg,
            weekend_daily_avg,
            no_spend_days,
            longest_no_spend,
            current_no_spend,
            buckets,
            categories,
            insights: Vec::new(),
        };
        let _ = last_day;
        stats.insights = insights(&stats, settings.weekend);
        Ok(stats)
    }

    pub fn net(&self) -> Money {
        self.income - self.expense
    }

    pub fn savings_rate(&self) -> Option<f32> {
        (self.income.0 > 0).then(|| (self.net().0 as f32 / self.income.0 as f32).clamp(-1.0, 1.0))
    }

    pub fn expense_change_pct(&self) -> Option<i64> {
        change_pct(self.prev_expense, self.expense)
    }

    pub fn income_change_pct(&self) -> Option<i64> {
        change_pct(self.prev_income, self.income)
    }

    /// Busiest heatmap cell (number of purchases), for scaling.
    pub fn heatmap_max(&self) -> u32 {
        self.heatmap_count
            .iter()
            .flat_map(|row| row.iter())
            .copied()
            .max()
            .unwrap_or_default()
    }
}

fn expense_count_total(by_hour: &[u32; 24]) -> u32 {
    by_hour.iter().sum()
}

fn period_label(range: Range, p: Period) -> String {
    let last = p.end.pred_opt().unwrap_or(p.end);
    match range {
        Range::ThisCycle | Range::LastCycle => p.label(),
        _ => format!(
            "{} – {}",
            p.start.format("%-d %b %Y"),
            last.format("%-d %b %Y")
        ),
    }
}

fn make_buckets(
    period: Period,
    today: NaiveDate,
    daily: &std::collections::BTreeMap<NaiveDate, (Money, Money)>,
) -> Vec<Bucket> {
    let total_days = period.total_days();
    let sum = |from: NaiveDate, to: NaiveDate| -> (Money, Money) {
        daily
            .range(from..to)
            .fold((Money::ZERO, Money::ZERO), |acc, (_, v)| {
                (acc.0 + v.0, acc.1 + v.1)
            })
    };
    let mut out = Vec::new();
    if total_days <= 31 {
        for d in period.start.iter_days().take(total_days as usize) {
            let (i, e) = sum(d, d + Days::new(1));
            out.push(Bucket {
                label: d.format("%-d").to_string(),
                start: d,
                income: i,
                expense: e,
                future: d > today,
            });
        }
    } else if total_days <= 120 {
        let mut d = period.start;
        while d < period.end {
            let next = (d + Days::new(7)).min(period.end);
            let (i, e) = sum(d, next);
            out.push(Bucket {
                label: d.format("%-d/%-m").to_string(),
                start: d,
                income: i,
                expense: e,
                future: d > today,
            });
            d = next;
        }
    } else {
        let mut d = period.start;
        while d < period.end {
            let month_end = NaiveDate::from_ymd_opt(d.year(), d.month(), 1)
                .map(|m| m + Months::new(1))
                .unwrap_or(period.end)
                .min(period.end);
            let (i, e) = sum(d, month_end);
            out.push(Bucket {
                label: d.format("%b").to_string(),
                start: d,
                income: i,
                expense: e,
                future: d > today,
            });
            d = month_end;
        }
    }
    out
}

fn insights(s: &Stats, weekend: Weekend) -> Vec<Insight> {
    let mut out = Vec::new();
    let pct = |x: f32| (x * 100.0).round() as i64;

    for c in &s.categories {
        if let Some(limit) = c.budget {
            if limit.0 > 0 && c.total.0 * 100 >= limit.0 * 80 {
                let used = c.total.0 * 100 / limit.0;
                out.push(Insight {
                    icon: "alert",
                    text: if c.total > limit {
                        format!(
                            "{} is over budget by {} ({used}% used).",
                            c.name,
                            (c.total - limit).format()
                        )
                    } else {
                        format!("{} has used {used}% of its budget.", c.name)
                    },
                    warning: true,
                });
            }
        }
    }

    if let Some(h) = s.peak_hour {
        let n = s.by_hour_count[h as usize];
        if s.expense_count >= 5 && n >= 2 {
            let share = n as f32 / s.expense_count as f32;
            out.push(Insight {
                icon: "clock",
                text: format!(
                    "You buy most often between {} and {} ({}% of purchases).",
                    hour_label(h),
                    hour_label((h + 1) % 24),
                    pct(share)
                ),
                warning: false,
            });
        }
    }

    if let Some(m) = s.purchase_work_minutes {
        out.push(Insight {
            icon: "briefcase",
            text: format!(
                "An average purchase costs you {} of work.",
                format_minutes(m.max(1))
            ),
            warning: false,
        });
    }

    if s.weekday_daily_avg.0 > 0 && s.weekend_daily_avg.0 > 0 {
        let diff = change_pct(s.weekday_daily_avg, s.weekend_daily_avg).unwrap_or(0);
        let days = if weekend == Weekend::FriSat {
            "Fri–Sat"
        } else {
            "Sat–Sun"
        };
        if diff.abs() >= 15 {
            out.push(Insight {
                icon: "calendar",
                text: if diff > 0 {
                    format!("You spend {diff}% more per day on weekends ({days}).")
                } else {
                    format!("You spend {}% less per day on weekends ({days}).", -diff)
                },
                warning: false,
            });
        }
    }

    if s.night_share >= 0.25 {
        out.push(Insight {
            icon: "moon",
            text: format!("{}% of spending happens after 20:00.", pct(s.night_share)),
            warning: false,
        });
    }

    if let Some(c) = s.categories.iter().max_by_key(|c| (c.count, c.total)) {
        if s.elapsed_days >= 7 && c.count >= 4 {
            let per_week = c.count as f32 * 7.0 / s.elapsed_days as f32;
            out.push(Insight {
                icon: "pie",
                text: format!(
                    "{} is your most frequent spend: about {:.0} times a week, {}% of spending.",
                    c.name,
                    per_week.max(1.0),
                    pct(c.share)
                ),
                warning: false,
            });
        }
    }

    if s.longest_no_spend >= 2 {
        out.push(Insight {
            icon: "leaf",
            text: if s.current_no_spend >= 2 {
                format!(
                    "{} no-spend days in a row right now (best: {}).",
                    s.current_no_spend, s.longest_no_spend
                )
            } else {
                format!(
                    "Longest no-spend streak: {} days ({} no-spend days in total).",
                    s.longest_no_spend, s.no_spend_days
                )
            },
            warning: false,
        });
    }

    if let Some(days) = s.runway_days {
        if days < 3650 {
            out.push(Insight {
                icon: "radar",
                text: format!("At this pace your balance covers about {days} days of spending."),
                warning: days < 14,
            });
        }
    }

    if let Some(rate) = s.savings_rate() {
        if rate < 0.0 {
            out.push(Insight {
                icon: "alert",
                text: format!(
                    "Spending is {} above income in this period.",
                    (s.expense - s.income).format()
                ),
                warning: true,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutes_are_human_readable() {
        assert_eq!(format_minutes(0), "0 min");
        assert_eq!(format_minutes(45), "45 min");
        assert_eq!(format_minutes(60), "1 h");
        assert_eq!(format_minutes(200), "3 h 20 min");
        assert_eq!(format_minutes(1440 * 2 + 60 * 4), "2 d 4 h");
    }

    #[test]
    fn rates_scale_by_minutes() {
        assert_eq!(per(Money(144_000), 1440, 60), Money(6_000));
        assert_eq!(per(Money(144_000), 1440, 1), Money(100));
        assert_eq!(per(Money(1), 0, 60), Money(0));
    }
}
