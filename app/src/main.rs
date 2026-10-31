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

    const SHOTS: &[Shot] = &[
        Shot {
            name: "home-lime-light",
            screen: Screen::Home,
            adding: false,
            palette: 0,
            dark: false,
            scroll: 0.0,
        },
        Shot {
            name: "home-lime-light-2",
            screen: Screen::Home,
            adding: false,
            palette: 0,
            dark: false,
            scroll: 700.0,
        },
        Shot {
            name: "home-lime-dark",
            screen: Screen::Home,
            adding: false,
            palette: 0,
            dark: true,
            scroll: 0.0,
        },
        Shot {
            name: "home-ember-dark",
            screen: Screen::Home,
            adding: false,
            palette: 2,
            dark: true,
            scroll: 0.0,
        },
        Shot {
            name: "home-solar-light",
            screen: Screen::Home,
            adding: false,
            palette: 1,
            dark: false,
            scroll: 0.0,
        },
        Shot {
            name: "home-lilac-light",
            screen: Screen::Home,
            adding: false,
            palette: 3,
            dark: false,
            scroll: 0.0,
        },
        Shot {
            name: "activity-lime-light",
            screen: Screen::Activity,
            adding: false,
            palette: 0,
            dark: false,
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
        Shot {
            name: "add-lime-dark",
            screen: Screen::Home,
            adding: true,
            palette: 0,
            dark: true,
            scroll: 0.0,
        },
        Shot {
            name: "accounts-lime-light",
            screen: Screen::Accounts,
            adding: false,
            palette: 0,
            dark: false,
            scroll: 0.0,
        },
        Shot {
            name: "settings-lime-light",
            screen: Screen::Settings,
            adding: false,
            palette: 0,
            dark: false,
            scroll: 0.0,
        },
    ];

    pub fn run(out: &Path) -> Result<(), Box<dyn Error>> {
        std::fs::create_dir_all(out)?;
        let data = std::env::temp_dir().join(format!("income-radar-shots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let ui = income_radar::build(&data)?;
        ui.invoke_load_demo();

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
        ui.invoke_palette_changed(shot.palette);
        ui.invoke_dark_changed(shot.dark);
        ui.set_screen(shot.screen);
        if shot.adding {
            ui.invoke_open_add();
        } else {
            ui.set_adding(false);
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
