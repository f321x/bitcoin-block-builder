use crate::validation::utils::{double_hash, varint};
use crate::{parsing::transaction_structs::Transaction, validation::validate_parsing::get_txid};
use hex_literal::hex as hexlit;

pub struct CoinbaseTxData {
    pub txid_hex: String,
    pub txid_natural_bytes: Vec<u8>,
    pub assembled_tx: Vec<u8>,
}

// calculates the HASH256 merkle root of a Vec of Vec<u8> ([w]txids).
// returns: root 32byte hash of the (w)txid structure as Vec<u8>.
pub fn get_merkle_root(block_txs: &[Vec<u8>]) -> Vec<u8> {
    let mut merkle_tree: Vec<Vec<u8>> = block_txs.to_owned();

    if merkle_tree.len() == 1 {
        return merkle_tree[0].clone();
    }

    while merkle_tree.len() > 1 {
        if merkle_tree.len() % 2 != 0 {
            let last: Vec<u8> = merkle_tree.last().unwrap().clone();
            merkle_tree.push(last);
        }

        let mut next_stage: Vec<Vec<u8>> = Vec::new();

        for i in (0..merkle_tree.len()).step_by(2) {
            let first = &merkle_tree[i];
            let second = &merkle_tree[i + 1];

            let mut concat = first.clone();
            concat.extend(second);

            let hash = double_hash(&concat);
            next_stage.push(hash);
        }
        merkle_tree = next_stage;
    }
    merkle_tree[0].clone()
}

// assembles the scriptpubkey for use as witness commitment in the coinbase tx.
// calculates the witness root hash and prepends it with the according opcodes
// ready for use as scriptpubkey returned as Vec<u8>
fn calc_wtxid_commitment_scriptpubkey(block_txs: &Vec<Transaction>) -> Vec<u8> {
    let mut txids_bytes: Vec<Vec<u8>> = Vec::new();

    txids_bytes
        .push(hexlit!("0000000000000000000000000000000000000000000000000000000000000000").to_vec());
    for tx in block_txs {
        let txid_bytes = hex::decode(&tx.meta.wtxid_hex)
            .expect("calc_wtxid_commitment_scriptpubkey: Error decoding hex ");
        let rev_txid_bytes: Vec<u8> = txid_bytes.into_iter().rev().collect();
        txids_bytes.push(rev_txid_bytes);
    }
    let mut wtxid_merkle_root = get_merkle_root(&txids_bytes);
    wtxid_merkle_root.extend(hexlit!(
        "0000000000000000000000000000000000000000000000000000000000000000"
    ));
    let witness_commitment = double_hash(&wtxid_merkle_root);
    let mut witness_commitment_scriptpubkey = hexlit!("6a24aa21a9ed").to_vec(); // OP_RETURN + len + witness code
    witness_commitment_scriptpubkey.extend(&witness_commitment);
    witness_commitment_scriptpubkey
}

// returns the sum of all fees in a Vec<Transaction>
fn count_fees(block_txs: &Vec<Transaction>) -> u64 {
    let mut all_fees = 0;

    for tx in block_txs {
        all_fees += tx.meta.fee;
    }
    all_fees
}

// serializes the coinbase transaction as Vec<u8>. If is_segwit is true it will include marker, flag
// and the witness reserved value.
fn serialize_coinbase_transaction(block_txs: &Vec<Transaction>, is_segwit: bool) -> Vec<u8> {
    let mut coinbase_transaction: Vec<u8> = Vec::new();
    let wtxid_commitment_scriptpubkey: Vec<u8> = calc_wtxid_commitment_scriptpubkey(block_txs);
    let reward: u64 = count_fees(block_txs) + 625000000;

    coinbase_transaction.extend(hexlit!("01000000")); // version
    if is_segwit {
        coinbase_transaction.extend(hexlit!("0001")); // marker + flag
    }
    coinbase_transaction.extend(hexlit!(
        "010000000000000000000000000000000000000000000000000000000000000000ffffffff"
    )); // input count + input + index
    let mut scriptsig = varint(varint(839653).len() as u128); //pushbytes len blockheight
    scriptsig.extend(varint(839653)); // blockheight
    scriptsig.extend(hexlit!("1043797068657270756E6B467574757265")); // this is 16 + secret ascii message :)
    coinbase_transaction.extend(varint(scriptsig.len() as u128));
    coinbase_transaction.extend(scriptsig);
    coinbase_transaction.extend(hexlit!("ffffffff")); // sequence
    coinbase_transaction.extend(hexlit!("02")); // 2 outputs (reward and witness commitment op_return)
    coinbase_transaction.extend(reward.to_le_bytes());
    coinbase_transaction.extend(varint(
        hexlit!("001435f6de260c9f3bdee47524c473a6016c0c055cb9").len() as u128,
    )); // reward p2wpkh scriptpubkey
    coinbase_transaction.extend(hexlit!("001435f6de260c9f3bdee47524c473a6016c0c055cb9"));
    coinbase_transaction.extend(hexlit!("0000000000000000")); // witness amount
    coinbase_transaction.extend(varint(wtxid_commitment_scriptpubkey.len() as u128)); // len wtxid commitment
    coinbase_transaction.extend(wtxid_commitment_scriptpubkey);
    // amnt witness stack items + len witness reserved value + value
    if is_segwit {
        coinbase_transaction.extend(hexlit!(
            "01200000000000000000000000000000000000000000000000000000000000000000"
        ));
    }
    coinbase_transaction.extend(hexlit!("00000000")); // locktime
    coinbase_transaction
}

// entry function to assemble the coinbase transaction which is returned as CoinbasTxData struct
pub fn assemble_coinbase_transaction(block_txs: &Vec<Transaction>) -> CoinbaseTxData {
    let coinbase_tx_witness = serialize_coinbase_transaction(block_txs, true);
    let coinbase_tx_no_witness = serialize_coinbase_transaction(block_txs, false);

    CoinbaseTxData {
        txid_hex: hex::encode(get_txid(&coinbase_tx_no_witness)),
        txid_natural_bytes: double_hash(&coinbase_tx_no_witness),
        assembled_tx: coinbase_tx_witness,
    }
}
