use crate::parsing::transaction_structs::Transaction;
use std::collections::HashMap;

struct FeeAndWeight {
    fee: u64,
    weight: u64,
}

// recursively goes to the bottom of a transaction dependency structure and sums up the fee and weight
// up to the passed transaction which are returned in FeeAndWeight
fn calc_parents(transactions: &HashMap<String, Transaction>, child_txid: &String) -> FeeAndWeight {
    let mut fee_and_weight: FeeAndWeight;

    if let Some(child_transaction) = transactions.get(child_txid) {
        fee_and_weight = FeeAndWeight {
            fee: child_transaction.meta.fee,
            weight: child_transaction.meta.weight,
        };

        if let Some(parents_txids) = child_transaction.meta.parents.as_ref() {
            for parent in parents_txids {
                let temp_result = calc_parents(transactions, parent);
                fee_and_weight.fee += temp_result.fee;
                fee_and_weight.weight += temp_result.weight;
            }
        } else {
            return fee_and_weight;
        };
    } else {
        panic!("calc_parent_fees: tx not found?");
    };

    fee_and_weight
}

// assigning the packet fee, weight and feerate to each transaction.
// the packet data are equal to the tx data if the tx has no parents
pub fn calculate_packet_weights(transactions: &mut HashMap<String, Transaction>) {
    let transactions_original_clone = transactions.clone();

    for (txid, tx) in transactions.iter_mut() {
        let temp_result = calc_parents(&transactions_original_clone, txid);
        tx.meta.packet_data.packet_fee_sat = temp_result.fee;
        tx.meta.packet_data.packet_weight = temp_result.weight;

        tx.meta.packet_data.packet_feerate_weight = temp_result.fee / temp_result.weight;
    }
}
