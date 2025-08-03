# Transaction Processor (Async Rust System)

This program processes financial transactions stored in a CSV file. It reads the input file, processes each transaction according to business logic, and outputs the resulting client account states in CSV format.

## Usage

```bash
payments_engine <input_csv_file>
```

## 📤 Output

The application produces two types of output:

- **Standard Output (stdout):**  
  Prints the final state of each client account in CSV format (see below).
  
- **Standard Error (stderr):**  
  Logs all informational messages (`info`), warnings (`warn`), and errors (`error`) related to processing.


## Directory Structure

```
payments_engine/
├── src/
│ ├── engine.rs # Core transaction processing logic
│ ├── lib.rs # Library entry point
│ ├── main.rs # Binary entry point for the CLI
│ ├── structures.rs # Data structures for transactions and clients
│ ├── producer.rs # Handles reading and streaming of input data
│ ├── reports.rs # Output formatting and result reporting
│ └── tester/
│ └── main.rs # Asynchronous test runner comparing engine output to expected results
├── sets/
│ ├── input_0001.csv # Input transaction sets for testing
│ ├── output_0001.csv # Expected outputs corresponding to each input
│ ├── input_0002.csv
│ └── output_0002.csv
│ └── ...
├── tests/
│ ├── engine_tests.rs # Unit tests for engine logic
│ └── producer_tests.rs # Unit tests for producer module
└── README.md
```


## AI Usage Disclosure


### Declaration of AI Tool Usage
I have used AI tools (specifically OpenAI ChatGPT) during the development of this submission. The assistance included:
- Drafting and refining parts of the source code logic
- Discussing design and implementation ideas
- Generating or editing sections of this README file
All decisions, code integration, and final wording were reviewed and approved by me.

### My original contributions:
- I designed the 2-layer architecture: a producer layer for file reading and parsing, and a consumer layer for transaction processing.
- I decided to use a channel (mpsc) to connect both layers asynchronously.
- I implemented parameter handling using `clap` to accept the input file path.
- I designed the logging and output strategy: using `tracing` (info!, error!, warn!) directed to stderr, and keeping final output via `println!` in stdout to match the problem specification.

### ChatGPT assistance:
- Used for code refactoring and cleanup.
- Provided suggestions for writing clearer comments and documentation.

All AI-generated code was reviewed, modified where needed, and fully understood before inclusion.


---

## ✅ Automated Testing with CSV Test Sets

To validate the correctness of the `payments_engine` application, a dedicated automated testing tool named `tester` has been developed.

### Test Structure

Test cases are organized as paired CSV files located in the `sets/` directory, which is placed next to the `src/` directory. Each test case consists of two files:

- `input_*.csv`: contains a list of transaction commands to be processed by `payments_engine`
- `output_*.csv`: contains the expected output that should be produced by the engine

Each pair is matched by a common suffix in the filename (e.g., `input_001.csv` ↔ `output_001.csv`).

### Testing Workflow

The `tester` application automatically performs the following steps:

1. Scans the `sets/` directory for all valid `input_*.csv` / `output_*.csv` pairs.
2. Sequentially runs `payments_engine` using each `input_*.csv` as the input stream.
3. Captures the output of `payments_engine` and compares it against the corresponding `output_*.csv`.
4. Reports the number of test failures (if any) at the end of the run.

This testing mechanism helps to ensure that `payments_engine` behaves as expected for a wide range of input scenarios.

## Running the Tester

To run the `tester` binary, you need to build both applications: `payments_engine` and `tester`. You can do this using:

```bash
cargo build
```

This will generate two executable files in the target/debug directory:

- payments_engine
- tester

Next, navigate to the target/debug directory:

```bash
./tester
```

The tester will:

Automatically run payments_engine for each input file found in the sets directory.

Compare the output of payments_engine to the corresponding expected output file.

Print a pass/fail result for each test case.

Display a summary of failed tests, if any.

💡 Note: The tester must be executed from the target/debug directory to correctly locate the sets folder relative to the binary.


```bash
ubuntu@mgmobl:/mnt/c/Git/payments_engine/target/debug$ ./tester
Using sets dir: "/mnt/c/Git/payments_engine/sets"
2025-08-05T19:56:47.191570Z  INFO Test passed for file input_000.csv
2025-08-05T19:56:47.297469Z  INFO Test passed for file input_001.csv
2025-08-05T19:56:47.401363Z  INFO Test passed for file input_002.csv
2025-08-05T19:56:47.526880Z  INFO Test passed for file input_003.csv
2025-08-05T19:56:47.632708Z  INFO Test passed for file input_004.csv
2025-08-05T19:56:47.741956Z  INFO Test passed for file input_005.csv
2025-08-05T19:56:47.847549Z  INFO Test passed for file input_006.csv
2025-08-05T19:56:47.964261Z  INFO Test passed for file input_007.csv
2025-08-05T19:56:48.091053Z  INFO Test passed for file input_basic.csv
2025-08-05T19:56:48.091080Z  INFO Test summary:
2025-08-05T19:56:48.091097Z  INFO Number of failed tests: 0
2025-08-05T19:56:48.091109Z  INFO All tests passed successfully!
```

### 🧪 Unit Tests

The project includes **example unit tests** to verify the core functionality of selected modules:

- **`engine_tests.rs`** — Tests the transaction processing logic of the `engine` module. Includes cases covering deposits, withdrawals, disputes, and more.
- **`producer_tests.rs`** — Tests the CSV parser (`producer` module) that reads transactions and sends entries via an async channel.

These tests serve as a foundation and can be extended with more edge cases and error handling scenarios.

You can run all tests using:

```bash
cargo test
```

## 🌐 Architecture and Future Improvements

This application is built using a **two-layer architecture**:

- **Layer 1 (Producer):**  
  Responsible for reading the CSV file line-by-line, deserializing each row into a transaction object, and pushing it into an asynchronous channel.

- **Layer 2 (Consumer):**  
  Listens on the receiving end of the channel and processes each incoming transaction according to business rules.

This architecture is **highly scalable** due to the decoupling via the channel. The channel could be easily replaced by a remote queue (e.g., over the network), multiple producers could be introduced, or consumers could be parallelized to handle transactions concurrently. These features enable **horizontal scalability**.

### 🧠 Design Choices

- The transaction registry is implemented using **DashMap**, which allows efficient concurrent access and mutation across multiple async tasks.
- The project uses **Tokio tasks** for concurrency, which is well-suited for I/O-bound workloads — such as processing a high volume of small transactions — by minimizing thread overhead and maximizing throughput.

### 🔧 Possible Enhancements

- **Memory Optimization:**  
  Currently, all transactions are stored in the registry indefinitely. Some can be safely removed, such as:
  - A transaction that was disputed and later resolved — this pair can be deleted once resolved.
  - Very old transactions could be archived or moved into long-term storage, based on a retention policy.

- **Advanced Channeling:**  
  Channels could be replaced with:
  - A message broker (e.g., Kafka, NATS) for distributed setups.
  - An async network stream, enabling remote producers or consumers.
  
- **Parallel Consumers:**  
  Consumer logic could be parallelized using a worker pool pattern to scale processing throughput on multi-core systems.

---

This architecture sets a strong foundation for building a robust, concurrent, and horizontally scalable transaction engine.