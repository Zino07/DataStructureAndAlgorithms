use income_radar_core::*;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn category(store: &Store, kind: Kind, name: &str) -> Id {
    store
        .categories(kind)
        .unwrap()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap()
        .id
}

fn add(store: &Store, account: Id, kind: Kind, cat: &str, amount: &str, date: NaiveDate) -> Id {
    store
        .add_transaction(&NewTransaction {
            account_id: account,
            category_id: category(store, kind, cat),
            kind,
            amount: Money::parse(amount).unwrap(),
            date,
            minute: 12 * 60,
            note: String::new(),
        })
        .unwrap()
}

#[test]
fn first_run_seeds_once() {
    let dir = std::env::temp_dir().join(format!("ir-seed-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.db");
    let _ = std::fs::remove_file(&path);
    {
        let store = Store::open(&path).unwrap();
        assert_eq!(store.accounts_with_balances().unwrap().len(), 1);
        assert_eq!(store.categories(Kind::Expense).unwrap().len(), 11);
        assert_eq!(store.categories(Kind::Income).unwrap().len(), 5);
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.accounts_with_balances().unwrap().len(), 1);
    assert_eq!(store.categories(Kind::Expense).unwrap().len(), 11);
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn balances_follow_the_ledger() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    let bank = store
        .add_account("Bank", AccountKind::Bank, Money(100_000))
        .unwrap();

    add(&store, bank, Kind::Income, "Salary", "8000", d(2026, 10, 1));
    let coffee = add(&store, cash, Kind::Expense, "Food", "12.50", d(2026, 10, 2));
    store
        .add_transfer(&NewTransfer {
            from_account: bank,
            to_account: cash,
            amount: Money(50_000),
            date: d(2026, 10, 3),
            minute: 600,
            note: String::new(),
        })
        .unwrap();

    let balances = store.accounts_with_balances().unwrap();
    let get = |id| {
        balances
            .iter()
            .find(|b| b.account.id == id)
            .unwrap()
            .balance
    };
    assert_eq!(get(cash), Money(50_000 - 1_250));
    assert_eq!(get(bank), Money(100_000 + 800_000 - 50_000));
    assert_eq!(
        store.total_balance().unwrap(),
        Money(100_000 + 800_000 - 1_250)
    );

    assert!(store.delete_transaction(coffee).unwrap());
    assert!(!store.delete_transaction(coffee).unwrap());
    assert_eq!(store.total_balance().unwrap(), Money(900_000));
}

#[test]
fn rejects_invalid_transactions() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    let food = category(&store, Kind::Expense, "Food");
    let base = NewTransaction {
        account_id: cash,
        category_id: food,
        kind: Kind::Expense,
        amount: Money(100),
        date: d(2026, 10, 1),
        minute: 600,
        note: String::new(),
    };
    assert!(store
        .add_transaction(&NewTransaction {
            amount: Money(0),
            ..base.clone()
        })
        .is_err());
    assert!(store
        .add_transaction(&NewTransaction {
            kind: Kind::Income,
            ..base.clone()
        })
        .is_err());
    assert!(store
        .add_transaction(&NewTransaction {
            category_id: 9999,
            ..base.clone()
        })
        .is_err());
    assert!(store
        .add_transaction(&NewTransaction {
            account_id: 9999,
            ..base.clone()
        })
        .is_err());
    assert!(store
        .add_transfer(&NewTransfer {
            from_account: cash,
            to_account: cash,
            amount: Money(1),
            date: d(2026, 10, 1),
            minute: 600,
            note: String::new()
        })
        .is_err());
    assert!(store
        .add_account("  ", AccountKind::Cash, Money(0))
        .is_err());
}

#[test]
fn settings_round_trip_and_clamp() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(store.settings().unwrap(), Settings::default());
    store
        .save_settings(&Settings {
            currency: "AED".into(),
            cycle_start_day: 40,
            palette: 9,
            dark: false,
            weekend: Weekend::SatSun,
        })
        .unwrap();
    let s = store.settings().unwrap();
    assert_eq!(s.currency, "AED");
    assert_eq!(s.cycle_start_day, 28);
    assert_eq!(s.palette, 3);
    assert!(!s.dark);
    assert_eq!(s.weekend, Weekend::SatSun);
}

#[test]
fn radar_uses_the_payday_cycle() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    store
        .save_settings(&Settings {
            cycle_start_day: 27,
            ..Settings::default()
        })
        .unwrap();

    // previous cycle: 27 Sep – 26 Oct
    add(&store, cash, Kind::Income, "Salary", "5000", d(2026, 9, 27));
    add(&store, cash, Kind::Expense, "Food", "1000", d(2026, 10, 10));
    // current cycle: 27 Oct – 26 Nov
    add(
        &store,
        cash,
        Kind::Income,
        "Salary",
        "6000",
        d(2026, 10, 27),
    );
    add(&store, cash, Kind::Expense, "Food", "300", d(2026, 10, 28));
    add(
        &store,
        cash,
        Kind::Expense,
        "Transport",
        "200",
        d(2026, 10, 31),
    );
    add(&store, cash, Kind::Expense, "Food", "100", d(2026, 10, 31));

    let today = d(2026, 10, 31);
    let r = Radar::compute(&store, today).unwrap();
    assert_eq!(r.period.start, d(2026, 10, 27));
    assert_eq!(r.income, Money(600_000));
    assert_eq!(r.expense, Money(60_000));
    assert_eq!(r.prev_income, Money(500_000));
    assert_eq!(r.prev_expense, Money(100_000));
    assert_eq!(r.income_change_pct(), Some(20));
    assert_eq!(r.expense_change_pct(), Some(-40));
    // 27 days left including today (31 Oct .. 26 Nov)
    assert_eq!(r.days_left, 27);
    // allowance is fixed at the start of the day: today's 300 is added back
    assert_eq!(r.spent_today, Money(30_000));
    assert_eq!(r.safe_per_day, Money(570_000 / 27));
    assert_eq!(r.today_left, Money(570_000 / 27 - 30_000));
    // typical day before today is 0 (one spike dropped), so the projection is
    // what is already spent
    assert_eq!(r.projected_expense, Money(60_000));
    assert_eq!(r.savings_rate(), Some(0.9));

    assert_eq!(r.last_7_days.len(), 7);
    assert_eq!(r.last_7_days.first().unwrap().0, d(2026, 10, 25));
    assert_eq!(r.last_7_days.last().unwrap(), &(today, Money(30_000)));
    assert_eq!(r.last_7_days[3], (d(2026, 10, 28), Money(30_000)));
    assert_eq!(r.last_7_days[0].1, Money(0));

    assert_eq!(r.top_expenses[0].name, "Food");
    assert_eq!(r.top_expenses[0].total, Money(40_000));
    assert_eq!(r.income_sources.len(), 1);
}

#[test]
fn radar_without_income_falls_back_to_balance() {
    let store = Store::open_in_memory().unwrap();
    store
        .add_account("Bank", AccountKind::Bank, Money(310_000))
        .unwrap();
    let r = Radar::compute(&store, d(2026, 10, 1)).unwrap();
    assert_eq!(r.days_left, 31);
    assert_eq!(r.safe_per_day, Money(10_000));
    assert!(!r.has_entries);
    assert_eq!(r.savings_rate(), None);
    assert_eq!(r.spent_ratio(), 0.0);
}

#[test]
fn budgets_track_spending() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    let food = category(&store, Kind::Expense, "Food");
    store.set_budget(food, Some(Money(50_000))).unwrap();
    add(&store, cash, Kind::Expense, "Food", "120", d(2026, 10, 5));
    let p = Period::cycle_containing(d(2026, 10, 5), 1);
    let b = store.budgets(p).unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!((b[0].1, b[0].2), (Money(50_000), Money(12_000)));
    store.set_budget(food, None).unwrap();
    assert!(store.budgets(p).unwrap().is_empty());
}

#[test]
fn rent_on_day_one_does_not_explode_the_projection() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    add(
        &store,
        cash,
        Kind::Income,
        "Salary",
        "10000",
        d(2026, 10, 1),
    );
    add(&store, cash, Kind::Expense, "Home", "4500", d(2026, 10, 1));
    for day in 2..=5 {
        add(&store, cash, Kind::Expense, "Food", "50", d(2026, 10, day));
    }
    let r = Radar::compute(&store, d(2026, 10, 5)).unwrap();
    // typical day = mean of [4500, 50, 50, 50] without the biggest = 50
    // projected = 4650 before today + 50 today + 26 more days x 50
    assert_eq!(r.projected_expense, Money(465_000 + 5_000 + 26 * 5_000));
}

#[test]
fn edit_search_and_suggestions() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    let food = category(&store, Kind::Expense, "Food");
    let mk = |minute: u16, note: &str, day: u32| NewTransaction {
        account_id: cash,
        category_id: food,
        kind: Kind::Expense,
        amount: Money(1_800),
        date: d(2026, 10, day),
        minute,
        note: note.into(),
    };
    for day in 1..=5 {
        store
            .add_transaction(&mk(8 * 60 + day as u16, "Morning coffee", day))
            .unwrap();
    }
    let lunch = store.add_transaction(&mk(13 * 60, "Lunch", 6)).unwrap();
    store.add_transaction(&mk(13 * 60, "lunch", 7)).unwrap();

    // morning: coffee first
    let s = store
        .suggestions(Kind::Expense, d(2026, 10, 8), 8 * 60, 3)
        .unwrap();
    assert_eq!(s[0].note, "Morning coffee");
    assert_eq!(s[0].uses, 5);
    // lunch time: lunch (case-insensitive group) ranks first despite fewer uses
    let s = store
        .suggestions(Kind::Expense, d(2026, 10, 8), 13 * 60, 3)
        .unwrap();
    assert_eq!(s[0].note.to_lowercase(), "lunch");

    // edit keeps the id, changes the amount and time
    let mut changed = mk(14 * 60, "Lunch with team", 6);
    changed.amount = Money(5_000);
    store.update_transaction(lunch, &changed).unwrap();
    let t = store.transaction(lunch).unwrap().unwrap();
    assert_eq!(
        (t.amount, t.minute, t.note.as_str()),
        (Money(5_000), 14 * 60, "Lunch with team")
    );
    assert!(store.update_transaction(9999, &changed).is_err());

    // search matches notes and categories; % is literal
    assert_eq!(store.search(None, "coffee", 50).unwrap().len(), 5);
    assert_eq!(store.search(None, "FOOD", 50).unwrap().len(), 7);
    assert_eq!(store.search(None, "%", 50).unwrap().len(), 0);
    // newest first, by minute within a day
    let all = store.transactions(None, 50).unwrap();
    assert!(all.windows(2).all(|w| w[0].stamp() >= w[1].stamp()));
}

#[test]
fn notes_and_names_are_cleaned() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    let id = store
        .add_transaction(&NewTransaction {
            account_id: cash,
            category_id: category(&store, Kind::Expense, "Food"),
            kind: Kind::Expense,
            amount: Money(100),
            date: d(2026, 10, 1),
            minute: 0,
            note: format!("  two   spaces\n{}", "x".repeat(500)),
        })
        .unwrap();
    let t = store.transaction(id).unwrap().unwrap();
    assert!(t.note.starts_with("two spaces "));
    assert_eq!(t.note.chars().count(), 120);
    assert!(
        store
            .add_account("cash", AccountKind::Cash, Money(0))
            .is_err(),
        "duplicate name"
    );
    assert!(store
        .add_account("Bank", AccountKind::Bank, Money(i64::MAX))
        .is_err());
}

#[test]
fn archived_accounts_keep_history_but_take_no_new_entries() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    assert!(store.archive_account(cash).is_err(), "last account");
    let bank = store
        .add_account("Bank", AccountKind::Bank, Money(0))
        .unwrap();
    add(&store, bank, Kind::Expense, "Food", "10", d(2026, 10, 1));
    store.archive_account(bank).unwrap();
    assert_eq!(store.accounts_with_balances().unwrap().len(), 1);
    assert_eq!(store.transactions(None, 10).unwrap().len(), 1);
    let food = category(&store, Kind::Expense, "Food");
    assert!(store
        .add_transaction(&NewTransaction {
            account_id: bank,
            category_id: food,
            kind: Kind::Expense,
            amount: Money(100),
            date: d(2026, 10, 2),
            minute: 0,
            note: String::new(),
        })
        .is_err());
}

#[test]
fn stats_are_internally_consistent() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    let food = category(&store, Kind::Expense, "Food");
    let salary = category(&store, Kind::Income, "Salary");
    store.set_budget(food, Some(Money(10_000))).unwrap();
    let entry = |kind, category_id, amount, day, minute| NewTransaction {
        account_id: cash,
        category_id,
        kind,
        amount: Money(amount),
        date: d(2026, 10, day),
        minute,
        note: String::new(),
    };
    store
        .add_transaction(&entry(Kind::Income, salary, 1_000_000, 1, 9 * 60))
        .unwrap();
    store
        .add_transaction(&entry(Kind::Expense, food, 6_000, 2, 13 * 60 + 15))
        .unwrap();
    store
        .add_transaction(&entry(Kind::Expense, food, 3_000, 2, 21 * 60))
        .unwrap();
    store
        .add_transaction(&entry(Kind::Expense, food, 3_000, 9, 13 * 60 + 40))
        .unwrap();

    let today = d(2026, 10, 10);
    let s = Stats::compute(&store, today, 12 * 60, Range::ThisCycle).unwrap();
    assert_eq!(s.expense, Money(12_000));
    assert_eq!(s.income, Money(1_000_000));
    assert_eq!(s.by_hour.iter().copied().sum::<Money>(), s.expense);
    assert_eq!(s.by_weekday.iter().copied().sum::<Money>(), s.expense);
    assert_eq!(
        s.heatmap.iter().flatten().copied().sum::<Money>(),
        s.expense
    );
    assert_eq!(
        s.categories.iter().map(|c| c.total).sum::<Money>(),
        s.expense
    );
    assert_eq!(
        s.buckets.iter().map(|b| b.expense).sum::<Money>(),
        s.expense
    );
    assert_eq!(s.buckets.len(), 31);
    assert_eq!(s.peak_hour, Some(13));
    assert_eq!(s.expense_count, 3);
    assert_eq!(s.median_expense, Money(3_000));
    assert_eq!(s.largest[0].amount, Money(6_000));
    // 9 full days + 12 h + 1 min
    assert_eq!(s.elapsed_minutes, 9 * 1440 + 721);
    assert_eq!(s.burn_per_minute, Money(12_000 / (9 * 1440 + 721)));
    // no-spend: days 1,3..8,10 → 8 days; longest run 3..8 = 6
    assert_eq!(s.no_spend_days, 8);
    assert_eq!(s.longest_no_spend, 6);
    assert_eq!(s.current_no_spend, 1);
    // 3_000 of 12_000 after 20:00
    assert!((s.night_share - 0.25).abs() < 1e-6);
    // budget 100, spent 120: warning insight
    assert!(s
        .insights
        .iter()
        .any(|i| i.warning && i.text.contains("Food is over budget")));
    // working hours: Oct 2026 days 1..10, weekend Fri-Sat (2,3,9,10 → Fri 2, Sat 3, Fri 9, Sat 10)
    assert_eq!(s.work_hours, 6 * 8);
    assert_eq!(s.income_per_work_hour, Some(Money(1_000_000 / 48)));

    for range in Range::ALL {
        let s = Stats::compute(&store, today, 600, range).unwrap();
        assert_eq!(
            s.buckets.iter().map(|b| b.expense).sum::<Money>(),
            s.expense,
            "{range:?}"
        );
    }
}
