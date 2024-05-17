use crate::parsing::transaction_structs::Transaction;

// checks the input sum of the passed &mut Transaction against the output sum
// to prevent money creation. Also checks if there are inputs and outputs.
// Sets the delta between input and output as fee (in satoshi) in the &mut Transaction.
// returns: true if valid
pub fn validate_values_and_set_fee(tx: &mut Transaction) -> bool {
    let mut input_sum = 0;
    let mut output_sum = 0;

    if tx.vin.is_empty() || tx.vout.is_empty() {
        // no in or outputs
        return false;
    }
    for txin in &tx.vin {
        input_sum += txin.prevout.value;
    }
    for txout in &tx.vout {
        output_sum += txout.value;
    }
    if input_sum < output_sum {
        // no inflation!
        return false;
    }
    if input_sum > (20999999 * 100000000) || output_sum > (20999999 * 100000000) {
        // this is unrealistic
        return false;
    };
    tx.meta.fee = input_sum - output_sum;
    true
}

// checks if feerate is below 1sat/vbyte which is not being relayed (standard)
// returns: true if > 1 sat/vbyte
pub fn validate_feerate(tx: &Transaction) -> bool {
    let vbyte_size: u64 = tx.meta.weight / 4;
    let feerate = tx.meta.fee / vbyte_size;
    if feerate < 1 {
        return false;
    }
    true
}
