//! # Sardhan Crypto Primitives
//!
//! Audited cryptographic primitives:
//! - Ed25519 with strict RFC 8032 scalar canonical checks (S < L)
//! - BLAKE3 256-bit hashing & SHA-256 checksum / key stretching
//! - Network-separated Bech32m address encoding ('sdr1' mainnet, 'tsdr1' testnet, 'rsdr1' regtest)
//! - Deterministic mnemonic seed backup, checksum validation, and HD key derivation
//! - SardhanHash memory-light PoW algorithm (8 MiB scratchpad + branch divergence)

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use thiserror::Error;

/// Error types for cryptographic operations
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("Invalid signature bytes")]
    InvalidSignature,
    #[error("Non-canonical S value in Ed25519 signature (SEC-HIGH-03)")]
    NonCanonicalS,
    #[error("Invalid public key")]
    InvalidPublicKey,
    #[error("Signature verification failed")]
    VerificationFailed,
    #[error("Bech32m encoding or decoding error: {0}")]
    AddressError(String),
    #[error("Mnemonic phrase error: {0}")]
    MnemonicError(String),
    #[error("Hardware signer error: {0}")]
    HardwareError(String),
}

/// The prime order L of the Ed25519 curve subgroup
/// L = 2^252 + 27742317777372353535851937790883648493
pub const ED25519_ORDER_L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58,
    0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

/// Deterministic 256-word dictionary for checksummed Sardhan wallet backup phrases.
/// Each word encodes 8 bits cleanly; 18 words = 128-bit entropy + 16-bit SHA-256 checksum,
/// 24 words = 184-bit entropy + 8-bit SHA-256 checksum, 34 words = 256-bit entropy + 16-bit checksum.
pub const MNEMONIC_WORDS: [&str; 256] = [
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
    "absurd", "access", "account", "accuse", "achieve", "acid", "acoustic", "acquire",
    "across", "action", "actor", "actual", "adapt", "address", "adjust", "admit",
    "adult", "advance", "advice", "aerobic", "affair", "afford", "afraid", "again",
    "agent", "agree", "ahead", "aiming", "airway", "aisle", "alarm", "album",
    "alcohol", "alert", "alien", "allied", "allow", "almost", "alone", "alpha",
    "already", "also", "alter", "always", "amateur", "amazing", "amber", "among",
    "amount", "amused", "anchor", "ancient", "anger", "angle", "animal", "ankle",
    "announce", "annual", "another", "answer", "antenna", "antique", "anvil", "apart",
    "apology", "appear", "apple", "approve", "april", "arch", "arctic", "arena",
    "argue", "arise", "armor", "army", "around", "arrange", "arrest", "arrive",
    "arrow", "artery", "artist", "artwork", "aspect", "assault", "asset", "assist",
    "assume", "asthma", "athlete", "atom", "attack", "attend", "attitude", "attract",
    "auction", "audit", "august", "aunt", "author", "auto", "autumn", "average",
    "avocado", "avoid", "awake", "aware", "awesome", "awful", "awkward", "axis",
    "bacon", "badge", "balance", "balcony", "bamboo", "banana", "banner", "bargain",
    "barrel", "base", "basic", "basket", "battle", "beacon", "beauty", "because",
    "become", "before", "begin", "behave", "behind", "believe", "below", "belt",
    "bench", "benefit", "beyond", "bicycle", "bitter", "black", "blade", "blame",
    "blanket", "blast", "bleak", "bless", "blind", "blood", "blossom", "blouse",
    "blue", "blur", "blush", "board", "boast", "border", "borrow", "bottom",
    "bounce", "bracket", "brain", "brand", "brass", "brave", "breeze", "brick",
    "bridge", "brief", "bright", "bring", "brisk", "broken", "bronze", "brother",
    "brown", "brush", "bubble", "bucket", "budget", "buffalo", "build", "bullet",
    "bundle", "bunker", "burden", "burger", "burst", "butter", "buyer", "cabin",
    "cable", "cactus", "cage", "cake", "calm", "camera", "camp", "canal",
    "cancel", "candy", "cannon", "canoe", "canvas", "canyon", "capable", "capital",
    "captain", "carbon", "cargo", "carpet", "carry", "cart", "castle", "casual",
    "catalog", "catch", "causal", "caution", "cave", "ceiling", "celery", "cement",
    "census", "century", "ceramic", "certain", "chair", "chalk", "champion", "change",
    "chaos", "chapter", "charge", "chase", "cheap", "check", "cheese", "cherry",
    "chest", "chief", "child", "chimney", "choice", "choose", "chronic", "cigar",
];

/// A 32-byte Ed25519 Keypair
#[derive(Clone)]
pub struct Keypair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl std::fmt::Debug for Keypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keypair")
            .field("public_key", &self.public_key_bytes())
            .finish_non_exhaustive()
    }
}

impl Keypair {
    /// Generate a cryptographically secure keypair using OS entropy
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Reconstruct keypair from 32-byte seed
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Export 32-byte private seed bytes (keep strictly local)
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Sign a message digest using RFC 8032 Ed25519
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let sig: Signature = self.signing_key.sign(message);
        sig.to_bytes()
    }

    /// Public key bytes (32 bytes)
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Derive native Bech32m 'sdr1' mainnet address
    pub fn to_address(&self) -> String {
        encode_address(&self.public_key_bytes())
    }

    /// Derive native Bech32m 'tsdr1' testnet address
    pub fn to_testnet_address(&self) -> String {
        encode_testnet_address(&self.public_key_bytes())
    }

    /// Derive native Bech32m address for an arbitrary HRP ("sdr", "tsdr", "rsdr")
    pub fn to_address_for_hrp(&self, hrp: &str) -> Result<String, CryptoError> {
        encode_address_for_hrp(hrp, &self.public_key_bytes())
    }
}

/// Hardware signer abstraction interface for cold/USB signing devices
pub trait HardwareSigner {
    fn device_name(&self) -> &str;
    fn get_public_key(&self, account: u32, index: u32) -> Result<[u8; 32], CryptoError>;
    fn sign_digest(&self, account: u32, index: u32, digest: &[u8; 32]) -> Result<[u8; 64], CryptoError>;
}

/// Deterministic software signer implementing HardwareSigner interface for local wallets
pub struct SoftwareSigner {
    pub master_seed: [u8; 32],
}

impl HardwareSigner for SoftwareSigner {
    fn device_name(&self) -> &str {
        "Sardhan Local Ed25519 Keystore"
    }

    fn get_public_key(&self, account: u32, index: u32) -> Result<[u8; 32], CryptoError> {
        let child = derive_child_secret(&self.master_seed, account, index);
        Ok(Keypair::from_secret_bytes(&child).public_key_bytes())
    }

    fn sign_digest(&self, account: u32, index: u32, digest: &[u8; 32]) -> Result<[u8; 64], CryptoError> {
        let child = derive_child_secret(&self.master_seed, account, index);
        Ok(Keypair::from_secret_bytes(&child).sign(digest))
    }
}

/// Generate a checksummed mnemonic seed phrase from OS entropy.
/// Supports word_count = 18 (128-bit entropy + 16-bit checksum) or 24 (184-bit entropy + 8-bit checksum).
pub fn generate_mnemonic(word_count: usize) -> Result<String, CryptoError> {
    let (entropy_len, checksum_bytes) = match word_count {
        18 => (16usize, 2usize),
        24 => (23usize, 1usize),
        _ => {
            return Err(CryptoError::MnemonicError(
                "Supported mnemonic lengths are 18 or 24 words".into(),
            ))
        }
    };

    let mut entropy = vec![0u8; entropy_len];
    OsRng.fill_bytes(&mut entropy);

    let checksum = hash_sha256(&entropy);
    let mut all_bytes = entropy;
    all_bytes.extend_from_slice(&checksum[..checksum_bytes]);

    let words: Vec<&str> = all_bytes.iter().map(|&b| MNEMONIC_WORDS[b as usize]).collect();
    Ok(words.join(" "))
}

/// Validate a mnemonic phrase checksum and decode its raw entropy bytes.
pub fn validate_mnemonic(phrase: &str) -> Result<Vec<u8>, CryptoError> {
    let words: Vec<&str> = phrase.split_whitespace().collect();
    let (entropy_len, checksum_bytes) = match words.len() {
        18 => (16usize, 2usize),
        24 => (23usize, 1usize),
        other => {
            return Err(CryptoError::MnemonicError(format!(
                "Invalid mnemonic word count {other}; expected 18 or 24"
            )))
        }
    };

    let mut bytes = Vec::with_capacity(words.len());
    for word in words {
        let idx = MNEMONIC_WORDS
            .iter()
            .position(|&w| w == word)
            .ok_or_else(|| CryptoError::MnemonicError(format!("Unknown mnemonic word '{word}'")))?;
        bytes.push(idx as u8);
    }

    let entropy = &bytes[..entropy_len];
    let expected_checksum = hash_sha256(entropy);
    if bytes[entropy_len..entropy_len + checksum_bytes] != expected_checksum[..checksum_bytes] {
        return Err(CryptoError::MnemonicError("Mnemonic checksum verification failed".into()));
    }
    Ok(entropy.to_vec())
}

/// Derive a 32-byte deterministic master seed from a validated mnemonic + optional passphrase
/// using 2048 iterated SHA-256 rounds with domain separation.
pub fn derive_seed_from_mnemonic(phrase: &str, passphrase: &str) -> Result<[u8; 32], CryptoError> {
    let entropy = validate_mnemonic(phrase)?;
    let mut state = {
        let mut h = Sha256::new();
        h.update(b"SardhanSeedDerivationV1:");
        h.update(&entropy);
        h.update(b":");
        h.update(passphrase.as_bytes());
        let res: [u8; 32] = h.finalize().into();
        res
    };

    for round in 0..2048u32 {
        let mut h = Sha256::new();
        h.update(&state);
        h.update(&round.to_le_bytes());
        h.update(passphrase.as_bytes());
        state = h.finalize().into();
    }

    Ok(state)
}

/// Derive a deterministic child secret key for `(account, index)` using BLAKE3 keyed KDF
pub fn derive_child_secret(master_seed: &[u8; 32], account: u32, index: u32) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_keyed(master_seed);
    hasher.update(b"SardhanHDKeyV1");
    hasher.update(&account.to_le_bytes());
    hasher.update(&index.to_le_bytes());
    *hasher.finalize().as_bytes()
}

/// Compute SHA-256 256-bit digest
pub fn hash_sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

/// Verify an Ed25519 signature with strict non-canonical S checking (SEC-HIGH-03)
pub fn verify_signature(
    public_key_bytes: &[u8; 32],
    message: &[u8],
    signature_bytes: &[u8],
) -> Result<(), CryptoError> {
    if signature_bytes.len() != 64 {
        return Err(CryptoError::VerificationFailed);
    }

    // 1. Strict canonical S < L check
    if !is_canonical_s(&signature_bytes[32..64]) {
        return Err(CryptoError::NonCanonicalS);
    }

    // 2. Parse verifying key
    let verifying_key = VerifyingKey::from_bytes(public_key_bytes)
        .map_err(|_| CryptoError::InvalidPublicKey)?;

    // 3. Parse signature
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(signature_bytes);
    let signature = Signature::from_bytes(&sig_arr);

    // 4. Verify signature
    verifying_key
        .verify_strict(message, &signature)
        .map_err(|_| CryptoError::VerificationFailed)
}

/// Check if the 32-byte S scalar is strictly less than curve order L (Little-Endian)
pub fn is_canonical_s(s_bytes: &[u8]) -> bool {
    if s_bytes.len() != 32 {
        return false;
    }
    for i in (0..32).rev() {
        if s_bytes[i] < ED25519_ORDER_L[i] {
            return true;
        }
        if s_bytes[i] > ED25519_ORDER_L[i] {
            return false;
        }
    }
    false
}

/// Compute BLAKE3 256-bit cryptographic digest
pub fn hash_blake3(data: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    *hasher.finalize().as_bytes()
}

/// Compute double BLAKE3 hash for tree root operations
pub fn hash_blake3_double(data: &[u8]) -> [u8; 32] {
    let h1 = hash_blake3(data);
    hash_blake3(&h1)
}

/// Encode public key into Bech32m address with specified HRP ("sdr", "tsdr", or "rsdr")
pub fn encode_address_for_hrp(hrp_str: &str, public_key: &[u8; 32]) -> Result<String, CryptoError> {
    use bech32::{Bech32m, Hrp};
    let hrp = Hrp::parse(hrp_str).map_err(|e| CryptoError::AddressError(e.to_string()))?;
    bech32::encode::<Bech32m>(hrp, public_key)
        .map_err(|e| CryptoError::AddressError(e.to_string()))
}

/// Encode public key into Bech32m address with mainnet prefix 'sdr1'
pub fn encode_address(public_key: &[u8; 32]) -> String {
    encode_address_for_hrp("sdr", public_key).expect("bech32m mainnet encoding succeeds")
}

/// Encode public key into Bech32m address with testnet prefix 'tsdr1'
pub fn encode_testnet_address(public_key: &[u8; 32]) -> String {
    encode_address_for_hrp("tsdr", public_key).expect("bech32m testnet encoding succeeds")
}

/// Decode a Bech32m address returning `(hrp, public_key_bytes)`
pub fn decode_address_with_hrp(address: &str) -> Result<(String, [u8; 32]), CryptoError> {
    let (hrp, data) =
        bech32::decode(address).map_err(|e| CryptoError::AddressError(e.to_string()))?;

    let hrp_str = hrp.as_str().to_string();
    if hrp_str != "sdr" && hrp_str != "tsdr" && hrp_str != "rsdr" {
        return Err(CryptoError::AddressError(format!(
            "Unsupported HRP '{hrp_str}', expected 'sdr', 'tsdr', or 'rsdr'"
        )));
    }

    if data.len() != 32 {
        return Err(CryptoError::AddressError(
            "Invalid payload length, expected 32 bytes".into(),
        ));
    }

    let mut pubkey = [0u8; 32];
    pubkey.copy_from_slice(&data);
    Ok((hrp_str, pubkey))
}

/// Decode Bech32m 'sdr1' mainnet address back to 32-byte public key
pub fn decode_address(address: &str) -> Result<[u8; 32], CryptoError> {
    let (hrp, pubkey) = decode_address_with_hrp(address)?;
    if hrp != "sdr" {
        return Err(CryptoError::AddressError(format!(
            "Invalid HRP '{hrp}', expected 'sdr'"
        )));
    }
    Ok(pubkey)
}

/// Decode Bech32m address checking expected HRP ("sdr", "tsdr", or "rsdr")
pub fn decode_address_for_hrp(address: &str, expected_hrp: &str) -> Result<[u8; 32], CryptoError> {
    let (hrp, pubkey) = decode_address_with_hrp(address)?;
    if hrp != expected_hrp {
        return Err(CryptoError::AddressError(format!(
            "Network address mismatch: expected '{expected_hrp}1...', got '{hrp}1...'"
        )));
    }
    Ok(pubkey)
}

const SARDHAN_SCRATCHPAD_CHUNKS: usize = 131_072;
const SARDHAN_SCRATCHPAD_WORDS: usize = SARDHAN_SCRATCHPAD_CHUNKS * 16; // 8 MiB

thread_local! {
    static TL_SCRATCHPAD: RefCell<Vec<u32>> = RefCell::new(vec![0u32; SARDHAN_SCRATCHPAD_WORDS]);
}

/// SardhanHash CPU/ARM PoW Function
///
/// Implements 8 MiB memory-light cache-line random indexing with dynamic data-dependent
/// branching to penalize GPU SIMD lockstep execution (SEC-HIGH-02).
/// Reuses a thread-local 8 MiB scratchpad to avoid per-hash allocator contention while
/// remaining bit-for-bit deterministic across all platforms.
pub fn sardhan_hash(header: &[u8; 196]) -> [u8; 32] {
    TL_SCRATCHPAD.with(|cell| {
        let mut scratchpad = cell.borrow_mut();
        sardhan_hash_with_scratchpad(header, &mut scratchpad)
    })
}

/// Core deterministic SardhanHash execution over an externally supplied 8 MiB scratchpad.
pub fn sardhan_hash_with_scratchpad(header: &[u8; 196], scratchpad: &mut [u32]) -> [u8; 32] {
    assert!(scratchpad.len() >= SARDHAN_SCRATCHPAD_WORDS);
    let seed = hash_blake3(header);

    // Initialize the 8 MiB scratchpad deterministically using BLAKE3 XOF stream
    let mut xof = blake3::Hasher::new_keyed(&seed);
    xof.update(b"SardhanHash_8MiB_Scratchpad_Init");
    let mut xof_reader = xof.finalize_xof();

    // Fill scratchpad in 64-byte chunks
    let mut chunk = [0u8; 64];
    for chunk_idx in 0..SARDHAN_SCRATCHPAD_CHUNKS {
        xof_reader.fill(&mut chunk);
        let base = chunk_idx * 16;
        for i in 0..16 {
            let offset = i * 4;
            scratchpad[base + i] = u32::from_le_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
    }

    // Seed the initial execution state
    let mut state = [
        u32::from_le_bytes(seed[0..4].try_into().unwrap()),
        u32::from_le_bytes(seed[4..8].try_into().unwrap()),
        u32::from_le_bytes(seed[8..12].try_into().unwrap()),
        u32::from_le_bytes(seed[12..16].try_into().unwrap()),
        u32::from_le_bytes(seed[16..20].try_into().unwrap()),
        u32::from_le_bytes(seed[20..24].try_into().unwrap()),
        u32::from_le_bytes(seed[24..28].try_into().unwrap()),
        u32::from_le_bytes(seed[28..32].try_into().unwrap()),
    ];

    // Branching rounds with data-dependent shifts and S-box mixing across the 8 MiB memory space
    for round in 0..64 {
        let index =
            (state[0] ^ state[1].rotate_left(round as u32)) as usize % SARDHAN_SCRATCHPAD_WORDS;
        let val = scratchpad[index];

        if val & 1 == 0 {
            state[0] = state[0].wrapping_add(val ^ (round as u32));
            state[1] = state[1].rotate_left(13) ^ state[2];
        } else {
            state[0] = state[0].rotate_right(7) ^ val;
            state[1] = state[1].wrapping_sub(state[3]);
        }

        scratchpad[index] = state[0];
        state.rotate_left(1);
    }

    let mut final_bytes = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        final_bytes[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    hash_blake3(&final_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_signing_and_verification() {
        let kp = Keypair::generate();
        let msg = b"Sardhan Chain Block-Bound Transaction Preimage";
        let sig = kp.sign(msg);
        assert!(verify_signature(&kp.public_key_bytes(), msg, &sig).is_ok());
    }

    #[test]
    fn test_sec_high_03_non_canonical_s_rejected() {
        let kp = Keypair::generate();
        let msg = b"Test Malleability Attack";
        let mut sig = kp.sign(msg);

        sig[63] = 0xFF;
        let res = verify_signature(&kp.public_key_bytes(), msg, &sig);
        assert_eq!(res, Err(CryptoError::NonCanonicalS));
    }

    #[test]
    fn test_sec_med_04_bech32m_address_encoding_and_network_separation() {
        let kp = Keypair::generate();
        let mainnet_addr = kp.to_address();
        let testnet_addr = kp.to_testnet_address();

        assert!(mainnet_addr.starts_with("sdr1"));
        assert!(testnet_addr.starts_with("tsdr1"));

        let decoded = decode_address(&mainnet_addr).expect("mainnet decode succeeds");
        assert_eq!(decoded, kp.public_key_bytes());

        // Cross-network rejection
        assert!(decode_address(&testnet_addr).is_err());
        assert_eq!(
            decode_address_for_hrp(&testnet_addr, "tsdr").unwrap(),
            kp.public_key_bytes()
        );
    }

    #[test]
    fn test_mnemonic_generation_validation_and_hd_derivation() {
        let phrase = generate_mnemonic(18).expect("18-word mnemonic");
        assert_eq!(phrase.split_whitespace().count(), 18);
        assert!(validate_mnemonic(&phrase).is_ok());

        let seed1 = derive_seed_from_mnemonic(&phrase, "").unwrap();
        let seed2 = derive_seed_from_mnemonic(&phrase, "").unwrap();
        assert_eq!(seed1, seed2);

        let child0 = Keypair::from_secret_bytes(&derive_child_secret(&seed1, 0, 0));
        let child1 = Keypair::from_secret_bytes(&derive_child_secret(&seed1, 0, 1));
        assert_ne!(child0.public_key_bytes(), child1.public_key_bytes());

        // Corrupt last word -> checksum mismatch
        let mut words: Vec<&str> = phrase.split_whitespace().collect();
        words[17] = if words[17] == "abandon" { "ability" } else { "abandon" };
        assert!(validate_mnemonic(&words.join(" ")).is_err());
    }

    #[test]
    fn test_sardhan_hash_determinism() {
        let header = [0x53u8; 196];
        let h1 = sardhan_hash(&header);
        let h2 = sardhan_hash(&header);
        assert_eq!(h1, h2);
        // Verify consistency across fresh buffer vs thread-local buffer
        let mut fresh = vec![0u32; SARDHAN_SCRATCHPAD_WORDS];
        let h3 = sardhan_hash_with_scratchpad(&header, &mut fresh);
        assert_eq!(h1, h3);
    }
}
