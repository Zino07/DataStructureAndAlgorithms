//! Money is stored as a signed count of minor units (cents, halalas, piasters).
//! Integer arithmetic means balances never drift the way `f64` sums do.

use std::fmt;

/// Number of minor units in one major unit. Every currency the app ships with
/// uses two decimals; three-decimal currencies (KWD, BHD, OMR) are displayed
/// with two as well, which is enough for personal budgeting.
pub const MINOR_PER_MAJOR: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Money(pub i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseMoneyError {
    Empty,
    Invalid,
    TooManyDecimals,
    Overflow,
}

impl fmt::Display for ParseMoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::Empty => "amount is empty",
            Self::Invalid => "amount is not a number",
            Self::TooManyDecimals => "amount has more than two decimals",
            Self::Overflow => "amount is too large",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for ParseMoneyError {}

impl Money {
    pub const ZERO: Money = Money(0);

    pub fn minor(self) -> i64 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn abs(self) -> Money {
        Money(self.0.saturating_abs())
    }

    /// Parses user input such as `"12"`, `"12.5"`, `"1,250.75"` or `"12٫50"`.
    /// Arabic-Indic digits are accepted so the parser works with any keyboard.
    pub fn parse(input: &str) -> Result<Money, ParseMoneyError> {
        let mut cleaned = String::with_capacity(input.len());
        for ch in input.trim().chars() {
            match ch {
                '0'..='9' | '.' | '-' => cleaned.push(ch),
                '\u{0660}'..='\u{0669}' => {
                    cleaned.push(char::from(b'0' + (ch as u32 - 0x0660) as u8))
                }
                '\u{06F0}'..='\u{06F9}' => {
                    cleaned.push(char::from(b'0' + (ch as u32 - 0x06F0) as u8))
                }
                '\u{066B}' => cleaned.push('.'),
                ',' | '\u{066C}' | ' ' | '\u{00A0}' | '\u{202F}' => {}
                _ => return Err(ParseMoneyError::Invalid),
            }
        }
        if cleaned.is_empty() {
            return Err(ParseMoneyError::Empty);
        }
        let (negative, body) = match cleaned.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, cleaned.as_str()),
        };
        let mut parts = body.split('.');
        let whole = parts.next().unwrap_or("");
        let frac = parts.next();
        if parts.next().is_some() || body.contains('-') {
            return Err(ParseMoneyError::Invalid);
        }
        if whole.is_empty() && frac.is_none_or(str::is_empty) {
            return Err(ParseMoneyError::Invalid);
        }
        let whole_value: i64 = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| ParseMoneyError::Overflow)?
        };
        let frac_value: i64 = match frac {
            None | Some("") => 0,
            Some(f) if f.len() == 1 => f.parse::<i64>().map_err(|_| ParseMoneyError::Invalid)? * 10,
            Some(f) if f.len() == 2 => f.parse::<i64>().map_err(|_| ParseMoneyError::Invalid)?,
            Some(_) => return Err(ParseMoneyError::TooManyDecimals),
        };
        let minor = whole_value
            .checked_mul(MINOR_PER_MAJOR)
            .and_then(|v| v.checked_add(frac_value))
            .ok_or(ParseMoneyError::Overflow)?;
        Ok(Money(if negative { -minor } else { minor }))
    }

    /// `31211.00` → `"31,211.00"`.
    pub fn format(self) -> String {
        let negative = self.0 < 0;
        let abs = self.0.unsigned_abs();
        let whole = abs / MINOR_PER_MAJOR as u64;
        let frac = abs % MINOR_PER_MAJOR as u64;
        let mut out = String::new();
        if negative {
            out.push('-');
        }
        out.push_str(&group_thousands(whole));
        out.push('.');
        out.push_str(&format!("{frac:02}"));
        out
    }

    /// Compact form for tight tiles: `"12.4k"`, `"1.2M"`, or the full amount below 10k.
    pub fn format_compact(self) -> String {
        let major = self.0 as f64 / MINOR_PER_MAJOR as f64;
        let abs = major.abs();
        let sign = if major < 0.0 { "-" } else { "" };
        if abs >= 1_000_000.0 {
            format!("{sign}{:.1}M", abs / 1_000_000.0)
        } else if abs >= 10_000.0 {
            format!("{sign}{:.1}k", abs / 1_000.0)
        } else {
            self.format()
        }
    }
}

fn group_thousands(mut n: u64) -> String {
    if n == 0 {
        return "0".into();
    }
    let mut groups = Vec::new();
    while n > 0 {
        groups.push(n % 1000);
        n /= 1000;
    }
    let mut out = groups.pop().unwrap().to_string();
    while let Some(g) = groups.pop() {
        out.push(',');
        out.push_str(&format!("{g:03}"));
    }
    out
}

impl std::ops::Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}

impl std::ops::AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        self.0 += rhs.0;
    }
}

impl std::iter::Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        Money(iter.map(|m| m.0).sum())
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_inputs() {
        assert_eq!(Money::parse("12"), Ok(Money(1200)));
        assert_eq!(Money::parse("12.5"), Ok(Money(1250)));
        assert_eq!(Money::parse("12.05"), Ok(Money(1205)));
        assert_eq!(Money::parse(".5"), Ok(Money(50)));
        assert_eq!(Money::parse("12."), Ok(Money(1200)));
        assert_eq!(Money::parse("1,250.75"), Ok(Money(125075)));
        assert_eq!(Money::parse("-3.10"), Ok(Money(-310)));
        assert_eq!(Money::parse("١٢٫٥٠"), Ok(Money(1250)));
        assert_eq!(Money::parse("۱۲"), Ok(Money(1200)));
    }

    #[test]
    fn rejects_bad_inputs() {
        assert_eq!(Money::parse(""), Err(ParseMoneyError::Empty));
        assert_eq!(Money::parse("."), Err(ParseMoneyError::Invalid));
        assert_eq!(Money::parse("1.2.3"), Err(ParseMoneyError::Invalid));
        assert_eq!(Money::parse("1.234"), Err(ParseMoneyError::TooManyDecimals));
        assert_eq!(Money::parse("abc"), Err(ParseMoneyError::Invalid));
        assert_eq!(Money::parse("1-2"), Err(ParseMoneyError::Invalid));
        assert_eq!(
            Money::parse("99999999999999999999"),
            Err(ParseMoneyError::Overflow)
        );
    }

    #[test]
    fn formats_with_grouping() {
        assert_eq!(Money(0).format(), "0.00");
        assert_eq!(Money(5).format(), "0.05");
        assert_eq!(Money(3121100).format(), "31,211.00");
        assert_eq!(Money(-100000050).format(), "-1,000,000.50");
        assert_eq!(Money(123456789).format_compact(), "1.2M");
        assert_eq!(Money(1234500).format_compact(), "12.3k");
        assert_eq!(Money(99999).format_compact(), "999.99");
    }
}
