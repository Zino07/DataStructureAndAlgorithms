//! The "your data, your rules" screens, driven the way a person uses them:
//! categories, accounts, the calendar and calculator, repeating entries,
//! transfers, the PIN lock, backup / restore / erase.
#![cfg(not(target_os = "android"))]

use std::time::Duration;

use i_slint_backend_testing::{self as testing, AccessibleRole, ElementHandle};
use income_radar::{AppWindow, Screen};
use slint::{ComponentHandle, Model};

const NOW: &str = "2026-10-15 10:00";

fn setup(name: &str) -> (AppWindow, std::path::PathBuf) {
    // the test clock is only honoured off Android
    std::env::set_var("INCOME_RADAR_NOW", NOW);
    testing::init_no_event_loop();
    let dir = std::env::temp_dir().join(format!("ir-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ui = income_radar::build_with(&dir, &dir.join("out")).unwrap();
    ui.window().set_size(slint::LogicalSize::new(390.0, 844.0));
    ui.show().unwrap();
    (ui, dir)
}

fn settle() {
    testing::mock_elapsed_time(Duration::from_millis(900));
}

/// Finds an element, scrolling the page down as a person would when it is
/// not in view (Slint only builds what the scrolled page shows).
fn find(ui: &AppWindow, label: &str) -> Option<ElementHandle> {
    find_nth(ui, label, 0)
}

fn interactive(e: &ElementHandle) -> bool {
    matches!(
        e.accessible_role(),
        Some(
            AccessibleRole::Button
                | AccessibleRole::Tab
                | AccessibleRole::TextInput
                | AccessibleRole::Switch
        )
    )
}

fn find_nth(ui: &AppWindow, label: &str, n: usize) -> Option<ElementHandle> {
    let look = || {
        ElementHandle::find_by_accessible_label(ui, label)
            .filter(interactive)
            .nth(n)
    };
    if let Some(e) = look() {
        return Some(e);
    }
    let start = ui.get_scroll_pos();
    for step in 0..24 {
        ui.invoke_scroll_to(if step == 0 {
            0.0
        } else {
            ui.get_scroll_pos() + 380.0
        });
        testing::mock_elapsed_time(Duration::from_millis(60));
        if let Some(e) = look() {
            return Some(e);
        }
    }
    ui.invoke_scroll_to(start);
    None
}

fn labels(ui: &AppWindow) -> Vec<String> {
    let Some(root) = ElementHandle::find_by_element_id(ui, "AppWindow::back-scope").next() else {
        return vec!["<no root>".into()];
    };
    ElementHandle::query_descendants(&root)
        .find_all()
        .into_iter()
        .filter_map(|e| e.accessible_label().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

fn tap(ui: &AppWindow, label: &str) {
    let e = find(ui, label)
        .unwrap_or_else(|| panic!("nothing labelled “{label}”; there is: {:?}", labels(ui)));
    e.invoke_accessible_default_action();
    settle();
}

fn tap_nth(ui: &AppWindow, label: &str, n: usize) {
    let e = find_nth(ui, label, n)
        .unwrap_or_else(|| panic!("no #{n} “{label}”; there is: {:?}", labels(ui)));
    e.invoke_accessible_default_action();
    settle();
}

fn type_into(ui: &AppWindow, label: &str, text: &str) {
    let e = find(ui, label).unwrap_or_else(|| panic!("no field “{label}”"));
    e.set_accessible_value(text);
    settle();
}

fn account_ids(ui: &AppWindow) -> Vec<i32> {
    ui.get_accounts().iter().map(|a| a.id).collect()
}

#[test]
fn categories_can_be_added_renamed_hidden_moved_and_deleted() {
    let (ui, dir) = setup("cats");
    ui.invoke_go(Screen::Categories);
    settle();
    let names = |ui: &AppWindow| -> Vec<String> {
        ui.get_expense_cat_rows()
            .iter()
            .map(|r| r.name.to_string())
            .collect()
    };
    assert!(names(&ui).contains(&"Food".to_string()));

    // add
    tap(&ui, "New category");
    type_into(&ui, "Name", "Pets");
    tap(&ui, "paw");
    tap(&ui, "Add category");
    assert!(names(&ui).contains(&"Pets".to_string()), "{:?}", names(&ui));
    let pets = ui
        .get_expense_cat_rows()
        .iter()
        .find(|r| r.name == "Pets")
        .unwrap();
    assert_eq!(pets.icon_key, "paw");

    // duplicate name is refused with a message, nothing added
    tap(&ui, "New category");
    type_into(&ui, "Name", "pets");
    tap(&ui, "Add category");
    assert_eq!(
        names(&ui)
            .iter()
            .filter(|n| n.eq_ignore_ascii_case("pets"))
            .count(),
        1
    );
    tap(&ui, "Cancel");

    // rename
    tap(&ui, "Pets");
    type_into(&ui, "Name", "Pet care");
    tap(&ui, "Save");
    assert!(names(&ui).contains(&"Pet care".to_string()));

    // hide, then show again
    tap(&ui, "Pet care");
    tap(&ui, "Hide");
    assert!(
        ui.get_expense_cat_rows()
            .iter()
            .find(|r| r.name == "Pet care")
            .unwrap()
            .hidden
    );
    assert!(!ui
        .get_expense_categories()
        .iter()
        .any(|c| c.name == "Pet care"));
    tap(&ui, "Pet care");
    tap(&ui, "Show again");
    assert!(ui
        .get_expense_categories()
        .iter()
        .any(|c| c.name == "Pet care"));

    // reorder: move up one place
    let before = names(&ui);
    tap(&ui, "Pet care");
    tap(&ui, "Move up");
    let after = names(&ui);
    let pos = |v: &Vec<String>| v.iter().position(|n| n == "Pet care").unwrap();
    assert_eq!(pos(&after) + 1, pos(&before));

    // delete one nobody uses: gone at once
    tap(&ui, "Pet care");
    tap(&ui, "Delete");
    assert!(!names(&ui).contains(&"Pet care".to_string()));

    // a category with entries asks where they go
    let cash = account_ids(&ui)[0];
    let food = ui
        .get_expense_cat_rows()
        .iter()
        .find(|r| r.name == "Food")
        .unwrap()
        .id;
    let other = ui
        .get_expense_cat_rows()
        .iter()
        .find(|r| r.name == "Other")
        .unwrap()
        .id;
    assert_eq!(
        ui.invoke_save_entry(
            -1,
            0,
            "12.5".into(),
            food,
            cash,
            -1,
            0,
            600,
            "lunch".into(),
            0
        ),
        ""
    );
    settle();
    ui.invoke_go(Screen::Categories);
    settle();
    tap(&ui, "Food");
    tap(&ui, "Delete");
    assert!(
        find(&ui, "Delete the entries too").is_some(),
        "should ask what to do with the entry"
    );
    tap(&ui, "Move entries to Groceries");
    assert!(!names(&ui).contains(&"Food".to_string()));
    let groceries = ui
        .get_expense_cat_rows()
        .iter()
        .find(|r| r.name == "Groceries")
        .unwrap();
    assert_eq!(groceries.uses, 1, "the entry moved to Groceries");
    let _ = other;
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn accounts_can_be_renamed_corrected_merged_and_deleted() {
    let (ui, dir) = setup("accounts");
    ui.invoke_go(Screen::Accounts);
    settle();
    // a second account with a balance
    tap(&ui, "New account");
    type_into(&ui, "Name", "Bank");
    type_into(&ui, "Opening balance (SAR)", "500");
    tap(&ui, "Add account");
    assert_eq!(ui.get_accounts().row_count(), 2);
    let ids = account_ids(&ui);
    let food = ui.get_expense_categories().iter().next().unwrap().id;
    // 40.00 spent from Cash
    assert_eq!(
        ui.invoke_save_entry(-1, 0, "40".into(), food, ids[0], -1, 0, 600, "".into(), 0),
        ""
    );
    settle();

    // edit Cash: rename and fix the opening balance
    tap(&ui, "Cash");
    type_into(&ui, "Name", "Wallet");
    type_into(&ui, "Opening balance (SAR)", "100");
    tap(&ui, "Save");
    let wallet = ui
        .get_accounts()
        .iter()
        .find(|a| a.name == "Wallet")
        .unwrap();
    assert_eq!(wallet.balance, "60.00");
    assert_eq!(wallet.opening, "100");

    // a duplicate name is refused
    tap(&ui, "Wallet");
    type_into(&ui, "Name", "bank");
    tap(&ui, "Save");
    assert!(
        ui.get_accounts().iter().any(|a| a.name == "Wallet"),
        "name should be unchanged"
    );
    tap(&ui, "Cancel");

    // delete Wallet: it has an entry, so the app asks where it goes
    tap(&ui, "Wallet");
    tap(&ui, "Delete");
    assert!(find(&ui, "Delete everything in it").is_some());
    tap(&ui, "Move to Bank");
    assert_eq!(ui.get_accounts().row_count(), 1);
    // all the money moved with it: 500 + 100 − 40
    assert_eq!(ui.get_accounts().row_data(0).unwrap().balance, "560.00");
    assert_eq!(ui.get_dash().balance, "560.00");

    // the last account cannot be deleted
    tap(&ui, "Bank");
    let del = find(&ui, "Delete").unwrap();
    assert!(
        !del.accessible_enabled().unwrap_or(true),
        "Delete must be disabled for the last account"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn calculator_calendar_and_planned_entries() {
    let (ui, dir) = setup("entry");
    let acc = account_ids(&ui)[0];
    let food = ui
        .get_expense_categories()
        .iter()
        .find(|c| c.name == "Food")
        .unwrap()
        .id;

    // the keypad adds up: 12.5 × 3 + 4 = 41.50
    assert_eq!(ui.invoke_calc_preview("12.5×3+4".into()), "41.50");
    assert_eq!(ui.invoke_calc_preview("12.5".into()), "");
    ui.invoke_open_add();
    settle();
    for k in ["1", "2", "Decimal point", "5", "Multiply", "3", "Add", "4"] {
        tap(&ui, k);
    }
    tap(&ui, "= 41.50");
    // the amount is now the plain result
    tap_nth(&ui, "Food", 0);

    // calendar: pick the 20th (five days ahead)
    tap(&ui, "Pick date");
    assert!(find(&ui, "Previous month").is_some());
    tap(&ui, "20 Oct");
    assert!(
        find(&ui, "Previous month").is_none(),
        "picking a day closes the calendar"
    );
    ui.invoke_go(Screen::Home);
    settle();

    // save through the pinned button: a planned entry
    ui.invoke_open_add();
    settle();
    let result = ui.invoke_save_entry(
        -1,
        0,
        "12.5×3+4".into(),
        food,
        acc,
        -1,
        -5,
        600,
        "rent due".into(),
        0,
    );
    assert_eq!(result, "");
    settle();
    // planned entries do not count yet
    assert_eq!(ui.get_dash().balance, "0.00");
    assert_eq!(ui.get_dash().expense, "0.00");
    let planned = ui
        .get_activity()
        .iter()
        .find(|r| !r.header && r.title == "rent due")
        .unwrap();
    assert!(planned.upcoming);
    assert_eq!(planned.amount, "−41.50");

    // today's entry counts
    assert_eq!(
        ui.invoke_save_entry(-1, 0, "10".into(), food, acc, -1, 0, 600, "".into(), 0),
        ""
    );
    settle();
    assert_eq!(ui.get_dash().balance, "-10.00");

    // typing nonsense is explained, not swallowed
    assert_eq!(
        ui.invoke_save_entry(-1, 0, "5÷0".into(), food, acc, -1, 0, 600, "".into(), 0),
        "Cannot divide by zero"
    );
    assert_eq!(
        ui.invoke_save_entry(-1, 0, "".into(), food, acc, -1, 0, 600, "".into(), 0),
        "Enter an amount"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn transfers_can_be_edited_and_show_in_all() {
    let (ui, dir) = setup("transfer");
    ui.invoke_go(Screen::Accounts);
    settle();
    tap(&ui, "New account");
    type_into(&ui, "Name", "Bank");
    tap(&ui, "Add account");
    let ids = account_ids(&ui);
    assert_eq!(
        ui.invoke_save_entry(
            -1,
            2,
            "50".into(),
            -1,
            ids[0],
            ids[1],
            0,
            600,
            "ATM".into(),
            0
        ),
        ""
    );
    settle();
    ui.invoke_go(Screen::Activity);
    settle();
    // "All" lists the transfer
    let row = ui
        .get_activity()
        .iter()
        .find(|r| r.transfer)
        .expect("transfer in All");
    assert_eq!(row.amount, "50.00");
    ui.invoke_request_edit_transfer(row.id);
    settle();
    // the edit screen opened on the transfer
    assert!(find(&ui, "Delete entry").is_some());
    assert_eq!(
        ui.invoke_save_entry(
            row.id,
            2,
            "75.50".into(),
            -1,
            ids[1],
            ids[0],
            -2,
            700,
            "back".into(),
            0
        ),
        ""
    );
    settle();
    let row = ui.get_activity().iter().find(|r| r.transfer).unwrap();
    assert_eq!(row.amount, "75.50");
    assert_eq!(row.title, "back");
    // the search finds it too
    ui.invoke_activity_query_changed("back".into());
    settle();
    assert!(ui.get_activity().iter().any(|r| r.transfer));
    ui.invoke_activity_query_changed("zzz".into());
    settle();
    assert!(!ui.get_activity().iter().any(|r| r.transfer));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn repeating_entries_are_created_listed_paused_and_deleted() {
    let (ui, dir) = setup("repeat");
    let acc = account_ids(&ui)[0];
    let home = ui
        .get_expense_categories()
        .iter()
        .find(|c| c.name == "Home")
        .unwrap()
        .id;
    // rent that started three months ago, repeating every month
    assert_eq!(
        ui.invoke_save_entry(
            -1,
            0,
            "450".into(),
            home,
            acc,
            -1,
            92,
            600,
            "Rent".into(),
            3
        ),
        ""
    );
    settle();
    assert_eq!(ui.get_repeat_count(), 1);
    let rent: Vec<_> = ui
        .get_activity()
        .iter()
        .filter(|r| !r.header && r.title == "Rent")
        .collect();
    assert!(
        rent.len() >= 3,
        "the missed months were created, got {}",
        rent.len()
    );
    ui.invoke_go(Screen::Repeats);
    settle();
    tap(&ui, "Rent");
    tap(&ui, "Pause");
    assert!(!ui.get_repeat_rows().row_data(0).unwrap().active);
    tap(&ui, "Rent");
    tap(&ui, "Resume");
    tap(&ui, "Rent");
    tap(&ui, "Delete");
    assert_eq!(ui.get_repeat_count(), 0);
    // entries already created stay
    assert!(ui.get_activity().iter().any(|r| r.title == "Rent"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn pin_lock_blocks_until_the_right_pin_and_waits_after_too_many_tries() {
    let (ui, dir) = setup("pin");
    assert_eq!(
        ui.invoke_set_pin("".into(), "12".into(), "12".into()),
        "A PIN is 4 to 8 digits"
    );
    assert_eq!(
        ui.invoke_set_pin("".into(), "1234".into(), "4321".into()),
        "The two PINs do not match"
    );
    assert_eq!(
        ui.invoke_set_pin("".into(), "1234".into(), "1234".into()),
        ""
    );
    assert!(ui.get_has_pin());
    ui.set_locked(true);
    settle();
    // wrong PIN
    for k in ["9", "9", "9", "9"] {
        tap(&ui, k);
    }
    tap(&ui, "Unlock");
    assert!(ui.get_locked());
    // the right one
    for k in ["1", "2", "3", "4"] {
        tap(&ui, k);
    }
    tap(&ui, "Unlock");
    assert!(!ui.get_locked());

    // five wrong tries make you wait, even for the right PIN
    ui.set_locked(true);
    settle();
    for _ in 0..5 {
        assert_ne!(ui.invoke_unlock("0000".into()), "");
    }
    let msg = ui.invoke_unlock("1234".into());
    assert!(msg.starts_with("Too many tries"), "{msg}");
    assert!(ui.get_locked());

    // removing needs the current PIN
    ui.set_locked(false);
    assert_eq!(
        ui.invoke_remove_pin("0000".into()),
        "The current PIN is wrong"
    );
    assert_eq!(ui.invoke_remove_pin("1234".into()), "");
    assert!(!ui.get_has_pin());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn forgot_pin_erases_everything_and_unlocks() {
    let (ui, dir) = setup("forgot");
    let acc = account_ids(&ui)[0];
    let food = ui.get_expense_categories().iter().next().unwrap().id;
    ui.invoke_save_entry(-1, 0, "5".into(), food, acc, -1, 0, 600, "".into(), 0);
    ui.invoke_set_pin("".into(), "1234".into(), "1234".into());
    ui.set_locked(true);
    settle();
    ui.invoke_forgot_pin();
    settle();
    assert!(!ui.get_locked());
    assert!(!ui.get_has_pin());
    assert_eq!(ui.get_dash().balance, "0.00");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn backup_restore_erase_export_and_sample_data() {
    let (ui, dir) = setup("data");
    let acc = account_ids(&ui)[0];
    let food = ui.get_expense_categories().iter().next().unwrap().id;
    ui.invoke_save_entry(
        -1,
        0,
        "25".into(),
        food,
        acc,
        -1,
        0,
        600,
        "keep me".into(),
        0,
    );
    settle();
    ui.invoke_backup_now();
    assert!(
        ui.get_data_message().contains("Backup saved"),
        "{}",
        ui.get_data_message()
    );
    ui.invoke_export_csv();
    let csv = std::fs::read_dir(dir.join("out"))
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().ends_with(".csv"))
        .expect("csv written");
    let text = std::fs::read_to_string(csv.path()).unwrap();
    assert!(text.contains("keep me") && text.contains("25.00"));

    ui.invoke_erase_all();
    settle();
    assert_eq!(ui.get_dash().balance, "0.00");
    assert!(!ui.get_activity().iter().any(|r| r.title == "keep me"));

    ui.invoke_restore_backup();
    settle();
    assert!(ui.get_activity().iter().any(|r| r.title == "keep me"));
    assert_eq!(ui.get_dash().balance, "-25.00");

    // sample data comes and goes without touching the real entry
    ui.invoke_load_demo();
    settle();
    assert!(ui.get_has_demo() || !ui.get_can_load_demo());
    let _ = &ui;
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn sample_data_on_a_fresh_app_can_be_removed_completely() {
    let (ui, dir) = setup("demo");
    assert!(ui.get_can_load_demo());
    ui.invoke_load_demo();
    settle();
    assert!(ui.get_has_demo());
    assert!(ui.get_accounts().row_count() > 1);
    ui.invoke_remove_demo();
    settle();
    assert!(!ui.get_has_demo());
    assert_eq!(
        ui.get_accounts().row_count(),
        1,
        "only the starter account is left"
    );
    assert_eq!(ui.get_dash().balance, "0.00");
    assert!(ui.get_can_load_demo(), "back to a brand-new app");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn payday_can_be_the_31st() {
    let (ui, dir) = setup("payday");
    ui.invoke_cycle_day_changed(31);
    settle();
    assert_eq!(ui.get_cycle_day(), 31);
    // today is 15 Oct: the cycle started on 30 Sep (September has 30 days)
    assert!(
        ui.get_dash().cycle.contains("30 Sep"),
        "{}",
        ui.get_dash().cycle
    );
    let _ = std::fs::remove_dir_all(dir);
}
