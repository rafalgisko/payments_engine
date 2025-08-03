use std::sync::Arc;

use clap::Parser;
use dashmap::DashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Command-line arguments parsed with `clap`.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Args {
    #[arg(value_name = "FILE")]
    pub input_file: String,
}

/// Represents the financial state of a client account.
#[derive(Debug, Clone, Default)]
pub struct ClientAccount {
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

/// Supported types of transactions.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Terminate,
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// A concurrent map of client IDs to their account state.
pub type ClientsMap = Arc<DashMap<u16, ClientAccount>>;

/// A concurrent map of transaction IDs to transaction records.
pub type TransactionsMap = Arc<DashMap<u32, TransactionRecord>>;

/// Serializable summary of a client's account state.
#[derive(Debug, Serialize)]
pub struct AccountSummary {
    /// Client ID
    pub client: u16,

    /// Available funds (not held/disputed)
    pub available: Decimal,

    /// Held/disputed funds
    pub held: Decimal,

    /// Total = available + held
    pub total: Decimal,

    /// Whether the account is locked (after chargeback)
    pub locked: bool,
}

impl std::str::FromStr for TransactionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "deposit" => Ok(TransactionType::Deposit),
            "withdrawal" => Ok(TransactionType::Withdrawal),
            "dispute" => Ok(TransactionType::Dispute),
            "resolve" => Ok(TransactionType::Resolve),
            "chargeback" => Ok(TransactionType::Chargeback),
            _ => Err(format!("Unknown transaction type: {s}")),
        }
    }
}

/// A message representing a transaction, parsed from CSV.
#[derive(Debug, Clone, Deserialize)]
pub struct TransactionMessage {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

/// A record representing the internal state of a transaction.
#[derive(Debug)]
pub struct TransactionRecord {
    pub client_id: u16,
    pub amount: Decimal,
    pub disputed: bool,
    pub tx_type: TransactionType,
}
