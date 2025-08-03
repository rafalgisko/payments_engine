use itertools::Itertools;

use crate::structures::ClientsMap;

/// Prints the final report of all client accounts in CSV format.
///
/// This function takes a map of client accounts and prints a summary line for each client,
/// including their available, held, total funds, and whether the account is locked. The output
/// is sorted by client ID for consistency.
///
/// The output format is:
/// ```text
/// client,available,held,total,locked
/// 1,100.0000,0.0000,100.0000,false
/// 2,50.0000,10.0000,60.0000,true
/// ...
/// ```
///
/// # Parameters
/// - `clients`: A `ClientsMap`, which is typically a `DashMap<u16, Account>` or similar concurrent map,
///   containing client account states keyed by client ID.
///
/// # Requirements
/// This function depends on the [`itertools`](https://docs.rs/itertools/latest/itertools/) crate
/// for the `.sorted_by_key()` method.
///
/// # Example
/// ```
/// let clients: ClientsMap = DashMap::new();
/// clients.insert(1, Account { available: dec!(100), held: dec!(0), total: dec!(100), locked: false });
/// print_final_report(clients);
/// ```
pub fn print_final_report(clients: ClientsMap) {
    println!("client,available,held,total,locked");

    clients
        .iter()
        .map(|entry| (*entry.key(), entry.value().clone()))
        .sorted_by_key(|(client_id, _)| *client_id) // wymaga itertools crate
        .for_each(|(client_id, account)| {
            println!(
                "{},{:.4},{:.4},{:.4},{}",
                client_id, account.available, account.held, account.total, account.locked
            );
        });
}
