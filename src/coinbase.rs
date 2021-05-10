use std::str::FromStr;

use anyhow::Result;
use bigdecimal::BigDecimal;
use coinbase_rs::{Private, MAIN_URL};
use uuid::Uuid;
use futures::stream::StreamExt;
use futures::pin_mut;

use crate::types::Transaction;

const PROVIDER: &str = "coinbase";

pub async fn transactions(key: &str, secret: &str) -> Result<Vec<Transaction>> {
    let client: Private = Private::new(MAIN_URL, key, secret);

    let mut transactions = Vec::new();

    let accounts_stream = client.accounts();
    pin_mut!(accounts_stream);

    while let Some(accounts_result) = accounts_stream.next().await {
        for account in accounts_result? {
            if let Ok(ref id) = Uuid::from_str(&account.id) {
                let transactions_stream = client.transactions(id);
                pin_mut!(transactions_stream);

                let code = account.currency.code;
                while let Some(transactions_result) = transactions_stream.next().await {
                    for trade in transactions_result? {
                        if trade.r#type != "buy" && trade.r#type != "sell" {
                            continue
                        }

                        if code == trade.native_amount.currency {
                            continue
                        }

                        let usd_amount = trade.native_amount.amount;
                        let trade_amount = trade.amount.amount;
                        let usd_rate = &usd_amount / &trade_amount;

                        let product_id = format!("{}-{}", &code, &trade.native_amount.currency);
                        transactions.push(Transaction {
                            id: trade.id.to_string(),
                            market: product_id,
                            token: code.clone(),
                            amount: trade_amount,
                            rate: BigDecimal::from(1),
                            usd_rate,
                            usd_amount,
                            created_at: trade.created_at,
                            provider: PROVIDER,
                        });
                    }
                }
            }
        }
    }

    Ok(transactions)
}
