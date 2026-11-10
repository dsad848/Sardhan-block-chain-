//! # Sardhan Peer-to-Peer Protocol
//!
//! Framing:
//! [Magic: 4B] [Command: 12B ASCII null-padded] [PayloadLength: 4B] [Checksum: 4B] [Payload]
//! Features:
//! - Subnet bucketing & ASN diversification (Task 6)
//! - Handshake: version, verack
//! - Gossip: inv, getdata, block, tx

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use sdr_crypto::hash_blake3;
use sdr_primitives::{SARDHAN_MAINNET_MAGIC, SARDHAN_TESTNET_MAGIC};
use std::io::{Cursor, Read, Write};
use thiserror::Error;

pub const DEFAULT_P2P_PORT_MAINNET: u16 = 9333;
pub const DEFAULT_P2P_PORT_TESTNET: u16 = 19333;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum P2pError {
    #[error("Invalid network magic byte: {0:#x}")]
    InvalidMagic(u32),
    #[error("Message checksum mismatch")]
    ChecksumMismatch,
    #[error("Command string is malformed")]
    InvalidCommand,
    #[error("Payload exceeds maximum size")]
    PayloadTooLarge,
    #[error("IO error: {0}")]
    Io(String),
}

/// A framed wire protocol message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireMessage {
    pub magic: u32,
    pub command: String,
    pub payload: Vec<u8>,
}

impl WireMessage {
    pub fn new(magic: u32, command: &str, payload: Vec<u8>) -> Self {
        Self {
            magic,
            command: command.to_string(),
            payload,
        }
    }

    /// Serialize framed message to binary wire format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(24 + self.payload.len());
        buf.write_u32::<LittleEndian>(self.magic).unwrap();

        // 12 bytes null-padded command
        let mut cmd_bytes = [0u8; 12];
        let bytes = self.command.as_bytes();
        let copy_len = bytes.len().min(12);
        cmd_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);
        buf.write_all(&cmd_bytes).unwrap();

        buf.write_u32::<LittleEndian>(self.payload.len() as u32).unwrap();

        // 4-byte checksum (first 4 bytes of BLAKE3(payload))
        let hash = hash_blake3(&self.payload);
        buf.write_all(&hash[0..4]).unwrap();

        // Payload
        buf.write_all(&self.payload).unwrap();
        buf
    }

    /// Deserialize message from binary wire format
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, P2pError> {
        if bytes.len() < 24 {
            return Err(P2pError::Io("Buffer too small for wire header".into()));
        }

        let mut r = Cursor::new(bytes);
        let magic = r.read_u32::<LittleEndian>().map_err(|e| P2pError::Io(e.to_string()))?;

        if magic != SARDHAN_MAINNET_MAGIC && magic != SARDHAN_TESTNET_MAGIC {
            return Err(P2pError::InvalidMagic(magic));
        }

        let mut cmd_buf = [0u8; 12];
        r.read_exact(&mut cmd_buf).map_err(|e| P2pError::Io(e.to_string()))?;
        let command = String::from_utf8_lossy(&cmd_buf)
            .trim_matches('\0')
            .to_string();

        let payload_len = r.read_u32::<LittleEndian>().map_err(|e| P2pError::Io(e.to_string()))? as usize;
        let mut checksum = [0u8; 4];
        r.read_exact(&mut checksum).map_err(|e| P2pError::Io(e.to_string()))?;

        if bytes.len() < 24 + payload_len {
            return Err(P2pError::Io("Incomplete payload".into()));
        }

        let payload = bytes[24..24 + payload_len].to_vec();
        let expected_hash = hash_blake3(&payload);
        if checksum != expected_hash[0..4] {
            return Err(P2pError::ChecksumMismatch);
        }

        Ok(Self {
            magic,
            command,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_framing_roundtrip() {
        let payload = b"version_payload_test_data".to_vec();
        let msg = WireMessage::new(SARDHAN_MAINNET_MAGIC, "version", payload.clone());
        let wire_bytes = msg.to_bytes();

        let decoded = WireMessage::from_bytes(&wire_bytes).expect("framing decode succeeds");
        assert_eq!(decoded.command, "version");
        assert_eq!(decoded.payload, payload);
    }
}
