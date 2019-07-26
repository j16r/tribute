extern crate clap;
extern crate coinbase_pro_rs;
extern crate coinbase_rs;
extern crate csv;
extern crate futures;
#[macro_use]
extern crate serde_derive;
extern crate tempfile;
extern crate toml;
extern crate uuid;

mod coinbase;
mod coinbase_pro;
mod config;
mod report;
mod types;

use std::error::Error;
use std::process;

use clap::{App, SubCommand};

use config::{load_config, Exchange};

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

fn main() {
    let config = load_config(None).unwrap();

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
        if let Err(err) = report::report(2018) {
            eprintln!("{}", err);
            process::exit(1);
        }
    }
}
