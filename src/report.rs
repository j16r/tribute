use std::error::Error;
use std::io;

use bigdecimal::{BigDecimal, Zero};
use chrono::{self, Datelike};

use crate::amount::Amount;
use crate::portfolio::{Portfolio, Trade, Kind};
use crate::symbol::Symbol;
use crate::types::DateTime;
use crate::types::{format_type, format_usd_amount, parse_amount};

#[derive(Debug, Eq, PartialEq)]
pub struct Realization {
    pub description: String,
    pub acquired_when: DateTime,
    pub disposed_when: DateTime,
    pub proceeds: BigDecimal,
    pub cost_basis: BigDecimal,
    pub gain: BigDecimal,
}

pub fn report(year: u16, denomination: &Symbol) -> Result<(), Box<dyn Error>> {
    let mut portfolio = Portfolio::new();

    let mut rdr = csv::Reader::from_reader(io::stdin());

    // Load everything into memory
    let mut line_items = rdr
        .records()
        .map(|r| r.unwrap())
        .collect::<Vec<csv::StringRecord>>();

    // Sort by date earliest to latest
    line_items.sort_by(|a, b| a.get(7).unwrap().partial_cmp(b.get(7).unwrap()).unwrap());

    for line_item in line_items {
        let market = line_item.get(1).unwrap();

        let amount = parse_amount(line_item.get(3).unwrap()).unwrap();
        let date_of_sale = chrono::DateTime::parse_from_rfc3339(line_item.get(7).unwrap())
            .unwrap()
            .with_timezone(&chrono::Utc);

        let rate = parse_amount(line_item.get(5).unwrap()).unwrap();
        let year_of_sale = date_of_sale.year();
        if year_of_sale > year as i32 {
            break;
        }

        let market_components = market.split("-").collect::<Vec<_>>();
        let from_symbol : Symbol = market_components[0].parse().unwrap();
        let to_symbol : Symbol = market_components[1].parse().unwrap();

        if amount > BigDecimal::zero() {
            portfolio.add_trade(&Trade{
                when: date_of_sale,
                kind: Kind::Buy{
                    offered: Amount{
                        amount: amount.clone(),
                        symbol: from_symbol,
                    },
                    gained: Amount{
                        amount: &rate * &amount,
                        symbol: to_symbol,
                    },
                }
            });
        } else if amount == BigDecimal::zero() {
            // Ignore zero transactions, they exist, but aren't particularly useful
            continue;
        } else {
            portfolio.add_trade(&Trade{
                when: date_of_sale,
                kind: Kind::Sell{
                    offered: Amount{
                        amount: amount.clone(),
                        symbol: from_symbol,
                    },
                    gained: Amount{
                        amount: &rate * &amount,
                        symbol: to_symbol,
                    },
                }
            });
        }
    }

    let mut writer = csv::Writer::from_writer(io::stdout());

    writer.write_record(&[
        "Description of property",
        "Date acquired",
        "Date sold or disposed of",
        "Proceeds",
        "Cost basis",
        "Gain or (loss)",
    ])?;

    let (mut total_proceeds, mut total_cost, mut total_gain) =
        (BigDecimal::zero(), BigDecimal::zero(), BigDecimal::zero());
    for realization in portfolio.realizations(&denomination) {

    }

    writer.write_record(&[
        "Total",
        "",
        "",
        &format_usd_amount(&total_proceeds),
        &format_usd_amount(&total_cost),
        &format_usd_amount(&total_gain),
    ])?;

    writer.flush()?;
    Ok(())
}

fn format_description(sold_currency: &str, token: &str, bought: bool) -> String {
    format!(
        "{} {} via {} pair",
        sold_currency,
        format_type(bought),
        token
    )
}
