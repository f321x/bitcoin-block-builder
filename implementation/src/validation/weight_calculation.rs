use crate::parsing::transaction_structs::Transaction;
use crate::validation::utils::varint;
use crate::validation::validate_parsing::{serialize_input, serialize_output};

// Weight multipliers for calculation of weight units from bytes:
// -------------------
// Field	Multiplier
// version	x4
// marker	x1
// flag		x1
// input	x4
// output	x4
// witness	x1
// locktime	x4

// returns: true if any &Transaction input contains a witness field
pub fn is_segwit(tx: &Transaction) -> bool {
    for txin in &tx.vin {
        if txin.witness.is_some() {
            return true;
        }
    }
    false
}

// returns: size of the complete input part of the transaction as u32
fn input_weight_sum(tx: &Transaction) -> u32 {
    let mut input_weight_sum: u32 = 0;
    input_weight_sum += varint(tx.vin.len() as u128).len() as u32;
    for txin in &tx.vin {
        input_weight_sum += serialize_input(txin).len() as u32;
    }
    input_weight_sum
}

// returns: size of the complete output part of the transaction as u32
fn output_weight_sum(tx: &Transaction) -> u32 {
    let mut output_weight_sum: u32 = 0;
    output_weight_sum += varint(tx.vout.len() as u128).len() as u32;
    for txout in &tx.vout {
        output_weight_sum += serialize_output(txout).len() as u32;
    }
    output_weight_sum
}

// returns: size in bytes of all witnesses contained in a transaction as u32
fn witness_weight_sum(tx: &Transaction) -> u32 {
    let mut witness_weight_sum: u32 = 0;
    for txin in &tx.vin {
        if let Some(hex_witness_vec) = &txin.witness {
            for witness in hex_witness_vec {
                witness_weight_sum += hex::decode(witness)
                    .expect("witness weight calculation hex decode failed")
                    .len() as u32;
            }
        };
    }
    witness_weight_sum
}

// calls the functions to calculate the weight of the different components
// of the transactions. Multiplies and sums them.
// returns: tx weight as u32
fn calculate_weight(tx: &Transaction) -> u32 {
    let mut weight: u32 = 4 * 4; // Version: 4 bytes x 4
    if is_segwit(tx) {
        weight += 2; // marker 1 byte + flag 1 byte
        weight += witness_weight_sum(tx); // weight of all witnesses in tx
    };
    weight += input_weight_sum(tx) * 4; // sum of all inputs * 4
    weight += output_weight_sum(tx) * 4; // sum of all outputs * 4
    weight += 4 * 4; // 4 bytes locktime * 4
    weight
}

// calculates tx weight and checks if the weight is invalid (> blocksize)
// returns: true if valid
pub fn validate_and_set_weight(tx: &mut Transaction) -> bool {
    let weight = calculate_weight(tx);
    if weight > (4000000 - (400 + 320)) {
        // leave some space for header and coinbase tx
        return false;
    };
    tx.meta.weight = weight as u64;
    true
}
