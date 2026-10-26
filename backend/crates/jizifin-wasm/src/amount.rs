//! Belgian amount notation: `.` thousands, `,` decimals, e.g. `-2.105,68`.

/// A parsed statement amount in signed integer cents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EuroAmount(pub i64);

impl EuroAmount {
    /// Parses `raw`; `None` when it holds no number.
    pub fn parse(raw: &str) -> Option<Self> {
        let clean: String = raw.chars().filter(|c| !matches!(c, '\'' | '"' | '€' | '.') && !c.is_whitespace()).collect();
        let (negative, digits) = split_sign(&clean);
        let (whole, fraction) = digits.split_once(',').unwrap_or((digits, ""));
        let cents = whole_cents(whole)? + fraction_cents(fraction)?;
        Some(Self(if negative { -cents } else { cents }))
    }

    /// Absolute cents.
    pub fn magnitude(self) -> i64 {
        self.0.abs()
    }

    /// Money received.
    pub fn is_credit(self) -> bool {
        self.0 > 0
    }
}

fn split_sign(s: &str) -> (bool, &str) {
    match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    }
}

fn whole_cents(whole: &str) -> Option<i64> {
    match whole.is_empty() {
        true => Some(0),
        false => whole.parse::<i64>().ok()?.checked_mul(100),
    }
}

/// Two decimals, rounded half-up on the third.
fn fraction_cents(fraction: &str) -> Option<i64> {
    if !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let digit = |i: usize| fraction.as_bytes().get(i).map_or(0, |b| i64::from(b - b'0'));
    Some(digit(0) * 10 + digit(1) + i64::from(digit(2) >= 5))
}
