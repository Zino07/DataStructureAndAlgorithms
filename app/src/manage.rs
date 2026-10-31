//! Everything the user can change or remove: accounts, categories, repeating
//! entries, the PIN, backups, exports and erasing. Wired to the UI here so
//! `lib.rs` stays about showing data.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::*;

/// `date` moved `days_ago` days into the past (negative: into the future).
pub(super) fn shift_days(date: NaiveDate, days_ago: i32) -> NaiveDate {
    date - chrono::Duration::days(days_ago as i64)
}

fn fail(e: impl std::fmt::Display) -> SharedString {
    capitalize(&e.to_string()).into()
}

fn parse_opening(text: &str) -> Result<Money, String> {
    if text.trim().is_empty() {
        return Ok(Money::ZERO);
    }
    Money::parse(text).map_err(|_| "Opening balance is not a valid amount".to_string())
}

pub(super) fn calc_preview(text: &str) -> String {
    if !calc::has_operator(text) {
        return String::new();
    }
    match calc::eval(text) {
        Ok(m) => m.format(),
        Err(_) => "—".into(),
    }
}

/// Rows for the calendar: the month `offset` months away from today's.
pub(super) fn month_grid(offset: i32, week_starts_sunday: bool) -> Vec<CalDay> {
    let today = today();
    let first = today.with_day(1).unwrap_or(today);
    let first = if offset >= 0 {
        first + chrono::Months::new(offset as u32)
    } else {
        first - chrono::Months::new((-offset) as u32)
    };
    let lead = if week_starts_sunday {
        first.weekday().num_days_from_sunday()
    } else {
        first.weekday().num_days_from_monday()
    } as usize;
    let days = (first + chrono::Months::new(1) - first).num_days() as usize;
    let mut cells = vec![
        CalDay {
            day: 0,
            days_ago: 0,
            today: false
        };
        lead
    ];
    for d in 0..days {
        let date = first + chrono::Days::new(d as u64);
        cells.push(CalDay {
            day: d as i32 + 1,
            days_ago: (today - date).num_days() as i32,
            today: date == today,
        });
    }
    while !cells.len().is_multiple_of(7) {
        cells.push(CalDay {
            day: 0,
            days_ago: 0,
            today: false,
        });
    }
    cells
}

pub(super) fn week_heads(week_starts_sunday: bool) -> Vec<SharedString> {
    let sun = ["S", "M", "T", "W", "T", "F", "S"];
    let mon = ["M", "T", "W", "T", "F", "S", "S"];
    (if week_starts_sunday { sun } else { mon })
        .iter()
        .map(|s| SharedString::from(*s))
        .collect()
}

pub(super) fn cat_rows(store: &Store, kind: Kind) -> Result<Vec<CatRow>, StoreError> {
    let infos = store.category_infos(kind)?;
    let n = infos.len();
    Ok(infos
        .into_iter()
        .enumerate()
        .map(|(i, c)| CatRow {
            id: c.category.id as i32,
            name: c.category.name.into(),
            icon: icons::path(&c.category.icon).into(),
            icon_key: c.category.icon.into(),
            uses: c.uses as i32,
            hidden: c.hidden,
            work: c.work,
            income: kind == Kind::Income,
            has_budget: c.has_budget,
            first: i == 0,
            last: i + 1 == n,
        })
        .collect())
}

pub(super) fn repeat_rows(store: &Store) -> Result<Vec<RepeatRow>, StoreError> {
    Ok(store
        .recurring_list()?
        .into_iter()
        .map(|r| RepeatRow {
            id: r.id as i32,
            title: if r.note.is_empty() {
                r.category_name.clone()
            } else {
                r.note.clone()
            }
            .into(),
            meta: format!(
                "{} · {} · next {}{}",
                r.repeat.label(),
                r.account_name,
                r.next_date.format("%-d %b"),
                if r.active { "" } else { " · paused" }
            )
            .into(),
            amount: format!(
                "{}{}",
                if r.kind == Kind::Income { "+" } else { "−" },
                r.amount.format()
            )
            .into(),
            icon: icons::path(&r.category_icon).into(),
            income: r.kind == Kind::Income,
            active: r.active,
        })
        .collect())
}

pub(super) fn icon_choices() -> Vec<IconChoice> {
    icons::CHOICES
        .iter()
        .map(|k| IconChoice {
            key: (*k).into(),
            path: icons::path(k).into(),
        })
        .collect()
}

/// Newest backup file in the export folder.
fn latest_backup(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("income-radar-backup") && name.ends_with(".db")
        })
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.path())
}

pub(super) fn wire_manage(ui: &AppWindow, ctx: &Rc<Ctx>) {
    let weak = ui.as_weak();

    // ----- calendar and calculator -----
    ui.on_month_title({
        move |offset| {
            let today = today();
            let first = today.with_day(1).unwrap_or(today);
            let d = if offset >= 0 {
                first + chrono::Months::new(offset as u32)
            } else {
                first - chrono::Months::new((-offset) as u32)
            };
            d.format("%B %Y").to_string().into()
        }
    });
    ui.on_month_grid({
        let ctx = ctx.clone();
        move |offset| {
            let sunday = ctx
                .store
                .settings()
                .map(|s| s.weekend == Weekend::FriSat)
                .unwrap_or(true);
            model(month_grid(offset, sunday))
        }
    });
    ui.on_calc_preview(|text| calc_preview(&text).into());
    ui.on_pin_key(|current, key| pin_key(&current, &key).into());

    // ----- editing a transfer -----
    ui.on_request_edit_transfer({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id| {
            let Some(ui) = weak.upgrade() else { return };
            match ctx.store.transfer(id as Id) {
                Ok(Some(t)) => ui.invoke_load_entry(
                    t.id as i32,
                    2,
                    t.amount.plain().into(),
                    -1,
                    t.from_account as i32,
                    t.to_account as i32,
                    (today() - t.date).num_days() as i32,
                    t.minute as i32,
                    t.note.into(),
                ),
                _ => toast(&ui, &ctx, "That transfer no longer exists", None),
            }
        }
    });

    // ----- accounts -----
    ui.on_update_account({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id, name, kind, opening| {
            let kind = AccountKind::ALL
                .get(kind as usize)
                .copied()
                .unwrap_or(AccountKind::Cash);
            let result = parse_opening(&opening).and_then(|o| {
                ctx.store
                    .update_account(id as Id, &name, kind, o)
                    .map_err(|e| capitalize(&e.to_string()))
            });
            match result {
                Ok(()) => {
                    if let Some(ui) = weak.upgrade() {
                        refresh(&ui, &ctx);
                        toast(&ui, &ctx, "Account updated", None);
                    }
                    SharedString::new()
                }
                Err(e) => e.into(),
            }
        }
    });
    ui.on_delete_account({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id, move_to| {
            let fate = if move_to >= 0 {
                Fate::MoveTo(move_to as Id)
            } else {
                Fate::DeleteAll
            };
            match ctx.store.delete_account(id as Id, fate) {
                Ok(()) => {
                    if let Some(ui) = weak.upgrade() {
                        refresh(&ui, &ctx);
                        toast(
                            &ui,
                            &ctx,
                            if move_to >= 0 {
                                "Account merged"
                            } else {
                                "Account deleted"
                            },
                            None,
                        );
                    }
                    SharedString::new()
                }
                Err(e) => fail(e),
            }
        }
    });

    // ----- categories -----
    let done = |ctx: &Rc<Ctx>, weak: &slint::Weak<AppWindow>, text: &str| {
        if let Some(ui) = weak.upgrade() {
            refresh(&ui, ctx);
            toast(&ui, ctx, text, None);
        }
    };
    ui.on_add_category({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |name, kind, icon, work| {
            let kind = if kind == 1 {
                Kind::Income
            } else {
                Kind::Expense
            };
            match ctx.store.add_category(&name, kind, &icon).and_then(|id| {
                if kind == Kind::Income && !work {
                    ctx.store.update_category(id, &name, &icon, false)
                } else {
                    Ok(())
                }
            }) {
                Ok(()) => {
                    done(&ctx, &weak, "Category added");
                    SharedString::new()
                }
                Err(e) => fail(e),
            }
        }
    });
    ui.on_update_category({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id, name, icon, work| match ctx.store.update_category(id as Id, &name, &icon, work) {
            Ok(()) => {
                done(&ctx, &weak, "Category saved");
                SharedString::new()
            }
            Err(e) => fail(e),
        }
    });
    ui.on_hide_category({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id, hidden| match ctx.store.set_category_hidden(id as Id, hidden) {
            Ok(()) => {
                done(
                    &ctx,
                    &weak,
                    if hidden {
                        "Category hidden"
                    } else {
                        "Category shown again"
                    },
                );
                SharedString::new()
            }
            Err(e) => fail(e),
        }
    });
    ui.on_move_category({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id, up| {
            if ctx.store.move_category(id as Id, up).is_ok() {
                if let Some(ui) = weak.upgrade() {
                    refresh(&ui, &ctx);
                }
            }
        }
    });
    ui.on_remove_category({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id, move_to| {
            let to = (move_to >= 0).then_some(move_to as Id);
            match ctx.store.delete_category(id as Id, to) {
                Ok(()) => {
                    done(&ctx, &weak, "Category deleted");
                    SharedString::new()
                }
                Err(e) => fail(e),
            }
        }
    });

    // ----- repeating entries -----
    ui.on_toggle_repeat({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id, on| {
            if ctx.store.set_recurring_active(id as Id, on).is_ok() {
                if let Some(ui) = weak.upgrade() {
                    refresh(&ui, &ctx);
                }
            }
        }
    });
    ui.on_remove_repeat({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id| {
            if ctx.store.delete_recurring(id as Id).is_ok() {
                done(&ctx, &weak, "Repeating entry deleted");
            }
        }
    });

    // ----- working hours -----
    ui.on_work_hours_changed({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |h| {
            let mut s = ctx.store.settings().unwrap_or_default();
            s.work_hours = h.clamp(1, 16) as u32;
            if ctx.store.save_settings(&s).is_ok() {
                if let Some(ui) = weak.upgrade() {
                    refresh(&ui, &ctx);
                }
            }
        }
    });

    // ----- app lock -----
    ui.on_unlock({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |pin| {
            if let Some(until) = ctx.locked_until.get() {
                let now = Instant::now();
                if now < until {
                    return format!("Too many tries. Wait {} s", (until - now).as_secs() + 1)
                        .into();
                }
                ctx.locked_until.set(None);
                ctx.wrong_pins.set(PIN_TRIES - 1);
            }
            match ctx.store.verify_pin(&pin) {
                Ok(true) => {
                    ctx.wrong_pins.set(0);
                    if let Some(ui) = weak.upgrade() {
                        ui.set_locked(false);
                        refresh(&ui, &ctx);
                    }
                    SharedString::new()
                }
                Ok(false) => {
                    let wrong = ctx.wrong_pins.get() + 1;
                    ctx.wrong_pins.set(wrong);
                    if wrong >= PIN_TRIES {
                        let wait = 30u64 << (wrong - PIN_TRIES).min(6);
                        ctx.locked_until
                            .set(Some(Instant::now() + Duration::from_secs(wait)));
                        format!("Too many tries. Wait {wait} s").into()
                    } else {
                        format!("Wrong PIN · {} tries left", PIN_TRIES - wrong).into()
                    }
                }
                Err(_) => "Could not check the PIN".into(),
            }
        }
    });
    ui.on_forgot_pin({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move || {
            if ctx.store.erase_all().is_ok() {
                ctx.wrong_pins.set(0);
                ctx.locked_until.set(None);
                if let Some(ui) = weak.upgrade() {
                    ui.set_locked(false);
                    refresh(&ui, &ctx);
                    toast(&ui, &ctx, "Everything was erased", None);
                }
            }
        }
    });
    ui.on_window_active({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |active| {
            if !active {
                ctx.inactive_since.set(Some(Instant::now()));
            } else if let Some(since) = ctx.inactive_since.take() {
                if since.elapsed() >= Duration::from_secs(RELOCK_SECS)
                    && ctx.store.has_pin().unwrap_or(false)
                {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_locked(true);
                    }
                }
            }
        }
    });
    ui.on_set_pin({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |current, new, again| {
            if ctx.store.has_pin().unwrap_or(false)
                && !ctx.store.verify_pin(&current).unwrap_or(false)
            {
                return "The current PIN is wrong".into();
            }
            if new != again {
                return "The two PINs do not match".into();
            }
            match ctx.store.set_pin(Some(&new)) {
                Ok(()) => {
                    done(&ctx, &weak, "PIN saved");
                    SharedString::new()
                }
                Err(e) => fail(e),
            }
        }
    });
    ui.on_remove_pin({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |current| {
            if !ctx.store.verify_pin(&current).unwrap_or(false) {
                return "The current PIN is wrong".into();
            }
            match ctx.store.set_pin(None) {
                Ok(()) => {
                    done(&ctx, &weak, "PIN removed");
                    SharedString::new()
                }
                Err(e) => fail(e),
            }
        }
    });

    // ----- your data -----
    let message = |weak: &slint::Weak<AppWindow>, text: String| {
        if let Some(ui) = weak.upgrade() {
            ui.set_data_message(text.into());
        }
    };
    ui.on_backup_now({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move || {
            let name = format!("income-radar-backup-{}.db", today().format("%Y-%m-%d"));
            let result = std::fs::create_dir_all(&ctx.export_dir)
                .map_err(|e| e.to_string())
                .and_then(|_| {
                    let path = ctx.export_dir.join(&name);
                    ctx.store
                        .backup_to(&path)
                        .map(|_| path)
                        .map_err(|e| e.to_string())
                });
            message(
                &weak,
                match result {
                    Ok(path) => format!("Backup saved to {}", path.display()),
                    Err(e) => format!("Backup failed: {e}"),
                },
            );
        }
    });
    ui.on_export_csv({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move || {
            let name = format!("income-radar-entries-{}.csv", today().format("%Y-%m-%d"));
            let result = std::fs::create_dir_all(&ctx.export_dir)
                .map_err(|e| e.to_string())
                .and_then(|_| {
                    let path = ctx.export_dir.join(&name);
                    let csv = ctx.store.export_csv().map_err(|e| e.to_string())?;
                    std::fs::write(&path, csv).map_err(|e| e.to_string())?;
                    Ok(path)
                });
            message(
                &weak,
                match result {
                    Ok(path) => format!("Entries saved to {}", path.display()),
                    Err(e) => format!("Export failed: {e}"),
                },
            );
        }
    });
    ui.on_restore_backup({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move || {
            let Some(file) = latest_backup(&ctx.export_dir) else {
                message(
                    &weak,
                    format!(
                        "No backup found in {}. Copy a backup file there first.",
                        ctx.export_dir.display()
                    ),
                );
                return;
            };
            match ctx.store.restore_from(&file) {
                Ok(()) => {
                    if let Some(ui) = weak.upgrade() {
                        refresh(&ui, &ctx);
                        ui.set_data_message(
                            format!(
                                "Restored from {}",
                                file.file_name().unwrap_or_default().to_string_lossy()
                            )
                            .into(),
                        );
                    }
                }
                Err(e) => message(
                    &weak,
                    format!("Restore failed: {}", capitalize(&e.to_string())),
                ),
            }
        }
    });
    ui.on_remove_demo({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move || match ctx.store.remove_demo() {
            Ok(()) => {
                done(&ctx, &weak, "Sample data removed");
                message(&weak, String::new());
            }
            Err(e) => message(&weak, format!("Could not remove: {e}")),
        }
    });
    ui.on_erase_all({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move || match ctx.store.erase_all() {
            Ok(()) => {
                done(&ctx, &weak, "Everything was erased");
                message(&weak, String::new());
            }
            Err(e) => message(&weak, format!("Could not erase: {e}")),
        }
    });
}

/// PIN pad editing: digits only, at most eight, `<` deletes one.
pub(super) fn pin_key(current: &str, key: &str) -> String {
    let mut s = current.to_string();
    match key {
        "<" => {
            s.pop();
        }
        d if d.len() == 1 && d.as_bytes()[0].is_ascii_digit() && s.len() < 8 => s.push_str(d),
        _ => {}
    }
    s
}
