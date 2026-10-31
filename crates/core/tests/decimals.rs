//! Three-decimal currencies (KWD, BHD, OMR...). Own test binary: the number of
//! decimals is process-wide, so it must not share a process with other tests.

use income_radar_core::*;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

#[test]
fn kuwaiti_dinar_keeps_fils_and_switching_currency_keeps_value() {
    let store = Store::open_in_memory().unwrap();
    let cash = store.accounts_with_balances().unwrap()[0].account.id;
    let food = store.categories(Kind::Expense).unwrap()[0].id;

    // SAR first: two decimals
    assert_eq!(Money::decimals(), 2);
    assert_eq!(Money::parse("1.234"), Err(ParseMoneyError::TooManyDecimals));
    store
        .add_transaction(&NewTransaction {
            account_id: cash,
            category_id: food,
            kind: Kind::Expense,
            amount: Money::parse("12.50").unwrap(),
            date: d(1),
            minute: 600,
            note: String::new(),
        })
        .unwrap();
    store
        .set_budget(food, Some(Money::parse("100").unwrap()))
        .unwrap();

    // switch to KWD: the same 12.5 dinars, now with three decimals
    let mut s = store.settings().unwrap();
    s.currency = "KWD".into();
    store.save_settings(&s).unwrap();
    assert_eq!(Money::decimals(), 3);
    let t = &store.transactions(None, 10).unwrap()[0];
    assert_eq!(t.amount.format(), "12.500");
    assert_eq!(t.amount.plain(), "12.500");
    assert_eq!(store.budget_limits().unwrap()[&food].format(), "100.000");

    // fils can be entered now
    assert_eq!(Money::parse("1.234").unwrap(), Money(1234));
    assert_eq!(Money::parse("0.005").unwrap(), Money(5));
    assert_eq!(
        Money::parse("1.2345"),
        Err(ParseMoneyError::TooManyDecimals)
    );
    store
        .add_transaction(&NewTransaction {
            account_id: cash,
            category_id: food,
            kind: Kind::Expense,
            amount: Money::parse("0.005").unwrap(),
            date: d(2),
            minute: 600,
            note: String::new(),
        })
        .unwrap();
    assert_eq!(store.total_balance().unwrap().format(), "-12.505");
    assert_eq!(Money(1_234_567).format_compact(), "1,234.567");

    // back to SAR: 0.005 KWD rounds to 0.01 (never to zero), 12.5 stays 12.50
    s.currency = "SAR".into();
    store.save_settings(&s).unwrap();
    assert_eq!(Money::decimals(), 2);
    let amounts: Vec<String> = store
        .transactions(None, 10)
        .unwrap()
        .iter()
        .map(|t| t.amount.format())
        .collect();
    assert_eq!(amounts, vec!["0.01", "12.50"]);

    // a currency without decimals (JPY): 12.50 becomes 13 (rounded)
    s.currency = "JPY".into();
    store.save_settings(&s).unwrap();
    assert_eq!(Money::decimals(), 0);
    assert_eq!(Money::parse("1500").unwrap(), Money(1500));
    assert_eq!(Money::parse("15.5"), Err(ParseMoneyError::TooManyDecimals));
    assert_eq!(Money(1500).format(), "1,500");
    let amounts: Vec<String> = store
        .transactions(None, 10)
        .unwrap()
        .iter()
        .map(|t| t.amount.format())
        .collect();
    assert_eq!(amounts, vec!["1", "13"]);

    // the keypad calculator follows the currency
    assert_eq!(calc::eval("100÷3").unwrap(), Money(33));
    s.currency = "KWD".into();
    store.save_settings(&s).unwrap();
    assert_eq!(calc::eval("100÷3").unwrap(), Money(33_333));
    s.currency = "SAR".into();
    store.save_settings(&s).unwrap();
}
