use std::collections::HashMap;
use std::fmt;

use bigdecimal::BigDecimal;

use crate::types::DateTime;
use crate::wallet::{Sale, Wallet};

pub struct Portfolio {
    wallets: HashMap<String, Wallet>
}

impl Portfolio {
    pub fn new() -> Self {
        Portfolio{
            wallets: HashMap::new()
        }
    }

    pub fn add_lot(&mut self, token: &str, amount: &BigDecimal, unit_cost: &BigDecimal, date: DateTime) {
        let wallet = self.wallets.entry(token.into()).or_insert_with(|| Wallet::new(token));
        wallet.add_lot(amount, unit_cost, date);
    }

    pub fn sell(&mut self, token: &str, amount: &BigDecimal) -> Sale {
        let wallet = self.wallets.entry(token.into()).or_insert_with(|| Wallet::new(token));
        wallet.sell(amount)
    }
}

impl fmt::Debug for Portfolio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (currency, wallet) in self.wallets.iter() {
            write!(f, "Wallet {:} {:} tokens remain worth ${:} ({:}/{:})", currency, wallet.count(), wallet.cost_basis(), wallet.cumulative_bought, wallet.cumulative_sold)?;
        }
        Ok(())
    }
}
