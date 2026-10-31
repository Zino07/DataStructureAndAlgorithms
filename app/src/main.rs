//! Desktop entry point: runs the same UI in a 390 × 844 window.
//!
//! `income-radar --screenshots <dir>` renders every screen in a few themes
//! into PNG files (uses a temporary database filled with sample data).

#[cfg(not(target_os = "android"))]
use slint::ComponentHandle;

#[cfg(not(target_os = "android"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--screenshots") {
        let out = args.get(i + 1).map(String::as_str).unwrap_or("screenshots");
        return screenshots::run(std::path::Path::new(out));
    }
    if let Some(i) = args.iter().position(|a| a == "--morph-frames") {
        let out = args.get(i + 1).map(String::as_str).unwrap_or("frames");
        let style = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(1);
        return screenshots::morph_frames(std::path::Path::new(out), style);
    }
    let data_dir = std::env::var_os("INCOME_RADAR_DATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").unwrap_or_else(|| ".".into());
            std::path::PathBuf::from(home).join(".local/share/income-radar")
        });
    let ui = income_radar::build(&data_dir)?;
    ui.run()?;
    Ok(())
}

#[cfg(target_os = "android")]
fn main() {}

#[cfg(not(target_os = "android"))]
mod screenshots {
    use std::cell::RefCell;
    use std::error::Error;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;
    use std::time::Duration;

    use income_radar::{AppWindow, Screen};
    use slint::ComponentHandle;

    struct Shot {
        name: &'static str,
        screen: Screen,
        adding: bool,
        palette: i32,
        dark: bool,
        scroll: f32,
        style: i32,
    }

    const fn shot(
        name: &'static str,
        screen: Screen,
        palette: i32,
        dark: bool,
        scroll: f32,
    ) -> Shot {
        Shot {
            name,
            screen,
            adding: false,
            palette,
            dark,
            scroll,
            style: 0,
        }
    }

    /// Aurora Glass shot.
    const fn glass(
        name: &'static str,
        screen: Screen,
        adding: bool,
        palette: i32,
        scroll: f32,
    ) -> Shot {
        Shot {
            name,
            screen,
            adding,
            palette,
            dark: true,
            scroll,
            style: 1,
        }
    }

    const fn add(name: &'static str, palette: i32, dark: bool) -> Shot {
        Shot {
            name,
            screen: Screen::Home,
            adding: true,
            palette,
            dark,
            scroll: 0.0,
            style: 0,
        }
    }

    const SHOTS: &[Shot] = &[
        shot("home-empty-lime-dark", Screen::Home, 0, true, 0.0),
        shot("home-lime-dark", Screen::Home, 0, true, 0.0),
        shot("home-lime-dark-2", Screen::Home, 0, true, 760.0),
        shot("home-lime-light", Screen::Home, 0, false, 0.0),
        shot("home-lime-light-2", Screen::Home, 0, false, 760.0),
        shot("home-lilac-dark", Screen::Home, 3, true, 0.0),
        shot("home-ember-light", Screen::Home, 2, false, 0.0),
        shot("home-solar-dark", Screen::Home, 1, true, 0.0),
        shot("stats-lime-dark", Screen::Stats, 0, true, 0.0),
        shot("stats-lime-dark-2", Screen::Stats, 0, true, 700.0),
        shot("stats-lime-dark-3", Screen::Stats, 0, true, 1400.0),
        shot("stats-lime-dark-4", Screen::Stats, 0, true, 2100.0),
        shot("stats-lime-light", Screen::Stats, 0, false, 0.0),
        shot("stats-lime-light-2", Screen::Stats, 0, false, 700.0),
        shot("activity-lime-dark", Screen::Activity, 0, true, 0.0),
        shot("activity-lime-light", Screen::Activity, 0, false, 0.0),
        add("add-lime-dark", 0, true),
        add("add-lime-light", 0, false),
        shot("accounts-lime-dark", Screen::Accounts, 0, true, 0.0),
        shot("accounts-lime-light", Screen::Accounts, 0, false, 0.0),
        shot("budgets-lime-dark", Screen::Budgets, 0, true, 0.0),
        shot("budgets-lime-light", Screen::Budgets, 0, false, 0.0),
        shot("settings-lime-dark", Screen::Settings, 0, true, 0.0),
        shot("settings-lime-light", Screen::Settings, 0, false, 0.0),
        glass("glass-home-lime", Screen::Home, false, 0, 0.0),
        glass("glass-home-lime-2", Screen::Home, false, 0, 760.0),
        glass("glass-home-lilac", Screen::Home, false, 3, 0.0),
        glass("glass-home-ember", Screen::Home, false, 2, 0.0),
        glass("glass-home-solar", Screen::Home, false, 1, 0.0),
        glass("glass-stats-lime", Screen::Stats, false, 0, 0.0),
        glass("glass-stats-lime-2", Screen::Stats, false, 0, 700.0),
        glass("glass-stats-lilac-3", Screen::Stats, false, 3, 1400.0),
        glass("glass-add-lime", Screen::Home, true, 0, 0.0),
        glass("glass-add-lilac", Screen::Home, true, 3, 0.0),
        glass("glass-activity-lime", Screen::Activity, false, 0, 0.0),
        glass("glass-accounts-lime", Screen::Accounts, false, 0, 0.0),
        glass("glass-budgets-ember", Screen::Budgets, false, 2, 0.0),
        glass("glass-settings-lime", Screen::Settings, false, 0, 0.0),
    ];

    pub fn run(out: &Path) -> Result<(), Box<dyn Error>> {
        std::fs::create_dir_all(out)?;
        let data = std::env::temp_dir().join(format!("income-radar-shots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let ui = income_radar::build(&data)?;

        let step = Rc::new(RefCell::new(0usize));
        let out: PathBuf = out.to_path_buf();
        let timer = Rc::new(slint::Timer::default());
        let weak = ui.as_weak();
        let t = timer.clone();
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(600),
            move || {
                let Some(ui) = weak.upgrade() else { return };
                let mut i = step.borrow_mut();
                // odd steps: capture the state set on the previous even step
                let shot_index = *i / 2;
                if shot_index >= SHOTS.len() {
                    t.stop();
                    let _ = slint::quit_event_loop();
                    return;
                }
                let shot = &SHOTS[shot_index];
                if i.is_multiple_of(2) {
                    apply(&ui, shot);
                } else if let Err(e) = capture(&ui, &out.join(format!("{}.png", shot.name))) {
                    eprintln!("{}: {e}", shot.name);
                }
                *i += 1;
            },
        );
        ui.run()?;
        let _ = std::fs::remove_dir_all(&data);
        Ok(())
    }

    /// Captures the "+" → New entry morph and its reverse at fixed times
    /// (real time, so each file name carries the measured milliseconds).
    pub fn morph_frames(out: &Path, style: i32) -> Result<(), Box<dyn Error>> {
        use income_radar::Morph;
        use std::time::Instant;
        std::fs::create_dir_all(out)?;
        let data = std::env::temp_dir().join(format!("income-radar-frames-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let ui = income_radar::build(&data)?;
        ui.invoke_load_demo();
        ui.invoke_style_changed(style);
        ui.invoke_palette_changed(0);
        ui.invoke_dark_changed(true);

        // (phase, target ms after the phase starts)
        let marks: Vec<(u8, u64)> = [0u64, 60, 120, 180, 240, 320, 420, 600]
            .iter()
            .map(|m| (0u8, *m))
            .chain([0u64, 80, 160, 240, 360, 600].iter().map(|m| (1u8, *m)))
            .collect();
        let state = Rc::new(RefCell::new((None::<Instant>, 0usize, 0u8)));
        let out: PathBuf = out.to_path_buf();
        let timer = Rc::new(slint::Timer::default());
        let weak = ui.as_weak();
        let t = timer.clone();
        let start = Instant::now();
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(4),
            move || {
                let Some(ui) = weak.upgrade() else { return };
                let mut st = state.borrow_mut();
                // settle the home screen first
                if st.0.is_none() {
                    if start.elapsed() < Duration::from_millis(1200) {
                        return;
                    }
                    // the dock "+" (third item) is the source
                    let w = ui.window().size().to_logical(ui.window().scale_factor());
                    let dock_w = 8.0 * 2.0 + 48.0 * 5.0 + 6.0 * 4.0;
                    let x = (w.width - dock_w) / 2.0 + 8.0 + 2.0 * 54.0;
                    let y = w.height - 16.0 - 64.0 + 8.0;
                    ui.global::<Morph>().invoke_from(
                        x,
                        y,
                        48.0,
                        48.0,
                        24.0,
                        slint::Color::from_rgb_u8(0xd4, 0xff, 0x3a),
                    );
                    ui.invoke_open_add();
                    st.0 = Some(Instant::now());
                    st.2 = 0;
                }
                let i = st.1;
                if i >= marks.len() {
                    t.stop();
                    let _ = slint::quit_event_loop();
                    return;
                }
                let (phase, at) = marks[i];
                if phase != st.2 {
                    // start closing
                    ui.set_adding(false);
                    st.0 = Some(Instant::now());
                    st.2 = phase;
                }
                let elapsed = st.0.unwrap().elapsed().as_millis() as u64;
                if elapsed >= at {
                    let name = format!(
                        "{}-{:03}ms.png",
                        if phase == 0 { "open" } else { "close" },
                        elapsed
                    );
                    if let Err(e) = capture(&ui, &out.join(name)) {
                        eprintln!("{e}");
                    }
                    st.1 += 1;
                }
            },
        );
        ui.run()?;
        let _ = std::fs::remove_dir_all(&data);
        Ok(())
    }

    fn apply(ui: &AppWindow, shot: &Shot) {
        if !shot.name.contains("empty") && ui.get_can_load_demo() {
            ui.invoke_load_demo();
        }
        ui.invoke_style_changed(shot.style);
        ui.invoke_palette_changed(shot.palette);
        ui.invoke_dark_changed(shot.dark);
        ui.invoke_go(shot.screen);
        if shot.adding {
            ui.invoke_open_add();
        }
        ui.invoke_scroll_to(shot.scroll);
    }

    fn capture(ui: &AppWindow, path: &Path) -> Result<(), Box<dyn Error>> {
        let shot = ui.window().take_snapshot()?;
        let file = std::fs::File::create(path)?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), shot.width(), shot.height());
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()?.write_image_data(shot.as_bytes())?;
        println!("wrote {}", path.display());
        Ok(())
    }
}
