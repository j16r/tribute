extern crate coinbase_pro_rs;

use coinbase_pro_rs::{Public, Sync, SANDBOX_URL};

fn main() {
    let client: Public<Sync> = Public::new(SANDBOX_URL);

    let products = client.get_products().unwrap();

    for product in products {
        let trades = client.get_trades(&product.id).unwrap();
        println!("Trade: {:?}", trades);
    }
}
