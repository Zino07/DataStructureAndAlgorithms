//! SQLite persistence. Balances are never stored: they are derived from the
//! opening amount plus the ledger, so editing or deleting a transaction can
//! never leave an account out of sync.

use std::collections::HashMap;
use std::path::Path;

use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::model::*;
use crate::money::Money;
use crate::period::Period;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug)]
pub enum StoreError {
    Sql(rusqlite::Error),
    Invalid(&'static str),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Sql(e) => write!(f, "database error: {e}"),
            StoreError::Invalid(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sql(e)
    }
}

const DATE_FMT: &str = "%Y-%m-%d";

/// Longest note kept, in characters.
pub const MAX_NOTE_CHARS: usize = 120;
/// Longest account or category name, in characters.
pub const MAX_NAME_CHARS: usize = 40;
/// Largest single amount accepted: 1 billion in major units.
pub const MAX_AMOUNT: Money = Money(100_000_000_000);

fn date_to_db(d: NaiveDate) -> String {
    d.format(DATE_FMT).to_string()
}

fn date_from_db(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, DATE_FMT).unwrap_or_default()
}

fn clean_text(s: &str, max: usize) -> String {
    let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(max).collect()
}

const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE account (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        kind INTEGER NOT NULL,
        opening_minor INTEGER NOT NULL DEFAULT 0,
        sort INTEGER NOT NULL DEFAULT 0,
        archived INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE category (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        kind INTEGER NOT NULL,
        icon TEXT NOT NULL,
        sort INTEGER NOT NULL DEFAULT 0,
        archived INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE txn (
        id INTEGER PRIMARY KEY,
        account_id INTEGER NOT NULL REFERENCES account(id),
        category_id INTEGER NOT NULL REFERENCES category(id),
        kind INTEGER NOT NULL,
        amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
        date TEXT NOT NULL,
        note TEXT NOT NULL DEFAULT ''
    );
    CREATE INDEX txn_date ON txn(date);
    CREATE INDEX txn_account ON txn(account_id);
    CREATE INDEX txn_category ON txn(category_id);
    CREATE TABLE transfer (
        id INTEGER PRIMARY KEY,
        from_account INTEGER NOT NULL REFERENCES account(id),
        to_account INTEGER NOT NULL REFERENCES account(id),
        amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
        date TEXT NOT NULL,
        note TEXT NOT NULL DEFAULT '',
        CHECK (from_account <> to_account)
    );
    CREATE TABLE budget (
        category_id INTEGER PRIMARY KEY REFERENCES category(id),
        limit_minor INTEGER NOT NULL CHECK (limit_minor > 0)
    );
    CREATE TABLE setting (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
    // v2: time of day for statistics by hour and minute
    "ALTER TABLE txn ADD COLUMN minute INTEGER NOT NULL DEFAULT 720
        CHECK (minute BETWEEN 0 AND 1439);
    ALTER TABLE transfer ADD COLUMN minute INTEGER NOT NULL DEFAULT 720
        CHECK (minute BETWEEN 0 AND 1439);
    CREATE INDEX txn_kind_date ON txn(kind, date, minute);
    CREATE INDEX transfer_date ON transfer(date);",
];

const DEFAULT_EXPENSE_CATEGORIES: &[(&str, &str)] = &[
    ("Food", "food"),
    ("Groceries", "cart"),
    ("Transport", "car"),
    ("Home", "home"),
    ("Bills", "bolt"),
    ("Shopping", "bag"),
    ("Health", "heart"),
    ("Fun", "play"),
    ("Education", "book"),
    ("Family", "users"),
    ("Other", "dots"),
];

const DEFAULT_INCOME_CATEGORIES: &[(&str, &str)] = &[
    ("Salary", "briefcase"),
    ("Freelance", "laptop"),
    ("Investments", "trend"),
    ("Gifts", "gift"),
    ("Other income", "dots"),
];

const TX_SELECT: &str = "SELECT t.id, t.account_id, a.name, t.category_id, c.name, c.icon,
        t.kind, t.amount_minor, t.date, t.minute, t.note
    FROM txn t
    JOIN account a ON a.id = t.account_id
    JOIN category c ON c.id = t.category_id";

fn tx_from_row(r: &Row<'_>) -> rusqlite::Result<Transaction> {
    Ok(Transaction {
        id: r.get(0)?,
        account_id: r.get(1)?,
        account_name: r.get(2)?,
        category_id: r.get(3)?,
        category_name: r.get(4)?,
        category_icon: r.get(5)?,
        kind: Kind::from_db(r.get(6)?),
        amount: Money(r.get(7)?),
        date: date_from_db(&r.get::<_, String>(8)?),
        minute: r.get::<_, i64>(9)?.clamp(0, 1439) as u16,
        note: r.get(10)?,
    })
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Store> {
        Store::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Store> {
        Store::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;
        let mut store = Store { conn };
        store.migrate()?;
        store.seed()?;
        Ok(store)
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    fn migrate(&mut self) -> Result<()> {
        let version = self.schema_version()? as usize;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        Ok(())
    }

    /// First-run content: a cash account and a starter set of categories.
    fn seed(&mut self) -> Result<()> {
        if self.setting("seeded")?.is_some() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO account (name, kind, opening_minor, sort) VALUES (?1, ?2, 0, 0)",
            params!["Cash", AccountKind::Cash.to_db()],
        )?;
        for (kind, list) in [
            (Kind::Expense, DEFAULT_EXPENSE_CATEGORIES),
            (Kind::Income, DEFAULT_INCOME_CATEGORIES),
        ] {
            for (sort, (name, icon)) in list.iter().enumerate() {
                tx.execute(
                    "INSERT INTO category (name, kind, icon, sort) VALUES (?1, ?2, ?3, ?4)",
                    params![name, kind.to_db(), icon, sort as i64],
                )?;
            }
        }
        tx.execute(
            "INSERT INTO setting (key, value) VALUES ('seeded', '1')",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    // ----- settings -------------------------------------------------------

    fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM setting WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO setting (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn settings(&self) -> Result<Settings> {
        let d = Settings::default();
        let num = |key: &str, default: u32| -> Result<u32> {
            Ok(self
                .setting(key)?
                .and_then(|v| v.parse().ok())
                .unwrap_or(default))
        };
        Ok(Settings {
            currency: self.setting("currency")?.unwrap_or(d.currency),
            cycle_start_day: num("cycle_start_day", d.cycle_start_day)?
                .clamp(1, crate::period::MAX_CYCLE_START_DAY),
            palette: num("palette", d.palette)?.min(3),
            dark: match self.setting("dark")?.as_deref() {
                Some(v) => v == "1",
                None => d.dark,
            },
            weekend: match self.setting("weekend")?.as_deref() {
                Some("1") => Weekend::SatSun,
                Some(_) => Weekend::FriSat,
                None => d.weekend,
            },
            style: num("style", d.style)?.min(1),
            reduce_motion: self.setting("reduce_motion")?.as_deref() == Some("1"),
            solid_glass: self.setting("solid_glass")?.as_deref() == Some("1"),
        })
    }

    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        let currency = s.currency.trim();
        if currency.is_empty() || currency.chars().count() > 8 {
            return Err(StoreError::Invalid("currency code is not valid"));
        }
        let tx = self.conn.unchecked_transaction()?;
        self.set_setting("currency", currency)?;
        self.set_setting(
            "cycle_start_day",
            &s.cycle_start_day
                .clamp(1, crate::period::MAX_CYCLE_START_DAY)
                .to_string(),
        )?;
        self.set_setting("palette", &s.palette.min(3).to_string())?;
        self.set_setting("dark", if s.dark { "1" } else { "0" })?;
        self.set_setting("style", &s.style.min(1).to_string())?;
        self.set_setting("reduce_motion", if s.reduce_motion { "1" } else { "0" })?;
        self.set_setting("solid_glass", if s.solid_glass { "1" } else { "0" })?;
        self.set_setting(
            "weekend",
            if s.weekend == Weekend::SatSun {
                "1"
            } else {
                "0"
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    // ----- accounts -------------------------------------------------------

    pub fn add_account(&self, name: &str, kind: AccountKind, opening: Money) -> Result<Id> {
        let name = clean_text(name, MAX_NAME_CHARS);
        if name.is_empty() {
            return Err(StoreError::Invalid("account name is required"));
        }
        if opening.abs() > MAX_AMOUNT {
            return Err(StoreError::Invalid("opening balance is too large"));
        }
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM account WHERE archived = 0 AND lower(name) = lower(?1))",
            [&name],
            |r| r.get(0),
        )?;
        if exists {
            return Err(StoreError::Invalid(
                "an account with this name already exists",
            ));
        }
        self.conn.execute(
            "INSERT INTO account (name, kind, opening_minor, sort)
             VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(sort), -1) + 1 FROM account))",
            params![name, kind.to_db(), opening.0],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Hides an account. Its history stays, so past statistics do not change.
    pub fn archive_account(&self, id: Id) -> Result<()> {
        let active: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM account WHERE archived = 0", [], |r| {
                    r.get(0)
                })?;
        if active <= 1 {
            return Err(StoreError::Invalid("keep at least one account"));
        }
        self.conn
            .execute("UPDATE account SET archived = 1 WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn accounts_with_balances(&self) -> Result<Vec<AccountBalance>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT a.id, a.name, a.kind, a.opening_minor,
                a.opening_minor
                + COALESCE((SELECT SUM(CASE WHEN t.kind = 1 THEN t.amount_minor ELSE -t.amount_minor END)
                            FROM txn t WHERE t.account_id = a.id), 0)
                + COALESCE((SELECT SUM(amount_minor) FROM transfer WHERE to_account = a.id), 0)
                - COALESCE((SELECT SUM(amount_minor) FROM transfer WHERE from_account = a.id), 0)
             FROM account a
             WHERE a.archived = 0
             ORDER BY a.sort, a.id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(AccountBalance {
                account: Account {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    kind: AccountKind::from_db(r.get(2)?),
                    opening: Money(r.get(3)?),
                },
                balance: Money(r.get(4)?),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn total_balance(&self) -> Result<Money> {
        Ok(self
            .accounts_with_balances()?
            .iter()
            .map(|a| a.balance)
            .sum())
    }

    fn account_is_active(&self, id: Id) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM account WHERE id = ?1 AND archived = 0)",
            [id],
            |r| r.get(0),
        )?)
    }

    // ----- categories -----------------------------------------------------

    pub fn categories(&self, kind: Kind) -> Result<Vec<Category>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, name, kind, icon FROM category
             WHERE archived = 0 AND kind = ?1 ORDER BY sort, id",
        )?;
        let rows = stmt.query_map([kind.to_db()], |r| {
            Ok(Category {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: Kind::from_db(r.get(2)?),
                icon: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Categories ordered by how often they were used in the last 60 days,
    /// so the most likely choice is the first chip on screen.
    pub fn categories_by_use(&self, kind: Kind, today: NaiveDate) -> Result<Vec<Category>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT c.id, c.name, c.kind, c.icon,
                    (SELECT COUNT(*) FROM txn t WHERE t.category_id = c.id AND t.date >= ?2) AS uses
             FROM category c
             WHERE c.archived = 0 AND c.kind = ?1
             ORDER BY uses DESC, c.sort, c.id",
        )?;
        let since = date_to_db(today - chrono::Days::new(60));
        let rows = stmt.query_map(params![kind.to_db(), since], |r| {
            Ok(Category {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: Kind::from_db(r.get(2)?),
                icon: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn add_category(&self, name: &str, kind: Kind, icon: &str) -> Result<Id> {
        let name = clean_text(name, MAX_NAME_CHARS);
        if name.is_empty() {
            return Err(StoreError::Invalid("category name is required"));
        }
        self.conn.execute(
            "INSERT INTO category (name, kind, icon, sort)
             VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(sort), -1) + 1 FROM category WHERE kind = ?2))",
            params![name, kind.to_db(), icon],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    // ----- transactions ---------------------------------------------------

    fn validate(&self, t: &NewTransaction) -> Result<()> {
        if t.amount.0 <= 0 {
            return Err(StoreError::Invalid("amount must be greater than zero"));
        }
        if t.amount > MAX_AMOUNT {
            return Err(StoreError::Invalid("amount is too large"));
        }
        if t.minute > 1439 {
            return Err(StoreError::Invalid("time of day is not valid"));
        }
        let category_kind: Option<i64> = self
            .conn
            .query_row(
                "SELECT kind FROM category WHERE id = ?1 AND archived = 0",
                [t.category_id],
                |r| r.get(0),
            )
            .optional()?;
        match category_kind {
            None => return Err(StoreError::Invalid("choose a category")),
            Some(k) if Kind::from_db(k) != t.kind => {
                return Err(StoreError::Invalid(
                    "category does not match the entry type",
                ))
            }
            Some(_) => {}
        }
        if !self.account_is_active(t.account_id)? {
            return Err(StoreError::Invalid("choose an account"));
        }
        Ok(())
    }

    pub fn add_transaction(&self, t: &NewTransaction) -> Result<Id> {
        self.validate(t)?;
        self.conn.execute(
            "INSERT INTO txn (account_id, category_id, kind, amount_minor, date, minute, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                t.account_id,
                t.category_id,
                t.kind.to_db(),
                t.amount.0,
                date_to_db(t.date),
                t.minute,
                clean_text(&t.note, MAX_NOTE_CHARS)
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_transaction(&self, id: Id, t: &NewTransaction) -> Result<()> {
        self.validate(t)?;
        let changed = self.conn.execute(
            "UPDATE txn SET account_id = ?2, category_id = ?3, kind = ?4, amount_minor = ?5,
                    date = ?6, minute = ?7, note = ?8
             WHERE id = ?1",
            params![
                id,
                t.account_id,
                t.category_id,
                t.kind.to_db(),
                t.amount.0,
                date_to_db(t.date),
                t.minute,
                clean_text(&t.note, MAX_NOTE_CHARS)
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::Invalid("this entry no longer exists"));
        }
        Ok(())
    }

    pub fn transaction(&self, id: Id) -> Result<Option<Transaction>> {
        let mut stmt = self
            .conn
            .prepare_cached(&format!("{TX_SELECT} WHERE t.id = ?1"))?;
        Ok(stmt.query_row([id], tx_from_row).optional()?)
    }

    pub fn delete_transaction(&self, id: Id) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM txn WHERE id = ?1", [id])? > 0)
    }

    /// Newest first. `kind = None` returns both directions. `query` matches
    /// the note, category or account name, case-insensitively.
    pub fn search(
        &self,
        kind: Option<Kind>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Transaction>> {
        let q = query.trim();
        let pattern = if q.is_empty() {
            None
        } else {
            let escaped = q
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            Some(format!("%{escaped}%"))
        };
        let mut stmt = self.conn.prepare_cached(&format!(
            "{TX_SELECT}
             WHERE (?1 IS NULL OR t.kind = ?1)
               AND (?2 IS NULL OR t.note LIKE ?2 ESCAPE '\\' OR c.name LIKE ?2 ESCAPE '\\'
                    OR a.name LIKE ?2 ESCAPE '\\')
             ORDER BY t.date DESC, t.minute DESC, t.id DESC
             LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![kind.map(Kind::to_db), pattern, limit as i64],
            tx_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn transactions(&self, kind: Option<Kind>, limit: usize) -> Result<Vec<Transaction>> {
        self.search(kind, "", limit)
    }

    /// Every entry with a date in `[period.start, period.end)`, oldest first.
    pub fn transactions_in(&self, period: Period) -> Result<Vec<Transaction>> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "{TX_SELECT} WHERE t.date >= ?1 AND t.date < ?2
             ORDER BY t.date, t.minute, t.id"
        ))?;
        let rows = stmt.query_map(
            params![date_to_db(period.start), date_to_db(period.end)],
            tx_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn first_entry_date(&self) -> Result<Option<NaiveDate>> {
        let d: Option<String> = self
            .conn
            .query_row("SELECT MIN(date) FROM txn", [], |r| r.get(0))?;
        Ok(d.map(|s| date_from_db(&s)))
    }

    pub fn entry_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM txn", [], |r| r.get(0))?)
    }

    // ----- transfers ------------------------------------------------------

    pub fn add_transfer(&self, t: &NewTransfer) -> Result<Id> {
        if t.amount.0 <= 0 {
            return Err(StoreError::Invalid("amount must be greater than zero"));
        }
        if t.amount > MAX_AMOUNT {
            return Err(StoreError::Invalid("amount is too large"));
        }
        if t.from_account == t.to_account {
            return Err(StoreError::Invalid("choose two different accounts"));
        }
        if !self.account_is_active(t.from_account)? || !self.account_is_active(t.to_account)? {
            return Err(StoreError::Invalid("choose an account"));
        }
        self.conn.execute(
            "INSERT INTO transfer (from_account, to_account, amount_minor, date, minute, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                t.from_account,
                t.to_account,
                t.amount.0,
                date_to_db(t.date),
                t.minute.min(1439),
                clean_text(&t.note, MAX_NOTE_CHARS)
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_transfer(&self, id: Id) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM transfer WHERE id = ?1", [id])?
            > 0)
    }

    pub fn transfers(&self, limit: usize) -> Result<Vec<Transfer>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT tr.id, tr.from_account, fa.name, tr.to_account, ta.name,
                    tr.amount_minor, tr.date, tr.minute, tr.note
             FROM transfer tr
             JOIN account fa ON fa.id = tr.from_account
             JOIN account ta ON ta.id = tr.to_account
             ORDER BY tr.date DESC, tr.minute DESC, tr.id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |r| {
            Ok(Transfer {
                id: r.get(0)?,
                from_account: r.get(1)?,
                from_name: r.get(2)?,
                to_account: r.get(3)?,
                to_name: r.get(4)?,
                amount: Money(r.get(5)?),
                date: date_from_db(&r.get::<_, String>(6)?),
                minute: r.get::<_, i64>(7)?.clamp(0, 1439) as u16,
                note: r.get(8)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ----- aggregates -----------------------------------------------------

    /// `(income, expense)` for dates in `[period.start, period.end)`.
    pub fn totals(&self, period: Period) -> Result<(Money, Money)> {
        let (income, expense) = self.conn.query_row(
            "SELECT COALESCE(SUM(CASE WHEN kind = 1 THEN amount_minor END), 0),
                    COALESCE(SUM(CASE WHEN kind = 0 THEN amount_minor END), 0)
             FROM txn WHERE date >= ?1 AND date < ?2",
            params![date_to_db(period.start), date_to_db(period.end)],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )?;
        Ok((Money(income), Money(expense)))
    }

    /// Expense total for each day in `[from, to]`, zero-filled, oldest first.
    pub fn daily_expenses(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<(NaiveDate, Money)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT date, SUM(amount_minor) FROM txn
             WHERE kind = 0 AND date >= ?1 AND date <= ?2
             GROUP BY date",
        )?;
        let found: HashMap<NaiveDate, Money> = stmt
            .query_map(params![date_to_db(from), date_to_db(to)], |r| {
                Ok((date_from_db(&r.get::<_, String>(0)?), Money(r.get(1)?)))
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(from
            .iter_days()
            .take_while(|d| *d <= to)
            .map(|d| (d, found.get(&d).copied().unwrap_or_default()))
            .collect())
    }

    /// Per-category totals for one direction in a period, largest first.
    pub fn category_totals(&self, kind: Kind, period: Period) -> Result<Vec<CategoryTotal>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT c.id, c.name, c.icon, SUM(t.amount_minor) AS total
             FROM txn t JOIN category c ON c.id = t.category_id
             WHERE t.kind = ?1 AND t.date >= ?2 AND t.date < ?3
             GROUP BY c.id
             ORDER BY total DESC, c.sort",
        )?;
        let rows = stmt.query_map(
            params![
                kind.to_db(),
                date_to_db(period.start),
                date_to_db(period.end)
            ],
            |r| {
                Ok(CategoryTotal {
                    category_id: r.get(0)?,
                    name: r.get(1)?,
                    icon: r.get(2)?,
                    total: Money(r.get(3)?),
                })
            },
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ----- suggestions ----------------------------------------------------

    /// Entries the user makes often around this time of day. Entries are
    /// grouped by category, account and note (amounts vary from day to day);
    /// each use counts more the closer it was to `minute` on the clock, so a
    /// monthly rent paid at 10:30 is not offered at lunch time.
    /// Looks at the last 60 days.
    pub fn suggestions(
        &self,
        kind: Kind,
        today: NaiveDate,
        minute: u16,
        limit: usize,
    ) -> Result<Vec<Suggestion>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT t.category_id, c.name, c.icon, t.account_id, t.amount_minor, t.note, t.minute
             FROM txn t
             JOIN category c ON c.id = t.category_id AND c.archived = 0
             JOIN account a ON a.id = t.account_id AND a.archived = 0
             WHERE t.kind = ?1 AND t.date >= ?2
             ORDER BY t.date DESC, t.minute DESC
             LIMIT 600",
        )?;
        let since = date_to_db(today - chrono::Days::new(60));
        struct Group {
            suggestion: Suggestion,
            score: f64,
            amounts: HashMap<i64, u32>,
        }
        type Key = (Id, Id, String);
        let mut groups: HashMap<Key, Group> = HashMap::new();
        let rows = stmt.query_map(params![kind.to_db(), since], |r| {
            Ok((
                r.get::<_, Id>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Id>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, i64>(6)?,
            ))
        })?;
        for row in rows {
            let (category_id, category_name, icon, account_id, amount, note, at) = row?;
            // circular distance on the 24 h clock, in hours
            let diff = (at - minute as i64).rem_euclid(1440);
            let hours = diff.min(1440 - diff) as f64 / 60.0;
            let closeness = (-hours / 1.5).exp();
            let key = (category_id, account_id, note.trim().to_lowercase());
            let group = groups.entry(key).or_insert_with(|| Group {
                // rows arrive newest first: keep the latest spelling and amount
                suggestion: Suggestion {
                    kind,
                    category_id,
                    category_name,
                    icon,
                    account_id,
                    amount: Money(amount),
                    note,
                    uses: 0,
                },
                score: 0.0,
                amounts: HashMap::new(),
            });
            group.suggestion.uses += 1;
            group.score += closeness;
            *group.amounts.entry(amount).or_default() += 1;
        }
        let mut list: Vec<(Suggestion, f64)> = groups
            .into_values()
            .filter(|g| g.suggestion.uses >= 2 && g.score >= 0.9)
            .map(|mut g| {
                // an amount used at least twice wins over the latest one
                if let Some((&amount, &n)) = g.amounts.iter().max_by_key(|(a, n)| (**n, **a)) {
                    if n >= 2 {
                        g.suggestion.amount = Money(amount);
                    }
                }
                (g.suggestion, g.score)
            })
            .collect();
        list.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.0.uses.cmp(&a.0.uses))
        });
        Ok(list.into_iter().take(limit).map(|(s, _)| s).collect())
    }

    // ----- budgets --------------------------------------------------------

    pub fn set_budget(&self, category_id: Id, limit: Option<Money>) -> Result<()> {
        match limit {
            Some(l) if l.0 > 0 => {
                if l > MAX_AMOUNT {
                    return Err(StoreError::Invalid("budget is too large"));
                }
                self.conn.execute(
                    "INSERT INTO budget (category_id, limit_minor) VALUES (?1, ?2)
                     ON CONFLICT(category_id) DO UPDATE SET limit_minor = excluded.limit_minor",
                    params![category_id, l.0],
                )?;
            }
            _ => {
                self.conn
                    .execute("DELETE FROM budget WHERE category_id = ?1", [category_id])?;
            }
        }
        Ok(())
    }

    /// `(category, limit, spent in period)` for every budgeted category.
    pub fn budgets(&self, period: Period) -> Result<Vec<(Category, Money, Money)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT c.id, c.name, c.kind, c.icon, b.limit_minor,
                    COALESCE((SELECT SUM(amount_minor) FROM txn t
                              WHERE t.category_id = c.id AND t.kind = 0
                                AND t.date >= ?1 AND t.date < ?2), 0)
             FROM budget b JOIN category c ON c.id = b.category_id
             WHERE c.archived = 0
             ORDER BY c.sort, c.id",
        )?;
        let rows = stmt.query_map(
            params![date_to_db(period.start), date_to_db(period.end)],
            |r| {
                Ok((
                    Category {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        kind: Kind::from_db(r.get(2)?),
                        icon: r.get(3)?,
                    },
                    Money(r.get(4)?),
                    Money(r.get(5)?),
                ))
            },
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Budget limit for every expense category (`None` when not set).
    pub fn budget_limits(&self) -> Result<HashMap<Id, Money>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT category_id, limit_minor FROM budget")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, Id>(0)?, Money(r.get(1)?))))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A database created by version 0.1 (schema v1) opens, upgrades in place
    /// and keeps its entries; old entries get noon as their time of day.
    #[test]
    fn upgrades_v1_database() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute_batch(
            "INSERT INTO account (name, kind) VALUES ('Cash', 0);
             INSERT INTO category (name, kind, icon) VALUES ('Food', 0, 'food');
             INSERT INTO txn (account_id, category_id, kind, amount_minor, date)
                 VALUES (1, 1, 0, 1250, '2026-10-30');
             INSERT INTO setting (key, value) VALUES ('seeded', '1');",
        )
        .unwrap();
        let store = Store::init(conn).unwrap();
        assert_eq!(store.schema_version().unwrap(), MIGRATIONS.len() as i64);
        let t = store.transactions(None, 10).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].amount, t[0].minute), (Money(1250), 720));
        assert_eq!(store.total_balance().unwrap(), Money(-1250));
    }
}
