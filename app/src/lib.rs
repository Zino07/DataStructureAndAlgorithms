//! Income Radar UI glue: wires the Slint interface to the core store.

slint::include_modules!();

pub mod demo;
mod icons;

use std::cell::Cell;
use std::error::Error;
use std::path::Path;
use std::rc::Rc;

use chrono::{Datelike, Local};
use income_radar_core::*;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

const ACTIVITY_LIMIT: usize = 300;

/// Today's local date. `INCOME_RADAR_TODAY=YYYY-MM-DD` pins it, which keeps
/// screenshots and demos reproducible.
pub fn today() -> NaiveDate {
    std::env::var("INCOME_RADAR_TODAY")
        .ok()
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| Local::now().date_naive())
}

type SettingsUpdate = Rc<dyn Fn(&dyn Fn(&mut Settings))>;

struct Ctx {
    store: Store,
    activity_filter: Cell<i32>,
}

/// Opens (or creates) the database in `data_dir` and builds the window.
pub fn build(data_dir: &Path) -> Result<AppWindow, Box<dyn Error>> {
    std::fs::create_dir_all(data_dir)?;
    let store = Store::open(data_dir.join("income-radar.db"))?;
    let ui = AppWindow::new()?;
    ui.set_version(env!("CARGO_PKG_VERSION").into());
    let ctx = Rc::new(Ctx {
        store,
        activity_filter: Cell::new(0),
    });
    wire(&ui, &ctx);
    refresh(&ui, &ctx);
    Ok(ui)
}

fn wire(ui: &AppWindow, ctx: &Rc<Ctx>) {
    let weak = ui.as_weak();

    ui.on_edit_amount(|current, key| edit_amount(&current, &key).into());

    ui.on_save_entry({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |income, amount, category, account, days_ago, note| {
            let result = save_entry(
                &ctx.store, income, &amount, category, account, days_ago, &note,
            );
            if result.is_ok() {
                if let Some(ui) = weak.upgrade() {
                    refresh(&ui, &ctx);
                }
            }
            result.err().unwrap_or_default().into()
        }
    });

    ui.on_delete_entry({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |id| {
            if let Err(e) = ctx.store.delete_transaction(id as Id) {
                eprintln!("delete failed: {e}");
            }
            if let Some(ui) = weak.upgrade() {
                refresh(&ui, &ctx);
            }
        }
    });

    ui.on_activity_filter_changed({
        let (ctx, weak) = (ctx.clone(), weak.clone());
        move |filter| {
            ctx.activity_filter.set(filter);
            if let Some(ui) = weak.upgrade() {
                refresh(&ui, &ctx);
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

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn save_entry(
    store: &Store,
    income: bool,
    amount: &str,
    category: i32,
    account: i32,
    days_ago: i32,
    note: &str,
) -> Result<(), String> {
    let amount = match Money::parse(amount) {
        Ok(m) if m.0 > 0 => m,
        Ok(_) | Err(ParseMoneyError::Empty) => return Err("Enter an amount".into()),
        Err(e) => return Err(capitalize(&e.to_string())),
    };
    if account < 0 {
        return Err("Add an account first".into());
    }
    let date = today() - chrono::Days::new(days_ago.clamp(0, 365) as u64);
    store
        .add_transaction(&NewTransaction {
            account_id: account as Id,
            category_id: category as Id,
            kind: if income { Kind::Income } else { Kind::Expense },
            amount,
            date,
            note: note.to_string(),
        })
        .map(|_| ())
        .map_err(|e| capitalize(&e.to_string()))
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
    if with_date {
        meta.push(day_label(t.date, today));
    }
    let sign = if t.kind == Kind::Income { "+" } else { "−" };
    TxRow {
        id: t.id as i32,
        header: false,
        title: title.into(),
        meta: meta.join(" · ").into(),
        amount: format!("{sign}{}", t.amount.format()).into(),
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

fn try_refresh(ui: &AppWindow, ctx: &Ctx) -> Result<(), StoreError> {
    let store = &ctx.store;
    let today = today();
    let settings = store.settings()?;

    let theme = ui.global::<Theme>();
    theme.set_palette(settings.palette as i32);
    theme.set_dark(settings.dark);
    ui.set_cycle_day(settings.cycle_start_day as i32);

    let radar = Radar::compute(store, today)?;
    let (income_delta, income_up) = delta(radar.income_change_pct());
    let (expense_delta, expense_up) = delta(radar.expense_change_pct());
    let week_total: Money = radar.last_7_days.iter().map(|(_, m)| *m).sum();
    let recent = store.transactions(None, 4)?;

    ui.set_dash(Dashboard {
        today: today.format("%a, %-d %b").to_string().into(),
        cycle: radar.period.label().into(),
        days_left: match radar.days_left {
            1 => "Last day".to_string(),
            n => format!("{n} days left"),
        }
        .into(),
        currency: settings.currency.clone().into(),
        balance: radar.total_balance.format().into(),
        safe_per_day: radar.safe_per_day.format().into(),
        income: radar.income.format_compact().into(),
        income_delta,
        income_up,
        expense: radar.expense.format_compact().into(),
        expense_delta,
        expense_up,
        spent_ratio: radar.spent_ratio(),
        projected: radar.projected_expense.format_compact().into(),
        savings_rate: radar
            .savings_rate()
            .map(|r| format!("{}%", (r * 100.0).round() as i64))
            .unwrap_or_else(|| "—".into())
            .into(),
        savings_ratio: radar.savings_rate().unwrap_or(0.0),
        week_total: week_total.format().into(),
        has_data: !recent.is_empty(),
    });
    ui.set_can_load_demo(recent.is_empty());

    // Last seven days: highlight the heaviest day.
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

    let filter = match ctx.activity_filter.get() {
        1 => Some(Kind::Expense),
        2 => Some(Kind::Income),
        _ => None,
    };
    let mut rows = Vec::new();
    let mut last_date = None;
    for t in store.transactions(filter, ACTIVITY_LIMIT)? {
        if last_date != Some(t.date) {
            last_date = Some(t.date);
            rows.push(TxRow {
                id: -1,
                header: true,
                title: day_label(t.date, today).into(),
                ..Default::default()
            });
        }
        rows.push(tx_row(&t, today, false));
    }
    ui.set_activity(model(rows));

    let top_max = radar.top_expenses.first().map(|c| c.total.0).unwrap_or(0);
    ui.set_top_categories(model(
        radar
            .top_expenses
            .iter()
            .take(4)
            .map(|c| CategoryShare {
                name: c.name.clone().into(),
                icon: icons::path(&c.icon).into(),
                amount: c.total.format().into(),
                share: if top_max > 0 {
                    c.total.0 as f32 / top_max as f32
                } else {
                    0.0
                },
                percent: if radar.expense.0 > 0 {
                    format!(
                        "{}%",
                        (c.total.0 * 100 + radar.expense.0 / 2) / radar.expense.0
                    )
                } else {
                    String::new()
                }
                .into(),
            })
            .collect(),
    ));

    let categories = |kind| -> Result<ModelRc<CategoryItem>, StoreError> {
        Ok(model(
            store
                .categories(kind)?
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
    use super::edit_amount;

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
}
