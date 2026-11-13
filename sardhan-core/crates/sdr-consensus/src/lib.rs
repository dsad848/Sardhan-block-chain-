//! # Sardhan Consensus Engine
//!
//! Implementation of:
//! - Exact 256-bit integer arithmetic (`U256`) with zero floating-point consensus math
//! - Exact Cumulative Chain Weight W(C) = sum(floor(2^256 / (Target + 1)))
//! - Dual-DAA (Independent PoW and PoS difficulty retargeting clamped [0.25x, 4.0x])
//! - 64-Block Channel Balancing Invariant [24, 40]
//! - Lagging Stake Modifier H-100 entropy commitment (SEC-CRIT-03)
//! - Median-Time-Past (MTP-11) & future drift timestamp protection
//! - Double-signing equivocation detection and SlashingEvidence verification
//! - Deterministic 21,000,000 SDR Halving Calculator with 0% Premine

use sdr_crypto::{hash_blake3, sardhan_hash, verify_signature, CryptoError};
use sdr_primitives::{BlockHeader, ConsensusType, OutPoint, PrimitiveError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const HALVING_INTERVAL_BLOCKS: u64 = 210_000;
pub const INITIAL_REWARD_SARDHANS: u64 = 5_000_000_000; // 50 SDR
pub const MAX_TOTAL_SUPPLY_SARDHANS: u64 = 2_100_000_000_000_000; // 21M SDR

pub const CHANNEL_BALANCE_WINDOW: usize = 64;
pub const CHANNEL_BALANCE_MIN_POW: usize = 24;
pub const CHANNEL_BALANCE_MAX_POW: usize = 40;

pub const STAKE_MODIFIER_LAG_BLOCKS: u64 = 100;
pub const DAA_WINDOW_BLOCKS: usize = 12;
pub const MTP_WINDOW_BLOCKS: usize = 11;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConsensusError {
    #[error("PoW target difficulty exceeded")]
    DifficultyExceeded,
    #[error("Invalid PoS staking kernel")]
    InvalidStakingKernel,
    #[error("Invalid PoS validator signature: {0}")]
    InvalidPosSignature(String),
    #[error("Channel balance invariant violated: in window of 64, PoW count was {0} (must be 24-40)")]
    ChannelBalanceImbalance(usize),
    #[error("Block timestamp {0} invalid (MTP={1}, max_allowed={2})")]
    InvalidTimestamp(u64, u64, u64),
    #[error("Difficulty bits mismatch: expected 0x{0:08x}, got 0x{1:08x}")]
    DifficultyBitsMismatch(u32, u32),
    #[error("Lagging stake modifier mismatch")]
    StakeModifierMismatch,
    #[error("Genesis block header mismatch")]
    InvalidGenesis,
    #[error("Invalid slashing evidence: {0}")]
    InvalidSlashingEvidence(String),
    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("Primitive error: {0}")]
    Primitive(#[from] PrimitiveError),
}

/// Minimal exact unsigned 256-bit integer (little-endian 4 x u64 limbs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct U256(pub [u64; 4]);

impl U256 {
    pub const ZERO: Self = Self([0, 0, 0, 0]);
    pub const ONE: Self = Self([1, 0, 0, 0]);
    pub const MAX: Self = Self([u64::MAX, u64::MAX, u64::MAX, u64::MAX]);

    pub fn from_u64(v: u64) -> Self {
        Self([v, 0, 0, 0])
    }

    pub fn from_u128(v: u128) -> Self {
        Self([v as u64, (v >> 64) as u64, 0, 0])
    }

    pub fn from_be_bytes(bytes: &[u8; 32]) -> Self {
        let mut limbs = [0u64; 4];
        for i in 0..4 {
            let offset = (3 - i) * 8;
            limbs[i] = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        }
        Self(limbs)
    }

    pub fn to_be_bytes(self) -> [u8; 32] {
        let mut out = [0u8; 32];
        for i in 0..4 {
            let offset = (3 - i) * 8;
            out[offset..offset + 8].copy_from_slice(&self.0[i].to_be_bytes());
        }
        out
    }

    pub fn cmp_val(&self, other: &Self) -> std::cmp::Ordering {
        for i in (0..4).rev() {
            match self.0[i].cmp(&other.0[i]) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        }
        std::cmp::Ordering::Equal
    }

    pub fn is_zero(&self) -> bool {
        self.0 == [0, 0, 0, 0]
    }

    pub fn saturating_add(self, rhs: Self) -> Self {
        let mut res = [0u64; 4];
        let mut carry = 0u128;
        for i in 0..4 {
            let sum = (self.0[i] as u128) + (rhs.0[i] as u128) + carry;
            res[i] = sum as u64;
            carry = sum >> 64;
        }
        if carry > 0 {
            Self::MAX
        } else {
            Self(res)
        }
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let mut res = [0u64; 4];
        let mut borrow = 0u128;
        for i in 0..4 {
            let a = self.0[i] as u128;
            let b = (rhs.0[i] as u128) + borrow;
            if a >= b {
                res[i] = (a - b) as u64;
                borrow = 0;
            } else {
                res[i] = ((1u128 << 64) + a - b) as u64;
                borrow = 1;
            }
        }
        if borrow > 0 {
            None
        } else {
            Some(Self(res))
        }
    }

    pub fn saturating_mul_u64(self, rhs: u64) -> Self {
        let mut res = [0u64; 4];
        let mut carry = 0u128;
        for i in 0..4 {
            let prod = (self.0[i] as u128) * (rhs as u128) + carry;
            res[i] = prod as u64;
            carry = prod >> 64;
        }
        if carry > 0 {
            Self::MAX
        } else {
            Self(res)
        }
    }

    pub fn bits(&self) -> u32 {
        for i in (0..4).rev() {
            if self.0[i] != 0 {
                return (i as u32) * 64 + (64 - self.0[i].leading_zeros());
            }
        }
        0
    }

    pub fn bit(&self, idx: u32) -> bool {
        if idx >= 256 {
            return false;
        }
        let limb = (idx / 64) as usize;
        let offset = idx % 64;
        ((self.0[limb] >> offset) & 1) == 1
    }

    pub fn set_bit(&mut self, idx: u32) {
        if idx < 256 {
            let limb = (idx / 64) as usize;
            let offset = idx % 64;
            self.0[limb] |= 1u64 << offset;
        }
    }

    pub fn shl1(&mut self) -> bool {
        let mut carry = 0u64;
        for i in 0..4 {
            let next_carry = self.0[i] >> 63;
            self.0[i] = (self.0[i] << 1) | carry;
            carry = next_carry;
        }
        carry != 0
    }

    pub fn div_rem(self, divisor: Self) -> (Self, Self) {
        assert!(!divisor.is_zero(), "division by zero in U256");
        if self.cmp_val(&divisor) == std::cmp::Ordering::Less {
            return (Self::ZERO, self);
        }
        let mut quotient = Self::ZERO;
        let mut remainder = Self::ZERO;
        let top_bit = self.bits();
        for bit_idx in (0..top_bit).rev() {
            let overflow = remainder.shl1();
            if self.bit(bit_idx) {
                remainder.0[0] |= 1;
            }
            if overflow || remainder.cmp_val(&divisor) != std::cmp::Ordering::Less {
                // If overflow occurred, remainder was >= 2^256 > divisor, so subtract mod 2^256 cleanly
                if overflow {
                    let diff = Self::MAX.checked_sub(divisor).unwrap().saturating_add(Self::ONE);
                    remainder = remainder.saturating_add(diff);
                } else {
                    remainder = remainder.checked_sub(divisor).unwrap();
                }
                quotient.set_bit(bit_idx);
            }
        }
        (quotient, remainder)
    }

    /// Exact (self * mul) / div using 512-bit intermediate product to avoid overflow
    pub fn mul_div_u64(self, mul: u64, div: u64) -> Self {
        assert!(div > 0, "division by zero");
        // Compute 5-limb (320-bit) product in base 2^64
        let mut prod = [0u64; 5];
        let mut carry = 0u128;
        for i in 0..4 {
            let p = (self.0[i] as u128) * (mul as u128) + carry;
            prod[i] = p as u64;
            carry = p >> 64;
        }
        prod[4] = carry as u64;

        // Divide 5-limb integer by u64 from MSB to LSB
        let mut rem = 0u128;
        let mut q = [0u64; 5];
        for i in (0..5).rev() {
            let cur = (rem << 64) | (prod[i] as u128);
            q[i] = (cur / (div as u128)) as u64;
            rem = cur % (div as u128);
        }

        if q[4] > 0 {
            Self::MAX
        } else {
            Self([q[0], q[1], q[2], q[3]])
        }
    }

    pub fn to_u128_saturating(self) -> u128 {
        if self.0[2] != 0 || self.0[3] != 0 {
            u128::MAX
        } else {
            (self.0[0] as u128) | ((self.0[1] as u128) << 64)
        }
    }
}

/// Convert compact 32-bit difficulty bits (e.g. 0x1e0ffff0) to 256-bit big-endian scalar target
pub fn compact_to_target(compact: u32) -> [u8; 32] {
    let size = (compact >> 24) as usize;
    let word = compact & 0x007fffff;

    let mut target = [0u8; 32];
    if size == 0 {
        return target;
    }
    if size <= 3 {
        let val = word >> (8 * (3 - size));
        target[31] = (val & 0xff) as u8;
        if size > 1 {
            target[30] = ((val >> 8) & 0xff) as u8;
        }
        if size > 2 {
            target[29] = ((val >> 16) & 0xff) as u8;
        }
    } else if size <= 32 {
        let index = 32 - size;
        if index < 32 {
            target[index] = ((word >> 16) & 0xff) as u8;
        }
        if index + 1 < 32 {
            target[index + 1] = ((word >> 8) & 0xff) as u8;
        }
        if index + 2 < 32 {
            target[index + 2] = (word & 0xff) as u8;
        }
    } else {
        return [0xff; 32];
    }
    target
}

/// Convert 256-bit big-endian target back into canonical 32-bit compact difficulty bits
pub fn target_to_compact(target: &[u8; 32]) -> u32 {
    let mut first_nonzero = 32usize;
    for (i, &b) in target.iter().enumerate() {
        if b != 0 {
            first_nonzero = i;
            break;
        }
    }
    if first_nonzero == 32 {
        return 0;
    }

    let mut size = (32 - first_nonzero) as u32;
    let mut mantissa: u32 = if size <= 3 {
        let mut v = 0u32;
        for i in first_nonzero..32 {
            v = (v << 8) | (target[i] as u32);
        }
        v << (8 * (3 - size))
    } else {
        ((target[first_nonzero] as u32) << 16)
            | ((target[first_nonzero + 1] as u32) << 8)
            | (target[first_nonzero + 2] as u32)
    };

    // Keep sign bit (0x00800000) clear per Bitcoin/Sardhan compact encoding
    if (mantissa & 0x0080_0000) != 0 {
        mantissa >>= 8;
        size += 1;
    }

    (size << 24) | (mantissa & 0x007f_ffff)
}

/// Compare two 256-bit hashes (returns true if hash < target)
pub fn hash_less_than_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    for i in 0..32 {
        if hash[i] < target[i] {
            return true;
        }
        if hash[i] > target[i] {
            return false;
        }
    }
    false
}

/// Calculate block reward at height h in Sardhans (1 SDR = 10^8 Sardhans).
/// Height 0 (Genesis) always carries 0 SDR premine.
pub fn calculate_block_reward(height: u64) -> u64 {
    if height == 0 {
        return 0;
    }
    let epoch = (height - 1) / HALVING_INTERVAL_BLOCKS;
    if epoch >= 33 {
        return 0; // Supply capped strictly <= 21M SDR, no perpetual tail emission
    }
    INITIAL_REWARD_SARDHANS >> epoch
}

/// Calculate exact integer cumulative supply emitted across blocks `1..=height`.
pub fn calculate_cumulative_supply(height: u64) -> u64 {
    if height == 0 {
        return 0;
    }
    let mut total: u64 = 0;
    let full_epochs = height / HALVING_INTERVAL_BLOCKS;

    for e in 0..full_epochs.min(33) {
        let ep_reward = INITIAL_REWARD_SARDHANS >> e;
        total = total.saturating_add(ep_reward.saturating_mul(HALVING_INTERVAL_BLOCKS));
    }

    if full_epochs < 33 {
        let remainder_blocks = height % HALVING_INTERVAL_BLOCKS;
        let ep_reward = INITIAL_REWARD_SARDHANS >> full_epochs;
        total = total.saturating_add(ep_reward.saturating_mul(remainder_blocks));
    }

    total
}

/// Exact Block Work / Weight = floor(2^256 / (Target + 1)) as u128
/// Uses identity: floor(2^256 / D) = (U256::MAX - (D - 1)) / D + 1 for all D >= 1.
pub fn calculate_block_weight_u128(consensus_bits: u32) -> u128 {
    let target_bytes = compact_to_target(consensus_bits);
    let target = U256::from_be_bytes(&target_bytes);
    if target.is_zero() {
        return u128::MAX;
    }
    if target == U256::MAX {
        return 1;
    }
    let d = target.saturating_add(U256::ONE);
    let numerator = U256::MAX
        .checked_sub(target)
        .unwrap_or(U256::ZERO);
    let (q, _) = numerator.div_rem(d);
    q.saturating_add(U256::ONE).to_u128_saturating()
}

/// Backwards-compatible u64 wrapper for exact block weight
pub fn calculate_block_weight(consensus_bits: u32) -> u64 {
    let w = calculate_block_weight_u128(consensus_bits);
    u64::try_from(w).unwrap_or(u64::MAX)
}

/// Independent Dual-DAA per consensus channel (PoW or PoS).
/// `channel_timestamps` must contain timestamps in chronological order (oldest -> newest)
/// for up to `DAA_WINDOW_BLOCKS` recent blocks of that same consensus channel.
pub fn calculate_next_channel_bits(
    channel_timestamps: &[u64],
    prev_channel_bits: u32,
    target_channel_spacing_secs: u64,
    max_channel_bits: u32,
) -> u32 {
    if channel_timestamps.len() < 2 {
        return prev_channel_bits;
    }

    let window_start = channel_timestamps.len().saturating_sub(DAA_WINDOW_BLOCKS);
    let slice = &channel_timestamps[window_start..];
    let intervals = (slice.len() - 1) as u64;
    let expected_timespan = intervals.saturating_mul(target_channel_spacing_secs).max(1);

    let oldest = slice[0];
    let newest = slice[slice.len() - 1];
    let raw_actual = newest.saturating_sub(oldest).max(1);

    // Clamp adjustment to [0.25x, 4.0x] per retarget step to resist difficulty oscillation
    let min_actual = (expected_timespan / 4).max(1);
    let max_actual = expected_timespan.saturating_mul(4);
    let clamped_actual = raw_actual.clamp(min_actual, max_actual);

    let prev_target = U256::from_be_bytes(&compact_to_target(prev_channel_bits));
    if prev_target.is_zero() {
        return prev_channel_bits;
    }

    let next_target = prev_target.mul_div_u64(clamped_actual, expected_timespan);
    let max_target = U256::from_be_bytes(&compact_to_target(max_channel_bits));

    let bounded = if next_target.cmp_val(&max_target) == std::cmp::Ordering::Greater {
        max_target
    } else if next_target.is_zero() {
        U256::ONE
    } else {
        next_target
    };

    let compact = target_to_compact(&bounded.to_be_bytes());
    if compact == 0 {
        max_channel_bits
    } else {
        compact
    }
}

/// Deterministic H-100 lagging stake modifier preventing last-block stake grinding (SEC-CRIT-03)
pub fn compute_lagging_stake_modifier(ancestor_block_hash: &[u8; 32], height: u64) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(20 + 32 + 8);
    preimage.extend_from_slice(b"SardhanStakeModifier");
    preimage.extend_from_slice(ancestor_block_hash);
    preimage.extend_from_slice(&height.to_le_bytes());
    hash_blake3(&preimage)
}

/// Compute Median-Time-Past over up to the last 11 block timestamps
pub fn compute_median_time_past(recent_timestamps: &[u64]) -> u64 {
    if recent_timestamps.is_empty() {
        return 0;
    }
    let start = recent_timestamps.len().saturating_sub(MTP_WINDOW_BLOCKS);
    let mut sorted = recent_timestamps[start..].to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

/// Enforce timestamp strictly > MTP(11) and <= wall_clock + max_future_drift_secs
pub fn validate_block_timestamp(
    block_ts: u64,
    mtp: u64,
    current_wall_ts: u64,
    max_future_drift_secs: u64,
) -> Result<(), ConsensusError> {
    let max_allowed = current_wall_ts.saturating_add(max_future_drift_secs);
    if block_ts <= mtp || block_ts > max_allowed {
        return Err(ConsensusError::InvalidTimestamp(block_ts, mtp, max_allowed));
    }
    Ok(())
}

/// Verify Channel Balancing Invariant in a 64-block sliding window (Task 1)
pub fn verify_channel_balance(window: &[ConsensusType]) -> Result<(), ConsensusError> {
    if window.len() < CHANNEL_BALANCE_WINDOW {
        return Ok(());
    }
    let pow_count = window
        .iter()
        .filter(|&&t| t == ConsensusType::ProofOfWork)
        .count();
    if !(CHANNEL_BALANCE_MIN_POW..=CHANNEL_BALANCE_MAX_POW).contains(&pow_count) {
        return Err(ConsensusError::ChannelBalanceImbalance(pow_count));
    }
    Ok(())
}

/// Verify a Proof-of-Work Block Header against SardhanHash and target difficulty
pub fn verify_pow_header(header: &BlockHeader) -> Result<(), ConsensusError> {
    if header.consensus_type != ConsensusType::ProofOfWork {
        return Ok(());
    }

    let bytes = header.to_bytes();
    let pow_hash = sardhan_hash(&bytes);
    let target = compact_to_target(header.consensus_bits);

    if !hash_less_than_target(&pow_hash, &target) {
        return Err(ConsensusError::DifficultyExceeded);
    }
    Ok(())
}

/// Verify a Proof-of-Stake Block Header's validator signature and staking kernel
pub fn verify_pos_header(
    header: &BlockHeader,
    staker_pubkey: &[u8; 32],
    kernel_outpoint: &OutPoint,
    utxo_value: u64,
) -> Result<(), ConsensusError> {
    if header.consensus_type != ConsensusType::ProofOfStake {
        return Ok(());
    }
    if header.pow_nonce != 0 {
        return Err(ConsensusError::InvalidPosSignature(
            "PoS block must have pow_nonce == 0".into(),
        ));
    }
    if header.validator_signature.len() != 64 {
        return Err(ConsensusError::InvalidPosSignature(
            "Missing 64-byte validator signature".into(),
        ));
    }
    let preimage = header.pos_signature_preimage();
    verify_signature(staker_pubkey, &preimage, &header.validator_signature)?;

    if !evaluate_pos_kernel(
        &header.lagging_stake_modifier,
        &kernel_outpoint.txid,
        kernel_outpoint.vout,
        header.timestamp,
        utxo_value,
        header.consensus_bits,
    ) {
        return Err(ConsensusError::InvalidStakingKernel);
    }
    Ok(())
}

/// Evaluate Proof-of-Stake Staking Kernel using exact 256-bit arithmetic:
/// `BLAKE3(Modifier || TxID || Vout || Time) < base_target * max(1, floor(utxo_value / 10^8))`
pub fn evaluate_pos_kernel(
    stake_modifier: &[u8; 32],
    utxo_txid: &[u8; 32],
    utxo_vout: u32,
    timestamp: u64,
    utxo_value: u64,
    target_bits: u32,
) -> bool {
    if utxo_value < 1000 {
        return false;
    }

    let mut preimage = Vec::with_capacity(32 + 32 + 4 + 8);
    preimage.extend_from_slice(stake_modifier);
    preimage.extend_from_slice(utxo_txid);
    preimage.extend_from_slice(&utxo_vout.to_le_bytes());
    preimage.extend_from_slice(&timestamp.to_le_bytes());

    let kernel_hash = hash_blake3(&preimage);
    let base_target = U256::from_be_bytes(&compact_to_target(target_bits));
    if base_target.is_zero() {
        return false;
    }

    let weight_sdr = (utxo_value / 100_000_000).max(1);
    let scaled_target = base_target.saturating_mul_u64(weight_sdr).to_be_bytes();

    hash_less_than_target(&kernel_hash, &scaled_target)
}

/// Cryptographic proof of double-signing (equivocation) at identical PoS height
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashingEvidence {
    pub validator_pubkey: [u8; 32],
    pub block_height: u64,
    pub header_a_bytes: Vec<u8>,
    pub header_b_bytes: Vec<u8>,
}

/// Verify that two conflicting PoS headers at the same height were signed by `validator_pubkey`
pub fn verify_slashing_evidence(evidence: &SlashingEvidence) -> Result<(), ConsensusError> {
    if evidence.header_a_bytes.len() != 196 || evidence.header_b_bytes.len() != 196 {
        return Err(ConsensusError::InvalidSlashingEvidence(
            "Headers must be canonical 196-byte headers".into(),
        ));
    }

    let header_a = BlockHeader::from_bytes(&evidence.header_a_bytes)?;
    let header_b = BlockHeader::from_bytes(&evidence.header_b_bytes)?;

    if header_a.consensus_type != ConsensusType::ProofOfStake
        || header_b.consensus_type != ConsensusType::ProofOfStake
    {
        return Err(ConsensusError::InvalidSlashingEvidence(
            "Both headers must be ProofOfStake blocks".into(),
        ));
    }

    if header_a.block_height != evidence.block_height
        || header_b.block_height != evidence.block_height
    {
        return Err(ConsensusError::InvalidSlashingEvidence(
            "Header block heights must match claimed slashing height".into(),
        ));
    }

    if header_a.block_hash() == header_b.block_hash() {
        return Err(ConsensusError::InvalidSlashingEvidence(
            "Conflicting headers cannot have identical block hashes".into(),
        ));
    }

    verify_signature(
        &evidence.validator_pubkey,
        &header_a.pos_signature_preimage(),
        &header_a.validator_signature,
    )?;
    verify_signature(
        &evidence.validator_pubkey,
        &header_b.pos_signature_preimage(),
        &header_b.validator_signature,
    )?;

    Ok(())
}

/// Verify Genesis Block Header against expected canonical Genesis Header
pub fn verify_genesis_header(
    candidate: &BlockHeader,
    expected_genesis: &BlockHeader,
) -> Result<(), ConsensusError> {
    if candidate.block_hash() != expected_genesis.block_hash() || candidate != expected_genesis {
        return Err(ConsensusError::InvalidGenesis);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdr_crypto::Keypair;

    #[test]
    fn test_task_5_halving_and_21m_cap() {
        assert_eq!(calculate_block_reward(0), 0); // 0% Genesis Premine
        assert_eq!(calculate_block_reward(1), 5_000_000_000); // 50 SDR
        assert_eq!(calculate_block_reward(210_000), 5_000_000_000);
        assert_eq!(calculate_block_reward(210_001), 2_500_000_000); // 25 SDR
        assert_eq!(calculate_block_reward(210_000 * 33 + 1), 0); // Terminated

        let max_supply = calculate_cumulative_supply(210_000 * 35);
        assert!(max_supply <= MAX_TOTAL_SUPPLY_SARDHANS);
        assert_eq!(max_supply, 2_099_999_997_690_000); // Exact 20,999,999.9769 SDR
    }

    #[test]
    fn test_exact_256bit_block_weight_and_compact_roundtrip() {
        let bits = 0x1e0ffff0;
        let w1 = calculate_block_weight_u128(bits);
        let easier_bits = 0x1f0fffff;
        let w2 = calculate_block_weight_u128(easier_bits);
        assert!(w1 > w2);
        assert!(w2 >= 1);

        let target = compact_to_target(bits);
        let roundtrip = target_to_compact(&target);
        assert_eq!(roundtrip, bits);
    }

    #[test]
    fn test_dual_daa_retargeting_clamps() {
        // Blocks coming 10x faster -> target must shrink (difficulty increases), clamped to 0.25x target
        let start_bits = 0x1f0fffff;
        let timestamps = vec![1000, 1005, 1010, 1015, 1020]; // 5s avg vs 300s target
        let next_bits = calculate_next_channel_bits(&timestamps, start_bits, 300, 0x207fffff);
        let prev_t = U256::from_be_bytes(&compact_to_target(start_bits));
        let next_t = U256::from_be_bytes(&compact_to_target(next_bits));
        assert!(next_t < prev_t);
    }

    #[test]
    fn test_task_1_channel_balancing_window() {
        let mut balanced = vec![ConsensusType::ProofOfWork; 32];
        balanced.extend(vec![ConsensusType::ProofOfStake; 32]);
        assert!(verify_channel_balance(&balanced).is_ok());

        let mut imbalanced = vec![ConsensusType::ProofOfWork; 50];
        imbalanced.extend(vec![ConsensusType::ProofOfStake; 14]);
        assert!(verify_channel_balance(&imbalanced).is_err());
    }

    #[test]
    fn test_slashing_double_signing_verification() {
        let staker = Keypair::generate();
        let mut h1 = BlockHeader {
            version: 1,
            prev_block_hash: [0x11; 32],
            merkle_root: [0x22; 32],
            timestamp: 1772928300,
            block_height: 42,
            consensus_bits: 0x200fffff,
            consensus_type: ConsensusType::ProofOfStake,
            lagging_stake_modifier: [0x33; 32],
            pow_nonce: 0,
            reserved_flags: [0; 3],
            validator_signature: vec![0; 64],
        };
        let mut h2 = h1.clone();
        h2.merkle_root = [0x44; 32];

        h1.validator_signature = staker.sign(&h1.pos_signature_preimage()).to_vec();
        h2.validator_signature = staker.sign(&h2.pos_signature_preimage()).to_vec();

        let evidence = SlashingEvidence {
            validator_pubkey: staker.public_key_bytes(),
            block_height: 42,
            header_a_bytes: h1.to_bytes().to_vec(),
            header_b_bytes: h2.to_bytes().to_vec(),
        };
        assert!(verify_slashing_evidence(&evidence).is_ok());
    }

    #[test]
    fn test_mtp_and_timestamp_validation() {
        let ts = vec![100, 110, 120, 130, 140];
        let mtp = compute_median_time_past(&ts);
        assert_eq!(mtp, 120);
        assert!(validate_block_timestamp(120, mtp, 200, 7200).is_err());
        assert!(validate_block_timestamp(121, mtp, 200, 7200).is_ok());
        assert!(validate_block_timestamp(7500, mtp, 200, 7200).is_err());
    }
}
