use super::construct_coinbase::{get_merkle_root, CoinbaseTxData};
use crate::{parsing::transaction_structs::Transaction, validation::utils::double_hash};
use hex_literal::hex as hexlit;
use num_bigint::BigUint;
use std::time::{SystemTime, UNIX_EPOCH};

// changes the 4 byte nonce at the end of the header to change the HASH256
// so long till the header + nonce produce a HASH256 below the specified target
// Comparison of the hash against the target happens as BigUint integer
// returns: nonce that produces a valid hash as u32
fn mine_nonce(block_header: &[u8]) -> u32 {
    let target = BigUint::from_bytes_be(&hexlit!(
        "00000ffff0000000000000000000000000000000000000000000000000000000"
    ));
    let max_nonce = std::u32::MAX;
    let mut candidate = block_header.to_vec();
    candidate.extend(0_u32.to_le_bytes());

    for nonce in 0..=max_nonce {
        let len = candidate.len();
        candidate[len - 4..].copy_from_slice(&u32::to_le_bytes(nonce));
        let block_hash: Vec<u8> = double_hash(&candidate);

        let block_hash_num = BigUint::from_bytes_le(&block_hash);

        if block_hash_num < target {
            return nonce;
        };
    }
    panic!("All nonces used in mining!");
}

// assembles the blockheader according to the specification using hardcoded previous block, version
// and target according to the exercise
pub fn construct_header(
    block_transactions: &Vec<Transaction>,
    coinbase_tx: &CoinbaseTxData,
) -> Vec<u8> {
    let mut block_header: Vec<u8> = Vec::new();

    block_header.extend(hexlit!("20000000")); // version not signaling updates
    let previous_block_bytes: Vec<u8> =
        hexlit!("00000000000000000001901b9f3b6c7a0c34b20b29b950d0d8ffa36c63979c1c")
            .into_iter()
            .rev()
            .collect();
    block_header.extend(previous_block_bytes); // rev bytes of previous block hash (natural order)

    let mut txids_bytes: Vec<Vec<u8>> = Vec::new();
    txids_bytes.push(coinbase_tx.txid_natural_bytes.clone());
    for tx in block_transactions {
        let txid_bytes =
            hex::decode(&tx.meta.txid_hex).expect("construct_header: Error decoding hex ");
        let rev_txid_bytes: Vec<u8> = txid_bytes.into_iter().rev().collect();
        txids_bytes.push(rev_txid_bytes);
    }
    block_header.extend(get_merkle_root(&txids_bytes)); // merkle root

    if let Ok(time_sec) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let time_sec: u32 = time_sec.as_secs() as u32;
        block_header.extend(time_sec.to_le_bytes());
    } else {
        panic!("Error getting unix time in header construction!")
    };

    let target_bits = u32::to_le_bytes(0x1f00ffff); // target
    block_header.extend(target_bits);
    let nonce: u32 = mine_nonce(&block_header);
    block_header.extend(nonce.to_le_bytes());
    block_header
}
