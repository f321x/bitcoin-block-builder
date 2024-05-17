# bitcoin block builder exercise

## Overview
In this challenge, you are tasked with the simulation of mining process of a block, which includes validating and including transactions from a given set of transactions.
The repository contains a folder `mempool` which contains JSON files.
These files represent individual transactions, some of which may be invalid. Your goal is to successfully mine a block by including only the valid transactions, following the specific requirements outlined below.

## Objective
Your primary objective is to write a script that processes a series of transactions, validates them, and then mines them into a block. The output of your script should be a file named `output.txt` that follows a specific format.

## Requirements
### Input
- You are provided with a folder named `mempool` containing several JSON files. Each file represents a transaction that includes all necessary information for validation.
- Among these transactions, some are invalid. Your script should be able to discern valid transactions from invalid ones.

### Output
Your script must generate an output file named `output.txt` with the following structure:
- First line: The block header.
- Second line: The serialized coinbase transaction.
- Following lines: The transaction IDs (txids) of the transactions mined in the block, in order. The first txid should be that of the coinbase transaction

### Difficulty Target
The difficulty target is `0000ffff00000000000000000000000000000000000000000000000000000000`. This is the value that the block hash must be less than for the block to be successfully mined.

## Execution
- Create a file named `run.sh` that contains the command to execute your script. This file should ideally contain a single command like `python main.py` or `node index.js`.
- Your script should autonomously perform all tasks when `run.sh` is executed, without requiring any manual intervention.

## Document your work

Apart from the code, you must also publish a `SOLUTION.md` file explaining your solution in the following format:
- **Design Approach:** Describe the approach you took to design your block construction program, explain all the key concepts of creating a valid block.
- **Implementation Details:** Provide pseudo code of your implementation, including sequence of logic, algorithms and variables used etc.
- **Results and Performance:** Present the results of your solution, and analyze the efficiency of your solution.
- **Conclusion:** Discuss any insights gained from solving the problem, and outline potential areas for future improvement or research. Include a list of references or resources consulted during the problem-solving process.

## What NOT to Do

In this challenge, it's crucial to understand and adhere to the following restrictions. These are put in place to ensure that you engage with the core concepts of bitcoin and apply your problem-solving skills to implement the solution from first principles.

- **Do Not Use Bitcoin Libraries for Transaction Validation:** You must not use any Bitcoin-specific libraries or frameworks that automate transaction validation processes. The intent of this challenge is for you to understand and implement the validation logic manually.
- **Permissible Libraries:** The use of standard cryptographic libraries, such as secp256k1 for elliptic curve cryptography, and standard hashing libraries (e.g., for SHA-256) is allowed and encouraged. These libraries are essential for implementing the cryptographic underpinnings of bitcoin without reinventing the wheel.
 - **Implement the Mining Algorithm Yourself:** You are required to implement the mining algorithm on your own. This includes creating a way to correctly form a block header, calculate the hash, and meet the challenge of finding a hash below a certain target.
