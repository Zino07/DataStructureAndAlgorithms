//! Income Radar UI glue: wires the Slint interface to the core store.

slint::include_modules!();

pub mod demo;
mod icons;

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use income_radar_core::*;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

const ACTIVITY_LIMIT: usize = 300;
const TOAST_MS: u64 = 4_000;

/// Local date and minute of the day. `INCOME_RADAR_NOW="YYYY-MM-DD HH:MM"`
/// (or `INCOME_RADAR_TODAY="YYYY-MM-DD"`) pins the clock for screenshots,
/// demos and simulations.
pub fn now() -> (NaiveDate, u16) {
    if let Ok(s) = std::env::var("INCOME_RADAR_NOW") {
        if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M") {
            return (dt.date(), (dt.hour() * 60 + dt.minute()) as u16);
        }
    }
    let local = Local::now();
    let minute = (local.hour() * 60 + local.minute()) as u16;
    if let Ok(s) = std::env::var("INCOME_RADAR_TODAY") {
        if let Ok(d) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
            return (d, 14 * 60 + 5);
        }
    }
    (local.date_naive(), minute)
}

pub fn today() -> NaiveDate {
    now().0
}

/// What the Undo button in the toast reverts.
enum Undo {
    Created(Id),
    CreatedTransfer(Id),
    Deleted(NewTransaction),
    DeletedTransfer(NewTransfer),
    Edited(Id, NewTransaction),
}

struct Ctx {
    store: Store,
    activity_filter: Cell<i32>,
    query: RefCell<String>,
    stats_range: Cell<i32>,
    undo: RefCell<Option<Undo>>,
    toast_timer: slint::Timer,
    clock_timer: slint::Timer,
    last_minute: Cell<(NaiveDate, u16)>,
}

/// Opens (or creates) the database in `data_dir` and builds the window.
pub fn build(data_dir: &Path) -> Result<AppWindow, Box<dyn Error>> {
    std::fs::create_dir_all(data_dir)?;
    let store = Store::open(data_dir.join("income-radar.db"))?;
    let ui = AppWindow::new()?;
    ui.set_version(env!("CARGO_PKG_VERSION").into());
    if cfg!(target_os = "android") {
        ui.set_min_top_inset(28.0);
        ui.set_min_bottom_inset(16.0);
    }
    let ctx = Rc::new(Ctx {
        store,
        activity_filter: Cell::new(0),
        query: RefCell::new(String::new()),
        stats_range: Cell::new(0),
        undo: RefCell::new(None),
        toast_timer: slint::Timer::default(),
        clock_timer: slint::Timer::default(),
        last_minute: Cell::new(now()),
    });
    wire(&ui, &ctx);
    refresh(&ui, &ctx);

    // Keep every per-minute figure live: refresh when the minute changes.
    // The timer lives inside `ctx`, so it must only hold a weak reference to
    // it; a strong one would keep the database open after the window closes.
    let weak = ui.as_weak();
    let c = Rc::downgrade(&ctx);
    ctx.clock_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(15),
        move || {
            let Some(c) = c.upgrade() else { return };
            let n = now();
            if n != c.last_minute.get() {
                c.last_minute.set(n);
                if let Some(ui) = weak.upgrade() {
                    refresh(&ui, &c);
                }
            }
        },
    );
    Ok(ui)
}

fn toast(ui: &AppWindow, ctx: &Rc<Ctx>, text: &str, undo: Option<Undo>) {
    let has_undo = undo.is_some();
    *ctx.undo.borrow_mut() = undo;
    ui.set_toast_text(text.into());
    ui.set_toast_action(if has_undo {
        "Undo".into()
    } else {
        SharedString::new()
    });
    let weak = ui.as_weak();
    let c = Rc::downgrade(ctx);
    ctx.toast_timer.start(
        slint::TimerMode::SingleShot,
        Duration::from_millis(TOAST_MS),
        move || {
            let Some(c) = c.upgrade() else { return };
            *c.undo.borrow_mut() = None;
            if let Some(ui) = weak.upgrade() {
                ui.set_toast_text(SharedString::new());
            }
        },
    );
}

fn as_new(t: &Transaction) -> NewTransaction {
    NewTransaction {
        account_id: t.account_id,
        category_id: t.category_id,
        kind: t.kind,
        amount: t.amount,
        date: t.date,
        minute: t.minute,
        note: t.note.clone(),
    }
}

fn wire(ui: &AppWindow, ctx: &Rc<Ctx>) {
    let weak = ui.as_weak();

    ui.on_edit_amount(|current, key| edit_amount(&current, &key).into());
    ui.on_day_label(|days_ago| {
        let today = today();
        day_label(today - chrono::Days::new(days_ago.max(0) as u64), today).into()
    });
    ui.on_time_label(|minute| time_label(minute.clamp(0, 1439) as u16).into());

    ui.on_save_entry({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id, kind, amount, category, account, to_account, days_ago, minute, note| {
            let Some(ui) = weak.upgrade() else {
                return SharedString::new();
            };
            let input = EntryInput {
                kind,
                amount: amount.to_string(),
                category,
                account,
                to_account,
                days_ago,
                minute,
                note: note.to_string(),
            };
            match save_entry(&ctx.store, id, &input) {
                Ok((message, undo)) => {
                    refresh(&ui, &ctx);
                    toast(&ui, &ctx, &message, Some(undo));
                    SharedString::new()
                }
                Err(e) => e.into(),
            }
        }
    });

    ui.on_delete_entry({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id| {
            let Some(ui) = weak.upgrade() else { return };
            let before = ctx.store.transaction(id as Id).ok().flatten();
            match ctx.store.delete_transaction(id as Id) {
                Ok(true) => {
                    refresh(&ui, &ctx);
                    let undo = before.as_ref().map(|t| Undo::Deleted(as_new(t)));
                    toast(&ui, &ctx, "Entry deleted", undo);
                }
                Ok(false) => toast(&ui, &ctx, "That entry was already deleted", None),
                Err(e) => toast(&ui, &ctx, &capitalize(&e.to_string()), None),
            }
        }
    });

    ui.on_delete_transfer({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id| {
            let Some(ui) = weak.upgrade() else { return };
            let before = ctx
                .store
                .transfers(usize::MAX >> 1)
                .ok()
                .and_then(|list| list.into_iter().find(|t| t.id == id as Id));
            if let Ok(true) = ctx.store.delete_transfer(id as Id) {
                refresh(&ui, &ctx);
                let undo = before.map(|t| {
                    Undo::DeletedTransfer(NewTransfer {
                        from_account: t.from_account,
                        to_account: t.to_account,
                        amount: t.amount,
                        date: t.date,
                        minute: t.minute,
                        note: t.note,
                    })
                });
                toast(&ui, &ctx, "Transfer deleted", undo);
            }
        }
    });

    ui.on_request_edit({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id| {
            let Some(ui) = weak.upgrade() else { return };
            match ctx.store.transaction(id as Id) {
                Ok(Some(t)) => {
                    let days_ago = (today() - t.date).num_days().max(0) as i32;
                    ui.invoke_load_entry(
                        t.id as i32,
                        if t.kind == Kind::Income { 1 } else { 0 },
                        plain_amount(t.amount).into(),
                        t.category_id as i32,
                        t.account_id as i32,
                        days_ago,
                        t.minute as i32,
                        t.note.into(),
                    );
                }
                _ => toast(&ui, &ctx, "That entry no longer exists", None),
            }
        }
    });

    ui.on_toast_act({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let undo = ctx.undo.borrow_mut().take();
            let result = match undo {
                Some(Undo::Created(id)) => ctx.store.delete_transaction(id).map(|_| ()),
                Some(Undo::CreatedTransfer(id)) => ctx.store.delete_transfer(id).map(|_| ()),
                Some(Undo::Deleted(t)) => ctx.store.add_transaction(&t).map(|_| ()),
                Some(Undo::DeletedTransfer(t)) => ctx.store.add_transfer(&t).map(|_| ()),
                Some(Undo::Edited(id, t)) => ctx.store.update_transaction(id, &t),
                None => Ok(()),
            };
            refresh(&ui, &ctx);
            match result {
                Ok(()) => toast(&ui, &ctx, "Undone", None),
                Err(e) => toast(&ui, &ctx, &capitalize(&e.to_string()), None),
            }
        }
    });

    ui.on_activity_filter_changed({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |filter| {
            ctx.activity_filter.set(filter);
            if let Some(ui) = weak.upgrade() {
                refresh_activity(&ui, &ctx);
            }
        }
    });

    ui.on_activity_query_changed({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |q| {
            *ctx.query.borrow_mut() = q.to_string();
            if let Some(ui) = weak.upgrade() {
                refresh_activity(&ui, &ctx);
            }
        }
    });

    ui.on_stats_range_changed({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |r| {
            ctx.stats_range.set(r.clamp(0, 3));
            if let Some(ui) = weak.upgrade() {
                refresh(&ui, &ctx);
            }
        }
    });

    ui.on_screen_changed({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.set_toast_text(SharedString::new());
                *ctx.undo.borrow_mut() = None;
            }
        }
    });

    ui.on_add_account({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |name, kind, opening| {
            let opening = if opening.trim().is_empty() {
                Ok(Money::ZERO)
            } else {
                Money::parse(&opening)
                    .map_err(|_| "Opening balance is not a valid amount".to_string())
            };
            let kind = AccountKind::ALL
                .get(kind as usize)
                .copied()
                .unwrap_or(AccountKind::Cash);
            let result = opening.and_then(|opening| {
                ctx.store
                    .add_account(&name, kind, opening)
                    .map_err(|e| capitalize(&e.to_string()))
            });
            match result {
                Ok(_) => {
                    if let Some(ui) = weak.upgrade() {
                        refresh(&ui, &ctx);
                        toast(&ui, &ctx, &format!("Account “{}” added", name.trim()), None);
                    }
                    SharedString::new()
                }
                Err(e) => e.into(),
            }
        }
    });

    ui.on_archive_account({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id| match ctx.store.archive_account(id as Id) {
            Ok(()) => {
                if let Some(ui) = weak.upgrade() {
                    refresh(&ui, &ctx);
                    toast(&ui, &ctx, "Account archived", None);
                }
                SharedString::new()
            }
            Err(e) => capitalize(&e.to_string()).into(),
        }
    });

    ui.on_save_budget({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id, text| {
            let limit = if text.trim().is_empty() {
                Ok(None)
            } else {
                match Money::parse(&text) {
                    Ok(m) if m.0 > 0 => Ok(Some(m)),
                    Ok(_) => Ok(None),
                    Err(e) => Err(capitalize(&e.to_string())),
                }
            };
            let result = limit.and_then(|l| {
                ctx.store
                    .set_budget(id as Id, l)
                    .map_err(|e| capitalize(&e.to_string()))
            });
            match result {
                Ok(()) => {
                    if let Some(ui) = weak.upgrade() {
                        refresh(&ui, &ctx);
                    }
                    SharedString::new()
                }
                Err(e) => e.into(),
            }
        }
    });

    let update_settings: SettingsUpdate = {
        let (ctx, weak) = (ctx.clone(), weak.clone());
        Rc::new(move |change: &dyn Fn(&mut Settings)| {
            let mut settings = ctx.store.settings().unwrap_or_default();
            change(&mut settings);
            if let Err(e) = ctx.store.save_settings(&settings) {
                eprintln!("saving settings failed: {e}");
            }
            if let Some(ui) = weak.upgrade() {
                refresh(&ui, &ctx);
            }
        })
    };

    ui.on_palette_changed({
        let update = update_settings.clone();
        move |p| update(&|s| s.palette = p.clamp(0, 3) as u32)
    });
    ui.on_dark_changed({
        let update = update_settings.clone();
        move |d| update(&|s| s.dark = d)
    });
    ui.on_cycle_day_changed({
        let update = update_settings.clone();
        move |d| update(&|s| s.cycle_start_day = d.clamp(1, 28) as u32)
    });
    ui.on_weekend_changed({
        let update = update_settings.clone();
        move |w| {
            update(&|s| {
                s.weekend = if w == 1 {
                    Weekend::SatSun
                } else {
                    Weekend::FriSat
                }
            })
        }
    });
    ui.on_currency_changed({
        let update = update_settings.clone();
        move |c| update(&|s| s.currency = c.to_string())
    });

    ui.on_load_demo({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move || {
            if let Err(e) = demo::seed(&ctx.store, today()) {
                eprintln!("loading sample data failed: {e}");
            }
            if let Some(ui) = weak.upgrade() {
                refresh(&ui, &ctx);
            }
        }
    });
}

type SettingsUpdate = Rc<dyn Fn(&dyn Fn(&mut Settings))>;

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Raw values from the entry screen.
pub struct EntryInput {
    /// 0 expense, 1 income, 2 transfer
    pub kind: i32,
    pub amount: String,
    pub category: i32,
    pub account: i32,
    pub to_account: i32,
    pub days_ago: i32,
    /// -1 means "now"
    pub minute: i32,
    pub note: String,
}

/// Validates and stores an entry. Returns the toast message and how to undo it.
fn save_entry(store: &Store, editing: i32, e: &EntryInput) -> Result<(String, Undo), String> {
    let amount = match Money::parse(&e.amount) {
        Ok(m) if m.0 > 0 => m,
        Ok(_) | Err(ParseMoneyError::Empty) => return Err("Enter an amount".into()),
        Err(err) => return Err(capitalize(&err.to_string())),
    };
    let (today, now_minute) = now();
    let date = today - chrono::Days::new(e.days_ago.clamp(0, 365) as u64);
    let minute = if e.minute < 0 {
        now_minute
    } else {
        e.minute.clamp(0, 1439) as u16
    };
    if date == today && e.minute >= 0 && minute > now_minute + 5 {
        return Err("That time is still in the future".into());
    }
    if e.account < 0 {
        return Err("Add an account first".into());
    }
    let fail = |err: StoreError| capitalize(&err.to_string());

    if e.kind == 2 {
        if e.to_account < 0 {
            return Err("Add a second account to move money".into());
        }
        let id = store
            .add_transfer(&NewTransfer {
                from_account: e.account as Id,
                to_account: e.to_account as Id,
                amount,
                date,
                minute,
                note: e.note.clone(),
            })
            .map_err(fail)?;
        return Ok((
            format!("Moved {}", amount.format()),
            Undo::CreatedTransfer(id),
        ));
    }
    if e.category < 0 {
        return Err("Choose a category".into());
    }
    let entry = NewTransaction {
        account_id: e.account as Id,
        category_id: e.category as Id,
        kind: if e.kind == 1 {
            Kind::Income
        } else {
            Kind::Expense
        },
        amount,
        date,
        minute,
        note: e.note.clone(),
    };
    if editing >= 0 {
        let before = store
            .transaction(editing as Id)
            .map_err(fail)?
            .ok_or_else(|| "This entry no longer exists".to_string())?;
        store
            .update_transaction(editing as Id, &entry)
            .map_err(fail)?;
        return Ok((
            "Changes saved".into(),
            Undo::Edited(editing as Id, as_new(&before)),
        ));
    }
    let id = store.add_transaction(&entry).map_err(fail)?;
    let what = if entry.kind == Kind::Income {
        "Income"
    } else {
        "Expense"
    };
    Ok((
        format!("{what} saved · {}", amount.format()),
        Undo::Created(id),
    ))
}

/// Keypad editing: digits, one decimal point, at most two decimals, 12 characters.
pub fn edit_amount(current: &str, key: &str) -> String {
    let mut s = current.to_string();
    match key {
        "<" => {
            s.pop();
        }
        "." => {
            if !s.contains('.') && s.len() < 12 {
                if s.is_empty() {
                    s.push('0');
                }
                s.push('.');
            }
        }
        d if d.len() == 1 && d.as_bytes()[0].is_ascii_digit() => {
            let decimals = s.find('.').map(|i| s.len() - i - 1);
            if s.len() >= 12 || decimals.is_some_and(|n| n >= 2) {
                return s;
            }
            if s == "0" {
                s.clear();
            }
            s.push_str(d);
        }
        _ => {}
    }
    s
}

/// Keypad text for an amount: `"1250.5"` → `"1250.50"`, no grouping.
fn plain_amount(m: Money) -> String {
    let whole = m.0 / 100;
    let frac = m.0 % 100;
    if frac == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{frac:02}")
    }
}

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}

fn day_label(date: NaiveDate, today: NaiveDate) -> String {
    match (today - date).num_days() {
        0 => "Today".into(),
        1 => "Yesterday".into(),
        _ if date.year() == today.year() => date.format("%a, %-d %b").to_string(),
        _ => date.format("%-d %b %Y").to_string(),
    }
}

fn time_label(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

fn tx_row(t: &Transaction, today: NaiveDate, with_date: bool) -> TxRow {
    let title = if t.note.is_empty() {
        t.category_name.clone()
    } else {
        t.note.clone()
    };
    let mut meta = vec![];
    if !t.note.is_empty() {
        meta.push(t.category_name.clone());
    }
    meta.push(t.account_name.clone());
    let sign = if t.kind == Kind::Income { "+" } else { "−" };
    TxRow {
        id: t.id as i32,
        header: false,
        transfer: false,
        title: title.into(),
        meta: meta.join(" · ").into(),
        amount: format!("{sign}{}", t.amount.format()).into(),
        time: if with_date {
            format!("{} · {}", day_label(t.date, today), time_label(t.minute))
        } else {
            time_label(t.minute)
        }
        .into(),
        income: t.kind == Kind::Income,
        icon: icons::path(&t.category_icon).into(),
    }
}

fn delta(pct: Option<i64>) -> (SharedString, bool) {
    match pct {
        Some(p) => (format!("{}%", p.abs()).into(), p >= 0),
        None => (SharedString::new(), true),
    }
}

fn refresh(ui: &AppWindow, ctx: &Ctx) {
    if let Err(e) = try_refresh(ui, ctx) {
        eprintln!("refresh failed: {e}");
    }
}

fn refresh_activity(ui: &AppWindow, ctx: &Ctx) {
    if let Err(e) = try_refresh_activity(ui, ctx, today()) {
        eprintln!("refresh failed: {e}");
    }
}

fn try_refresh_activity(ui: &AppWindow, ctx: &Ctx, today: NaiveDate) -> Result<(), StoreError> {
    let store = &ctx.store;
    let query = ctx.query.borrow().clone();
    let mut rows = Vec::new();
    let mut last_date = None;
    let mut push_header = |rows: &mut Vec<TxRow>, date: NaiveDate| {
        if last_date != Some(date) {
            last_date = Some(date);
            rows.push(TxRow {
                id: -1,
                header: true,
                title: day_label(date, today).into(),
                ..Default::default()
            });
        }
    };
    let mut note = String::new();
    if ctx.activity_filter.get() == 3 {
        let q = query.to_lowercase();
        let list: Vec<Transfer> = store
            .transfers(ACTIVITY_LIMIT)?
            .into_iter()
            .filter(|t| {
                q.is_empty()
                    || t.note.to_lowercase().contains(&q)
                    || t.from_name.to_lowercase().contains(&q)
                    || t.to_name.to_lowercase().contains(&q)
            })
            .collect();
        for t in &list {
            push_header(&mut rows, t.date);
            rows.push(TxRow {
                id: t.id as i32,
                header: false,
                transfer: true,
                title: if t.note.is_empty() {
                    "Transfer".into()
                } else {
                    t.note.clone().into()
                },
                meta: format!("{} → {}", t.from_name, t.to_name).into(),
                amount: t.amount.format().into(),
                time: time_label(t.minute).into(),
                income: false,
                icon: icons::path("transfer").into(),
            });
        }
        if !query.is_empty() {
            note = format!("{} matching transfers", list.len());
        }
    } else {
        let filter = match ctx.activity_filter.get() {
            1 => Some(Kind::Expense),
            2 => Some(Kind::Income),
            _ => None,
        };
        let list = store.search(filter, &query, ACTIVITY_LIMIT)?;
        for t in &list {
            push_header(&mut rows, t.date);
            rows.push(tx_row(t, today, false));
        }
        if !query.is_empty() {
            let spent: Money = list
                .iter()
                .filter(|t| t.kind == Kind::Expense)
                .map(|t| t.amount)
                .sum();
            let earned: Money = list
                .iter()
                .filter(|t| t.kind == Kind::Income)
                .map(|t| t.amount)
                .sum();
            note = format!(
                "{} {} · {} spent · {} received",
                list.len(),
                if list.len() == 1 { "match" } else { "matches" },
                spent.format(),
                earned.format()
            );
        }
    }
    ui.set_activity(model(rows));
    ui.set_activity_note(note.into());
    Ok(())
}

fn sparse_label(i: usize, n: usize, label: String) -> SharedString {
    let step = if n > 20 {
        5
    } else if n > 10 {
        2
    } else {
        1
    };
    if i.is_multiple_of(step) || i + 1 == n {
        label.into()
    } else {
        SharedString::new()
    }
}

fn try_refresh(ui: &AppWindow, ctx: &Ctx) -> Result<(), StoreError> {
    let store = &ctx.store;
    let (today, minute) = now();
    let settings = store.settings()?;

    let theme = ui.global::<Theme>();
    theme.set_palette(settings.palette as i32);
    theme.set_dark(settings.dark);
    ui.set_cycle_day(settings.cycle_start_day as i32);
    ui.set_weekend(if settings.weekend == Weekend::SatSun {
        1
    } else {
        0
    });
    ui.set_now_minute(minute as i32);

    // ----- home -----
    let radar = Radar::compute(store, today)?;
    let (income_delta, income_up) = delta(radar.income_change_pct());
    let (expense_delta, expense_up) = delta(radar.expense_change_pct());
    let week_total: Money = radar.last_7_days.iter().map(|(_, m)| *m).sum();
    let recent = store.transactions(None, 4)?;
    let pace = radar.period.days_elapsed(today) as f32 / radar.period.total_days() as f32;

    ui.set_dash(Dashboard {
        today: today.format("%a, %-d %b").to_string().into(),
        clock: time_label(minute).into(),
        cycle: radar.period.label().into(),
        days_left: match radar.days_left {
            1 => "Last day".to_string(),
            n => format!("{n} days left"),
        }
        .into(),
        currency: settings.currency.clone().into(),
        has_entries: radar.has_entries,
        has_income: radar.income.0 > 0,
        balance: radar.total_balance.format().into(),
        allowance: radar.safe_per_day.format().into(),
        today_left: radar.today_left.abs().format().into(),
        today_over: radar.today_left.0 < 0,
        today_ratio: radar.today_ratio(),
        spent_today: radar.spent_today.format().into(),
        income: radar.income.format_compact().into(),
        income_delta,
        income_up,
        expense: radar.expense.format_compact().into(),
        expense_delta,
        expense_up,
        spent_ratio: radar.spent_ratio(),
        pace_ratio: pace.clamp(0.0, 1.0),
        projected: radar.projected_expense.format_compact().into(),
        savings_rate: radar
            .savings_rate()
            .map(|r| format!("{}%", (r * 100.0).round() as i64))
            .unwrap_or_else(|| "—".into())
            .into(),
        savings_ratio: radar.savings_rate().unwrap_or(0.0),
        week_total: week_total.format().into(),
    });
    let entries = store.entry_count()?;
    ui.set_can_load_demo(entries == 0);
    ui.set_data_note(
        match store.first_entry_date()? {
            Some(d) if entries > 0 => format!(
                "{entries} {} since {}",
                if entries == 1 { "entry" } else { "entries" },
                d.format("%-d %b %Y")
            ),
            _ => String::new(),
        }
        .into(),
    );

    let max = radar
        .last_7_days
        .iter()
        .map(|(_, m)| m.0)
        .max()
        .unwrap_or(0);
    let peak = radar
        .last_7_days
        .iter()
        .rposition(|(_, m)| m.0 == max && max > 0);
    ui.set_week(model(
        radar
            .last_7_days
            .iter()
            .enumerate()
            .map(|(i, (date, m))| Bar {
                value: if max > 0 {
                    m.0 as f32 / max as f32
                } else {
                    0.0
                },
                label: date.format("%a").to_string().into(),
                tip: if Some(i) == peak {
                    m.format_compact().into()
                } else {
                    SharedString::new()
                },
                highlight: Some(i) == peak,
                empty: m.0 == 0,
            })
            .collect(),
    ));
    ui.set_recent(model(
        recent.iter().map(|t| tx_row(t, today, true)).collect(),
    ));

    let limits = store.budget_limits()?;
    let share_row = |id: Id,
                     name: &str,
                     icon: &str,
                     total: Money,
                     count: Option<u32>,
                     of: Money,
                     top: Money| {
        let budget = limits.get(&id).copied();
        CategoryShare {
            id: id as i32,
            name: name.into(),
            icon: icons::path(icon).into(),
            amount: total.format().into(),
            share: if top.0 > 0 {
                total.0 as f32 / top.0 as f32
            } else {
                0.0
            },
            percent: if of.0 > 0 {
                format!("{}%", (total.0 * 100 + of.0 / 2) / of.0)
            } else {
                String::new()
            }
            .into(),
            count: count
                .map(|c| format!("{c} {}", if c == 1 { "entry" } else { "entries" }))
                .unwrap_or_default()
                .into(),
            budget: budget
                .map(|b| format!("budget {}", b.format()))
                .unwrap_or_default()
                .into(),
            budget_ratio: budget
                .filter(|b| b.0 > 0)
                .map(|b| (total.0 as f32 / b.0 as f32).min(1.0))
                .unwrap_or(-1.0),
            over: budget.is_some_and(|b| total > b),
        }
    };
    let top = radar
        .top_expenses
        .first()
        .map(|c| c.total)
        .unwrap_or_default();
    ui.set_top_categories(model(
        radar
            .top_expenses
            .iter()
            .take(4)
            .map(|c| {
                share_row(
                    c.category_id,
                    &c.name,
                    &c.icon,
                    c.total,
                    None,
                    radar.expense,
                    top,
                )
            })
            .collect(),
    ));

    // ----- entry -----
    let categories = |kind| -> Result<ModelRc<CategoryItem>, StoreError> {
        Ok(model(
            store
                .categories_by_use(kind, today)?
                .into_iter()
                .map(|c| CategoryItem {
                    id: c.id as i32,
                    name: c.name.into(),
                    icon: icons::path(&c.icon).into(),
                })
                .collect(),
        ))
    };
    ui.set_expense_categories(categories(Kind::Expense)?);
    ui.set_income_categories(categories(Kind::Income)?);
    let suggestions = |kind| -> Result<ModelRc<SuggestionItem>, StoreError> {
        Ok(model(
            store
                .suggestions(kind, today, minute, 4)?
                .into_iter()
                .map(|s| SuggestionItem {
                    title: if s.note.is_empty() {
                        s.category_name.clone()
                    } else {
                        s.note.clone()
                    }
                    .into(),
                    amount: plain_amount(s.amount).into(),
                    amount_label: s.amount.format().into(),
                    category: s.category_id as i32,
                    account: s.account_id as i32,
                    icon: icons::path(&s.icon).into(),
                    note: s.note.into(),
                })
                .collect(),
        ))
    };
    ui.set_expense_suggestions(suggestions(Kind::Expense)?);
    ui.set_income_suggestions(suggestions(Kind::Income)?);

    ui.set_accounts(model(
        store
            .accounts_with_balances()?
            .into_iter()
            .map(|a| AccountItem {
                id: a.account.id as i32,
                name: a.account.name.into(),
                kind: a.account.kind.label().into(),
                icon: icons::path(a.account.kind.icon()).into(),
                balance: a.balance.format().into(),
                negative: a.balance.0 < 0,
            })
            .collect(),
    ));

    // ----- budgets -----
    let spent_by_cat: std::collections::HashMap<Id, Money> = radar
        .top_expenses
        .iter()
        .map(|c| (c.category_id, c.total))
        .collect();
    ui.set_budget_rows(model(
        store
            .categories(Kind::Expense)?
            .into_iter()
            .map(|c| {
                let limit = limits.get(&c.id).copied();
                let spent = spent_by_cat.get(&c.id).copied().unwrap_or_default();
                BudgetRow {
                    id: c.id as i32,
                    name: c.name.into(),
                    icon: icons::path(&c.icon).into(),
                    limit: limit.map(plain_amount).unwrap_or_default().into(),
                    spent: spent.format().into(),
                    ratio: limit
                        .filter(|l| l.0 > 0)
                        .map(|l| (spent.0 as f32 / l.0 as f32).min(1.0))
                        .unwrap_or(0.0),
                    over: limit.is_some_and(|l| spent > l),
                }
            })
            .collect(),
    ));

    // ----- stats -----
    let range = Range::ALL[ctx.stats_range.get().clamp(0, 3) as usize];
    let s = Stats::compute(store, today, minute, range)?;
    let money_or_dash = |m: Option<Money>| m.map(|m| m.format()).unwrap_or_else(|| "—".into());
    let (s_expense_delta, s_expense_up) = delta(s.expense_change_pct());
    let (s_income_delta, s_income_up) = delta(s.income_change_pct());
    let peak_day_name = s
        .peak_weekday
        .map(|d| match s.week_labels[d] {
            "Mon" => "Monday",
            "Tue" => "Tuesday",
            "Wed" => "Wednesday",
            "Thu" => "Thursday",
            "Fri" => "Friday",
            "Sat" => "Saturday",
            _ => "Sunday",
        })
        .unwrap_or("—");
    ui.set_stats(StatsView {
        range_label: s.label.clone().into(),
        earn_hour: s.income_per_hour.format().into(),
        earn_minute: s.income_per_minute.format().into(),
        earn_work_hour: money_or_dash(s.income_per_work_hour).into(),
        burn_hour: s.burn_per_hour.format().into(),
        burn_minute: s.burn_per_minute.format().into(),
        burn_day: s.burn_per_day.format().into(),
        elapsed: format!("{} tracked", format_minutes(s.elapsed_minutes)).into(),
        income: s.income.format().into(),
        expense: s.expense.format().into(),
        net: s.net().format().into(),
        net_negative: s.net().0 < 0,
        savings_rate: s
            .savings_rate()
            .map(|r| format!("{}%", (r * 100.0).round() as i64))
            .unwrap_or_else(|| "—".into())
            .into(),
        expense_delta: s_expense_delta,
        expense_up: s_expense_up,
        income_delta: s_income_delta,
        income_up: s_income_up,
        runway: s
            .runway_days
            .map(|d| format!("{d} {}", if d == 1 { "day" } else { "days" }))
            .unwrap_or_else(|| "—".into())
            .into(),
        entries: (s.expense_count + s.income_count).to_string().into(),
        entries_per_day: format!("{:.1} a day", s.entries_per_day).into(),
        avg: s.avg_expense.format().into(),
        median: s.median_expense.format().into(),
        gap: s
            .avg_gap_minutes
            .map(format_minutes)
            .unwrap_or_else(|| "—".into())
            .into(),
        work_cost: s
            .purchase_work_minutes
            .map(|m| format_minutes(m.max(1)))
            .unwrap_or_else(|| "—".into())
            .into(),
        peak_hour: s
            .peak_hour
            .map(|h| {
                format!(
                    "{}–{}",
                    time_label(h as u16 * 60),
                    time_label(((h as u16 + 1) % 24) * 60)
                )
            })
            .unwrap_or_else(|| "—".into())
            .into(),
        peak_day: peak_day_name.into(),
        night_share: format!("{}%", (s.night_share * 100.0).round() as i64).into(),
        weekday_avg: s.weekday_daily_avg.format().into(),
        weekend_avg: s.weekend_daily_avg.format().into(),
        no_spend: format!(
            "{} {}",
            s.no_spend_days,
            if s.no_spend_days == 1 { "day" } else { "days" }
        )
        .into(),
        streak: format!("best {} · now {}", s.longest_no_spend, s.current_no_spend).into(),
        has_data: s.expense_count + s.income_count > 0,
    });

    let flow_max = s.buckets.iter().map(|b| b.expense.0).max().unwrap_or(0);
    let flow_peak = s
        .buckets
        .iter()
        .rposition(|b| b.expense.0 == flow_max && flow_max > 0);
    let n = s.buckets.len();
    ui.set_flow(model(
        s.buckets
            .iter()
            .enumerate()
            .map(|(i, b)| Bar {
                value: if flow_max > 0 {
                    b.expense.0 as f32 / flow_max as f32
                } else {
                    0.0
                },
                label: sparse_label(i, n, b.label.clone()),
                tip: if Some(i) == flow_peak {
                    b.expense.format_compact().into()
                } else {
                    SharedString::new()
                },
                highlight: Some(i) == flow_peak,
                empty: b.expense.0 == 0 || b.future,
            })
            .collect(),
    ));
    let hour_max = s.by_hour_count.iter().copied().max().unwrap_or(0);
    ui.set_hours(model(
        (0..24usize)
            .map(|h| {
                let n = s.by_hour_count[h];
                Bar {
                    value: if hour_max > 0 {
                        n as f32 / hour_max as f32
                    } else {
                        0.0
                    },
                    label: if h % 6 == 0 {
                        format!("{h:02}").into()
                    } else {
                        SharedString::new()
                    },
                    tip: if Some(h as u8) == s.peak_hour {
                        format!("{n}×").into()
                    } else {
                        SharedString::new()
                    },
                    highlight: Some(h as u8) == s.peak_hour,
                    empty: n == 0,
                }
            })
            .collect(),
    ));
    let heat_max = s.heatmap_max();
    let mut heat = Vec::with_capacity(7 * 24);
    let mut heat_peak = -1;
    for (d, row) in s.heatmap_count.iter().enumerate() {
        for (h, &cell) in row.iter().enumerate() {
            if heat_max > 0 && cell == heat_max && heat_peak < 0 {
                heat_peak = (d * 24 + h) as i32;
            }
            heat.push(if heat_max > 0 {
                cell as f32 / heat_max as f32
            } else {
                0.0
            });
        }
    }
    ui.set_heat(model(heat));
    ui.set_heat_peak(heat_peak);
    ui.set_week_labels(model(
        s.week_labels
            .iter()
            .map(|l| SharedString::from(*l))
            .collect(),
    ));
    let cat_top = s.categories.first().map(|c| c.total).unwrap_or_default();
    ui.set_stat_categories(model(
        s.categories
            .iter()
            .map(|c| {
                share_row(
                    c.category_id,
                    &c.name,
                    &c.icon,
                    c.total,
                    Some(c.count),
                    s.expense,
                    cat_top,
                )
            })
            .collect(),
    ));
    ui.set_largest(model(
        s.largest.iter().map(|t| tx_row(t, today, true)).collect(),
    ));
    ui.set_insights(model(
        s.insights
            .iter()
            .map(|i| Insight {
                icon: icons::path(i.icon).into(),
                text: i.text.clone().into(),
                warning: i.warning,
            })
            .collect(),
    ));

    try_refresh_activity(ui, ctx, today)?;
    Ok(())
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    let data_dir = app
        .internal_data_path()
        .unwrap_or_else(|| std::path::PathBuf::from("/data/local/tmp/income-radar"));
    slint::android::init(app).expect("Slint Android backend failed to start");
    match build(&data_dir) {
        Ok(ui) => {
            if let Err(e) = ui.run() {
                eprintln!("event loop failed: {e}");
            }
        }
        Err(e) => eprintln!("startup failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{edit_amount, plain_amount};
    use income_radar_core::Money;

    #[test]
    fn keypad_rules() {
        let type_keys = |keys: &[&str]| keys.iter().fold(String::new(), |s, k| edit_amount(&s, k));
        assert_eq!(type_keys(&["1", "2", ".", "5", "0", "9"]), "12.50");
        assert_eq!(type_keys(&[".", "5"]), "0.5");
        assert_eq!(type_keys(&["0", "0", "7"]), "7");
        assert_eq!(type_keys(&["1", ".", ".", "2"]), "1.2");
        assert_eq!(type_keys(&["1", "2", "<"]), "1");
        assert_eq!(type_keys(&["<"]), "");
        assert_eq!(type_keys(&["9"; 20]).len(), 12);
    }

    #[test]
    fn plain_amount_round_trips() {
        for m in [Money(0), Money(5), Money(1250), Money(125_075), Money(100)] {
            assert_eq!(Money::parse(&plain_amount(m)).unwrap(), m);
        }
        assert_eq!(plain_amount(Money(1250)), "12.50");
        assert_eq!(plain_amount(Money(1200)), "12");
    }
}
