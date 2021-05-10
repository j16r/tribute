use std::collections::{HashMap, VecDeque};
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

#[derive(Clone, Debug, Eq, PartialEq)]
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
                self.sell(trade.when, offered, gained);
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
        let (mut trades_by_gained, mut final_sales) = organize_trades(&self.trades, denomination);
        dbg!(&trades_by_gained, &final_sales);

        let mut realizations: Vec<Realization> = Vec::new();

        while let Some(trade) = final_sales.pop_front() {
            let description =
                format!(
                    "{original} sold via {original}-{} pair",
                    denomination.symbol(),
                    original = trade.original_symbol.symbol(),
                );

            dbg!(&trade);

            if let Some(matching_sales) = trades_by_gained.get_mut(&trade.offered.symbol) {
                eprintln!("\nStarting new trade match");
                dbg!(&trade);

                if matching_sales.is_empty() {
                    let realization = Realization{
                        description: description.clone(),
                        acquired_when: None,
                        disposed_when: trade.when.clone(),
                        proceeds: trade.gained.amount.clone(),
                        cost_basis: BigDecimal::zero(),
                        gain: trade.gained.amount.clone(),
                    };
                    dbg!(&realization);
                    realizations.push(realization);
                    // trades_by_gained.remove(&trade.offered.symbol);
                }

                if let Some(matching) = matching_sales.pop_front() {
                    dbg!(&matching);

                    if trade.offered.amount < matching.gained.amount {
                        dbg!(&matching);

                        let divisor = &trade.offered.amount / &matching.gained.amount;
                        let proceeds = trade.gained.amount.clone();
                        let cost_basis = (&matching.offered.amount * &divisor).clone();
                        let gain = &proceeds - &cost_basis;

                        if &matching.offered.symbol == denomination {
                            let realization = Realization{
                                description: description.clone(),
                                acquired_when: Some(matching.when.clone()),
                                disposed_when: trade.when.clone(),
                                proceeds: proceeds.clone(),
                                cost_basis: cost_basis.clone(),
                                gain: gain.clone(),
                            };
                            dbg!(&realization);
                            realizations.push(realization);
                        } else {
                            let sale = Sale{
                                when: trade.when.clone(),
                                original_symbol: trade.original_symbol.clone(),
                                offered: Amount{amount: matching.offered.amount.clone(), symbol: matching.offered.symbol},
                                gained: Amount{amount: proceeds.clone(), symbol: matching.gained.symbol},
                            };
                            dbg!(&sale);

                            final_sales.push_front(sale);
                        }

                        // Only part of the matching trade was accounted for, add a new trade in
                        // with the remainder
                        let remainder_gained = (&matching.gained.amount - &trade.offered.amount).clone();
                        let remainder_offered = (&matching.offered.amount - &matching.offered.amount * &divisor).clone();

                        let sale = Sale{
                            when: matching.when.clone(),
                            original_symbol: matching.original_symbol.clone(),
                            offered: Amount{amount: remainder_offered, symbol: matching.offered.symbol},
                            gained: Amount{amount: remainder_gained, symbol: matching.gained.symbol},
                        };
                        dbg!(&sale);

                        matching_sales.push_front(sale);

                    } else {
                        let divisor = &matching.gained.amount / &trade.offered.amount;
                        let proceeds = (&trade.gained.amount * &divisor).clone();
                        let cost_basis = &matching.offered.amount;
                        let gain = &proceeds - cost_basis;

                        if &matching.offered.symbol == denomination {
                            let realization = Realization{
                                description: description.clone(),
                                acquired_when: Some(matching.when.clone()),
                                disposed_when: trade.when.clone(),
                                proceeds: proceeds.clone(),
                                cost_basis: cost_basis.clone(),
                                gain: gain.clone(),
                            };
                            dbg!(&realization);
                            realizations.push(realization);
                        } else {
                            let sale = Sale{
                                when: trade.when.clone(),
                                original_symbol: trade.original_symbol.clone(),
                                offered: Amount{amount: matching.offered.amount.clone(), symbol: matching.offered.symbol},
                                gained: Amount{amount: proceeds.clone(), symbol: matching.gained.symbol},
                            };
                            dbg!(&sale);

                            final_sales.push_front(sale);
                        }

                        let remainder_gained = (&trade.gained.amount - &proceeds).clone();
                        let remainder_offered = (&trade.offered.amount - &trade.offered.amount * &divisor).clone();

                        if !remainder_gained.is_zero() {
                            let sale = Sale{
                                when: trade.when.clone(),
                                original_symbol: trade.original_symbol.clone(),
                                offered: Amount{amount: remainder_offered, symbol: trade.offered.symbol},
                                gained: Amount{amount: remainder_gained, symbol: trade.gained.symbol},
                            };
                            dbg!(&sale);

                            final_sales.push_front(sale);
                        }
                    }
                }
            }
        }

        realizations
    }
}


fn organize_trades(trades: &Vec<Trade>, denomination: &Symbol) ->
    (HashMap<Symbol, VecDeque<Sale>>, VecDeque<Sale>) {
    let mut trades_by_gained : HashMap<Symbol, VecDeque<Sale>> = HashMap::new();
    let mut final_sales : VecDeque<Sale> = VecDeque::new();

    // Organize all trades by what was obtained
    for trade in trades.iter() {
        let Trade{ when, kind: Kind::Trade{ gained, offered, .. }, .. } = trade;
        let sale = Sale{
            when: when.clone(),
            original_symbol: offered.symbol.clone(),
            offered: offered.clone(),
            gained: gained.clone(),
        };
        if &gained.symbol == denomination {
            final_sales.push_back(sale);
        } else {
            trades_by_gained
                .entry(gained.symbol)
                .or_insert_with(|| VecDeque::new())
                .push_back(sale);
        }
    }

    (trades_by_gained, final_sales)
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
    use bigdecimal::FromPrimitive;
    use chrono::Utc;
    use chrono::offset::TimeZone;
    use pretty_assertions::{assert_eq, assert_ne};

    use crate::symbol::{Symbol, Fiat, Crypto, USD, BTC};
    use crate::{usd, usdt, eth, btc};


    use super::*;

    #[test]
    fn test_organize_trades() {
        let mut trades : Vec<Trade> = Vec::new();
        trades.push(Trade{
            when: Utc.ymd(2020, 1, 3).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(300),
                gained: btc!(1),
            },
        });
        trades.push(Trade{
            when: Utc.ymd(2020, 1, 2).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: btc!(1),
                gained: usd!(57000),
            },
        });

        let (rest, to_usd) = organize_trades(&trades, &USD);

        assert_eq!(rest.len(), 1);
        assert_eq!(rest.get(&BTC), Some(&VecDeque::from(vec![Sale{
            when: Utc.ymd(2020, 1, 3).and_hms(0, 0, 0),
            original_symbol: USD,
            offered: usd!(300),
            gained: btc!(1),
        }])));
        assert_eq!(to_usd[0], Sale{
            when: Utc.ymd(2020, 1, 2).and_hms(0, 0, 0),
            original_symbol: BTC,
            offered: btc!(1),
            gained: usd!(57000),
        });
    }

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
    fn test_portfolio_sell_with_exchange_and_small_trade() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1000),
                gained: btc!(2),
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
                offered: usdt!(1000),
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
                cost_basis: BigDecimal::from_f32(500.).unwrap(),
                gain: BigDecimal::from_f32(1500.).unwrap(),
            },
        ]);
    }

    #[test]
    fn test_portfolio_sell_multiple_early_buys() {
        let mut portfolio = Portfolio::new();

        portfolio.add_trade(&Trade{
            when: Utc.ymd(2016, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1),
                gained: usdt!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2016, 1, 2).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1),
                gained: usdt!(1),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2016, 1, 3).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usd!(1),
                gained: usdt!(1),
            }
        });


        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usdt!(2),
                gained: usd!(2),
            }
        });
        portfolio.add_trade(&Trade{
            when: Utc.ymd(2020, 1, 2).and_hms(0, 0, 0),
            kind: Kind::Trade{
                offered: usdt!(1),
                gained: usd!(1),
            }
        });

        let realizations = portfolio.realizations(&USD);
        assert_eq!(realizations, vec![
            Realization {
                description: "USDT sold via USDT-USD pair".into(),
                acquired_when: Some(Utc.ymd(2016, 1, 1).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: "1.".parse().unwrap(),
                cost_basis: "1.".parse().unwrap(),
                gain: "0.0".parse().unwrap(),
            },
            Realization {
                description: "USDT sold via USDT-USD pair".into(),
                acquired_when: Some(Utc.ymd(2016, 1, 2).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                proceeds: "1.".parse().unwrap(),
                cost_basis: "1.".parse().unwrap(),
                gain: "0.0".parse().unwrap(),
            },
            Realization {
                description: "USDT sold via USDT-USD pair".into(),
                acquired_when: Some(Utc.ymd(2016, 1, 3).and_hms(0, 0, 0)),
                disposed_when: Utc.ymd(2020, 1, 2).and_hms(0, 0, 0),
                proceeds: "1.".parse().unwrap(),
                cost_basis: "1.".parse().unwrap(),
                gain: "0.0".parse().unwrap(),
            }
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
