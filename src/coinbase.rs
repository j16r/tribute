use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use bigdecimal::BigDecimal;
use coinbase_rs::private::{Account, Transaction as CBTransaction};
use coinbase_rs::{CBError, Private, Sync, MAIN_URL};
use uuid::Uuid;

use crate::types::Transaction;

struct ThrottledClient {
    client: Private<Sync>,
}

impl ThrottledClient {
    fn new(key: &str, secret: &str) -> ThrottledClient {
        let client: Private<Sync> = Private::new(MAIN_URL, key, secret);
        ThrottledClient { client: client }
    }

    fn get_accounts(&self) -> Result<Vec<Account>, CBError> {
        thread::sleep(Duration::from_millis(350));

        self.client.accounts()
    }

    fn get_account_hist(&self, id: Uuid) -> Result<Vec<CBTransaction>, CBError> {
        thread::sleep(Duration::from_millis(350));

        self.client.transactions(&id)
    }
}

pub fn transactions(key: &str, secret: &str) -> Result<Vec<Transaction>, Box<Error>> {
    let client = ThrottledClient::new(key, secret);

    let mut transactions = Vec::new();

    let mut balances: HashMap<String, Balance> = HashMap::new();
    let accounts = client.get_accounts().unwrap();

    for account in accounts {
        let code = account.currency.code.clone();
        if &code == "USD" {
            continue;
        }

        let balance = balances
            .entry(code.clone())
            .or_insert_with(|| Balance::new(&account.currency.code));

        if let Ok(id) = Uuid::from_str(&account.id) {
            for trade in client.get_account_hist(id).unwrap() {
                let usd_amount = trade.native_amount.amount;
                let trade_amount = trade.amount.amount;
                let usd_rate = &usd_amount / &trade_amount;

                let product_id = format!("{}-USD", code.clone());

                balance.add_trade(&trade_amount);

                transactions.push(Transaction {
                    id: trade.id.to_string(),
                    market: product_id,
                    token: code.clone(),
                    amount: trade_amount,
                    balance: balance.amount.clone(),
                    rate: BigDecimal::from(1),
                    usd_rate: usd_rate,
                    usd_amount: usd_amount,
                    created_at: trade.created_at,
                });
            }
        }
    }

    Ok(transactions)
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
