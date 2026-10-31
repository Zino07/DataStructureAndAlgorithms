//! Three-decimal currencies through the screens. Own test binary: the number
//! of decimals is process-wide, so it must not share a process with tests
//! that use two.
#![cfg(not(target_os = "android"))]

use std::time::Duration;

use i_slint_backend_testing as testing;
use income_radar::AppWindow;
use slint::{ComponentHandle, Model};

fn setup(name: &str) -> (AppWindow, std::path::PathBuf) {
    std::env::set_var("INCOME_RADAR_NOW", "2026-10-15 10:00");
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

fn account_ids(ui: &AppWindow) -> Vec<i32> {
    ui.get_accounts().iter().map(|a| a.id).collect()
}

#[test]
fn currency_with_three_decimals_works_through_the_screens() {
    let (ui, dir) = setup("kwd");
    let acc = account_ids(&ui)[0];
    let food = ui.get_expense_categories().iter().next().unwrap().id;
    ui.invoke_save_entry(-1, 0, "12.5".into(), food, acc, -1, 0, 600, "".into(), 0);
    ui.invoke_currency_changed("KWD".into());
    settle();
    assert_eq!(ui.get_dash().balance, "-12.500");
    assert_eq!(
        ui.invoke_save_entry(-1, 0, "0.005".into(), food, acc, -1, 0, 600, "".into(), 0),
        ""
    );
    settle();
    assert_eq!(ui.get_dash().balance, "-12.505");
    // a fourth decimal is refused
    assert_eq!(
        ui.invoke_save_entry(-1, 0, "0.0051".into(), food, acc, -1, 0, 600, "".into(), 0),
        "Amount has too many decimals"
    );
    // the keypad stops at three
    assert_eq!(ui.invoke_edit_amount("1.234".into(), "5".into()), "1.234");
    ui.invoke_currency_changed("SAR".into());
    settle();
    assert_eq!(ui.get_dash().balance, "-12.51");
    let _ = std::fs::remove_dir_all(dir);
}
