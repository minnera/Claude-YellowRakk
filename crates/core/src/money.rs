//! Money stored as integer minor units, never as floats.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    #[error("currency code must be three ASCII letters, got {0:?}")]
    InvalidCurrency(String),
    #[error("cannot parse amount {0:?}")]
    InvalidAmount(String),
    #[error("amount {0:?} has more decimals than the currency allows")]
    TooPrecise(String),
}

/// An ISO 4217 currency code, always upper-case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Currency([u8; 3]);

impl Currency {
    /// Number of decimal places in the currency's minor unit (ISO 4217).
    #[must_use]
    pub fn exponent(self) -> u32 {
        match self.as_str() {
            "BIF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF"
            | "UGX" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
            "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
            _ => 2,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        // Construction guarantees ASCII, so this cannot fail.
        std::str::from_utf8(&self.0).unwrap_or("???")
    }
}

impl FromStr for Currency {
    type Err = MoneyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes: [u8; 3] = s
            .trim()
            .as_bytes()
            .try_into()
            .map_err(|_| MoneyError::InvalidCurrency(s.to_owned()))?;
        if !bytes.iter().all(u8::is_ascii_alphabetic) {
            return Err(MoneyError::InvalidCurrency(s.to_owned()));
        }
        Ok(Self(bytes.map(|b| b.to_ascii_uppercase())))
    }
}

impl TryFrom<String> for Currency {
    type Error = MoneyError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Currency> for String {
    fn from(value: Currency) -> Self {
        value.as_str().to_owned()
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An amount of money in a specific currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    /// Amount in the currency's minor unit (cents for EUR/USD).
    pub minor_units: i64,
    pub currency: Currency,
}

impl Money {
    /// Parses a decimal string such as `"19.99"` or `"1,299.00"`.
    ///
    /// Thousands separators (`,`, space, apostrophe) are dropped and `.` is
    /// the decimal point. Locale-specific formats should be normalised by
    /// the caller before reaching this point.
    pub fn parse(amount: &str, currency: Currency) -> Result<Self, MoneyError> {
        let invalid = || MoneyError::InvalidAmount(amount.to_owned());
        let cleaned: String = amount
            .trim()
            .chars()
            .filter(|c| !matches!(c, ',' | ' ' | '\'' | '\u{a0}'))
            .collect();
        let (negative, digits) = match cleaned.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, cleaned.as_str()),
        };
        let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
        if whole.is_empty() && fraction.is_empty() {
            return Err(invalid());
        }
        if !whole
            .chars()
            .chain(fraction.chars())
            .all(|c| c.is_ascii_digit())
        {
            return Err(invalid());
        }

        let exponent = currency.exponent();
        let fraction = fraction.trim_end_matches('0');
        let fraction_len = u32::try_from(fraction.len()).map_err(|_| invalid())?;
        if fraction_len > exponent {
            return Err(MoneyError::TooPrecise(amount.to_owned()));
        }

        let scale = 10_i64.pow(exponent);
        let whole: i64 = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| invalid())?
        };
        let fraction_value: i64 = if fraction.is_empty() {
            0
        } else {
            fraction.parse::<i64>().map_err(|_| invalid())? * 10_i64.pow(exponent - fraction_len)
        };
        let minor = whole
            .checked_mul(scale)
            .and_then(|w| w.checked_add(fraction_value))
            .ok_or_else(invalid)?;

        Ok(Self {
            minor_units: if negative { -minor } else { minor },
            currency,
        })
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let exponent = self.currency.exponent();
        let sign = if self.minor_units < 0 { "-" } else { "" };
        let abs = self.minor_units.unsigned_abs();
        if exponent == 0 {
            return write!(f, "{sign}{abs} {}", self.currency);
        }
        let scale = 10_u64.pow(exponent);
        let width = usize::try_from(exponent).map_err(|_| fmt::Error)?;
        write!(
            f,
            "{sign}{}.{:0width$} {}",
            abs / scale,
            abs % scale,
            self.currency
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cur(code: &str) -> Currency {
        code.parse().unwrap()
    }

    #[test]
    fn currency_is_normalised_and_validated() {
        assert_eq!(cur("eur").as_str(), "EUR");
        assert!("EURO".parse::<Currency>().is_err());
        assert!("E1R".parse::<Currency>().is_err());
    }

    #[test]
    fn parses_common_amounts() {
        let eur = cur("EUR");
        assert_eq!(Money::parse("19.99", eur).unwrap().minor_units, 1999);
        assert_eq!(Money::parse("19.9", eur).unwrap().minor_units, 1990);
        assert_eq!(Money::parse("19", eur).unwrap().minor_units, 1900);
        assert_eq!(Money::parse("1,299.00", eur).unwrap().minor_units, 129_900);
        assert_eq!(Money::parse(".5", eur).unwrap().minor_units, 50);
        assert_eq!(Money::parse("-3.10", eur).unwrap().minor_units, -310);
    }

    #[test]
    fn respects_currency_exponent() {
        assert_eq!(Money::parse("1500", cur("JPY")).unwrap().minor_units, 1500);
        assert_eq!(
            Money::parse("1.5", cur("JPY")),
            Err(MoneyError::TooPrecise("1.5".into()))
        );
        assert_eq!(Money::parse("1.234", cur("KWD")).unwrap().minor_units, 1234);
    }

    #[test]
    fn rejects_garbage() {
        let eur = cur("EUR");
        assert!(Money::parse("", eur).is_err());
        assert!(Money::parse("abc", eur).is_err());
        assert!(Money::parse("1.2.3", eur).is_err());
        assert!(Money::parse("99999999999999999999", eur).is_err());
    }

    #[test]
    fn displays_with_correct_decimals() {
        assert_eq!(
            Money::parse("19.5", cur("EUR")).unwrap().to_string(),
            "19.50 EUR"
        );
        assert_eq!(
            Money::parse("1500", cur("JPY")).unwrap().to_string(),
            "1500 JPY"
        );
        assert_eq!(
            Money::parse("-0.05", cur("USD")).unwrap().to_string(),
            "-0.05 USD"
        );
    }

    #[test]
    fn serde_round_trip() {
        let m = Money::parse("4.20", cur("usd")).unwrap();
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"minor_units":420,"currency":"USD"}"#);
        assert_eq!(serde_json::from_str::<Money>(&json).unwrap(), m);
    }
}
