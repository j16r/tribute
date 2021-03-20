use std::fmt;

use bigdecimal::{BigDecimal, Zero};

use crate::types::DateTime;

#[derive(Debug)]
pub struct Lot {
    // amount represents a count of items in a lot
    amount: BigDecimal,
    // unit_cost represents the cost of each item in a lot
    unit_cost: BigDecimal,
    // date_of_purchase represents the date at which the lot was acquired
    date_of_purchase: DateTime,
}

#[derive(Debug)]
pub struct Sale {
    // how much did all the tokens that were sold cost in total
    pub cost_basis: BigDecimal,
    // when was the first purchase of tokens made
    pub date_of_purchase: Option<DateTime>,
}


pub struct Wallet {
    token: String,
    pub cumulative_bought: BigDecimal,
    pub cumulative_sold: BigDecimal,
    lots: Vec<Lot>,
}

impl Wallet {
    pub fn new(token: &str) -> Wallet {
        Wallet {
            token: token.into(),
            cumulative_bought: BigDecimal::zero(),
            cumulative_sold: BigDecimal::zero(),
            lots: Vec::new(),
        }
    }

    // add_lot adds a purchase of some unit of an item, with a count and a total cost
    pub fn add_lot(&mut self, amount: &BigDecimal, unit_cost: &BigDecimal, date: DateTime) {
        self.cumulative_bought += amount;
        self.lots.push(Lot {
            amount: amount.clone(),
            unit_cost: unit_cost.clone(),
            date_of_purchase: date,
        });

        eprintln!("Buying {:} of {:} (Total {:})", amount, self.token, self.count());

    }

    // the total cost basis of everything in this wallet
    pub fn cost_basis(&self) -> BigDecimal {
        self.lots
            .iter()
            .map(|lot| &lot.amount * &lot.unit_cost)
            .sum()
    }

    // the number of tokens stored in this wallet
    pub fn count(&self) -> BigDecimal {
        self.lots
            .iter()
            .map(|lot| &lot.amount)
            .sum()
    }

    // sell some tokens, returning the Sale, remove any lots that were completely consumed
    pub fn sell(&mut self, amount: &BigDecimal) -> Sale {
        self.cumulative_sold += amount;

        let mut date_of_purchase: Option<DateTime> = None;

        let mut total_cost = BigDecimal::zero();
        let mut lots_consumed = 0;
        let mut amount_to_consume = amount.clone();

        for lot in self.lots.iter_mut() {
            if date_of_purchase.is_none() {
                date_of_purchase = Some(lot.date_of_purchase);
            }

            if &amount_to_consume < &lot.amount {
                lot.amount -= &amount_to_consume;
                total_cost += &amount_to_consume * &lot.unit_cost;
                amount_to_consume = BigDecimal::zero();
                break;
            }

            total_cost += &lot.amount * &lot.unit_cost;
            amount_to_consume -= &lot.amount;
            lots_consumed += 1;
        }

        if amount_to_consume > BigDecimal::zero() {
            eprintln!("Sale of {:} could not be satisfied with lots {:} ({:} was sold)", self.token, amount_to_consume, amount - &amount_to_consume);
        }

        self.lots.drain(..lots_consumed);

        eprintln!("Selling {:} of {:} (Total {:})", amount, self.token, self.count());
        Sale {
            cost_basis: total_cost,
            date_of_purchase,
        }
    }
}

impl fmt::Debug for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for lot in self.lots.iter() {
            write!(
                f,
                "lot {} units at {} = {}, ",
                lot.amount,
                lot.unit_cost,
                &lot.amount * &lot.unit_cost
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bigdecimal::FromPrimitive;
    use chrono::offset::TimeZone;
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_wallet_sell() {
        let mut wallet = Wallet::new("BTC");

        wallet.add_lot(
            &BigDecimal::from_f32(10.0).unwrap(),
            &BigDecimal::from_f32(1.0).unwrap(),
            Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
        );
        wallet.add_lot(
            &BigDecimal::from_f32(10.0).unwrap(),
            &BigDecimal::from_f32(2.0).unwrap(),
            Utc.ymd(2018, 2, 1).and_hms(0, 0, 0),
        );
        wallet.add_lot(
            &BigDecimal::from_f32(10.0).unwrap(),
            &BigDecimal::from_f32(3.0).unwrap(),
            Utc.ymd(2018, 3, 1).and_hms(0, 0, 0),
        );

        let sale1 = wallet.sell(&BigDecimal::from_f32(5.0).unwrap());
        assert_eq!(sale1.cost_basis, BigDecimal::from_f32(5.0).unwrap());
        assert_eq!(
            sale1.date_of_purchase,
            Some(Utc.ymd(2018, 1, 1).and_hms(0, 0, 0))
        );

        let sale2 = wallet.sell(&BigDecimal::from_f32(10.0).unwrap());
        assert_eq!(sale2.cost_basis, BigDecimal::from_f32(15.0).unwrap());
        assert_eq!(
            sale2.date_of_purchase,
            Some(Utc.ymd(2018, 1, 1).and_hms(0, 0, 0))
        );

        let sale3 = wallet.sell(&BigDecimal::from_f32(10.0).unwrap());
        assert_eq!(sale3.cost_basis, BigDecimal::from_f32(25.0).unwrap());
        assert_eq!(
            sale3.date_of_purchase,
            Some(Utc.ymd(2018, 2, 1).and_hms(0, 0, 0))
        );
    }

    #[test]
    fn test_wallet_sell_fail() {
        let mut wallet = Wallet::new("BTC");

        wallet.add_lot(
            &BigDecimal::from_f32(0.0444).unwrap(),
            &BigDecimal::from_f32(2.0).unwrap(),
            Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
        );
        wallet.add_lot(
            &BigDecimal::from_f32(1.0).unwrap(),
            &BigDecimal::from_f32(1.0).unwrap(),
            Utc.ymd(2018, 2, 1).and_hms(0, 0, 0),
        );

        assert_eq!(wallet.count(), BigDecimal::from_f32(1.0444).unwrap());

        let sale = wallet.sell(&BigDecimal::from_f32(0.5).unwrap());
        assert_eq!(sale.cost_basis, BigDecimal::from_f32(0.5444).unwrap());

        assert_eq!(wallet.count(), BigDecimal::from_f32(0.5444).unwrap());
    }

    #[test]
    fn test_wallet_sell_no_lots() {
        let mut wallet = Wallet::new("BTC");

        let sale = wallet.sell(&BigDecimal::from_f32(5.0).unwrap());
        assert_eq!(sale.cost_basis, BigDecimal::from_f32(0.0).unwrap());
        assert_eq!(sale.date_of_purchase, None);
    }

    #[test]
    fn test_wallet_sell_in_excess_of_lots() {
        let mut wallet = Wallet::new("BTC");

        wallet.add_lot(
            &BigDecimal::from_f32(2.0).unwrap(),
            &BigDecimal::from_f32(1.0).unwrap(),
            Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
        );

        let sale = wallet.sell(&BigDecimal::from_f32(5.0).unwrap());
        assert_eq!(sale.cost_basis, BigDecimal::from_f32(2.0).unwrap());
        assert_eq!(
            sale.date_of_purchase,
            Some(Utc.ymd(2018, 1, 1).and_hms(0, 0, 0))
        );

        assert!(wallet.count().is_zero());
    }
}

