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

mod coinbase;
mod coinbase_pro;
mod config;
mod ethereum;
mod etherscan;
mod export;
mod report;
mod types;

use std::process;

use clap::{App, SubCommand};

use config::load_config;

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
        if let Err(err) = export::export(&config) {
            eprintln!("{}", err);
            process::exit(1);
        }
    } else if let Some(_) = matches.subcommand_matches("report") {
        if let Err(err) = report::report(config.tax_year) {
            eprintln!("{}", err);
            process::exit(1);
        }
    }
}
