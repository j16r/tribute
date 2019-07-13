use std::error::Error;
use std::io;
use std::thread;
use std::time::Duration;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use coinbase_rs::private::{Account, Transaction};
use coinbase_rs::{CBError, Private, Sync, MAIN_URL};
use std::str::FromStr;
use uuid::Uuid;

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
        self.client.list_transactions(&id)
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

    let accounts = client.get_accounts().unwrap();

    for account in accounts {
        //dbg!(&account);
        if account.currency.code == "USD" {
            continue;
        }

        if let Ok(id) = Uuid::from_str(&account.id) {
            for trade in client.get_account_hist(id).unwrap() {
                let time_of_trade = trade.created_at;

                let mut usd_amount = trade.native_amount.amount;
                let trade_amount = trade.amount.amount;
                let usd_rate = &usd_amount / &trade_amount;

                let product_id = format!("{}-USD", account.currency.code);

                writer.write_record(&[
                    &trade.id.to_string(),
                    &product_id,
                    &account.currency.name,
                    &trade_amount.to_string(),
                    &"".to_string(), // trade.balance.amount.to_string(),
                    &"1.0".to_string(),
                    &format_amount(&usd_rate),
                    &usd_amount.to_string(),
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

fn format_amount(amount: &BigDecimal) -> String {
    if amount < &BigDecimal::from(0.0) {
        format!("$({:.2})", amount.abs())
    } else {
        format!("${:.2}", amount)
    }
}
