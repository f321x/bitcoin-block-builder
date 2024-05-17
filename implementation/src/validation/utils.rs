use crate::parsing::transaction_structs::TxIn;
use num_traits::ToPrimitive;
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

// returns: outpoint (rev txid bytes + index) of TxIn as serialized byte Vec<u8>
pub fn get_outpoint(input: &TxIn) -> Vec<u8> {
    let mut outpoint: Vec<u8> = hex::decode(&input.txid)
        .expect("Failed to decode transaction ID")
        .into_iter()
        .rev()
        .collect();
    let outpoint_index = input.vout.to_le_bytes();
    outpoint.extend_from_slice(&outpoint_index);
    outpoint
}

// returns: sha256 digest of passed byte slice as Vec<u8>
pub fn hash_sha256(preimage: &[u8]) -> Vec<u8> {
    let digest = preimage.to_owned();
    let mut hasher = Sha256::new();

    hasher.update(&digest);
    hasher.finalize_reset().to_vec()
}

// Hashes byte slice argument bytes twice
// returns: Vec<u8> of the second hash bytes
pub fn double_hash(preimage: &[u8]) -> Vec<u8> {
    let mut digest: Vec<u8> = preimage.to_owned();

    for _ in 0..2 {
        digest = hash_sha256(&digest);
    }
    digest
}

// applies sha256 and ripemd160 hash on passed byte slice
// returns: 20 byte hash as Vec<u8>
pub fn hash160(preimage: &[u8]) -> Vec<u8> {
    let preimage = hash_sha256(preimage);
    let mut hasher = Ripemd160::new();
    hasher.update(&preimage);
    hasher.finalize().to_vec()
}

// converts a given u128 integer to a little endian Vec<u8>
// with variable size according to bitcoin wiki specification
pub fn varint(n: u128) -> Vec<u8> {
    if n <= 252 {
        vec![n as u8]
    } else if n <= 0xffff {
        let mut bytes = vec![0xfd];
        bytes.extend(&(n as u16).to_le_bytes());
        bytes
    } else if n <= 0xffffffff {
        let mut bytes = vec![0xfe];
        bytes.extend(&(n as u32).to_le_bytes());
        bytes
    } else if n <= 0xffffffffffffffff {
        let mut bytes = vec![0xff];
        bytes.extend(&(n as u64).to_le_bytes());
        bytes
    } else {
        panic!("Varint: Values larger than 0xffffffffffffffff not supported")
    }
}

// When used as numbers, byte vectors are interpreted as little-endian variable-length integers with the most significant
// bit determining the sign of the integer. Thus 0x81 represents -1. 0x80 is another representation of zero
// (so called negative 0). Positive 0 is represented by a null-length vector.
// Byte vectors are interpreted as Booleans where
// False is represented by any representation of zero and True is represented by any representation of non-zero.
pub fn decode_num(number: &[u8]) -> i128 {
    let number = num_bigint::BigInt::from_signed_bytes_le(number);
    number.to_i128().expect("number outside of i128 scope")
}
