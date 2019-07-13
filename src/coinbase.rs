use std::error::Error;
use std::io;
use std::thread;
use std::time::Duration;

use chrono::NaiveDate;
use coinbase_rs::private::{Account, AccountHistory};
use coinbase_rs::{CBError, Private, Sync, MAIN_URL};
use uuid::Uuid;

struct ThrottledClient {
    client: Private<Sync>,
}

impl ThrottledClient {
    fn new(key: &str, secret: &str) -> ThrottledClient {
        println!("ThrottledClient::new");

        let client: Private<Sync> = Private::new(MAIN_URL, key, secret);
        ThrottledClient { client: client }
    }

    fn get_rate_at(&self, product_id: &str, time_of_trade: NaiveDate) -> Result<f64, Box<Error>> {
        thread::sleep(Duration::from_millis(350));

        //let market_at_trade = self
        //.client
        //.public()
        //.get_candles(&product_id, Some(time_of_trade), None, Granularity::M1)
        //.unwrap();

        //let mut rate = 0.0;
        //if let Some(candle) = market_at_trade.first() {
        //rate = (candle.1 + candle.2) / 2.0;
        //}
        //Ok(rate)

        Ok(0.0)
    }

    fn get_usd_rate(&self, product_id: &str, time_of_trade: NaiveDate) -> Result<f64, Box<Error>> {
        //if let Ok(rate) = self.get_rate_at(product_id, time_of_trade) {
        //if let Some(product_lhs) = product_rhs(product_id) {
        //if product_lhs == "USD" {
        //return Ok(rate);
        //}

        //let next_product_id = format!("{}-USD", product_lhs);

        //if let Ok(usd_rate) = self.get_rate_at(&next_product_id, time_of_trade) {
        //return Ok(rate * usd_rate);
        //}
        //}
        //}

        Ok(0.0)
    }

    fn get_accounts(&self) -> Result<Vec<Account>, CBError> {
        println!("ThrottledClient#get_accounts");

        self.client.accounts()
    }

    fn get_account_hist(&self, id: Uuid) -> Result<Vec<AccountHistory>, CBError> {
        //self.client.get_account_hist(id)
        Ok(Vec::new())
    }
}

pub fn export(key: &str, secret: &str) -> Result<(), Box<Error>> {
    let client = ThrottledClient::new(key, secret);

    let accounts = client.get_accounts().unwrap();
    dbg!(accounts);

    Ok(())
}
