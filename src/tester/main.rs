use std::{collections::HashMap, path::Path};

use tokio::{fs, process::Command};
use tracing::{error, info};

/// @brief Normalize text lines by trimming, removing empty lines, and formatting numbers.
///
/// This function processes a multiline input string by:
/// - Removing any empty or whitespace-only lines.
/// - Trimming trailing whitespace from each line.
/// - Splitting each line by commas into tokens.
/// - Trimming whitespace around each token.
/// - Attempting to parse each token as a floating-point number:
/// - If successful, format the number with exactly four decimal places.
/// - If parsing fails, keep the token as a string unchanged.
/// - Rejoining tokens with commas.
/// - Rejoining processed lines with newline characters.
///
/// @param text Input multiline string to be normalized.
/// @return A normalized string with consistent numeric formatting and no extra whitespace.
fn normalize_text(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim_end()
                .split(',')
                .map(|token| {
                    let token = token.trim();
                    match token.parse::<f64>() {
                        Ok(num) => format!("{num:.4}"),
                        Err(_) => token.to_string(),
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// @brief Entry point of the asynchronous test runner program.
///
/// This function performs the following tasks:
/// - Initializes the logging system with tracing_subscriber.
/// - Reads all CSV files from the "../../sets" directory.
/// - Separates input and output files based on their filename prefixes ("input_" and "output_").
/// - Sorts input files to ensure consistent test execution order.
/// - For each input file:
/// - Finds the corresponding expected output file.
/// - Executes the external program payments_engine with the input file as argument.
/// - Captures and normalizes the program's output.
/// - Reads and normalizes the expected output from the corresponding output file.
/// - Compares the normalized actual output to the expected output.
/// - Logs the test result and tracks failures.
/// - After all tests, logs a summary of the test outcomes.
///
/// @return anyhow::Result<()> Result indicating success or failure of the test runner.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let sets_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("sets");
    //let sets_dir = Path::new("../../sets");
    println!("Using sets dir: {sets_dir:?}");

    // Read all files in the sets directory
    let mut entries = fs::read_dir(sets_dir).await?;
    let mut inputs = Vec::new();
    let mut outputs = HashMap::new();

    // Group files by name (without input_/output_ prefix)
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
            if fname.starts_with("input_") && fname.ends_with(".csv") {
                inputs.push(path);
            } else if fname.starts_with("output_") && fname.ends_with(".csv") {
                outputs.insert(fname.to_string(), path);
            }
        }
    }

    // Sort inputs for consistent test order
    inputs.sort();

    let mut failed = 0;
    let mut failed_inputs = Vec::new();

    for input_path in inputs {
        let input_fname = input_path.file_name().unwrap().to_str().unwrap();
        let suffix = &input_fname["input_".len()..];

        let expected_output_name = format!("output_{suffix}");
        let expected_output_path = match outputs.get(&expected_output_name) {
            Some(p) => p,
            None => {
                failed_inputs.push(input_fname.to_string());
                failed += 1;
                error!("Missing corresponding output file for {input_fname}");
                continue;
            }
        };

        let output = Command::new("./payments_engine")
            .arg(&input_path)
            .output()
            .await?;

        if !output.status.success() {
            failed_inputs.push(input_fname.to_string());
            failed += 1;
            error!("payments_engine returned an error for file {input_fname}");
            continue;
        }

        let actual = String::from_utf8(output.stdout)?;
        let actual_norm = normalize_text(&actual);

        let expected = fs::read_to_string(&expected_output_path).await?;
        let expected_norm = normalize_text(&expected);

        if actual_norm != expected_norm {
            error!("Test failed for file {}", input_fname);
            failed += 1;
            failed_inputs.push(input_fname.to_string());
        } else {
            info!("Test passed for file {}", input_fname);
        }
    }

    info!("Test summary:");
    info!("Number of failed tests: {}", failed);
    if !failed_inputs.is_empty() {
        error!("Failed test files:");
        for f in failed_inputs {
            info!("  - {}", f);
        }
    } else {
        info!("All tests passed successfully!");
    }

    Ok(())
}
