//! # Sardhan Blockchain Primitives
//!
//! Core data structures:
//! - Canonical fixed 196-byte Block Header (SEC-MED-01)
//! - Segregated Transaction with mandatory ChainID (SEC-CRIT-02) and Block-Bound Micro-PoW (SEC-CRIT-01)
//! - OutPoint, TxIn, TxOut
//! - BLAKE3 Merkle Tree calculation

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use sdr_crypto::{hash_blake3, CryptoError};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};
use thiserror::Error;

pub const SARDHAN_MAINNET_MAGIC: u32 = 0x53445201; // "SDR\x01"
pub const SARDHAN_TESTNET_MAGIC: u32 = 0x53445202; // "SDR\x02"

pub const BLOCK_HEADER_SIZE: usize = 196; // Fixed canonical size (SEC-MED-01)
pub const MIN_TRANSACTION_OUTPUT_VALUE: u64 = 1000; // Dust limit: 1000 Sardhans (0.00001 SDR)

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PrimitiveError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    #[error("Invalid header length: expected 196 bytes, got {0}")]
    InvalidHeaderLength(usize),
    #[error("Invalid transaction format: {0}")]
    InvalidTransaction(String),
    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),
}

/// Consensus block type: 0x01 = PoW block, 0x02 = PoS block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ConsensusType {
    ProofOfWork = 0x01,
    ProofOfStake = 0x02,
}

impl TryFrom<u8> for ConsensusType {
    type Error = PrimitiveError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x01 => Ok(ConsensusType::ProofOfWork),
            0x02 => Ok(ConsensusType::ProofOfStake),
            _ => Err(PrimitiveError::Deserialization(format!("Unknown consensus type 0x{:02x}", v))),
        }
    }
}

/// Fixed 196-byte Block Header
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub prev_block_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub timestamp: u64,
    pub block_height: u64,
    pub consensus_bits: u32,
    pub consensus_type: ConsensusType,
    pub lagging_stake_modifier: [u8; 32], // Entropy from (Height - 100) (SEC-CRIT-03)
    pub pow_nonce: u64,
    pub reserved_flags: [u8; 3],
    pub validator_signature: Vec<u8>, // 64 bytes (Zeroed in PoW, Ed25519 in PoS)
}

impl BlockHeader {
    /// Serialize header to canonical 196-byte array
    pub fn to_bytes(&self) -> [u8; BLOCK_HEADER_SIZE] {
        let mut buf = [0u8; BLOCK_HEADER_SIZE];
        let mut w = Cursor::new(&mut buf[..]);

        w.write_u32::<LittleEndian>(self.version).unwrap();
        w.write_all(&self.prev_block_hash).unwrap();
        w.write_all(&self.merkle_root).unwrap();
        w.write_u64::<LittleEndian>(self.timestamp).unwrap();
        w.write_u64::<LittleEndian>(self.block_height).unwrap();
        w.write_u32::<LittleEndian>(self.consensus_bits).unwrap();
        w.write_u8(self.consensus_type as u8).unwrap();
        w.write_all(&self.lagging_stake_modifier).unwrap();
        w.write_u64::<LittleEndian>(self.pow_nonce).unwrap();
        w.write_all(&self.reserved_flags).unwrap();
        
        let mut sig_buf = [0u8; 64];
        let copy_len = self.validator_signature.len().min(64);
        sig_buf[..copy_len].copy_from_slice(&self.validator_signature[..copy_len]);
        w.write_all(&sig_buf).unwrap();

        buf
    }

    /// Deserialize header from exact 196-byte slice
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PrimitiveError> {
        if bytes.len() != BLOCK_HEADER_SIZE {
            return Err(PrimitiveError::InvalidHeaderLength(bytes.len()));
        }

        let mut r = Cursor::new(bytes);
        let version = r.read_u32::<LittleEndian>().map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;
        let mut prev_block_hash = [0u8; 32];
        r.read_exact(&mut prev_block_hash).map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;
        let mut merkle_root = [0u8; 32];
        r.read_exact(&mut merkle_root).map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;
        let timestamp = r.read_u64::<LittleEndian>().map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;
        let block_height = r.read_u64::<LittleEndian>().map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;
        let consensus_bits = r.read_u32::<LittleEndian>().map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;
        let consensus_type = ConsensusType::try_from(r.read_u8().map_err(|e| PrimitiveError::Deserialization(e.to_string()))?)?;
        let mut lagging_stake_modifier = [0u8; 32];
        r.read_exact(&mut lagging_stake_modifier).map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;
        let pow_nonce = r.read_u64::<LittleEndian>().map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;
        let mut reserved_flags = [0u8; 3];
        r.read_exact(&mut reserved_flags).map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;
        let mut validator_signature = [0u8; 64];
        r.read_exact(&mut validator_signature).map_err(|e| PrimitiveError::Deserialization(e.to_string()))?;

        Ok(Self {
            version,
            prev_block_hash,
            merkle_root,
            timestamp,
            block_height,
            consensus_bits,
            consensus_type,
            lagging_stake_modifier,
            pow_nonce,
            reserved_flags,
            validator_signature: validator_signature.to_vec(),
        })
    }

    /// Compute 32-byte BLAKE3 block hash
    pub fn block_hash(&self) -> [u8; 32] {
        let bytes = self.to_bytes();
        hash_blake3(&bytes)
    }

    /// Signature message digest for PoS blocks (header without signature field)
    pub fn pos_signature_preimage(&self) -> [u8; 32] {
        let bytes = self.to_bytes();
        // Zero out the last 64 bytes (signature slot)
        let mut preimage = [0u8; BLOCK_HEADER_SIZE];
        preimage[0..132].copy_from_slice(&bytes[0..132]);
        hash_blake3(&preimage)
    }
}

/// OutPoint referencing a previous transaction output
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutPoint {
    pub txid: [u8; 32],
    pub vout: u32,
}

/// Transaction Input
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxIn {
    pub previous_output: OutPoint,
    pub sequence: u32,
}

/// Transaction Output (value in Sardhans, 1 SDR = 10^8 Sardhans)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxOut {
    pub value: u64, // In Sardhans
    pub recipient_pubkey: [u8; 32],
}

/// Witness Signature Data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxWitness {
    pub signature: Vec<u8>,
    pub public_key: [u8; 32],
}

/// Segregated Witness Transaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub chain_id: u32,               // 0x53445201 (Mainnet) or 0x53445202 (Testnet) (SEC-CRIT-02)
    pub version: u32,
    pub recent_block_hash: [u8; 32], // Bound to canonical block within last 16 blocks (SEC-CRIT-01)
    pub micro_pow_nonce: u64,        // Client-side anti-spam nonce
    pub lock_time: u32,
    pub inputs: Vec<TxIn>,
    pub outputs: Vec<TxOut>,
    pub witnesses: Vec<TxWitness>,   // Segregated from TxID computation (SEC-HIGH-03)
}

impl Transaction {
    /// Compute canonical preimage for signature generation and verification
    pub fn signature_digest(&self, input_index: usize) -> [u8; 32] {
        let mut buf = Vec::new();
        buf.write_u32::<LittleEndian>(self.chain_id).unwrap();
        buf.write_u32::<LittleEndian>(self.version).unwrap();
        buf.write_all(&self.recent_block_hash).unwrap();
        buf.write_u32::<LittleEndian>(self.lock_time).unwrap();
        buf.write_u32::<LittleEndian>(input_index as u32).unwrap();

        // Commit to target input
        if let Some(input) = self.inputs.get(input_index) {
            buf.write_all(&input.previous_output.txid).unwrap();
            buf.write_u32::<LittleEndian>(input.previous_output.vout).unwrap();
            buf.write_u32::<LittleEndian>(input.sequence).unwrap();
        }

        // Commit to all outputs
        buf.write_u32::<LittleEndian>(self.outputs.len() as u32).unwrap();
        for out in &self.outputs {
            buf.write_u64::<LittleEndian>(out.value).unwrap();
            buf.write_all(&out.recipient_pubkey).unwrap();
        }

        hash_blake3(&buf)
    }

    /// Canonical TxID (excludes witness signatures to prevent transaction malleability)
    pub fn txid(&self) -> [u8; 32] {
        let mut buf = Vec::new();
        buf.write_u32::<LittleEndian>(self.chain_id).unwrap();
        buf.write_u32::<LittleEndian>(self.version).unwrap();
        buf.write_all(&self.recent_block_hash).unwrap();
        buf.write_u64::<LittleEndian>(self.micro_pow_nonce).unwrap();
        buf.write_u32::<LittleEndian>(self.lock_time).unwrap();

        buf.write_u32::<LittleEndian>(self.inputs.len() as u32).unwrap();
        for inp in &self.inputs {
            buf.write_all(&inp.previous_output.txid).unwrap();
            buf.write_u32::<LittleEndian>(inp.previous_output.vout).unwrap();
            buf.write_u32::<LittleEndian>(inp.sequence).unwrap();
        }

        buf.write_u32::<LittleEndian>(self.outputs.len() as u32).unwrap();
        for out in &self.outputs {
            buf.write_u64::<LittleEndian>(out.value).unwrap();
            buf.write_all(&out.recipient_pubkey).unwrap();
        }

        hash_blake3(&buf)
    }

    /// Compute micro-PoW anti-spam hash
    pub fn micro_pow_hash(&self) -> [u8; 32] {
        self.txid()
    }
}

/// Compute BLAKE3 Merkle Root over a list of transaction IDs
pub fn compute_merkle_root(txids: &[[u8; 32]]) -> [u8; 32] {
    if txids.is_empty() {
        return [0u8; 32];
    }
    if txids.len() == 1 {
        return txids[0];
    }

    let mut current_layer = txids.to_vec();
    while current_layer.len() > 1 {
        let mut next_layer = Vec::new();
        for chunk in current_layer.chunks(2) {
            let mut combined = [0u8; 64];
            combined[0..32].copy_from_slice(&chunk[0]);
            if chunk.len() > 1 {
                combined[32..64].copy_from_slice(&chunk[1]);
            } else {
                combined[32..64].copy_from_slice(&chunk[0]); // Duplicate odd element
            }
            next_layer.push(hash_blake3(&combined));
        }
        current_layer = next_layer;
    }
    current_layer[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sec_med_01_fixed_196_byte_header() {
        let header = BlockHeader {
            version: 1,
            prev_block_hash: [0x11; 32],
            merkle_root: [0x22; 32],
            timestamp: 1772928000,
            block_height: 100,
            consensus_bits: 0x1e0ffff0,
            consensus_type: ConsensusType::ProofOfWork,
            lagging_stake_modifier: [0x33; 32],
            pow_nonce: 42,
            reserved_flags: [0; 3],
            validator_signature: vec![0; 64],
        };

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 196);

        let decoded = BlockHeader::from_bytes(&bytes).expect("decoding must succeed");
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_merkle_root_determinism() {
        let tx1 = [0x01; 32];
        let tx2 = [0x02; 32];
        let root = compute_merkle_root(&[tx1, tx2]);
        assert_ne!(root, [0u8; 32]);
    }
}
