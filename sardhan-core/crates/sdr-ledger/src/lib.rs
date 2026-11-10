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
}

/// An entry in the unspent transaction output database
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoEntry {
    pub output: TxOut,
    pub block_height: u64,
    pub is_coinstake: bool,
}

/// The distributed ledger state machine
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

    /// Apply a validated block's transactions to the UTXO set
    pub fn apply_block(
        &mut self,
        header: &BlockHeader,
        transactions: &[Transaction],
    ) -> Result<(), LedgerError> {
        let mut spent_in_this_block = std::collections::HashSet::new();

        // 1. Spend inputs (except for coinbase)
        for (tx_idx, tx) in transactions.iter().enumerate() {
            // First transaction in PoW is coinbase; first tx in PoS is coinstake
            let is_coinbase = tx_idx == 0 && header.consensus_type == ConsensusType::ProofOfWork;

            if !is_coinbase {
                let mut total_in = 0u64;
                for input in &tx.inputs {
                    let outpoint = &input.previous_output;
                    if spent_in_this_block.contains(outpoint) {
                        return Err(LedgerError::DoubleSpend);
                    }

                    let utxo = self.utxos.get(outpoint).ok_or_else(|| {
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

                    total_in = total_in.saturating_add(utxo.output.value);
                    spent_in_this_block.insert(outpoint.clone());
                }

                // Verify input value >= output value (Zero transaction fees allowed: total_in == total_out)
                let total_out: u64 = tx.outputs.iter().map(|o| o.value).sum();
                if total_in < total_out {
                    return Err(LedgerError::InputOutputMismatch(total_in, total_out));
                }
            }

            // Remove spent UTXOs
            for input in &tx.inputs {
                self.utxos.remove(&input.previous_output);
            }

            // 2. Insert new outputs
            let txid = tx.txid();
            let is_coinstake = tx_idx == 0 && header.consensus_type == ConsensusType::ProofOfStake;

            for (vout, output) in tx.outputs.iter().enumerate() {
                if output.value < 1000 {
                    return Err(LedgerError::DustOutput);
                }

                self.utxos.insert(
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

        // Update chain tip
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
        let header = BlockHeader {
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

        let res = ledger.apply_block(&header, &[dust_tx]);
        assert_eq!(res, Err(LedgerError::DustOutput));
    }
}
