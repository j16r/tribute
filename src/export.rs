use crate::itertools::Itertools;
use std::error::Error;
use std::io;

use crate::config::{Config, Exchange};
use crate::types::{format_amount, format_usd_amount, Transaction};
use crate::{coinbase, coinbase_pro, ethereum, etherscan};

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

pub async fn export(config: &Config) -> Result<(), Box<dyn Error>> {
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
            } => coinbase_pro::transactions(key, secret, passphrase, config.denomination()).await?,
            Exchange::Coinbase {
                ref key,
                ref secret,
            } => coinbase::transactions(key, secret).await?,
            Exchange::Ethereum { ref url } => {
                if let Some(ref a) = config.accounts {
                    ethereum::transactions(url, a)?
                } else {
                    eprintln!("Specified ethereum configuration with no accounts");
                    vec![]
                }
            },
            Exchange::Etherscan {
                ref key,
            } => {
                if let Some(ref a) = config.accounts {
                    etherscan::transactions(key, a).await?
                } else {
                    eprintln!("Specified etherscan configuration with no accounts");
                    vec![]
                }
            },
        });
    }

    // Output
    let mut writer = csv::Writer::from_writer(io::stdout());

    writer.write_record(&[
        "ID",
        "Market",
        "Token",
        "Amount",
        "Rate",
        "USD Rate",
        "USD Amount",
        "Created At",
        "Provider",
    ])?;

    // This will likely need to hold the entire set of transactions in memory, so watch out...
    let transactions = itertools::kmerge(exchange_transactions).sorted();

    for transaction in transactions {
        writer.write_record(&[
            &transaction.id,
            &transaction.market,
            &transaction.token,
            &format_amount(&transaction.amount),
            &format_amount(&transaction.rate),
            &format_usd_amount(&transaction.usd_rate),
            &format_usd_amount(&transaction.usd_amount),
            &transaction
                .created_at
                .map_or("".to_string(), |t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            transaction.provider,
        ])?;
    }

    writer.flush()?;
    Ok(())
}
