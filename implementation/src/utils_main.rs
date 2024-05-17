use crate::parsing::transaction_structs::Transaction;
use std::collections::{HashMap, HashSet};

// Converts a Vec<Transaction> to HashMap<hex txid Sting, Transaction>
pub fn convert_to_hashmap(transactions: Vec<Transaction>) -> HashMap<String, Transaction> {
    let mut txid_tx_map = HashMap::new();

    for transaction in transactions {
        txid_tx_map.insert(transaction.meta.txid_hex.clone(), transaction);
    }
    txid_tx_map
}

// Returns the passed Vec<Transaction> as HashMap<hex txid Sting, Transaction>
// with all invalid transactions specified in HashSet<hex txid String> removed from it
pub fn remove_invalid_transactions(
    transactions: Vec<Transaction>,
    mut invalid_transactions: HashSet<String>,
) -> HashMap<String, Transaction> {
    let mut transactions = convert_to_hashmap(transactions);
    let mut nothing_removed: bool = false;

    while !nothing_removed {
        nothing_removed = true;

        for (txid, tx) in transactions.iter() {
            for input in &tx.vin {
                if invalid_transactions.contains(&input.txid) {
                    // also remove transactions with invalid, unconfirmed (mempool) parents
                    invalid_transactions.insert(txid.clone());
                };
            }
        }

        for invalid_txid in &invalid_transactions {
            if transactions.contains_key(invalid_txid) {
                transactions.remove(invalid_txid);
                nothing_removed = false;
            };
        }
    }
    transactions
}
