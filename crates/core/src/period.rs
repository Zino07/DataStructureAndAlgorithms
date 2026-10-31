//! Budget cycles. A cycle starts on the user's payday (day 1..=28 of the month)
//! and runs until the day before the next payday, so a salary paid on the 27th
//! is counted in the cycle it funds rather than split across two calendar months.

use chrono::{Datelike, Months, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    /// First day, inclusive.
    pub start: NaiveDate,
    /// Day after the last day, exclusive.
    pub end: NaiveDate,
}

pub const MAX_CYCLE_START_DAY: u32 = 28;

impl Period {
    /// The cycle that contains `date`.
    pub fn cycle_containing(date: NaiveDate, start_day: u32) -> Period {
        let day = start_day.clamp(1, MAX_CYCLE_START_DAY);
        let this_month = date
            .with_day(day)
            .expect("days 1..=28 exist in every month");
        let start = if date.day() >= day {
            this_month
        } else {
            this_month - Months::new(1)
        };
        Period {
            start,
            end: start + Months::new(1),
        }
    }

    pub fn previous(self) -> Period {
        Period {
            start: self.start - Months::new(1),
            end: self.start,
        }
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
    fn start_day_is_clamped() {
        let p = Period::cycle_containing(d(2026, 2, 28), 31);
        assert_eq!(p.start, d(2026, 2, 28));
        let p = Period::cycle_containing(d(2026, 2, 10), 0);
        assert_eq!(p.start, d(2026, 2, 1));
    }
}
