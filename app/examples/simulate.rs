//! Simulates 100 people using Income Radar 5 times each by tapping the real UI
//! (Slint testing backend: hit-tested pointer clicks on the actual widgets).
//!
//! Every person has a persona with habits, a real life that produces income and
//! purchases, and a memory that sometimes forgets. In each session they log what
//! happened since last time, make typos, edit, delete, undo, search, move money,
//! set budgets and look at statistics. After every session the UI and the
//! database are checked against a shadow ledger kept by the simulator.
//!
//! Run: `cargo run -p income-radar --example simulate --release -- [report.md]`

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use i_slint_backend_testing::{self as testing, AccessibleRole, ElementHandle};
use income_radar::demo::Lcg;
use income_radar::{AppWindow, Screen};
use income_radar_core::*;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::LogicalPosition;
use slint::{ComponentHandle, Model};

const USERS: usize = 100;
const SESSIONS: usize = 5;
const W: f32 = 390.0;
const H: f32 = 844.0;
/// Content below this line sits behind the floating dock.
const DOCK_TOP: f32 = H - 16.0 - 64.0;

// ---------------------------------------------------------------- personas

#[derive(Clone, Copy)]
struct Habit {
    category: &'static str,
    note: &'static str,
    min: i64,
    max: i64,
    from_h: u16,
    to_h: u16,
    per_week: f32,
    account: usize,
}

#[allow(clippy::too_many_arguments)]
const fn h(
    category: &'static str,
    note: &'static str,
    min: i64,
    max: i64,
    from_h: u16,
    to_h: u16,
    per_week: f32,
    account: usize,
) -> Habit {
    Habit {
        category,
        note,
        min,
        max,
        from_h,
        to_h,
        per_week,
        account,
    }
}

struct Persona {
    name: &'static str,
    currency: &'static str,
    payday: u32,
    weekend_sat_sun: bool,
    accounts: &'static [(&'static str, usize, &'static str)], // name, chip index, opening
    income: (&'static str, &'static str, i64, i64),           // category, note, min, max (minor)
    irregular_income: bool,
    habits: &'static [Habit],
    typo: f64,
    forget: f64,
    adjusts_time: f64,
    session_hours: (u16, u16),
    // probability per session of each extra action
    p_stats: f64,
    p_search: f64,
    p_edit: f64,
    p_delete: f64,
    p_transfer: f64,
    p_budget: f64,
    p_settings: f64,
    p_account: f64,
}

const PERSONAS: &[Persona] = &[
    Persona {
        name: "Salaried commuter",
        currency: "SAR",
        payday: 27,
        weekend_sat_sun: false,
        accounts: &[("Main bank", 1, "8200")],
        income: ("Salary", "Salary", 1_200_000, 1_200_000),
        irregular_income: false,
        habits: &[
            h("Food", "Morning coffee", 1_400, 2_200, 7, 9, 5.0, 0),
            h("Food", "Lunch", 2_500, 6_000, 12, 14, 4.0, 1),
            h("Transport", "Fuel", 6_000, 12_000, 7, 21, 1.5, 1),
            h("Groceries", "Supermarket", 12_000, 45_000, 18, 22, 1.2, 1),
            h("Fun", "Cinema", 5_000, 15_000, 20, 23, 0.4, 0),
        ],
        typo: 0.06,
        forget: 0.12,
        adjusts_time: 0.15,
        session_hours: (20, 23),
        p_stats: 0.5,
        p_search: 0.15,
        p_edit: 0.15,
        p_delete: 0.08,
        p_transfer: 0.2,
        p_budget: 0.1,
        p_settings: 0.1,
        p_account: 0.05,
    },
    Persona {
        name: "Freelancer",
        currency: "AED",
        payday: 1,
        weekend_sat_sun: true,
        accounts: &[("Business", 1, "15000"), ("Card", 2, "")],
        income: ("Freelance", "Client payment", 150_000, 900_000),
        irregular_income: true,
        habits: &[
            h("Food", "Coffee", 1_800, 3_000, 9, 12, 6.0, 2),
            h("Food", "Delivery", 4_000, 9_000, 21, 24, 3.0, 2),
            h("Bills", "Software", 4_000, 30_000, 10, 18, 0.6, 1),
            h("Transport", "Taxi", 2_500, 7_000, 8, 23, 2.0, 2),
            h("Shopping", "Gear", 10_000, 120_000, 14, 22, 0.3, 2),
        ],
        typo: 0.05,
        forget: 0.2,
        adjusts_time: 0.25,
        session_hours: (10, 24),
        p_stats: 0.7,
        p_search: 0.25,
        p_edit: 0.1,
        p_delete: 0.05,
        p_transfer: 0.3,
        p_budget: 0.15,
        p_settings: 0.15,
        p_account: 0.1,
    },
    Persona {
        name: "Student",
        currency: "EGP",
        payday: 1,
        weekend_sat_sun: false,
        accounts: &[],
        income: ("Gifts", "Pocket money", 150_000, 300_000),
        irregular_income: false,
        habits: &[
            h("Food", "Snacks", 1_000, 4_000, 10, 22, 8.0, 0),
            h("Transport", "Bus", 500, 1_500, 7, 19, 8.0, 0),
            h("Education", "Books", 5_000, 25_000, 12, 18, 0.5, 0),
            h("Fun", "Games", 3_000, 10_000, 18, 24, 1.0, 0),
        ],
        typo: 0.12,
        forget: 0.3,
        adjusts_time: 0.05,
        session_hours: (12, 24),
        p_stats: 0.3,
        p_search: 0.05,
        p_edit: 0.2,
        p_delete: 0.15,
        p_transfer: 0.0,
        p_budget: 0.05,
        p_settings: 0.25,
        p_account: 0.15,
    },
    Persona {
        name: "Family manager",
        currency: "KWD",
        payday: 25,
        weekend_sat_sun: false,
        accounts: &[("Joint account", 1, "2400"), ("Savings", 3, "9000")],
        income: ("Salary", "Salary", 120_000, 120_000),
        irregular_income: false,
        habits: &[
            h("Groceries", "Supermarket", 1_500, 9_000, 17, 22, 3.0, 1),
            h("Family", "School", 2_000, 15_000, 7, 9, 0.6, 1),
            h("Bills", "Electricity", 2_000, 6_000, 10, 20, 0.25, 1),
            h("Health", "Pharmacy", 500, 3_000, 9, 22, 0.8, 0),
            h("Food", "Bakery", 200, 900, 6, 9, 4.0, 0),
        ],
        typo: 0.04,
        forget: 0.08,
        adjusts_time: 0.3,
        session_hours: (21, 24),
        p_stats: 0.4,
        p_search: 0.2,
        p_edit: 0.1,
        p_delete: 0.05,
        p_transfer: 0.35,
        p_budget: 0.4,
        p_settings: 0.1,
        p_account: 0.05,
    },
    Persona {
        name: "Power user",
        currency: "SAR",
        payday: 1,
        weekend_sat_sun: false,
        accounts: &[
            ("Bank", 1, "30000"),
            ("Credit card", 2, ""),
            ("Savings", 3, "50000"),
        ],
        income: ("Salary", "Salary", 2_000_000, 2_000_000),
        irregular_income: false,
        habits: &[
            h("Food", "Coffee", 1_600, 2_600, 7, 10, 7.0, 1),
            h("Food", "Lunch", 3_000, 9_000, 12, 15, 5.0, 1),
            h("Shopping", "Online order", 5_000, 80_000, 21, 24, 1.5, 1),
            h("Transport", "Fuel", 8_000, 15_000, 6, 22, 1.5, 0),
            h("Health", "Gym", 30_000, 30_000, 18, 21, 0.25, 0),
            h("Fun", "Restaurant", 15_000, 60_000, 20, 24, 1.0, 1),
        ],
        typo: 0.03,
        forget: 0.03,
        adjusts_time: 0.6,
        session_hours: (7, 24),
        p_stats: 0.9,
        p_search: 0.5,
        p_edit: 0.3,
        p_delete: 0.15,
        p_transfer: 0.4,
        p_budget: 0.5,
        p_settings: 0.3,
        p_account: 0.1,
    },
];

// ---------------------------------------------------------------- results

#[derive(Default)]
struct Report {
    sessions: u32,
    taps: u32,
    tap_times_us: Vec<u64>,
    open_times_us: Vec<u64>,
    entries_logged: u32,
    entry_taps: Vec<u32>,
    entries_via_suggestion: u32,
    suggestion_amount_retyped: u32,
    events_forgotten: u32,
    events_too_old: u32,
    backdated: u32,
    time_error_min: Vec<i64>,
    typos_caught: u32,
    typos_uncaught: u32,
    typos_fixed_later: u32,
    expected_errors_shown: BTreeMap<String, u32>,
    unexpected: Vec<String>,
    feature_use: BTreeMap<&'static str, u32>,
    hidden_by_dock: u32,
    needed_scroll: u32,
    needed_swipe: u32,
    rss_kb: Vec<u64>,
    db_bytes: Vec<u64>,
    stats_ms: Vec<f64>,
    invariant_checks: u32,
    per_persona: BTreeMap<&'static str, (u32, u32)>, // entries, sessions
    undo_used: u32,
    suggestions_offered: u32,
}

impl Report {
    fn bug(&mut self, user: usize, msg: String) {
        if self.unexpected.len() < 200 {
            self.unexpected.push(format!("user {user}: {msg}"));
        }
    }
    fn use_feature(&mut self, f: &'static str) {
        *self.feature_use.entry(f).or_default() += 1;
    }
}

// ---------------------------------------------------------------- driver

struct Driver<'a> {
    ui: AppWindow,
    rep: &'a mut Report,
    user: usize,
    taps_this_entry: u32,
}

fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse().ok()))
        })
        .unwrap_or(0)
}

fn clickable(e: &ElementHandle) -> bool {
    matches!(
        e.accessible_role(),
        Some(AccessibleRole::Button | AccessibleRole::Tab | AccessibleRole::TextInput)
    )
}

impl Driver<'_> {
    fn find(&self, label: &str, nth: usize) -> Option<ElementHandle> {
        ElementHandle::find_by_accessible_label(&self.ui, label)
            .filter(clickable)
            .nth(nth)
    }

    /// A page is laid out wider than the screen when its title is not at
    /// the 16 px margin (the whole page then shifts and gets clipped).
    fn check_page(&mut self, title: &str) {
        let e = ElementHandle::find_by_accessible_label(&self.ui, title)
            .find(|e| e.accessible_role() == Some(AccessibleRole::Text));
        if let Some(e) = e {
            let x = e.absolute_position().x;
            if (x - 16.0).abs() > 0.5 {
                let user = self.user;
                self.rep.bug(
                    user,
                    format!(
                        "page with title “{title}” is wider than the screen (title at x = {x})"
                    ),
                );
            }
        }
    }

    /// Taps a control the way a person would: scroll it into view (out from
    /// under the dock), swipe chip rows, then click its centre.
    /// Finds a control, scrolling the page like a person looking for it.
    fn seek(&mut self, label: &str, nth: usize) -> Option<ElementHandle> {
        if let Some(e) = self.find(label, nth) {
            return Some(e);
        }
        let start = self.ui.get_scroll_pos();
        for pass in 0..2 {
            let mut y = if pass == 0 { start } else { 0.0 };
            self.ui.invoke_scroll_to(y);
            for _ in 0..12 {
                if let Some(e) = self.find(label, nth) {
                    self.rep.needed_scroll += 1;
                    return Some(e);
                }
                let before = self.ui.get_scroll_pos();
                y += 500.0;
                self.ui.invoke_scroll_to(y);
                if (self.ui.get_scroll_pos() - before).abs() < 1.0 {
                    break;
                }
            }
        }
        self.ui.invoke_scroll_to(start);
        None
    }

    /// Chip rows scroll sideways: drag the row (starting in the gap right of
    /// its last visible chip) until the wanted chip comes into view.
    fn swipe_to(&mut self, label: &str, nth: usize) -> Option<ElementHandle> {
        let ui = &self.ui;
        let mut groups: Vec<Vec<String>> = vec![
            ui.get_expense_categories()
                .iter()
                .map(|c| c.name.to_string())
                .collect(),
            ui.get_income_categories()
                .iter()
                .map(|c| c.name.to_string())
                .collect(),
            ui.get_accounts()
                .iter()
                .map(|a| a.name.to_string())
                .collect(),
        ];
        groups.retain(|g| g.iter().any(|n| n == label));
        let group = groups.into_iter().next()?;
        let target = group.iter().position(|n| n == label)?;
        let mut last_x = f32::NAN;
        let mut flip = false;
        for _ in 0..10 {
            let collect = |this: &Self| -> Vec<(usize, ElementHandle)> {
                group
                    .iter()
                    .enumerate()
                    .filter_map(|(i, n)| {
                        let e = ElementHandle::find_by_accessible_label(&this.ui, n)
                            .filter(clickable)
                            .nth(if n == label { nth } else { 0 })?;
                        let x = e.absolute_position().x;
                        (x >= 0.0 && x + e.size().width < W - 8.0).then_some((i, e))
                    })
                    .collect()
            };
            let mut visible = collect(self);
            if visible.is_empty() {
                // the row itself is scrolled out of sight: bring it into view
                for n in &group {
                    if self.seek(n, 0).is_some() {
                        break;
                    }
                }
                visible = collect(self);
            }
            if std::env::var("SIM_DEBUG").is_ok() {
                eprintln!(
                    "swipe {label}: target={target} visible={:?} scroll={}",
                    visible
                        .iter()
                        .map(|(i, e)| (
                            *i,
                            e.absolute_position().x as i32,
                            e.absolute_position().y as i32
                        ))
                        .collect::<Vec<_>>(),
                    self.ui.get_scroll_pos()
                );
            }
            let first = visible.iter().map(|(i, _)| *i).min()?;
            let first_x = visible
                .iter()
                .find(|(i, _)| *i == first)
                .map(|(_, e)| e.absolute_position().x)
                .unwrap_or(0.0);
            // a drag that moved nothing: the row is stuck at its end, try the other way
            flip = if (first_x - last_x).abs() < 1.0 {
                !flip
            } else {
                false
            };
            last_x = first_x;
            let towards_start = (target < first) != flip;
            // start in the gap right of a visible chip, drag the row
            let anchor = if towards_start {
                &visible.iter().find(|(i, _)| *i == first)?.1
            } else {
                &visible.iter().max_by_key(|(i, _)| *i)?.1
            };
            let p = anchor.absolute_position();
            let x = p.x + anchor.size().width + 4.0;
            let y = p.y + anchor.size().height / 2.0;
            let dir = if towards_start { 25.0 } else { -25.0 };
            if std::env::var("SIM_DEBUG").is_ok() {
                eprintln!("  press at ({x}, {y}) anchor w={}", anchor.size().width);
            }
            let w = self.ui.window();
            w.dispatch_event(WindowEvent::PointerPressed {
                position: LogicalPosition::new(x, y),
                button: PointerEventButton::Left,
            });
            self.wait(150);
            for i in 1..=10 {
                self.wait(20);
                w.dispatch_event(WindowEvent::PointerMoved {
                    position: LogicalPosition::new(x + dir * i as f32, y),
                });
            }
            w.dispatch_event(WindowEvent::PointerReleased {
                position: LogicalPosition::new(x + dir * 10.0, y),
                button: PointerEventButton::Left,
            });
            self.wait(400);
            self.rep.needed_swipe += 1;
            if let Some(e) = self.find(label, nth) {
                return Some(e);
            }
        }
        None
    }

    fn tap_nth(&mut self, label: &str, nth: usize) -> bool {
        let found = match self.seek(label, nth) {
            Some(e) => Some(e),
            None => self.swipe_to(label, nth),
        };
        let Some(mut e) = found else {
            return false;
        };
        let pos = e.absolute_position();
        let size = e.size();
        let cy = pos.y + size.height / 2.0;
        let dock_shown = !self.ui.get_adding()
            && !matches!(self.ui.get_screen(), Screen::Settings | Screen::Budgets);
        let bottom = if dock_shown {
            DOCK_TOP
        } else if self.ui.get_adding() {
            H - 100.0 // the pinned Save bar covers the last 100 px
        } else {
            H - 8.0
        };
        if cy > bottom || cy < 60.0 {
            if cy > bottom && cy < H && dock_shown {
                self.rep.hidden_by_dock += 1;
            }
            self.rep.needed_scroll += 1;
            let target = self.ui.get_scroll_pos() + cy - H / 2.0;
            self.ui.invoke_scroll_to(target.max(0.0));
            // the scroll glides: a person waits for it to stop before tapping
            self.wait(700);
            match self.find(label, nth) {
                Some(again) => e = again,
                None => return false,
            }
        }
        let pos = e.absolute_position();
        let size = e.size();
        let cx = pos.x + size.width / 2.0;
        let cy = pos.y + size.height / 2.0;
        let start = Instant::now();
        let was_adding = self.ui.get_adding();
        // the part of the element inside the page margins (chip rows clip there)
        let vis_l = pos.x.max(16.0);
        let vis_r = (pos.x + size.width).min(W - 16.0);
        if !(0.0..=W).contains(&cx) || !(0.0..=H).contains(&cy) || vis_r - vis_l < 24.0 {
            // chip rows scroll sideways: a swipe, then the tap
            self.rep.needed_swipe += 1;
            e.invoke_accessible_default_action();
        } else if vis_l > pos.x + 1.0 || vis_r < pos.x + size.width - 1.0 {
            // partly scrolled out of view: a person taps the part they see
            self.click_at((vis_l + vis_r) / 2.0, cy);
        } else {
            e.mock_single_click(PointerEventButton::Left);
        }
        self.rep
            .tap_times_us
            .push(start.elapsed().as_micros() as u64);
        // the entry page morphs in and out on a spring: a person waits for
        // it to settle before the next tap, so does the simulation
        if self.ui.get_adding() != was_adding {
            self.wait(900);
        }
        self.rep.taps += 1;
        self.taps_this_entry += 1;
        true
    }

    fn click_at(&self, x: f32, y: f32) {
        let w = self.ui.window();
        let position = slint::LogicalPosition::new(x, y);
        w.dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        self.wait(30);
        w.dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
        self.wait(30);
    }

    fn tap(&mut self, label: &str) -> bool {
        self.tap_nth(label, 0)
    }

    fn type_text(&mut self, label: &str, text: &str) -> bool {
        match self.seek(label, 0) {
            Some(e) => {
                e.set_accessible_value(text);
                self.rep.taps += 1;
                self.taps_this_entry += 1;
                true
            }
            None => false,
        }
    }

    fn type_amount(&mut self, amount: &str) {
        for ch in amount.chars() {
            let label = if ch == '.' {
                "Decimal point".to_string()
            } else {
                ch.to_string()
            };
            self.tap(&label);
        }
    }

    fn clear_amount(&mut self, len: usize) {
        for _ in 0..len {
            self.tap("Delete digit");
        }
    }

    fn go_home(&mut self) {
        if self.ui.get_adding() {
            self.tap("Close");
        }
        if matches!(self.ui.get_screen(), Screen::Settings | Screen::Budgets) {
            self.tap("Back");
        }
        if self.ui.get_screen() != Screen::Home {
            self.tap("Home");
        }
    }

    fn wait(&self, ms: u64) {
        testing::mock_elapsed_time(Duration::from_millis(ms));
    }
}

fn set_clock(date: NaiveDate, minute: u16) {
    std::env::set_var(
        "INCOME_RADAR_NOW",
        format!(
            "{} {:02}:{:02}",
            date.format("%Y-%m-%d"),
            minute / 60,
            minute % 60
        ),
    );
}

fn debug_check(dir: &std::path::Path, shadow: &HashMap<String, Money>, what: &str) {
    if std::env::var("SIM_DEBUG").is_err() {
        return;
    }
    let store = Store::open(dir.join("income-radar.db")).unwrap();
    for a in store.accounts_with_balances().unwrap() {
        let want = shadow.get(&a.account.name).copied().unwrap_or_default();
        if a.balance != want {
            let last = store
                .search(None, "", 100_000)
                .unwrap()
                .into_iter()
                .max_by_key(|t| t.id);
            eprintln!(
                "DIVERGE after {what}: {} db={} shadow={} last={:?}",
                a.account.name,
                a.balance.format(),
                want.format(),
                last.map(|t| (t.amount.format(), t.account_name, t.category_name, t.note))
            );
        }
    }
}

/// How many rows above this one share its title (the n-th element to tap).
fn same_title_before(rows: &slint::ModelRc<income_radar::TxRow>, idx: usize) -> usize {
    let title = rows
        .row_data(idx)
        .map(|r| r.title.to_string())
        .unwrap_or_default();
    (0..idx)
        .filter(|&i| rows.row_data(i).is_some_and(|r| r.title.as_str() == title))
        .count()
}

fn money_text(m: Money) -> String {
    // keypad text
    if m.0 % 100 == 0 {
        (m.0 / 100).to_string()
    } else {
        format!("{}.{:02}", m.0 / 100, m.0 % 100)
    }
}

fn parse_shown(s: &str) -> Option<Money> {
    Money::parse(&s.replace('−', "-").replace('+', "")).ok()
}

struct Event {
    date: NaiveDate,
    minute: u16,
    kind: Kind,
    category: &'static str,
    note: &'static str,
    amount: Money,
    account: String,
}

fn main() {
    testing::init_no_event_loop();
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/SIMULATION_RESULTS.md".into());
    let root = std::env::temp_dir().join(format!("income-radar-sim-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut rep = Report::default();
    let started = Instant::now();
    let start_date = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();

    let users = std::env::var("SIM_USERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(USERS);
    for user in 0..users {
        let persona = &PERSONAS[user % PERSONAS.len()];
        let mut rng = Lcg(0x9e3779b97f4a7c15 ^ (user as u64 * 7919));
        let dir: PathBuf = root.join(format!("user-{user:03}"));
        simulate_user(user, persona, &mut rng, &dir, start_date, &mut rep);
        rep.rss_kb.push(rss_kb());
        if let Ok(meta) = std::fs::metadata(dir.join("income-radar.db")) {
            let wal = std::fs::metadata(dir.join("income-radar.db-wal"))
                .map(|m| m.len())
                .unwrap_or(0);
            rep.db_bytes.push(meta.len() + wal);
        }
        if user % 10 == 9 {
            eprintln!(
                "{} users done, {} bugs so far",
                user + 1,
                rep.unexpected.len()
            );
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    let text = render(&rep, started.elapsed());
    std::fs::write(&out, &text).expect("write report");
    println!("{text}");
}

fn simulate_user(
    user: usize,
    p: &Persona,
    rng: &mut Lcg,
    dir: &std::path::Path,
    start: NaiveDate,
    rep: &mut Report,
) {
    let roll = |rng: &mut Lcg, prob: f64| (rng.next(10_000) as f64) < prob * 10_000.0;

    // real life: 30 days of income and purchases
    let day0 = start + chrono::Days::new(rng.next(10));
    let mut accounts: Vec<String> = vec!["Cash".into()];
    accounts.extend(p.accounts.iter().map(|a| a.0.to_string()));
    let mut events: Vec<Event> = Vec::new();
    for d in 0..30u64 {
        let date = day0 + chrono::Days::new(d);
        let payday = chrono::Datelike::day(&date) == p.payday;
        let irregular = p.irregular_income && rng.next(100) < 8;
        if payday || irregular {
            let amount = p.income.2 + rng.next((p.income.3 - p.income.2 + 1) as u64) as i64;
            events.push(Event {
                date,
                minute: 9 * 60 + rng.next(120) as u16,
                kind: Kind::Income,
                category: p.income.0,
                note: p.income.1,
                amount: Money(amount),
                account: accounts[accounts.len().min(2) - 1].clone(),
            });
        }
        for habit in p.habits {
            let chance = (habit.per_week / 7.0 * 1000.0) as u64;
            let mut times = 0;
            while rng.next(1000) < chance && times < 2 {
                times += 1;
                let amount = habit.min + rng.next((habit.max - habit.min + 1) as u64) as i64;
                // round to whole units most of the time, like real prices
                let amount = if rng.next(3) > 0 {
                    amount / 100 * 100
                } else {
                    amount
                };
                events.push(Event {
                    date,
                    minute: (habit.from_h * 60
                        + rng.next(((habit.to_h - habit.from_h) * 60) as u64) as u16)
                        .min(1439),
                    kind: Kind::Expense,
                    category: habit.category,
                    note: habit.note,
                    amount: Money(amount.max(100)),
                    account: accounts[habit.account.min(accounts.len() - 1)].clone(),
                });
            }
        }
    }
    events.sort_by_key(|e| stamp(e.date, e.minute));

    // five sessions, a few days apart
    let mut session_days = vec![rng.next(2)];
    for _ in 1..SESSIONS {
        let last = *session_days.last().unwrap();
        session_days.push(last + 1 + rng.next(6));
    }

    let mut shadow: HashMap<String, Money> = HashMap::new();
    shadow.insert("Cash".into(), Money::ZERO);
    let mut logged = 0usize;
    let mut next_event = 0usize;
    let mut persona_entries = 0u32;

    for (s, &day) in session_days.iter().enumerate() {
        let date = day0 + chrono::Days::new(day);
        let mut minute = p.session_hours.0 * 60
            + rng.next(((p.session_hours.1 - p.session_hours.0) * 60) as u64) as u16;
        minute = minute.min(1430);
        set_clock(date, minute);

        let t0 = Instant::now();
        let ui = match income_radar::build(dir) {
            Ok(ui) => ui,
            Err(e) => {
                rep.bug(user, format!("app failed to open: {e}"));
                return;
            }
        };
        ui.window().set_size(slint::LogicalSize::new(W, H));
        ui.show().ok();
        rep.open_times_us.push(t0.elapsed().as_micros() as u64);
        rep.sessions += 1;
        let mut d = Driver {
            ui,
            rep: &mut *rep,
            user,
            taps_this_entry: 0,
        };

        // first launch: settings and accounts
        if s == 0 {
            d.tap("Settings");
            let cur = d.ui.get_dash().currency.to_string();
            if cur != p.currency {
                d.tap(p.currency);
            }
            for _ in 1..p.payday {
                d.tap("Later payday");
            }
            if p.weekend_sat_sun {
                d.tap("Sat – Sun");
            }
            if d.ui.get_cycle_day() != p.payday as i32 {
                d.rep.bug(
                    user,
                    format!(
                        "payday is {} after setting {}",
                        d.ui.get_cycle_day(),
                        p.payday
                    ),
                );
            }
            d.tap("Back");
            for (name, chip, opening) in p.accounts {
                d.tap("Accounts");
                // the form opens on request
                d.tap("New account");
                d.type_text("Name", name);
                d.tap(["Cash", "Bank", "Card", "Savings"][*chip]);
                if !opening.is_empty() {
                    d.type_text(&format!("Opening balance ({})", p.currency), opening);
                }
                d.tap("Add account");
                let opening_m = if opening.is_empty() {
                    Money::ZERO
                } else {
                    Money::parse(opening).unwrap()
                };
                shadow.insert(name.to_string(), opening_m);
                d.rep.use_feature("add account");
            }
            d.go_home();
        }

        // log what happened since last time
        let session_stamp = stamp(date, minute);
        while next_event < events.len()
            && stamp(events[next_event].date, events[next_event].minute) <= session_stamp
        {
            let e = &events[next_event];
            next_event += 1;
            let days_ago = (date - e.date).num_days();
            if days_ago > 6 {
                d.rep.events_too_old += 1;
                continue;
            }
            if roll(rng, p.forget) && e.kind == Kind::Expense {
                d.rep.events_forgotten += 1;
                continue;
            }
            d.taps_this_entry = 0;
            d.go_home();
            if !d.tap("Add entry") {
                d.rep.bug(user, "no Add entry button on home".into());
                continue;
            }
            if e.kind == Kind::Income {
                d.tap("Income");
            }
            let mut typed = Money::ZERO;
            let suggestion_prefix = format!("{} · ", e.note);
            let chip =
                d.ui.get_expense_suggestions()
                    .iter()
                    .chain(d.ui.get_income_suggestions().iter())
                    .find(|sg| sg.title.as_str() == e.note);
            if chip.is_some() {
                d.rep.suggestions_offered += 1;
            }
            let mut used_suggestion = false;
            if let Some(sg) = chip {
                let label = format!("{}{}", suggestion_prefix, sg.amount_label);
                if roll(rng, 0.75) && d.tap(&label) {
                    if std::env::var("SIM_DEBUG").is_ok() && d.ui.get_entry_category() < 0 {
                        let hits: Vec<String> =
                            ElementHandle::find_by_accessible_label(&d.ui, &label)
                                .map(|h| {
                                    format!(
                                        "{:?}@{:?} {:?}",
                                        h.accessible_role(),
                                        h.absolute_position(),
                                        h.size()
                                    )
                                })
                                .collect();
                        eprintln!(
                            "suggestion tap did not apply {label}: settled={} hits={hits:?}",
                            d.ui.get_morph_settled()
                        );
                    }
                    used_suggestion = true;
                    d.rep.entries_via_suggestion += 1;
                    typed = Money::parse(&sg.amount).unwrap_or_default();
                    if typed != e.amount {
                        d.rep.suggestion_amount_retyped += 1;
                        d.clear_amount(sg.amount.len());
                        d.type_amount(&money_text(e.amount));
                        typed = e.amount;
                    }
                    if sg.account != account_id(&d.ui, &e.account) {
                        d.tap(&e.account);
                    }
                }
            }
            let mut skipped_category = false;
            if !used_suggestion {
                // typo: an extra digit
                let mut text = money_text(e.amount);
                let typo = roll(rng, p.typo);
                if typo {
                    text.insert(0, char::from(b'1' + rng.next(9) as u8));
                }
                d.type_amount(&text);
                typed = Money::parse(&text).unwrap();
                if typo {
                    if roll(rng, 0.5) {
                        // noticed before saving
                        d.clear_amount(text.len());
                        d.type_amount(&money_text(e.amount));
                        typed = e.amount;
                        d.rep.typos_caught += 1;
                    } else {
                        d.rep.typos_uncaught += 1;
                    }
                }
                if roll(rng, p.typo) {
                    skipped_category = true;
                } else if d.tap(e.category) {
                    if std::env::var("SIM_DEBUG").is_ok() && d.ui.get_entry_category() < 0 {
                        let hits: Vec<String> =
                            ElementHandle::find_by_accessible_label(&d.ui, e.category)
                                .map(|h| {
                                    format!(
                                        "{:?}@{:?} {:?}",
                                        h.accessible_role(),
                                        h.absolute_position(),
                                        h.size()
                                    )
                                })
                                .collect();
                        eprintln!(
                            "category tap did not select {}: settled={} hits={hits:?}",
                            e.category,
                            d.ui.get_morph_settled()
                        );
                    }
                } else {
                    eprintln!(
                        "category tap failed (first): {} cats={:?}",
                        e.category,
                        d.ui.get_expense_categories()
                            .iter()
                            .map(|c| c.name.to_string())
                            .collect::<Vec<_>>()
                    );
                }
                if account_id(&d.ui, &e.account)
                    != d.ui.get_accounts().row_data(0).map(|a| a.id).unwrap_or(-1)
                {
                    d.tap(&e.account);
                }
                if roll(rng, 0.4) {
                    d.type_text("Note", e.note);
                }
            }
            for _ in 0..days_ago {
                d.tap("Earlier day");
            }
            if days_ago > 0 {
                d.rep.backdated += 1;
            }
            let mut stored_minute = minute as i64;
            if roll(rng, p.adjusts_time) {
                // step the clock towards the real time, at most 8 taps
                let mut current = minute as i64;
                if days_ago == 0 || (e.minute as i64 - current).abs() > 90 {
                    // quick chips first, then fine steps
                    let m = minute as i64;
                    let options = [
                        ("1 h ago", (m - 60).rem_euclid(1440)),
                        ("3 h ago", (m - 180).rem_euclid(1440)),
                        ("Morning", 480),
                        ("Night", 1260),
                    ];
                    let dist = |a: i64, b: i64| (a - b).abs().min(1440 - (a - b).abs());
                    let (name, at) = options
                        .iter()
                        .min_by_key(|(_, at)| dist(*at, e.minute as i64))
                        .unwrap();
                    if dist(*at, e.minute as i64) < dist(current, e.minute as i64) && d.tap(name) {
                        current = *at;
                    }
                }
                for _ in 0..6 {
                    let diff = e.minute as i64 - current;
                    if diff.abs() < 15 {
                        break;
                    }
                    if diff < 0 {
                        d.tap("15 minutes earlier");
                        current -= 15;
                    } else {
                        d.tap("15 minutes later");
                        current += 15;
                    }
                }
                stored_minute = current;
            }
            let ok = d.tap("Save entry");
            if std::env::var("SIM_DEBUG").is_ok() {
                eprintln!(
                    "save tap ok={ok} adding={} err={:?} amount={:?} cat={:?}",
                    d.ui.get_adding(),
                    d.ui.get_entry_error(),
                    typed.format(),
                    e.category
                );
            }
            if skipped_category {
                let err = d.ui.get_entry_error().to_string();
                if err.is_empty() || !d.ui.get_adding() {
                    d.rep.bug(user, "saved an entry without a category".into());
                } else {
                    *d.rep.expected_errors_shown.entry(err).or_default() += 1;
                }
                let ok = d.tap(e.category);
                if ok && std::env::var("SIM_DEBUG").is_ok() && d.ui.get_entry_category() < 0 {
                    let hits: Vec<String> =
                        ElementHandle::find_by_accessible_label(&d.ui, e.category)
                            .map(|h| {
                                format!(
                                    "{:?}@{:?} {:?}",
                                    h.accessible_role(),
                                    h.absolute_position(),
                                    h.size()
                                )
                            })
                            .collect();
                    eprintln!(
                        "retry did not select {}: scroll={} hits={hits:?}",
                        e.category,
                        d.ui.get_scroll_pos()
                    );
                }
                if !ok {
                    eprintln!(
                        "category tap failed: {} cats={:?}",
                        e.category,
                        d.ui.get_expense_categories()
                            .iter()
                            .map(|c| c.name.to_string())
                            .collect::<Vec<_>>()
                    );
                }
                d.tap("Save entry");
            }
            let err = d.ui.get_entry_error().to_string();
            if d.ui.get_adding() && days_ago == 0 && stored_minute > minute as i64 {
                // future time on today: the app refuses, the user taps Now
                *d.rep.expected_errors_shown.entry(err.clone()).or_default() += 1;
                d.tap("Now");
                d.tap("Save entry");
                stored_minute = minute as i64;
            }
            if d.ui.get_adding() {
                d.rep.bug(
                    user,
                    format!(
                        "could not save {} {}: “{}”",
                        e.note,
                        typed.format(),
                        d.ui.get_entry_error()
                    ),
                );
                d.tap("Close");
                continue;
            }
            d.rep.time_error_min.push(
                (stored_minute - e.minute as i64)
                    .abs()
                    .min(1440 - (stored_minute - e.minute as i64).abs()),
            );
            let delta = e.kind.signed(typed);
            *shadow.entry(e.account.clone()).or_default() += delta;
            debug_check(
                dir,
                &shadow,
                &format!(
                    "save {} {} {} typed {} wanted {}",
                    e.note,
                    e.account,
                    e.kind.signed(e.amount).format(),
                    typed.format(),
                    e.amount.format()
                ),
            );
            logged += 1;
            persona_entries += 1;
            d.rep.entries_logged += 1;
            d.rep.entry_taps.push(d.taps_this_entry);
            if typed != e.amount && roll(rng, 0.5) {
                // notices the typo on Home and fixes it
                d.go_home();
                let recent = d.ui.get_recent();
                if let Some(idx) = recent
                    .iter()
                    .position(|r| parse_shown(&r.amount).map(|m| m.abs()) == Some(typed))
                {
                    let row = recent.row_data(idx).unwrap();
                    if std::env::var("SIM_DEBUG").is_ok() {
                        let titles: Vec<_> = recent
                            .iter()
                            .map(|r| format!("{}#{} {}", r.title, r.id, r.amount))
                            .collect();
                        eprintln!(
                            "typo-fix: idx={idx} row={}#{} typed={} wanted={} recent={titles:?}",
                            row.title,
                            row.id,
                            typed.format(),
                            e.amount.format()
                        );
                    }
                    if d.tap_nth(&row.title, same_title_before(&recent, idx)) {
                        d.clear_amount(money_text(typed).len());
                        d.type_amount(&money_text(e.amount));
                        d.tap("Save changes");
                        if d.ui.get_adding() {
                            d.rep
                                .bug(user, format!("editing failed: {}", d.ui.get_entry_error()));
                            d.tap("Close");
                        } else {
                            *shadow.entry(e.account.clone()).or_default() +=
                                e.kind.signed(e.amount) - delta;
                            d.rep.typos_fixed_later += 1;
                            debug_check(dir, &shadow, "typo fix");
                            d.rep.use_feature("edit");
                        }
                    }
                }
            }
            d.wait(5_000); // toast goes away
        }

        // extras
        if roll(rng, p.p_stats) {
            d.go_home();
            d.tap("Stats");
            d.check_page("Deep");
            d.rep.use_feature("stats");
            let r = rng.next(4) as usize;
            d.tap(["This cycle", "Last cycle", "90 days", "12 months"][r]);
            let sv = d.ui.get_stats();
            for f in [
                &sv.earn_hour,
                &sv.burn_hour,
                &sv.burn_minute,
                &sv.avg,
                &sv.net,
            ] {
                if f.contains("NaN") || f.contains("inf") {
                    d.rep.bug(user, format!("stats shows {f}"));
                }
            }
            if d.ui.get_flow().row_count() == 0 {
                d.rep.bug(user, "cash-flow chart is empty".into());
            }
        }
        if roll(rng, p.p_search) && logged > 0 {
            d.go_home();
            d.tap("Activity");
            d.check_page("Your");
            let habit = &p.habits[rng.next(p.habits.len() as u64) as usize];
            let q = &habit.note[..habit.note.len().min(4)];
            d.type_text("Search notes, categories, accounts", q);
            d.rep.use_feature("search");
            let rows = d.ui.get_activity().iter().filter(|r| !r.header).count();
            let store = Store::open(dir.join("income-radar.db")).unwrap();
            let expected = store.search(None, q, 300).unwrap().len();
            if rows != expected {
                d.rep.bug(
                    user,
                    format!("search “{q}” shows {rows} rows, database has {expected}"),
                );
            }
            d.type_text("Search notes, categories, accounts", "");
        }
        if roll(rng, p.p_edit) && logged > 0 {
            d.go_home();
            if let Some(row) = d.ui.get_recent().row_data(0) {
                if d.tap(&row.title) {
                    // change only the note: amount stays
                    d.type_text("Note", "edited in simulation");
                    d.tap("Save changes");
                    if d.ui.get_adding() {
                        d.rep
                            .bug(user, format!("edit failed: {}", d.ui.get_entry_error()));
                        d.tap("Close");
                    } else {
                        d.rep.use_feature("edit");
                    }
                }
            }
        }
        if roll(rng, p.p_delete) && logged > 0 {
            d.go_home();
            if let Some(row) = d.ui.get_recent().row_data(0) {
                let amount = parse_shown(&row.amount).unwrap_or_default();
                let account = row.meta.split(" · ").last().unwrap_or("").to_string();
                if d.tap(&row.title) {
                    d.tap("Delete entry");
                    d.rep.use_feature("delete");
                    let undo = roll(rng, 0.4);
                    if undo && d.tap("Undo") {
                        d.rep.undo_used += 1;
                    } else {
                        *shadow.entry(account).or_default() += Money(-amount.0);
                        logged -= 1;
                    }
                }
            }
            d.wait(5_000);
        }
        if roll(rng, p.p_transfer) && accounts.len() >= 2 {
            d.go_home();
            d.tap("Accounts");
            if d.tap("Move money") {
                let amount = Money((1 + rng.next(20) as i64) * 5_000);
                d.type_amount(&money_text(amount));
                let from = accounts[1].clone();
                let to = accounts[0].clone();
                d.tap_nth(&from, 0);
                d.tap_nth(&to, 1);
                d.tap("Move money");
                if d.ui.get_adding() {
                    d.rep
                        .bug(user, format!("transfer failed: {}", d.ui.get_entry_error()));
                    d.tap("Close");
                } else {
                    *shadow.entry(from).or_default() += Money(-amount.0);
                    *shadow.entry(to).or_default() += amount;
                    d.rep.use_feature("transfer");
                    debug_check(dir, &shadow, "transfer");
                }
            }
            d.wait(5_000);
        }
        if roll(rng, p.p_budget) {
            d.go_home();
            d.tap("Stats");
            if d.tap("Set budgets") {
                let rows = d.ui.get_budget_rows();
                let idx = rng.next(rows.row_count().max(1) as u64) as usize;
                let label = if rows
                    .row_data(idx)
                    .map(|r| r.limit.is_empty())
                    .unwrap_or(true)
                {
                    "Set"
                } else {
                    "Edit"
                };
                let nth = rows
                    .iter()
                    .take(idx)
                    .filter(|r| (r.limit.is_empty()) == (label == "Set"))
                    .count();
                if d.tap_nth(label, nth) {
                    let limit = if roll(rng, 0.1) {
                        "abc".to_string()
                    } else {
                        (100 + rng.next(2000)).to_string()
                    };
                    d.type_text("Monthly limit, empty = none", &limit);
                    d.tap("Save");
                    d.rep.use_feature("budget");
                    let ok =
                        d.ui.get_budget_rows()
                            .row_data(idx)
                            .map(|r| r.limit.to_string())
                            == Some(limit.clone());
                    if limit == "abc" {
                        if ok {
                            d.rep.bug(user, "budget accepted letters".into());
                        } else {
                            *d.rep
                                .expected_errors_shown
                                .entry("budget: not a number".into())
                                .or_default() += 1;
                        }
                    } else if !ok {
                        if std::env::var("SIM_DEBUG").is_ok() {
                            let limits: Vec<String> =
                                d.ui.get_budget_rows()
                                    .iter()
                                    .map(|r| format!("{}={}", r.name, r.limit))
                                    .collect();
                            eprintln!(
                                "budget miss: idx={idx} label={label} nth={nth} err={:?} rows={limits:?}",
                                d.ui.get_entry_error()
                            );
                        }
                        d.rep.bug(user, format!("budget {limit} not saved"));
                    }
                }
                d.tap("Back");
            }
        }
        if roll(rng, p.p_settings) {
            d.go_home();
            d.tap("Settings");
            d.check_page("App");
            d.rep.use_feature("settings");
            d.tap(["Lime", "Solar", "Ember", "Lilac"][rng.next(4) as usize]);
            if roll(rng, 0.3) {
                d.tap(if roll(rng, 0.5) { "Light" } else { "Dark" });
            }
            d.tap("Back");
        }
        if roll(rng, p.p_account) {
            d.go_home();
            d.tap("Accounts");
            d.tap("New account");
            // odd names people type: empty, duplicate, Arabic, very long
            let name = match rng.next(4) {
                0 => "".to_string(),
                1 => "cash".to_string(),
                2 => "محفظة".to_string(),
                _ => "My very long account name for everyday spending money".to_string(),
            };
            d.type_text("Name", &name);
            d.tap("Add account");
            d.rep.use_feature("add account");
            let created = d.ui.get_accounts().iter().any(|a| {
                !name.is_empty() && name.chars().take(40).collect::<String>() == a.name.as_str()
            });
            match (name.as_str(), created) {
                ("", false) | ("cash", false) => {
                    *d.rep
                        .expected_errors_shown
                        .entry("account: empty or duplicate name".into())
                        .or_default() += 1;
                }
                ("", true) | ("cash", true) => {
                    d.rep.bug(user, format!("account “{name}” was accepted"))
                }
                (_, true) => {
                    let stored: String = name.chars().take(40).collect();
                    shadow.insert(stored.clone(), Money::ZERO);
                    accounts.push(stored);
                }
                (_, false) => d.rep.bug(user, format!("account “{name}” was rejected")),
            }
        }

        // ---- checks: UI vs shadow ledger, database invariants
        d.go_home();
        d.ui.invoke_scroll_to(0.0);
        d.check_page("Income");
        d.tap("Accounts");
        d.check_page("Your");
        d.go_home();
        d.rep.invariant_checks += 1;
        let ui_accounts: HashMap<String, Money> =
            d.ui.get_accounts()
                .iter()
                .map(|a| {
                    (
                        a.name.to_string(),
                        parse_shown(&a.balance).unwrap_or(Money(i64::MIN)),
                    )
                })
                .collect();
        for (name, expected) in &shadow {
            match ui_accounts.get(name) {
                Some(shown) if shown == expected => {}
                Some(shown) => d.rep.bug(
                    user,
                    format!(
                        "{name} shows {} but should be {}",
                        shown.format(),
                        expected.format()
                    ),
                ),
                None => d.rep.bug(user, format!("account {name} missing")),
            }
        }
        let total: Money = shadow.values().copied().sum();
        if parse_shown(&d.ui.get_dash().balance) != Some(total) {
            d.rep.bug(
                user,
                format!(
                    "home balance {} ≠ {}",
                    d.ui.get_dash().balance,
                    total.format()
                ),
            );
        }
        let dash = d.ui.get_dash();
        for f in [
            &dash.balance,
            &dash.allowance,
            &dash.today_left,
            &dash.income,
            &dash.expense,
            &dash.projected,
        ] {
            if f.contains("NaN") || f.as_str() == "-0.00" || f.starts_with("--") {
                d.rep.bug(user, format!("home shows “{f}”"));
            }
        }
        if !(0.0..=1.0).contains(&dash.today_ratio) || !(0.0..=1.0).contains(&dash.spent_ratio) {
            d.rep.bug(user, "a progress bar is outside 0–100%".into());
        }
        let store = Store::open(dir.join("income-radar.db")).unwrap();
        if store.entry_count().unwrap() as usize != logged {
            d.rep.bug(
                user,
                format!(
                    "database has {} entries, user logged {logged}",
                    store.entry_count().unwrap()
                ),
            );
        }
        let radar = Radar::compute(&store, date).unwrap();
        if radar.safe_per_day.0 < 0 || radar.days_left == 0 || !radar.period.contains(date) {
            d.rep.bug(user, "radar figures out of range".into());
        }
        for range in Range::ALL {
            let t = Instant::now();
            let st = Stats::compute(&store, date, minute, range).unwrap();
            d.rep.stats_ms.push(t.elapsed().as_secs_f64() * 1000.0);
            let sum = |v: &[Money]| v.iter().copied().sum::<Money>();
            if sum(&st.by_hour) != st.expense
                || sum(&st.by_weekday) != st.expense
                || st.categories.iter().map(|c| c.total).sum::<Money>() != st.expense
                || st.buckets.iter().map(|b| b.expense).sum::<Money>() != st.expense
                || st.by_hour_count.iter().sum::<u32>() != st.expense_count
            {
                d.rep
                    .bug(user, format!("statistics do not add up for {range:?}"));
            }
        }
        drop(store);
        drop(d);
    }
    rep.per_persona.entry(p.name).or_default().0 += persona_entries;
    rep.per_persona.entry(p.name).or_default().1 += SESSIONS as u32;
}

fn account_id(ui: &AppWindow, name: &str) -> i32 {
    ui.get_accounts()
        .iter()
        .find(|a| a.name.as_str() == name)
        .map(|a| a.id)
        .unwrap_or(-1)
}

fn pct(v: &mut [u64], p: f64) -> u64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn render(r: &Report, took: Duration) -> String {
    let mut s = String::new();
    let mut taps = r.tap_times_us.clone();
    let mut opens = r.open_times_us.clone();
    let mut entry_taps: Vec<u64> = r.entry_taps.iter().map(|&t| t as u64).collect();
    let mut time_err: Vec<u64> = r.time_error_min.iter().map(|&t| t as u64).collect();
    let mut stats_us: Vec<u64> = r.stats_ms.iter().map(|&m| (m * 1000.0) as u64).collect();
    let rss_first = r.rss_kb.first().copied().unwrap_or(0);
    let rss_last = r.rss_kb.last().copied().unwrap_or(0);
    let mut db = r.db_bytes.clone();
    let _ = writeln!(s, "# Simulation results\n");
    let _ = writeln!(
        s,
        "Generated by `cargo run -p income-radar --example simulate --release`."
    );
    let _ = writeln!(s, "{USERS} people × {SESSIONS} sessions, driving the real UI with simulated taps. Took {:.1} s.\n", took.as_secs_f64());
    let _ = writeln!(s, "## Volume\n");
    let _ = writeln!(s, "| | |\n|---|---|");
    let _ = writeln!(s, "| Sessions | {} |", r.sessions);
    let _ = writeln!(s, "| Taps and text inputs | {} |", r.taps);
    let _ = writeln!(s, "| Entries logged | {} |", r.entries_logged);
    let _ = writeln!(
        s,
        "| Invariant checks (UI vs. shadow ledger, statistics sums) | {} sessions |",
        r.invariant_checks
    );
    let _ = writeln!(s, "| Unexpected problems | **{}** |", r.unexpected.len());
    let _ = writeln!(s, "\n## Speed (desktop CPU, debug-free release build)\n");
    let _ = writeln!(s, "| | p50 | p95 | max |\n|---|---|---|---|");
    let _ = writeln!(
        s,
        "| Tap → screen updated | {:.2} ms | {:.2} ms | {:.2} ms |",
        pct(&mut taps, 0.5) as f64 / 1000.0,
        pct(&mut taps, 0.95) as f64 / 1000.0,
        pct(&mut taps, 1.0) as f64 / 1000.0
    );
    let _ = writeln!(
        s,
        "| App open (database + first frame data) | {:.2} ms | {:.2} ms | {:.2} ms |",
        pct(&mut opens, 0.5) as f64 / 1000.0,
        pct(&mut opens, 0.95) as f64 / 1000.0,
        pct(&mut opens, 1.0) as f64 / 1000.0
    );
    let _ = writeln!(
        s,
        "| Statistics for one range | {:.2} ms | {:.2} ms | {:.2} ms |",
        pct(&mut stats_us, 0.5) as f64 / 1000.0,
        pct(&mut stats_us, 0.95) as f64 / 1000.0,
        pct(&mut stats_us, 1.0) as f64 / 1000.0
    );
    let _ = writeln!(
        s,
        "\nDatabase size per person after 5 sessions: p50 {} KB, max {} KB.",
        pct(&mut db, 0.5) / 1024,
        pct(&mut db, 1.0) / 1024
    );
    let _ = writeln!(s, "Process memory (RSS) after user 1: {} MB, after user {USERS}: {} MB (all 500 sessions in one process; a steady rise would mean a leak).", rss_first / 1024, rss_last / 1024);
    let _ = writeln!(s, "\n## Logging effort\n");
    let _ = writeln!(s, "| | |\n|---|---|");
    let _ = writeln!(
        s,
        "| Taps per entry (p50 / p95) | {} / {} |",
        pct(&mut entry_taps, 0.5),
        pct(&mut entry_taps, 0.95)
    );
    let _ = writeln!(
        s,
        "| Entries made from a suggestion chip | {} of {} ({:.0}%) |",
        r.entries_via_suggestion,
        r.entries_logged,
        r.entries_via_suggestion as f64 * 100.0 / r.entries_logged.max(1) as f64
    );
    let _ = writeln!(
        s,
        "| Suggestion offered for the entry being logged | {} times |",
        r.suggestions_offered
    );
    let _ = writeln!(
        s,
        "| Suggestion picked but amount retyped | {} |",
        r.suggestion_amount_retyped
    );
    let _ = writeln!(s, "| Backdated entries (Earlier day) | {} |", r.backdated);
    let _ = writeln!(
        s,
        "| Time of day off by (p50 / p95) | {} min / {} min |",
        pct(&mut time_err, 0.5),
        pct(&mut time_err, 0.95)
    );
    let _ = writeln!(
        s,
        "| Real purchases never logged (forgot) | {} |",
        r.events_forgotten
    );
    let _ = writeln!(
        s,
        "| Purchases older than 6 days at next session (skipped) | {} |",
        r.events_too_old
    );
    let _ = writeln!(
        s,
        "| Typos caught before saving / saved / fixed later | {} / {} / {} |",
        r.typos_caught, r.typos_uncaught, r.typos_fixed_later
    );
    let _ = writeln!(s, "| Undo used after delete | {} |", r.undo_used);
    let _ = writeln!(s, "\n## Reachability\n");
    let _ = writeln!(s, "| | |\n|---|---|");
    let _ = writeln!(
        s,
        "| Target was behind the floating dock (user scrolls) | {} |",
        r.hidden_by_dock
    );
    let _ = writeln!(
        s,
        "| Target needed vertical scrolling | {} |",
        r.needed_scroll
    );
    let _ = writeln!(
        s,
        "| Target needed a sideways swipe (chip rows) | {} |",
        r.needed_swipe
    );
    let _ = writeln!(s, "\n## Errors the app showed (expected)\n");
    for (k, v) in &r.expected_errors_shown {
        let _ = writeln!(s, "* {v} × “{k}”");
    }
    let _ = writeln!(s, "\n## Feature use\n");
    for (k, v) in &r.feature_use {
        let _ = writeln!(s, "* {k}: {v}");
    }
    let _ = writeln!(
        s,
        "\n## Per persona\n\n| Persona | Entries | Sessions |\n|---|---|---|"
    );
    for (k, (e, n)) in &r.per_persona {
        let _ = writeln!(s, "| {k} | {e} | {n} |");
    }
    let _ = writeln!(s, "\n## Unexpected problems\n");
    if r.unexpected.is_empty() {
        let _ = writeln!(s, "None.");
    }
    for u in &r.unexpected {
        let _ = writeln!(s, "* {u}");
    }
    s
}
