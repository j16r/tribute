#![feature(is_some_with)]
#![feature(drain_filter)]
#![feature(async_closure)]

extern crate clap;
extern crate coinbase_pro_rs;
extern crate coinbase_rs;
extern crate csv;
extern crate futures;
extern crate itertools;
#[macro_use]
extern crate serde_derive;
extern crate tempfile;
extern crate toml;
extern crate uuid;
extern crate web3;

mod amount;
mod coinbase;
mod coinbase_pro;
mod config;
mod ethereum;
mod etherscan;
mod export;
mod portfolio;
mod report;
mod symbol;
mod types;
mod wallet;

use std::process;

use clap::{arg, Command};

use crate::config::{load_config, ConfigError};
use crate::report::Format;

#[tokio::main]
async fn main() {
    let config = load_config(None).unwrap_or_else(|error| match error {
        ConfigError::TomlError(e) => {
            eprintln!("Error parsing config.toml: {}", e);
            process::exit(1);
        }
        e => {
            eprintln!("Error loading config.toml: {:?}", e);
            process::exit(1);
        }
    });

    let matches = Command::new("Tribute")
        .version("1.0")
        .author("John Barker <me@j16r.net>")
        .about("Generate tax records from various crypto exchanges")
        .subcommand(Command::new("export").about("Exports your exchange order history"))
        .subcommand(
            Command::new("report")
                .arg(arg!(--format <FORMAT>).help("Sets the output report format, one of: IRS1099B, TurboTax"))
                .about("Create a report from your order history"),
        )
        .get_matches();

    if matches.subcommand_matches("export").is_some() {
        if let Err(err) = export::export(&config).await {
            eprintln!("Error while exporting: {}", err);
            process::exit(1);
        }
    } else if let Some(subcommand) = matches.subcommand_matches("report") {
        let format: Option<Format> = subcommand
            .get_one::<String>("format")
            .map(|v| v.parse().unwrap())
            .or(config.report_format.clone());
        if let Err(err) = report::report(config.tax_year, &config.denomination(), &format) {
            eprintln!("Error while generating report: {}", err);
            process::exit(1);
        }
    }
}
