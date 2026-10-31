//! Sample data so the radar has something to show on first launch.

use income_radar_core::*;

pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self, max: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % max.max(1)
    }

    /// A minute between `from` and `to` hours (24 h clock).
    pub fn between(&mut self, from: u16, to: u16) -> u16 {
        (from * 60 + self.next(((to - from) * 60) as u64) as u16).min(1439)
    }
}

/// Fills two budget cycles ending at `today` with plausible entries.
pub fn seed(store: &Store, today: NaiveDate) -> Result<(), StoreError> {
    let settings = store.settings()?;
    let cash = match store.accounts_with_balances()?.first() {
        Some(a) => a.account.id,
        None => store.add_account("Cash", AccountKind::Cash, Money(0))?,
    };
    let find_or_add = |name: &str, kind: AccountKind, opening: i64| -> Result<Id, StoreError> {
        match store
            .accounts_with_balances()?
            .into_iter()
            .find(|a| a.account.name == name)
        {
            Some(a) => Ok(a.account.id),
            None => store.add_account(name, kind, Money(opening)),
        }
    };
    let bank = find_or_add("Main bank", AccountKind::Bank, 420_000)?;
    let savings = find_or_add("Savings", AccountKind::Savings, 1_200_000)?;

    let find = |kind: Kind, name: &str| -> Result<Id, StoreError> {
        store
            .categories(kind)?
            .into_iter()
            .find(|c| c.name == name)
            .map(|c| c.id)
            .ok_or(StoreError::Invalid("missing default category"))
    };
    let add = |account: Id,
               kind: Kind,
               cat: &str,
               minor: i64,
               date: NaiveDate,
               minute: u16,
               note: &str| {
        store.add_transaction(&NewTransaction {
            account_id: account,
            category_id: find(kind, cat)?,
            kind,
            amount: Money(minor),
            date,
            minute,
            note: note.into(),
        })
    };

    let current = Period::cycle_containing(today, settings.cycle_start_day);
    let previous = current.previous();
    let mut rng = Lcg(0x1d2c3b4a);

    for (i, period) in [previous, current].into_iter().enumerate() {
        add(
            bank,
            Kind::Income,
            "Salary",
            1_250_000 + i as i64 * 50_000,
            period.start,
            9 * 60,
            "Monthly salary",
        )?;
        add(
            bank,
            Kind::Expense,
            "Home",
            450_000,
            period.start,
            10 * 60 + 30,
            "Rent",
        )?;
        let mid = period.start + chrono::Days::new(12);
        if mid <= today {
            add(
                bank,
                Kind::Income,
                "Freelance",
                180_000 + i as i64 * 40_000,
                mid,
                16 * 60,
                "Logo project",
            )?;
            add(
                bank,
                Kind::Expense,
                "Bills",
                38_500,
                mid,
                19 * 60 + 10,
                "Electricity & internet",
            )?;
        }
        for (from, to, amount, minute, note) in [
            (bank, cash, 200_000, 11 * 60, "ATM withdrawal"),
            (bank, savings, 150_000, 11 * 60 + 5, "Monthly saving"),
        ] {
            store.add_transfer(&NewTransfer {
                from_account: from,
                to_account: to,
                amount: Money(amount),
                date: period.start + chrono::Days::new(1),
                minute,
                note: note.into(),
            })?;
        }

        let last = if period.contains(today) {
            today
        } else {
            period.end.pred_opt().unwrap()
        };
        for day in period.start.iter_days().take_while(|d| *d <= last) {
            if rng.next(10) < 8 {
                add(
                    cash,
                    Kind::Expense,
                    "Food",
                    1_500 + rng.next(800) as i64,
                    day,
                    rng.between(7, 9),
                    "Morning coffee",
                )?;
            }
            if rng.next(10) < 6 {
                add(
                    cash,
                    Kind::Expense,
                    "Food",
                    2_800 + rng.next(3_500) as i64,
                    day,
                    rng.between(12, 14),
                    "Lunch",
                )?;
            }
            if rng.next(3) == 0 {
                add(
                    bank,
                    Kind::Expense,
                    "Transport",
                    6_000 + rng.next(4_000) as i64,
                    day,
                    rng.between(7, 22),
                    "Fuel",
                )?;
            }
            if rng.next(4) == 0 {
                add(
                    bank,
                    Kind::Expense,
                    "Groceries",
                    9_000 + rng.next(25_000) as i64,
                    day,
                    rng.between(18, 22),
                    "Supermarket",
                )?;
            }
            if rng.next(7) == 0 {
                add(
                    bank,
                    Kind::Expense,
                    "Shopping",
                    5_000 + rng.next(30_000) as i64,
                    day,
                    rng.between(16, 23),
                    "",
                )?;
            }
            if rng.next(9) == 0 {
                add(
                    cash,
                    Kind::Expense,
                    "Fun",
                    4_000 + rng.next(12_000) as i64,
                    day,
                    rng.between(20, 24),
                    "Cinema",
                )?;
            }
        }
    }
    Ok(())
}
