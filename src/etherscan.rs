use std::error::Error;
use std::str::FromStr;

use bigdecimal::BigDecimal;

use crate::types::{DateTime, Transaction};
use chrono::prelude::*;

pub fn transactions(
    key: &str,
    accounts: &Vec<web3::types::H160>,
) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let mut transactions = Vec::new();

    for account in accounts.iter() {
        let txes = txlist(&key, &account).unwrap();

        for tx in txes.iter() {
            let timestamp = NaiveDateTime::parse_from_str(&tx.time_stamp, "%s").unwrap();
            let token_decimal: u32 = tx.token_decimal.parse().unwrap();
            let divisor = 10_u64.pow(token_decimal);
            let amount = BigDecimal::from_str(&tx.value).unwrap() / BigDecimal::from(divisor);
            let transaction = Transaction {
                id: tx.hash.clone(),
                market: "LINK-USD".to_string(),
                token: tx.token_symbol.clone(),
                amount: amount,
                rate: BigDecimal::from(0),
                usd_rate: BigDecimal::from(0),
                usd_amount: BigDecimal::from(0),
                created_at: Some(DateTime::from_utc(timestamp, chrono::Utc)),
            };
            transactions.push(transaction);
        }
    }

    Ok(transactions)
}

fn txlist(api_key: &str, account: &web3::types::H160) -> Result<Vec<Tx>, Box<dyn Error>> {
    let query = vec![
        "module=account",
        "action=tokentx",
        &format!("address={:#x}", account).to_string(),
        "startblock=0",
        "endblock=999999999",
        "sort=asc",
        &format!("apiKey={}", api_key).to_string(),
    ].join("&");
    let url = format!("https://api.etherscan.io/api?{}", query);
    let response = reqwest::blocking::get(&url).unwrap().json::<Response>().unwrap();

    Ok(response.result)
}

#[derive(Deserialize, Debug)]
struct Response {
    status: String,
    message: String,
    result: Vec<Tx>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Tx {
    block_number: String,
    time_stamp: String,
    hash: String,
    nonce: String,
    block_hash: String,
    from: String,
    contract_address: String,
    to: String,
    value: String,
    token_name: String,
    token_symbol: String,
    token_decimal: String,
    transaction_index: String,
    gas: String,
    gas_price: String,
    gas_used: String,
    cumulative_gas_used: String,
    input: String,
    confirmations: String,
}
