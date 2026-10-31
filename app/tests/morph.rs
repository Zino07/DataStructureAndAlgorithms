//! The morph ("magic move") is one spring. These checks drive it with mock
//! time: it moves smoothly, a reversal mid-way never jumps, and it settles.
#![cfg(not(target_os = "android"))]

use std::time::Duration;

use i_slint_backend_testing as testing;
use income_radar::Motion;
use slint::ComponentHandle;

fn step(ms: u64) {
    testing::mock_elapsed_time(Duration::from_millis(ms));
}

#[test]
fn morph_moves_smoothly_reverses_without_a_jump_and_settles() {
    testing::init_no_event_loop();
    let dir = std::env::temp_dir().join(format!("income-radar-morph-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ui = income_radar::build(&dir).unwrap();

    // nothing has been drawn in the test backend: read the value once so the
    // next change animates from it (a first read never animates)
    assert_eq!(ui.get_morph_progress(), 0.0);

    // opening: starts at the source, rises with no frame jumping far
    ui.invoke_open_add();
    let mut last = ui.get_morph_progress();
    assert!(last < 0.05, "starts at the source, got {last}");
    for _ in 0..8 {
        step(16);
        let p = ui.get_morph_progress();
        assert!(p >= last - 1e-4, "went backwards: {last} -> {p}");
        assert!(p - last < 0.2, "frame jump {last} -> {p}");
        last = p;
    }
    assert!(
        last > 0.2 && last < 0.95,
        "should be mid-way after 128 ms, got {last}"
    );

    // close while it is still opening: no jump, the motion turns around
    ui.set_adding(false);
    let mut prev = last;
    let mut lowest = last;
    let mut t = 0;
    while ui.get_morph_progress() > 0.005 && t < 3000 {
        step(16);
        t += 16;
        let p = ui.get_morph_progress();
        assert!((p - prev).abs() < 0.2, "frame jump {prev} -> {p} at {t} ms");
        lowest = lowest.min(p);
        prev = p;
    }
    assert!(t <= 1200, "closing took {t} ms");
    assert!(lowest > -0.1, "overshot below the source: {lowest}");
    assert!(ui.get_morph_settled());

    // reduce motion: the page is there at once
    ui.global::<Motion>().set_reduce(true);
    step(1);
    ui.invoke_open_add();
    step(1);
    assert!(
        ui.get_morph_progress() > 0.99,
        "reduce motion should not animate"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
