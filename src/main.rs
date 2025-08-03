use crate::engine::process_transaction;
use crate::producer::process_file;
use crate::reports::print_final_report;
use crate::structures::{Args, ClientsMap, TransactionsMap};
use clap::Parser;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::{io, main, sync::mpsc};
use tracing::{error, info};

mod engine;
mod producer;
mod reports;
mod structures;

/// @brief Asynchronous entry point of the application.
///
/// This function initializes the logging system and sets up
/// asynchronous producer-consumer tasks for processing transactions.
///
/// Tasks:
/// - Parses command-line arguments.
/// - Creates a bounded channel for sending transaction messages.
/// - Initializes shared concurrent maps for clients and transactions.
/// - Spawns a consumer task that processes transactions received from the channel.
/// - Spawns a producer task that reads input data and sends transaction messages.
/// - Waits for both tasks to complete.
/// - After completion, prints the final report of client states.
///
/// @return `io::Result<()>` Result indicating the success or failure of the runtime.
#[main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_level(true)
        .init();

    let args = Args::parse();

    let (sender, receiver) = mpsc::channel(100);
    let args_clone = args.clone();
    let clients: ClientsMap = Arc::new(DashMap::new());
    let transactions: TransactionsMap = Arc::new(DashMap::new());

    let consumer_clients = Arc::clone(&clients);
    let consumer_transactions = Arc::clone(&transactions);

    let consumer_handle = tokio::spawn(async move {
        info!("Consumer task started");
        process_transaction(receiver, consumer_clients, consumer_transactions).await;
        info!("Consumer task completed");
    });

    let producer_handle = tokio::spawn(async move {
        info!("Producer task started");
        if let Err(e) = process_file(args_clone, sender).await {
            error!("Producer task encountered error: {:?}", e);
        } else {
            info!("Producer task completed");
        }
    });

    let _ = producer_handle.await;
    let _ = consumer_handle.await;

    info!("All tasks completed, printing final report");
    print_final_report(clients);

    Ok(())
}
