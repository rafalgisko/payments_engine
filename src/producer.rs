use csv_async::AsyncReaderBuilder;
use futures_util::stream::StreamExt;
use std::str::FromStr;
use tokio::fs::File;
use tokio::io;
use tokio::sync::mpsc;
use tokio_util::compat::TokioAsyncReadCompatExt;
use tracing::{error, warn};

use crate::structures::Args;
use crate::structures::TransactionMessage;
use crate::structures::TransactionType;
use serde::{self, Deserialize, Deserializer};

/// Custom deserializer for trimming whitespace from string fields.
///
/// This function is used with Serde to automatically remove leading and trailing
/// whitespace from strings during deserialization.
///
/// # Type Parameters
/// - `'de`: Lifetime of the data being deserialized.
/// - `D`: Type implementing the `Deserializer` trait.
///
/// # Parameters
/// - `deserializer`: A deserializer instance provided by Serde.
///
/// # Returns
/// - `Ok(String)`: A string with surrounding whitespace removed.
/// - `Err(D::Error)`: An error if deserialization fails.
///
/// # Usage
/// This function is typically used with the `#[serde(deserialize_with = "trimmed_string")]`
/// attribute on struct fields to sanitize input from CSV, JSON, or other text-based sources.
///
/// # Example
/// ```rust
/// #[derive(Deserialize)]
/// struct Record {
///     #[serde(deserialize_with = "trimmed_string")]
///     name: String,
/// }
/// ```
fn trimmed_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(s.trim().to_owned())
}

/// Represents a single transaction record parsed from a CSV file.
///
/// This struct is used for deserializing CSV input using Serde. Each record
/// corresponds to one transaction instruction with optional amount information.
///
/// Fields:
/// - `tx_type`: Type of the transaction (e.g., "deposit", "withdrawal", etc.).
///   This is a trimmed string, deserialized with a custom function to remove
///   leading/trailing whitespace.
/// - `client`: Unique client identifier.
/// - `tx`: Unique transaction identifier.
/// - `amount`: Optional monetary amount involved in the transaction (if applicable),
///   represented as a `Decimal` with expected precision up to 4 decimal places.
///
/// The CSV must include a header row with columns: `type`, `client`, `tx`, `amount`.
#[derive(Debug, Deserialize)]
struct CsvRecord {
    #[serde(rename = "type", deserialize_with = "trimmed_string")]
    tx_type: String,
    client: u16,
    tx: u32,
    amount: Option<rust_decimal::Decimal>,
}

/// Asynchronously processes a CSV input file and sends parsed transactions over a channel.
///
/// This function opens the provided CSV file, deserializes each record into a `CsvRecord`,
/// converts each record into a `TransactionMessage`, and sends it through the provided
/// asynchronous channel. It also sends a final `Terminate` message to signal the end
/// of input.
///
/// The CSV file must contain headers and should follow the expected transaction format:
/// - `type`: String representation of the transaction type (e.g., deposit, withdrawal, etc.)
/// - `client`: Client ID (u16)
/// - `tx`: Transaction ID (u32)
/// - `amount`: Optional amount (decimal, rounded to 4 places)
///
/// If the transaction type cannot be parsed, the record is skipped. If the receiver is dropped,
/// the loop terminates early.
///
/// @param args        Command-line arguments containing the input file path.
/// @param tx          Asynchronous channel sender used to forward transaction messages.
/// @return            `Ok(())` if processing completes successfully, or an I/O error otherwise.
pub async fn process_file(args: Args, tx: mpsc::Sender<TransactionMessage>) -> io::Result<()> {
    let file = File::open(&args.input_file).await?;
    // convert tokio::fs::File to a compatibility layer so csv_async can use it
    let reader = file.compat();

    let mut csv_reader = AsyncReaderBuilder::new()
        .has_headers(true)
        .trim(csv_async::Trim::All)
        .create_deserializer(reader);

    let mut records = csv_reader.deserialize::<CsvRecord>();

    while let Some(record) = records.next().await {
        let record = record?;

        // Parse transaction type from string to enum
        let tx_type = match TransactionType::from_str(&record.tx_type) {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to parse transaction type: {}", e);
                continue;
            }
        };

        let amount_rounded = record.amount.map(|a| a.round_dp(4));

        let message = TransactionMessage {
            tx_type,
            client: record.client,
            tx: record.tx,
            amount: amount_rounded,
        };

        // Send the transaction message through the channel
        if tx.send(message).await.is_err() {
            warn!("Receiver dropped, stopping processing");
            break;
        }
    }

    let message = TransactionMessage {
        tx_type: TransactionType::Terminate,
        client: 0,
        tx: 0,
        amount: None,
    };

    // Send the transaction message through the channel
    if tx.send(message).await.is_err() {
        error!("Receiver dropped, stopping processing");
    }

    Ok(())
}
