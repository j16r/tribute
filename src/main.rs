extern crate coinbase_pro_rs;
extern crate csv;

use std::error::Error;
use std::io;
use std::process;

use coinbase_pro_rs::{Public, Sync, SANDBOX_URL};

fn export() -> Result<(), Box<Error>> {
    let client: Public<Sync> = Public::new(SANDBOX_URL);

    let mut writer = csv::Writer::from_writer(io::stdout());

    writer.write_record(&["Description", "Token", "Count", "Price", "Time"])?;

    let products = client.get_products().unwrap();

    for product in products {
        let trades = client.get_trades(&product.id).unwrap();
        for trade in trades {
            writer.write_record(&[
                &product.id,
                &product.quote_currency,
                &trade.size.to_string(),
                &trade.price.to_string(),
                &trade.time.to_string(),
            ])?;
        }
    }

    writer.flush()?;
    Ok(())
}

fn main() {
    if let Err(err) = export() {
        println!("{}", err);
        process::exit(1);
    }
}
