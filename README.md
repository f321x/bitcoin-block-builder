# bitcoin block builder exercise - f321x
In the following document i will explain my solution to the exercise implemented in this repository.
The exact exercise definition can be found in EXERCISE.md

## Design Approach

The program is structured in three main modules and a directory of test scripts:

1. Parsing
2. Validation
3. Block construction
4. Test scripts

I decided to implement the assignment in the Rust programming language because of its known benefits and usage in many bitcoin open source projects, and also because i wanted to learn the language.

### <u>1. Parsing</u>

The parsing module contains the logic to load, parse and deserialize the transaction data from the JSON files contained in the mempool directory into the defined data structures for later use.

The parsing module expects files with **valid JSON format** and will panic if the loaded directory contains invalid files. Parsing the files consists of loading them in a heap allocated *String* variable and deserializing it by using the *Serde JSON* rust crate.

### <u>2. Validation</u>

The validation logic consists of simple **sanity checks** to sort out obviously invalid transactions in a less ressource consuming way and will perform **signature/script** verification of the remaining transactions afterwards.

If a transaction in the mempool is considered invalid is referenced in another transaction (parent transaction), these child transactions will be considered invalid too.

#### *Sanity checks*

The transaction properties are checked on each transaction passed by the parsing module:

* Input and output values
* Input and output count
* Transaction weight
* Validation of txid hash against filename
* Feerate

While calculating the values for verification they will also be stored in the transaction structure for further use (weight, fee, txid).

#### *Script and signature verification*

After a transaction passes the sanity checks the program will call the according signature verification function depending on the transaction type. My solution is able to verify P2PKH and P2WPKH transactions. Other transaction types like P2SH will be considered invalid and could be implemented later.

##### P2PKH
The P2PKH verification function will assemble the validation script from the transaction data and pass it to a script verification submodule able to interpret bitcoin script.


##### P2WPKH
The P2WPKH verification function assembles the transaction commitment accoding to BIP143 and verifies the commitment HASH256 against the witness as well as the ScriptPubKey-pubkey against the HASH160 of the witness pubkey.


### <u>3. Block construction ("mining")</u>

The block construction module expects any amount of valid transactions and will construct a block consisting of header, coinbase transactions and a sorted constellation of transaction ids.

The block construction module will aim to maximise fee revenue respecting the limited block size of 4 000 000 weight units.

Block construction happens in this order:

1. Assigning parents to transactions
2. Calculating packet weights of transactions with their ancestors
3. Sorting transactions aiming at maximum fee revenue
4. Removing transactions with lowest feerate to respect block size limit
5. Assembly of coinbase transaction
	* including construction of wtxid commitment
6. Assembly of block header
	* including hashing to reach target difficulty (the "mining")

After the block data is determined it will be passed to a function storing it in a output.txt file formatted according to the subject requirements.

### <u>4. Test scripts</u>

In the process of writing the program i also used two python scripts to verify some results of the implementation.

#### test_tx_assembly.py
Contains some loose functions to construct a standard p2wpkh transaction commitment.

#### validate_wtxids.py
Script to verify the wtxid construction of my program. Takes a file containing my constructed txids and wtxids and compares them with the correct wtxids pulled from a self hosted mempool.space API. If a wrong wtxid is encountered i can manually debug to find the differences.

## Implementation details
This section will go trough the program in the same order as the previous one (order of execution) and explain the implementation in more detail assuming understanding of the previous chapter.

### Global
The *Transaction* struct is used to store the transaction data parsed out of the json files by the parsing module.

```
struct Transaction
    meta: 		MetadataStruct,
    version: 	4 byte integer,
    locktime: 	4 byte unsigned integer,
    vins: 		List<TxIn struct>,
    vouts: 		List<TxOut struct>,
```
The *Transaction* struct and the contained sub-structs are defined in **parsing/transaction_structs.rs**.
Most of the runtime the *Transaction* structs are stored and passed between functions in a Vec<_Transaction_>.

Variables contained only in some JSON files are Option<_Some_> variables and some variables are complemented later once available.

The _MetadataStruct_ contained in the Transaction struct contains the following useful transaction metadata:
```
struct MetadataStruct
    json_path: 		Option<absolute path String>,
    txid_hex: 		String,
    wtxid_hex: 		String,
    packet_data: 	Packet,
    weight: 		u64,
    fee: 			u64,
    parents: 		Option<Vec<hex txids>>,
```

### <u>1. Parsing</u>
```
├── parse_transactions_from_dir(directory_path: &str)
│   │
│   └── fs::read_dir(directory_path)
│      └── Iterate over files in the directory
│          │
│          └── parse_file_content(file_to_load: fs::DirEntry)
│              │
│              ├── Check file extension
│              │   └── If not "json", continue to next file
│              │
│              ├── fs::read_to_string(file_path_buf)
│              │   └── Read file content into a String
│              │
│              └── parse_json(&file_content [ref to Sting]) -> using serde json crate
│                  │
│                  ├── from_str::<Transaction>(str_content)
│                  │   └── Deserialize JSON into Transaction struct
│                  │
│                  ├── If deserialization successful
│                  │   ├── Update tx.meta.json_path to absolute json path
│                  │   └── Set input types for each tx.vin
│                  │
│                  └── If deserialization fails
│                      └── Panic with error message -> Invalid JSON file
│
└── Return Vec<Transaction> (parsed transactions)
```
The Vec<_Transaction_> returned by the parsing module is now passed on to the validation module to verify the transactions and sort out invalid ones to be able to construct a valid block.

### <u>2.1 Transaction validation - Sanity checks</u>

#### ***Input and output values and count***

```fn validate_values_and_set_fee(tx: &mut Transaction) -> bool```

Validates that the transaction has higher sum of input values than output values (no "inflation"), also checks if the transaction even has inputs and outputs and that the values are possible (below 21m bitcoin).

If the all checks pass the fee will be stored in the passed mutable _Transaction_ reference.

#### ***Parsing validation***

```fn validate_txid_hash_filename(tx: &mut Transaction) -> bool ```

Compares the SHA256 hash of the TXID against the filename of the JSON file to verify correct parsing of the data.

To get the TXID the transaction has to be byte serialized without witness parts (no marker, flag and witnesses) in the specified structure below:

1. Version [4 bytes LE]
2. (WTXID: 1 byte marker & 1 byte flag)
3. Input count [varint LE]
4. All serialized inputs each consisting of:
    1. Outpoint txid in natural bytes (txid referenced in input)
    2. Outpoint index [4 bytes LE]
    3. Scriptsig length [varint LE bytes]
    4. Scriptsig bytes
    5. Sequence bytes [4 bytes LE]
5. Output count [varint LE]
6. All serialized outputs each consisting of:
    1. Value in satoshi [8 bytes LE]
    2. Length of output scriptpubkey [varint LE bytes]
    3. Output scriptpubkey bytes
7. (WTXID: serialized witnesses)
8. Locktime [4 bytes LE]

Afterwards this bytes stored in a Vec<_u8_> can be double SHA256 hashed and compared with the filenames, if they are unequal there would be some problem in the transaction parsing or serialization.

The functions in validate_parsing.rs will also calculate the WTXID due to the similar logic and store it alongside the TXID and store it in the mutable _Transaction_ reference for later use.

#### ***Transaction weight***

``` fn validate_and_set_weight(tx: &mut Transaction) -> bool ```

Weight units define the "size" of the transaction used later for calculation of the feerate and priorization of the transaction by the miner. Weight units discount some parts of the transaction so it can't be compared to bytes.

This are the multipliers used when calculating the transaction weight from its byte size:

| Field   | Multiplier |
|---------|------------|
| version | x4         |
| marker  | x1         |
| flag    | x1         |
| input   | x4         |
| output  | x4         |
| witness | x1         |
| locktime| x4         |

Calculating the weight is done by decoding all parts stored in the _Transaction_ into bytes and calculating the sum of all parts each multiplied by its weight multiplier. If it is a segwit transaction marker, flag and witness are included in the calculation too as they are stored on the blockchain as well.

As part of the sanity check the function *validate_and_set_weight(tx: &mut Transaction)* will check if the weight of the transaction is above 4 000 000 WU (- 320 WU for the block header & - 400 WU reserve for the coinbase transaction) which would be too large to be included in any block.

Afterwards the transaction weight will be stored in the mutable _Transaction_ reference for later use.

#### ***Transaction feerate***
``` fn validate_feerate(tx: &Transaction) -> bool ```

The last simple sanity check is now able to calculate the feerate from the previously calculated weight and transaction fee. The **Bitcoin Core** implementation of bitcoin will only relay transactions with a feerate above 1 satoshi per vbyte. A virtual byte is another unit of size and can be calculated by dividing the weight by 4.

```
vbyte_size = tx.meta.weight / 4
feerate = tx.meta.fee / vbyte_size
    if feerate < 1
        return false
return true
```

Although transactions with a feerate below 1 sat/vbyte are not strictly invalid they would have to be mined out of band by a miner as they won't be stored in the mempool so i will consider them invalid in the program.

### <u>2.2 Transaction validation - Signature and Script verification</u>
Out of the available transaction types in the given mempool i decided to implement verification for P2PKH and P2WPKH and consider other transaction types invalid.

To verify contained scripts and to learn the function of *Bitcoin Script*, the "language" used to specify and satisfy the spending conditions of transaction outputs i implemented a *Script* verification "engine" located in validation/script.rs.

The function *evaluate_script()* goes trough the script byte by byte and calls the according function if an opcode is encountered. The stack is implemented as a VecDeque<_Vec<*u8*>_> data structure.
```
fn evaluate_script(
    script: Vec<u8>,
    txin: &TxIn,
    tx: &Transaction, ) -> Result<(), Box<dyn Error>>
```
This are the opcodes supported by the function:

| Hex | OP_NAME | Function Call |
|-----|---------|---------------|
| 0xa8 | OP_SHA256 | `stack.push_back(hash_sha256(&last))` |
| 0xa9 | OP_HASH160 | `stack.push_back(hash160(&last))` |
| 0x75 | OP_DROP | `stack.pop_back()` |
| 0x7c | OP_SWAP | `op_swap(&mut stack)?` |
| 0x00 | OP_0 | `stack.push_back(Vec::new())` |
| 0x76 | OP_DUP | `stack.push_back(last.clone())` |
| 0x87 | OP_EQUAL | `op_equal(&mut stack)?` |
| 0x7b | OP_ROT | `op_rot(&mut stack)?` |
| 0x82 | OP_SIZE | `op_size(&mut stack)?` |
| 0x78 | OP_OVER | `op_over(&mut stack)?` |
| 0xa0 | OP_GREATERTHAN | `op_greaterthan(&mut stack)?` |
| 0x88 | OP_EQUALVERIFY | `op_equalverify(&mut stack)?` |
| 0x73 | OP_IFDUP | `op_ifdup(&mut stack)?` |
| 0xb2 | OP_CHECKSEQUENCEVERIFY | `op_checksequenceverify(&mut stack, txin, tx)?` |
| 0xb1 | OP_CHECKLOCKTIMEVERIFY | `op_checklocktimeverify(&mut stack, tx, txin)?` |
| 0xac | OP_CHECKSIG | `op_checksig(&mut stack, tx, txin)?` |
| 0x74 | OP_DEPTH | `op_depth(&mut stack)?` |
| 0xad | OP_CHECKSIGVERIFY | `op_checksig(&mut stack, tx, txin)?; op_verify(&mut stack)?` |
| 0x51..=0x60 | OP_PUSHNUM (1-16) | `op_pushnum(&mut stack, opcode)?` |
| 0x4f | OP_1NEGATE | `stack.push_back(vec![255])` |
| 0x01..=0x4b | OP_PUSHBYTES | `op_pushbytes(&mut stack, &mut index, &script)?` |
| 0x4c | OP_PUSHDATA1 | `op_pushdata(&mut stack, 1, &mut index, &script)?` |
| 0x4d | OP_PUSHDATA2 | `op_pushdata(&mut stack, 2, &mut index, &script)?` |
| 0x4e | OP_PUSHDATA4 | `op_pushdata(&mut stack, 4, &mut index, &script)?` |
| 0xae | OP_CHECKMULTISIG | `op_checkmultisig(&mut stack, tx, txin)?` |


#### P2PKH
```
fn verify_p2pkh(tx: &Transaction, txin: &TxIn) -> ValidationResult
```
When the main verifying loop detects the transaction input type as P2PKH the function above will assemble a script from the transaction input data with a structure similar to this (but in bytes):
```
scriptSig part
------------
OP_PUSHBYTES
SIGNATURE
OP_PUSHBYTES
PUBLIC_KEY
------------
+
ScriptPubKey part
-------------------------
OP_DUP
OP_HASH160
OP_PUSHBYTES_20
PUBLIC KEY HASH (HASH160)
OP_EQUALVERIFY
OP_CHECKSIG
------------------------
```

The script will then be passed to the script verification function which will return the result.

If any transaction input is invalid the transaction will be considered invalid.

#### P2WPKH
My P2WPKH verification is more hardcoded as i implemented the Script engine afterwards and could be refactored to use the script engine as further improvement.

I first assemble the commitment to generate HASH256(commitment) message for signatue verification according to the BIP143 serialization specification:

1. Version [4-byte little endian]
2. hashPrevouts [*HASH256(tx.serialize_all_outpoints())*]
3. hashSequence [*HASH256(tx.serialize_all_sequences())*]
4. outpoint [32-byte outpoint txid natural byte order + 4-byte little endian index]
5. scriptCode of the input (byte serialized scriptcode)
6. value of the output spent by this input (8-byte little endian)
7. Sequence of the input (4-byte little endian)
8. hashOutputs [*HASH256(tx.serialize_all_outputs())*]
9. Locktime of the transaction (4-byte little endian)
10. sighash type of the signature [4-byte little endian, *SIGHASH_ALL hardcoded*]

Then the program compares if HASH160(witness public key) is equal to the public key encoded in the ScriptPubKey. If so the commitment hash is verified against the signature and public key using ecdsa on secp256k1 (imported as rust crate).


#### All transaction considered invalid according to the previous tests will be stored be stored in a HashSet in form of their hex txid. Afterwards all Transactions contained in the HashSet will be removed from the Vec<*Transaction*> of parsed transactions and the remaining, valid *Transaction* structs will be stored in a HashMap<TXID hex String, *Transaction*> for block construction.

### <u>3. Block construction</u>


#### Assigning parents to transactions

```assign_mempool_parents(&mut txid_tx_map)```

This function will traverse trough each input in each transaction in txid_tx_map and create, for each transaction, a Vec<*String*> of hex txids of the referenced outpoints contained in the mempool (the parents). This Vec<*hex txid String*> is then stored in the value of txid in txid_tx_map (*Transaction.meta.parents*).

#### Calculating packet weights of transactions with their ancestors
``` calculate_packet_weights(&mut txid_tx_map)```

The function calculates the package weight for each transaction by recursively going to the bottom of the ancestor dependence and summing up fees and weight to each calling txid with using the following logic:

```
fn calc_parents(all_transactions, child_txid_hex_string) -> FeeAndWeight:

    new struct fee_and_weight = {
        fee: child_transaction.meta.fee,
        weight: child_transaction.meta.weight,
    }

    if child transaction has parents in mempool:
        for parent in parents_txids
            let temp_result = calc_parents(transactions, parent);
            fee_and_weight.fee += temp_result.fee;
            fee_and_weight.weight += temp_result.weight;

    return fee_and_weight
```

After the packet weight and fees have been calculated the packet feerate is calculated out of them:
```
tx.meta.packet_data.packet_feerate_weight = previous.fee / previous.weight;
```

Now that we have the packet feerate for each transaction we are able to sort them by their profitability more accurate.

#### Sorting transactions
```fn sort_transactions(txid_tx_map) -> Vec<Transaction>```

The program now first sorts all transactions by their packet feerate, then will push the parents of the transactions in front of their children. This way it can be ensured that the block never contains a child before its parents.

Sorting happens with standard rust functions:
```
transactions.sort_by(|a, b: &&Transaction| {
        b.meta
            .packet_data
            .packet_feerate_weight
            .cmp(&a.meta.packet_data.packet_feerate_weight)
    })
```

Pushing the parents in front of the children is implemented as a loop that will terminate as soon as no parent has been moved anymore after a full iteration trough the Vec<*Transaction*>.

#### Removing transactions with lowest feerate to respect block size limit
```
fn cut_size(sorted_transactions: Vec<Transaction>) -> Vec<Transaction>
```
After we have created a Vec<*Transaction*> sorted from high to low revenue transactions it is neccessary to remove enough transactions to respect the block size limit of 4 000 000 weight units (minus header and coinbase tx reserve).

This is implemented by pushing the Transactions from *sorted_transactions* to a new Vec "block" and simultaneously adding their tx.meta.weight to a sum until the hardcoded limit of 3 992 000 is reached. Afterwards the new Vec<*Transaction*> is returned safe to be fully included in a block.

#### Assembly of coinbase transaction
```
fn assemble_coinbase_transaction(block_txs: &Vec<Transaction>) -> CoinbaseTxData
```

The coinbase transaction is the first transaction in a block, constructed by the miner to reward himself with the current block subsidy and transaction fees of the included transactions. The coinbase transaction also contains a commitment to all witnesses in the transaction by including a modified merkle root hash of all included wTXIDs as OP_RETURN output in the coinbase transaction.

The coinbase transaction will be assembled two times, once with witness values (witness reserved value, marker and flag) for inclusion of the raw transaction in the second row of the output.txt. The coinbase transaction without witness values will be used to generate its TXID for use in the blockheader merkle tree.

The coinbase transaction has the following byte structure:

1. Version [4 bytes, LE]
2. (if wTXID: marker + flag, each 1 byte)
3. Input count [1 input, 1 byte, LE]
4. Input [always empty TXID, 32 zero bytes]
5. Input amount [always maximum value: 0xffffffff]
6. Scriptsig byte length [varint, LE]
7. Scriptsig -> has to contain current block height in LE, random data can be added afterwards, for example an extra nonce or a miner tag
8. Sequence [0xffffffff]
9. Output count [1 byte LE, reward & wtxid commitment]
10. Reward amount, all feed + block reward in satoshi [8 bytes, LE]
11. Locking script size [varint, LE]
12. Reward payout locking script, i used a P2WPKH scriptPubKey
13. wTXID OP_RETURN commitment amount [8 bytes of 0]
14. wTXID commitment script lenght [1 byte, 0x26]
15. the wTXID commitment consisting of the following structure:
```OP_RETURN OP_PUSHBYTES_36 commitment_header_(aa21a9ed)  wTXID_COMMITMENT```
16. Stack item count [1 item, 1 byte, the witness reserved value]
17. Size of the witness reserved value [0x20]
18. Witness reserved value [32 null bytes]
19. Locktime [4 null bytes]

The wTXID merkle root is calculated as a HASH256 merkle root of all wTXIDs (the txids of the transactions including the witness part). The coinbase transaction is included as empty txid (32 null bytes) to prevent circular reference.
The wTXID commitment used in the OP_RETURN output is the HASH256 of the wtxid merkle root concatenated with the witness reserved value (32 null bytes).

Now we return the following result to the main block construction function:
```
struct CoinbaseTxData
    txid_hex:           String -> first txid in output.txt (3rd line)
    txid_natural_bytes: bytes -> for block header merkle root
    full_assembled_tx:  bytes -> second line in output.txt
```

#### Assembly of the block header
```
fn construct_header(sorted_block_transactions: &Vec<Transaction>,
                    coinbase_tx: &CoinbaseTxData)
                    -> Vec<u8>
```

The block header is the first data contained in the block. It links the block to the previous block by referencing the hash of the previous blocks header. The block commits to all contained transactions by including a merkle root of all TXIDs and is in itself the proof of the work (energy consumption) utilized to construct it.

A regular block header is byte serialized in the following structure:
1. **Version**, can also be used as bitfield to signal readyness for softforks [usually **0x20000000**]
2. **Previous block** hash [32 byte, natural order]
3. HASH256 **merkle root** of all contained transactions (txids), including the coinbase tx [32 byte, natural order]
4. Current **unix time** [4 bytes, LE]
5. **Target bits**, more compact representation of the Proof of Work target required for this block (current difficulty epoch). [4 bytes]
6. **Nonce** [4 bytes, LE]

The process of finding a block (header) hash below the required difficulty target happens in the following way:
```
loop {
    current_hash = HASH256(first header bytes + current nonce bytes)
    if BigUint(current_hash) < BigUint(Target difficulty)
        break and return Nonce
    else
        nonce + 1
}
```
The nonce is being increased by one until the HASH256 of the block header with the tested nonce is below the target. The target for the assignment was defined as: ```0x0000ffff00000000000000000000000000000000000000000000000000000000```

The smaller the target the longer it takes to find a valid hash, making it more difficult to find a valid block. The bitcoin network automatically adjusts the difficulty to keep a block interval around 10 minutes.

The block construction part now returns the block data in form of a struct back to the main function for exporting:
```
struct Block
    header_hex:      String
    coinbase_tx_hex: String
    txids_hex:       Vec<String>
```

### <u>Output</u>
```
output_block(mined_block: &Block, output_path: &str)
```

Now that the program has a sorted list of valid transaction ids, a block header and a coinbase transaction commiting to the transaction list it can construct the output.txt required by the subject.

This is done using the rust std functions ```File::create(output_path)``` and
```writeln!(output_file, "{}", mined_block.data)```.

The complete output.txt file will be created in the root of the repository :)

## Results and Performance

### Results

Of the given **8131** transactions the program is able to construct a block including around **3200** valid transactions containing **fees of ~20 260 000 satoshi** and a **weight of ~3 950 000 WU**. The actual values are subject to smaller variance due to the non-deterministic nature of data structures i used.

This is around **99% block space utilization** and **20m satoshi** additional revenue on top of the block reward.

### Performance metrics

The measurements were taken on the following hardware:
```
iMac 20.1

Processor Name:	            6-Core Intel Core i5
Processor Speed:	        3.1 GHz
Number of Processors:	    1
Total Number of Cores:	    6
L2 Cache (per Core):	    256 KB
L3 Cache:	                12 MB
Hyper-Threading Technology:	Enabled
Memory:	                    8 GB
```

The program has been compiled in release mode using the following settings:
```
[profile.release]
lto = true
strip = true
```

Due to the system running in some kind of virtualization system the actual speed is way lower than the specs seem to let one expect.

All measurements were taken on the given mempool transaction set.

#### Runtime
The full program runtime from loading transaction files to exporting the output.txt is ~ 41000 ms (41 seconds).

#### Loading transaction files
Running the program the first time parsing the json files into the Vec<*Transaction*> takes **~27000 ms** (27 sec). Running the program again it will only take **~200ms** due to caching done by the operating system.

#### Validating transactions

Transaction validation and creation of the new HashMap containing only valid transactions takes **~4000 ms** (4 sec).

#### Full block assembly
Assembling all data for the block (coinbase, header and sorting the transactions) takes **~10000 ms** (10 sec).

#### Hashing the header (nonce search)
Searching for a txid below the target difficulty of
```0x00000ffff0000000000000000000000000000000000000000000000000000000```
takes **~1250 ms** on average.

## Conclusion

### <u>Insights</u>
While working on the assignment i could gain many insights. Bitcoin is a very sophisticated project with many details to each supposedly small part. Most functionality has been thought about on how to optimize it down to the single bit. This makes bitcoin a very interesting project to work on as its possible to learn a ton of sophisticated concepts. But it also requires a lot of patience and view for the detail. One value in big endian? Completely different hashes :)

It was also my first bigger project written in Rust and seeing the large amounts of serialization, deserialization, hashing and calculating i understand why many bitcoin projects can profit of the safety and performance of the language.

### <u>Further improvements</u>
Due to the limited time and me learning many new concepts in the process of writing the program there are many possible improvements:

#### Test coverage
A possible improvement to make the program more safe and defined would be to implement tests for each relevant function by utilizing Rusts good testing functionality.

#### Using more rustacean syntax
Due to me coming from C and Python the coding style is probably looking a bit functional and could utilize more of Rusts features like Traits, Generics and more suitable data structures.

#### Performace improvements
To make the program more performant it could be optimized to make more use of references instead of cloning data. It could also be benchmarked with a profiler to see functions causing performance bottlenecks to be improved.

#### Implement more input types and bitcoin functionality
To be able to process more different transaction types for higher fee revenue and better block space utilization it would be neccessary to implement more input types like P2TR, P2WSH and P2SH. To do this it would be neccessary to implement some more opcodes like OP_IF in the script engine. It would also be possible to implement more sighash types besides SIGHASH_ALL to be able to verify these transactions too.

#### Add sigops counting
No transaction input seemed to contain excessive amounts of signature operations but to make the program more reliable in respecting the block creation rules a function to count the sigops in the candidate block to limit them below 80000 operations should be implemented.

#### Make the program output deterministic
Currently there is a small variance in block creation even tough the input data provided is constant. To make this deterministic would make the program more predictable and allow for more accurate benchmarks. To do this it would be neccessary to change some data types from hash based ordering to Vectors and logic handling the transactions.

#### Combine functions
Even tough i value simple code and avoid duplicate code there is some code that could be combined to make it easier to read and simpler. The P2WPKH script verification could for example be refactored to use the script verification module which would just need a new function to assemble a TX commitment according to BIP143 in order to verify P2WPKH inputs.

#### Making it open source
Open sourcing the program could attract contributors to improve the code with their knowledge and enable them to utilize the code in their software providing more value to more users.

### <u>Resources</u>

I used a lot of ressources to understand the concepts and how to implement them. These are the ressources that helped me the most:
* [learn me a bitcoin](https://learnmeabitcoin.com)
* [Bitcoin Stackexchange](https://bitcoin.stackexchange.com/)
* [BIP 143](https://github.com/bitcoin/bips/blob/master/bip-0143.mediawiki)
* [Bitcoin Wiki](https://en.bitcoin.it/wiki)
* [Mastering Bitcoin Rev. 3](https://github.com/bitcoinbook/bitcoinbook)
* [Grokking Bitcoin](https://www.manning.com/books/grokking-bitcoin)
* [BIP 141](https://github.com/bitcoin/bips/blob/master/bip-0141.mediawiki)
