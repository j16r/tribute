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
mod report;
mod types;

use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process;

use clap::{App, SubCommand};

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
        if let Err(err) = report::report(2018) {
            eprintln!("{}", err);
            process::exit(1);
        }
    }
}
