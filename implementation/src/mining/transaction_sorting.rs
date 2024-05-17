use crate::parsing::transaction_structs::Transaction;
use std::collections::HashMap;

// returns the index of txid in Vec<Transaction> transactions.
fn get_parent_index(transactions: &Vec<Transaction>, txid: &String) -> usize {
    let mut parent_index: usize = 0;

    for tx in transactions {
        if *tx.meta.txid_hex == *txid {
            break;
        };
        parent_index += 1;
    }
    parent_index
}

// gets called by put_parents_in_front to take the Transaction at parent_index and put it in front of
// child_index
fn push_parent_in_front(
    transactions: &mut Vec<Transaction>,
    parent_index: usize,
    child_index: usize,
) {
    if parent_index < transactions.len() && child_index < transactions.len() {
        let parent = transactions.remove(parent_index);
        transactions.insert(child_index, parent);
    }
}

// puts parents in front of their children in the presorted Vec<Transaction>
fn put_parents_in_front(presorted: &mut Vec<Transaction>) {
    let mut nothing_changed: bool = false;

    'outer: while !nothing_changed {
        nothing_changed = true;
        let mut tx_index: usize = 0;

        let transactions_cloned = presorted.clone();
        for tx in transactions_cloned.iter() {
            if let Some(parents) = tx.meta.parents.as_ref() {
                for parent_txid in parents {
                    let parent_index = get_parent_index(presorted, parent_txid);
                    if parent_index > tx_index {
                        push_parent_in_front(presorted, parent_index, tx_index);
                        nothing_changed = false;
                        continue 'outer;
                    };
                }
            };
            tx_index += 1;
        }
    }
}

// entry function for sorting. sorts by packet feerate, then puts the parents in front
// of the children
pub fn sort_transactions(txid_tx_map: &HashMap<String, Transaction>) -> Vec<Transaction> {
    let mut transactions: Vec<&Transaction> = txid_tx_map.values().collect();
    transactions.sort_by(|a, b: &&Transaction| {
        b.meta
            .packet_data
            .packet_feerate_weight
            .cmp(&a.meta.packet_data.packet_feerate_weight)
    });

    let mut sorted_transactions: Vec<Transaction> = transactions.into_iter().cloned().collect();
    put_parents_in_front(&mut sorted_transactions);
    // validate_sorting(&sorted_transactions);  // call to validation function for testing
    sorted_transactions
}

// removes enough Transactions from the sorted Vec<Transaction> to respect the
// block size limit of 4 000 000 weight units
pub fn cut_size(sorted_transactions: Vec<Transaction>) -> Vec<Transaction> {
    let mut block: Vec<Transaction> = Vec::new();
    let mut free_block_space: i64 = 3970000;
    for tx in sorted_transactions {
        if free_block_space > tx.meta.weight as i64 {
            free_block_space -= tx.meta.weight as i64;
            block.push(tx);
        } else {
            break;
        };
    }
    block
}

// Function to validate that no child occurs before its parents.
// pub fn validate_sorting(sorted_transactions: &Vec<Transaction>) -> () {
//     let mut index = 0;

//     for tx in sorted_transactions {
//         if let Some(parents_txids) = tx.meta.parents.as_ref() {
//             for parent in parents_txids {
//                 let parent_index = get_parent_index(sorted_transactions, parent);
//                 if parent_index >= index {
//                     panic!("Parent after child!");
//                 }
//             }
//         };
//         index += 1;
//     }
// }
