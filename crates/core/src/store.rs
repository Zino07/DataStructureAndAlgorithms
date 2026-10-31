//! SQLite persistence. Balances are never stored: they are derived from the
//! opening amount plus the ledger, so editing or deleting a transaction can
//! never leave an account out of sync.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::Path;

use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::model::*;
use crate::money::{currency_decimals, Money};
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
pub fn max_amount() -> Money {
    Money(1_000_000_000 * Money::minor_per_major())
}

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
    // v3: indexes for the balance sub-queries, editable categories, repeating
    // entries, sample-data markers. Accounts can no longer be archived (they
    // are deleted or merged instead), so any hidden account comes back.
    "CREATE INDEX transfer_from ON transfer(from_account);
    CREATE INDEX transfer_to ON transfer(to_account);
    ALTER TABLE category ADD COLUMN work INTEGER NOT NULL DEFAULT 0;
    UPDATE category SET work = 1 WHERE kind = 1 AND name IN ('Salary', 'Freelance');
    ALTER TABLE account ADD COLUMN demo INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE category ADD COLUMN demo INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE txn ADD COLUMN demo INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE transfer ADD COLUMN demo INTEGER NOT NULL DEFAULT 0;
    UPDATE account SET archived = 0;
    CREATE TABLE recurring (
        id INTEGER PRIMARY KEY,
        account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
        category_id INTEGER NOT NULL REFERENCES category(id),
        kind INTEGER NOT NULL,
        amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
        minute INTEGER NOT NULL DEFAULT 720 CHECK (minute BETWEEN 0 AND 1439),
        note TEXT NOT NULL DEFAULT '',
        freq INTEGER NOT NULL,
        anchor_day INTEGER NOT NULL,
        next_date TEXT NOT NULL,
        active INTEGER NOT NULL DEFAULT 1
    );",
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
    /// Entries dated after this day are planned, not real: they stay out of
    /// balances and totals until their day comes. The app sets it every
    /// refresh; before that nothing is cut off.
    today: Cell<NaiveDate>,
}

fn far_future() -> NaiveDate {
    NaiveDate::from_ymd_opt(9999, 12, 31).unwrap_or(NaiveDate::MAX)
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
        let store = Store {
            conn,
            today: Cell::new(far_future()),
        };
        store.migrate()?;
        store.seed()?;
        Money::set_decimals(currency_decimals(&store.settings()?.currency));
        Ok(store)
    }

    /// Tells the store what day it is, so planned (future) entries stay out of
    /// balances and totals. Cheap; call it on every refresh.
    pub fn set_today(&self, today: NaiveDate) {
        self.today.set(today);
    }

    fn cap(&self) -> String {
        date_to_db(self.today.get())
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    fn migrate(&self) -> Result<()> {
        let version = self.schema_version()? as usize;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
            let tx = self.conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        Ok(())
    }

    /// First-run content: a cash account and a starter set of categories.
    fn seed(&self) -> Result<()> {
        if self.setting("seeded")?.is_some() {
            return Ok(());
        }
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO account (name, kind, opening_minor, sort) VALUES (?1, ?2, 0, 0)",
            params!["Cash", AccountKind::Cash.to_db()],
        )?;
        for (kind, list) in [
            (Kind::Expense, DEFAULT_EXPENSE_CATEGORIES),
            (Kind::Income, DEFAULT_INCOME_CATEGORIES),
        ] {
            for (sort, (name, icon)) in list.iter().enumerate() {
                let work = kind == Kind::Income && matches!(*name, "Salary" | "Freelance");
                tx.execute(
                    "INSERT INTO category (name, kind, icon, sort, work) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![name, kind.to_db(), icon, sort as i64, work],
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
            work_hours: num("work_hours", d.work_hours)?.clamp(1, 16),
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
        // a currency with a different number of decimals changes what one
        // stored unit means: rescale everything so the amounts keep their value
        let old = self.settings()?.currency;
        let (from, to) = (currency_decimals(&old), currency_decimals(currency));
        if from != to {
            self.rescale(from, to)?;
            Money::set_decimals(to);
        }
        self.set_setting("currency", currency)?;
        self.set_setting("work_hours", &s.work_hours.clamp(1, 16).to_string())?;
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

    /// Multiplies or divides every stored amount when the currency has a
    /// different number of decimals (KWD 3 → SAR 2 drops the last digit,
    /// rounded; the other way adds a zero).
    fn rescale(&self, from: u8, to: u8) -> Result<()> {
        let up = to > from;
        let f = 10_i64.pow(from.abs_diff(to) as u32);
        let amount = |col: &str| {
            if up {
                format!("{col} * {f}")
            } else {
                // never round a positive amount down to zero (CHECK > 0)
                format!("MAX(1, ({col} + {}) / {f})", f / 2)
            }
        };
        let signed = |col: &str| {
            if up {
                format!("{col} * {f}")
            } else {
                format!(
                    "CASE WHEN {col} >= 0 THEN ({col} + {h}) / {f} ELSE -((-{col} + {h}) / {f}) END",
                    h = f / 2
                )
            }
        };
        for table in ["txn", "transfer", "recurring"] {
            self.conn.execute_batch(&format!(
                "UPDATE {table} SET amount_minor = {};",
                amount("amount_minor")
            ))?;
        }
        self.conn.execute_batch(&format!(
            "UPDATE budget SET limit_minor = {};
             UPDATE account SET opening_minor = {};",
            amount("limit_minor"),
            signed("opening_minor")
        ))?;
        Ok(())
    }

    // ----- app lock -------------------------------------------------------

    fn pin_hash(salt: &str, pin: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::digest(format!("{salt}:{pin}").as_bytes());
        for _ in 0..20_000 {
            digest = Sha256::digest([digest.as_slice(), salt.as_bytes()].concat());
        }
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn has_pin(&self) -> Result<bool> {
        Ok(self.setting("pin_hash")?.is_some())
    }

    /// Sets (4–8 digits) or removes the app-lock PIN. This only keeps casual
    /// eyes away: the database file itself is not encrypted.
    pub fn set_pin(&self, pin: Option<&str>) -> Result<()> {
        match pin {
            None => {
                self.conn.execute(
                    "DELETE FROM setting WHERE key IN ('pin_hash', 'pin_salt')",
                    [],
                )?;
            }
            Some(p) => {
                if !(4..=8).contains(&p.len()) || !p.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(StoreError::Invalid("a PIN is 4 to 8 digits"));
                }
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                let salt = format!("{nanos:x}{:x}", std::process::id());
                self.set_setting("pin_salt", &salt)?;
                self.set_setting("pin_hash", &Self::pin_hash(&salt, p))?;
            }
        }
        Ok(())
    }

    pub fn verify_pin(&self, pin: &str) -> Result<bool> {
        let (Some(salt), Some(hash)) = (self.setting("pin_salt")?, self.setting("pin_hash")?)
        else {
            return Ok(true);
        };
        Ok(Self::pin_hash(&salt, pin) == hash)
    }

    // ----- accounts -------------------------------------------------------

    pub fn add_account(&self, name: &str, kind: AccountKind, opening: Money) -> Result<Id> {
        let name = clean_text(name, MAX_NAME_CHARS);
        if name.is_empty() {
            return Err(StoreError::Invalid("account name is required"));
        }
        if opening.abs() > max_amount() {
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

    /// Renames an account, changes its type or corrects its opening balance.
    pub fn update_account(
        &self,
        id: Id,
        name: &str,
        kind: AccountKind,
        opening: Money,
    ) -> Result<()> {
        let name = clean_text(name, MAX_NAME_CHARS);
        if name.is_empty() {
            return Err(StoreError::Invalid("account name is required"));
        }
        if opening.abs() > max_amount() {
            return Err(StoreError::Invalid("opening balance is too large"));
        }
        let clash: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM account WHERE archived = 0 AND id <> ?2 AND lower(name) = lower(?1))",
            params![name, id],
            |r| r.get(0),
        )?;
        if clash {
            return Err(StoreError::Invalid(
                "an account with this name already exists",
            ));
        }
        let changed = self.conn.execute(
            "UPDATE account SET name = ?2, kind = ?3, opening_minor = ?4 WHERE id = ?1",
            params![id, name, kind.to_db(), opening.0],
        )?;
        if changed == 0 {
            return Err(StoreError::Invalid("this account no longer exists"));
        }
        Ok(())
    }

    /// How many entries and transfers use an account.
    pub fn account_usage(&self, id: Id) -> Result<(i64, i64)> {
        let entries = self.conn.query_row(
            "SELECT COUNT(*) FROM txn WHERE account_id = ?1",
            [id],
            |r| r.get(0),
        )?;
        let transfers = self.conn.query_row(
            "SELECT COUNT(*) FROM transfer WHERE from_account = ?1 OR to_account = ?1",
            [id],
            |r| r.get(0),
        )?;
        Ok((entries, transfers))
    }

    /// Deletes an account. Its entries either move to another account or go
    /// with it. Transfers between the two accounts disappear when merged
    /// (they would be transfers to itself). At least one account must remain.
    pub fn delete_account(&self, id: Id, fate: Fate) -> Result<()> {
        let accounts: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM account", [], |r| r.get(0))?;
        if accounts <= 1 {
            return Err(StoreError::Invalid("keep at least one account"));
        }
        let tx = self.conn.unchecked_transaction()?;
        match fate {
            Fate::MoveTo(to) => {
                if to == id {
                    return Err(StoreError::Invalid("choose another account"));
                }
                let exists: bool = self.conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM account WHERE id = ?1)",
                    [to],
                    |r| r.get(0),
                )?;
                if !exists {
                    return Err(StoreError::Invalid("choose another account"));
                }
                self.conn.execute(
                    "UPDATE txn SET account_id = ?2 WHERE account_id = ?1",
                    params![id, to],
                )?;
                self.conn.execute(
                    "DELETE FROM transfer
                     WHERE (from_account = ?1 AND to_account = ?2)
                        OR (from_account = ?2 AND to_account = ?1)",
                    params![id, to],
                )?;
                self.conn.execute(
                    "UPDATE transfer SET from_account = ?2 WHERE from_account = ?1",
                    params![id, to],
                )?;
                self.conn.execute(
                    "UPDATE transfer SET to_account = ?2 WHERE to_account = ?1",
                    params![id, to],
                )?;
                self.conn.execute(
                    "UPDATE recurring SET account_id = ?2 WHERE account_id = ?1",
                    params![id, to],
                )?;
                // keep the money: the old opening balance moves with the entries
                self.conn.execute(
                    "UPDATE account SET opening_minor = opening_minor +
                        (SELECT opening_minor FROM account WHERE id = ?1) WHERE id = ?2",
                    params![id, to],
                )?;
            }
            Fate::DeleteAll => {
                self.conn
                    .execute("DELETE FROM txn WHERE account_id = ?1", [id])?;
                self.conn.execute(
                    "DELETE FROM transfer WHERE from_account = ?1 OR to_account = ?1",
                    [id],
                )?;
                self.conn
                    .execute("DELETE FROM recurring WHERE account_id = ?1", [id])?;
            }
        }
        let gone = self
            .conn
            .execute("DELETE FROM account WHERE id = ?1", [id])?;
        if gone == 0 {
            return Err(StoreError::Invalid("this account no longer exists"));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn accounts_with_balances(&self) -> Result<Vec<AccountBalance>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT a.id, a.name, a.kind, a.opening_minor,
                a.opening_minor
                + COALESCE((SELECT SUM(CASE WHEN t.kind = 1 THEN t.amount_minor ELSE -t.amount_minor END)
                            FROM txn t WHERE t.account_id = a.id AND t.date <= ?1), 0)
                + COALESCE((SELECT SUM(amount_minor) FROM transfer WHERE to_account = a.id AND date <= ?1), 0)
                - COALESCE((SELECT SUM(amount_minor) FROM transfer WHERE from_account = a.id AND date <= ?1), 0)
             FROM account a
             WHERE a.archived = 0
             ORDER BY a.sort, a.id",
        )?;
        let rows = stmt.query_map([self.cap()], |r| {
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

    /// Every category of one kind, hidden ones included, for the manager.
    pub fn category_infos(&self, kind: Kind) -> Result<Vec<CategoryInfo>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT c.id, c.name, c.kind, c.icon, c.archived, c.work,
                    (SELECT COUNT(*) FROM txn t WHERE t.category_id = c.id),
                    EXISTS(SELECT 1 FROM budget b WHERE b.category_id = c.id)
             FROM category c WHERE c.kind = ?1 ORDER BY c.sort, c.id",
        )?;
        let rows = stmt.query_map([kind.to_db()], |r| {
            Ok(CategoryInfo {
                category: Category {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    kind: Kind::from_db(r.get(2)?),
                    icon: r.get(3)?,
                },
                hidden: r.get::<_, i64>(4)? != 0,
                work: r.get::<_, i64>(5)? != 0,
                uses: r.get(6)?,
                has_budget: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    fn category_kind(&self, id: Id) -> Result<Kind> {
        let k: Option<i64> = self
            .conn
            .query_row("SELECT kind FROM category WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        k.map(Kind::from_db)
            .ok_or(StoreError::Invalid("this category no longer exists"))
    }

    fn visible_categories(&self, kind: Kind) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM category WHERE kind = ?1 AND archived = 0",
            [kind.to_db()],
            |r| r.get(0),
        )?)
    }

    fn category_name_taken(&self, name: &str, kind: Kind, except: Id) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM category
                           WHERE kind = ?1 AND archived = 0 AND id <> ?2 AND lower(name) = lower(?3))",
            params![kind.to_db(), except, name],
            |r| r.get(0),
        )?)
    }

    /// Renames a category, changes its icon, or marks it as working income.
    pub fn update_category(&self, id: Id, name: &str, icon: &str, work: bool) -> Result<()> {
        let name = clean_text(name, MAX_NAME_CHARS);
        if name.is_empty() {
            return Err(StoreError::Invalid("category name is required"));
        }
        let kind = self.category_kind(id)?;
        if self.category_name_taken(&name, kind, id)? {
            return Err(StoreError::Invalid(
                "a category with this name already exists",
            ));
        }
        self.conn.execute(
            "UPDATE category SET name = ?2, icon = ?3, work = ?4 WHERE id = ?1",
            params![id, name, icon, work && kind == Kind::Income],
        )?;
        Ok(())
    }

    /// Hides a category from the pickers (its history stays) or shows it again.
    pub fn set_category_hidden(&self, id: Id, hidden: bool) -> Result<()> {
        let kind = self.category_kind(id)?;
        if hidden && self.visible_categories(kind)? <= 1 {
            return Err(StoreError::Invalid("keep at least one category"));
        }
        if !hidden {
            let name: String =
                self.conn
                    .query_row("SELECT name FROM category WHERE id = ?1", [id], |r| {
                        r.get(0)
                    })?;
            if self.category_name_taken(&name, kind, id)? {
                return Err(StoreError::Invalid(
                    "a category with this name already exists",
                ));
            }
        }
        self.conn.execute(
            "UPDATE category SET archived = ?2 WHERE id = ?1",
            params![id, hidden],
        )?;
        Ok(())
    }

    /// Moves a category one place up or down in its list.
    pub fn move_category(&self, id: Id, up: bool) -> Result<()> {
        let kind = self.category_kind(id)?;
        let ids: Vec<Id> = self
            .category_infos(kind)?
            .into_iter()
            .map(|c| c.category.id)
            .collect();
        let Some(pos) = ids.iter().position(|c| *c == id) else {
            return Ok(());
        };
        let other = if up {
            pos.checked_sub(1)
        } else {
            (pos + 1 < ids.len()).then_some(pos + 1)
        };
        let Some(other) = other else { return Ok(()) };
        let mut order = ids;
        order.swap(pos, other);
        let tx = self.conn.unchecked_transaction()?;
        for (i, cid) in order.iter().enumerate() {
            self.conn.execute(
                "UPDATE category SET sort = ?2 WHERE id = ?1",
                params![cid, i as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Deletes a category. Its entries move to `move_to` (same kind) or, when
    /// `None`, are deleted with it. The budget for it goes either way.
    pub fn delete_category(&self, id: Id, move_to: Option<Id>) -> Result<()> {
        let kind = self.category_kind(id)?;
        let others: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM category WHERE kind = ?1 AND id <> ?2 AND archived = 0",
            params![kind.to_db(), id],
            |r| r.get(0),
        )?;
        if others == 0 {
            return Err(StoreError::Invalid("keep at least one category"));
        }
        let tx = self.conn.unchecked_transaction()?;
        match move_to {
            Some(to) => {
                if to == id || self.category_kind(to)? != kind {
                    return Err(StoreError::Invalid("choose another category"));
                }
                self.conn.execute(
                    "UPDATE txn SET category_id = ?2 WHERE category_id = ?1",
                    params![id, to],
                )?;
                self.conn.execute(
                    "UPDATE recurring SET category_id = ?2 WHERE category_id = ?1",
                    params![id, to],
                )?;
            }
            None => {
                self.conn
                    .execute("DELETE FROM txn WHERE category_id = ?1", [id])?;
                self.conn
                    .execute("DELETE FROM recurring WHERE category_id = ?1", [id])?;
            }
        }
        self.conn
            .execute("DELETE FROM budget WHERE category_id = ?1", [id])?;
        self.conn
            .execute("DELETE FROM category WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn add_category(&self, name: &str, kind: Kind, icon: &str) -> Result<Id> {
        let name = clean_text(name, MAX_NAME_CHARS);
        if name.is_empty() {
            return Err(StoreError::Invalid("category name is required"));
        }
        if self.category_name_taken(&name, kind, -1)? {
            return Err(StoreError::Invalid(
                "a category with this name already exists",
            ));
        }
        self.conn.execute(
            "INSERT INTO category (name, kind, icon, sort, work)
             VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(sort), -1) + 1 FROM category WHERE kind = ?2), ?4)",
            params![name, kind.to_db(), icon, kind == Kind::Income],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    // ----- transactions ---------------------------------------------------

    fn validate(&self, t: &NewTransaction) -> Result<()> {
        if t.amount.0 <= 0 {
            return Err(StoreError::Invalid("amount must be greater than zero"));
        }
        if t.amount > max_amount() {
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
            "{TX_SELECT} WHERE t.date >= ?1 AND t.date < ?2 AND t.date <= ?3
             ORDER BY t.date, t.minute, t.id"
        ))?;
        let rows = stmt.query_map(
            params![date_to_db(period.start), date_to_db(period.end), self.cap()],
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
        self.validate_transfer(t)?;
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

    fn validate_transfer(&self, t: &NewTransfer) -> Result<()> {
        if t.amount.0 <= 0 {
            return Err(StoreError::Invalid("amount must be greater than zero"));
        }
        if t.amount > max_amount() {
            return Err(StoreError::Invalid("amount is too large"));
        }
        if t.from_account == t.to_account {
            return Err(StoreError::Invalid("choose two different accounts"));
        }
        if !self.account_is_active(t.from_account)? || !self.account_is_active(t.to_account)? {
            return Err(StoreError::Invalid("choose an account"));
        }
        Ok(())
    }

    pub fn update_transfer(&self, id: Id, t: &NewTransfer) -> Result<()> {
        self.validate_transfer(t)?;
        let changed = self.conn.execute(
            "UPDATE transfer SET from_account = ?2, to_account = ?3, amount_minor = ?4,
                    date = ?5, minute = ?6, note = ?7
             WHERE id = ?1",
            params![
                id,
                t.from_account,
                t.to_account,
                t.amount.0,
                date_to_db(t.date),
                t.minute.min(1439),
                clean_text(&t.note, MAX_NOTE_CHARS)
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::Invalid("this transfer no longer exists"));
        }
        Ok(())
    }

    pub fn transfer(&self, id: Id) -> Result<Option<Transfer>> {
        Ok(self
            .transfers_where("WHERE tr.id = ?1", params![id], 1)?
            .into_iter()
            .next())
    }

    pub fn delete_transfer(&self, id: Id) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM transfer WHERE id = ?1", [id])?
            > 0)
    }

    pub fn transfers(&self, limit: usize) -> Result<Vec<Transfer>> {
        self.transfers_where("", params![], limit)
    }

    fn transfers_where(
        &self,
        filter: &str,
        args: &[&dyn rusqlite::ToSql],
        limit: usize,
    ) -> Result<Vec<Transfer>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT tr.id, tr.from_account, fa.name, tr.to_account, ta.name,
                    tr.amount_minor, tr.date, tr.minute, tr.note
             FROM transfer tr
             JOIN account fa ON fa.id = tr.from_account
             JOIN account ta ON ta.id = tr.to_account
             {filter}
             ORDER BY tr.date DESC, tr.minute DESC, tr.id DESC
             LIMIT {}",
            limit.min(i64::MAX as usize)
        ))?;
        let rows = stmt.query_map(args, |r| {
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
             FROM txn WHERE date >= ?1 AND date < ?2 AND date <= ?3",
            params![date_to_db(period.start), date_to_db(period.end), self.cap()],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )?;
        Ok((Money(income), Money(expense)))
    }

    /// Income from categories marked as working income, for "earned per hour".
    pub fn work_income(&self, period: Period) -> Result<Money> {
        let v: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(t.amount_minor), 0)
             FROM txn t JOIN category c ON c.id = t.category_id
             WHERE t.kind = 1 AND c.work = 1
               AND t.date >= ?1 AND t.date < ?2 AND t.date <= ?3",
            params![date_to_db(period.start), date_to_db(period.end), self.cap()],
            |r| r.get(0),
        )?;
        Ok(Money(v))
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
             WHERE t.kind = ?1 AND t.date >= ?2 AND t.date < ?3 AND t.date <= ?4
             GROUP BY c.id
             ORDER BY total DESC, c.sort",
        )?;
        let rows = stmt.query_map(
            params![
                kind.to_db(),
                date_to_db(period.start),
                date_to_db(period.end),
                self.cap()
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

    // ----- repeating entries ---------------------------------------------

    fn advance(date: NaiveDate, repeat: Repeat, anchor: u32) -> NaiveDate {
        use chrono::{Datelike, Days, Months};
        let monthly = |n: u32| {
            let first = date.with_day(1).unwrap_or(date) + Months::new(n);
            let last = (first + Months::new(1)).pred_opt().unwrap_or(first).day();
            first.with_day(anchor.min(last)).unwrap_or(first)
        };
        match repeat {
            Repeat::Daily => date + Days::new(1),
            Repeat::Weekly => date + Days::new(7),
            Repeat::Monthly => monthly(1),
            Repeat::Yearly => monthly(12),
        }
    }

    /// The date a repeating entry comes back after `date`.
    pub fn next_occurrence(date: NaiveDate, repeat: Repeat) -> NaiveDate {
        use chrono::Datelike;
        Self::advance(date, repeat, date.day())
    }

    /// Saves a repeating entry. The first copy is created on `first_date`
    /// (immediately if that day has come).
    pub fn add_recurring(&self, r: &NewRecurring) -> Result<Id> {
        use chrono::Datelike;
        self.validate(&NewTransaction {
            account_id: r.account_id,
            category_id: r.category_id,
            kind: r.kind,
            amount: r.amount,
            date: r.first_date,
            minute: r.minute,
            note: r.note.clone(),
        })?;
        self.conn.execute(
            "INSERT INTO recurring (account_id, category_id, kind, amount_minor, minute, note,
                                    freq, anchor_day, next_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                r.account_id,
                r.category_id,
                r.kind.to_db(),
                r.amount.0,
                r.minute,
                clean_text(&r.note, MAX_NOTE_CHARS),
                r.repeat.to_db(),
                r.first_date.day(),
                date_to_db(r.first_date)
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn recurring_list(&self) -> Result<Vec<Recurring>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT r.id, a.name, c.name, c.icon, r.kind, r.amount_minor, r.minute, r.note,
                    r.freq, r.next_date, r.active
             FROM recurring r
             JOIN account a ON a.id = r.account_id
             JOIN category c ON c.id = r.category_id
             ORDER BY r.active DESC, r.next_date, r.id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Recurring {
                id: r.get(0)?,
                account_name: r.get(1)?,
                category_name: r.get(2)?,
                category_icon: r.get(3)?,
                kind: Kind::from_db(r.get(4)?),
                amount: Money(r.get(5)?),
                minute: r.get::<_, i64>(6)?.clamp(0, 1439) as u16,
                note: r.get(7)?,
                repeat: Repeat::from_db(r.get(8)?),
                next_date: date_from_db(&r.get::<_, String>(9)?),
                active: r.get::<_, i64>(10)? != 0,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete_recurring(&self, id: Id) -> Result<()> {
        self.conn
            .execute("DELETE FROM recurring WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn set_recurring_active(&self, id: Id, active: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE recurring SET active = ?2 WHERE id = ?1",
            params![id, active],
        )?;
        Ok(())
    }

    /// Creates every repeating entry that has come due up to `today`
    /// (catching up after days without opening the app). Returns how many
    /// entries were created.
    pub fn run_recurring(&self, today: NaiveDate) -> Result<u32> {
        struct Due {
            id: Id,
            tx: NewTransaction,
            repeat: Repeat,
            anchor: u32,
            next: NaiveDate,
        }
        let mut due = Vec::new();
        {
            let mut stmt = self.conn.prepare_cached(
                "SELECT id, account_id, category_id, kind, amount_minor, minute, note, freq,
                        anchor_day, next_date
                 FROM recurring WHERE active = 1 AND next_date <= ?1",
            )?;
            let rows = stmt.query_map([date_to_db(today)], |r| {
                let next = date_from_db(&r.get::<_, String>(9)?);
                Ok(Due {
                    id: r.get(0)?,
                    tx: NewTransaction {
                        account_id: r.get(1)?,
                        category_id: r.get(2)?,
                        kind: Kind::from_db(r.get(3)?),
                        amount: Money(r.get(4)?),
                        minute: r.get::<_, i64>(5)?.clamp(0, 1439) as u16,
                        note: r.get(6)?,
                        date: next,
                    },
                    repeat: Repeat::from_db(r.get(7)?),
                    anchor: r.get::<_, i64>(8)?.clamp(1, 31) as u32,
                    next,
                })
            })?;
            for row in rows {
                due.push(row?);
            }
        }
        let tx = self.conn.unchecked_transaction()?;
        let mut created = 0;
        for mut d in due {
            let mut guard = 0;
            while d.next <= today && guard < 800 {
                d.tx.date = d.next;
                match self.add_transaction(&d.tx) {
                    Ok(_) => created += 1,
                    Err(StoreError::Invalid(_)) => {
                        // its account or category is gone or hidden: pause it
                        self.set_recurring_active(d.id, false)?;
                        break;
                    }
                    Err(e) => return Err(e),
                }
                d.next = Self::advance(d.next, d.repeat, d.anchor);
                guard += 1;
            }
            self.conn.execute(
                "UPDATE recurring SET next_date = ?2 WHERE id = ?1",
                params![d.id, date_to_db(d.next)],
            )?;
        }
        tx.commit()?;
        Ok(created)
    }

    // ----- sample data ----------------------------------------------------

    /// Highest ids before loading sample data; see [`Store::mark_demo`].
    pub fn demo_baseline(&self) -> Result<[Id; 4]> {
        let max = |table: &str| -> Result<Id> {
            Ok(self.conn.query_row(
                &format!("SELECT COALESCE(MAX(id), 0) FROM {table}"),
                [],
                |r| r.get(0),
            )?)
        };
        Ok([
            max("txn")?,
            max("transfer")?,
            max("account")?,
            max("category")?,
        ])
    }

    /// Flags everything created since `base` as sample data, so it can be
    /// removed on its own later without touching the user's real entries.
    pub fn mark_demo(&self, base: [Id; 4]) -> Result<()> {
        for (table, since) in ["txn", "transfer", "account", "category"].iter().zip(base) {
            self.conn.execute(
                &format!("UPDATE {table} SET demo = 1 WHERE id > ?1"),
                [since],
            )?;
        }
        Ok(())
    }

    pub fn has_demo(&self) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM txn WHERE demo = 1)
                 OR EXISTS(SELECT 1 FROM transfer WHERE demo = 1)
                 OR EXISTS(SELECT 1 FROM account WHERE demo = 1)",
            [],
            |r| r.get(0),
        )?)
    }

    /// Removes the sample entries, transfers and accounts. An account the
    /// user has since added real entries to is kept (and stops being "sample").
    pub fn remove_demo(&self) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        self.conn.execute_batch(
            "DELETE FROM txn WHERE demo = 1;
             DELETE FROM transfer WHERE demo = 1;
             UPDATE account SET demo = 0 WHERE demo = 1 AND (
                 EXISTS(SELECT 1 FROM txn WHERE account_id = account.id)
              OR EXISTS(SELECT 1 FROM transfer WHERE from_account = account.id OR to_account = account.id)
              OR (SELECT COUNT(*) FROM account) <= 1);
             DELETE FROM recurring WHERE account_id IN (SELECT id FROM account WHERE demo = 1);
             DELETE FROM account WHERE demo = 1;
             DELETE FROM budget WHERE category_id IN (SELECT id FROM category WHERE demo = 1);
             UPDATE category SET demo = 0 WHERE demo = 1 AND EXISTS(SELECT 1 FROM txn WHERE category_id = category.id);
             DELETE FROM category WHERE demo = 1;",
        )?;
        tx.commit()?;
        Ok(())
    }

    // ----- your data: erase, back up, restore, export ---------------------

    /// Deletes every account, entry, category, budget and the PIN, then sets
    /// up the starter account and categories again. Look settings stay.
    pub fn erase_all(&self) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        self.conn.execute_batch(
            "DELETE FROM recurring; DELETE FROM budget; DELETE FROM txn; DELETE FROM transfer;
             DELETE FROM category; DELETE FROM account;
             DELETE FROM setting WHERE key IN ('seeded', 'pin_hash', 'pin_salt');",
        )?;
        tx.commit()?;
        self.seed()
    }

    /// Writes a complete, self-contained copy of the database to `path`
    /// (an SQLite file that `restore_from` can read back).
    pub fn backup_to(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)
                .map_err(|_| StoreError::Invalid("could not replace the old backup file"))?;
        }
        let target = path
            .to_str()
            .ok_or(StoreError::Invalid("the backup path is not valid text"))?;
        self.conn.execute("VACUUM INTO ?1", [target])?;
        Ok(())
    }

    /// Replaces everything with the contents of a backup written by
    /// [`Store::backup_to`] (also from an older version of the app).
    pub fn restore_from(&self, path: &Path) -> Result<()> {
        let invalid = StoreError::Invalid("this is not an Income Radar backup");
        // work on a copy: opening it migrates old backups, and the original stays untouched
        let tmp = path.with_extension("restoring");
        std::fs::copy(path, &tmp)
            .map_err(|_| StoreError::Invalid("could not read the backup file"))?;
        let result = (|| -> Result<()> {
            {
                let probe =
                    Connection::open_with_flags(&tmp, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                        .map_err(|_| StoreError::Invalid("this is not an Income Radar backup"))?;
                let has_txn: bool = probe
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='txn')",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(|_| StoreError::Invalid("this is not an Income Radar backup"))?;
                if !has_txn {
                    return Err(invalid);
                }
                let version: i64 = probe.query_row("PRAGMA user_version", [], |r| r.get(0))?;
                if version > MIGRATIONS.len() as i64 {
                    return Err(StoreError::Invalid(
                        "this backup was made by a newer version of the app",
                    ));
                }
            }
            drop(Store::open(&tmp)?); // migrates an older backup in place
            let tmp_text = tmp
                .to_str()
                .ok_or(StoreError::Invalid("the backup path is not valid text"))?;
            self.conn.execute("ATTACH DATABASE ?1 AS bk", [tmp_text])?;
            let copied = (|| -> Result<()> {
                let tx = self.conn.unchecked_transaction()?;
                self.conn.execute_batch(
                    "DELETE FROM recurring; DELETE FROM budget; DELETE FROM txn; DELETE FROM transfer;
                     DELETE FROM category; DELETE FROM account; DELETE FROM setting;
                     INSERT INTO main.account SELECT * FROM bk.account;
                     INSERT INTO main.category SELECT * FROM bk.category;
                     INSERT INTO main.setting SELECT * FROM bk.setting;
                     INSERT INTO main.txn SELECT * FROM bk.txn;
                     INSERT INTO main.transfer SELECT * FROM bk.transfer;
                     INSERT INTO main.budget SELECT * FROM bk.budget;
                     INSERT INTO main.recurring SELECT * FROM bk.recurring;",
                )?;
                tx.commit()?;
                Ok(())
            })();
            let _ = self.conn.execute("DETACH DATABASE bk", []);
            copied
        })();
        for suffix in ["restoring", "restoring-wal", "restoring-shm"] {
            let _ = std::fs::remove_file(path.with_extension(suffix));
        }
        Money::set_decimals(currency_decimals(&self.settings()?.currency));
        result
    }

    /// Every entry and transfer as CSV (opens in any spreadsheet), oldest first.
    pub fn export_csv(&self) -> Result<String> {
        fn cell(s: &str) -> String {
            // quotes doubled; a leading = + - @ is neutralised so a note can
            // never run as a spreadsheet formula
            let mut v = s.replace('"', "\"\"");
            if v.starts_with(['=', '+', '-', '@']) {
                v.insert(0, '\'');
            }
            format!("\"{v}\"")
        }
        let digits = Money::decimals() as usize;
        let amount = |m: Money| {
            let per = Money::minor_per_major();
            let (w, f) = (m.0 / per, m.0 % per);
            if digits == 0 {
                w.to_string()
            } else {
                format!("{w}.{f:0digits$}")
            }
        };
        let mut rows: Vec<(String, u16, String)> = Vec::new();
        for t in self.transactions_all()? {
            let sign = if t.kind == Kind::Income {
                "Income"
            } else {
                "Expense"
            };
            rows.push((
                date_to_db(t.date),
                t.minute,
                format!(
                    "{},{:02}:{:02},{},{},{},{},{}",
                    date_to_db(t.date),
                    t.minute / 60,
                    t.minute % 60,
                    sign,
                    cell(&t.account_name),
                    cell(&t.category_name),
                    amount(t.amount),
                    cell(&t.note)
                ),
            ));
        }
        for t in self.transfers(usize::MAX >> 1)? {
            rows.push((
                date_to_db(t.date),
                t.minute,
                format!(
                    "{},{:02}:{:02},Transfer,{},{},{},{}",
                    date_to_db(t.date),
                    t.minute / 60,
                    t.minute % 60,
                    cell(&format!("{} -> {}", t.from_name, t.to_name)),
                    cell("Transfer"),
                    amount(t.amount),
                    cell(&t.note)
                ),
            ));
        }
        rows.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
        let mut out = String::from("date,time,type,account,category,amount,note\n");
        for (_, _, line) in rows {
            out.push_str(&line);
            out.push('\n');
        }
        Ok(out)
    }

    fn transactions_all(&self) -> Result<Vec<Transaction>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{TX_SELECT} ORDER BY t.date, t.minute, t.id"))?;
        let rows = stmt.query_map([], tx_from_row)?;
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
             LIMIT 5000",
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
                if l > max_amount() {
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
                                AND t.date >= ?1 AND t.date < ?2 AND t.date <= ?3), 0)
             FROM budget b JOIN category c ON c.id = b.category_id
             WHERE c.archived = 0
             ORDER BY c.sort, c.id",
        )?;
        let rows = stmt.query_map(
            params![date_to_db(period.start), date_to_db(period.end), self.cap()],
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
