use std::fmt;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

use bigdecimal::BigDecimal;

use crate::types::{self, DateTime};

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct Transaction {
    pub id: String,
    pub market: String,
    pub token: String,
    pub amount: BigDecimal,
    pub rate: BigDecimal,
    pub usd_rate: BigDecimal,
    pub usd_amount: BigDecimal,
    pub created_at: Option<toml::value::Datetime>,
}

impl Eq for Transaction {}

pub enum ConfigError {
    IoError(io::Error),
    TomlError(toml::de::Error),
}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        ConfigError::IoError(error)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        ConfigError::TomlError(error)
    }
}

impl fmt::Debug for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let output = match *self {
            ConfigError::IoError(ref error) => format!("I/O error: {}", error),
            ConfigError::TomlError(ref error) => format!("Toml error: {:?}", error),
        };
        formatter.write_str(&output)
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct Config {
    pub exchanges: Vec<Exchange>,
    transactions: Option<Vec<Transaction>>,
    pub tax_year: u16,
    pub accounts: Option<Vec<web3::types::H160>>,
    pub denomination: Option<String>,
}

impl Config {
    pub fn transactions(&self) -> Vec<types::Transaction> {
        self.transactions
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|t| types::Transaction {
                id: t.id.clone(),
                market: t.market.clone(),
                token: t.token.clone(),
                amount: t.amount.clone(),
                rate: t.rate.clone(),
                usd_rate: t.usd_rate.clone(),
                usd_amount: t.usd_amount.clone(),
                created_at: t.created_at.clone().map(|t| chrono_to_toml_date(t)),
            })
            .collect()
    }

    pub fn denomination(&self) -> String {
        self.denomination.as_ref().unwrap_or(&"USD".to_string()).into()
    }
}

fn chrono_to_toml_date(value: toml::value::Datetime) -> DateTime {
    let input = format!("{}", &value);
    let naive_date = chrono::NaiveDate::parse_from_str(&input, "%Y-%m-%d").unwrap();
    chrono::DateTime::from_utc(naive_date.and_hms(0, 0, 0), chrono::Utc)
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub enum Exchange {
    CoinbasePro {
        key: String,
        secret: String,
        passphrase: String,
    },
    Coinbase {
        key: String,
        secret: String,
    },
    Ethereum {
        url: String,
    },
    Etherscan {
        key: String,
    },
}

pub fn load_config(path: Option<PathBuf>) -> Result<Config, ConfigError> {
    let mut input = String::new();

    File::open(path.unwrap_or("./".into()).join("config.toml"))
        .and_then(|mut f| f.read_to_string(&mut input))?;

    let config: Config = toml::from_str(&input)?;
    Ok(config)
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::path::PathBuf;
    use std::str::FromStr;

    use bigdecimal::{BigDecimal, FromPrimitive};
    use tempfile::TempDir;
    use toml::value::Datetime;

    use super::*;

    #[test]
    fn test_load_config() {
        let project = project(
            r#"
                tax_year = 2018

                exchanges = [
                    { Coinbase = { key = "coinbase-key", secret = "coinbase-secret" } },
                    { CoinbasePro = { key = "coinbase-pro-key", secret = "coinbase-pro-secret", passphrase = "coinbase-pro-passphrase" } },
                    { Ethereum = { url = "wss://ethereum.io/ws/v3/magic-token" } }
                ]

                accounts = [
                    "0xffffffffffffffffffffffffffffffffffffffff",
                ]

                [[transactions]]
                id = "0x1"
                market = "BTC-USD"
                token = "BTC"
                amount = 1255.66
                rate = 0.387690
                usd_rate = 0.387690
                usd_amount = 848.85
                created_at = 1997-02-14

                [[transactions]]
                id = "0x2"
                market = "BTC-USD"
                token = "BTC"
                amount = 6572.94
                rate = 0.257547
                usd_rate = 0.257547
                usd_amount = 1692.84
                created_at = 1997-08-04
            "#,
        )
        .unwrap();

        let config = load_config(Some(project.root.path().into())).unwrap();

        assert_eq!(
            config,
            Config {
                tax_year: 2018,
                exchanges: vec![
                    Exchange::Coinbase {
                        key: "coinbase-key".to_string(),
                        secret: "coinbase-secret".to_string()
                    },
                    Exchange::CoinbasePro {
                        key: "coinbase-pro-key".to_string(),
                        secret: "coinbase-pro-secret".to_string(),
                        passphrase: "coinbase-pro-passphrase".to_string()
                    },
                    Exchange::Ethereum {
                        url: "wss://ethereum.io/ws/v3/magic-token".to_string(),
                    },
                ],
                transactions: Some(vec![
                    Transaction {
                        id: "0x1".to_string(),
                        market: "BTC-USD".to_string(),
                        token: "BTC".to_string(),
                        amount: BigDecimal::from_f32(1255.66).unwrap(),
                        rate: BigDecimal::from_f32(0.387690).unwrap(),
                        usd_rate: BigDecimal::from_f32(0.387690).unwrap(),
                        usd_amount: BigDecimal::from_f32(848.85).unwrap(),
                        created_at: Some(Datetime::from_str("1997-02-14").unwrap()),
                    },
                    Transaction {
                        id: "0x2".to_string(),
                        market: "BTC-USD".to_string(),
                        token: "BTC".to_string(),
                        amount: BigDecimal::from_f32(6572.94).unwrap(),
                        rate: BigDecimal::from_f32(0.257547).unwrap(),
                        usd_rate: BigDecimal::from_f32(0.257547).unwrap(),
                        usd_amount: BigDecimal::from_f32(1692.84).unwrap(),
                        created_at: Some(Datetime::from_str("1997-08-04").unwrap()),
                    },
                ]),
                accounts: Some(vec![
                    web3::types::H160::from_str("ffffffffffffffffffffffffffffffffffffffff").unwrap(),
                ]),
                denomination: None,
            }
        );
        assert_eq!(config.denomination(), "USD".to_string());
    }

    #[test]
    fn test_load_config_empty_transactions() {
        let project = project(
            r#"
                tax_year = 2018
                exchanges = []
            "#,
        )
        .unwrap();

        let config = load_config(Some(project.root.path().into()));
        assert!(config.is_ok())
    }

    #[must_use]
    struct Project {
        root: TempDir,
        config_path: PathBuf,
    }

    fn project(body: &str) -> io::Result<Project> {
        let project_root = TempDir::new().expect("TempDir");

        let config_path = project_root.path().join("config.toml");

        let mut file = fs::File::create(&config_path).expect("File::create");
        file.write_all(body.as_bytes()).expect("file.write_all");

        Ok(Project {
            root: project_root,
            config_path: config_path,
        })
    }
}
