pub mod mining;
pub mod parsing;
mod utils_main;
pub mod validation;

use mining::{mine_block, Block};
use parsing::{parse_transactions_from_dir, transaction_structs::Transaction};
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use utils_main::remove_invalid_transactions;
use validation::ValidationResult;

// writes the final content stored in the Block struct to the passed output_path
// as output.txt formatted according to the exercise specification
fn output_block(mined_block: &Block, output_path: &str) {
    let mut output_file = File::create(output_path).expect("Unable to create output file");

    writeln!(output_file, "{}", mined_block.header_hex).expect("Unable to write to file");
    writeln!(output_file, "{}", mined_block.coinbase_tx_hex).expect("Unable to write to file");

    let len = mined_block.txids_hex.len();
    for (index, tx) in mined_block.txids_hex.iter().enumerate() {
        if index < len - 1 {
            writeln!(output_file, "{}", tx).expect("Unable to write to file");
        } else {
            write!(output_file, "{}", tx).expect("Unable to write to file");
        }
    }
}

// calls validate() on each Transaction in the passed Vec of Transaction
// returns: HashSet(txid as hex String) of all invalid and untested transactions
fn validate_transactions(parsed_transactions: &mut Vec<Transaction>) -> HashSet<String> {
    let mut invalid_transactions: HashSet<String> = HashSet::new();

    for tx in parsed_transactions {
        match tx.validate() {
            ValidationResult::Valid => {}
            ValidationResult::Invalid(_) => {
                invalid_transactions.insert(tx.meta.txid_hex.clone());
            }
        }
    }
    invalid_transactions
}

fn main() {
    // parses all json transactions in a Vec of Transaction structs
    let mut parsed_transactions = parse_transactions_from_dir("../mempool");

    // creates a Hashset of the TXIDs of all invalid and non verified transactions
    let invalid_transactions = validate_transactions(&mut parsed_transactions);

    // stores all transactions that are not invalid in a HashMap (TXID(hex String), Transaction Struct)
    let mut valid_transactions =
    remove_invalid_transactions(parsed_transactions, invalid_transactions);

    // returns a Block struckt containing header, coinbase and final transaction list
    let block: Block = mine_block(&mut valid_transactions);

    // writes blockfile to output.txt according to exercise specification
    output_block(&block, "../../output.txt");
    println!(
        "\nDone. Number of mined transactions: {}\n",
        &block.txids_hex.len()
    );
}
