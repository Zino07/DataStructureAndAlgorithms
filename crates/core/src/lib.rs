//! Income Radar core: everything that is not UI.
//!
//! * [`money`] integer money type, parsing and formatting
//! * [`period`] payday-based budget cycles
//! * [`store`] SQLite storage with derived balances
//! * [`radar`] the dashboard analytics
//! * [`stats`] statistics by hour and minute, patterns and insights

pub mod model;
pub mod money;
pub mod period;
pub mod radar;
pub mod stats;
pub mod store;

pub use chrono::NaiveDate;
pub use model::*;
pub use money::{Money, ParseMoneyError};
pub use period::Period;
pub use radar::Radar;
pub use stats::{format_minutes, Range, Stats};
pub use store::{Store, StoreError};
