//! # Sardhan JSON-RPC 2.0 Interface
//!
//! Standard RPC methods:
//! - getblockchaininfo
//! - getblock
//! - getrawtransaction
//! - sendrawtransaction
//! - getmininginfo
//! - getstakinginfo

use sdr_consensus::{calculate_cumulative_supply, calculate_block_reward};
use sdr_ledger::Ledger;
use sdr_mempool::Mempool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainInfoResponse {
    pub chain: String,
    pub blocks: u64,
    pub bestblockhash: String,
    pub difficulty_pow: u32,
    pub difficulty_pos: u32,
    pub circulating_supply: f64,
    pub max_supply: f64,
    pub halving_epoch: u64,
    pub block_reward_sdr: f64,
}

pub fn handle_get_blockchain_info(ledger: &Ledger) -> BlockchainInfoResponse {
    let current_supply_sardhans = calculate_cumulative_supply(ledger.best_height);
    let reward_sardhans = calculate_block_reward(ledger.best_height);

    BlockchainInfoResponse {
        chain: "sdr-mainnet".to_string(),
        blocks: ledger.best_height,
        bestblockhash: hex::encode(ledger.best_block_hash),
        difficulty_pow: 0x1e0ffff0,
        difficulty_pos: 0x1e0ffff0,
        circulating_supply: (current_supply_sardhans as f64) / 100_000_000.0,
        max_supply: 21_000_000.0,
        halving_epoch: ledger.best_height / 210_000,
        block_reward_sdr: (reward_sardhans as f64) / 100_000_000.0,
    }
}
