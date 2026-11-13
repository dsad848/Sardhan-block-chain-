//! # Sardhan JSON-RPC 2.0 HTTP Server & Handlers
//!
//! Standard RPC methods:
//! - getblockchaininfo, getblock, getlatestblocks
//! - getrawtransaction, sendrawtransaction
//! - getaddressinfo
//! - getmempoolinfo, getrawmempool
//! - getmininginfo, getblocktemplate, submitblock
//! - getstakinginfo
//! - getpeerinfo, getnetworkinfo, getnodehealth
//! - addnode, disconnectnode

use sdr_consensus::{
    calculate_block_reward, calculate_block_weight_u128, calculate_cumulative_supply,
    compact_to_target, MAX_TOTAL_SUPPLY_SARDHANS,
};
use sdr_crypto::{decode_address_for_hrp, encode_address_for_hrp};
use sdr_ledger::{current_unix_timestamp, Ledger};
use sdr_mempool::{Mempool, RECENT_BLOCK_WINDOW};
use sdr_p2p::P2pManager;
use sdr_primitives::{
    compute_merkle_root, Block, BlockHeader, ConsensusType, Transaction, TxOut, MAX_BLOCK_BYTES,
    MAINNET_ENABLED,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

const MAX_RPC_BODY_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainInfoResponse {
    pub chain: String,
    pub chain_id: u32,
    pub mainnet_enabled: bool,
    pub blocks: u64,
    pub bestblockhash: String,
    pub difficulty_pow: u32,
    pub difficulty_pos: u32,
    pub difficulty_pow_hex: String,
    pub difficulty_pos_hex: String,
    pub network_hashrate_hps: f64,
    pub circulating_supply_sardhans: u64,
    pub circulating_supply: f64,
    pub max_supply_sardhans: u64,
    pub max_supply: f64,
    pub halving_epoch: u64,
    pub block_reward_sardhans: u64,
    pub block_reward_sdr: f64,
    pub cumulative_weight: String,
    pub pow_blocks_64: usize,
    pub pos_blocks_64: usize,
    pub mempool_txs: usize,
    pub utxo_count: usize,
    pub slashed_validators: usize,
}

#[derive(Clone)]
pub struct RpcContext {
    pub ledger: Arc<RwLock<Ledger>>,
    pub mempool: Arc<RwLock<Mempool>>,
    pub p2p: Arc<P2pManager>,
    pub started_at: u64,
}

pub fn handle_get_blockchain_info(ledger: &Ledger) -> BlockchainInfoResponse {
    let current_supply_sardhans = calculate_cumulative_supply(ledger.best_height);
    let next_height = ledger.best_height + 1;
    let reward_sardhans = calculate_block_reward(next_height);
    let pow_bits = ledger.expected_next_bits(ledger.best_block_hash, ConsensusType::ProofOfWork);
    let pos_bits = ledger.expected_next_bits(ledger.best_block_hash, ConsensusType::ProofOfStake);
    let (pow_64, pos_64) = ledger.channel_counts_recent_64();

    // Honest estimate of network hashrate based on actual recent PoW block timestamps & target
    let pow_blocks: Vec<_> = ledger
        .get_latest_blocks(12)
        .into_iter()
        .filter(|n| n.block.header.block_height > 0 && n.block.header.consensus_type == ConsensusType::ProofOfWork)
        .collect();
    let est_hps = if pow_blocks.len() >= 2 {
        let newest = pow_blocks[0].block.header.timestamp;
        let oldest = pow_blocks[pow_blocks.len() - 1].block.header.timestamp;
        let dt = newest.saturating_sub(oldest).max(1) as f64;
        let avg_dt = dt / ((pow_blocks.len() - 1) as f64);
        let work = calculate_block_weight_u128(pow_bits) as f64;
        work / avg_dt.max(1.0)
    } else {
        0.0
    };

    BlockchainInfoResponse {
        chain: ledger.config.name.to_string(),
        chain_id: ledger.config.chain_id,
        mainnet_enabled: MAINNET_ENABLED,
        blocks: ledger.best_height,
        bestblockhash: hex::encode(ledger.best_block_hash),
        difficulty_pow: pow_bits,
        difficulty_pos: pos_bits,
        difficulty_pow_hex: format!("0x{pow_bits:08x}"),
        difficulty_pos_hex: format!("0x{pos_bits:08x}"),
        network_hashrate_hps: est_hps,
        circulating_supply_sardhans: current_supply_sardhans,
        circulating_supply: (current_supply_sardhans as f64) / 100_000_000.0,
        max_supply_sardhans: MAX_TOTAL_SUPPLY_SARDHANS,
        max_supply: 21_000_000.0,
        halving_epoch: ledger.best_height / 210_000,
        block_reward_sardhans: reward_sardhans,
        block_reward_sdr: (reward_sardhans as f64) / 100_000_000.0,
        cumulative_weight: ledger.cumulative_weight.to_string(),
        pow_blocks_64: pow_64,
        pos_blocks_64: pos_64,
        mempool_txs: 0,
        utxo_count: ledger.utxos.len(),
        slashed_validators: ledger.slashed_validators.len(),
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(default)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

/// Start async HTTP/1.1 JSON-RPC 2.0 server on `bind_addr`
pub async fn start_rpc_server(
    bind_addr: SocketAddr,
    ctx: RpcContext,
) -> std::io::Result<SocketAddr> {
    let listener = TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let c = ctx.clone();
                    tokio::spawn(async move {
                        let _ = handle_http_connection(stream, c).await;
                    });
                }
                Err(e) => {
                    log::warn!("RPC accept error: {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
        }
    });

    Ok(local_addr)
}

async fn handle_http_connection(mut stream: TcpStream, ctx: RpcContext) -> std::io::Result<()> {
    let mut buffer = Vec::with_capacity(4096);
    let mut temp = [0u8; 2048];
    let header_end;

    loop {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&temp[..n]);
        if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        if buffer.len() > 65_536 {
            write_http_response(&mut stream, 400, "{\"error\":\"Headers too large\"}").await?;
            return Ok(());
        }
    }

    let header_str = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = header_str.lines();
    let request_line = lines.next().unwrap_or("");
    let method_http = request_line.split_whitespace().next().unwrap_or("").to_string();

    if method_http == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    let mut content_length: usize = 0;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse::<usize>().unwrap_or(0);
        }
    }

    if content_length > MAX_RPC_BODY_BYTES {
        write_http_response(&mut stream, 413, "{\"error\":\"Payload too large\"}").await?;
        return Ok(());
    }

    while buffer.len() < header_end + content_length {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..n]);
    }

    if method_http == "GET" {
        let info = {
            let l = ctx.ledger.read().await;
            let m = ctx.mempool.read().await;
            let mut r = handle_get_blockchain_info(&l);
            r.mempool_txs = m.len();
            r
        };
        let body = serde_json::to_string(&info).unwrap_or_else(|_| "{}".into());
        write_http_response(&mut stream, 200, &body).await?;
        return Ok(());
    }

    let body_slice = &buffer[header_end..(header_end + content_length).min(buffer.len())];
    let resp_json = match serde_json::from_slice::<JsonRpcRequest>(body_slice) {
        Ok(req) => execute_rpc_request(req, &ctx).await,
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": { "code": -32700, "message": format!("Parse error: {e}") }
        }),
    };

    let out = serde_json::to_string(&resp_json).unwrap_or_else(|_| "{}".into());
    write_http_response(&mut stream, 200, &out).await
}

async fn write_http_response(stream: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        status_text,
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.flush().await
}

pub async fn execute_rpc_method(method: &str, params: Value, ctx: &RpcContext) -> Value {
    let req = JsonRpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(1)),
        method: method.to_string(),
        params,
    };
    execute_rpc_request(req, ctx).await
}

async fn execute_rpc_request(req: JsonRpcRequest, ctx: &RpcContext) -> Value {
    let _ = &req.jsonrpc;
    let id = req.id.clone().unwrap_or(Value::Null);
    let result = dispatch_method(&req.method, req.params, ctx).await;
    match result {
        Ok(val) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": val
        }),
        Err((code, msg)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": msg }
        }),
    }
}

fn serialize_tx_json(tx: &Transaction, hrp: &str, height: Option<u64>, block_hash: Option<[u8; 32]>, is_reward: bool) -> Value {
    let txid = hex::encode(tx.txid());
    let total_out: u64 = tx.outputs.iter().map(|o| o.value).sum();
    let inputs: Vec<Value> = tx
        .inputs
        .iter()
        .map(|i| {
            json!({
                "txid": hex::encode(i.previous_output.txid),
                "vout": i.previous_output.vout,
                "sequence": i.sequence
            })
        })
        .collect();
    let outputs: Vec<Value> = tx
        .outputs
        .iter()
        .enumerate()
        .map(|(vout, o)| {
            let addr = encode_address_for_hrp(hrp, &o.recipient_pubkey).unwrap_or_default();
            json!({
                "vout": vout,
                "value_sardhans": o.value,
                "value_sdr": (o.value as f64) / 100_000_000.0,
                "address": addr,
                "recipient_pubkey": hex::encode(o.recipient_pubkey)
            })
        })
        .collect();

    json!({
        "txid": txid,
        "chain_id": tx.chain_id,
        "version": tx.version,
        "recent_block_hash": hex::encode(tx.recent_block_hash),
        "micro_pow_nonce": tx.micro_pow_nonce,
        "lock_time": tx.lock_time,
        "byte_size": tx.byte_size(),
        "is_reward_tx": is_reward,
        "fee_sardhans": 0,
        "total_output_sardhans": total_out,
        "total_output_sdr": (total_out as f64) / 100_000_000.0,
        "inputs": inputs,
        "outputs": outputs,
        "block_height": height,
        "block_hash": block_hash.map(hex::encode),
        "confirmed": height.is_some()
    })
}

fn serialize_block_json(node: &sdr_ledger::BlockNode, hrp: &str) -> Value {
    let h = &node.block.header;
    let ctype = match h.consensus_type {
        ConsensusType::ProofOfWork => "PoW",
        ConsensusType::ProofOfStake => "PoS",
    };
    let txs: Vec<Value> = node
        .block
        .transactions
        .iter()
        .enumerate()
        .map(|(idx, tx)| serialize_tx_json(tx, hrp, Some(h.block_height), Some(node.block_hash), idx == 0))
        .collect();

    json!({
        "hash": hex::encode(node.block_hash),
        "height": h.block_height,
        "version": h.version,
        "prev_block_hash": hex::encode(h.prev_block_hash),
        "merkle_root": hex::encode(h.merkle_root),
        "timestamp": h.timestamp,
        "consensus_bits": h.consensus_bits,
        "consensus_bits_hex": format!("0x{:08x}", h.consensus_bits),
        "consensus_type": ctype,
        "lagging_stake_modifier": hex::encode(h.lagging_stake_modifier),
        "pow_nonce": h.pow_nonce,
        "cumulative_weight": node.cumulative_weight.to_string(),
        "on_main_chain": node.on_main_chain,
        "tx_count": node.block.transactions.len(),
        "size_bytes": node.block.to_bytes().len(),
        "transactions": txs,
    })
}

async fn dispatch_method(method: &str, params: Value, ctx: &RpcContext) -> Result<Value, (i32, String)> {
    match method {
        "getblockchaininfo" => {
            let l = ctx.ledger.read().await;
            let m = ctx.mempool.read().await;
            let mut info = handle_get_blockchain_info(&l);
            info.mempool_txs = m.len();
            Ok(serde_json::to_value(info).unwrap())
        }
        "getblock" => {
            let param = params
                .as_array()
                .and_then(|a| a.first())
                .ok_or((-32602, "Missing block height or hash parameter".to_string()))?;
            let l = ctx.ledger.read().await;
            let node = if let Some(height) = param.as_u64() {
                l.get_block_by_height(height)
            } else if let Some(hash_str) = param.as_str() {
                let bytes = hex::decode(hash_str.trim_start_matches("0x"))
                    .map_err(|_| (-32602, "Invalid block hash hex".to_string()))?;
                if bytes.len() != 32 {
                    return Err((-32602, "Block hash must be 32 bytes".to_string()));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                l.get_block_by_hash(&arr)
            } else {
                return Err((-32602, "Expected block height or hash".to_string()));
            };
            let node = node.ok_or((-5, "Block not found".to_string()))?;
            Ok(serialize_block_json(node, l.config.address_hrp))
        }
        "getlatestblocks" => {
            let count = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_u64())
                .unwrap_or(10)
                .min(100) as usize;
            let l = ctx.ledger.read().await;
            let blocks: Vec<Value> = l
                .get_latest_blocks(count)
                .into_iter()
                .map(|n| serialize_block_json(n, l.config.address_hrp))
                .collect();
            Ok(json!(blocks))
        }
        "getrawtransaction" => {
            let txid_hex = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .ok_or((-32602, "Missing txid hex".to_string()))?;
            let bytes = hex::decode(txid_hex.trim_start_matches("0x"))
                .map_err(|_| (-32602, "Invalid txid hex".to_string()))?;
            if bytes.len() != 32 {
                return Err((-32602, "Txid must be 32 bytes".to_string()));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);

            let l = ctx.ledger.read().await;
            if let Some(rec) = l.tx_index.get(&arr) {
                return Ok(serialize_tx_json(
                    &rec.tx,
                    l.config.address_hrp,
                    Some(rec.block_height),
                    Some(rec.block_hash),
                    rec.is_reward_tx,
                ));
            }
            let m = ctx.mempool.read().await;
            if let Some(entry) = m.entries.get(&arr) {
                return Ok(serialize_tx_json(
                    &entry.tx,
                    l.config.address_hrp,
                    None,
                    None,
                    false,
                ));
            }
            Err((-5, "Transaction not found".to_string()))
        }
        "sendrawtransaction" => {
            let raw_hex = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .ok_or((-32602, "Missing raw_tx_hex".to_string()))?;
            let raw_bytes = hex::decode(raw_hex.trim_start_matches("0x"))
                .map_err(|_| (-32602, "Invalid raw_tx_hex".to_string()))?;
            let tx = Transaction::from_bytes(&raw_bytes)
                .map_err(|e| (-22, format!("Transaction decode failed: {e}")))?;

            let now = current_unix_timestamp();
            let txid = {
                let l = ctx.ledger.read().await;
                let mut m = ctx.mempool.write().await;
                m.insert_transaction_with_ledger(tx.clone(), &l, None, now)
                    .map_err(|e| (-26, format!("Mempool rejected transaction: {e}")))?
            };
            ctx.p2p.broadcast_tx(&tx).await;
            Ok(json!({
                "txid": hex::encode(txid),
                "accepted": true
            }))
        }
        "getaddressinfo" => {
            let addr = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .ok_or((-32602, "Missing address parameter".to_string()))?;
            let l = ctx.ledger.read().await;
            let pubkey = decode_address_for_hrp(addr, l.config.address_hrp)
                .map_err(|e| (-5, format!("Invalid address for network {}: {e}", l.config.name)))?;
            let total_sardhans = l.get_balance(&pubkey);
            let spendable_sardhans = l.get_spendable_balance(&pubkey);
            let staking_utxos = l.get_staking_utxos(&pubkey, l.best_height);
            let mature_staking_sardhans: u64 = staking_utxos.iter().map(|(_, v)| *v).sum();

            let utxos: Vec<Value> = l
                .get_address_utxos(&pubkey)
                .into_iter()
                .map(|(op, entry)| {
                    json!({
                        "txid": hex::encode(op.txid),
                        "vout": op.vout,
                        "value_sardhans": entry.output.value,
                        "value_sdr": (entry.output.value as f64) / 100_000_000.0,
                        "block_height": entry.block_height,
                        "is_coinstake": entry.is_coinstake
                    })
                })
                .collect();
            let history = l.get_address_history(&pubkey);

            Ok(json!({
                "address": addr,
                "network": l.config.name,
                "pubkey_hex": hex::encode(pubkey),
                "balance_sardhans": total_sardhans,
                "balance_sdr": (total_sardhans as f64) / 100_000_000.0,
                "spendable_sardhans": spendable_sardhans,
                "spendable_sdr": (spendable_sardhans as f64) / 100_000_000.0,
                "mature_staking_sardhans": mature_staking_sardhans,
                "utxos": utxos,
                "history": history
            }))
        }
        "getmempoolinfo" => {
            let m = ctx.mempool.read().await;
            let target = m.current_micro_pow_target();
            Ok(json!({
                "size": m.len(),
                "bytes": m.total_bytes,
                "max_bytes": m.max_bytes,
                "micro_pow_target_hex": hex::encode(target),
                "recent_window_blocks": RECENT_BLOCK_WINDOW,
                "zero_fee": true
            }))
        }
        "getrawmempool" => {
            let l = ctx.ledger.read().await;
            let m = ctx.mempool.read().await;
            let list: Vec<Value> = m
                .entries
                .values()
                .map(|e| serialize_tx_json(&e.tx, l.config.address_hrp, None, None, false))
                .collect();
            Ok(json!(list))
        }
        "getmininginfo" => {
            let l = ctx.ledger.read().await;
            let m = ctx.mempool.read().await;
            let info = handle_get_blockchain_info(&l);
            Ok(json!({
                "blocks": l.best_height,
                "difficulty_pow": info.difficulty_pow,
                "difficulty_pow_hex": info.difficulty_pow_hex,
                "target_hex": hex::encode(compact_to_target(info.difficulty_pow)),
                "network_hashrate_hps": info.network_hashrate_hps,
                "pooled_tx": m.len(),
                "can_mine_pow_next": l.can_append_channel(ConsensusType::ProofOfWork),
                "chain": l.config.name,
                "micro_pow_target_hex": hex::encode(m.current_micro_pow_target()),
                "recent_block_hashes": l.recent_block_hashes(RECENT_BLOCK_WINDOW).into_iter().map(hex::encode).collect::<Vec<_>>()
            }))
        }
        "getblocktemplate" => {
            let addr_opt = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str().or_else(|| v.get("coinbase_address").and_then(|c| c.as_str())));
            let l = ctx.ledger.read().await;
            let m = ctx.mempool.read().await;

            if !l.can_append_channel(ConsensusType::ProofOfWork) {
                return Err((-10, "PoW channel window currently saturated; awaiting PoS block".into()));
            }

            let miner_pubkey = if let Some(addr) = addr_opt {
                decode_address_for_hrp(addr, l.config.address_hrp)
                    .map_err(|e| (-5, format!("Invalid miner address: {e}")))?
            } else {
                [0x53; 32]
            };

            let next_height = l.best_height + 1;
            let reward = calculate_block_reward(next_height);
            let mempool_txs = m.get_block_template(MAX_BLOCK_BYTES - 1024);

            let coinbase_tx = Transaction {
                chain_id: l.config.chain_id,
                version: 1,
                recent_block_hash: l.best_block_hash,
                micro_pow_nonce: 0,
                lock_time: 0,
                inputs: vec![],
                outputs: vec![TxOut {
                    value: reward,
                    recipient_pubkey: miner_pubkey,
                }],
                witnesses: vec![],
            };

            let mut txs = Vec::with_capacity(1 + mempool_txs.len());
            txs.push(coinbase_tx);
            txs.extend(mempool_txs);

            let txids: Vec<[u8; 32]> = txs.iter().map(|t| t.txid()).collect();
            let merkle_root = compute_merkle_root(&txids);
            let bits = l.expected_next_bits(l.best_block_hash, ConsensusType::ProofOfWork);
            let lsm = l.expected_lagging_modifier(l.best_block_hash, next_height);
            let mtp = l.median_time_past_for_parent(l.best_block_hash);
            let ts = current_unix_timestamp().max(mtp + 1);

            let header = BlockHeader {
                version: 1,
                prev_block_hash: l.best_block_hash,
                merkle_root,
                timestamp: ts,
                block_height: next_height,
                consensus_bits: bits,
                consensus_type: ConsensusType::ProofOfWork,
                lagging_stake_modifier: lsm,
                pow_nonce: 0,
                reserved_flags: [0; 3],
                validator_signature: vec![0; 64],
            };
            let template_block = Block {
                header: header.clone(),
                transactions: txs,
            };

            Ok(json!({
                "height": next_height,
                "bits": bits,
                "target_hex": hex::encode(compact_to_target(bits)),
                "reward_sardhans": reward,
                "tx_count": template_block.transactions.len(),
                "block_hex": hex::encode(template_block.to_bytes()),
                "header_hex": hex::encode(header.to_bytes())
            }))
        }
        "submitblock" => {
            let raw_hex = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .ok_or((-32602, "Missing block_hex".to_string()))?;
            let raw = hex::decode(raw_hex.trim_start_matches("0x"))
                .map_err(|_| (-32602, "Invalid block_hex".to_string()))?;
            let block = Block::from_bytes(&raw)
                .map_err(|e| (-22, format!("Block decode failed: {e}")))?;
            let bhash = block.block_hash();
            let height = block.header.block_height;

            let accepted_tip = {
                let mut l = ctx.ledger.write().await;
                l.submit_block_at(block.clone(), current_unix_timestamp())
                    .map_err(|e| (-25, format!("Block rejected: {e}")))?
            };
            {
                let l = ctx.ledger.read().await;
                let mut m = ctx.mempool.write().await;
                m.prune_against_ledger(&l);
            }
            ctx.p2p.broadcast_block(&block).await;

            Ok(json!({
                "accepted": true,
                "became_best_tip": accepted_tip,
                "height": height,
                "hash": hex::encode(bhash)
            }))
        }
        "getstakinginfo" => {
            let l = ctx.ledger.read().await;
            let pos_bits = l.expected_next_bits(l.best_block_hash, ConsensusType::ProofOfStake);
            let (pow_64, pos_64) = l.channel_counts_recent_64();
            Ok(json!({
                "blocks": l.best_height,
                "difficulty_pos": pos_bits,
                "difficulty_pos_hex": format!("0x{pos_bits:08x}"),
                "pos_target_hex": hex::encode(compact_to_target(pos_bits)),
                "min_stake_confirmation_age": l.config.min_stake_confirmation_age,
                "coinstake_maturity_lock_blocks": l.config.coinstake_maturity_lock_blocks,
                "can_stake_pos_next": l.can_append_channel(ConsensusType::ProofOfStake),
                "pos_blocks_64": pos_64,
                "pow_blocks_64": pow_64,
                "slashed_validators": l.slashed_validators.len()
            }))
        }
        "getpeerinfo" => {
            let peers = ctx.p2p.get_peer_snapshots().await;
            Ok(json!(peers))
        }
        "getnetworkinfo" => {
            let peers = ctx.p2p.get_peer_snapshots().await;
            let known = ctx.p2p.known_peer_count().await;
            let l = ctx.ledger.read().await;
            Ok(json!({
                "network": l.config.name,
                "chain_id": l.config.chain_id,
                "magic_hex": format!("0x{:08x}", l.config.magic),
                "address_hrp": l.config.address_hrp,
                "p2p_listen": ctx.p2p.listen_addr.to_string(),
                "connections": peers.len(),
                "known_peers": known,
                "protocol_version": 2,
                "subversion": "/SardhanCore:0.2.0/"
            }))
        }
        "getnodehealth" => {
            let l = ctx.ledger.read().await;
            let m = ctx.mempool.read().await;
            let peers = ctx.p2p.peer_count().await;
            let now = current_unix_timestamp();
            Ok(json!({
                "status": if peers > 0 { "healthy" } else { "isolated" },
                "uptime_secs": now.saturating_sub(ctx.started_at),
                "best_height": l.best_height,
                "best_block_hash": hex::encode(l.best_block_hash),
                "peer_count": peers,
                "mempool_txs": m.len(),
                "mempool_bytes": m.total_bytes,
                "utxo_count": l.utxos.len(),
                "mainnet_enabled": MAINNET_ENABLED,
                "chain": l.config.name
            }))
        }
        "addnode" => {
            let addr = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .ok_or((-32602, "Missing peer ip:port".to_string()))?;
            ctx.p2p
                .connect_peer(addr)
                .await
                .map_err(|e| (-1, format!("Failed to connect peer {addr}: {e}")))?;
            Ok(json!({ "connected_initiated": addr }))
        }
        "disconnectnode" => {
            let target = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .unwrap_or("all");
            if target == "all" {
                let count = ctx.p2p.disconnect_all().await;
                Ok(json!({ "disconnected_count": count }))
            } else {
                let ok = ctx.p2p.disconnect_peer(target).await;
                Ok(json!({ "disconnected": ok, "target": target }))
            }
        }
        _ => Err((-32601, format!("Method '{method}' not found"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdr_primitives::{Network, SARDHAN_REGTEST_MAGIC};

    #[tokio::test]
    async fn test_rpc_server_end_to_end() {
        let ledger = Arc::new(RwLock::new(Ledger::new_for_network(Network::Regtest)));
        let mempool = Arc::new(RwLock::new(Mempool::new(SARDHAN_REGTEST_MAGIC)));
        let p2p = P2pManager::new(
            SARDHAN_REGTEST_MAGIC,
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&ledger),
            Arc::clone(&mempool),
            None,
        );
        let ctx = RpcContext {
            ledger,
            mempool,
            p2p,
            started_at: current_unix_timestamp(),
        };

        let addr = start_rpc_server("127.0.0.1:0".parse().unwrap(), ctx)
            .await
            .expect("rpc server binds");

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let body = r#"{"jsonrpc":"2.0","id":7,"method":"getblockchaininfo","params":[]}"#;
        let req = format!(
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(resp.contains("\"chain\":\"sdr-regtest\""));
        assert!(resp.contains("\"mainnet_enabled\":false"));
    }
}
