use std::io;

use bigdecimal::{BigDecimal, Zero};
use chrono::{self, Datelike};
use anyhow::Result;

use crate::amount::Amount;
use crate::portfolio::{Portfolio, Trade, Kind};
use crate::symbol::Symbol;
use crate::types::DateTime;
use crate::types::{format_amount, format_usd_amount, deserialize_amount, deserialize_date};

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

pub enum Format {
    IRS1099B,
    TurboTax,
}

pub fn report(year: u16, denomination: &Symbol) -> Result<()> {
    let mut portfolio = Portfolio::new();

    let mut rdr = csv::Reader::from_reader(io::stdin());

    for result in rdr.deserialize() {
        let record: Record = result?;

        let market_components = record.market.split("-").collect::<Vec<_>>();
        let from_symbol : Symbol = market_components[0].parse().unwrap();
        let to_symbol : Symbol = market_components[1].parse().unwrap();

        let trade = if record.amount >= BigDecimal::zero() {
            Trade{
                when: record.created_at,
                kind: Kind::Trade{
                    offered: Amount{
                        amount: &record.rate * &record.amount.abs(),
                        symbol: to_symbol,
                    },
                    gained: Amount{
                        amount: record.amount.abs().clone(),
                        symbol: from_symbol,
                    },
                }
            }
        } else {
            Trade{
                when: record.created_at,
                kind: Kind::Trade{
                    offered: Amount{
                        amount: record.amount.abs().clone(),
                        symbol: from_symbol,
                    },
                    gained: Amount{
                        amount: &record.rate * &record.amount.abs(),
                        symbol: to_symbol,
                    },
                }
            }
        };
        portfolio.add_trade(&trade);
    }

    let mut writer = csv::Writer::from_writer(io::stdout());

    let format = Format::TurboTax;
    match format {
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
        },
        Format::TurboTax => {
            writer.write_record(&[
                "Amount",
                "Currency Name",
                "Purchase Date",
                "Date Sold",
                "Cost Basis",
                "Proceeds",
            ])?;

            for realization in portfolio.realizations(&denomination) {
                let year_of_sale = realization.disposed_when.year();
                if year_of_sale != year as i32 {
                    continue;
                }

                writer.write_record(&[
                    format_amount(&realization.amount),
                    realization.symbol.symbol(),
                    realization.acquired_when.map_or("".to_string(), |d| d.format("%D").to_string()),
                    realization.disposed_when.format("%D").to_string(),
                    format_usd_amount(&realization.cost_basis),
                    format_usd_amount(&realization.proceeds),
                ])?;
            }
        }
    }

    writer.flush()?;

    eprintln!("Portfolio:\n\n{:#?}\n", &portfolio);

    Ok(())
}
