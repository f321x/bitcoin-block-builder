import hashlib
from typing import List

# utilizing some old code i wrote in the chaincode labs seminar to verify my transaction serialization

def input_from_utxo(txid: bytes, index: int) -> bytes:
    reversed_txid = txid[::-1]
    index = index.to_bytes(4, "little")
    return reversed_txid + index

def get_p2wpkh_scriptcode(asm) -> bytes:
    tokens = asm.split(" ")
    pubkey_hash = bytes.fromhex(tokens[-1])
    scriptcode = bytes.fromhex("1976a914") + pubkey_hash + bytes.fromhex("88ac")
    return scriptcode

def get_commitment_hash(outpoint: bytes, scriptcode: bytes, value: int, outputs: List[bytes]) -> bytes:
    def dsha256(data: bytes) -> bytes:
        return hashlib.new("sha256", hashlib.new("sha256", data).digest()).digest()
    result = b""
    result += (1).to_bytes(4, "little")
    # print("Version " + (1).to_bytes(4, "little").hex())

    result += dsha256(outpoint)  # hashPrevouts
    # print("dsha outpoint: " + dsha256(outpoint).hex())
    result += dsha256(bytes.fromhex("ffffffff"))
    # print("Sequences: " + dsha256(bytes.fromhex("ffffffff")).hex())
    result += outpoint
    # print("Outpoint: " + outpoint.hex())

    result += scriptcode
    # print("Scriptcode: " + scriptcode.hex())

    result += value.to_bytes(8, "little")
    # print("prevout value: " + value.to_bytes(8, "little").hex())

    result += bytes.fromhex("ffffffff")
    # print("sequence: " + )

    result += dsha256(b"".join(outputs))
    print("output0: " + outputs[0].hex())
    print("output1: " + outputs[1].hex())
    print("all outputs: " + b"".join(outputs).hex())
    # print("all outputs: " + dsha256(b"".join(outputs)).hex())

    result += bytes.fromhex("00000000")
    result += bytes.fromhex("01000000")
    return dsha256(result)

# Given an output script and value (in satoshis), return a serialized transaction output
def output_from_options(script: bytes, value: int) -> bytes:
    value = value.to_bytes(8, "little")
    script_length = len(script).to_bytes(1, "little")
    print("Script length: " + script_length.hex())
    return value + script_length + script

def spend_p2wpkh():
    outpoint = input_from_utxo(bytes.fromhex("3b7dc918e5671037effad7848727da3d3bf302b05f5ded9bec89449460473bbb"), 16)
    scriptcode = get_p2wpkh_scriptcode("OP_0 OP_PUSHBYTES_20 f8d9f2203c6f0773983392a487d45c0c818f9573")

    # serialized_input = input_from_utxo(bytes.fromhex("d1283ec7f6a2bcb65a5905033168258ca282e806c9dc7164415519a5ef041b14"),
                                        #   0,
                                        #   bytes.fromhex("4730440220200b9a61529151f9f264a04e9aa17bb6e1d53fb345747c44885b1e185a82c17502200e41059f8ab4d3b3709dcb91b050c344b06c5086f05598d62bc06a8b746db4290121025f0ba0cdc8aa97ec1fffd01fac34d3a7f700baf07658048263a2c925825e8d33"),
                                        #   4294967295)

    output1 = output_from_options(bytes.fromhex("76a9146085312a9c500ff9cc35b571b0a1e5efb7fb9f1688ac"), 100000)
    output2 = output_from_options(bytes.fromhex("0014ad4cc1cc859c57477bf90d0f944360d90a3998bf"), 36977942)

    message = get_commitment_hash(outpoint, scriptcode, 37079526, [output1, output2])
    print("Serialized commitment: " + message.hex())


print(spend_p2wpkh())

# {
#   "version": 1,
#   "locktime": 0,
#   "vin": [
#     {
#       "txid": "3b7dc918e5671037effad7848727da3d3bf302b05f5ded9bec89449460473bbb",
#       "vout": 16,
#       "prevout": {
#         "scriptpubkey": "0014f8d9f2203c6f0773983392a487d45c0c818f9573",
#         "scriptpubkey_asm": "OP_0 OP_PUSHBYTES_20 f8d9f2203c6f0773983392a487d45c0c818f9573",
#         "scriptpubkey_type": "v0_p2wpkh",
#         "scriptpubkey_address": "bc1qlrvlygpudurh8xpnj2jg04zupjqcl9tnk5np40",
#         "value": 37079526
#       },
#       "scriptsig": "",
#       "scriptsig_asm": "",
#       "witness": [
#         "30440220780ad409b4d13eb1882aaf2e7a53a206734aa302279d6859e254a7f0a7633556022011fd0cbdf5d4374513ef60f850b7059c6a093ab9e46beb002505b7cba0623cf301",
#         "022bf8c45da789f695d59f93983c813ec205203056e19ec5d3fbefa809af67e2ec"
#       ],
#       "is_coinbase": false,
#       "sequence": 4294967295
#     }
#   ],
#   "vout": [
#     {
#       "scriptpubkey": "76a9146085312a9c500ff9cc35b571b0a1e5efb7fb9f1688ac",
#       "scriptpubkey_asm": "OP_DUP OP_HASH160 OP_PUSHBYTES_20 6085312a9c500ff9cc35b571b0a1e5efb7fb9f16 OP_EQUALVERIFY OP_CHECKSIG",
#       "scriptpubkey_type": "p2pkh",
#       "scriptpubkey_address": "19oMRmCWMYuhnP5W61ABrjjxHc6RphZh11",
#       "value": 100000
#     },
#     {
#       "scriptpubkey": "0014ad4cc1cc859c57477bf90d0f944360d90a3998bf",
#       "scriptpubkey_asm": "OP_0 OP_PUSHBYTES_20 ad4cc1cc859c57477bf90d0f944360d90a3998bf",
#       "scriptpubkey_type": "v0_p2wpkh",
#       "scriptpubkey_address": "bc1q44xvrny9n3t5w7lep58egsmqmy9rnx9lt6u0tc",
#       "value": 36977942
#     }
#   ]
# }
