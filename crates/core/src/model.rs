use chrono::NaiveDate;

use crate::money::Money;

pub type Id = i64;

/// Direction of a transaction. Stored as 0 / 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Expense,
    Income,
}

impl Kind {
    pub fn to_db(self) -> i64 {
        match self {
            Kind::Expense => 0,
            Kind::Income => 1,
        }
    }

    pub fn from_db(v: i64) -> Kind {
        if v == 1 {
            Kind::Income
        } else {
            Kind::Expense
        }
    }

    /// Signed effect of an amount of this kind on an account balance.
    pub fn signed(self, amount: Money) -> Money {
        match self {
            Kind::Expense => Money(-amount.0),
            Kind::Income => amount,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountKind {
    Cash,
    Bank,
    Card,
    Savings,
}

impl AccountKind {
    pub const ALL: [AccountKind; 4] = [
        AccountKind::Cash,
        AccountKind::Bank,
        AccountKind::Card,
        AccountKind::Savings,
    ];

    pub fn to_db(self) -> i64 {
        match self {
            AccountKind::Cash => 0,
            AccountKind::Bank => 1,
            AccountKind::Card => 2,
            AccountKind::Savings => 3,
        }
    }

    pub fn from_db(v: i64) -> AccountKind {
        match v {
            1 => AccountKind::Bank,
            2 => AccountKind::Card,
            3 => AccountKind::Savings,
            _ => AccountKind::Cash,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AccountKind::Cash => "Cash",
            AccountKind::Bank => "Bank",
            AccountKind::Card => "Card",
            AccountKind::Savings => "Savings",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            AccountKind::Cash => "cash",
            AccountKind::Bank => "bank",
            AccountKind::Card => "card",
            AccountKind::Savings => "savings",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: Id,
    pub name: String,
    pub kind: AccountKind,
    pub opening: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountBalance {
    pub account: Account,
    pub balance: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: Id,
    pub name: String,
    pub kind: Kind,
    /// Icon key, resolved to vector data by the UI layer.
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTransaction {
    pub account_id: Id,
    pub category_id: Id,
    pub kind: Kind,
    /// Always positive; `kind` carries the direction.
    pub amount: Money,
    pub date: NaiveDate,
    /// Local minute of the day, 0..=1439.
    pub minute: u16,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub id: Id,
    pub account_id: Id,
    pub account_name: String,
    pub category_id: Id,
    pub category_name: String,
    pub category_icon: String,
    pub kind: Kind,
    pub amount: Money,
    pub date: NaiveDate,
    pub minute: u16,
    pub note: String,
}

impl Transaction {
    /// Minutes since 0001-01-01 in local time; orders entries precisely.
    pub fn stamp(&self) -> i64 {
        stamp(self.date, self.minute)
    }
}

pub fn stamp(date: NaiveDate, minute: u16) -> i64 {
    use chrono::Datelike;
    (date.num_days_from_ce() as i64) * 1440 + minute as i64
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTransfer {
    pub from_account: Id,
    pub to_account: Id,
    pub amount: Money,
    pub date: NaiveDate,
    pub minute: u16,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    pub id: Id,
    pub from_account: Id,
    pub from_name: String,
    pub to_account: Id,
    pub to_name: String,
    pub amount: Money,
    pub date: NaiveDate,
    pub minute: u16,
    pub note: String,
}

/// A one-tap entry proposed from the user's own history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub kind: Kind,
    pub category_id: Id,
    pub category_name: String,
    pub icon: String,
    pub account_id: Id,
    pub amount: Money,
    pub note: String,
    /// How many times this exact entry was made in the look-back window.
    pub uses: u32,
}

/// Which two days count as the weekend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Weekend {
    /// Friday and Saturday (Saudi Arabia, Kuwait, Qatar, Bahrain, Oman, Egypt, Jordan...).
    #[default]
    FriSat,
    /// Saturday and Sunday (UAE, Morocco, Europe, Americas...).
    SatSun,
}

impl Weekend {
    pub fn contains(self, day: chrono::Weekday) -> bool {
        use chrono::Weekday::*;
        match self {
            Weekend::FriSat => matches!(day, Fri | Sat),
            Weekend::SatSun => matches!(day, Sat | Sun),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryTotal {
    pub category_id: Id,
    pub name: String,
    pub icon: String,
    pub total: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// ISO 4217 code shown next to amounts.
    pub currency: String,
    /// Day of the month the budgeting cycle starts on (payday), 1..=28.
    pub cycle_start_day: u32,
    /// Bento Neo palette: 0 lime, 1 solar, 2 ember, 3 lilac.
    pub palette: u32,
    pub dark: bool,
    pub weekend: Weekend,
    /// 0 Bento (solid), 1 Aurora Glass.
    pub style: u32,
    /// In-app "Reduce motion": no springs, entrances only fade.
    pub reduce_motion: bool,
    /// In-app "Reduce transparency": glass panels become solid.
    pub solid_glass: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            currency: "SAR".into(),
            cycle_start_day: 1,
            palette: 0,
            dark: true,
            weekend: Weekend::FriSat,
            style: 0,
            reduce_motion: false,
            solid_glass: false,
        }
    }
}
