//! Budget cycles. A cycle starts on the user's payday (day 1..=31 of the month;
//! in months that are shorter, the last day of the month) and runs until the
//! day before the next payday, so a salary paid on the 27th is counted in the
//! cycle it funds rather than split across two calendar months.

use chrono::{Datelike, Months, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    /// First day, inclusive.
    pub start: NaiveDate,
    /// Day after the last day, exclusive.
    pub end: NaiveDate,
}

pub const MAX_CYCLE_START_DAY: u32 = 31;

/// Payday of the month containing `anchor`, clamped to the month's length.
fn payday_in(anchor: NaiveDate, day: u32) -> NaiveDate {
    let first = anchor.with_day(1).unwrap_or(anchor);
    let next_first = first + Months::new(1);
    let last = next_first.pred_opt().unwrap_or(first).day();
    first.with_day(day.min(last)).unwrap_or(first)
}

impl Period {
    /// The cycle that contains `date`.
    pub fn cycle_containing(date: NaiveDate, start_day: u32) -> Period {
        let day = start_day.clamp(1, MAX_CYCLE_START_DAY);
        let this_month = payday_in(date, day);
        let start = if date >= this_month {
            this_month
        } else {
            payday_in(date.with_day(1).unwrap_or(date) - Months::new(1), day)
        };
        let end = payday_in(start.with_day(1).unwrap_or(start) + Months::new(1), day);
        Period { start, end }
    }

    pub fn previous(self) -> Period {
        // the same payday setting, one cycle earlier
        Period::cycle_containing(
            self.start.pred_opt().unwrap_or(self.start),
            self.payday_setting(),
        )
    }

    /// The configured day this cycle was built from (its start day, or the
    /// month length when the start was clamped).
    fn payday_setting(self) -> u32 {
        // A clamped start (28th of February for "31") still belongs to the
        // "31" setting: the next cycle's end tells us. Use the larger day.
        self.start.day().max(self.end.day())
    }

    pub fn contains(self, date: NaiveDate) -> bool {
        date >= self.start && date < self.end
    }

    pub fn total_days(self) -> u32 {
        (self.end - self.start).num_days() as u32
    }

    /// Days from `today` to the end of the cycle, counting today. At least 1.
    pub fn days_left(self, today: NaiveDate) -> u32 {
        if today >= self.end {
            return 1;
        }
        let from = today.max(self.start);
        ((self.end - from).num_days() as u32).max(1)
    }

    /// Days of the cycle already lived, counting today. At least 1.
    pub fn days_elapsed(self, today: NaiveDate) -> u32 {
        let until = today.min(self.end.pred_opt().unwrap_or(self.end));
        ((until - self.start).num_days() as u32 + 1).max(1)
    }

    /// `"October 2026"` for calendar months, `"27 Oct – 26 Nov"` otherwise.
    pub fn label(self) -> String {
        let last = self.end.pred_opt().unwrap_or(self.end);
        if self.start.day() == 1 {
            self.start.format("%B %Y").to_string()
        } else {
            format!(
                "{} – {}",
                self.start.format("%-d %b"),
                last.format("%-d %b")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn calendar_month_cycle() {
        let p = Period::cycle_containing(d(2026, 10, 31), 1);
        assert_eq!(p.start, d(2026, 10, 1));
        assert_eq!(p.end, d(2026, 11, 1));
        assert_eq!(p.total_days(), 31);
        assert_eq!(p.days_left(d(2026, 10, 31)), 1);
        assert_eq!(p.days_elapsed(d(2026, 10, 31)), 31);
        assert_eq!(p.label(), "October 2026");
    }

    #[test]
    fn payday_cycle_before_and_after_payday() {
        let before = Period::cycle_containing(d(2026, 10, 20), 27);
        assert_eq!(before.start, d(2026, 9, 27));
        assert_eq!(before.end, d(2026, 10, 27));

        let after = Period::cycle_containing(d(2026, 10, 27), 27);
        assert_eq!(after.start, d(2026, 10, 27));
        assert_eq!(after.end, d(2026, 11, 27));
        assert_eq!(after.label(), "27 Oct – 26 Nov");
        assert_eq!(after.previous(), before);
    }

    #[test]
    fn cycle_crosses_year_boundary() {
        let p = Period::cycle_containing(d(2027, 1, 5), 25);
        assert_eq!(p.start, d(2026, 12, 25));
        assert_eq!(p.end, d(2027, 1, 25));
        assert!(p.contains(d(2026, 12, 31)));
        assert!(!p.contains(d(2027, 1, 25)));
    }

    #[test]
    fn payday_on_the_31st_uses_the_last_day_of_short_months() {
        // payday 31: Feb 28 -> Mar 31 -> Apr 30 -> May 31
        let feb = Period::cycle_containing(d(2026, 2, 28), 31);
        assert_eq!((feb.start, feb.end), (d(2026, 2, 28), d(2026, 3, 31)));
        let mar = Period::cycle_containing(d(2026, 3, 31), 31);
        assert_eq!((mar.start, mar.end), (d(2026, 3, 31), d(2026, 4, 30)));
        let apr = Period::cycle_containing(d(2026, 5, 10), 31);
        assert_eq!((apr.start, apr.end), (d(2026, 4, 30), d(2026, 5, 31)));
        // every day belongs to exactly one cycle and cycles chain together
        let mut day = d(2026, 1, 1);
        let mut last_end = Period::cycle_containing(day, 31).end;
        while day < d(2027, 3, 1) {
            let p = Period::cycle_containing(day, 31);
            assert!(p.contains(day), "{day}");
            if p.end != last_end {
                assert_eq!(p.start, last_end);
                last_end = p.end;
            }
            day = day.succ_opt().unwrap();
        }
    }

    #[test]
    fn payday_29_and_30() {
        let p = Period::cycle_containing(d(2026, 3, 1), 30);
        assert_eq!((p.start, p.end), (d(2026, 2, 28), d(2026, 3, 30)));
        assert_eq!(p.previous().end, p.start);
        let leap = Period::cycle_containing(d(2028, 2, 29), 29);
        assert_eq!(leap.start, d(2028, 2, 29));
    }

    #[test]
    fn start_day_is_clamped() {
        let p = Period::cycle_containing(d(2026, 2, 28), 99);
        assert_eq!(p.start, d(2026, 2, 28));
        let p = Period::cycle_containing(d(2026, 2, 10), 0);
        assert_eq!(p.start, d(2026, 2, 1));
    }
}
