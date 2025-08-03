use dashmap::DashMap;
use payments_engine::engine::process_transaction;
use payments_engine::structures::{
    ClientsMap, TransactionMessage, TransactionType, TransactionsMap,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::mpsc;

/// @brief Asynchronous test for basic transaction processing flow.
///
/// This test verifies that the transaction processor correctly handles
/// a simple sequence of deposit and withdrawal transactions, followed
/// by a termination message.
///
/// Steps tested:
/// - Sending a deposit transaction (client 1 deposits 10.0).
/// - Sending a withdrawal transaction (client 1 withdraws 5.0).
/// - Sending a terminate transaction to stop the processor.
///
/// After processing:
/// - The client's available balance and total balance are correctly updated.
/// - The transactions map contains records for both deposit and withdrawal.
#[tokio::test]
async fn test_process_transaction_basic_flow() {
    let clients: ClientsMap = Arc::new(DashMap::new());
    let transactions: TransactionsMap = Arc::new(DashMap::new());

    let (tx, rx) = mpsc::channel(10);

    let send_task = tokio::spawn(async move {
        tx.send(TransactionMessage {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(100, 1)), // 10.0
        })
        .await
        .unwrap();

        tx.send(TransactionMessage {
            tx_type: TransactionType::Withdrawal,
            client: 1,
            tx: 2,
            amount: Some(Decimal::new(50, 1)), // 5.0
        })
        .await
        .unwrap();

        tx.send(TransactionMessage {
            tx_type: TransactionType::Terminate,
            client: 0,
            tx: 0,
            amount: None,
        })
        .await
        .unwrap();
    });

    let transactions_clone = transactions.clone();
    let clients_clone = clients.clone();
    let processor_task = tokio::spawn(async move {
        process_transaction(rx, clients_clone, transactions_clone).await;
    });

    send_task.await.unwrap();
    processor_task.await.unwrap();

    let client1 = clients.get(&1).expect("Client 1 should exist");
    let client1 = client1.value();

    assert_eq!(client1.available, Decimal::new(50, 1)); // 10 - 5 = 5.0
    assert_eq!(client1.total, Decimal::new(50, 1)); // total updated accordingly

    assert!(transactions.contains_key(&1)); // Deposit
    assert!(transactions.contains_key(&2)); // Withdrawal
}

/// @brief Asynchronous test for processing all types of transactions including dispute workflow.
///
/// This test covers a complete scenario involving:
/// - Deposit
/// - Withdrawal
/// - Dispute
/// - Resolve
/// - Chargeback
/// - Attempted withdrawal after account is locked
/// - Termination of processing
///
/// It verifies correct state transitions of client balances and transaction dispute flags,
/// as well as account locking after chargeback.#[tokio::test]
#[tokio::test]
async fn test_process_transaction_all_types() {
    let clients: ClientsMap = Arc::new(DashMap::new());
    let transactions: TransactionsMap = Arc::new(DashMap::new());

    let (tx, rx) = mpsc::channel(10);

    let send_task = tokio::spawn(async move {
        // Deposit 10.0
        tx.send(TransactionMessage {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(Decimal::new(100, 1)), // 10.0
        })
        .await
        .unwrap();

        // Withdrawal 5.0 (should succeed)
        tx.send(TransactionMessage {
            tx_type: TransactionType::Withdrawal,
            client: 1,
            tx: 2,
            amount: Some(Decimal::new(50, 1)), // 5.0
        })
        .await
        .unwrap();

        // Dispute on Deposit tx=1
        tx.send(TransactionMessage {
            tx_type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        })
        .await
        .unwrap();

        // Resolve dispute on tx=1
        tx.send(TransactionMessage {
            tx_type: TransactionType::Resolve,
            client: 1,
            tx: 1,
            amount: None,
        })
        .await
        .unwrap();

        // Dispute again on tx=1
        tx.send(TransactionMessage {
            tx_type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        })
        .await
        .unwrap();

        // Chargeback on tx=1 (freezes account)
        tx.send(TransactionMessage {
            tx_type: TransactionType::Chargeback,
            client: 1,
            tx: 1,
            amount: None,
        })
        .await
        .unwrap();

        // Attempt withdrawal after chargeback (should be ignored because account locked)
        tx.send(TransactionMessage {
            tx_type: TransactionType::Withdrawal,
            client: 1,
            tx: 3,
            amount: Some(Decimal::new(10, 1)),
        })
        .await
        .unwrap();

        // Terminate processor
        tx.send(TransactionMessage {
            tx_type: TransactionType::Terminate,
            client: 0,
            tx: 0,
            amount: None,
        })
        .await
        .unwrap();
    });

    let clients_clone = clients.clone();
    let transactions_clone = transactions.clone();
    let processor_task = tokio::spawn(async move {
        process_transaction(rx, clients_clone, transactions_clone).await;
    });

    send_task.await.unwrap();
    processor_task.await.unwrap();

    let client = clients.get(&1).expect("Client 1 should exist");
    let client = client.value();

    // After deposit 10, withdrawal 5, dispute holds 10, resolve returns 10, dispute again holds 10,
    // chargeback deducts 10 and locks account
    // Available = 0, held = 0, total = 0, locked = true

    assert_eq!(client.available, Decimal::new(4, 0));
    assert_eq!(client.held, Decimal::new(0, 0));
    assert_eq!(client.total, Decimal::new(4, 0));
    assert!(!client.locked);

    // Check transactions:
    let tx1 = transactions.get(&1).expect("Transaction 1 should exist");
    let tx1 = tx1.value();

    assert_eq!(tx1.client_id, 1);
    assert_eq!(tx1.amount, Decimal::new(100, 1)); // 10.0
    assert!(
        !tx1.disputed,
        "Transaction 1 should no longer be disputed after chargeback"
    );
    assert_eq!(tx1.tx_type, TransactionType::Deposit);

    // Withdrawal tx=2 should exist
    assert!(transactions.contains_key(&2));

    // Withdrawal tx=3 should NOT exist, because account locked prevented it
    assert!(!transactions.contains_key(&3));
}
