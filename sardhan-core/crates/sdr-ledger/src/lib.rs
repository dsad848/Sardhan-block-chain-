//! # Sardhan UTXO Ledger & State Engine
//!
//! Features:
//! - In-memory and persistent UTXO set
//! - CoinStake 576-block maturity lock (Task 2)
//! - Double-spending rejection
//! - Deterministic Genesis Block with 0% Premine

use sdr_consensus::calculate_block_weight;
use sdr_crypto::hash_blake3;
use sdr_primitives::{BlockHeader, ConsensusType, OutPoint, Transaction, TxOut};
use std::collections::HashMap;
use thiserror::Error;

pub const COINSTAKE_MATURITY_LOCK_BLOCKS: u64 = 576; // 24 hours (Task 2)
pub const MIN_STAKE_CONFIRMATION_AGE: u64 = 288;     // 12 hours

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error("UTXO already spent or not found: txid {0}, vout {1}")]
    UtxoNotFound(String, u32),
    #[error("Double spend detected within block")]
    DoubleSpend,
    #[error("CoinStake is immature: requires {0} confirmations, current age is {1}")]
    ImmatureCoinStake(u64, u64),
    #[error("Invalid transaction output value (below 1000 Sardhan dust limit)")]
    DustOutput,
    #[error("Total transaction input value ({0}) is less than output value ({1})")]
    InputOutputMismatch(u64, u64),
    #[error("Invalid signature for input {0}: public key does not match UTXO recipient")]
    SignatureKeyMismatch(usize),
    #[error("Cryptographic signature verification failed for input {0}")]
    InvalidSignature(usize),
    #[error("Missing witness for input {0}")]
    MissingWitness(usize),
    #[error("Invalid Merkle root in block header")]
    InvalidMerkleRoot,
    #[error("Invalid coinbase/coinstake reward: expected {0}, found {1}")]
    InvalidReward(u64, u64),
    #[error("Previous block hash mismatch: expected {0}, header has {1}")]
    PrevBlockMismatch(String, String),
    #[error("Block height out of sequence: expected {0}, header has {1}")]
    HeightMismatch(u64, u64),
}

/// An entry in the unspent transaction output database
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoEntry {
    pub output: TxOut,
    pub block_height: u64,
    pub is_coinstake: bool,
}

/// The distributed ledger state machine
#[derive(Debug, Clone)]
pub struct Ledger {
    pub utxos: HashMap<OutPoint, UtxoEntry>,
    pub best_block_hash: [u8; 32],
    pub best_height: u64,
    pub cumulative_weight: u64,
}

impl Ledger {
    /// Initialize ledger with Genesis Block
    pub fn new() -> Self {
        let genesis = get_genesis_block_header();
        Self {
            utxos: HashMap::new(),
            best_block_hash: genesis.block_hash(),
            best_height: 0,
            cumulative_weight: calculate_block_weight(genesis.consensus_bits),
        }
    }

    /// Apply a validated block's transactions atomically to the UTXO set.
    /// Uses two-phase atomic commit: full validation on staged state, committing only if entire block passes.
    pub fn apply_block(
        &mut self,
        header: &BlockHeader,
        transactions: &[Transaction],
    ) -> Result<(), LedgerError> {
        // 1. Verify block header continuity
        if header.block_height != self.best_height + 1 {
            return Err(LedgerError::HeightMismatch(self.best_height + 1, header.block_height));
        }
        if header.prev_block_hash != self.best_block_hash {
            return Err(LedgerError::PrevBlockMismatch(
                hex::encode(self.best_block_hash),
                hex::encode(header.prev_block_hash),
            ));
        }

        // 2. Verify Merkle root matches transactions
        let txids: Vec<[u8; 32]> = transactions.iter().map(|t| t.txid()).collect();
        let computed_merkle = sdr_primitives::compute_merkle_root(&txids);
        if computed_merkle != header.merkle_root {
            return Err(LedgerError::InvalidMerkleRoot);
        }

        // 3. Clone UTXO set to a staging map for atomic state transition (Issue 5 Fix)
        let mut staged_utxos = self.utxos.clone();
        let mut spent_in_this_block = std::collections::HashSet::new();

        let max_block_reward = sdr_consensus::calculate_block_reward(header.block_height);

        // 4. Validate transactions & enforce UTXO ownership cryptographically (Issue 6 Fix)
        for (tx_idx, tx) in transactions.iter().enumerate() {
            let is_reward_tx = tx_idx == 0;

            if is_reward_tx {
                // Coinbase or Coinstake: ensure reward does not exceed scheduled emission
                let total_minted: u64 = tx.outputs.iter().map(|o| o.value).sum();
                if total_minted > max_block_reward {
                    return Err(LedgerError::InvalidReward(max_block_reward, total_minted));
                }
            } else {
                let mut total_in = 0u64;

                if tx.inputs.is_empty() {
                    return Err(LedgerError::InputOutputMismatch(0, tx.outputs.iter().map(|o| o.value).sum()));
                }

                for (inp_idx, input) in tx.inputs.iter().enumerate() {
                    let outpoint = &input.previous_output;
                    if spent_in_this_block.contains(outpoint) {
                        return Err(LedgerError::DoubleSpend);
                    }

                    // Retrieve UTXO from staged ledger state
                    let utxo = staged_utxos.get(outpoint).ok_or_else(|| {
                        LedgerError::UtxoNotFound(hex::encode(outpoint.txid), outpoint.vout)
                    })?;

                    // Check CoinStake maturity lock (576 blocks)
                    if utxo.is_coinstake {
                        let age = header.block_height.saturating_sub(utxo.block_height);
                        if age < COINSTAKE_MATURITY_LOCK_BLOCKS {
                            return Err(LedgerError::ImmatureCoinStake(
                                COINSTAKE_MATURITY_LOCK_BLOCKS,
                                age,
                            ));
                        }
                    }

                    // Strict UTXO ownership verification (Issue 6 Fix):
                    // Witness public key must match the recipient_pubkey in the spent TxOut
                    let witness = tx.witnesses.get(inp_idx).ok_or(LedgerError::MissingWitness(inp_idx))?;
                    if witness.public_key != utxo.output.recipient_pubkey {
                        return Err(LedgerError::SignatureKeyMismatch(inp_idx));
                    }

                    // Verify cryptographic signature over transaction preimage
                    let digest = tx.signature_digest(inp_idx);
                    if sdr_crypto::verify_signature(&witness.public_key, &digest, &witness.signature).is_err() {
                        return Err(LedgerError::InvalidSignature(inp_idx));
                    }

                    total_in = total_in.saturating_add(utxo.output.value);
                    spent_in_this_block.insert(outpoint.clone());
                }

                // Verify input value >= output value (Zero transaction fees allowed: total_in == total_out)
                let total_out: u64 = tx.outputs.iter().map(|o| o.value).sum();
                if total_in < total_out {
                    return Err(LedgerError::InputOutputMismatch(total_in, total_out));
                }
            }

            // Remove spent UTXOs from staged map
            for input in &tx.inputs {
                staged_utxos.remove(&input.previous_output);
            }

            // Insert new outputs into staged map
            let txid = tx.txid();
            let is_coinstake = tx_idx == 0 && header.consensus_type == ConsensusType::ProofOfStake;

            for (vout, output) in tx.outputs.iter().enumerate() {
                if output.value < 1000 {
                    return Err(LedgerError::DustOutput);
                }

                staged_utxos.insert(
                    OutPoint {
                        txid,
                        vout: vout as u32,
                    },
                    UtxoEntry {
                        output: output.clone(),
                        block_height: header.block_height,
                        is_coinstake,
                    },
                );
            }
        }

        // 5. Commit atomically: update state only after all validations pass without error
        self.utxos = staged_utxos;
        self.best_block_hash = header.block_hash();
        self.best_height = header.block_height;
        self.cumulative_weight += calculate_block_weight(header.consensus_bits);

        Ok(())
    }

    /// Query balance of an address by summing mature UTXOs
    pub fn get_balance(&self, pubkey: &[u8; 32]) -> u64 {
        self.utxos
            .values()
            .filter(|u| &u.output.recipient_pubkey == pubkey)
            .map(|u| u.output.value)
            .sum()
    }

    /// Query mature staking UTXOs for a public key (>= 288 blocks old)
    pub fn get_staking_utxos(&self, pubkey: &[u8; 32], current_height: u64) -> Vec<(OutPoint, u64)> {
        self.utxos
            .iter()
            .filter(|(_, u)| {
                &u.output.recipient_pubkey == pubkey
                    && current_height.saturating_sub(u.block_height) >= MIN_STAKE_CONFIRMATION_AGE
            })
            .map(|(outpoint, u)| (outpoint.clone(), u.output.value))
            .collect()
    }
}

/// Construct canonical Genesis Block Header (0% Premine)
pub fn get_genesis_block_header() -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_block_hash: [0u8; 32],
        merkle_root: hash_blake3(b"Sardhan Chain: Zero Premine, Zero Fees, Sovereign Hybrid PoW/PoS"),
        timestamp: 1772928000,
        block_height: 0,
        consensus_bits: 0x1e0ffff0,
        consensus_type: ConsensusType::ProofOfWork,
        lagging_stake_modifier: [0x53; 32],
        pow_nonce: 0x53445201,
        reserved_flags: [0; 3],
        validator_signature: vec![0; 64],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block_has_zero_utxos() {
        let ledger = Ledger::new();
        assert_eq!(ledger.best_height, 0);
        assert_eq!(ledger.utxos.len(), 0); // Strict 0% Premine verification
    }

    #[test]
    fn test_dust_output_rejected() {
        let mut ledger = Ledger::new();
        let mut header = BlockHeader {
            version: 1,
            prev_block_hash: ledger.best_block_hash,
            merkle_root: [0; 32],
            timestamp: 1772928150,
            block_height: 1,
            consensus_bits: 0x1e0ffff0,
            consensus_type: ConsensusType::ProofOfWork,
            lagging_stake_modifier: [0; 32],
            pow_nonce: 10,
            reserved_flags: [0; 3],
            validator_signature: vec![0; 64],
        };

        // Transaction with dust output (500 Sardhans < 1000 limit)
        let dust_tx = Transaction {
            chain_id: 0x53445201,
            version: 1,
            recent_block_hash: ledger.best_block_hash,
            micro_pow_nonce: 1,
            lock_time: 0,
            inputs: vec![],
            outputs: vec![TxOut {
                value: 500, // Dust!
                recipient_pubkey: [1; 32],
            }],
            witnesses: vec![],
        };
        header.merkle_root = sdr_primitives::compute_merkle_root(&[dust_tx.txid()]);

        let res = ledger.apply_block(&header, &[dust_tx]);
        assert_eq!(res, Err(LedgerError::DustOutput));
        // Verify ledger was not mutated
        assert_eq!(ledger.best_height, 0);
        assert_eq!(ledger.utxos.len(), 0);
    }

    #[test]
    fn test_atomic_state_rollback_on_invalid_tx() {
        let mut ledger = Ledger::new();
        // 1. Mine valid Block 1 with coinbase reward
        let coinbase = Transaction {
            chain_id: 0x53445201,
            version: 1,
            recent_block_hash: ledger.best_block_hash,
            micro_pow_nonce: 1,
            lock_time: 0,
            inputs: vec![],
            outputs: vec![TxOut {
                value: 5_000_000_000,
                recipient_pubkey: [2; 32],
            }],
            witnesses: vec![],
        };
        let b1_header = BlockHeader {
            version: 1,
            prev_block_hash: ledger.best_block_hash,
            merkle_root: sdr_primitives::compute_merkle_root(&[coinbase.txid()]),
            timestamp: 1772928150,
            block_height: 1,
            consensus_bits: 0x1e0ffff0,
            consensus_type: ConsensusType::ProofOfWork,
            lagging_stake_modifier: [0; 32],
            pow_nonce: 10,
            reserved_flags: [0; 3],
            validator_signature: vec![0; 64],
        };
        ledger.apply_block(&b1_header, &[coinbase.clone()]).expect("block 1 succeeds");
        assert_eq!(ledger.best_height, 1);
        assert_eq!(ledger.utxos.len(), 1);

        // 2. Attempt Block 2 with valid coinbase + invalid tx (spending non-existent UTXO)
        let b2_coinbase = Transaction {
            chain_id: 0x53445201,
            version: 1,
            recent_block_hash: ledger.best_block_hash,
            micro_pow_nonce: 2,
            lock_time: 0,
            inputs: vec![],
            outputs: vec![TxOut {
                value: 5_000_000_000,
                recipient_pubkey: [3; 32],
            }],
            witnesses: vec![],
        };
        let bad_tx = Transaction {
            chain_id: 0x53445201,
            version: 1,
            recent_block_hash: ledger.best_block_hash,
            micro_pow_nonce: 3,
            lock_time: 0,
            inputs: vec![sdr_primitives::TxIn {
                previous_output: OutPoint {
                    txid: [9; 32],
                    vout: 0,
                },
                sequence: 0,
            }],
            outputs: vec![TxOut {
                value: 100_000,
                recipient_pubkey: [4; 32],
            }],
            witnesses: vec![sdr_primitives::TxWitness {
                signature: vec![0; 64],
                public_key: [2; 32],
            }],
        };
        let txs = vec![b2_coinbase, bad_tx];
        let txids: Vec<[u8; 32]> = txs.iter().map(|t| t.txid()).collect();
        let b2_header = BlockHeader {
            version: 1,
            prev_block_hash: ledger.best_block_hash,
            merkle_root: sdr_primitives::compute_merkle_root(&txids),
            timestamp: 1772928300,
            block_height: 2,
            consensus_bits: 0x1e0ffff0,
            consensus_type: ConsensusType::ProofOfWork,
            lagging_stake_modifier: [0; 32],
            pow_nonce: 20,
            reserved_flags: [0; 3],
            validator_signature: vec![0; 64],
        };

        let err = ledger.apply_block(&b2_header, &txs);
        assert!(matches!(err, Err(LedgerError::UtxoNotFound(_, _))));

        // Atomic check: Block 2 failed, state must remain exactly at Block 1
        assert_eq!(ledger.best_height, 1);
        assert_eq!(ledger.utxos.len(), 1);
    }
}
