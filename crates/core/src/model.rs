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
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTransfer {
    pub from_account: Id,
    pub to_account: Id,
    pub amount: Money,
    pub date: NaiveDate,
    pub note: String,
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
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            currency: "SAR".into(),
            cycle_start_day: 1,
            palette: 0,
            dark: false,
        }
    }
}
