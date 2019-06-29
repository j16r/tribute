#[macro_use]
extern crate serde_derive;
extern crate clap;
extern crate coinbase_pro_rs;
extern crate csv;
extern crate toml;
extern crate uuid;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process;
use std::thread;
use std::time::Duration;

use chrono::Datelike;
use clap::{App, SubCommand};
use coinbase_pro_rs::structs::private::*;
use coinbase_pro_rs::structs::public::*;
use coinbase_pro_rs::structs::DateTime;
use coinbase_pro_rs::{CBError, Private, Sync, MAIN_URL};
use uuid::Uuid;

#[derive(Deserialize)]
struct Config {
    exchanges: Vec<Exchange>,
}

#[derive(Deserialize)]
enum Exchange {
    Coinbase {
        key: String,
        secret: String,
        passphrase: String,
    },
}

fn product_rhs(product_id: &str) -> Option<String> {
    product_id
        .split("-")
        .collect::<Vec<&str>>()
        .get(1)
        .map(|v| v.clone().into())
}

#[test]
fn test_product_rhs() {
    assert_eq!(product_rhs("ETH-BTC"), Some("BTC".into()));
    assert_eq!(product_rhs("ETH"), None);
    assert_eq!(product_rhs(""), None);
}

struct ThrottledClient {
    client: Private<Sync>,
}

impl ThrottledClient {
    fn new(key: &str, secret: &str, passphrase: &str) -> ThrottledClient {
        let client: Private<Sync> = Private::new(MAIN_URL, key, secret, passphrase);
        ThrottledClient { client: client }
    }

    fn get_rate_at(&self, product_id: &str, time_of_trade: DateTime) -> Result<f64, Box<Error>> {
        thread::sleep(Duration::from_millis(350));

        let market_at_trade = self
            .client
            .public()
            .get_candles(&product_id, Some(time_of_trade), None, Granularity::M1)
            .unwrap();

        let mut rate = 0.0;
        if let Some(candle) = market_at_trade.first() {
            rate = (candle.1 + candle.2) / 2.0;
        }
        Ok(rate)
    }

    fn get_usd_rate(&self, product_id: &str, time_of_trade: DateTime) -> Result<f64, Box<Error>> {
        if let Ok(rate) = self.get_rate_at(product_id, time_of_trade) {
            if let Some(product_lhs) = product_rhs(product_id) {
                if product_lhs == "USD" {
                    return Ok(rate);
                }

                let next_product_id = format!("{}-USD", product_lhs);

                if let Ok(usd_rate) = self.get_rate_at(&next_product_id, time_of_trade) {
                    return Ok(rate * usd_rate);
                }
            }
        }

        Ok(0.0)
    }

    fn get_accounts(&self) -> Result<Vec<Account>, CBError> {
        self.client.get_accounts()
    }

    fn get_account_hist(&self, id: Uuid) -> Result<Vec<AccountHistory>, CBError> {
        self.client.get_account_hist(id)
    }
}

fn export_coinbase(key: &str, secret: &str, passphrase: &str) -> Result<(), Box<Error>> {
    let client = ThrottledClient::new(key, secret, passphrase);

    let mut writer = csv::Writer::from_writer(io::stdout());

    writer.write_record(&[
        "ID",
        "Market",
        "Token",
        "Amount",
        "Balance",
        "Rate",
        "USD Rate",
        "USD Amount",
        "Created At",
    ])?;

    let accounts = client.get_accounts().unwrap();

    for account in accounts {
        for trade in client.get_account_hist(account.id).unwrap() {
            if let AccountHistoryDetails::Match { product_id, .. } = trade.details {
                let time_of_trade = trade.created_at;

                let rate = client.get_rate_at(&product_id, time_of_trade)?;
                let usd_rate = client.get_usd_rate(&product_id, time_of_trade)?;
                let usd_amount = trade.amount * usd_rate;

                writer.write_record(&[
                    &trade.id.to_string(),
                    &product_id,
                    &account.currency,
                    &trade.amount.to_string(),
                    &trade.balance.to_string(),
                    &rate.to_string(),
                    &usd_rate.to_string(),
                    &usd_amount.to_string(),
                    &trade.created_at.to_rfc3339(),
                ])?;
            }
        }
    }

    writer.flush()?;
    Ok(())
}

fn export(exchange: &Exchange) -> Result<(), Box<Error>> {
    match exchange {
        Exchange::Coinbase {
            key,
            secret,
            passphrase,
        } => export_coinbase(key, secret, passphrase),
    }
}

fn load_config() -> io::Result<Config> {
    let mut input = String::new();
    File::open("config.toml").and_then(|mut f| f.read_to_string(&mut input))?;

    let config: Config = toml::from_str(&input).unwrap();
    Ok(config)
}

fn report(year: u16) -> Result<(), Box<Error>> {
    let mut accounts: HashMap<String, f64> = HashMap::new();

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

    // Load everything into memory sorted by date earliest to latest
    let mut line_items = rdr
        .records()
        .map(|r| r.unwrap())
        .collect::<Vec<csv::StringRecord>>();

    line_items.sort_by(|a, b| a.get(8).unwrap().partial_cmp(b.get(8).unwrap()).unwrap());

    let (mut total_proceeds, mut total_cost, mut total_gain) = (0.0, 0.0, 0.0);
    for line_item in line_items {
        if line_item.get(2).unwrap() == "USD" {
            continue;
        }

        let balance = accounts
            .entry(line_item.get(2).unwrap().into())
            .or_insert_with(|| 0.0);

        *balance += line_item.get(7).unwrap().parse::<f64>().unwrap();

        let date_of_sale = chrono::DateTime::parse_from_rfc3339(line_item.get(8).unwrap()).unwrap();
        let year_of_sale = date_of_sale.year();

        if year_of_sale == year as i32 {
            let proceeds = line_item.get(7).unwrap().parse::<f64>().unwrap();
            let cost_basis = 0.0;
            let gain = proceeds - cost_basis;

            total_proceeds += proceeds;
            total_cost += cost_basis;
            total_gain += gain;

            writer.write_record(&[
                &format!(
                    "{} sold via {} pair",
                    line_item.get(2).unwrap(),
                    line_item.get(1).unwrap()
                ),
                "",
                &date_of_sale.format("%D").to_string(),
                &format_amount(proceeds),
                &format_amount(cost_basis),
                &format_amount(gain),
            ])?;
        } else if year_of_sale > year as i32 {
            break;
        }
    }

    writer.write_record(&[
        "Total",
        "",
        "",
        &format_amount(total_proceeds),
        &format_amount(total_cost),
        &format_amount(total_gain),
    ])?;

    for (currency, balance) in accounts {
        eprintln!("Currency {:?} balance = {:?}", currency, balance);
    }

    writer.flush()?;
    Ok(())
}

fn format_amount(amount: f64) -> String {
    if amount < 0.0 {
        format!("$({:.2})", amount.abs())
    } else {
        format!("${:.2}", amount)
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
                println!("{}", err);
                process::exit(1);
            }
        }
    } else if let Some(_) = matches.subcommand_matches("report") {
        if let Err(err) = report(2018) {
            println!("{}", err);
            process::exit(1);
        }
    }
}
