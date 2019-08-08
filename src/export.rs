use std::error::Error;
use std::io;

use crate::config::{Config, Exchange};
use crate::types::Transaction;
use crate::{coinbase, coinbase_pro};

pub fn export(config: &Config) -> Result<(), Box<Error>> {
    let mut exchange_transactions: Vec<Vec<Transaction>> = Vec::new();

    // Add the manual transactions
    exchange_transactions.push(config.transactions());

    // Add all exchange transactions
    for exchange in &config.exchanges {
        exchange_transactions.push(match exchange {
            Exchange::CoinbasePro {
                ref key,
                ref secret,
                ref passphrase,
            } => coinbase_pro::transactions(key, secret, passphrase)?,
            Exchange::Coinbase {
                ref key,
                ref secret,
            } => coinbase::transactions(key, secret)?,
        });
    }

    // Output
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

    let transactions = itertools::kmerge(exchange_transactions);

    //writer.write_record(&[
    //&trade.id.to_string(),
    //&product_id,
    //&account.currency,
    //&trade.amount.to_string(),
    //&trade.balance.to_string(),
    //&rate.to_string(),
    //&usd_rate.to_string(),
    //&usd_amount.to_string(),
    //&trade.created_at.to_rfc3339(),
    //])?;

    writer.flush()?;
    Ok(())
}
