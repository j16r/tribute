use std::cmp::Ordering;

use bigdecimal::{BigDecimal, ParseBigDecimalError, Zero};
use regex::Regex;

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct Transaction {
    pub id: String,
    pub market: String,
    pub token: String,
    pub amount: BigDecimal,
    pub balance: BigDecimal,
    pub rate: BigDecimal,
    pub usd_rate: BigDecimal,
    pub usd_amount: BigDecimal,
    pub created_at: Option<chrono::NaiveDateTime>,
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

pub fn format_usd_amount(amount: &BigDecimal) -> String {
    if amount < &BigDecimal::zero() {
        format!("(${:.4})", amount.abs())
    } else {
        format!("${:.4}", amount)
    }
}

pub fn format_type(bought: bool) -> String {
    if bought {
        "bought".to_string()
    } else {
        "sold".to_string()
    }
}

pub fn parse_amount(input: &str) -> Result<BigDecimal, ParseBigDecimalError> {
    let re = Regex::new(r"\A\((.*)\)\z").unwrap();
    if let Some(matches) = re.captures(input) {
        let amount = matches.get(1).unwrap().as_str();
        let result = amount.trim_start_matches('$').parse::<BigDecimal>()?;
        return Ok(result * BigDecimal::from(-1));
    }
    input.trim_start_matches('$').parse::<BigDecimal>()
}

#[test]
fn test_parse_amount() {
    assert_eq!(parse_amount("0"), Ok(BigDecimal::from(0)));
    assert_eq!(parse_amount("0.0"), Ok(BigDecimal::from(0)));
    assert_eq!(parse_amount("1.1"), Ok(BigDecimal::from(1.1)));
    assert_eq!(parse_amount("(1.0)"), Ok(BigDecimal::from(-1)));
    assert_eq!(parse_amount("$1.0"), Ok(BigDecimal::from(1)));
    assert_eq!(parse_amount("($3.1427)"), Ok(BigDecimal::from(-3.1427)));

    assert!(parse_amount("").is_err());
}
