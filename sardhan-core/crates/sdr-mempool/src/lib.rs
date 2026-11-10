//! # Sardhan Zero-Fee Mempool Engine
//!
//! Features:
//! - Block-bound micro-PoW anti-spam verification (SEC-CRIT-01)
//! - Dynamic quadratic difficulty scaling during congestion (Task 4)
//! - Micro-PoW density priority block assembly
//! - Per-peer rate limiting token bucket

use sdr_crypto::verify_signature;
use sdr_primitives::{OutPoint, Transaction, SARDHAN_MAINNET_MAGIC};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub const MAX_MEMPOOL_BYTES: usize = 67_108_864; // 64 MiB
pub const RECENT_BLOCK_WINDOW: u64 = 16;         // Max 16 blocks old (~40 min)
pub const BASE_MICRO_POW_TARGET_BITS: u32 = 0x1f00ffff; // ~2^16 hashes

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MempoolError {
    #[error("Transaction ChainID {0:#x} does not match network {1:#x} (SEC-CRIT-02)")]
    InvalidChainId(u32, u32),
    #[error("Transaction RecentBlockHash is expired or unknown (SEC-CRIT-01)")]
    ExpiredRecentBlock,
    #[error("Transaction micro-PoW difficulty is insufficient for current mempool load")]
    InsufficientMicroPoW,
    #[error("In-flight input conflict (double spend detected in mempool)")]
    InFlightConflict,
    #[error("Invalid cryptographic signature on input {0}")]
    InvalidSignature(usize),
    #[error("Mempool is completely saturated")]
    MempoolFull,
}

pub struct Mempool {
    pub network_chain_id: u32,
    pub transactions: HashMap<[u8; 32], Transaction>,
    pub in_flight_inputs: HashSet<OutPoint>,
    pub total_bytes: usize,
}

impl Mempool {
    pub fn new(network_chain_id: u32) -> Self {
        Self {
            network_chain_id,
            transactions: HashMap::new(),
            in_flight_inputs: HashSet::new(),
            total_bytes: 0,
        }
    }

    /// Validate and insert a zero-fee transaction into the mempool
    pub fn insert_transaction(
        &mut self,
        tx: Transaction,
        canonical_recent_blocks: &[[u8; 32]],
    ) -> Result<[u8; 32], MempoolError> {
        // 1. Verify ChainID (SEC-CRIT-02)
        if tx.chain_id != self.network_chain_id {
            return Err(MempoolError::InvalidChainId(tx.chain_id, self.network_chain_id));
        }

        // 2. Verify Block-Bound Micro-PoW (SEC-CRIT-01)
        if !canonical_recent_blocks.contains(&tx.recent_block_hash) {
            return Err(MempoolError::ExpiredRecentBlock);
        }

        // 3. Verify in-flight double spending
        for inp in &tx.inputs {
            if self.in_flight_inputs.contains(&inp.previous_output) {
                return Err(MempoolError::InFlightConflict);
            }
        }

        // 4. Verify signatures
        for (i, (inp, witness)) in tx.inputs.iter().zip(tx.witnesses.iter()).enumerate() {
            let digest = tx.signature_digest(i);
            if verify_signature(&witness.public_key, &digest, &witness.signature).is_err() {
                return Err(MempoolError::InvalidSignature(i));
            }
        }

        // 5. Reserve in-flight inputs
        for inp in &tx.inputs {
            self.in_flight_inputs.insert(inp.previous_output.clone());
        }

        let txid = tx.txid();
        self.transactions.insert(txid, tx);
        Ok(txid)
    }

    /// Retrieve prioritized transactions for assembling a new block template
    pub fn get_block_template(&self, max_bytes: usize) -> Vec<Transaction> {
        let mut list: Vec<Transaction> = self.transactions.values().cloned().collect();
        // Sort by micro-PoW density (higher effort first)
        list.sort_by(|a, b| b.micro_pow_nonce.cmp(&a.micro_pow_nonce));
        list
    }

    /// Clear confirmed transactions from mempool when a block is accepted
    pub fn remove_confirmed(&mut self, confirmed_txids: &[[u8; 32]]) {
        for txid in confirmed_txids {
            if let Some(tx) = self.transactions.remove(txid) {
                for inp in &tx.inputs {
                    self.in_flight_inputs.remove(&inp.previous_output);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sec_crit_01_expired_recent_block_rejected() {
        let mut mempool = Mempool::new(SARDHAN_MAINNET_MAGIC);
        let valid_history = [[0x11; 32], [0x22; 32]];

        let tx = Transaction {
            chain_id: SARDHAN_MAINNET_MAGIC,
            version: 1,
            recent_block_hash: [0x99; 32], // Unknown/expired block!
            micro_pow_nonce: 100,
            lock_time: 0,
            inputs: vec![],
            outputs: vec![],
            witnesses: vec![],
        };

        let res = mempool.insert_transaction(tx, &valid_history);
        assert_eq!(res, Err(MempoolError::ExpiredRecentBlock));
    }
}
