use std::error::Error;
use std::thread;
use std::time::Duration;

use bigdecimal::{BigDecimal, Zero};
use coinbase_pro_rs::structs::private::*;
use coinbase_pro_rs::structs::public::*;
use coinbase_pro_rs::{CBError, Private, Sync, MAIN_URL};
use uuid::Uuid;

use crate::types::{DateTime, Transaction};

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

    fn get_rate_at(
        &self,
        product_id: &str,
        time_of_trade: DateTime,
    ) -> Result<BigDecimal, Box<dyn Error>> {
        thread::sleep(Duration::from_millis(350));

        let start = Some(time_of_trade);
        let bucket = chrono::Duration::seconds(60);
        let end = Some(
            time_of_trade.checked_add_signed(bucket).unwrap(),
        );
        let market_at_trade = self
            .client
            .public()
            .get_candles(&product_id, start, end, Granularity::M1)
            .unwrap();

        let mut rate = BigDecimal::zero();
        if let Some(candle) = market_at_trade.first() {
            rate =
                (BigDecimal::from(candle.1) + BigDecimal::from(candle.2)) / BigDecimal::from(2.0);
        }
        Ok(rate)
    }

    fn get_usd_rate(
        &self,
        product_id: &str,
        time_of_trade: DateTime,
    ) -> Result<BigDecimal, Box<dyn Error>> {
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

        Ok(BigDecimal::zero())
    }

    fn get_accounts(&self) -> Result<Vec<Account>, CBError> {
        self.client.get_accounts()
    }

    fn get_account_hist(&self, id: Uuid) -> Result<Vec<AccountHistory>, CBError> {
        self.client.get_account_hist(id)
    }
}

pub fn transactions(
    key: &str,
    secret: &str,
    passphrase: &str,
) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let client = ThrottledClient::new(key, secret, passphrase);

    let mut transactions = Vec::new();

    let accounts = client.get_accounts().unwrap();
    for account in accounts {
        if account.currency == "USD" {
            continue;
        }

        for trade in client.get_account_hist(account.id).unwrap() {
            if let AccountHistoryDetails::Match { product_id, trade_id, .. } = trade.details {
                let time_of_trade = trade.created_at;

                let rate = client.get_rate_at(&product_id, time_of_trade)?;
                let usd_rate = client.get_usd_rate(&product_id, time_of_trade)?;
                let usd_amount = BigDecimal::from(trade.amount) * &usd_rate;

                let transaction = Transaction {
                    id: trade_id.to_string(),
                    market: product_id,
                    token: account.currency.clone(),
                    amount: BigDecimal::from(trade.amount),
                    rate: rate,
                    usd_rate: usd_rate,
                    usd_amount: usd_amount,
                    created_at: Some(time_of_trade),
                };
                transactions.push(transaction);
            }
        }
    }

    Ok(transactions)
}
