use std::error::Error;
use std::str::FromStr;

use bigdecimal::BigDecimal;
use web3::futures::Future;
use web3::types::{BlockId, BlockNumber};

use crate::types::Transaction;
use chrono::prelude::*;

const PROVIDER: &str = "ethereum";

pub fn transactions(url: &str, accounts: &Vec<web3::types::H160>) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let (_eloop, transport) = web3::transports::WebSocket::new(url)?;
    let web3 = web3::Web3::new(transport);
    let current_block = web3.eth().block_number().wait()?;

    let mut transactions = Vec::new();

    for block_id in (0..current_block.as_usize()).rev() {
        let number = BlockId::Number(BlockNumber::Number(block_id.into()));
        let block = web3.eth().block_with_txs(number).wait()?;
        for transaction in block.unwrap().transactions {
            if !transaction_related(accounts, &transaction) {
                continue;
            }

            {
                let now = Utc::now();
                let amount = BigDecimal::from_str(&format!("{:}", transaction.value)).unwrap();
                let transaction = Transaction {
                    id: format!("{:}", transaction.hash),
                    market: "ETH-USD".to_string(),
                    token: "ETH".to_string(),
                    amount,
                    rate: BigDecimal::from(0),
                    usd_rate: BigDecimal::from(0),
                    usd_amount: BigDecimal::from(0),
                    created_at: Some(now),
                    provider: PROVIDER,
                };
                transactions.push(transaction);
            }

            if transaction.nonce.is_zero() {
                break;
            }
        }
    }

    Ok(transactions)
}

fn transaction_related(accounts: &Vec<web3::types::H160>, transaction: &web3::types::Transaction) -> bool {
    accounts.contains(&transaction.from) || transaction.to.map_or(false, |ref t| accounts.contains(t))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_transaction_related_empty() {
        let empty_accounts: Vec<web3::types::H160> = Vec::new();
        let transaction_blank = web3::types::Transaction {
            ..Default::default()
        };

        assert!(!transaction_related(&empty_accounts, &transaction_blank));
    }

    #[test]
    fn test_transaction_related_to() {
        let address = web3::types::H160::from_str("4c0457c5fB35183Cb25db52C14fEA30e737fcF5e").unwrap();
        let accounts: Vec<web3::types::H160> = vec![address];

        let transaction_to_account = web3::types::Transaction {
            to: Some(address),
            ..Default::default()
        };

        assert!(transaction_related(&accounts, &transaction_to_account));
    }

    #[test]
    fn test_transaction_related_from() {
        let address = web3::types::H160::from_str("4c0457c5fB35183Cb25db52C14fEA30e737fcF5e").unwrap();
        let accounts: Vec<web3::types::H160> = vec![address];

        let transaction_from_account = web3::types::Transaction {
            from: address,
            ..Default::default()
        };

        assert!(transaction_related(&accounts, &transaction_from_account));
    }
}
