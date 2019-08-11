use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io;

use bigdecimal::{BigDecimal, Zero};
use chrono::{self, Datelike};

use crate::types::{format_type, format_usd_amount, parse_amount, DateTime};

struct Wallet {
    lots: Vec<Lot>,
}

impl Wallet {
    fn new() -> Wallet {
        Wallet { lots: Vec::new() }
    }

    // add_lot adds a purchase of some unit of an item, with a count and a total cost
    fn add_lot(&mut self, amount: &BigDecimal, unit_cost: &BigDecimal, date: DateTime) {
        self.lots.push(Lot {
            amount: amount.clone(),
            unit_cost: unit_cost.clone(),
            date_of_purchase: date,
        })
    }

    fn total_value(&self) -> BigDecimal {
        self.lots
            .iter()
            .map(|lot| &lot.amount * &lot.unit_cost)
            .sum()
    }

    fn sell(&mut self, amount: &BigDecimal) -> Sale {
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
                break;
            }

            total_cost += &lot.amount * &lot.unit_cost;
            amount_to_consume -= &lot.amount;
            lots_consumed += 1;
        }

        // Remove all consumed lots
        self.lots.drain(..lots_consumed);

        Sale {
            cost_basis: total_cost,
            date_of_purchase: date_of_purchase,
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

#[derive(Debug)]
struct Lot {
    // amount represents a count of items in a lot
    amount: BigDecimal,
    // unit_cost represents the cost of each item in a lot
    unit_cost: BigDecimal,
    // date_of_purchase represents the date at which the lot was acquired
    date_of_purchase: DateTime,
}

#[derive(Debug)]
struct Sale {
    cost_basis: BigDecimal,
    date_of_purchase: Option<DateTime>,
}

#[cfg(test)]
mod test {
    use chrono::offset::TimeZone;
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_wallet_sell() {
        let mut wallet = Wallet::new();

        wallet.add_lot(
            &BigDecimal::from(10.0),
            &BigDecimal::from(1.0),
            Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
        );
        wallet.add_lot(
            &BigDecimal::from(10.0),
            &BigDecimal::from(2.0),
            Utc.ymd(2018, 2, 1).and_hms(0, 0, 0),
        );
        wallet.add_lot(
            &BigDecimal::from(10.0),
            &BigDecimal::from(3.0),
            Utc.ymd(2018, 3, 1).and_hms(0, 0, 0),
        );

        let sale1 = wallet.sell(&BigDecimal::from(5.0));
        assert_eq!(sale1.cost_basis, BigDecimal::from(5.0));
        assert_eq!(
            sale1.date_of_purchase,
            Some(Utc.ymd(2018, 1, 1).and_hms(0, 0, 0))
        );

        let sale2 = wallet.sell(&BigDecimal::from(10.0));
        assert_eq!(sale2.cost_basis, BigDecimal::from(15.0));
        assert_eq!(
            sale2.date_of_purchase,
            Some(Utc.ymd(2018, 1, 1).and_hms(0, 0, 0))
        );

        let sale3 = wallet.sell(&BigDecimal::from(10.0));
        assert_eq!(sale3.cost_basis, BigDecimal::from(25.0));
        assert_eq!(
            sale3.date_of_purchase,
            Some(Utc.ymd(2018, 2, 1).and_hms(0, 0, 0))
        );
    }
}

pub fn report(year: u16) -> Result<(), Box<Error>> {
    let mut wallets: HashMap<String, Wallet> = HashMap::new();

    let mut rdr = csv::Reader::from_reader(io::stdin());

    let mut writer = csv::Writer::from_writer(io::stdout());

    writer.write_record(&[
        "Description of property",
        "Date acquired",
        "Date sold or disposed of",
        "Proceeds",
        "Cost basis",
        "Gain or (loss)",
    ])?;

    // Load everything into memory
    let mut line_items = rdr
        .records()
        .map(|r| r.unwrap())
        .collect::<Vec<csv::StringRecord>>();

    // Sort by date earliest to latest
    line_items.sort_by(|a, b| a.get(7).unwrap().partial_cmp(b.get(7).unwrap()).unwrap());

    let (mut total_proceeds, mut total_cost, mut total_gain) =
        (BigDecimal::zero(), BigDecimal::zero(), BigDecimal::zero());
    for line_item in line_items {
        let token = line_item.get(2).unwrap();
        let market = line_item.get(1).unwrap();

        let wallet = wallets.entry(token.into()).or_insert_with(|| Wallet::new());

        let amount = parse_amount(line_item.get(3).unwrap()).unwrap();
        let date_of_sale = chrono::DateTime::parse_from_rfc3339(line_item.get(7).unwrap())
            .unwrap()
            .with_timezone(&chrono::Utc);

        let rate = parse_amount(line_item.get(5).unwrap()).unwrap();
        let year_of_sale = date_of_sale.year();
        if year_of_sale > year as i32 {
            break;
        }

        let bought = amount > BigDecimal::zero();
        if bought {
            wallet.add_lot(&amount, &rate, date_of_sale);
            continue;
        }

        let Sale {
            cost_basis,
            date_of_purchase,
        } = wallet.sell(&amount.abs());

        let proceeds = parse_amount(line_item.get(6).unwrap()).unwrap().abs();
        let gain = &proceeds - &cost_basis;

        // Only print sales for the specified year
        if token != "USD" && year_of_sale == year as i32 {
            total_proceeds += &proceeds;
            total_cost += &cost_basis;
            total_gain += &gain;

            writer.write_record(&[
                &format_description(&token, &market, bought),
                &date_of_purchase.map_or("".to_string(), |d| d.format("%D").to_string()),
                &date_of_sale.format("%D").to_string(),
                &format_usd_amount(&proceeds),
                &format_usd_amount(&cost_basis),
                &format_usd_amount(&gain),
            ])?;
        }
    }

    writer.write_record(&[
        "Total",
        "",
        "",
        &format_usd_amount(&total_proceeds),
        &format_usd_amount(&total_cost),
        &format_usd_amount(&total_gain),
    ])?;

    for (currency, wallet) in wallets {
        eprintln!("Wallet {:?} balance = {:?}", currency, wallet.total_value());
    }

    writer.flush()?;
    Ok(())
}

fn format_description(sold_currency: &str, token: &str, bought: bool) -> String {
    format!(
        "{} {} via {} pair",
        sold_currency,
        format_type(bought),
        token
    )
}
