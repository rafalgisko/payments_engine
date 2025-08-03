use payments_engine::producer::process_file;
use payments_engine::structures::{Args, TransactionType};
use rust_decimal::Decimal;
use std::io::Write;
use tempfile::NamedTempFile;
use tokio::io::{self};
use tokio::sync::mpsc;

/// Basic integration test for the `process_file` function.
///
/// This test creates a temporary CSV file containing a header and two transactions:
/// a deposit and a withdrawal. It then calls `process_file` with this file and
/// collects all transaction messages sent over the channel.
///
/// The test verifies:
/// - That the final message is a termination signal (`TransactionType::Terminate`).
/// - That the first transaction corresponds to a deposit with the expected client ID,
///   transaction ID, type, and amount.
/// - That the second transaction corresponds to a withdrawal with the expected details.
///
/// # Returns
///
/// Returns `io::Result<()>` to propagate any I/O errors during file creation or writing.
///
/// # Async
///
/// This is an asynchronous test function using `tokio::test`.
#[tokio::test]
async fn test_process_file_basic() -> io::Result<()> {
    let mut tmpfile = NamedTempFile::new()?;
    writeln!(tmpfile, "type,client,tx,amount")?;
    writeln!(tmpfile, "deposit,1,1,1.0")?;
    writeln!(tmpfile, "withdrawal,1,2,0.5")?;
    tmpfile.flush()?;

    let (tx, mut rx) = mpsc::channel(10);

    let args = Args {
        input_file: tmpfile.path().to_str().unwrap().to_string(),
    };

    process_file(args, tx).await?;

    let mut received_messages = Vec::new();
    while let Some(msg) = rx.recv().await {
        received_messages.push(msg);
    }

    assert!(matches!(
        received_messages.last().unwrap().tx_type,
        TransactionType::Terminate
    ));

    assert_eq!(received_messages[0].client, 1);
    assert_eq!(received_messages[0].tx, 1);
    assert_eq!(received_messages[0].tx_type, TransactionType::Deposit);
    assert_eq!(received_messages[0].amount.unwrap(), Decimal::new(10, 1)); // 1.0

    assert_eq!(received_messages[1].client, 1);
    assert_eq!(received_messages[1].tx, 2);
    assert_eq!(received_messages[1].tx_type, TransactionType::Withdrawal);
    assert_eq!(received_messages[1].amount.unwrap(), Decimal::new(5, 1)); // 0.5

    Ok(())
}
