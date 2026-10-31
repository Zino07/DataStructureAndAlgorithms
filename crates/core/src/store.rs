//! SQLite persistence. Balances are never stored: they are derived from the
//! opening amount plus the ledger, so editing or deleting a transaction can
//! never leave an account out of sync.

use std::path::Path;

use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};

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

fn date_to_db(d: NaiveDate) -> String {
    d.format(DATE_FMT).to_string()
}

fn date_from_db(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, DATE_FMT).unwrap_or_default()
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
];

const DEFAULT_EXPENSE_CATEGORIES: &[(&str, &str)] = &[
    ("Food", "food"),
    ("Transport", "car"),
    ("Home", "home"),
    ("Bills", "bolt"),
    ("Shopping", "bag"),
    ("Health", "heart"),
    ("Fun", "play"),
    ("Education", "book"),
    ("Other", "dots"),
];

const DEFAULT_INCOME_CATEGORIES: &[(&str, &str)] = &[
    ("Salary", "briefcase"),
    ("Freelance", "laptop"),
    ("Investments", "trend"),
    ("Gifts", "gift"),
    ("Other income", "dots"),
];

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

    fn migrate(&mut self) -> Result<()> {
        let version: usize =
            self.conn
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
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
        let seeded: Option<String> = self.setting("seeded")?;
        if seeded.is_some() {
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
            dark: self.setting("dark")?.as_deref() == Some("1"),
        })
    }

    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        if s.currency.trim().is_empty() {
            return Err(StoreError::Invalid("currency is required"));
        }
        self.set_setting("currency", s.currency.trim())?;
        self.set_setting(
            "cycle_start_day",
            &s.cycle_start_day
                .clamp(1, crate::period::MAX_CYCLE_START_DAY)
                .to_string(),
        )?;
        self.set_setting("palette", &s.palette.min(3).to_string())?;
        self.set_setting("dark", if s.dark { "1" } else { "0" })?;
        Ok(())
    }

    // ----- accounts -------------------------------------------------------

    pub fn add_account(&self, name: &str, kind: AccountKind, opening: Money) -> Result<Id> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StoreError::Invalid("account name is required"));
        }
        self.conn.execute(
            "INSERT INTO account (name, kind, opening_minor, sort)
             VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(sort), -1) + 1 FROM account))",
            params![name, kind.to_db(), opening.0],
        )?;
        Ok(self.conn.last_insert_rowid())
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

    pub fn add_category(&self, name: &str, kind: Kind, icon: &str) -> Result<Id> {
        let name = name.trim();
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

    pub fn add_transaction(&self, t: &NewTransaction) -> Result<Id> {
        if t.amount.0 <= 0 {
            return Err(StoreError::Invalid("amount must be greater than zero"));
        }
        let category_kind: Option<i64> = self
            .conn
            .query_row(
                "SELECT kind FROM category WHERE id = ?1",
                [t.category_id],
                |r| r.get(0),
            )
            .optional()?;
        match category_kind {
            None => return Err(StoreError::Invalid("unknown category")),
            Some(k) if Kind::from_db(k) != t.kind => {
                return Err(StoreError::Invalid(
                    "category does not match the transaction type",
                ))
            }
            Some(_) => {}
        }
        self.conn.execute(
            "INSERT INTO txn (account_id, category_id, kind, amount_minor, date, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                t.account_id,
                t.category_id,
                t.kind.to_db(),
                t.amount.0,
                date_to_db(t.date),
                t.note.trim()
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_transaction(&self, id: Id) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM txn WHERE id = ?1", [id])? > 0)
    }

    pub fn add_transfer(&self, t: &NewTransfer) -> Result<Id> {
        if t.amount.0 <= 0 {
            return Err(StoreError::Invalid("amount must be greater than zero"));
        }
        if t.from_account == t.to_account {
            return Err(StoreError::Invalid("choose two different accounts"));
        }
        self.conn.execute(
            "INSERT INTO transfer (from_account, to_account, amount_minor, date, note)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                t.from_account,
                t.to_account,
                t.amount.0,
                date_to_db(t.date),
                t.note.trim()
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Newest first. `kind = None` returns both directions.
    pub fn transactions(&self, kind: Option<Kind>, limit: usize) -> Result<Vec<Transaction>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT t.id, t.account_id, a.name, t.category_id, c.name, c.icon,
                    t.kind, t.amount_minor, t.date, t.note
             FROM txn t
             JOIN account a ON a.id = t.account_id
             JOIN category c ON c.id = t.category_id
             WHERE (?1 IS NULL OR t.kind = ?1)
             ORDER BY t.date DESC, t.id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![kind.map(Kind::to_db), limit as i64], |r| {
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
                note: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

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
        let found: std::collections::HashMap<NaiveDate, Money> = stmt
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

    // ----- budgets --------------------------------------------------------

    pub fn set_budget(&self, category_id: Id, limit: Option<Money>) -> Result<()> {
        match limit {
            Some(l) if l.0 > 0 => {
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
}
