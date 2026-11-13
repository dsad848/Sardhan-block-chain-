//! # Sardhan Zero-Fee Mempool Engine
//!
//! Features:
//! - Block-bound micro-PoW anti-spam verification (SEC-CRIT-01)
//! - Dynamic quadratic difficulty scaling during congestion (Task 4)
//! - Full UTXO backing, zero-fee invariance (total_in == total_out), and witness verification
//! - Bounded 64 MiB memory with micro-PoW density eviction
//! - Recent-block expiry pruning (16-block window)
//! - Per-peer token-bucket rate limiting against Sybil/flood attacks

use sdr_consensus::{compact_to_target, hash_less_than_target, U256};
use sdr_crypto::verify_signature;
use sdr_ledger::Ledger;
use sdr_primitives::{
    OutPoint, Transaction, MAX_TX_BYTES, MIN_TRANSACTION_OUTPUT_VALUE,
};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub const MAX_MEMPOOL_BYTES: usize = 67_108_864; // 64 MiB
pub const RECENT_BLOCK_WINDOW: usize = 16;       // Max 16 blocks old
pub const BASE_MICRO_POW_TARGET_BITS: u32 = 0x1f0fffff;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MempoolError {
    #[error("Transaction ChainID {0:#x} does not match network {1:#x} (SEC-CRIT-02)")]
    InvalidChainId(u32, u32),
    #[error("Transaction already exists in mempool")]
    DuplicateTransaction,
    #[error("Transaction RecentBlockHash is expired or unknown (SEC-CRIT-01)")]
    ExpiredRecentBlock,
    #[error("Transaction micro-PoW difficulty is insufficient for current mempool load")]
    InsufficientMicroPoW,
    #[error("In-flight input conflict (double spend detected in mempool)")]
    InFlightConflict,
    #[error("Malformed transaction: {0}")]
    MalformedTransaction(String),
    #[error("Invalid cryptographic signature on input {0}")]
    InvalidSignature(usize),
    #[error("Referenced UTXO missing or already spent")]
    MissingUtxo,
    #[error("Referenced CoinStake UTXO is still immature")]
    ImmatureUtxo,
    #[error("Zero-fee invariant violated: total_in ({0}) != total_out ({1})")]
    NonZeroFeeOrMismatch(u64, u64),
    #[error("Mempool is completely saturated")]
    MempoolFull,
    #[error("Peer rate limit exceeded")]
    RateLimited,
}

#[derive(Debug, Clone)]
struct Bucket {
    tokens: u32,
    last_refill_secs: u64,
}

/// Per-peer token-bucket rate limiter protecting CPU & memory against transaction floods
#[derive(Debug, Clone)]
pub struct PeerRateLimiter {
    buckets: HashMap<String, Bucket>,
    capacity: u32,
    refill_per_sec: u32,
}

impl PeerRateLimiter {
    pub fn new(capacity: u32, refill_per_sec: u32) -> Self {
        Self {
            buckets: HashMap::new(),
            capacity,
            refill_per_sec,
        }
    }

    pub fn allow(&mut self, peer_id: &str, now_secs: u64) -> bool {
        let entry = self.buckets.entry(peer_id.to_string()).or_insert(Bucket {
            tokens: self.capacity,
            last_refill_secs: now_secs,
        });
        let elapsed = now_secs.saturating_sub(entry.last_refill_secs);
        if elapsed > 0 {
            let added = (elapsed as u32).saturating_mul(self.refill_per_sec);
            entry.tokens = (entry.tokens.saturating_add(added)).min(self.capacity);
            entry.last_refill_secs = now_secs;
        }
        if entry.tokens > 0 {
            entry.tokens -= 1;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct MempoolEntry {
    pub tx: Transaction,
    pub txid: [u8; 32],
    pub byte_size: usize,
    pub work_score: u128,
}

pub struct Mempool {
    pub network_chain_id: u32,
    pub base_micro_pow_bits: u32,
    pub max_bytes: usize,
    pub entries: HashMap<[u8; 32], MempoolEntry>,
    pub in_flight_inputs: HashSet<OutPoint>,
    pub total_bytes: usize,
    pub rate_limiter: PeerRateLimiter,
}

impl Mempool {
    pub fn new(network_chain_id: u32) -> Self {
        Self::with_config(network_chain_id, BASE_MICRO_POW_TARGET_BITS, MAX_MEMPOOL_BYTES)
    }

    pub fn with_config(network_chain_id: u32, base_micro_pow_bits: u32, max_bytes: usize) -> Self {
        Self {
            network_chain_id,
            base_micro_pow_bits,
            max_bytes,
            entries: HashMap::new(),
            in_flight_inputs: HashSet::new(),
            total_bytes: 0,
            rate_limiter: PeerRateLimiter::new(30, 10),
        }
    }

    /// Compute current micro-PoW target using quadratic congestion scaling:
    /// `scaled_target = base_target / (1 + 64 * (total_bytes / max_bytes)^2)`
    pub fn current_micro_pow_target(&self) -> [u8; 32] {
        let base = U256::from_be_bytes(&compact_to_target(self.base_micro_pow_bits));
        if self.max_bytes == 0 || self.total_bytes == 0 {
            return base.to_be_bytes();
        }
        // Use scaled integer numerator/denominator to compute quadratic penalty cleanly
        let ratio_bps = ((self.total_bytes as u128) * 10_000 / (self.max_bytes as u128)).min(10_000);
        let penalty = 1u64 + ((64 * ratio_bps * ratio_bps) / 100_000_000) as u64;
        base.mul_div_u64(1, penalty).to_be_bytes()
    }

    /// Compute scalar micro-PoW effort score (higher = harder hash achieved)
    fn tx_work_score(tx: &Transaction, byte_size: usize) -> u128 {
        let h = tx.micro_pow_hash();
        // Leading zero bits + inverted top 64 bits divided by byte_size
        let prefix = u64::from_be_bytes(h[0..8].try_into().unwrap()).max(1);
        (u128::MAX / (prefix as u128)) / (byte_size.max(1) as u128)
    }

    /// Validate and insert a zero-fee transaction against recent block history (stateless + mempool conflicts)
    pub fn insert_transaction(
        &mut self,
        tx: Transaction,
        canonical_recent_blocks: &[[u8; 32]],
    ) -> Result<[u8; 32], MempoolError> {
        self.insert_internal(tx, canonical_recent_blocks, None)
    }

    /// Validate and insert a zero-fee transaction against full Ledger UTXO state & peer rate limiter
    pub fn insert_transaction_with_ledger(
        &mut self,
        tx: Transaction,
        ledger: &Ledger,
        peer_id: Option<&str>,
        now_secs: u64,
    ) -> Result<[u8; 32], MempoolError> {
        if let Some(pid) = peer_id {
            if !self.rate_limiter.allow(pid, now_secs) {
                return Err(MempoolError::RateLimited);
            }
        }
        let recent = ledger.recent_block_hashes(RECENT_BLOCK_WINDOW);
        self.insert_internal(tx, &recent, Some(ledger))
    }

    fn insert_internal(
        &mut self,
        tx: Transaction,
        canonical_recent_blocks: &[[u8; 32]],
        ledger: Option<&Ledger>,
    ) -> Result<[u8; 32], MempoolError> {
        // 1. ChainID separation
        if tx.chain_id != self.network_chain_id {
            return Err(MempoolError::InvalidChainId(tx.chain_id, self.network_chain_id));
        }

        // 2. Structural sanity checks
        if tx.inputs.is_empty() || tx.outputs.is_empty() {
            return Err(MempoolError::MalformedTransaction(
                "Transaction must have >= 1 input and >= 1 output".into(),
            ));
        }
        if tx.witnesses.len() != tx.inputs.len() {
            return Err(MempoolError::MalformedTransaction(
                "Witness count must match input count".into(),
            ));
        }
        for out in &tx.outputs {
            if out.value < MIN_TRANSACTION_OUTPUT_VALUE {
                return Err(MempoolError::MalformedTransaction(
                    "Output below dust limit (1000 Sardhans)".into(),
                ));
            }
        }
        let byte_size = tx.byte_size();
        if byte_size > MAX_TX_BYTES {
            return Err(MempoolError::MalformedTransaction(
                "Transaction exceeds MAX_TX_BYTES".into(),
            ));
        }

        let txid = tx.txid();
        if self.entries.contains_key(&txid) {
            return Err(MempoolError::DuplicateTransaction);
        }

        // 3. Block-bound recent block check (SEC-CRIT-01)
        if !canonical_recent_blocks.contains(&tx.recent_block_hash) {
            return Err(MempoolError::ExpiredRecentBlock);
        }

        // 4. Duplicate inputs inside same tx & in-flight mempool double-spend checks
        let mut seen_inputs = HashSet::new();
        for inp in &tx.inputs {
            if !seen_inputs.insert(&inp.previous_output) {
                return Err(MempoolError::InFlightConflict);
            }
            if self.in_flight_inputs.contains(&inp.previous_output) {
                return Err(MempoolError::InFlightConflict);
            }
        }

        // 5. Micro-PoW difficulty verification against quadratic congestion target
        let required_target = self.current_micro_pow_target();
        let pow_hash = tx.micro_pow_hash();
        if !hash_less_than_target(&pow_hash, &required_target) {
            return Err(MempoolError::InsufficientMicroPoW);
        }

        // 6. Verify UTXO backing against Ledger if provided (zero-fee invariant + maturity + pubkey ownership)
        if let Some(l) = ledger {
            let mut total_in = 0u64;
            for (i, inp) in tx.inputs.iter().enumerate() {
                let Some(utxo) = l.utxos.get(&inp.previous_output) else {
                    return Err(MempoolError::MissingUtxo);
                };
                if utxo.is_coinstake {
                    let age = l.best_height.saturating_sub(utxo.block_height);
                    if age < l.config.coinstake_maturity_lock_blocks {
                        return Err(MempoolError::ImmatureUtxo);
                    }
                }
                if tx.witnesses[i].public_key != utxo.output.recipient_pubkey {
                    return Err(MempoolError::InvalidSignature(i));
                }
                total_in = total_in.saturating_add(utxo.output.value);
            }
            let total_out: u64 = tx
                .outputs
                .iter()
                .try_fold(0u64, |acc, o| acc.checked_add(o.value))
                .ok_or(MempoolError::NonZeroFeeOrMismatch(total_in, u64::MAX))?;
            if total_in != total_out {
                return Err(MempoolError::NonZeroFeeOrMismatch(total_in, total_out));
            }
        }

        // 7. Verify canonical Ed25519 signatures
        for (i, witness) in tx.witnesses.iter().enumerate() {
            let digest = tx.signature_digest(i);
            if verify_signature(&witness.public_key, &digest, &witness.signature).is_err() {
                return Err(MempoolError::InvalidSignature(i));
            }
        }

        // 8. Enforce bounded memory (evict lowest micro-PoW density entries if needed)
        let score = Self::tx_work_score(&tx, byte_size);
        if self.total_bytes + byte_size > self.max_bytes {
            self.evict_for_space(byte_size, score)?;
        }

        // 9. Reserve in-flight inputs and commit
        for inp in &tx.inputs {
            self.in_flight_inputs.insert(inp.previous_output.clone());
        }
        self.total_bytes += byte_size;
        self.entries.insert(
            txid,
            MempoolEntry {
                tx,
                txid,
                byte_size,
                work_score: score,
            },
        );
        Ok(txid)
    }

    fn evict_for_space(&mut self, needed_bytes: usize, incoming_score: u128) -> Result<(), MempoolError> {
        if needed_bytes > self.max_bytes {
            return Err(MempoolError::MempoolFull);
        }
        while self.total_bytes + needed_bytes > self.max_bytes {
            let weakest = self
                .entries
                .values()
                .min_by_key(|e| e.work_score)
                .map(|e| (e.txid, e.work_score));
            match weakest {
                Some((weak_id, weak_score)) if incoming_score > weak_score => {
                    self.remove_single(&weak_id);
                }
                _ => return Err(MempoolError::MempoolFull),
            }
        }
        Ok(())
    }

    fn remove_single(&mut self, txid: &[u8; 32]) {
        if let Some(entry) = self.entries.remove(txid) {
            self.total_bytes = self.total_bytes.saturating_sub(entry.byte_size);
            for inp in &entry.tx.inputs {
                self.in_flight_inputs.remove(&inp.previous_output);
            }
        }
    }

    /// Retrieve prioritized transactions for assembling a new block template up to `max_bytes`
    pub fn get_block_template(&self, max_bytes: usize) -> Vec<Transaction> {
        let mut list: Vec<&MempoolEntry> = self.entries.values().collect();
        list.sort_by(|a, b| b.work_score.cmp(&a.work_score));

        let mut selected = Vec::new();
        let mut used_bytes = 0usize;
        for entry in list {
            if used_bytes + entry.byte_size <= max_bytes {
                used_bytes += entry.byte_size;
                selected.push(entry.tx.clone());
            }
        }
        selected
    }

    /// Clear confirmed transactions from mempool when a block is accepted
    pub fn remove_confirmed(&mut self, confirmed_txids: &[[u8; 32]]) {
        for txid in confirmed_txids {
            self.remove_single(txid);
        }
    }

    /// Prune expired transactions (outside 16-block window) or spent inputs after a new block
    pub fn prune_against_ledger(&mut self, ledger: &Ledger) {
        let recent: HashSet<[u8; 32]> = ledger
            .recent_block_hashes(RECENT_BLOCK_WINDOW)
            .into_iter()
            .collect();
        let to_remove: Vec<[u8; 32]> = self
            .entries
            .iter()
            .filter(|(_, entry)| {
                !recent.contains(&entry.tx.recent_block_hash)
                    || entry
                        .tx
                        .inputs
                        .iter()
                        .any(|inp| !ledger.utxos.contains_key(&inp.previous_output))
            })
            .map(|(id, _)| *id)
            .collect();
        for id in to_remove {
            self.remove_single(&id);
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Solve a valid micro-PoW nonce locally for a signed/unsigned transaction template
    pub fn solve_micro_pow(tx: &mut Transaction, target: &[u8; 32], max_iters: u64) -> bool {
        for nonce in 1..=max_iters {
            tx.micro_pow_nonce = nonce;
            let h = tx.micro_pow_hash();
            if hash_less_than_target(&h, target) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdr_crypto::Keypair;
    use sdr_primitives::{TxIn, TxOut, TxWitness, SARDHAN_MAINNET_MAGIC};

    fn make_signed_tx(
        kp: &Keypair,
        chain_id: u32,
        recent_block: [u8; 32],
        outpoint: OutPoint,
        value: u64,
        target: &[u8; 32],
    ) -> Transaction {
        let mut tx = Transaction {
            chain_id,
            version: 1,
            recent_block_hash: recent_block,
            micro_pow_nonce: 0,
            lock_time: 0,
            inputs: vec![TxIn {
                previous_output: outpoint,
                sequence: 0xFFFFFFFF,
            }],
            outputs: vec![TxOut {
                value,
                recipient_pubkey: Keypair::generate().public_key_bytes(),
            }],
            witnesses: vec![],
        };
        assert!(Mempool::solve_micro_pow(&mut tx, target, 100_000));
        let digest = tx.signature_digest(0);
        let sig = kp.sign(&digest);
        tx.witnesses.push(TxWitness {
            signature: sig.to_vec(),
            public_key: kp.public_key_bytes(),
        });
        tx
    }

    #[test]
    fn test_sec_crit_01_expired_recent_block_rejected() {
        let mut mempool = Mempool::new(SARDHAN_MAINNET_MAGIC);
        let valid_history = [[0x11; 32], [0x22; 32]];
        let kp = Keypair::generate();
        let target = mempool.current_micro_pow_target();

        let tx = make_signed_tx(
            &kp,
            SARDHAN_MAINNET_MAGIC,
            [0x99; 32], // Expired/unknown
            OutPoint {
                txid: [0x01; 32],
                vout: 0,
            },
            10_000,
            &target,
        );

        let res = mempool.insert_transaction(tx, &valid_history);
        assert_eq!(res, Err(MempoolError::ExpiredRecentBlock));
    }

    #[test]
    fn test_quadratic_congestion_scaling_and_eviction() {
        let easy_bits = 0x207fffff;
        let mut mempool = Mempool::with_config(SARDHAN_MAINNET_MAGIC, easy_bits, 450);
        let recent = [[0x11; 32]];
        let kp = Keypair::generate();

        let t1 = mempool.current_micro_pow_target();
        let tx1 = make_signed_tx(
            &kp,
            SARDHAN_MAINNET_MAGIC,
            recent[0],
            OutPoint { txid: [1; 32], vout: 0 },
            10_000,
            &t1,
        );
        mempool.insert_transaction(tx1, &recent).expect("tx1 admitted");

        let t2 = mempool.current_micro_pow_target();
        // Congestion target must tighten after bytes entered
        assert!(t2 < t1);

        let tx2 = make_signed_tx(
            &kp,
            SARDHAN_MAINNET_MAGIC,
            recent[0],
            OutPoint { txid: [2; 32], vout: 0 },
            10_000,
            &t2,
        );
        mempool.insert_transaction(tx2, &recent).expect("tx2 admitted");

        // Trying a 3rd tx exceeds 450 bytes -> either evicts or rejects cleanly
        let tx3 = make_signed_tx(
            &kp,
            SARDHAN_MAINNET_MAGIC,
            recent[0],
            OutPoint { txid: [3; 32], vout: 0 },
            10_000,
            &t2,
        );
        let res3 = mempool.insert_transaction(tx3, &recent);
        assert!(res3.is_ok() || res3 == Err(MempoolError::MempoolFull));
        assert!(mempool.total_bytes <= 450);
    }

    #[test]
    fn test_rate_limiter_blocks_flood() {
        let mut limiter = PeerRateLimiter::new(2, 1);
        assert!(limiter.allow("10.0.0.1", 100));
        assert!(limiter.allow("10.0.0.1", 100));
        assert!(!limiter.allow("10.0.0.1", 100));
        assert!(limiter.allow("10.0.0.1", 101));
    }
}
