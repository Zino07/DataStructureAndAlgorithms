//! The keypad calculator: `12.5+3×4−2÷4` with the usual precedence.
//!
//! Numbers are fixed-point (six decimals) in `i128`, so there is no floating
//! point drift; the result is rounded to the currency's decimals.

use crate::money::Money;

const SCALE: i128 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcError {
    Empty,
    Invalid,
    DivideByZero,
    TooLarge,
}

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => "amount is empty",
            Self::Invalid => "amount is not a number",
            Self::DivideByZero => "cannot divide by zero",
            Self::TooLarge => "amount is too large",
        })
    }
}

pub fn is_operator(c: char) -> bool {
    matches!(c, '+' | '-' | '−' | '×' | '*' | '÷' | '/' | 'x')
}

/// True when the text is a calculation rather than a plain number.
pub fn has_operator(expr: &str) -> bool {
    expr.char_indices()
        .any(|(i, c)| is_operator(c) && !(i == 0 && (c == '-' || c == '−')))
}

fn number(s: &str) -> Result<i128, CalcError> {
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    if whole.is_empty() && frac.is_empty() {
        return Err(CalcError::Invalid);
    }
    if !whole.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CalcError::Invalid);
    }
    let w: i128 = if whole.is_empty() {
        0
    } else {
        whole.parse().map_err(|_| CalcError::TooLarge)?
    };
    let mut f = frac.chars().take(6).collect::<String>();
    while f.len() < 6 {
        f.push('0');
    }
    let f: i128 = f.parse().map_err(|_| CalcError::Invalid)?;
    w.checked_mul(SCALE)
        .and_then(|v| v.checked_add(f))
        .ok_or(CalcError::TooLarge)
}

/// Evaluates the expression. A trailing operator is ignored (`"12+"` is 12).
pub fn eval(expr: &str) -> Result<Money, CalcError> {
    let text: String = expr
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    if text.is_empty() {
        return Err(CalcError::Empty);
    }
    // tokens: numbers and operators
    let mut terms: Vec<i128> = Vec::new(); // values after × ÷ are applied
    let mut ops: Vec<char> = Vec::new(); // + or − between terms
    let mut negative = false;
    let mut chars = text.chars().peekable();
    if matches!(chars.peek(), Some('-') | Some('−')) {
        negative = true;
        chars.next();
    }
    let mut buf = String::new();
    let mut current: Option<i128> = None;
    let mut pending: Option<char> = None; // × or ÷ waiting for its right side
    let finish = |buf: &mut String,
                  current: &mut Option<i128>,
                  pending: &mut Option<char>|
     -> Result<(), CalcError> {
        if buf.is_empty() {
            return Ok(());
        }
        let n = number(buf)?;
        buf.clear();
        *current = Some(match (*current, pending.take()) {
            (None, _) => n,
            (Some(a), Some('×')) => a.checked_mul(n).ok_or(CalcError::TooLarge)? / SCALE,
            (Some(a), Some('÷')) => {
                if n == 0 {
                    return Err(CalcError::DivideByZero);
                }
                let scaled = a.checked_mul(SCALE).ok_or(CalcError::TooLarge)?;
                (scaled + n / 2) / n
            }
            (Some(_), _) => return Err(CalcError::Invalid),
        });
        Ok(())
    };
    let mut first = true;
    for c in chars {
        match c {
            '0'..='9' | '.' => buf.push(c),
            '+' | '-' | '−' => {
                finish(&mut buf, &mut current, &mut pending)?;
                if let Some(mut v) = current.take() {
                    if first && negative {
                        v = -v;
                    }
                    first = false;
                    terms.push(v);
                    ops.push(if c == '+' { '+' } else { '-' });
                } else {
                    return Err(CalcError::Invalid);
                }
            }
            '×' | '*' | 'x' | '÷' | '/' => {
                finish(&mut buf, &mut current, &mut pending)?;
                if current.is_none() {
                    return Err(CalcError::Invalid);
                }
                pending = Some(if matches!(c, '×' | '*' | 'x') {
                    '×'
                } else {
                    '÷'
                });
            }
            _ => return Err(CalcError::Invalid),
        }
    }
    finish(&mut buf, &mut current, &mut pending)?;
    if let Some(mut v) = current.take() {
        if first && negative {
            v = -v;
        }
        terms.push(v);
    } else {
        // trailing + / −: drop the dangling operator
        ops.pop();
    }
    if terms.is_empty() {
        return Err(CalcError::Empty);
    }
    let mut total = terms[0];
    for (i, op) in ops.iter().enumerate() {
        let t = terms[i + 1];
        total = if *op == '+' {
            total.checked_add(t)
        } else {
            total.checked_sub(t)
        }
        .ok_or(CalcError::TooLarge)?;
    }
    let per = Money::minor_per_major() as i128;
    let scaled = total.checked_mul(per).ok_or(CalcError::TooLarge)?;
    let half = SCALE / 2;
    let minor = if scaled >= 0 {
        (scaled + half) / SCALE
    } else {
        -((-scaled + half) / SCALE)
    };
    i64::try_from(minor)
        .map(Money)
        .map_err(|_| CalcError::TooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> Result<i64, CalcError> {
        eval(s).map(|m| m.0)
    }

    #[test]
    fn precedence_and_rounding() {
        assert_eq!(ev("12.5+3×4"), Ok(2450));
        assert_eq!(ev("10−2×3"), Ok(400));
        assert_eq!(ev("100÷3"), Ok(3333));
        assert_eq!(ev("200÷3"), Ok(6667));
        assert_eq!(ev("2×3×4"), Ok(2400));
        assert_eq!(ev("1+2+3"), Ok(600));
        assert_eq!(ev("10-3-2"), Ok(500));
        assert_eq!(ev("0.1+0.2"), Ok(30));
    }

    #[test]
    fn plain_numbers_and_dangling_operators() {
        assert_eq!(ev("12.5"), Ok(1250));
        assert_eq!(ev("12+"), Ok(1200));
        assert_eq!(ev("12×"), Ok(1200));
        assert_eq!(ev("-5+2"), Ok(-300));
        assert_eq!(ev(""), Err(CalcError::Empty));
        assert_eq!(ev("+3"), Err(CalcError::Invalid));
        assert_eq!(ev("5÷0"), Err(CalcError::DivideByZero));
        assert_eq!(ev("5÷0.0"), Err(CalcError::DivideByZero));
    }

    #[test]
    fn operator_detection() {
        assert!(!has_operator("12.5"));
        assert!(!has_operator("-12.5"));
        assert!(has_operator("12+3"));
        assert!(has_operator("12×"));
    }
}
