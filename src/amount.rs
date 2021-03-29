use std::fmt::{Debug, Display, Error, Formatter};

use bigdecimal::BigDecimal;

use crate::symbol::Symbol;

#[derive(Clone, Eq, PartialEq)]
pub struct Amount {
    pub amount: BigDecimal,
    pub symbol: Symbol,
}

impl Debug for Amount {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.write_fmt(format_args!("{:#} ({:#})", self.amount, self.symbol))?;
        Ok(())
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self.symbol {
            Symbol::Fiat(ref symbol) => {
                f.write_fmt(format_args!("{:}{:}", symbol, self.amount))?;
            },
            Symbol::Crypto(ref symbol) => {
                f.write_fmt(format_args!("{:} {:}", self.amount, symbol))?;
            },
        }
        Ok(())
    }
}

// #[macro_export]
// macro_rules! amt {
//     ($symbol:literal:char, $amount:item) => {
//         Amount{
//             amount: stringify!($amount).parse().unwrap(),
//             symbol: Symbol::Fiat(Fiat::USD),
//         }
//     };
//     ($amount:item, $symbol:item) => {
//         Amount{
//             amount: stringify!($amount).parse().unwrap(),
//             symbol: Symbol::Fiat(Fiat::USD),
//         }
//     };
// }

#[macro_export]
macro_rules! usd {
    ($amount:expr) => {
        Amount{
            amount: stringify!($amount).parse().unwrap(),
            symbol: Symbol::Fiat(Fiat::USD),
        }
    };
}

#[macro_export]
macro_rules! usdt {
    ($amount:expr) => {
        Amount{
            amount: stringify!($amount).parse().unwrap(),
            symbol: Symbol::Crypto(Crypto::USDT),
        }
    };
}

#[macro_export]
macro_rules! eth {
    ($amount:expr) => {
        Amount{
            amount: stringify!($amount).parse().unwrap(),
            symbol: Symbol::Crypto(Crypto::ETH),
        }
    };
}

#[macro_export]
macro_rules! btc {
    ($amount:expr) => {
        Amount{
            amount: stringify!($amount).parse().unwrap(),
            symbol: Symbol::Crypto(Crypto::BTC),
        }
    };
}

#[cfg(test)]
mod test {
    use bigdecimal::{BigDecimal, FromPrimitive};

    use crate::symbol::{Symbol, Fiat, Crypto};

    use super::*;

    #[test]
    fn test_macros() {
        assert_eq!(usd!(100.0), Amount{amount: BigDecimal::from_f32(100.0).unwrap(), symbol: Symbol::Fiat(Fiat::USD)});
        assert_eq!(btc!(9007199254740993), Amount{amount: BigDecimal::from_i64(9007199254740993i64).unwrap(), symbol: Symbol::Crypto(Crypto::BTC)});
        // assert_eq!(amt!($39.2), Amount{amount: BigDecimal::from_f32(39.2).unwrap(), symbol: Symbol::Fiat(Fiat::USD)});
        // assert_eq!(amt!(11 BTC), Amount{amount: BigDecimal::from_i32(11).unwrap(), symbol: Symbol::Crypto(Crypto::BTC)});
    }
}
