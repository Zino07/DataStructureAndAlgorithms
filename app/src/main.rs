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
        }
    }

    const SHOTS: &[Shot] = &[
        shot("home-empty-lime-dark", Screen::Home, 0, true, 0.0),
        shot("home-lime-dark", Screen::Home, 0, true, 0.0),
        shot("home-lime-dark-2", Screen::Home, 0, true, 760.0),
        shot("home-lime-light", Screen::Home, 0, false, 0.0),
        shot("home-lilac-dark", Screen::Home, 3, true, 0.0),
        shot("home-ember-light", Screen::Home, 2, false, 0.0),
        shot("home-solar-dark", Screen::Home, 1, true, 0.0),
        shot("stats-lime-dark", Screen::Stats, 0, true, 0.0),
        shot("stats-lime-dark-2", Screen::Stats, 0, true, 700.0),
        shot("stats-lime-dark-3", Screen::Stats, 0, true, 1400.0),
        shot("stats-lime-dark-4", Screen::Stats, 0, true, 2100.0),
        shot("stats-lime-light", Screen::Stats, 0, false, 0.0),
        shot("activity-lime-dark", Screen::Activity, 0, true, 0.0),
        Shot {
            name: "add-lime-dark",
            screen: Screen::Home,
            adding: true,
            palette: 0,
            dark: true,
            scroll: 0.0,
        },
        Shot {
            name: "add-lime-light",
            screen: Screen::Home,
            adding: true,
            palette: 0,
            dark: false,
            scroll: 0.0,
        },
        shot("accounts-lime-dark", Screen::Accounts, 0, true, 0.0),
        shot("budgets-lime-dark", Screen::Budgets, 0, true, 0.0),
        shot("settings-lime-dark", Screen::Settings, 0, true, 0.0),
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

    fn apply(ui: &AppWindow, shot: &Shot) {
        if !shot.name.contains("empty") && ui.get_can_load_demo() {
            ui.invoke_load_demo();
        }
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
