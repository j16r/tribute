use std::io;
use std::str::FromStr;

use anyhow::Result;
use bigdecimal::{BigDecimal, Zero};
use chrono::{self, Datelike};

use crate::amount::Amount;
use crate::portfolio::{Kind, Portfolio, Trade};
use crate::symbol::Symbol;
use crate::types::DateTime;
use crate::types::{
    deserialize_amount, deserialize_date, format_amount, format_amount_for_turbotax,
    format_usd_amount,
};

#[derive(Debug, Deserialize)]
struct Record {
    #[serde(alias = "ID")]
    id: String,
    #[serde(alias = "Market")]
    market: String,
    #[serde(alias = "Token")]
    token: String,
    #[serde(alias = "Amount", deserialize_with = "deserialize_amount")]
    amount: BigDecimal,
    #[serde(alias = "Rate")]
    rate: BigDecimal,
    #[serde(alias = "USD Rate", deserialize_with = "deserialize_amount")]
    usd_rate: BigDecimal,
    #[serde(alias = "USD Amount", deserialize_with = "deserialize_amount")]
    usd_amount: BigDecimal,
    #[serde(alias = "Created At", deserialize_with = "deserialize_date")]
    created_at: DateTime,
    #[serde(alias = "Provider")]
    provider: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Realization {
    pub amount: BigDecimal,
    pub description: String,
    pub symbol: Symbol,
    pub acquired_when: Option<DateTime>,
    pub disposed_when: DateTime,
    pub proceeds: BigDecimal,
    pub cost_basis: BigDecimal,
    pub gain: BigDecimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFormatError {}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub enum Format {
    #[serde(alias = "irs", alias = "irs1099b")]
    IRS1099B,
    #[serde(alias = "turbotax")]
    TurboTax,
}

impl FromStr for Format {
    type Err = ParseFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "irs" | "irs1099b" => Ok(Format::IRS1099B),
            "turbotax" => Ok(Format::TurboTax),
            _ => Err(ParseFormatError {}),
        }
    }
}

pub fn report(year: u16, denomination: &Symbol, format: &Option<Format>) -> Result<()> {
    let mut portfolio = Portfolio::new();

    let mut rdr = csv::Reader::from_reader(io::stdin());

    for result in rdr.deserialize() {
        let record: Record = result?;

        let market_components = record.market.split('-').collect::<Vec<_>>();
        let from_symbol: Symbol = market_components[0].parse().unwrap();
        let to_symbol: Symbol = market_components[1].parse().unwrap();

        let trade = if record.amount >= BigDecimal::zero() {
            Trade {
                when: record.created_at,
                kind: Kind::Trade {
                    offered: Amount {
                        amount: &record.rate * &record.amount.abs(),
                        symbol: to_symbol,
                    },
                    gained: Amount {
                        amount: record.amount.abs().clone(),
                        symbol: from_symbol,
                    },
                },
            }
        } else {
            Trade {
                when: record.created_at,
                kind: Kind::Trade {
                    offered: Amount {
                        amount: record.amount.abs().clone(),
                        symbol: from_symbol,
                    },
                    gained: Amount {
                        amount: &record.rate * &record.amount.abs(),
                        symbol: to_symbol,
                    },
                },
            }
        };
        portfolio.add_trade(&trade);
    }

    let mut writer = csv::Writer::from_writer(io::stdout());

    match format.as_ref().unwrap_or(&Format::IRS1099B) {
        Format::IRS1099B => {
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
            for realization in portfolio.realizations(denomination) {
                let year_of_sale = realization.disposed_when.year();
                if year_of_sale != year as i32 {
                    continue;
                }

                total_proceeds += &realization.proceeds;
                total_cost += &realization.cost_basis;
                total_gain += &realization.gain;

                writer.write_record(&[
                    realization.description,
                    realization
                        .acquired_when
                        .map_or("".to_string(), |d| d.format("%D").to_string()),
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
        }
        Format::TurboTax => {
            writer.write_record(&[
                "Amount",
                "Currency Name",
                "Purchase Date",
                "Date Sold",
                "Cost Basis",
                "Proceeds",
            ])?;

            for realization in portfolio.realizations(denomination) {
                let year_of_sale = realization.disposed_when.year();
                if year_of_sale != year as i32 {
                    continue;
                }

                writer.write_record(&[
                    format_amount_for_turbotax(&realization.amount),
                    realization.symbol.symbol(),
                    realization
                        .acquired_when
                        .map_or("".to_string(), |d| d.format("%D %R").to_string()),
                    realization.disposed_when.format("%D %R").to_string(),
                    format_amount(&realization.cost_basis),
                    format_amount(&realization.proceeds),
                ])?;
            }
        }
    }

    writer.flush()?;

    eprintln!("Portfolio:\n\n{:#?}\n", &portfolio);

    Ok(())
}
