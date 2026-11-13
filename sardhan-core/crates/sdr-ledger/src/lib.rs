//! # Sardhan Persistent UTXO Ledger & Fork-Choice State Engine
//!
//! Features:
//! - Persistent append-only checksummed block log (`blocks.bin`) + atomic index in `datadir`
//! - Full consensus enforcement (PoW SardhanHash, PoS kernel + Ed25519 signature, Dual-DAA, MTP-11, H-100 modifier, 64-block channel balance)
//! - Exact 256-bit cumulative chain weight fork-choice with `BlockUndo` reorg support
//! - Weak subjectivity long-range protection (`MAX_REORG_DEPTH = 288`)
//! - Double-signing equivocation detection & validator slashing registry
//! - Address UTXO, transaction history, and block explorer indexes

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use sdr_consensus::{
    calculate_block_reward, calculate_block_weight_u128, calculate_next_channel_bits,
    compute_lagging_stake_modifier, compute_median_time_past, validate_block_timestamp,
    verify_channel_balance, verify_pos_header, verify_pow_header, verify_slashing_evidence,
    ConsensusError, SlashingEvidence, CHANNEL_BALANCE_WINDOW, DAA_WINDOW_BLOCKS,
    MTP_WINDOW_BLOCKS, STAKE_MODIFIER_LAG_BLOCKS,
};
use sdr_crypto::hash_blake3;
use sdr_primitives::{
    compute_merkle_root, Block, BlockHeader, ChainConfig, ConsensusType, Network, OutPoint,
    Transaction, TxOut, MAX_BLOCK_TXS, MIN_TRANSACTION_OUTPUT_VALUE,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub const COINSTAKE_MATURITY_LOCK_BLOCKS: u64 = 576; // Mainnet default (24h)
pub const MIN_STAKE_CONFIRMATION_AGE: u64 = 288;     // Mainnet default (12h)
const DISK_RECORD_MAGIC: u32 = 0x53445242; // "SDRB"

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error("UTXO already spent or not found: txid {0}, vout {1}")]
    UtxoNotFound(String, u32),
    #[error("Double spend detected within block")]
    DoubleSpend,
    #[error("CoinStake is immature: requires {0} confirmations, current age is {1}")]
    ImmatureCoinStake(u64, u64),
    #[error("Staking UTXO is immature: requires {0} confirmations, current age is {1}")]
    ImmatureStakeUtxo(u64, u64),
    #[error("Invalid transaction output value (below 1000 Sardhan dust limit)")]
    DustOutput,
    #[error("Zero-fee invariant violated: total input ({0}) != total output ({1})")]
    InputOutputMismatch(u64, u64),
    #[error("Invalid signature for input {0}: public key does not match UTXO recipient")]
    SignatureKeyMismatch(usize),
    #[error("Cryptographic signature verification failed for input {0}")]
    InvalidSignature(usize),
    #[error("Missing witness for input {0}")]
    MissingWitness(usize),
    #[error("Invalid Merkle root in block header")]
    InvalidMerkleRoot,
    #[error("Invalid coinbase/coinstake reward: max {0}, found {1}")]
    InvalidReward(u64, u64),
    #[error("Previous block hash mismatch or unknown parent: {0}")]
    PrevBlockMismatch(String),
    #[error("Block height out of sequence: expected {0}, header has {1}")]
    HeightMismatch(u64, u64),
    #[error("Block already known: {0}")]
    DuplicateBlock(String),
    #[error("Chain ID mismatch in transaction: expected {0:#x}, got {1:#x}")]
    InvalidChainId(u32, u32),
    #[error("Block must contain at least a coinbase or coinstake transaction")]
    EmptyBlockTransactions,
    #[error("Invalid coinbase/coinstake structure: {0}")]
    InvalidRewardTx(String),
    #[error("Validator {0} has been slashed for double-signing")]
    SlashedValidator(String),
    #[error("Double-signing equivocation detected at height {0} for validator {1}")]
    DoubleSigningDetected(u64, String),
    #[error("Reorg depth {0} exceeds weak-subjectivity limit {1} (long-range attack protection)")]
    ReorgDepthExceeded(u64, u64),
    #[error("Consensus error: {0}")]
    Consensus(#[from] ConsensusError),
    #[error("Storage IO error: {0}")]
    Io(String),
}

/// An entry in the unspent transaction output database
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtxoEntry {
    pub output: TxOut,
    pub block_height: u64,
    pub is_coinstake: bool,
}

/// Undo journal for reverting a connected block during reorganization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockUndo {
    pub spent_utxos: Vec<(OutPoint, UtxoEntry)>,
    pub created_outpoints: Vec<OutPoint>,
}

/// Indexed block metadata in the multi-branch block tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockNode {
    pub block: Block,
    pub block_hash: [u8; 32],
    pub cumulative_weight: u128,
    pub on_main_chain: bool,
}

/// Recorded transaction location for Explorer & Wallet lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRecord {
    pub tx: Transaction,
    pub block_hash: [u8; 32],
    pub block_height: u64,
    pub timestamp: u64,
    pub is_reward_tx: bool,
}

/// Recorded slashing incident for double-signing validators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingRecord {
    pub validator_pubkey: [u8; 32],
    pub block_height: u64,
    pub evidence: SlashingEvidence,
    pub detected_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressTxSummary {
    pub txid: String,
    pub block_height: u64,
    pub timestamp: u64,
    pub direction: String, // "received" | "sent" | "reward"
    pub amount_sardhans: u64,
}

/// The distributed ledger state machine with multi-fork DAG + persistent disk log
#[derive(Debug, Clone)]
pub struct Ledger {
    pub config: ChainConfig,
    pub utxos: HashMap<OutPoint, UtxoEntry>,
    pub blocks: HashMap<[u8; 32], BlockNode>,
    pub main_chain: Vec<[u8; 32]>,
    pub undo_log: HashMap<[u8; 32], BlockUndo>,
    pub tx_index: HashMap<[u8; 32], TxRecord>,
    pub pos_signer_log: HashMap<(u64, [u8; 32]), BlockHeader>,
    pub slashed_validators: HashMap<[u8; 32], SlashingRecord>,
    pub best_block_hash: [u8; 32],
    pub best_height: u64,
    pub cumulative_weight: u128,
    pub datadir: Option<PathBuf>,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger {
    /// Initialize in-memory ledger with Genesis Block (Mainnet config by default)
    pub fn new() -> Self {
        Self::new_for_network(Network::Mainnet)
    }

    /// Initialize in-memory ledger for a specific network (Mainnet, Testnet, Regtest)
    pub fn new_for_network(network: Network) -> Self {
        let config = ChainConfig::for_network(network);
        let genesis_header = get_genesis_header_for_network(network);
        let genesis_hash = genesis_header.block_hash();
        let genesis_weight = calculate_block_weight_u128(genesis_header.consensus_bits);

        let genesis_block = Block {
            header: genesis_header,
            transactions: vec![],
        };

        let mut blocks = HashMap::new();
        blocks.insert(
            genesis_hash,
            BlockNode {
                block: genesis_block,
                block_hash: genesis_hash,
                cumulative_weight: genesis_weight,
                on_main_chain: true,
            },
        );

        Self {
            config,
            utxos: HashMap::new(),
            blocks,
            main_chain: vec![genesis_hash],
            undo_log: HashMap::new(),
            tx_index: HashMap::new(),
            pos_signer_log: HashMap::new(),
            slashed_validators: HashMap::new(),
            best_block_hash: genesis_hash,
            best_height: 0,
            cumulative_weight: genesis_weight,
            datadir: None,
        }
    }

    /// Open or initialize a persistent Ledger backed by `datadir/blocks.bin` + `slashing.json`
    pub fn open(datadir: impl AsRef<Path>, network: Network) -> Result<Self, LedgerError> {
        let dir = datadir.as_ref().to_path_buf();
        fs::create_dir_all(&dir).map_err(|e| LedgerError::Io(e.to_string()))?;

        let mut ledger = Self::new_for_network(network);
        ledger.datadir = Some(dir.clone());

        let blocks_path = dir.join("blocks.bin");
        if blocks_path.exists() {
            let file = File::open(&blocks_path).map_err(|e| LedgerError::Io(e.to_string()))?;
            let mut reader = BufReader::new(file);
            let now_ts = current_unix_timestamp();

            loop {
                let magic = match reader.read_u32::<LittleEndian>() {
                    Ok(m) => m,
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(LedgerError::Io(e.to_string())),
                };
                if magic != DISK_RECORD_MAGIC {
                    return Err(LedgerError::Io(format!(
                        "Corrupt block log magic 0x{magic:08x} in blocks.bin"
                    )));
                }
                let len = reader
                    .read_u32::<LittleEndian>()
                    .map_err(|e| LedgerError::Io(e.to_string()))? as usize;
                let mut payload = vec![0u8; len];
                reader
                    .read_exact(&mut payload)
                    .map_err(|e| LedgerError::Io(e.to_string()))?;
                let mut checksum = [0u8; 4];
                reader
                    .read_exact(&mut checksum)
                    .map_err(|e| LedgerError::Io(e.to_string()))?;
                let expected = hash_blake3(&payload);
                if checksum != expected[0..4] {
                    return Err(LedgerError::Io("Block record checksum mismatch in blocks.bin".into()));
                }
                let block = Block::from_bytes(&payload)
                    .map_err(|e| LedgerError::Io(format!("Failed decoding persisted block: {e}")))?;

                // Replay without re-appending to blocks.bin
                ledger.datadir = None;
                let _ = ledger.submit_block_at(block, now_ts);
                ledger.datadir = Some(dir.clone());
            }
        } else {
            // Create empty blocks.bin cleanly
            File::create(&blocks_path).map_err(|e| LedgerError::Io(e.to_string()))?;
        }

        // Load persisted slashing records if any
        let slashing_path = dir.join("slashing.json");
        if slashing_path.exists() {
            if let Ok(data) = fs::read_to_string(&slashing_path) {
                if let Ok(records) = serde_json::from_str::<Vec<SlashingRecord>>(&data) {
                    for rec in records {
                        let _ = ledger.submit_slashing_evidence(rec.evidence);
                    }
                }
            }
        }

        Ok(ledger)
    }

    fn persist_block(&self, block: &Block) -> Result<(), LedgerError> {
        let Some(ref dir) = self.datadir else {
            return Ok(());
        };
        let blocks_path = dir.join("blocks.bin");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&blocks_path)
            .map_err(|e| LedgerError::Io(e.to_string()))?;

        let payload = block.to_bytes();
        let checksum = hash_blake3(&payload);
        file.write_u32::<LittleEndian>(DISK_RECORD_MAGIC)
            .map_err(|e| LedgerError::Io(e.to_string()))?;
        file.write_u32::<LittleEndian>(payload.len() as u32)
            .map_err(|e| LedgerError::Io(e.to_string()))?;
        file.write_all(&payload)
            .map_err(|e| LedgerError::Io(e.to_string()))?;
        file.write_all(&checksum[0..4])
            .map_err(|e| LedgerError::Io(e.to_string()))?;
        file.sync_data()
            .map_err(|e| LedgerError::Io(e.to_string()))?;
        Ok(())
    }

    fn persist_slashing(&self) -> Result<(), LedgerError> {
        let Some(ref dir) = self.datadir else {
            return Ok(());
        };
        let records: Vec<&SlashingRecord> = self.slashed_validators.values().collect();
        let tmp = dir.join("slashing.json.tmp");
        let dst = dir.join("slashing.json");
        let json = serde_json::to_string_pretty(&records)
            .map_err(|e| LedgerError::Io(e.to_string()))?;
        fs::write(&tmp, json).map_err(|e| LedgerError::Io(e.to_string()))?;
        fs::rename(&tmp, &dst).map_err(|e| LedgerError::Io(e.to_string()))?;
        Ok(())
    }

    /// Walk ancestors backwards from `start_hash` (inclusive) collecting headers newest-first
    fn walk_ancestors(&self, start_hash: [u8; 32], max_count: usize) -> Vec<&BlockNode> {
        let mut res = Vec::with_capacity(max_count.min(128));
        let mut cur = Some(start_hash);
        while let Some(hash) = cur {
            if res.len() >= max_count {
                break;
            }
            let Some(node) = self.blocks.get(&hash) else {
                break;
            };
            res.push(node);
            if node.block.header.block_height == 0 {
                break;
            }
            cur = Some(node.block.header.prev_block_hash);
        }
        res
    }

    /// Compute expected Dual-DAA `consensus_bits` for the next block of `consensus_type` atop `parent_hash`
    pub fn expected_next_bits(&self, parent_hash: [u8; 32], consensus_type: ConsensusType) -> u32 {
        let initial_bits = match consensus_type {
            ConsensusType::ProofOfWork => self.config.pow_initial_bits,
            ConsensusType::ProofOfStake => self.config.pos_initial_bits,
        };
        let max_bits = match consensus_type {
            ConsensusType::ProofOfWork => self.config.max_pow_bits,
            ConsensusType::ProofOfStake => self.config.max_pos_bits,
        };

        let ancestors = self.walk_ancestors(parent_hash, DAA_WINDOW_BLOCKS * 4);
        let mut channel_timestamps = Vec::with_capacity(DAA_WINDOW_BLOCKS);
        let mut last_channel_bits = initial_bits;

        for node in ancestors {
            if node.block.header.block_height > 0 && node.block.header.consensus_type == consensus_type {
                if channel_timestamps.is_empty() {
                    last_channel_bits = node.block.header.consensus_bits;
                }
                channel_timestamps.push(node.block.header.timestamp);
                if channel_timestamps.len() >= DAA_WINDOW_BLOCKS {
                    break;
                }
            }
        }
        channel_timestamps.reverse();
        let target_channel_spacing = self.config.target_block_spacing_secs.saturating_mul(2);
        calculate_next_channel_bits(
            &channel_timestamps,
            last_channel_bits,
            target_channel_spacing,
            max_bits,
        )
    }

    /// Compute expected lagging stake modifier (H - 100) for a block built atop `parent_hash`
    pub fn expected_lagging_modifier(&self, parent_hash: [u8; 32], height: u64) -> [u8; 32] {
        let genesis_hash = self.main_chain[0];
        if height <= STAKE_MODIFIER_LAG_BLOCKS {
            return compute_lagging_stake_modifier(&genesis_hash, height);
        }
        let target_ancestor_height = height - STAKE_MODIFIER_LAG_BLOCKS;
        let ancestors = self.walk_ancestors(parent_hash, (STAKE_MODIFIER_LAG_BLOCKS + 2) as usize);
        for node in ancestors {
            if node.block.header.block_height == target_ancestor_height {
                return compute_lagging_stake_modifier(&node.block_hash, height);
            }
        }
        compute_lagging_stake_modifier(&genesis_hash, height)
    }

    /// Compute Median-Time-Past(11) for a block built atop `parent_hash`
    pub fn median_time_past_for_parent(&self, parent_hash: [u8; 32]) -> u64 {
        let ancestors = self.walk_ancestors(parent_hash, MTP_WINDOW_BLOCKS);
        let mut ts: Vec<u64> = ancestors.iter().map(|n| n.block.header.timestamp).collect();
        ts.reverse();
        compute_median_time_past(&ts)
    }

    /// Submit a block on the tip using current system clock
    pub fn apply_block(
        &mut self,
        header: &BlockHeader,
        transactions: &[Transaction],
    ) -> Result<(), LedgerError> {
        let block = Block {
            header: header.clone(),
            transactions: transactions.to_vec(),
        };
        let now_ts = current_unix_timestamp().max(header.timestamp.min(current_unix_timestamp() + 60));
        self.submit_block_at(block, now_ts).map(|_| ())
    }

    /// Submit a block validating against `now_ts` (handles tip extension, side branches, and fork-choice reorgs)
    pub fn submit_block_at(&mut self, block: Block, now_ts: u64) -> Result<bool, LedgerError> {
        let header = &block.header;
        let block_hash = header.block_hash();

        if self.blocks.contains_key(&block_hash) {
            return Err(LedgerError::DuplicateBlock(hex::encode(block_hash)));
        }

        let parent = self
            .blocks
            .get(&header.prev_block_hash)
            .ok_or_else(|| LedgerError::PrevBlockMismatch(hex::encode(header.prev_block_hash)))?
            .clone();

        if header.block_height != parent.block.header.block_height + 1 {
            return Err(LedgerError::HeightMismatch(
                parent.block.header.block_height + 1,
                header.block_height,
            ));
        }

        // Stateless + header-level consensus verification before attaching to DAG
        self.validate_block_header_on_parent(&block, &parent, now_ts)?;

        let block_weight = calculate_block_weight_u128(header.consensus_bits);
        let new_cumulative_weight = parent.cumulative_weight.saturating_add(block_weight);

        // Fast path: extends active best tip
        if header.prev_block_hash == self.best_block_hash {
            self.connect_tip_block(&block, block_hash, new_cumulative_weight)?;
            self.persist_block(&block)?;
            return Ok(true);
        }

        // Side-branch block: insert into DAG first
        self.blocks.insert(
            block_hash,
            BlockNode {
                block: block.clone(),
                block_hash,
                cumulative_weight: new_cumulative_weight,
                on_main_chain: false,
            },
        );
        self.persist_block(&block)?;

        // Fork-choice comparison: switch only if strictly heavier and within MAX_REORG_DEPTH
        if new_cumulative_weight > self.cumulative_weight {
            self.reorganize_to(block_hash)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Validate header consensus fields, timestamps, Dual-DAA bits, modifier, channel balance, and PoW
    fn validate_block_header_on_parent(
        &mut self,
        block: &Block,
        parent: &BlockNode,
        now_ts: u64,
    ) -> Result<(), LedgerError> {
        let header = &block.header;
        let transactions = &block.transactions;

        if transactions.is_empty() || transactions.len() > MAX_BLOCK_TXS {
            return Err(LedgerError::EmptyBlockTransactions);
        }

        // Merkle root check
        let txids: Vec<[u8; 32]> = transactions.iter().map(|t| t.txid()).collect();
        let computed_merkle = compute_merkle_root(&txids);
        if computed_merkle != header.merkle_root {
            return Err(LedgerError::InvalidMerkleRoot);
        }

        // Chain ID check on all transactions
        for tx in transactions {
            if tx.chain_id != self.config.chain_id {
                return Err(LedgerError::InvalidChainId(self.config.chain_id, tx.chain_id));
            }
        }

        // Timestamp MTP-11 + future drift check
        let mtp = self.median_time_past_for_parent(parent.block_hash);
        validate_block_timestamp(
            header.timestamp,
            mtp,
            now_ts,
            self.config.max_future_drift_secs,
        )?;

        // Dual-DAA difficulty check
        let expected_bits = self.expected_next_bits(parent.block_hash, header.consensus_type);
        if header.consensus_bits != expected_bits {
            return Err(ConsensusError::DifficultyBitsMismatch(
                expected_bits,
                header.consensus_bits,
            )
            .into());
        }

        // Lagging Stake Modifier check
        let expected_mod = self.expected_lagging_modifier(parent.block_hash, header.block_height);
        if header.lagging_stake_modifier != expected_mod {
            return Err(ConsensusError::StakeModifierMismatch.into());
        }

        // 64-block channel balancing window
        let ancestors = self.walk_ancestors(parent.block_hash, CHANNEL_BALANCE_WINDOW - 1);
        let mut window: Vec<ConsensusType> = ancestors
            .iter()
            .filter(|n| n.block.header.block_height > 0)
            .map(|n| n.block.header.consensus_type)
            .collect();
        window.reverse();
        window.push(header.consensus_type);
        verify_channel_balance(&window)?;

        // PoW header verification
        if header.consensus_type == ConsensusType::ProofOfWork {
            verify_pow_header(header)?;
        } else {
            // Double-signing equivocation detection on PoS header
            if let Some(first_witness) = transactions[0].witnesses.first() {
                let staker_pk = first_witness.public_key;
                if self.slashed_validators.contains_key(&staker_pk) {
                    return Err(LedgerError::SlashedValidator(hex::encode(staker_pk)));
                }
                if let Some(prev_header) = self.pos_signer_log.get(&(header.block_height, staker_pk)) {
                    if prev_header.block_hash() != header.block_hash() {
                        let evidence = SlashingEvidence {
                            validator_pubkey: staker_pk,
                            block_height: header.block_height,
                            header_a_bytes: prev_header.to_bytes().to_vec(),
                            header_b_bytes: header.to_bytes().to_vec(),
                        };
                        if verify_slashing_evidence(&evidence).is_ok() {
                            self.record_slashing(evidence)?;
                            return Err(LedgerError::DoubleSigningDetected(
                                header.block_height,
                                hex::encode(staker_pk),
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Connect a block extending the active main tip atomically with full UTXO + PoS kernel validation
    fn connect_tip_block(
        &mut self,
        block: &Block,
        block_hash: [u8; 32],
        new_cumulative_weight: u128,
    ) -> Result<(), LedgerError> {
        let header = &block.header;
        let transactions = &block.transactions;

        let mut staged_utxos = self.utxos.clone();
        let mut spent_in_this_block = HashSet::new();
        let mut undo = BlockUndo {
            spent_utxos: Vec::new(),
            created_outpoints: Vec::new(),
        };

        let max_block_reward = calculate_block_reward(header.block_height);
        let mut po_staker_pk_to_log: Option<[u8; 32]> = None;

        for (tx_idx, tx) in transactions.iter().enumerate() {
            let is_reward_tx = tx_idx == 0;

            if is_reward_tx {
                match header.consensus_type {
                    ConsensusType::ProofOfWork => {
                        if !tx.inputs.is_empty() || !tx.witnesses.is_empty() {
                            return Err(LedgerError::InvalidRewardTx(
                                "PoW coinbase must have 0 inputs and 0 witnesses".into(),
                            ));
                        }
                        if tx.outputs.is_empty() {
                            return Err(LedgerError::InvalidRewardTx(
                                "PoW coinbase must have >= 1 output".into(),
                            ));
                        }
                        let total_minted: u64 = tx.outputs.iter().try_fold(0u64, |acc, o| acc.checked_add(o.value)).ok_or(LedgerError::InvalidReward(max_block_reward, u64::MAX))?;
                        if total_minted > max_block_reward {
                            return Err(LedgerError::InvalidReward(max_block_reward, total_minted));
                        }
                    }
                    ConsensusType::ProofOfStake => {
                        if tx.inputs.is_empty() || tx.witnesses.len() != tx.inputs.len() {
                            return Err(LedgerError::InvalidRewardTx(
                                "PoS coinstake must spend >= 1 mature stake input with matching witnesses".into(),
                            ));
                        }
                        if tx.outputs.is_empty() {
                            return Err(LedgerError::InvalidRewardTx(
                                "PoS coinstake must have >= 1 output".into(),
                            ));
                        }

                        let kernel_outpoint = &tx.inputs[0].previous_output;
                        let kernel_utxo = staged_utxos.get(kernel_outpoint).cloned().ok_or_else(|| {
                            LedgerError::UtxoNotFound(
                                hex::encode(kernel_outpoint.txid),
                                kernel_outpoint.vout,
                            )
                        })?;

                        let kernel_age = header.block_height.saturating_sub(kernel_utxo.block_height);
                        if kernel_age < self.config.min_stake_confirmation_age {
                            return Err(LedgerError::ImmatureStakeUtxo(
                                self.config.min_stake_confirmation_age,
                                kernel_age,
                            ));
                        }

                        let staker_pk = tx.witnesses[0].public_key;
                        if self.slashed_validators.contains_key(&staker_pk) {
                            return Err(LedgerError::SlashedValidator(hex::encode(staker_pk)));
                        }

                        verify_pos_header(
                            header,
                            &staker_pk,
                            kernel_outpoint,
                            kernel_utxo.output.value,
                        )?;
                        po_staker_pk_to_log = Some(staker_pk);

                        // Validate every coinstake input & witness
                        let mut total_in = 0u64;
                        for (inp_idx, input) in tx.inputs.iter().enumerate() {
                            let outpoint = &input.previous_output;
                            if spent_in_this_block.contains(outpoint) {
                                return Err(LedgerError::DoubleSpend);
                            }
                            let utxo = staged_utxos.get(outpoint).cloned().ok_or_else(|| {
                                LedgerError::UtxoNotFound(hex::encode(outpoint.txid), outpoint.vout)
                            })?;
                            if utxo.is_coinstake {
                                let age = header.block_height.saturating_sub(utxo.block_height);
                                if age < self.config.coinstake_maturity_lock_blocks {
                                    return Err(LedgerError::ImmatureCoinStake(
                                        self.config.coinstake_maturity_lock_blocks,
                                        age,
                                    ));
                                }
                            }
                            let witness = tx
                                .witnesses
                                .get(inp_idx)
                                .ok_or(LedgerError::MissingWitness(inp_idx))?;
                            if witness.public_key != utxo.output.recipient_pubkey {
                                return Err(LedgerError::SignatureKeyMismatch(inp_idx));
                            }
                            let digest = tx.signature_digest(inp_idx);
                            if sdr_crypto::verify_signature(&witness.public_key, &digest, &witness.signature).is_err() {
                                return Err(LedgerError::InvalidSignature(inp_idx));
                            }
                            total_in = total_in.saturating_add(utxo.output.value);
                            spent_in_this_block.insert(outpoint.clone());
                            undo.spent_utxos.push((outpoint.clone(), utxo));
                        }

                        let total_out: u64 = tx.outputs.iter().try_fold(0u64, |acc, o| acc.checked_add(o.value)).ok_or(LedgerError::InvalidReward(max_block_reward, u64::MAX))?;
                        let minted = total_out.saturating_sub(total_in);
                        if total_out < total_in || minted > max_block_reward {
                            return Err(LedgerError::InvalidReward(max_block_reward, minted));
                        }
                    }
                }
            } else {
                if tx.inputs.is_empty() || tx.outputs.is_empty() {
                    return Err(LedgerError::InputOutputMismatch(
                        0,
                        tx.outputs.iter().map(|o| o.value).sum(),
                    ));
                }
                if tx.witnesses.len() != tx.inputs.len() {
                    return Err(LedgerError::MissingWitness(tx.witnesses.len()));
                }

                let mut total_in = 0u64;
                for (inp_idx, input) in tx.inputs.iter().enumerate() {
                    let outpoint = &input.previous_output;
                    if spent_in_this_block.contains(outpoint) {
                        return Err(LedgerError::DoubleSpend);
                    }

                    let utxo = staged_utxos.get(outpoint).cloned().ok_or_else(|| {
                        LedgerError::UtxoNotFound(hex::encode(outpoint.txid), outpoint.vout)
                    })?;

                    if utxo.is_coinstake {
                        let age = header.block_height.saturating_sub(utxo.block_height);
                        if age < self.config.coinstake_maturity_lock_blocks {
                            return Err(LedgerError::ImmatureCoinStake(
                                self.config.coinstake_maturity_lock_blocks,
                                age,
                            ));
                        }
                    }

                    let witness = tx
                        .witnesses
                        .get(inp_idx)
                        .ok_or(LedgerError::MissingWitness(inp_idx))?;
                    if witness.public_key != utxo.output.recipient_pubkey {
                        return Err(LedgerError::SignatureKeyMismatch(inp_idx));
                    }

                    let digest = tx.signature_digest(inp_idx);
                    if sdr_crypto::verify_signature(
                        &witness.public_key,
                        &digest,
                        &witness.signature,
                    )
                    .is_err()
                    {
                        return Err(LedgerError::InvalidSignature(inp_idx));
                    }

                    total_in = total_in.saturating_add(utxo.output.value);
                    spent_in_this_block.insert(outpoint.clone());
                    undo.spent_utxos.push((outpoint.clone(), utxo));
                }

                // Zero transaction fee invariant: total_in == total_out strictly
                let total_out: u64 = tx.outputs.iter().try_fold(0u64, |acc, o| acc.checked_add(o.value)).ok_or(LedgerError::InputOutputMismatch(total_in, u64::MAX))?;
                if total_in != total_out {
                    return Err(LedgerError::InputOutputMismatch(total_in, total_out));
                }
            }

            // Remove spent inputs from staged UTXOs
            for input in &tx.inputs {
                staged_utxos.remove(&input.previous_output);
            }

            // Insert new outputs into staged UTXOs
            let txid = tx.txid();
            let is_coinstake =
                tx_idx == 0 && header.consensus_type == ConsensusType::ProofOfStake;

            for (vout, output) in tx.outputs.iter().enumerate() {
                if output.value < MIN_TRANSACTION_OUTPUT_VALUE {
                    return Err(LedgerError::DustOutput);
                }

                let outpoint = OutPoint {
                    txid,
                    vout: vout as u32,
                };
                staged_utxos.insert(
                    outpoint.clone(),
                    UtxoEntry {
                        output: output.clone(),
                        block_height: header.block_height,
                        is_coinstake,
                    },
                );
                undo.created_outpoints.push(outpoint);
            }
        }

        // Atomic commit
        self.utxos = staged_utxos;
        self.best_block_hash = block_hash;
        self.best_height = header.block_height;
        self.cumulative_weight = new_cumulative_weight;
        self.undo_log.insert(block_hash, undo);

        self.blocks.insert(
            block_hash,
            BlockNode {
                block: block.clone(),
                block_hash,
                cumulative_weight: new_cumulative_weight,
                on_main_chain: true,
            },
        );
        self.main_chain.push(block_hash);

        if let Some(staker_pk) = po_staker_pk_to_log {
            self.pos_signer_log
                .insert((header.block_height, staker_pk), header.clone());
        }

        for (idx, tx) in transactions.iter().enumerate() {
            self.tx_index.insert(
                tx.txid(),
                TxRecord {
                    tx: tx.clone(),
                    block_hash,
                    block_height: header.block_height,
                    timestamp: header.timestamp,
                    is_reward_tx: idx == 0,
                },
            );
        }

        Ok(())
    }

    /// Reorganize main chain to `target_tip` if within `MAX_REORG_DEPTH`
    fn reorganize_to(&mut self, target_tip: [u8; 32]) -> Result<(), LedgerError> {
        let new_branch = self.walk_ancestors(target_tip, (self.config.max_reorg_depth + 2) as usize);
        let main_set: HashSet<[u8; 32]> = self.main_chain.iter().copied().collect();

        let mut connect_path: Vec<BlockNode> = Vec::new();
        let mut lca_hash = None;
        for node in new_branch {
            if main_set.contains(&node.block_hash) {
                lca_hash = Some(node.block_hash);
                break;
            }
            connect_path.push(node.clone());
        }

        let Some(lca) = lca_hash else {
            return Err(LedgerError::ReorgDepthExceeded(
                self.config.max_reorg_depth + 1,
                self.config.max_reorg_depth,
            ));
        };
        connect_path.reverse(); // LCA+1 .. target_tip

        let lca_height = self.blocks[&lca].block.header.block_height;
        let reorg_depth = self.best_height.saturating_sub(lca_height);
        if reorg_depth > self.config.max_reorg_depth {
            return Err(LedgerError::ReorgDepthExceeded(
                reorg_depth,
                self.config.max_reorg_depth,
            ));
        }

        // Snapshot full state before mutating in case a block on the side branch fails UTXO validation
        let snapshot = self.clone();

        // Disconnect blocks back to LCA
        while self.best_block_hash != lca {
            let tip_hash = self.best_block_hash;
            self.disconnect_tip_block(tip_hash);
        }

        // Connect new winning branch
        for node in connect_path {
            let parent_weight = self.blocks[&node.block.header.prev_block_hash].cumulative_weight;
            let new_weight = parent_weight
                .saturating_add(calculate_block_weight_u128(node.block.header.consensus_bits));
            if let Err(e) = self.connect_tip_block(&node.block, node.block_hash, new_weight) {
                *self = snapshot;
                return Err(e);
            }
        }

        Ok(())
    }

    fn disconnect_tip_block(&mut self, tip_hash: [u8; 32]) {
        if let Some(undo) = self.undo_log.remove(&tip_hash) {
            for outpoint in undo.created_outpoints {
                self.utxos.remove(&outpoint);
            }
            for (outpoint, entry) in undo.spent_utxos {
                self.utxos.insert(outpoint, entry);
            }
        }

        if let Some(node) = self.blocks.get_mut(&tip_hash) {
            node.on_main_chain = false;
            for tx in &node.block.transactions {
                self.tx_index.remove(&tx.txid());
            }
            let prev = node.block.header.prev_block_hash;
            self.main_chain.pop();
            self.best_block_hash = prev;
            if let Some(parent_node) = self.blocks.get(&prev) {
                self.best_height = parent_node.block.header.block_height;
                self.cumulative_weight = parent_node.cumulative_weight;
            }
        }
    }

    /// Submit third-party slashing evidence against a double-signing PoS validator
    pub fn submit_slashing_evidence(&mut self, evidence: SlashingEvidence) -> Result<(), LedgerError> {
        verify_slashing_evidence(&evidence)?;
        self.record_slashing(evidence)?;
        Ok(())
    }

    fn record_slashing(&mut self, evidence: SlashingEvidence) -> Result<(), LedgerError> {
        let pk = evidence.validator_pubkey;
        self.slashed_validators.insert(
            pk,
            SlashingRecord {
                validator_pubkey: pk,
                block_height: evidence.block_height,
                evidence,
                detected_at: current_unix_timestamp(),
            },
        );
        self.persist_slashing()
    }

    /// Query balance of an address by summing UTXOs
    pub fn get_balance(&self, pubkey: &[u8; 32]) -> u64 {
        self.utxos
            .values()
            .filter(|u| &u.output.recipient_pubkey == pubkey)
            .map(|u| u.output.value)
            .sum()
    }

    /// Query spendable balance (excluding immature coinstake rewards)
    pub fn get_spendable_balance(&self, pubkey: &[u8; 32]) -> u64 {
        self.utxos
            .values()
            .filter(|u| {
                &u.output.recipient_pubkey == pubkey
                    && (!u.is_coinstake
                        || self.best_height.saturating_sub(u.block_height)
                            >= self.config.coinstake_maturity_lock_blocks)
            })
            .map(|u| u.output.value)
            .sum()
    }

    /// List all spendable UTXOs for an address
    pub fn get_address_utxos(&self, pubkey: &[u8; 32]) -> Vec<(OutPoint, UtxoEntry)> {
        let mut res: Vec<(OutPoint, UtxoEntry)> = self
            .utxos
            .iter()
            .filter(|(_, u)| {
                &u.output.recipient_pubkey == pubkey
                    && (!u.is_coinstake
                        || self.best_height.saturating_sub(u.block_height)
                            >= self.config.coinstake_maturity_lock_blocks)
            })
            .map(|(op, u)| (op.clone(), u.clone()))
            .collect();
        res.sort_by_key(|(_, u)| u.block_height);
        res
    }

    /// List confirmed transaction history involving `pubkey`
    pub fn get_address_history(&self, pubkey: &[u8; 32]) -> Vec<AddressTxSummary> {
        let mut history = Vec::new();
        for hash in self.main_chain.iter().rev().take(500) {
            if let Some(node) = self.blocks.get(hash) {
                for (idx, tx) in node.block.transactions.iter().enumerate() {
                    let received: u64 = tx
                        .outputs
                        .iter()
                        .filter(|o| &o.recipient_pubkey == pubkey)
                        .map(|o| o.value)
                        .sum();
                    let spent_mine = tx.witnesses.iter().any(|w| &w.public_key == pubkey);
                    if received > 0 || spent_mine {
                        let direction = if idx == 0 {
                            "reward"
                        } else if spent_mine && received == 0 {
                            "sent"
                        } else if spent_mine {
                            "self/sent"
                        } else {
                            "received"
                        };
                        history.push(AddressTxSummary {
                            txid: hex::encode(tx.txid()),
                            block_height: node.block.header.block_height,
                            timestamp: node.block.header.timestamp,
                            direction: direction.to_string(),
                            amount_sardhans: received,
                        });
                    }
                }
            }
        }
        history
    }

    /// Query mature staking UTXOs for a public key (>= min_stake_confirmation_age blocks old)
    pub fn get_staking_utxos(&self, pubkey: &[u8; 32], current_height: u64) -> Vec<(OutPoint, u64)> {
        self.utxos
            .iter()
            .filter(|(_, u)| {
                &u.output.recipient_pubkey == pubkey
                    && (!u.is_coinstake
                        || current_height.saturating_sub(u.block_height)
                            >= self.config.coinstake_maturity_lock_blocks)
                    && current_height.saturating_sub(u.block_height)
                        >= self.config.min_stake_confirmation_age
            })
            .map(|(outpoint, u)| (outpoint.clone(), u.output.value))
            .collect()
    }

    /// Return canonical recent block hashes (last `limit` main-chain blocks) for micro-PoW expiry checks
    pub fn recent_block_hashes(&self, limit: usize) -> Vec<[u8; 32]> {
        self.main_chain
            .iter()
            .rev()
            .take(limit)
            .copied()
            .collect()
    }

    /// Get block by height on main chain
    pub fn get_block_by_height(&self, height: u64) -> Option<&BlockNode> {
        self.main_chain
            .get(height as usize)
            .and_then(|h| self.blocks.get(h))
    }

    /// Get block by 32-byte hash
    pub fn get_block_by_hash(&self, hash: &[u8; 32]) -> Option<&BlockNode> {
        self.blocks.get(hash)
    }

    /// Return up to `count` latest main-chain blocks newest-first
    pub fn get_latest_blocks(&self, count: usize) -> Vec<&BlockNode> {
        self.main_chain
            .iter()
            .rev()
            .take(count)
            .filter_map(|h| self.blocks.get(h))
            .collect()
    }

    /// Count PoW and PoS blocks in the current 64-block window
    pub fn channel_counts_recent_64(&self) -> (usize, usize) {
        let recent = self.get_latest_blocks(CHANNEL_BALANCE_WINDOW);
        let mut pow = 0usize;
        let mut pos = 0usize;
        for node in recent {
            if node.block.header.block_height == 0 {
                continue;
            }
            match node.block.header.consensus_type {
                ConsensusType::ProofOfWork => pow += 1,
                ConsensusType::ProofOfStake => pos += 1,
            }
        }
        (pow, pos)
    }

    /// Check whether a PoW or PoS block can be appended next without violating the 64-block channel window
    pub fn can_append_channel(&self, candidate_type: ConsensusType) -> bool {
        let mut window: Vec<ConsensusType> = self
            .get_latest_blocks(CHANNEL_BALANCE_WINDOW - 1)
            .into_iter()
            .filter(|n| n.block.header.block_height > 0)
            .map(|n| n.block.header.consensus_type)
            .collect();
        window.reverse();
        window.push(candidate_type);
        verify_channel_balance(&window).is_ok()
    }
}

pub fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1772928000)
}

/// Canonical Genesis Block Header per network (strict 0% premine)
pub fn get_genesis_header_for_network(network: Network) -> BlockHeader {
    let config = ChainConfig::for_network(network);
    let genesis_tag = match network {
        Network::Mainnet => b"Sardhan Chain: Zero Premine, Zero Fees, Sovereign Hybrid PoW/PoS".as_slice(),
        Network::Testnet => b"Sardhan Testnet v2: Public 3-Node L1 Validation Chain".as_slice(),
        Network::Regtest => b"Sardhan Regtest: Deterministic Local Integration Chain".as_slice(),
    };
    BlockHeader {
        version: 1,
        prev_block_hash: [0u8; 32],
        merkle_root: hash_blake3(genesis_tag),
        timestamp: config.genesis_timestamp,
        block_height: 0,
        consensus_bits: config.pow_initial_bits,
        consensus_type: ConsensusType::ProofOfWork,
        lagging_stake_modifier: [0x53; 32],
        pow_nonce: 0x53445201,
        reserved_flags: [0; 3],
        validator_signature: vec![0; 64],
    }
}

/// Construct canonical Mainnet Genesis Block Header (0% Premine)
pub fn get_genesis_block_header() -> BlockHeader {
    get_genesis_header_for_network(Network::Mainnet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdr_consensus::{compact_to_target, hash_less_than_target};
    use sdr_crypto::{sardhan_hash, Keypair};
    use sdr_primitives::{TxIn, TxOut, TxWitness};

    fn mine_regtest_pow(ledger: &Ledger, coinbase_to: [u8; 32], txs: Vec<Transaction>) -> Block {
        let mut all_txs = Vec::with_capacity(1 + txs.len());
        let height = ledger.best_height + 1;
        let reward = calculate_block_reward(height);
        all_txs.push(Transaction {
            chain_id: ledger.config.chain_id,
            version: 1,
            recent_block_hash: ledger.best_block_hash,
            micro_pow_nonce: 0,
            lock_time: 0,
            inputs: vec![],
            outputs: vec![TxOut {
                value: reward,
                recipient_pubkey: coinbase_to,
            }],
            witnesses: vec![],
        });
        all_txs.extend(txs);

        let txids: Vec<[u8; 32]> = all_txs.iter().map(|t| t.txid()).collect();
        let bits = ledger.expected_next_bits(ledger.best_block_hash, ConsensusType::ProofOfWork);
        let lsm = ledger.expected_lagging_modifier(ledger.best_block_hash, height);
        let parent_ts = ledger.blocks[&ledger.best_block_hash].block.header.timestamp;

        let mut header = BlockHeader {
            version: 1,
            prev_block_hash: ledger.best_block_hash,
            merkle_root: compute_merkle_root(&txids),
            timestamp: parent_ts + 15,
            block_height: height,
            consensus_bits: bits,
            consensus_type: ConsensusType::ProofOfWork,
            lagging_stake_modifier: lsm,
            pow_nonce: 0,
            reserved_flags: [0; 3],
            validator_signature: vec![0; 64],
        };

        let target = compact_to_target(bits);
        for nonce in 1..10_000u64 {
            header.pow_nonce = nonce;
            let h = sardhan_hash(&header.to_bytes());
            if hash_less_than_target(&h, &target) {
                return Block {
                    header,
                    transactions: all_txs,
                };
            }
        }
        panic!("regtest PoW mining should succeed quickly");
    }

    #[test]
    fn test_genesis_block_has_zero_utxos() {
        let ledger = Ledger::new();
        assert_eq!(ledger.best_height, 0);
        assert_eq!(ledger.utxos.len(), 0);
    }

    #[test]
    fn test_dust_output_rejected() {
        let mut ledger = Ledger::new_for_network(Network::Regtest);
        let dust_tx = Transaction {
            chain_id: ledger.config.chain_id,
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
        let height = 1;
        let bits = ledger.expected_next_bits(ledger.best_block_hash, ConsensusType::ProofOfWork);
        let lsm = ledger.expected_lagging_modifier(ledger.best_block_hash, height);
        let mut header = BlockHeader {
            version: 1,
            prev_block_hash: ledger.best_block_hash,
            merkle_root: compute_merkle_root(&[dust_tx.txid()]),
            timestamp: ledger.config.genesis_timestamp + 15,
            block_height: height,
            consensus_bits: bits,
            consensus_type: ConsensusType::ProofOfWork,
            lagging_stake_modifier: lsm,
            pow_nonce: 0,
            reserved_flags: [0; 3],
            validator_signature: vec![0; 64],
        };
        let target = compact_to_target(bits);
        for nonce in 1..10_000u64 {
            header.pow_nonce = nonce;
            if hash_less_than_target(&sardhan_hash(&header.to_bytes()), &target) {
                break;
            }
        }

        let res = ledger.apply_block(&header, &[dust_tx]);
        assert_eq!(res, Err(LedgerError::DustOutput));
        assert_eq!(ledger.best_height, 0);
        assert_eq!(ledger.utxos.len(), 0);
    }

    #[test]
    fn test_atomic_state_rollback_on_invalid_tx() {
        let mut ledger = Ledger::new_for_network(Network::Regtest);
        let b1 = mine_regtest_pow(&ledger, [2; 32], vec![]);
        ledger.apply_block(&b1.header, &b1.transactions).expect("block 1 succeeds");
        assert_eq!(ledger.best_height, 1);
        assert_eq!(ledger.utxos.len(), 1);

        let bad_tx = Transaction {
            chain_id: ledger.config.chain_id,
            version: 1,
            recent_block_hash: ledger.best_block_hash,
            micro_pow_nonce: 3,
            lock_time: 0,
            inputs: vec![TxIn {
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
            witnesses: vec![TxWitness {
                signature: vec![0; 64],
                public_key: [2; 32],
            }],
        };

        let b2 = mine_regtest_pow(&ledger, [3; 32], vec![bad_tx]);
        let err = ledger.apply_block(&b2.header, &b2.transactions);
        assert!(matches!(err, Err(LedgerError::UtxoNotFound(_, _))));
        assert_eq!(ledger.best_height, 1);
        assert_eq!(ledger.utxos.len(), 1);
    }

    #[test]
    fn test_persistent_disk_storage_and_fork_reorg() {
        let tmp_dir = std::env::temp_dir().join(format!("sdr_ledger_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_dir);

        let alice = Keypair::generate();
        let bob = Keypair::generate();
        let tip_hash;

        {
            let mut ledger = Ledger::open(&tmp_dir, Network::Regtest).expect("open persistent db");
            let b1 = mine_regtest_pow(&ledger, alice.public_key_bytes(), vec![]);
            ledger.apply_block(&b1.header, &b1.transactions).unwrap();
            let b2 = mine_regtest_pow(&ledger, alice.public_key_bytes(), vec![]);
            ledger.apply_block(&b2.header, &b2.transactions).unwrap();
            tip_hash = ledger.best_block_hash;
            assert_eq!(ledger.get_balance(&alice.public_key_bytes()), 10_000_000_000);
        }

        // Reopen from disk and verify exact recovery
        let mut reopened = Ledger::open(&tmp_dir, Network::Regtest).expect("reopen persistent db");
        assert_eq!(reopened.best_height, 2);
        assert_eq!(reopened.best_block_hash, tip_hash);
        assert_eq!(reopened.get_balance(&alice.public_key_bytes()), 10_000_000_000);

        // Build competing heavier fork from height 1 with 2 blocks (height 2' and 3') -> triggers reorg!
        let fork_parent_hash = reopened.main_chain[1];
        let mut fork_ledger = Ledger::new_for_network(Network::Regtest);
        let b1_saved = reopened.get_block_by_height(1).unwrap().block.clone();
        fork_ledger.apply_block(&b1_saved.header, &b1_saved.transactions).unwrap();
        assert_eq!(fork_ledger.best_block_hash, fork_parent_hash);

        let b2_fork = mine_regtest_pow(&fork_ledger, bob.public_key_bytes(), vec![]);
        fork_ledger.apply_block(&b2_fork.header, &b2_fork.transactions).unwrap();
        let b3_fork = mine_regtest_pow(&fork_ledger, bob.public_key_bytes(), vec![]);
        fork_ledger.apply_block(&b3_fork.header, &b3_fork.transactions).unwrap();

        let switched1 = reopened.submit_block_at(b2_fork, current_unix_timestamp() + 100).unwrap();
        assert!(!switched1); // Equal weight does not reorg
        let switched2 = reopened.submit_block_at(b3_fork, current_unix_timestamp() + 100).unwrap();
        assert!(switched2); // Heavier branch triggers atomic reorg
        assert_eq!(reopened.best_height, 3);
        assert_eq!(reopened.get_balance(&bob.public_key_bytes()), 10_000_000_000);
        assert_eq!(reopened.get_balance(&alice.public_key_bytes()), 5_000_000_000);

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
