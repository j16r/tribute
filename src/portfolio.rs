use std::collections::HashMap;
use std::fmt;

use bigdecimal::{BigDecimal, FromPrimitive, Zero};

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
    Trade{
        offered: Amount,
        gained: Amount,
    },
    // StakingReward{
    //     symbol: Symbol,
    //     amount: BigDecimal,
    // },
    // Airdrop{
    //     symbol: Symbol,
    //     amount: BigDecimal,
    // }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    pub when: DateTime,
    pub kind: Kind,
}

#[derive(Clone, Debug)]
pub struct Sale {
    when: DateTime,
    original_symbol: Symbol,
    offered: Amount,
    gained: Amount,
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
            Kind::Trade{ref offered, ref gained} => {
                self.buy(trade.when, offered, gained);
                self.sell(trade.when, gained, offered);
            }
        };
        self.trades.push(trade.clone());
    }

    fn buy(&mut self, date: DateTime, _offered: &Amount, gained: &Amount) {
        let wallet = self.wallets
            .entry(gained.symbol)
            .or_insert_with(|| Wallet::new(&gained.symbol));
        wallet.add_lot(&gained.amount, &gained.amount, date);
    }

    fn sell(&mut self, _date: DateTime, offered: &Amount, gained: &Amount) {
        let wallet = self.wallets
            .entry(offered.symbol)
            .or_insert_with(|| Wallet::new(&gained.symbol));
        wallet.sell(&offered.amount);
    }

    pub fn realizations(&self, denomination: &Symbol) -> Vec<Realization> {
        let mut trades = self.trades.clone();

        // First step, remove all the sales to the final denomination into their own collection,
        // e.g: BTC->USD
        let mut liquidations: Vec<Sale> = trades.drain_filter(|trade| {
            let Trade{ kind: Kind::Trade{ ref gained, .. } , .. } = trade;
            &gained.symbol == denomination
        }).map(|trade| {
            let Trade{ ref when, kind: Kind::Trade{ ref offered, ref gained } } = trade;
            Sale{
                when: when.clone(),
                original_symbol: offered.symbol.clone(),
                offered: offered.clone(),
                gained: gained.clone(),
            }
        }).collect();

        // This is a list of sales that have had their full cost basis traced
        let mut realizations: Vec<Realization> = Vec::new();

        loop {
            let mut processed = 0;
            let mut processed_liquidations = Vec::<Sale>::new();

            for liquidation in &liquidations {
                eprintln!("Searching for trade to satisfy liquidation of {:#?}", &liquidation);

                let mut realization_processed = false;
                let rhs_offered = &liquidation.offered;
                let rhs_gained = &liquidation.gained;

                let mut processed_trades: Vec<Trade> = Vec::new();
                for trade in &trades {
                    if realization_processed {
                        processed_trades.push(trade.clone());
                        continue;
                    }

                    match trade {
                        // So this matches a purchase using the taxable denomination, e.g: ETH
                        // purchased with USD, whenever one of these are found, we can turn a Sale
                        // into a Realization
                        Trade{ ref when, kind: Kind::Trade{ offered: ref lhs_offered, gained: ref lhs_gained } } if &lhs_offered.symbol == denomination && lhs_gained.symbol == rhs_offered.symbol => {
                            dbg!(&when);
                            dbg!(&lhs_offered);
                            dbg!(&lhs_gained);

                            processed += 1;
                            realization_processed = true;

                            let description =
                                format!("{} sold via {}-{} pair", liquidation.original_symbol.symbol(), liquidation.original_symbol.symbol(), denomination.symbol());
                            if lhs_gained.amount <= rhs_offered.amount {

                                eprintln!("Found lot of {} smaller than current offering {}", lhs_gained.amount, rhs_offered.amount);

                                let divisor = &lhs_gained.amount / &rhs_offered.amount;
                                let proceeds = (&rhs_gained.amount * &divisor).clone();
                                let cost_basis = &lhs_offered.amount;
                                let gain = &proceeds - cost_basis;

                                // This trade is larger, so the trade needs to be split
                                let realization = Realization{
                                    description,
                                    acquired_when: Some(when.clone()),
                                    disposed_when: liquidation.when.clone(),
                                    proceeds: proceeds.clone(),
                                    cost_basis: cost_basis.clone(),
                                    gain: gain.clone(),
                                };
                                dbg!(&realization);
                                realizations.push(realization);

                                let remainder_gained = (&rhs_gained.amount - &proceeds).clone();
                                let remainder_offered = (&rhs_offered.amount - &rhs_offered.amount * &divisor).clone();

                                if !remainder_gained.is_zero() {
                                    let sale = Sale{
                                        when: liquidation.when.clone(),
                                        original_symbol: liquidation.original_symbol.clone(),
                                        offered: Amount{amount: remainder_offered, symbol: rhs_offered.symbol},
                                        gained: Amount{amount: remainder_gained, symbol: rhs_gained.symbol},
                                    };
                                    dbg!(&sale);
                                    processed_liquidations.push(sale);
                                }

                            } else {

                                eprintln!("Found lot of {} larger than current offering {}", lhs_gained.amount, rhs_offered.amount);

                                let divisor = &rhs_offered.amount / &lhs_gained.amount;
                                let proceeds = rhs_gained.amount.clone();
                                let cost_basis = (&lhs_offered.amount * &divisor).clone();
                                let gain = &proceeds - &cost_basis;

                                // This trade is smaller than the offered amount, so we need to split
                                let realization = Realization{
                                    description,
                                    acquired_when: Some(when.clone()),
                                    disposed_when: liquidation.when.clone(),
                                    proceeds: proceeds.clone(),
                                    cost_basis: cost_basis.clone(),
                                    gain: gain.clone(),
                                };
                                dbg!(&realization);
                                realizations.push(realization);

                                let remainder_offered = (&lhs_offered.amount * &divisor).clone();
                                let remainder_gained = (&lhs_gained.amount - &rhs_offered.amount).clone();

                                let trade = Trade{
                                    when: when.clone(),
                                    kind: Kind::Trade{
                                        offered: Amount{amount: remainder_offered, symbol: lhs_offered.symbol},
                                        gained: Amount{amount: remainder_gained, symbol: lhs_gained.symbol},
                                    }
                                };
                                dbg!(&trade);
                                processed_trades.push(trade);
                            }
                        }
                        Trade{ ref when, kind: Kind::Trade{ offered: ref lhs_offered, gained: ref lhs_gained } } if &lhs_gained.symbol == &rhs_offered.symbol => {
                            processed += 1;

                            if rhs_offered.amount == lhs_gained.amount {

                                // eprintln!("Found 1:1 match of {:?}", &lhs_offered);

                                processed_liquidations.push(Sale{
                                    when: liquidation.when.clone(),
                                    original_symbol: liquidation.original_symbol.clone(),
                                    offered: lhs_offered.clone(),
                                    gained: rhs_gained.clone(),
                                });

                            } else if rhs_offered.amount > lhs_gained.amount {

                                let proceeds = (&lhs_gained.amount / &rhs_offered.amount) * &rhs_gained.amount;

                                processed_trades.push(Trade{
                                    when: when.clone(),
                                    kind: Kind::Trade{
                                        offered: rhs_offered.clone(),
                                        gained: Amount{amount: proceeds.clone(), symbol: rhs_offered.symbol},
                                    }
                                });

                            } else if rhs_offered.amount < lhs_gained.amount {

                                let difference = &lhs_gained.amount - &rhs_offered.amount;

                                processed_trades.push(Trade{
                                    when: when.clone(),
                                    kind: Kind::Trade{
                                        offered: rhs_offered.clone(),
                                        gained: Amount{amount: difference.clone(), symbol: rhs_offered.symbol},
                                    }
                                });
                            }
                        },
                        _ => {
                            processed_trades.push(trade.clone());
                        },
                    }
                }

                if !realization_processed {
                    processed_liquidations.push(liquidation.clone());
                }

                trades = processed_trades;
            }

            if processed == 0 {
                break
            }

            liquidations = processed_liquidations;
        }

        for liquidation in liquidations {
            realizations.push(Realization{
                description: format!("{} sold via {}-{} pair", liquidation.original_symbol.symbol(), liquidation.offered.symbol.symbol(), denomination.symbol()),
                acquired_when: None,
                disposed_when: liquidation.when.clone(),
                proceeds: liquidation.gained.amount.clone(),
                cost_basis: BigDecimal::zero(),
                gain: liquidation.gained.amount.clone(),
            });
        }

        realizations
    }
}

impl fmt::Debug for Portfolio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (currency, wallet) in self.wallets.iter() {
            write!(f, "Wallet {:} {:} tokens remain worth ${:} ({:}/{:})\n", currency, wallet.count(), wallet.cost_basis(), wallet.cumulative_bought, wallet.cumulative_sold)?;
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
    fn test_portfolio_one_to_one_sell_with_profit() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1000),
                gained: btc!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(1),
                gained: usd!(2000),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2017, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(2000.).unwrap(),
                cost_basis: BigDecimal::from_f32(1000.).unwrap(),
                gain: BigDecimal::from_f32(1000.).unwrap(),
            },
        ]);
    }

    #[test]
    fn test_portfolio_one_to_one_sell_with_loss() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1000),
                gained: btc!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(1),
                gained: usd!(500),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2017, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(500.).unwrap(),
                cost_basis: BigDecimal::from_f32(1000.).unwrap(),
                gain: BigDecimal::from_f32(-500.).unwrap(),
            },
        ]);
    }

    #[test]
    fn test_portfolio_one_to_one_partial_sell_with_profit() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1000),
                gained: btc!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(0.5),
                gained: usd!(600),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2017, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(600.).unwrap(),
                cost_basis: BigDecimal::from_f32(500.).unwrap(),
                gain: BigDecimal::from_f32(100.).unwrap(),
            },
        ]);
    }

    #[test]
    fn test_portfolio_one_to_many_partial_sales() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1000),
                gained: btc!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(0.5),
                gained: usd!(600),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(0.25),
                gained: usd!(700),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2017, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(600.).unwrap(),
                cost_basis: BigDecimal::from_f32(500.).unwrap(),
                gain: BigDecimal::from_f32(100.).unwrap(),
            },
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2017, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(700.).unwrap(),
                cost_basis: BigDecimal::from_f32(250.).unwrap(),
                gain: BigDecimal::from_f32(450.).unwrap(),
            },
        ]);
    }

    #[test]
    fn test_portfolio_two_to_one_sell_with_profit() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1000),
                gained: btc!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1000),
                gained: btc!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(2),
                gained: usd!(4000),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2017, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(2000.).unwrap(),
                cost_basis: BigDecimal::from_f32(1000.).unwrap(),
                gain: BigDecimal::from_f32(1000.).unwrap(),
            },
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2018, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(2000.).unwrap(),
                cost_basis: BigDecimal::from_f32(1000.).unwrap(),
                gain: BigDecimal::from_f32(1000.).unwrap(),
            },
        ]);
    }

    #[test]
    fn test_portfolio_one_to_one_sell_with_insufficient_funds() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1000),
                gained: btc!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(2),
                gained: usd!(4000),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2017, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(2000.).unwrap(),
                cost_basis: BigDecimal::from_f32(1000.).unwrap(),
                gain: BigDecimal::from_f32(1000.).unwrap(),
            },
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: None,
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(2000.).unwrap(),
                cost_basis: BigDecimal::zero(),
                gain: BigDecimal::from_f32(2000.).unwrap(),
            },
        ]);
    }

    #[test]
    fn test_portfolio_one_to_one_with_exchange() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1000),
                gained: btc!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(1),
                gained: usdt!(2000),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usdt!(2000),
                gained: usd!(2000),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "USDT sold via USDT-USD pair".into(),
                acquired_when: Some(Utc.ymd(2017, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(2000.).unwrap(),
                cost_basis: BigDecimal::from_f32(1000.).unwrap(),
                gain: BigDecimal::from_f32(1000.).unwrap(),
            },
        ]);
    }

    #[test]
    fn test_portfolio_one_to_one_with_exchange_and_loss() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(4000),
                gained: btc!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(1),
                gained: usdt!(2000),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usdt!(2000),
                gained: usd!(2000),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "USDT sold via USDT-USD pair".into(),
                acquired_when: Some(Utc.ymd(2017, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(2000.).unwrap(),
                cost_basis: BigDecimal::from_f32(4000.).unwrap(),
                gain: BigDecimal::from_f32(-2000.).unwrap(),
            },
        ]);
    }

    #[test]
    fn test_portfolio_sell() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2016, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(100),
                gained: usdt!(25),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2016, 1, 2).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(100),
                gained: usdt!(25),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(100),
                gained: usdt!(25),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usdt!(40),
                gained: eth!(2),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: eth!(2),
                gained: btc!(0.1),
            }
        });

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 2).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(0.1),
                gained: usd!(4000),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2016, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 2).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(2500.).unwrap(),
                cost_basis: BigDecimal::from_f32(100.).unwrap(),
                gain: BigDecimal::from_f32(2400.).unwrap(),
            },
            Realization{
                description: "BTC sold via BTC-USD pair".into(),
                acquired_when: Some(Utc.ymd(2016, 1, 2).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 2).and_hms(0, 0, 0),
                proceeds: BigDecimal::from_f32(1500.).unwrap(),
                cost_basis: BigDecimal::from_f32(60.).unwrap(),
                gain: BigDecimal::from_f32(1440.).unwrap(),
            }
        ]);

    }
}
