mod script;
mod signature_verification;
pub mod utils;
pub mod validate_parsing;
pub mod validate_values;
pub mod weight_calculation;

use self::signature_verification::{verify_p2pkh, verify_p2wpkh};
use self::validate_parsing::validate_txid_hash_filename;
use self::validate_values::{validate_feerate, validate_values_and_set_fee};
use self::weight_calculation::validate_and_set_weight;
use crate::parsing::transaction_structs::{InputType, Transaction};

pub enum ValidationResult {
    Valid,
    Invalid(String), // String = reason
}

// Sanity checks to sort out impossible transactions before doing
// more compute intensive signature verification. Gets called on each Transaction.
// Also sets weight and fee in the Transaction while calculating it for the checks.
// returns: ValidationResult
fn sanity_checks(tx: &mut Transaction) -> ValidationResult {
    if !validate_values_and_set_fee(tx) {
        return ValidationResult::Invalid("Values don't add up.".to_string());
    }
    if !validate_txid_hash_filename(tx) {
        return ValidationResult::Invalid("Txid does not represent filename!".to_string());
    }
    if !validate_and_set_weight(tx) {
        return ValidationResult::Invalid("Transaction weight too high!".to_string());
    }
    if !validate_feerate(tx) {
        return ValidationResult::Invalid("too low feerate".to_string());
    }
    ValidationResult::Valid
}

// takes a transaction and calls the according signature/script verification
// function on each input. Implemented checks for p2pkh and p2wpkh.
// returns: ValidationResult
fn signature_verification(tx: &Transaction) -> ValidationResult {
    for txin in &tx.vin {
        let tx_type = &txin.in_type;
        let result = match tx_type {
            InputType::P2WPKH => verify_p2wpkh(tx, txin),
            InputType::P2PKH => verify_p2pkh(tx, txin),
            _ => {
                // println!("Unknown type: {:#?}", tx_type);
                ValidationResult::Invalid("Input type not implemented!".to_string())
            }
        };
        match result {
            ValidationResult::Valid => (),
            ValidationResult::Invalid(msg) => {
                return ValidationResult::Invalid(msg);
            }
        }
    }
    ValidationResult::Valid
}

// implements validate function that does sanity checks and cryptographic verification
// returns: ValidationResult enum either ::Valid or ::Invalid(reason String)
impl Transaction {
    pub fn validate(&mut self) -> ValidationResult {
        match sanity_checks(self) {
            ValidationResult::Valid => (),
            ValidationResult::Invalid(msg) => {
                return ValidationResult::Invalid(msg);
            }
        }
        match signature_verification(self) {
            ValidationResult::Valid => (),
            ValidationResult::Invalid(msg) => {
                return ValidationResult::Invalid(msg);
            }
        }
        ValidationResult::Valid
    }
}
