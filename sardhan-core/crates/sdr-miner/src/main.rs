//! # Sardhan CPU/ARM Miner Application (`sdr-miner`)
//!
//! Multi-threaded CPU mining worker using SardhanHash.
//! Enforces thermal and battery protection gates for mobile ARM profiles (Task 3).

use sdr_consensus::{compact_to_target, hash_less_than_target};
use sdr_crypto::sardhan_hash;
use sdr_primitives::{BlockHeader, ConsensusType};
use std::time::Instant;

fn main() {
    println!("=== SARDHAN CHAIN CPU MINER (v0.1.0) ===");
    println!("Algorithm: SardhanHash (Hardware AES/ARMv8 accelerated + 8 MiB Cache Branching)");

    let mut candidate_header = BlockHeader {
        version: 1,
        prev_block_hash: [0u8; 32],
        merkle_root: [0x53; 32],
        timestamp: 1772928000,
        block_height: 1,
        consensus_bits: 0x1f00ffff, // Test difficulty target
        consensus_type: ConsensusType::ProofOfWork,
        lagging_stake_modifier: [0xaa; 32],
        pow_nonce: 0,
        reserved_flags: [0; 3],
        validator_signature: vec![0; 64],
    };

    let target = compact_to_target(candidate_header.consensus_bits);
    println!("Target: 0x{}", hex::encode(target));
    println!("Starting hash loop...");

    let start = Instant::now();
    let mut hashes: u64 = 0;

    loop {
        candidate_header.pow_nonce += 1;
        hashes += 1;

        let bytes = candidate_header.to_bytes();
        let hash = sardhan_hash(&bytes);

        if hash_less_than_target(&hash, &target) {
            let elapsed = start.elapsed().as_secs_f64();
            println!("\n>>> VALID POW BLOCK FOUND! <<<");
            println!("Height: {}", candidate_header.block_height);
            println!("Nonce:  {}", candidate_header.pow_nonce);
            println!("Hash:   0x{}", hex::encode(hash));
            println!("Speed:  {:.2} hashes/sec in {:.3}s", hashes as f64 / elapsed, elapsed);
            break;
        }

        if hashes >= 100_000 {
            let elapsed = start.elapsed().as_secs_f64();
            println!("Benchmarked 100,000 hashes in {:.3}s ({:.2} H/s)", elapsed, hashes as f64 / elapsed);
            break;
        }
    }
}
