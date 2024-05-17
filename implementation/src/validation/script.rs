use byteorder::{ByteOrder, LittleEndian};
use core::panic;
use hex_literal::hex as hexlit;
use secp256k1::{ecdsa::Signature, Message, PublicKey};
use std::collections::VecDeque;
use std::error::Error;

use super::utils::{decode_num, double_hash, get_outpoint, hash160, hash_sha256, varint};
use super::validate_parsing::serialize_output;
use crate::parsing::transaction_structs::{InputType, Transaction, TxIn};

// Implementation of Script opcodes for use in tx verification
// The Stack is represented as VecDeque<Vec<u8>>
// If an opcode returns Err(reason) script execution fails.
// Entry is fn evaluate_script()

fn op_swap(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    if stack.len() >= 2 {
        let len = stack.len();
        stack.swap(len - 1, len - 2);
        return Ok(());
    }
    Err("OP_SWAP stack < 2")
}

fn op_equal(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    if stack.len() >= 2 {
        let last = &stack.pop_back().expect("Unwrap op_equal");
        let second_last = &stack.pop_back().expect("OP_Equal");
        if last == second_last {
            stack.push_back(vec![1u8]);
            return Ok(());
        }
    } else {
        return Err("OP_EQUAL stack len < 2");
    }
    stack.push_back(Vec::new());
    Ok(())
}

fn op_rot(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    if stack.len() >= 3 {
        let third_item = stack.pop_back().expect("OP_ROT pop_back");
        let second_item = stack.pop_back().expect("OP_ROT pop_back");
        let first_item = stack.pop_back().expect("OP_ROT pop_back");
        stack.push_back(second_item);
        stack.push_back(first_item);
        stack.push_back(third_item);
        return Ok(());
    }
    Err("OP_ROT stack len < 3")
}

fn op_size(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    if !stack.is_empty() {
        if let Some(last) = stack.back() {
            let length = last.len();
            let length_bytes = length.to_le_bytes().to_vec();
            stack.push_back(length_bytes);
            return Ok(());
        } else {
            return Err("OP_SIZE getting last element failed");
        }
    }
    Err("OP_SIZE stack empty")
}

fn op_over(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    let stack_len = stack.len();
    if stack_len >= 2 {
        if let Some(second_element) = stack.get(stack_len - 2) {
            stack.push_back(second_element.clone());
            return Ok(());
        } else {
            return Err("OP_OVER getting second element failed");
        }
    }
    Err("OP_OVER stack < 2")
}

fn op_greaterthan(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    let stack_size = stack.len();
    if stack_size >= 2 {
        if let Some(b) = stack.pop_back() {
            if let Some(a) = stack.pop_back() {
                let a = decode_num(&a);
                let b = decode_num(&b);
                if a > b {
                    stack.push_back(vec![1u8]);
                } else {
                    stack.push_back(Vec::new());
                }
                return Ok(());
            } else {
                return Err("OP_GREATERTHAN second element pop failed");
            }
        } else {
            return Err("OP_GREATERTHAN first element pop failed");
        }
    }
    Err("OP_GREATERTHAN stack < 2")
}

fn op_equalverify(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    op_equal(stack)?;
    if let Some(bool) = stack.pop_back() {
        if bool.is_empty() {
            Err("Equalverify false")
        } else {
            Ok(())
        }
    } else {
        Err("OP_EQUALVERIFY stack pop failed")
    }
}

fn op_ifdup(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    let length = stack.len();
    if length < 1 {
        return Err("OP_IFDUP length < 1");
    };
    if let Some(last_item) = stack.get(length - 1) {
        if last_item.is_empty() {
            return Ok(());
        } else {
            stack.push_back(last_item.clone());
        }
        Ok(())
    } else {
        Err("OP_IFDUP getting last element failed")
    }
}

// Marks transaction as invalid if the relative lock time of the input (enforced by BIP 0068 with nSequence)
// is not equal to or longer than the value of the top stack item. The precise semantics are described in BIP 0112.
fn op_checksequenceverify(
    stack: &mut VecDeque<Vec<u8>>,
    txin: &TxIn,
    tx: &Transaction,
) -> Result<(), &'static str> {
    let sequence = txin.sequence;
    let disable_flag = 1 << 31;
    let locktime_mask = 0x0000ffff;
    let time_flag = 1 << 22;
    if stack.is_empty() {
        return Err("OP_CSV stack empty");
    };

    if let Some(locktime_element) = stack.pop_back() {
        let number = decode_num(&locktime_element);
        if number < 0 || locktime_element.is_empty() {
            return Err("OP_CSV number < 0 or empty");
        };
        let number = number as u32;

        if (number & disable_flag) == 0 {
            if tx.version < 2 {
                return Err("OP_CSV Transaction version is less than 2.");
            };
            if (sequence & disable_flag) != 0 {
                return Err("OP_CSV Transaction input sequence number disable flag is set.");
            };
            if (number & time_flag) != (sequence & time_flag) {
                return Err("OP_CSV Relative lock-time types are not the same.");
            };

            let locktime_sequence = sequence & locktime_mask;
            let locktime_stack = number & locktime_mask;
            if locktime_stack > locktime_sequence {
                return Err("OP_CSV Stack > Sequence LT");
            };
        }
    } else {
        return Err("OP_CSV time pop from stack failed.");
    }
    Ok(())
}

fn op_checklocktimeverify(
    stack: &mut VecDeque<Vec<u8>>,
    tx: &Transaction,
    txin: &TxIn,
) -> Result<(), String> {
    if stack.is_empty() {
        return Err("OP_CLTV stack empty".to_string());
    };
    if let Some(top_item) = stack.pop_back() {
        let decoded_number = decode_num(&top_item);

        if decoded_number < 0 {
            return Err("OP_CLTV number < 0".to_string());
        };
        let decoded_number: u32 = decoded_number as u32;
        if (decoded_number < 500000000 && tx.locktime > 500000000)
            || (decoded_number > 500000000 && tx.locktime < 500000000)
        {
            return Err("OP_CLTV different locktime types".to_string());
        }
        if tx.locktime < decoded_number {
            return Err(format!(
                "OP_CLTV locktime {} < {} stack num.",
                tx.locktime, decoded_number
            ));
        }
        if txin.sequence == 0xffffffff {
            return Err("OP_CLTV in sequence is 0xffffffff".to_string());
        }
    } else {
        return Err("OP_CLTV pop item failed".to_string());
    };
    Ok(())
}

// serializes input of legacy transaction into Vec<u8>
// all inputs except the one that is being verified (parameter) will be returned as 0x00
// returns: byte serialized input as Vec<u8>
fn serialize_input_legacy(input: &TxIn, signing_txin: &TxIn) -> Vec<u8> {
    let mut serialized_input = get_outpoint(input);

    if input == signing_txin {
        let scriptpubkey_len = varint(
            hex::decode(&signing_txin.prevout.scriptpubkey)
                .expect("serialize_input_legacy hex encoding")
                .len() as u128,
        );
        serialized_input.extend(scriptpubkey_len);
        serialized_input.extend(
            hex::decode(&signing_txin.prevout.scriptpubkey)
                .expect("OP_CHECKSIG scriptpubkey hex decode failed"),
        );
    } else {
        serialized_input.extend(hexlit!("00"));
    }
    serialized_input.extend(input.sequence.to_le_bytes());
    serialized_input
}

// Serialize legacy transaction (non segwit) for signature verification of specified input
// returns: double SHA256 digest of serialized transaction
fn serialize_legacy_tx(tx: &Transaction, signing_txin: &TxIn, sighash: u32) -> Vec<u8> {
    let mut preimage: Vec<u8> = Vec::new();

    preimage.extend(&tx.version.to_le_bytes()); // VERSION
    preimage.extend(varint(tx.vin.len() as u128)); // INPUT amount
    for tx_in in &tx.vin {
        preimage.append(&mut serialize_input_legacy(tx_in, signing_txin));
    }
    preimage.extend(varint(tx.vout.len() as u128)); // Output amount
    for tx_out in &tx.vout {
        preimage.append(&mut serialize_output(tx_out));
    }
    preimage.extend(tx.locktime.to_le_bytes());
    preimage.extend(sighash.to_le_bytes());
    double_hash(&preimage)
}

// Verify DER encoded signature against message and pubkey
fn verify_sig_op_checksig(msg: &[u8], pubkey: &[u8], sig: &[u8]) -> Result<(), String> {
    let sig = Signature::from_der(sig);
    let mut sig = match sig {
        Ok(value) => value,
        Err(err) => {
            return Err(format!("Loading DER encoded signature failed: {}", err));
        }
    };
    Signature::normalize_s(&mut sig);
    let msg: [u8; 32] = msg.try_into().expect("Commitment hash is not 32 byte!");
    let msg = Message::from_digest(msg);
    let pubkey = PublicKey::from_slice(pubkey).expect("Pubkey invalid!");
    let result = sig.verify(&msg, &pubkey);
    match result {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Signature verification failed: {}", err)),
    }
}

// implemented for non-witness transactions and SIGHASH_ALL only
fn op_checksig(stack: &mut VecDeque<Vec<u8>>, tx: &Transaction, txin: &TxIn) -> Result<(), String> {
    if stack.len() < 2 {
        return Err("OP_CHECKSIG stack < 2".to_string());
    };
    let pubkey = if let Some(pubkey) = stack.pop_back() {
        pubkey
    } else {
        return Err("OP_CHECKSIG popping pubkey from stack failed!".to_string());
    };
    let mut der_signature = if let Some(signature) = stack.pop_back() {
        signature
    } else {
        return Err("OP_CHECKSIG popping signature from stack failed!".to_string());
    };
    let sighash: u32 = if let Some(sighash_byte) = der_signature.pop() {
        sighash_byte as u32
    } else {
        return Err("OP_CHECKSIG popping sighash from signature failed".to_string());
    };
    if sighash != 0x00000001 {
        // SIGHASH_ALL
        return Err("sighash not implemented".to_string());
    }
    let message = match txin.in_type {
        InputType::P2PKH => serialize_legacy_tx(tx, txin, sighash),
        InputType::P2SH => serialize_legacy_tx(tx, txin, sighash),
        _ => panic!("op_checksig unsupported txtype"),
    };
    match verify_sig_op_checksig(&message, &pubkey, &der_signature) {
        Ok(_) => stack.push_back(vec![1u8]),
        Err(_) => stack.push_back(vec![]),
    }
    Ok(())
}

fn op_verify(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    if let Some(top_stack_element) = stack.pop_back() {
        if top_stack_element.is_empty() {
            Err("OP_VERIFY not valid")
        } else {
            Ok(())
        }
    } else {
        Err("OP_VERIFY popping top stack element failed")
    }
}

fn op_pushnum(stack: &mut VecDeque<Vec<u8>>, amount: u8) -> Result<(), &'static str> {
    let number: u8 = amount - 80;
    let number_bytes: Vec<u8> = vec![number];
    stack.push_back(number_bytes);
    Ok(())
}

fn op_pushbytes(
    stack: &mut VecDeque<Vec<u8>>,
    index: &mut usize,
    script: &[u8],
) -> Result<(), &'static str> {
    let opcode: u8 = script[*index];
    let mut bytes: Vec<u8> = Vec::new();

    if *index + opcode as usize <= script.len() {
        bytes.resize(opcode as usize, 0);
        bytes.clone_from_slice(&script[*index + 1..*index + 1 + opcode as usize]);
        stack.push_back(bytes);
        *index += opcode as usize;
    } else {
        return Err("OP_PUSHBYTES opcode out of range");
    }
    Ok(())
}

pub fn get_pushdata_amount(
    script: &[u8],
    amount_bytes: u8,
    current_index: usize,
) -> Result<u32, &'static str> {
    let mut amount_of_bytes_to_push: Vec<u8> = vec![0; amount_bytes as usize];

    amount_of_bytes_to_push
        .clone_from_slice(&script[current_index + 1..current_index + 1 + amount_bytes as usize]);
    match amount_bytes {
        1 => Ok(amount_of_bytes_to_push[0] as u32),
        2 => Ok(LittleEndian::read_u16(&amount_of_bytes_to_push) as u32),
        4 => Ok(LittleEndian::read_u32(&amount_of_bytes_to_push)),
        _ => Err("get_pushdata_amount weird amount in match"),
    }
}

fn op_pushdata(
    stack: &mut VecDeque<Vec<u8>>,
    amount_bytes: u8,
    index: &mut usize,
    script: &[u8],
) -> Result<(), &'static str> {
    let mut data_push: Vec<u8> = Vec::new();

    let amount_of_bytes_to_push = get_pushdata_amount(script, amount_bytes, *index)?;
    *index += amount_bytes as usize + 1;
    data_push.resize(amount_of_bytes_to_push as usize, 0);
    data_push.clone_from_slice(&script[*index..*index + amount_of_bytes_to_push as usize]);
    stack.push_back(data_push);
    *index += amount_of_bytes_to_push as usize - 1;
    Ok(())
}

fn op_depth(stack: &mut VecDeque<Vec<u8>>) -> Result<(), &'static str> {
    stack.push_back(varint(stack.len() as u128));
    Ok(())
}

// NULL sig1 sig2 ... <number of signatures> pub1 pub2 <number of public keys>
fn op_checkmultisig(
    stack: &mut VecDeque<Vec<u8>>,
    tx: &Transaction,
    txin: &TxIn,
) -> Result<(), &'static str> {
    let mut signatures: VecDeque<Vec<u8>> = VecDeque::new();
    let mut pubkeys: VecDeque<Vec<u8>> = VecDeque::new();
    let number_of_pubkeys;
    let mut number_of_signatures;

    if let Some(pubkey_amount) = stack.pop_back() {
        number_of_pubkeys = pubkey_amount[0]; // should be enough for these scripts
        for _ in 0..number_of_pubkeys {
            if let Some(pubkey) = stack.pop_back() {
                pubkeys.push_back(pubkey);
            } else {
                return Err("OP_CHECKMULTISIG error popping pubkey from stack");
            };
        }
    } else {
        return Err("OP_CHECKMULTISIG error popping number of pubkeys");
    };
    if let Some(signature_amount) = stack.pop_back() {
        number_of_signatures = signature_amount[0];
        for _ in 0..number_of_signatures {
            if let Some(signature) = stack.pop_back() {
                signatures.push_front(signature);
            } else {
                return Err("OP_CHECKMULTISIG error popping signature from stack");
            };
        }
        stack.pop_back(); // OP_CHECKMULTISIG BUG
    } else {
        return Err("OP_CHECKMULTISIG error popping number of signatures");
    };

    'outer: for mut signature in signatures {
        let mut retry = true;

        let sighash: u32 = if let Some(sighash_byte) = signature.pop() {
            sighash_byte as u32
        } else {
            return Err("OP_CHECKSIG popping sighash from signature failed");
        };
        if sighash != 0x00000001 {
            // IMPLEMENT OTHER SIGHASH TYPES
            return Err("OP_CHECKMULTISIG sighash not implemented");
        };
        let message = match txin.in_type {
            InputType::P2SH => serialize_legacy_tx(tx, txin, sighash),
            _ => panic!("op_checkmultisig unsupported txtype"),
        };

        while retry {
            if let Some(pubkey) = pubkeys.pop_back() {
                retry = false;
                match verify_sig_op_checksig(&message, &pubkey, &signature) {
                    Ok(_) => {
                        number_of_signatures -= 1;
                    }
                    Err(err) => {
                        println!("{}", err);
                        retry = true;
                    }
                };
            } else {
                break 'outer;
            };
        }
    }
    if number_of_signatures == 0 {
        stack.push_back(vec![1u8]);
    } else {
        stack.push_back(vec![]);
    };
    Ok(())
}

// main script interpretion function
// executes the script argument and returns Ok() if the script is valid and True
pub fn evaluate_script(
    script: Vec<u8>,
    txin: &TxIn,
    tx: &Transaction,
) -> Result<(), Box<dyn Error>> {
    let mut stack: VecDeque<Vec<u8>> = VecDeque::new();
    let mut index = 0;

    while index < script.len() {
        let opcode = script[index];
        match opcode {
            0xa8 => {
                // SHA256
                if let Some(last) = stack.pop_back() {
                    stack.push_back(hash_sha256(&last));
                } else {
                    return Err("OP_SHA256 stack empty".into());
                }
            }
            0xa9 => {
                // OP_HASH160
                if let Some(last) = stack.pop_back() {
                    stack.push_back(hash160(&last));
                } else {
                    return Err("OP_HASH160 stack empty".into());
                }
            }
            0x75 => {
                if stack.pop_back().is_none() {
                    return Err("OP_DROP stack empty".into());
                }
            } // OP_DROP
            0x7c => op_swap(&mut stack)?,        // OP_SWAP
            0x00 => stack.push_back(Vec::new()), // OP_0
            0x76 => {
                // OP_DUP
                if let Some(last) = stack.back() {
                    stack.push_back(last.clone());
                } else {
                    return Err("OP_DUP stack empty.".into());
                }
            }
            0x87 => op_equal(&mut stack)?,       // OP_EQUAL
            0x7b => op_rot(&mut stack)?,         // OP_ROT
            0x82 => op_size(&mut stack)?,        // OP_SIZE
            0x78 => op_over(&mut stack)?,        // OP_OVER
            0xa0 => op_greaterthan(&mut stack)?, // OP_GREATERTHAN
            0x88 => op_equalverify(&mut stack)?, // OP_EQUALVERIFY
            0x73 => op_ifdup(&mut stack)?,       // OP_IFDUP
            0xb2 => op_checksequenceverify(&mut stack, txin, tx)?, // OP_CSV
            0xb1 => op_checklocktimeverify(&mut stack, tx, txin)?, // OP_CLTV
            0xac => op_checksig(&mut stack, tx, txin)?, // OP_CHECKSIG
            0x74 => op_depth(&mut stack)?,       // OP_DEPTH
            0xad => {
                // OP_CHECKSIGVERIFY
                op_checksig(&mut stack, tx, txin)?;
                op_verify(&mut stack)?;
            }
            0x51..=0x60 => op_pushnum(&mut stack, opcode)?, // OP_PUSHNUM (1-16)
            0x4f => stack.push_back(vec![255]),             // OP_1NEGATE
            0x01..=0x4b => op_pushbytes(&mut stack, &mut index, &script)?, // OP_PUSHBYTES
            0x4c => op_pushdata(&mut stack, 1, &mut index, &script)?, // OP_PUSHDATA1
            0x4d => op_pushdata(&mut stack, 2, &mut index, &script)?, // OP_PUSHDATA2
            0x4e => op_pushdata(&mut stack, 4, &mut index, &script)?, // OP_PUSHDATA4
            0xae => op_checkmultisig(&mut stack, tx, txin)?, // OP_CHECKMULTISIG
            _ => panic!("no script operator found!"),
        };
        index += 1;
    }
    if let Some(last) = stack.pop_back() {
        if last.is_empty() {
            return Err("SCRIPT INVALID".into());
        };
    }
    Ok(())
}
