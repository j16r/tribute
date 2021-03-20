use std::error::Error;
use std::io;

use bigdecimal::{BigDecimal, Zero};
use chrono::{self, Datelike};

use crate::types::{format_type, format_usd_amount, parse_amount};
use crate::wallet::Sale;
use crate::portfolio::Portfolio;

pub fn report(year: u16) -> Result<(), Box<dyn Error>> {
    let mut portfolio = Portfolio::new();

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
        if token != "BTC" {
            continue
        }

        let market = line_item.get(1).unwrap();

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
            portfolio.add_lot(&token, &amount, &rate, date_of_sale);
            continue;
        }

        let Sale {
            cost_basis,
            date_of_purchase,
        } = portfolio.sell(&token, &amount.abs());

        let proceeds = parse_amount(line_item.get(6).unwrap()).unwrap().abs();

        // Skip zero value transactions
        if proceeds.is_zero() && cost_basis.is_zero() {
            continue;
        }

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
