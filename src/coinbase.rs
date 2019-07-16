use std::error::Error;
use std::io;

use bigdecimal::BigDecimal;
use coinbase_rs::private::{Account, Transaction};
use coinbase_rs::{CBError, Private, Sync, MAIN_URL};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

use crate::types::{format_amount, format_usd_amount};

struct ThrottledClient {
    client: Private<Sync>,
}

impl ThrottledClient {
    fn new(key: &str, secret: &str) -> ThrottledClient {
        let client: Private<Sync> = Private::new(MAIN_URL, key, secret);
        ThrottledClient { client: client }
    }

    fn get_accounts(&self) -> Result<Vec<Account>, CBError> {
        self.client.accounts()
    }

    fn get_account_hist(&self, id: Uuid) -> Result<Vec<Transaction>, CBError> {
        self.client.transactions(&id)
    }
}

pub fn export(key: &str, secret: &str) -> Result<(), Box<Error>> {
    let client = ThrottledClient::new(key, secret);

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

    let mut balances: HashMap<String, Balance> = HashMap::new();
    let accounts = client.get_accounts().unwrap();

    for account in accounts {
        if account.currency.code == "USD" {
            continue;
        }

        let balance = balances
            .entry(account.currency.code.to_string())
            .or_insert_with(|| Balance::new(&account.currency.code));

        if let Ok(id) = Uuid::from_str(&account.id) {
            for trade in client.get_account_hist(id).unwrap() {
                let usd_amount = trade.native_amount.amount;
                let trade_amount = trade.amount.amount;
                let usd_rate = &usd_amount / &trade_amount;

                let product_id = format!("{}-USD", &account.currency.code);

                balance.add_trade(&trade_amount);

                writer.write_record(&[
                    &trade.id.to_string(),
                    &product_id,
                    &account.currency.code,
                    &trade_amount.to_string(),
                    &format_amount(&balance.amount),
                    &"1.0".to_string(),
                    &format_usd_amount(&usd_rate),
                    &format_usd_amount(&usd_amount),
                    &trade
                        .created_at
                        .map(|d| d.to_rfc3339())
                        .unwrap_or("".to_string()),
                ])?;
            }
        }
    }

    writer.flush()?;
    Ok(())
}

struct Balance {
    currency: String,
    amount: BigDecimal,
}

impl Balance {
    fn new(currency: &str) -> Balance {
        Balance {
            currency: currency.to_string(),
            amount: BigDecimal::from(0.0),
        }
    }

    fn add_trade(&mut self, amount: &BigDecimal) {
        self.amount += amount;
    }
}
