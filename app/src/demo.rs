//! Sample data so the radar has something to show on first launch.

use income_radar_core::*;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self, max: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % max
    }
}

/// Fills two budget cycles ending at `today` with plausible entries.
pub fn seed(store: &Store, today: NaiveDate) -> Result<(), StoreError> {
    let settings = store.settings()?;
    let cash = match store.accounts_with_balances()?.first() {
        Some(a) => a.account.id,
        None => store.add_account("Cash", AccountKind::Cash, Money(0))?,
    };
    let bank = store.add_account("Main bank", AccountKind::Bank, Money(420_000))?;
    let savings = store.add_account("Savings", AccountKind::Savings, Money(1_200_000))?;

    let find = |kind: Kind, name: &str| -> Result<Id, StoreError> {
        store
            .categories(kind)?
            .into_iter()
            .find(|c| c.name == name)
            .map(|c| c.id)
            .ok_or(StoreError::Invalid("missing default category"))
    };
    let add = |account: Id, kind: Kind, cat: &str, minor: i64, date: NaiveDate, note: &str| {
        store.add_transaction(&NewTransaction {
            account_id: account,
            category_id: find(kind, cat)?,
            kind,
            amount: Money(minor),
            date,
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
            "Monthly salary",
        )?;
        add(bank, Kind::Expense, "Home", 450_000, period.start, "Rent")?;
        let mid = period.start + chrono::Days::new(12);
        if mid <= today {
            add(
                bank,
                Kind::Income,
                "Freelance",
                180_000 + i as i64 * 40_000,
                mid,
                "Logo project",
            )?;
            add(
                bank,
                Kind::Expense,
                "Bills",
                38_500,
                mid,
                "Electricity & internet",
            )?;
        }
        store.add_transfer(&NewTransfer {
            from_account: bank,
            to_account: cash,
            amount: Money(200_000),
            date: period.start + chrono::Days::new(1),
            note: "ATM withdrawal".into(),
        })?;
        store.add_transfer(&NewTransfer {
            from_account: bank,
            to_account: savings,
            amount: Money(150_000),
            date: period.start + chrono::Days::new(1),
            note: "Monthly saving".into(),
        })?;

        let last = if period.contains(today) {
            today
        } else {
            period.end.pred_opt().unwrap()
        };
        for day in period.start.iter_days().take_while(|d| *d <= last) {
            add(
                cash,
                Kind::Expense,
                "Food",
                1_500 + rng.next(4_500) as i64,
                day,
                "Coffee & lunch",
            )?;
            if rng.next(3) == 0 {
                add(
                    bank,
                    Kind::Expense,
                    "Transport",
                    2_000 + rng.next(6_000) as i64,
                    day,
                    "Fuel",
                )?;
            }
            if rng.next(5) == 0 {
                add(
                    bank,
                    Kind::Expense,
                    "Shopping",
                    5_000 + rng.next(30_000) as i64,
                    day,
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
                    "Cinema",
                )?;
            }
        }
    }
    Ok(())
}
