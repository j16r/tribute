#[macro_use]
extern crate serde_derive;
extern crate clap;
extern crate coinbase_pro_rs;
extern crate coinbase_rs;
extern crate csv;
extern crate futures;
extern crate toml;
extern crate uuid;

mod coinbase;
mod coinbase_pro;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process;

use bigdecimal::{BigDecimal, ParseBigDecimalError, Zero};
use chrono::Datelike;
use clap::{App, SubCommand};
use regex::Regex;

#[derive(Deserialize)]
struct Config {
    exchanges: Vec<Exchange>,
}

#[derive(Deserialize)]
enum Exchange {
    CoinbasePro {
        key: String,
        secret: String,
        passphrase: String,
    },
    Coinbase {
        key: String,
        secret: String,
    },
}

fn export(exchange: &Exchange) -> Result<(), Box<Error>> {
    match exchange {
        Exchange::CoinbasePro {
            key,
            secret,
            passphrase,
        } => coinbase_pro::export(key, secret, passphrase),
        Exchange::Coinbase { key, secret } => coinbase::export(key, secret),
    }
}

fn load_config() -> io::Result<Config> {
    let mut input = String::new();
    File::open("config.toml").and_then(|mut f| f.read_to_string(&mut input))?;

    let config: Config = toml::from_str(&input).unwrap();
    Ok(config)
}

struct Wallet {
    lots: Vec<Lot>,
}

impl Wallet {
    fn new() -> Wallet {
        Wallet { lots: Vec::new() }
    }

    // add_lot adds a purchase of some unit of an item, with a count and a total cost
    fn add_lot(&mut self, amount: &BigDecimal, unit_cost: &BigDecimal, date: chrono::NaiveDate) {
        self.lots.push(Lot {
            amount: amount.clone(),
            unit_cost: unit_cost.clone(),
            date_of_purchase: date,
        })
    }

    fn total_value(&self) -> BigDecimal {
        self.lots
            .iter()
            .map(|lot| &lot.amount * &lot.unit_cost)
            .sum()
    }

    fn sell(&mut self, amount: &BigDecimal) -> Sale {
        let mut date_of_purchase: Option<chrono::NaiveDate> = None;

        let mut total_cost = BigDecimal::zero();
        let mut lots_consumed = 0;
        let mut amount_to_consume = amount.clone();

        for lot in self.lots.iter_mut() {
            if date_of_purchase.is_none() {
                date_of_purchase = Some(lot.date_of_purchase);
            }

            if &amount_to_consume < &lot.amount {
                lot.amount -= &amount_to_consume;
                total_cost += &amount_to_consume * &lot.unit_cost;
                break;
            }

            total_cost += &lot.amount * &lot.unit_cost;
            amount_to_consume -= &lot.amount;
            lots_consumed += 1;
        }

        // Remove all consumed lots
        self.lots.drain(..lots_consumed);

        Sale {
            cost_basis: total_cost,
            date_of_purchase: date_of_purchase,
        }
    }
}

#[derive(Debug)]
struct Lot {
    // amount represents a count of items in a lot
    amount: BigDecimal,
    // unit_cost represents the cost of each item in a lot
    unit_cost: BigDecimal,
    // date_of_purchase represents the date at which the lot was acquired
    date_of_purchase: chrono::NaiveDate,
}

#[derive(Debug)]
struct Sale {
    cost_basis: BigDecimal,
    date_of_purchase: Option<chrono::NaiveDate>,
}

#[test]
fn test_wallet_sell() {
    let mut wallet = Wallet::new();

    wallet.add_lot(
        &BigDecimal::from(10.0),
        &BigDecimal::from(1.0),
        chrono::NaiveDate::from_yo(2018, 1),
    );
    wallet.add_lot(
        &BigDecimal::from(10.0),
        &BigDecimal::from(2.0),
        chrono::NaiveDate::from_yo(2018, 2),
    );
    wallet.add_lot(
        &BigDecimal::from(10.0),
        &BigDecimal::from(3.0),
        chrono::NaiveDate::from_yo(2018, 3),
    );

    let sale1 = wallet.sell(&BigDecimal::from(5.0));
    assert_eq!(sale1.cost_basis, BigDecimal::from(5.0));
    assert_eq!(
        sale1.date_of_purchase,
        Some(chrono::NaiveDate::from_yo(2018, 1))
    );

    let sale2 = wallet.sell(&BigDecimal::from(10.0));
    assert_eq!(sale2.cost_basis, BigDecimal::from(15.0));
    assert_eq!(
        sale2.date_of_purchase,
        Some(chrono::NaiveDate::from_yo(2018, 1))
    );

    let sale3 = wallet.sell(&BigDecimal::from(10.0));
    assert_eq!(sale3.cost_basis, BigDecimal::from(25.0));
    assert_eq!(
        sale3.date_of_purchase,
        Some(chrono::NaiveDate::from_yo(2018, 2))
    );
}

fn report(year: u16) -> Result<(), Box<Error>> {
    let mut wallets: HashMap<String, Wallet> = HashMap::new();

    let mut rdr = csv::Reader::from_reader(io::stdin());

    let mut writer = csv::Writer::from_writer(io::stdout());

    writer.write_record(&[
        "Description of property",
        "Date acquired",
        "Date sold or disposed of",
        "Proceeds",
        "Cost basis",
        "Gain or (loss)",
    ])?;

    // Load everything into memory
    let mut line_items = rdr
        .records()
        .map(|r| r.unwrap())
        .collect::<Vec<csv::StringRecord>>();

    // Sort by date earliest to latest
    line_items.sort_by(|a, b| a.get(8).unwrap().partial_cmp(b.get(8).unwrap()).unwrap());

    let (mut total_proceeds, mut total_cost, mut total_gain) =
        (BigDecimal::zero(), BigDecimal::zero(), BigDecimal::zero());
    for line_item in line_items {
        if line_item.get(2).unwrap() == "USD" {
            continue;
        }

        let wallet = wallets
            .entry(line_item.get(2).unwrap().into())
            .or_insert_with(|| Wallet::new());

        let amount = parse_amount(line_item.get(3).unwrap()).unwrap();
        let date_of_sale_tz =
            chrono::DateTime::parse_from_rfc3339(line_item.get(8).unwrap()).unwrap();
        let date_of_sale = date_of_sale_tz.naive_utc().date();

        let rate = parse_amount(line_item.get(6).unwrap()).unwrap();
        let year_of_sale = date_of_sale.year();
        if year_of_sale > year as i32 {
            break;
        }

        let bought = amount > BigDecimal::zero();
        if bought {
            wallet.add_lot(&amount, &rate, date_of_sale);
            continue;
        }

        let Sale {
            cost_basis,
            date_of_purchase,
        } = wallet.sell(&amount.abs());

        let proceeds = parse_amount(line_item.get(7).unwrap()).unwrap().abs();
        let gain = &proceeds - &cost_basis;

        // Only print sales for the specified year
        if year_of_sale == year as i32 {
            total_proceeds += &proceeds;
            total_cost += &cost_basis;
            total_gain += &gain;

            writer.write_record(&[
                &format!(
                    "{} {} via {} pair",
                    line_item.get(2).unwrap(),
                    format_type(bought),
                    line_item.get(1).unwrap(),
                ),
                &date_of_purchase
                    .map(|d| d.format("%D").to_string())
                    .unwrap_or("".to_string()),
                &date_of_sale.format("%D").to_string(),
                &format_usd_amount(&proceeds),
                &format_usd_amount(&cost_basis),
                &format_usd_amount(&gain),
            ])?;
        }
    }

    writer.write_record(&[
        "Total",
        "",
        "",
        &format_usd_amount(&total_proceeds),
        &format_usd_amount(&total_cost),
        &format_usd_amount(&total_gain),
    ])?;

    for (currency, wallet) in wallets {
        eprintln!("Wallet {:?} balance = {:?}", currency, wallet.total_value());
    }

    writer.flush()?;
    Ok(())
}

fn format_type(bought: bool) -> String {
    if bought {
        "bought".to_string()
    } else {
        "sold".to_string()
    }
}

fn parse_amount(input: &str) -> Result<BigDecimal, ParseBigDecimalError> {
    let re = Regex::new(r"\A\((.*)\)\z").unwrap();
    if let Some(matches) = re.captures(input) {
        let amount = matches.get(1).unwrap().as_str();
        let result = amount.trim_start_matches('$').parse::<BigDecimal>()?;
        return Ok(result * BigDecimal::from(-1));
    }
    input.trim_start_matches('$').parse::<BigDecimal>()
}

fn format_usd_amount(amount: &BigDecimal) -> String {
    if amount < &BigDecimal::from(0.0) {
        format!("(${:.4})", amount.abs())
    } else {
        format!("${:.4}", amount)
    }
}

fn main() {
    let config = load_config().unwrap();

    let matches = App::new("Tribute")
        .version("1.0")
        .author("John Barker <me@j16r.net>")
        .about("Generate tax records from various crypto exchanges")
        .subcommand(SubCommand::with_name("export").about("Exports your exchange order history"))
        .subcommand(
            SubCommand::with_name("report").about("Create a report from your order history"),
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("export") {
        for exchange in config.exchanges {
            if let Err(err) = export(&exchange) {
                eprintln!("{}", err);
                process::exit(1);
            }
        }
    } else if let Some(_) = matches.subcommand_matches("report") {
        if let Err(err) = report(2018) {
            eprintln!("{}", err);
            process::exit(1);
        }
    }
}
