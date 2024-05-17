// Definition of data structures to hold a bitcoin transaction and relevant metadata

use crate::validation::utils::{get_outpoint, varint};
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct TxOut {
    #[serde_as(as = "NoneAsEmptyString")]
    pub scriptpubkey: Option<String>,
    pub scriptpubkey_asm: String,
    pub scriptpubkey_type: String,
    pub scriptpubkey_address: Option<String>,
    pub value: u64,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Script {
    pub scriptpubkey: String,
    pub scriptpubkey_asm: String,
    pub scriptpubkey_type: String,
    pub scriptpubkey_address: Option<String>,
    pub value: u64,
}

#[serde_as]
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct TxIn {
    #[serde(skip_deserializing)]
    pub in_type: InputType,
    pub txid: String,
    pub vout: u32,
    #[serde_as(as = "NoneAsEmptyString")]
    pub scriptsig: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub scriptsig_asm: Option<String>,
    pub prevout: Script,
    pub witness: Option<Vec<String>>,
    pub inner_witnessscript_asm: Option<String>,
    pub inner_redeemscript_asm: Option<String>,
    pub is_coinbase: bool,
    pub sequence: u32,
}

#[derive(Default, Debug, Clone)]
pub struct Packet {
    pub packet_weight: u64,
    pub packet_fee_sat: u64,
    pub packet_feerate_weight: u64, // sat/weight_unit
}

#[derive(Default, Debug, Clone)]
pub struct TxMetadata {
    pub json_path: Option<String>,
    pub txid_hex: String,
    pub wtxid_hex: String,
    pub packet_data: Packet,
    pub weight: u64,
    pub fee: u64,
    pub parents: Option<Vec<String>>,
}

// main Transaction struct, containing all other transaction (meta-)data
#[derive(Deserialize, Debug, Clone)]
pub struct Transaction {
    #[serde(skip_deserializing)]
    pub meta: TxMetadata,
    pub version: i32,
    pub locktime: u32,
    pub vin: Vec<TxIn>,
    pub vout: Vec<TxOut>,
}

impl Transaction {
    // return Vec<u8> of all sequences in little endian byte format
    pub fn serialize_all_sequences(&self) -> Vec<u8> {
        let mut all_sequences = Vec::new();
        for input in &self.vin {
            all_sequences.extend(input.sequence.to_le_bytes());
        }
        all_sequences
    }

    // return Vec<u8> of all outpoints of referenced Transaction
    pub fn serialize_all_outpoints(&self) -> Vec<u8> {
        let mut all_outpoints = Vec::new();
        for input in &self.vin {
            all_outpoints.extend(get_outpoint(input));
        }
        all_outpoints
    }

    // return all outputs of Transaction serialized as Vec<u8>
    pub fn serialize_all_outputs(&self) -> Vec<u8> {
        let mut all_outputs = Vec::new();
        for output in &self.vout {
            all_outputs.extend(&output.value.to_le_bytes());
            if let Some(scriptpubkey) = &output.scriptpubkey {
                all_outputs.extend(varint(hex::decode(scriptpubkey).unwrap().len() as u128));
                all_outputs.extend(hex::decode(scriptpubkey).unwrap());
            } else {
                panic!("No scriptpubkey in output!");
            }
        }
        all_outputs
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum InputType {
    P2TR,
    P2PKH,
    P2SH,
    P2WPKH,
    P2WSH,
    UNKNOWN(String),
}

impl Default for InputType {
    fn default() -> Self {
        InputType::UNKNOWN("notSerialized".to_string())
    }
}

impl InputType {
    // can be applied on TxIn to set the according InputType
    pub fn fetch_type(txin: &mut TxIn) {
        let type_string = &txin.prevout.scriptpubkey_type;
        txin.in_type = match type_string.as_str() {
            "v1_p2tr" => InputType::P2TR,
            "v0_p2wpkh" => InputType::P2WPKH,
            "v0_p2wsh" => InputType::P2WSH,
            "p2sh" => InputType::P2SH,
            "p2pkh" => InputType::P2PKH,
            _ => InputType::UNKNOWN(type_string.to_string()),
        };
    }
}

// Sample Transaction:

// # {
//     #   "version": 1,
//     #   "vin": [
//     #   "locktime": 0,
//     #     {
//     #       "txid": "3b7dc918e5671037effad7848727da3d3bf302b05f5ded9bec89449460473bbb",
//     #       "vout": 16,
//     #       "prevout": {
//     #         "scriptpubkey": "0014f8d9f2203c6f0773983392a487d45c0c818f9573",
//     #         "scriptpubkey_asm": "OP_0 OP_PUSHBYTES_20 f8d9f2203c6f0773983392a487d45c0c818f9573",
//     #         "scriptpubkey_type": "v0_p2wpkh",
//     #         "scriptpubkey_address": "bc1qlrvlygpudurh8xpnj2jg04zupjqcl9tnk5np40",
//     #         "value": 37079526
//     #       },
//     #       "scriptsig": "",
//     #       "scriptsig_asm": "",
//     #       "witness": [
//     #         "30440220780ad409b4d13eb1882aaf2e7a53a206734aa302279d6859e254a7f0a7633556022011fd0cbdf5d4374513ef60f850b7059c6a093ab9e46beb002505b7cba0623cf301",
//     #         "022bf8c45da789f695d59f93983c813ec205203056e19ec5d3fbefa809af67e2ec"
//     #       ],
//     #       "is_coinbase": false,
//     #       "sequence": 4294967295
//     #     }
//     #   ],
//     #   "vout": [
//     #     {
//     #       "scriptpubkey": "76a9146085312a9c500ff9cc35b571b0a1e5efb7fb9f1688ac",
//     #       "scriptpubkey_asm": "OP_DUP OP_HASH160 OP_PUSHBYTES_20 6085312a9c500ff9cc35b571b0a1e5efb7fb9f16 OP_EQUALVERIFY OP_CHECKSIG",
//     #       "scriptpubkey_type": "p2pkh",
//     #       "scriptpubkey_address": "19oMRmCWMYuhnP5W61ABrjjxHc6RphZh11",
//     #       "value": 100000
//     #     },
//     #     {
//     #       "scriptpubkey": "0014ad4cc1cc859c57477bf90d0f944360d90a3998bf",
//     #       "scriptpubkey_asm": "OP_0 OP_PUSHBYTES_20 ad4cc1cc859c57477bf90d0f944360d90a3998bf",
//     #       "scriptpubkey_type": "v0_p2wpkh",
//     #       "scriptpubkey_address": "bc1q44xvrny9n3t5w7lep58egsmqmy9rnx9lt6u0tc",
//     #       "value": 36977942
//     #     }
//     #   ]
//     # }
