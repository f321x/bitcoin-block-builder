mod assign_parents;
mod construct_coinbase;
mod header;
mod packet_weight;
mod transaction_sorting;

use self::{
    assign_parents::assign_mempool_parents,
    construct_coinbase::{assemble_coinbase_transaction, CoinbaseTxData},
    header::construct_header,
    packet_weight::calculate_packet_weights,
    transaction_sorting::{cut_size, sort_transactions},
};
use crate::parsing::transaction_structs::Transaction;
use std::collections::HashMap;

pub struct Block {
    pub header_hex: String,
    pub coinbase_tx_hex: String,
    pub txids_hex: Vec<String>,
}

// hex encodes header and coinbase tx and creates a Vec<hex txid String> including
// the coinbase txid and returns it as Block struct for use in writing the output.txt
fn return_block(
    block_header_bytes: &[u8],
    coinbase_tx: CoinbaseTxData,
    transactions: &Vec<Transaction>,
) -> Block {
    let header_hex = hex::encode(block_header_bytes);
    let coinbase_tx_hex = hex::encode(coinbase_tx.assembled_tx);
    let mut txids_hex: Vec<String> = vec![coinbase_tx.txid_hex];
    for tx in transactions {
        txids_hex.push(tx.meta.txid_hex.clone());
    }
    Block {
        header_hex,
        coinbase_tx_hex,
        txids_hex,
    }
}

// main "mining" function. Takes a HashMap of valid transactions,
// Returns a Block struct with a blockheader, coinbase transaction and
// a Vec of txids sorted to maximise fee revenue and block space utilization
pub fn mine_block(txid_tx_map: &mut HashMap<String, Transaction>) -> Block {
    // link children with parent transactions
    assign_mempool_parents(txid_tx_map);

    // calculate packet weights for transactions with ancestors in mempool
    calculate_packet_weights(txid_tx_map);

    // sorts transactions by packet feerate and ancestry and removes enough respect block size
    let block_ordered: Vec<Transaction> = cut_size(sort_transactions(txid_tx_map));

    // assembles the coinbase transaction including the witness commitment
    let coinbase_tx: CoinbaseTxData = assemble_coinbase_transaction(&block_ordered);

    // assembles the block header
    let block_header = construct_header(&block_ordered, &coinbase_tx);

    // encode in Block struct and returns final data needed for output.txt
    return_block(&block_header, coinbase_tx, &block_ordered)
}

// -----------------------
// For validation use in mine_block():
// for wtxid validation with python script.
// pipe output in >> wtxids.txt & run python3 test_scripts/validate_wtxids.py
// for tx in &block_ordered {
// 	println!("{},{},{}", tx.meta.txid_hex, tx.meta.wtxid_hex, tx.meta.json_path.as_ref().unwrap());
// }
