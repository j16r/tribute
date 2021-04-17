use std::error::Error;
use std::io;

use bigdecimal::{BigDecimal, Zero};
use chrono::{self, Datelike};

use crate::amount::Amount;
use crate::portfolio::{Portfolio, Trade, Kind};
use crate::symbol::Symbol;
use crate::types::DateTime;
use crate::types::{format_usd_amount, parse_amount};

#[derive(Debug, Deserialize)]
struct Record {
    id: String,
    market: String,
    token: String,
    amount: String,
    rate: String,
    usd_rate: String,
    usd_amount: String,
    created_at: String,
    provider: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Realization {
    pub description: String,
    pub acquired_when: Option<DateTime>,
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

        let market_components = market.split("-").collect::<Vec<_>>();
        let from_symbol : Symbol = market_components[0].parse().unwrap();
        let to_symbol : Symbol = market_components[1].parse().unwrap();

        if amount >= BigDecimal::zero() {
            portfolio.add_trade(&Trade{
                when: date_of_sale,
                kind: Kind::Trade{
                    offered: Amount{
                        amount: &rate * &amount.abs(),
                        symbol: to_symbol,
                    },
                    gained: Amount{
                        amount: amount.abs().clone(),
                        symbol: from_symbol,
                    },
                }
            });
        } else {
            portfolio.add_trade(&Trade{
                when: date_of_sale,
                kind: Kind::Trade{
                    offered: Amount{
                        amount: amount.abs().clone(),
                        symbol: from_symbol,
                    },
                    gained: Amount{
                        amount: &rate * &amount.abs(),
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
        let year_of_sale = realization.disposed_when.year();
        if year_of_sale != year as i32 {
            continue;
        }

        total_proceeds += &realization.proceeds;
        total_cost += &realization.cost_basis;
        total_gain += &realization.gain;

        writer.write_record(&[
            realization.description,
            realization.acquired_when.map_or("".to_string(), |d| d.format("%D").to_string()),
            realization.disposed_when.format("%D").to_string(),
            format_usd_amount(&realization.proceeds),
            format_usd_amount(&realization.cost_basis),
            format_usd_amount(&realization.gain),
        ])?;
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

    eprintln!("Portfolio:\n\n{:#?}\n", &portfolio);

    Ok(())
}
