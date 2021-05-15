use std::cmp::Ordering;

use bigdecimal::{BigDecimal, ParseBigDecimalError, Zero, FromPrimitive};
use chrono::ParseError;
use regex::Regex;
use serde::de::{self, Deserializer};
use serde::Deserialize;

pub type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(Clone, Deserialize, Debug)]
pub struct Transaction {
    pub id: String,
    pub market: String,
    pub token: String,
    pub amount: BigDecimal,
    pub rate: BigDecimal,
    pub usd_rate: BigDecimal,
    pub usd_amount: BigDecimal,
    pub created_at: Option<DateTime>,
    pub provider: &'static str,
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.created_at == other.created_at
    }
}

impl Eq for Transaction {}

impl PartialOrd for Transaction {
    fn partial_cmp(&self, other: &Transaction) -> Option<Ordering> {
        if let Some(ref created_at) = self.created_at {
            if let Some(ref other_created_at) = other.created_at {
                return Some(created_at.cmp(other_created_at));
            }
        }

        Some(Ordering::Equal)
    }
}

impl Ord for Transaction {
    fn cmp(&self, other: &Transaction) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub fn format_usd_amount(amount: &BigDecimal) -> String {
    if amount < &BigDecimal::zero() {
        format!("(${:.4})", amount.abs())
    } else {
        format!("${:.4}", amount)
    }
}

pub fn format_amount(amount: &BigDecimal) -> String {
    if amount < &BigDecimal::zero() {
        format!("({:.4})", amount.abs())
    } else {
        format!("{:.4}", amount)
    }
}

pub fn deserialize_amount<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BigDecimal, D::Error> {
    let input = String::deserialize(deserializer)?;
    parse_amount(&input).map_err(|e| {
        de::Error::invalid_value(de::Unexpected::Str(&input), &"")
    })
}

pub fn deserialize_date<'de, D: Deserializer<'de>>(deserializer: D) -> Result<DateTime, D::Error> {
    let input = String::deserialize(deserializer)?;
    parse_date(&input).map_err(|e| {
        de::Error::invalid_value(de::Unexpected::Str(&input), &"")
    })
}

pub fn parse_amount(input: &str) -> Result<BigDecimal, ParseBigDecimalError> {
    let re = Regex::new(r"\A\((.*)\)\z").unwrap();
    if let Some(matches) = re.captures(input) {
        let amount = matches.get(1).unwrap().as_str();
        let result = amount.trim_start_matches('$').parse::<BigDecimal>()?;
        return Ok(result * BigDecimal::from_i32(-1).unwrap());
    }
    input.trim_start_matches('$').parse::<BigDecimal>()
}

pub fn parse_date(input: &str) -> Result<DateTime, ParseError> {
    Ok(chrono::DateTime::parse_from_rfc3339(input)?.with_timezone(&chrono::Utc))
}

#[test]
fn test_parse_amount() {
    assert_eq!(parse_amount("0"), Ok(BigDecimal::from_f32(0.0).unwrap()));
    assert_eq!(parse_amount("0.0"), Ok(BigDecimal::from_f32(0.0).unwrap()));
    assert_eq!(parse_amount("1.1"), Ok(BigDecimal::from_f32(1.1).unwrap()));
    assert_eq!(parse_amount("(1.0)"), Ok(BigDecimal::from_f32(-1.0).unwrap()));
    assert_eq!(parse_amount("$1.0"), Ok(BigDecimal::from_f32(1.0).unwrap()));
    assert_eq!(parse_amount("($3.1427)"), Ok(BigDecimal::from_f32(-3.1427).unwrap()));

    assert!(parse_amount("").is_err());
}
