use std::collections::HashSet;
use std::error::Error;
use std::thread;
use std::time::Duration;

use bigdecimal::{BigDecimal, FromPrimitive, Zero};
use coinbase_pro_rs::Uuid;
use coinbase_pro_rs::structs::private::*;
use coinbase_pro_rs::structs::public::*;
use coinbase_pro_rs::{ASync, CBError, Private, MAIN_URL};
use futures::pin_mut;
use futures::stream::{Stream, StreamExt};

use crate::symbol::Symbol;
use crate::types::{DateTime, Transaction};

const PROVIDER: &str = "coinbase-pro";

fn product_rhs(product_id: &str) -> Option<String> {
    product_id
        .split('-')
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
    client: Private<ASync>,
}

impl ThrottledClient {
    fn new(key: &str, secret: &str, passphrase: &str) -> ThrottledClient {
        let client: Private<ASync> = Private::new(MAIN_URL, key, secret, passphrase);
        ThrottledClient { client }
    }

    async fn get_rate_at(
        &self,
        product_id: &str,
        time_of_trade: DateTime,
    ) -> Result<BigDecimal, Box<dyn Error>> {
        thread::sleep(Duration::from_millis(350));

        let start = Some(time_of_trade);
        let bucket = chrono::Duration::seconds(60);
        let end = Some(time_of_trade.checked_add_signed(bucket).unwrap());
        let market_at_trade = self
            .client
            .public()
            .get_candles(product_id, start, end, Granularity::M1)
            .await
            .unwrap();

        let mut rate = BigDecimal::zero();
        if let Some(candle) = market_at_trade.first() {
            rate = (BigDecimal::from_f64(candle.1).unwrap()
                + BigDecimal::from_f64(candle.2).unwrap())
                / BigDecimal::from_f64(2.0).unwrap();
        }
        Ok(rate)
    }

    async fn get_denomination_rate(
        &self,
        product_id: &str,
        time_of_trade: DateTime,
        denomination: Symbol,
    ) -> Result<BigDecimal, Box<dyn Error>> {
        thread::sleep(Duration::from_millis(350));

        if let Ok(token_rate) = self.get_rate_at(product_id, time_of_trade).await {
            if let Some(product_lhs) = product_rhs(product_id) {
                if product_lhs == denomination.symbol() {
                    return Ok(token_rate);
                }

                let next_product_id = format!("{}-{}", product_lhs, denomination.symbol());

                if let Ok(denomination_rate) =
                    self.get_rate_at(&next_product_id, time_of_trade).await
                {
                    return Ok(token_rate * denomination_rate);
                }
            }
        }

        Ok(BigDecimal::zero())
    }

    async fn get_accounts(&self) -> Result<Vec<Account>, CBError> {
        thread::sleep(Duration::from_millis(350));

        self.client.get_accounts().await
    }

    fn get_account_hist_stream<'a>(
        &'a self,
        id: Uuid,
    ) -> impl Stream<Item = Result<Vec<AccountHistory>, CBError>> + 'a {
        thread::sleep(Duration::from_millis(350));

        self.client.get_account_hist_stream(id)
    }
}

pub async fn transactions(
    key: &str,
    secret: &str,
    passphrase: &str,
    denomination: Symbol,
) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let client = ThrottledClient::new(key, secret, passphrase);

    let mut observed_transactions = HashSet::new();
    let mut transactions = Vec::new();

    let accounts = client.get_accounts().await.unwrap();
    for account in accounts {
        if account.currency == denomination.symbol() {
            continue;
        }

        let account_hist_stream = client.get_account_hist_stream(account.id);
        pin_mut!(account_hist_stream);

        while let Some(account_hist_result) = account_hist_stream.next().await {
            for trade in account_hist_result? {
                if let AccountHistoryDetails::Match {
                    product_id,
                    trade_id,
                    ..
                } = trade.details
                {
                    if observed_transactions.contains(&trade_id) {
                        continue;
                    }
                    observed_transactions.insert(trade_id);

                    let time_of_trade = trade.created_at;

                    let rate = client.get_rate_at(&product_id, time_of_trade).await?;
                    let denomination_rate = client
                        .get_denomination_rate(&product_id, time_of_trade, denomination)
                        .await?;
                    let amount = BigDecimal::from_f64(trade.amount).unwrap() * &denomination_rate;

                    let transaction = Transaction {
                        id: trade_id.to_string(),
                        market: product_id,
                        token: account.currency.clone(),
                        amount: BigDecimal::from_f64(trade.amount).unwrap(),
                        rate,
                        usd_rate: denomination_rate,
                        usd_amount: amount,
                        created_at: Some(time_of_trade),
                        provider: PROVIDER,
                    };
                    transactions.push(transaction);
                }
            }
        }
    }

    Ok(transactions)
}
