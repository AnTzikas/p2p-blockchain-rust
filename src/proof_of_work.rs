use crate::block::Block;
use data_encoding::HEXLOWER;
use num_bigint::{BigInt, Sign};
use std::borrow::Borrow;
use std::ops::ShlAssign;
use log::debug;


/// Represents the Proof of Work (PoW) mechanism for mining new blocks.
///
/// The PoW algorithm requires miners to find a nonce that results in a block hash
/// lower than the target difficulty. This process ensures network security and
/// prevents spam by requiring computational work.
///
/// # Example
/// ```
/// use your_crate::{Block, ProofOfWork};
///
/// let block = Block::new_block("prev_hash".to_string(), &[], 1);
/// let pow = ProofOfWork::new_proof_of_work(block);
/// let (nonce, hash) = pow.run();
/// println!("Block mined with nonce: {}, hash: {}", nonce, hash);
/// ```
pub struct ProofOfWork {
    /// The block being mined.
    block: Block,

    /// The target difficulty for mining (derived from `TARGET_BITS`).
    target: BigInt,
}

/// Number of leading zeros required in the block hash (affects mining difficulty).
///
/// - **Lower values** make mining easier.
/// - **Higher values** make mining harder.
const TARGET_BITS: i32 = 8;

/// The maximum nonce value to try before giving up.
const MAX_NONCE: i64 = i64::MAX;

impl ProofOfWork {
    /// Creates a new `ProofOfWork` instance for the given block.
    ///
    /// # Arguments
    ///
    /// * `block` - The block to apply Proof of Work on.
    ///
    /// # Returns
    ///
    /// A `ProofOfWork` instance with the target difficulty set.
    ///
    /// # Example
    /// ```
    /// let block = Block::new_block("prev_hash".to_string(), &[], 1);
    /// let pow = ProofOfWork::new_proof_of_work(block);
    /// ```
    pub fn new_proof_of_work(block: Block) -> ProofOfWork {
        let mut target = BigInt::from(1);
        target.shl_assign(256 - TARGET_BITS); // Set target difficulty based on TARGET_BITS
        ProofOfWork { block, target }
    }

    /// Prepares the block data for hashing by combining:
    /// - Previous block hash
    /// - Transaction hash
    /// - Timestamp
    /// - Difficulty target
    /// - Nonce
    ///
    /// # Arguments
    ///
    /// * `nonce` - The current nonce to include in the hash.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` representing the concatenated data to be hashed.
    fn prepare_data(&self, nonce: i64) -> Vec<u8> {
        let pre_block_hash = self.block.get_pre_block_hash();
        let transactions_hash = self.block.hash_transactions();
        let timestamp = self.block.get_timestamp();
        let mut data_bytes = vec![];

        data_bytes.extend(pre_block_hash.as_bytes());
        data_bytes.extend(transactions_hash);
        data_bytes.extend(timestamp.to_be_bytes());
        data_bytes.extend(TARGET_BITS.to_be_bytes());
        data_bytes.extend(nonce.to_be_bytes());

        data_bytes
    }

    /// Runs the Proof of Work algorithm to mine the block.
    ///
    /// Iteratively tries different nonce values until it finds a hash
    /// that meets the target difficulty.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - `nonce`: The valid nonce found.
    /// - `hash`: The resulting block hash in hexadecimal format.
    ///
    /// # Example
    /// ```
    /// let block = Block::new_block("prev_hash".to_string(), &[], 1);
    /// let pow = ProofOfWork::new_proof_of_work(block);
    /// let (nonce, hash) = pow.run();
    /// println!("Mined with nonce: {}, hash: {}", nonce, hash);
    /// ```
    pub fn run(&self) -> (i64, String) {
        let mut nonce = 0;
        let mut hash = Vec::new();
        debug!("Start PoW with TARGET_BITS: {TARGET_BITS}");
        while nonce < MAX_NONCE {
            let data = self.prepare_data(nonce);
            hash = crate::util::sha256_digest(data.as_slice());
            let hash_int = BigInt::from_bytes_be(Sign::Plus, hash.as_slice());

            if hash_int.lt(self.target.borrow()) {
                // Successful PoW found
                break;
            } else {
                nonce += 1;
            }
        }

        (nonce, HEXLOWER.encode(hash.as_slice()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::Transaction;
    use crate::block::Block;

    #[test]
    fn test_pow_produces_valid_hash() {
        let tx = Transaction::new("test".to_string());
        let block = Block::new_block("prev".to_string(), &[tx], 1);
        let pow = ProofOfWork::new_proof_of_work(block.clone());
        let (_, hash) = pow.run();
        assert_eq!(block.get_hash(), hash);
    }
}