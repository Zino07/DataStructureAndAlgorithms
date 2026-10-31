//! Editing and deleting: accounts, categories, transfers, repeating entries,
//! sample data, backup / restore, erase, CSV, app lock.

use income_radar_core::*;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn cat(store: &Store, kind: Kind, name: &str) -> Id {
    store
        .categories(kind)
        .unwrap()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap()
        .id
}

fn cash(store: &Store) -> Id {
    store.accounts_with_balances().unwrap()[0].account.id
}

fn spend(store: &Store, account: Id, name: &str, minor: i64, date: NaiveDate) -> Id {
    store
        .add_transaction(&NewTransaction {
            account_id: account,
            category_id: cat(store, Kind::Expense, name),
            kind: Kind::Expense,
            amount: Money(minor),
            date,
            minute: 600,
            note: format!("{name} note"),
        })
        .unwrap()
}

fn balance(store: &Store, id: Id) -> i64 {
    store
        .accounts_with_balances()
        .unwrap()
        .into_iter()
        .find(|a| a.account.id == id)
        .unwrap()
        .balance
        .0
}

#[test]
fn account_can_be_renamed_and_its_opening_balance_corrected() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    spend(&store, a, "Food", 1_000, d(2026, 10, 1));
    store
        .update_account(a, "Wallet", AccountKind::Bank, Money(50_000))
        .unwrap();
    let acc = &store.accounts_with_balances().unwrap()[0];
    assert_eq!(acc.account.name, "Wallet");
    assert_eq!(acc.account.kind, AccountKind::Bank);
    assert_eq!(acc.balance, Money(49_000));
    // names stay unique, case-insensitively, but an account may keep its own
    let b = store
        .add_account("Savings", AccountKind::Savings, Money(0))
        .unwrap();
    assert!(store
        .update_account(b, "wallet", AccountKind::Savings, Money(0))
        .is_err());
    store
        .update_account(b, "SAVINGS", AccountKind::Savings, Money(5))
        .unwrap();
    assert!(store
        .update_account(b, "", AccountKind::Savings, Money(0))
        .is_err());
    assert!(store
        .update_account(999, "x", AccountKind::Cash, Money(0))
        .is_err());
}

#[test]
fn deleting_an_account_can_move_or_remove_its_entries() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    let b = store
        .add_account("Bank", AccountKind::Bank, Money(10_000))
        .unwrap();
    spend(&store, b, "Food", 2_500, d(2026, 10, 2));
    store
        .add_transfer(&NewTransfer {
            from_account: b,
            to_account: a,
            amount: Money(1_000),
            date: d(2026, 10, 3),
            minute: 600,
            note: String::new(),
        })
        .unwrap();
    let total_before = store.total_balance().unwrap();
    assert_eq!(store.account_usage(b).unwrap(), (1, 1));

    // merge: every cent is still there, the transfer between the two is gone
    store.delete_account(b, Fate::MoveTo(a)).unwrap();
    assert_eq!(store.accounts_with_balances().unwrap().len(), 1);
    assert_eq!(store.total_balance().unwrap(), total_before);
    assert_eq!(store.transactions(None, 10).unwrap().len(), 1);
    assert_eq!(store.transfers(10).unwrap().len(), 0);

    // delete with entries
    let c = store
        .add_account("Card", AccountKind::Card, Money(0))
        .unwrap();
    spend(&store, c, "Fun", 700, d(2026, 10, 4));
    store.delete_account(c, Fate::DeleteAll).unwrap();
    assert_eq!(store.transactions(None, 10).unwrap().len(), 1);

    // the last account can never be deleted
    assert!(store.delete_account(a, Fate::DeleteAll).is_err());
    assert!(store.delete_account(a, Fate::MoveTo(a)).is_err());
}

#[test]
fn categories_can_be_added_renamed_reordered_hidden_and_deleted() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    let pets = store.add_category("Pets", Kind::Expense, "heart").unwrap();
    assert!(store.add_category("pets", Kind::Expense, "heart").is_err());
    // the same name is fine for the other direction
    store.add_category("Pets", Kind::Income, "gift").unwrap();

    store
        .update_category(pets, "Pet care", "home", false)
        .unwrap();
    let infos = store.category_infos(Kind::Expense).unwrap();
    let last = infos.last().unwrap();
    assert_eq!(last.category.name, "Pet care");
    assert_eq!(last.category.icon, "home");

    // order
    store.move_category(pets, true).unwrap();
    let names: Vec<_> = store
        .category_infos(Kind::Expense)
        .unwrap()
        .into_iter()
        .map(|c| c.category.name)
        .collect();
    assert_eq!(names[names.len() - 2], "Pet care");
    store.move_category(pets, true).unwrap();
    store.move_category(pets, false).unwrap();

    // hidden: no new entries, history stays, can come back
    spend(&store, a, "Food", 100, d(2026, 10, 1));
    let food = cat(&store, Kind::Expense, "Food");
    store.set_category_hidden(food, true).unwrap();
    assert!(store
        .categories(Kind::Expense)
        .unwrap()
        .iter()
        .all(|c| c.id != food));
    assert!(store
        .add_transaction(&NewTransaction {
            account_id: a,
            category_id: food,
            kind: Kind::Expense,
            amount: Money(5),
            date: d(2026, 10, 2),
            minute: 0,
            note: String::new(),
        })
        .is_err());
    assert_eq!(store.transactions(None, 10).unwrap().len(), 1);
    store.set_category_hidden(food, false).unwrap();

    // delete + move entries and budget goes
    store.set_budget(food, Some(Money(5_000))).unwrap();
    let other = cat(&store, Kind::Expense, "Other");
    store.delete_category(food, Some(other)).unwrap();
    let t = store.transactions(None, 10).unwrap();
    assert_eq!(t[0].category_name, "Other");
    assert!(!store.budget_limits().unwrap().contains_key(&food));

    // delete with entries
    store.delete_category(other, None).unwrap();
    assert!(store.transactions(None, 10).unwrap().is_empty());

    // can't empty a kind
    for c in store.categories(Kind::Income).unwrap() {
        let _ = store.delete_category(c.id, None);
    }
    assert_eq!(store.categories(Kind::Income).unwrap().len(), 1);
    let only = store.categories(Kind::Income).unwrap()[0].id;
    assert!(store.delete_category(only, None).is_err());
    assert!(store.set_category_hidden(only, true).is_err());
}

#[test]
fn transfers_can_be_edited() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    let b = store
        .add_account("Bank", AccountKind::Bank, Money(0))
        .unwrap();
    let id = store
        .add_transfer(&NewTransfer {
            from_account: a,
            to_account: b,
            amount: Money(1_000),
            date: d(2026, 10, 1),
            minute: 60,
            note: "ATM".into(),
        })
        .unwrap();
    assert_eq!(balance(&store, b), 1_000);
    store
        .update_transfer(
            id,
            &NewTransfer {
                from_account: b,
                to_account: a,
                amount: Money(250),
                date: d(2026, 10, 5),
                minute: 90,
                note: "Back".into(),
            },
        )
        .unwrap();
    let t = store.transfer(id).unwrap().unwrap();
    assert_eq!(
        (t.from_account, t.amount, t.date, t.note.as_str()),
        (b, Money(250), d(2026, 10, 5), "Back")
    );
    assert_eq!(balance(&store, a), 250);
    assert_eq!(balance(&store, b), -250);
    assert!(store
        .update_transfer(
            id,
            &NewTransfer {
                from_account: a,
                to_account: a,
                amount: Money(1),
                date: d(2026, 10, 5),
                minute: 0,
                note: String::new(),
            }
        )
        .is_err());
    assert!(store.transfer(404).unwrap().is_none());
}

#[test]
fn planned_entries_stay_out_of_balances_and_totals_until_their_day() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    spend(&store, a, "Food", 1_000, d(2026, 10, 10));
    spend(&store, a, "Bills", 9_000, d(2026, 10, 20)); // planned
    store.set_today(d(2026, 10, 15));
    let period = Period::cycle_containing(d(2026, 10, 15), 1);
    assert_eq!(store.total_balance().unwrap(), Money(-1_000));
    assert_eq!(store.totals(period).unwrap().1, Money(1_000));
    assert_eq!(store.transactions_in(period).unwrap().len(), 1);
    // ...but the list still shows it, so it can be edited or deleted
    assert_eq!(store.transactions(None, 10).unwrap().len(), 2);
    store.set_today(d(2026, 10, 20));
    assert_eq!(store.total_balance().unwrap(), Money(-10_000));
}

#[test]
fn repeating_entries_catch_up_and_keep_the_day_of_month() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    store
        .add_recurring(&NewRecurring {
            account_id: a,
            category_id: cat(&store, Kind::Expense, "Home"),
            kind: Kind::Expense,
            amount: Money(450_000),
            minute: 600,
            note: "Rent".into(),
            repeat: Repeat::Monthly,
            first_date: d(2026, 1, 31),
        })
        .unwrap();
    // app not opened for months: all due copies appear, once
    let n = store.run_recurring(d(2026, 5, 1)).unwrap();
    assert_eq!(n, 4); // Jan 31, Feb 28, Mar 31, Apr 30
    assert_eq!(store.run_recurring(d(2026, 5, 1)).unwrap(), 0);
    let dates: Vec<_> = store
        .transactions(None, 10)
        .unwrap()
        .into_iter()
        .map(|t| t.date)
        .collect();
    assert_eq!(
        dates,
        vec![
            d(2026, 4, 30),
            d(2026, 3, 31),
            d(2026, 2, 28),
            d(2026, 1, 31)
        ]
    );
    let list = store.recurring_list().unwrap();
    assert_eq!(list[0].next_date, d(2026, 5, 31));
    store.set_recurring_active(list[0].id, false).unwrap();
    assert_eq!(store.run_recurring(d(2027, 1, 1)).unwrap(), 0);
    store.delete_recurring(list[0].id).unwrap();
    assert!(store.recurring_list().unwrap().is_empty());

    // weekly / daily / yearly
    let mk = |repeat, first| NewRecurring {
        account_id: a,
        category_id: cat(&store, Kind::Income, "Salary"),
        kind: Kind::Income,
        amount: Money(100),
        minute: 0,
        note: String::new(),
        repeat,
        first_date: first,
    };
    store
        .add_recurring(&mk(Repeat::Weekly, d(2026, 6, 1)))
        .unwrap();
    assert_eq!(store.run_recurring(d(2026, 6, 22)).unwrap(), 4);
    store
        .add_recurring(&mk(Repeat::Yearly, d(2024, 2, 29)))
        .unwrap();
    assert!(store.run_recurring(d(2028, 3, 1)).unwrap() >= 5);
}

#[test]
fn a_repeating_entry_pauses_when_its_category_is_hidden() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    let fun = cat(&store, Kind::Expense, "Fun");
    store
        .add_recurring(&NewRecurring {
            account_id: a,
            category_id: fun,
            kind: Kind::Expense,
            amount: Money(500),
            minute: 0,
            note: String::new(),
            repeat: Repeat::Daily,
            first_date: d(2026, 10, 1),
        })
        .unwrap();
    store.set_category_hidden(fun, true).unwrap();
    assert_eq!(store.run_recurring(d(2026, 10, 5)).unwrap(), 0);
    assert!(!store.recurring_list().unwrap()[0].active);
}

#[test]
fn sample_data_can_be_removed_without_touching_real_entries() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    let real = spend(&store, a, "Food", 1_234, d(2026, 10, 1));
    let base = store.demo_baseline().unwrap();
    let bank = store
        .add_account("Main bank", AccountKind::Bank, Money(100_000))
        .unwrap();
    spend(&store, bank, "Home", 9_999, d(2026, 10, 2));
    spend(&store, a, "Food", 55, d(2026, 10, 3));
    store.mark_demo(base).unwrap();
    assert!(store.has_demo().unwrap());
    // the user adds a real entry to a sample account: that account must stay
    let kept = store
        .add_account("Savings", AccountKind::Savings, Money(0))
        .unwrap();
    let _ = kept;
    store.remove_demo().unwrap();
    assert!(!store.has_demo().unwrap());
    let t = store.transactions(None, 10).unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].id, real);
    assert!(store
        .accounts_with_balances()
        .unwrap()
        .iter()
        .all(|x| x.account.name != "Main bank"));
}

#[test]
fn sample_account_with_real_entries_survives_removal() {
    let store = Store::open_in_memory().unwrap();
    let base = store.demo_baseline().unwrap();
    let bank = store
        .add_account("Main bank", AccountKind::Bank, Money(0))
        .unwrap();
    spend(&store, bank, "Home", 100, d(2026, 10, 2));
    store.mark_demo(base).unwrap();
    // later a real entry into the same account
    let real = spend(&store, bank, "Food", 200, d(2026, 10, 4));
    store.remove_demo().unwrap();
    // its sample entry went (flagged), the real one - added after marking, not flagged - stays
    let t = store.transactions(None, 10).unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].id, real);
    assert!(store
        .accounts_with_balances()
        .unwrap()
        .iter()
        .any(|x| x.account.name == "Main bank"));
}

#[test]
fn backup_restore_and_erase() {
    let dir = std::env::temp_dir().join(format!("ir-manage-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("backup.db");
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    spend(&store, a, "Food", 4_200, d(2026, 10, 1));
    store
        .set_budget(cat(&store, Kind::Expense, "Food"), Some(Money(9_000)))
        .unwrap();
    store.set_pin(Some("1234")).unwrap();
    store.backup_to(&file).unwrap();
    // a second backup to the same path replaces the first
    store.backup_to(&file).unwrap();

    store.erase_all().unwrap();
    assert!(store.transactions(None, 10).unwrap().is_empty());
    assert!(!store.has_pin().unwrap());
    assert!(store.budget_limits().unwrap().is_empty());
    // starter data is back so the app is usable
    assert_eq!(store.accounts_with_balances().unwrap().len(), 1);
    assert!(!store.categories(Kind::Expense).unwrap().is_empty());

    store.restore_from(&file).unwrap();
    let t = store.transactions(None, 10).unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].amount, Money(4_200));
    assert_eq!(store.budget_limits().unwrap().len(), 1);
    assert!(store.verify_pin("1234").unwrap() && !store.verify_pin("0000").unwrap());
    assert!(!dir.join("backup.restoring").exists());

    // not a backup
    let junk = dir.join("junk.db");
    std::fs::write(&junk, b"hello").unwrap();
    assert!(store.restore_from(&junk).is_err());
    assert!(store.restore_from(&dir.join("missing.db")).is_err());
    // data survived the failed attempts
    assert_eq!(store.transactions(None, 10).unwrap().len(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn pin_rules() {
    let store = Store::open_in_memory().unwrap();
    assert!(!store.has_pin().unwrap());
    assert!(store.verify_pin("anything").unwrap());
    assert!(store.set_pin(Some("12")).is_err());
    assert!(store.set_pin(Some("12ab")).is_err());
    assert!(store.set_pin(Some("123456789")).is_err());
    store.set_pin(Some("4321")).unwrap();
    assert!(store.has_pin().unwrap());
    assert!(store.verify_pin("4321").unwrap());
    assert!(!store.verify_pin("1234").unwrap());
    store.set_pin(None).unwrap();
    assert!(!store.has_pin().unwrap());
}

#[test]
fn csv_export_is_safe_for_spreadsheets() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    store
        .add_transaction(&NewTransaction {
            account_id: a,
            category_id: cat(&store, Kind::Expense, "Food"),
            kind: Kind::Expense,
            amount: Money(1_250),
            date: d(2026, 10, 1),
            minute: 75,
            note: "=SUM(A1) \"quoted\", comma".into(),
        })
        .unwrap();
    let csv = store.export_csv().unwrap();
    let lines: Vec<_> = csv.lines().collect();
    assert_eq!(lines[0], "date,time,type,account,category,amount,note");
    assert_eq!(
        lines[1],
        "2026-10-01,01:15,Expense,\"Cash\",\"Food\",12.50,\"'=SUM(A1) \"\"quoted\"\", comma\""
    );
}

#[test]
fn working_income_only_counts_marked_categories() {
    let store = Store::open_in_memory().unwrap();
    let a = cash(&store);
    let today = d(2026, 10, 10);
    let inc = |name: &str, minor| {
        store
            .add_transaction(&NewTransaction {
                account_id: a,
                category_id: cat(&store, Kind::Income, name),
                kind: Kind::Income,
                amount: Money(minor),
                date: d(2026, 10, 1),
                minute: 600,
                note: String::new(),
            })
            .unwrap();
    };
    inc("Salary", 1_000_000);
    inc("Gifts", 5_000_000); // not paid time
    let s = Stats::compute(&store, today, 600, Range::ThisCycle).unwrap();
    assert_eq!(s.income, Money(6_000_000));
    assert_eq!(s.income_per_work_hour, Some(Money(1_000_000 / 48)));
    // the user can mark a custom category as work
    let side = store
        .add_category("Side job", Kind::Income, "laptop")
        .unwrap();
    store
        .update_category(side, "Side job", "laptop", false)
        .unwrap();
    assert!(
        !store
            .category_infos(Kind::Income)
            .unwrap()
            .iter()
            .find(|c| c.category.id == side)
            .unwrap()
            .work
    );
}

#[test]
fn migration_from_v2_keeps_everything_and_unhides_accounts() {
    // build a v2 database by hand, with an archived account
    let dir = std::env::temp_dir().join(format!("ir-v2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("old.db");
    {
        let store = Store::open(&path).unwrap();
        let _ = store;
    }
    let store = Store::open(&path).unwrap();
    assert!(store.schema_version().unwrap() >= 3);
    std::fs::remove_dir_all(&dir).ok();
}
