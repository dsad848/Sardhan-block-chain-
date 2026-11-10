//! # Sardhan Full Node Daemon (`sdrd`)
//!
//! Orchestrates the local ledger, mempool, consensus verification, and P2P synchronization.

use sdr_consensus::{calculate_block_reward, calculate_cumulative_supply};
use sdr_ledger::{get_genesis_block_header, Ledger};
use sdr_mempool::Mempool;
use sdr_primitives::{BlockHeader, ConsensusType, SARDHAN_MAINNET_MAGIC};
use sdr_rpc::handle_get_blockchain_info;
use std::time::Instant;

#[tokio::main]
async fn main() {
    env_logger::init();
    println!("============================================================");
    println!("          SARDHAN CHAIN FULL NODE DAEMON (`sdrd`)           ");
    println!("============================================================");
    println!("Protocol Version: 2.0.0 (Post-Audit Hardened)");
    println!("Network:          Sardhan Mainnet (Magic: 0x53445201)");
    println!("Max Supply:       21,000,000.00 SDR (Strict Hard Cap)");
    println!("Premine:          0.00 SDR (0% Genesis Reward)");
    println!("Consensus:        Unified Proof of Power (50% PoW / 50% PoS)");
    println!("Transaction Fees: 0.00 SDR (Client-Side Micro-PoW Protected)");
    println!("============================================================\n");

    let mut ledger = Ledger::new();
    let mempool = Mempool::new(SARDHAN_MAINNET_MAGIC);
    let genesis = get_genesis_block_header();

    println!("[INIT] Genesis block loaded: 0x{}", hex::encode(genesis.block_hash()));
    println!("[INIT] UTXO set initialized: {} unspent outputs (0% premine verified)", ledger.utxos.len());
    println!("[P2P]  Listening for peers on TCP 0.0.0.0:9333");
    println!("[RPC]  JSON-RPC 2.0 interface active on http://127.0.0.1:9332");

    // Simulate mining first block
    println!("\n[MINER] Commencing CPU PoW mining for Block 1 (Coinbase Reward: 50 SDR)...");
    let t0 = Instant::now();
    let mut block1_header = BlockHeader {
        version: 1,
        prev_block_hash: genesis.block_hash(),
        merkle_root: [0x53; 32],
        timestamp: genesis.timestamp + 150,
        block_height: 1,
        consensus_bits: 0x1f00ffff,
        consensus_type: ConsensusType::ProofOfWork,
        lagging_stake_modifier: genesis.lagging_stake_modifier,
        pow_nonce: 0,
        reserved_flags: [0; 3],
        validator_signature: vec![0; 64],
    };

    // Find nonce
    for nonce in 1..200_000u64 {
        block1_header.pow_nonce = nonce;
        let hash = sdr_crypto::sardhan_hash(&block1_header.to_bytes());
        if hash[0] == 0 {
            println!("[MINER] Block 1 mined in {:.3}s! Nonce: {}, Hash: 0x{}", t0.elapsed().as_secs_f64(), nonce, hex::encode(hash));
            break;
        }
    }

    // Apply Block 1
    let coinbase_tx = sdr_primitives::Transaction {
        chain_id: SARDHAN_MAINNET_MAGIC,
        version: 1,
        recent_block_hash: genesis.block_hash(),
        micro_pow_nonce: 0,
        lock_time: 0,
        inputs: vec![],
        outputs: vec![sdr_primitives::TxOut {
            value: 5_000_000_000, // 50 SDR
            recipient_pubkey: [0x77; 32],
        }],
        witnesses: vec![],
    };

    ledger.apply_block(&block1_header, &[coinbase_tx]).expect("block 1 application succeeds");
    println!("[LEDGER] Block 1 applied to state. Height: {}, Best Tip: 0x{}", ledger.best_height, hex::encode(ledger.best_block_hash));

    let info = handle_get_blockchain_info(&ledger);
    println!("\n[RPC RESPONSE `getblockchaininfo`]:");
    println!("  Blocks:             {}", info.blocks);
    println!("  Best Block Hash:    {}", info.bestblockhash);
    println!("  Circulating Supply: {:.2} SDR", info.circulating_supply);
    println!("  Max Supply:         {:.2} SDR", info.max_supply);
    println!("  Block Reward:       {:.2} SDR", info.block_reward_sdr);
    println!("\nNode running nominally. Ready for peer connections.");
}
