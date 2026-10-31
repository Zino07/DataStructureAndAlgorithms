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
        assert_eq!(store.categories(Kind::Expense).unwrap().len(), 9);
        assert_eq!(store.categories(Kind::Income).unwrap().len(), 5);
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.accounts_with_balances().unwrap().len(), 1);
    assert_eq!(store.categories(Kind::Expense).unwrap().len(), 9);
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
            dark: true,
        })
        .unwrap();
    let s = store.settings().unwrap();
    assert_eq!(s.currency, "AED");
    assert_eq!(s.cycle_start_day, 28);
    assert_eq!(s.palette, 3);
    assert!(s.dark);
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
    assert_eq!(r.safe_per_day, Money(540_000 / 27));
    // 5 days elapsed of a 31-day cycle
    assert_eq!(r.projected_expense, Money(60_000 * 31 / 5));
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
