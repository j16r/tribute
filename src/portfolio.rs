use std::collections::HashMap;
use std::fmt;

use bigdecimal::{BigDecimal, Zero};

use crate::amount::Amount;
use crate::symbol::Symbol;
use crate::types::DateTime;
use crate::wallet::Wallet;
use crate::report::Realization;

pub struct Portfolio {
    wallets: HashMap<Symbol, Wallet>,
    trades: Vec<Trade>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Kind {
    Buy{
        offered: Amount,
        gained: Amount,
    },
    Sell{
        offered: Amount,
        gained: Amount,
    },
    StakingReward{
        symbol: Symbol,
        amount: BigDecimal,
    },
    Airdrop{
        symbol: Symbol,
        amount: BigDecimal,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    pub when: DateTime,
    pub kind: Kind,
}

impl Portfolio {
    pub fn new() -> Self {
        Portfolio{
            wallets: HashMap::new(),
            trades: Vec::new(),
        }
    }

    pub fn add_trade(&mut self, trade: &Trade) {
        match trade.kind {
            Kind::Buy{ref offered, ref gained} => self.buy(trade.when, offered, gained),
            Kind::Sell{ref offered, ref gained} => self.sell(trade.when, offered, gained),
            _ => panic!("not implemented yet"),
        };
        self.trades.push(trade.clone());
    }

    fn buy(&mut self, date: DateTime, offered: &Amount, gained: &Amount) {
        let wallet = self.wallets
            .entry(gained.symbol)
            .or_insert_with(|| Wallet::new(&gained.symbol));
        wallet.add_lot(&gained.amount, &gained.amount, date);
    }

    fn sell(&mut self, date: DateTime, offered: &Amount, gained: &Amount) {
        let wallet = self.wallets
            .entry(gained.symbol)
            .or_insert_with(|| Wallet::new(&gained.symbol));
        wallet.sell(&offered.amount);
    }

    pub fn realizations(&self, denomination: &Symbol) -> Vec<Realization> {
        dbg!(&denomination);
        let mut realizations: Vec<Realization> = Vec::new();

        let mut working_trades = self.trades.clone();
        for trade in &self.trades {
            if let Trade{ref when, kind: Kind::Sell{ offered, gained } } = trade {
                // TODO: backtrack through trades

                let realization = Realization{
                    description: format!("{} sold via {}-{} pair", offered.symbol.symbol(), offered.symbol.symbol(), denomination.symbol()),
                    acquired_when: when.clone(),
                    disposed_when: when.clone(),
                    proceeds: BigDecimal::zero(),
                    cost_basis: BigDecimal::zero(),
                    gain: BigDecimal::zero(),
                };
                realizations.push(realization);
            }
        }

        realizations
    }

    // find all the trades that were used to make up a specific trade
    fn find_trades(&self, mut trades: Vec<Trade>, offered: &Amount, gained: &Amount) -> Vec<Trade> {

        // So we just made a BTC - USD trade, but we bought the BTC with ETH, we need to find the
        // ETH trades that make up this trade

        let matched_trades = trades.drain_filter(|proposed_trade| {
            match &proposed_trade.kind {
                Kind::Sell{ offered: proposed_offer, gained: proposed_gain } if proposed_gain.symbol == gained.symbol => {
                    true
                }
                _ => false
            }
        });


        // for proposed_trade in trades.iter_mut() {
        //     match &proposed_trade .kind {
        //         Kind::Sell{ offered: proposed_offer, gained: proposed_gain } if proposed_gain.symbol == gained.symbol => {
        //         }
        //         _ => continue,
        //     }
        // }

        Vec::new()
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

#[cfg(test)]
mod test {
    use chrono::offset::TimeZone;
    use chrono::Utc;

    use bigdecimal::FromPrimitive;

    use crate::symbol::{Symbol, Fiat, Crypto, USD};
    use crate::{usd, usdt, eth, btc};

    use super::*;

    #[test]
    fn test_portfolio_sell() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Buy{
                offered: usd!(100),
                gained: usdt!(33.3333),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Buy{
                offered: usd!(100),
                gained: usdt!(33.3333),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Buy{
                offered: usd!(100),
                gained: usdt!(33.3333),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Buy{
                offered: usdt!(47),
                gained: eth!(2),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Buy{
                offered: eth!(2),
                gained: btc!(0.1),
            }
        });

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 2).and_hms(0, 0, 0),
            kind: Kind::Sell{
                offered: btc!(0.1),
                gained: usd!(4000),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
                disposed_when: Utc.ymd(2018, 1, 2).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(47.).unwrap(),
                cost_basis: BigDecimal::from_f32(47.).unwrap(),
                gain: BigDecimal::from_f32(3953.).unwrap(),
            }
        ]);

    }
}
