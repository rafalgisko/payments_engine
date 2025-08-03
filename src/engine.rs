use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::structures::{
    ClientsMap, TransactionMessage, TransactionRecord, TransactionType, TransactionsMap,
};

/// Processes incoming transaction messages asynchronously.
///
/// This function listens on a channel for incoming `TransactionMessage`s and applies
/// the appropriate logic to update client accounts and track transaction records.
/// It supports the following transaction types:
/// - Deposit: Adds funds to the client's account.
/// - Withdrawal: Removes funds from the client's available balance.
/// - Dispute: Moves a deposit amount from available to held funds.
/// - Resolve: Moves a held amount back to available funds.
/// - Chargeback: Removes held funds and locks the client's account.
/// - Terminate: Terminates the processing loop.
///
/// # Arguments
/// * `receiver` - An `mpsc::Receiver` for receiving `TransactionMessage`s.
/// * `clients` - A thread-safe map (`ClientsMap`) of client accounts.
/// * `transactions` - A thread-safe map (`TransactionsMap`) storing transaction records.
///
/// # Notes
/// - Accounts that are locked will not process any new transactions.
/// - Transactions are logged for auditing and error diagnosis.
///
/// # Panics
/// This function does not panic but logs errors for invalid transactions.
///
/// # Example
/// ```no_run
/// let (tx, rx) = mpsc::channel(100);
/// let clients = Arc::new(DashMap::new());
/// let transactions = Arc::new(DashMap::new());
/// tokio::spawn(async move {
///     process_transaction(rx, clients, transactions).await;
/// });
/// ```
pub async fn process_transaction(
    mut receiver: mpsc::Receiver<TransactionMessage>,
    clients: ClientsMap,
    transactions: TransactionsMap,
) {
    while let Some(msg) = receiver.recv().await {
        info!("msg received: {:?}", msg);
        if msg.tx_type == TransactionType::Terminate {
            warn!("Terminate message received, stopping processor.");
            break;
        }

        let mut client_entry = clients.entry(msg.client).or_default();
        let account = client_entry.clone();

        if account.locked {
            warn!(
                "Account {} is locked. Ignoring transaction: {:?}",
                msg.client, msg
            );
            continue;
        }

        match msg.tx_type {
            TransactionType::Deposit => {
                if let Some(amount) = msg.amount {
                    client_entry.available += amount;
                    client_entry.total += amount;

                    // Store transaction for future dispute reference
                    transactions.insert(
                        msg.tx,
                        TransactionRecord {
                            client_id: msg.client,
                            amount,
                            disputed: false,
                            tx_type: TransactionType::Deposit,
                        },
                    );
                }
            }
            TransactionType::Withdrawal => {
                if client_entry.locked {
                    warn!("Withdrawal ignored: account {} is locked", msg.client);
                    return;
                }
                if let Some(amount) = msg.amount {
                    if client_entry.available >= amount {
                        client_entry.available -= amount;
                        client_entry.total -= amount;

                        // Store withdrawal transaction as well (optional depending on specs)
                        transactions.insert(
                            msg.tx,
                            TransactionRecord {
                                client_id: msg.client,
                                amount,
                                disputed: false,
                                tx_type: TransactionType::Withdrawal,
                            },
                        );
                    } else {
                        warn!(
                            "Withdrawal failed due to insufficient funds for client {}",
                            msg.client
                        );
                    }
                }
            }
            TransactionType::Dispute => {
                if let Some(tx_rec) = transactions.get(&msg.tx) {
                    let tx_client = tx_rec.client_id;
                    let amount = tx_rec.amount;
                    let tx_type = tx_rec.tx_type.clone();
                    let was_disputed = tx_rec.disputed;
                    drop(tx_rec);

                    // Only allow dispute on client's own deposit transactions not already disputed
                    if tx_client == msg.client && !was_disputed {
                        if tx_type != TransactionType::Deposit {
                            warn!(
                                "Dispute failed: transaction {} is not a deposit (type: {:?})",
                                msg.tx, tx_type
                            );
                            return;
                        }

                        if client_entry.available >= amount {
                            client_entry.available -= amount;
                            client_entry.held += amount;

                            // Mark transaction as disputed
                            transactions.entry(msg.tx).and_modify(|rec| {
                                rec.disputed = true;
                            });
                        } else {
                            warn!(
                                "Dispute failed: client {} does not have enough available funds to hold",
                                msg.client
                            );
                        }
                    } else if tx_client != msg.client {
                        warn!(
                            "Dispute failed: client {} attempted to dispute transaction {} owned by client {}",
                            msg.client, msg.tx, tx_client
                        );
                    }
                } else {
                    warn!("Dispute failed: transaction {} not found", msg.tx);
                }
            }
            TransactionType::Resolve => {
                if let Some(tx_rec) = transactions.get(&msg.tx) {
                    let tx_client = tx_rec.client_id;
                    let amount = tx_rec.amount;
                    let was_disputed = tx_rec.disputed;
                    drop(tx_rec);

                    if tx_client == msg.client && was_disputed {
                        client_entry.held -= amount;
                        client_entry.available += amount;

                        // Mark transaction as no longer disputed
                        transactions
                            .entry(msg.tx)
                            .and_modify(|rec| rec.disputed = false);
                    }
                }
            }
            TransactionType::Chargeback => {
                if let Some(tx_rec) = transactions.get(&msg.tx) {
                    let tx_client = tx_rec.client_id;
                    let amount = tx_rec.amount;
                    let was_disputed = tx_rec.disputed;
                    drop(tx_rec);

                    if tx_client == msg.client && was_disputed {
                        client_entry.held -= amount;
                        client_entry.total -= amount;

                        client_entry.locked = true; // freeze account on chargeback

                        // Mark transaction as no longer disputed
                        transactions
                            .entry(msg.tx)
                            .and_modify(|rec| rec.disputed = false);
                    }
                }
            }
            _ => {}
        }
    }
    info!("Transaction processor stopped.");
}
