//! # Sardhan Full Node Daemon (`sdrd`)
//!
//! Complete Layer-1 full node orchestrating:
//! - Persistent UTXO & Block Storage (`blocks.bin` + `peers.json` + `slashing.json`)
//! - Multi-peer TCP P2P Mesh & Partition Recovery (`sdr-p2p`)
//! - HTTP/1.1 JSON-RPC 2.0 Server (`sdr-rpc`)
//! - Optional Multi-Threaded SardhanHash CPU Miner & PoS Coinstake Validator
//! - Mainnet Security Release Gate (`MAINNET_ENABLED = false`)

use sdr_consensus::{
    calculate_block_reward, compact_to_target, evaluate_pos_kernel, hash_less_than_target,
};
use sdr_crypto::{decode_address_for_hrp, encode_address_for_hrp, sardhan_hash, Keypair};
use sdr_ledger::{current_unix_timestamp, Ledger};
use sdr_mempool::Mempool;
use sdr_p2p::P2pManager;
use sdr_primitives::{
    compute_merkle_root, Block, BlockHeader, ConsensusType, Network, Transaction, TxIn, TxOut,
    TxWitness, MAINNET_ENABLED, MAX_BLOCK_BYTES,
};
use sdr_rpc::{handle_get_blockchain_info, start_rpc_server, RpcContext};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

struct NodeArgs {
    network: Network,
    datadir: PathBuf,
    bind_ip: String,
    p2p_port: Option<u16>,
    rpc_port: Option<u16>,
    addnodes: Vec<String>,
    mine: bool,
    mine_threads: usize,
    mine_address: Option<String>,
    mine_interval_ms: u64,
    stake: bool,
    stake_key_hex: Option<String>,
    unlock_mainnet_rc: bool,
}

fn parse_args() -> NodeArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut network = Network::Testnet;
    let mut datadir = PathBuf::from("./sdr-data");
    let mut bind_ip = "127.0.0.1".to_string();
    let mut p2p_port = None;
    let mut rpc_port = None;
    let mut addnodes = Vec::new();
    let mut mine = false;
    let mut mine_threads = 1usize;
    let mut mine_address = None;
    let mut mine_interval_ms = 250u64;
    let mut stake = false;
    let mut stake_key_hex = None;
    let mut unlock_mainnet_rc = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--network" if i + 1 < args.len() => {
                network = Network::from_str_name(&args[i + 1]).unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                i += 2;
            }
            "--datadir" if i + 1 < args.len() => {
                datadir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--bind" if i + 1 < args.len() => {
                bind_ip = args[i + 1].clone();
                i += 2;
            }
            "--p2p-port" if i + 1 < args.len() => {
                p2p_port = Some(args[i + 1].parse().expect("valid p2p-port"));
                i += 2;
            }
            "--rpc-port" if i + 1 < args.len() => {
                rpc_port = Some(args[i + 1].parse().expect("valid rpc-port"));
                i += 2;
            }
            "--addnode" if i + 1 < args.len() => {
                addnodes.push(args[i + 1].clone());
                i += 2;
            }
            "--mine" => {
                mine = true;
                i += 1;
            }
            "--mine-threads" if i + 1 < args.len() => {
                mine_threads = args[i + 1].parse().unwrap_or(1).max(1);
                i += 2;
            }
            "--mine-address" if i + 1 < args.len() => {
                mine_address = Some(args[i + 1].clone());
                i += 2;
            }
            "--mine-interval-ms" if i + 1 < args.len() => {
                mine_interval_ms = args[i + 1].parse().unwrap_or(250);
                i += 2;
            }
            "--stake" => {
                stake = true;
                i += 1;
            }
            "--stake-key" if i + 1 < args.len() => {
                stake_key_hex = Some(args[i + 1].clone());
                i += 2;
            }
            "--unlock-mainnet-rc" => {
                unlock_mainnet_rc = true;
                i += 1;
            }
            "--help" | "-h" => {
                println!(
                    "Sardhan Full Node Daemon (sdrd)
Usage: sdrd [OPTIONS]
  --network <testnet|mainnet|regtest>   Chain network (default: testnet)
  --datadir <path>                      Persistent data directory (default: ./sdr-data)
  --bind <ip>                           Bind IP address (default: 127.0.0.1)
  --p2p-port <port>                     TCP P2P port
  --rpc-port <port>                     HTTP JSON-RPC 2.0 port
  --addnode <ip:port>                   Connect to peer (repeatable)
  --mine                                Enable built-in PoW CPU mining
  --mine-threads <n>                    PoW mining thread count (default: 1)
  --mine-address <addr>                 Coinbase reward address
  --mine-interval-ms <ms>               Cooldown between mined blocks (default: 250)
  --stake                               Enable built-in PoS validator staking
  --stake-key <hex>                     32-byte Ed25519 staking secret key hex"
                );
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown argument: {other}");
                std::process::exit(1);
            }
        }
    }

    NodeArgs {
        network,
        datadir,
        bind_ip,
        p2p_port,
        rpc_port,
        addnodes,
        mine,
        mine_threads,
        mine_address,
        mine_interval_ms,
        stake,
        stake_key_hex,
        unlock_mainnet_rc,
    }
}

fn load_or_create_node_key(datadir: &PathBuf) -> Keypair {
    let key_path = datadir.join("node_identity.key");
    if let Ok(hex_str) = std::fs::read_to_string(&key_path) {
        if let Ok(bytes) = hex::decode(hex_str.trim()) {
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                return Keypair::from_secret_bytes(&arr);
            }
        }
    }
    let kp = Keypair::generate();
    let _ = std::fs::write(&key_path, hex::encode(kp.secret_bytes()));
    kp
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let opts = parse_args();
    let config = opts.network.config();

    if opts.network == Network::Mainnet && !MAINNET_ENABLED && !opts.unlock_mainnet_rc {
        eprintln!("============================================================");
        eprintln!("  MAINNET ACTIVATION BLOCKED BY SECURITY RELEASE GATE       ");
        eprintln!("============================================================");
        eprintln!("MAINNET_ENABLED is currently `false` (Section 12 Gate).");
        eprintln!("Mainnet remains disabled until independent external audit,");
        eprintln!("GPU/ASIC resistance benchmarks, and release sign-off complete.");
        eprintln!("Use `--network testnet` for live 3-node network validation.");
        std::process::exit(2);
    }

    let p2p_port = opts.p2p_port.unwrap_or(config.default_p2p_port);
    let rpc_port = opts.rpc_port.unwrap_or(config.default_rpc_port);

    let ledger = Ledger::open(&opts.datadir, opts.network).unwrap_or_else(|e| {
        eprintln!("Fatal: failed opening ledger at {:?}: {e}", opts.datadir);
        std::process::exit(1);
    });
    let mempool = Mempool::with_config(config.chain_id, config.micro_pow_base_bits, sdr_mempool::MAX_MEMPOOL_BYTES);

    let node_key = if let Some(ref hex_sec) = opts.stake_key_hex {
        let bytes = hex::decode(hex_sec.trim()).expect("valid 64-char hex secret key");
        let arr: [u8; 32] = bytes.try_into().expect("32-byte secret key");
        Keypair::from_secret_bytes(&arr)
    } else {
        load_or_create_node_key(&opts.datadir)
    };
    let node_addr = encode_address_for_hrp(config.address_hrp, &node_key.public_key_bytes())
        .expect("valid address");

    let miner_pubkey = if let Some(ref addr) = opts.mine_address {
        decode_address_for_hrp(addr, config.address_hrp).unwrap_or_else(|e| {
            eprintln!("Invalid --mine-address: {e}");
            std::process::exit(1);
        })
    } else {
        node_key.public_key_bytes()
    };

    let info_init = handle_get_blockchain_info(&ledger);
    println!("============================================================");
    println!("          SARDHAN CHAIN FULL NODE DAEMON (`sdrd`)           ");
    println!("============================================================");
    println!("Network:          {} (Magic: 0x{:08x})", config.name, config.magic);
    println!("Mainnet Gate:     MAINNET_ENABLED = {}", MAINNET_ENABLED);
    println!("Data Directory:   {:?}", opts.datadir);
    println!("Node Address:     {}", node_addr);
    println!("Best Height:      #{}", info_init.blocks);
    println!("Best Block Hash:  0x{}", info_init.bestblockhash);
    println!("UTXO Count:       {}", ledger.utxos.len());
    println!("Consensus:        Hybrid PoW (SardhanHash) + PoS (Proof of Power)");
    println!("Zero Fees:        0.00 SDR (Block-Bound Micro-PoW Protected)");
    println!("============================================================");

    let ledger_arc = Arc::new(RwLock::new(ledger));
    let mempool_arc = Arc::new(RwLock::new(mempool));

    let p2p_bind: SocketAddr = format!("{}:{}", opts.bind_ip, p2p_port)
        .parse()
        .expect("valid p2p bind addr");
    let rpc_bind: SocketAddr = format!("{}:{}", opts.bind_ip, rpc_port)
        .parse()
        .expect("valid rpc bind addr");

    let p2p = P2pManager::new(
        config.magic,
        p2p_bind,
        Arc::clone(&ledger_arc),
        Arc::clone(&mempool_arc),
        Some(opts.datadir.clone()),
    );
    let actual_p2p = p2p.start().await.unwrap_or_else(|e| {
        eprintln!("Fatal: failed binding P2P on {p2p_bind}: {e}");
        std::process::exit(1);
    });
    println!("[P2P] Listening on tcp://{}", actual_p2p);

    let rpc_ctx = RpcContext {
        ledger: Arc::clone(&ledger_arc),
        mempool: Arc::clone(&mempool_arc),
        p2p: Arc::clone(&p2p),
        started_at: current_unix_timestamp(),
    };
    let actual_rpc = start_rpc_server(rpc_bind, rpc_ctx).await.unwrap_or_else(|e| {
        eprintln!("Fatal: failed binding JSON-RPC on {rpc_bind}: {e}");
        std::process::exit(1);
    });
    println!("[RPC] Listening on http://{}", actual_rpc);

    for peer in opts.addnodes {
        let p2p_c = Arc::clone(&p2p);
        tokio::spawn(async move {
            for attempt in 0..5 {
                if p2p_c.connect_peer(&peer).await.is_ok() {
                    println!("[P2P] Connected to seed peer {}", peer);
                    return;
                }
                sleep(Duration::from_millis(400 * (attempt + 1))).await;
            }
        });
    }

    // Optional built-in PoW miner loop
    if opts.mine {
        let m_ledger = Arc::clone(&ledger_arc);
        let m_mempool = Arc::clone(&mempool_arc);
        let m_p2p = Arc::clone(&p2p);
        let threads = opts.mine_threads;
        let interval_ms = opts.mine_interval_ms;

        tokio::spawn(async move {
            println!("[MINER] Built-in CPU PoW miner active ({threads} thread(s))");
            loop {
                sleep(Duration::from_millis(interval_ms)).await;

                let template_opt = {
                    let l = m_ledger.read().await;
                    let m = m_mempool.read().await;

                    if !l.can_append_channel(ConsensusType::ProofOfWork) {
                        None
                    } else {
                        let next_h = l.best_height + 1;
                        let reward = calculate_block_reward(next_h);
                        let mempool_txs = m.get_block_template(MAX_BLOCK_BYTES - 1024);
                        let coinbase = Transaction {
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
                        txs.push(coinbase);
                        txs.extend(mempool_txs);

                        let txids: Vec<[u8; 32]> = txs.iter().map(|t| t.txid()).collect();
                        let bits = l.expected_next_bits(l.best_block_hash, ConsensusType::ProofOfWork);
                        let lsm = l.expected_lagging_modifier(l.best_block_hash, next_h);
                        let mtp = l.median_time_past_for_parent(l.best_block_hash);
                        let ts = current_unix_timestamp().max(mtp + 1);

                        let hdr = BlockHeader {
                            version: 1,
                            prev_block_hash: l.best_block_hash,
                            merkle_root: compute_merkle_root(&txids),
                            timestamp: ts,
                            block_height: next_h,
                            consensus_bits: bits,
                            consensus_type: ConsensusType::ProofOfWork,
                            lagging_stake_modifier: lsm,
                            pow_nonce: 0,
                            reserved_flags: [0; 3],
                            validator_signature: vec![0; 64],
                        };
                        Some((hdr, txs, l.best_block_hash))
                    }
                };

                let Some((hdr, txs, tip_snapshot)) = template_opt else {
                    sleep(Duration::from_millis(500)).await;
                    continue;
                };

                // Run CPU SardhanHash search on blocking threadpool
                let found_header = tokio::task::spawn_blocking(move || {
                    let target = compact_to_target(hdr.consensus_bits);
                    let stop = Arc::new(AtomicBool::new(false));
                    let mut handles = Vec::with_capacity(threads);
                    for tid in 0..threads {
                        let mut local_hdr = hdr.clone();
                        let stop_c = Arc::clone(&stop);
                        handles.push(std::thread::spawn(move || -> Option<BlockHeader> {
                            let mut nonce = (tid as u64) + 1;
                            for _ in 0..50_000 {
                                if stop_c.load(Ordering::Relaxed) {
                                    return None;
                                }
                                local_hdr.pow_nonce = nonce;
                                let h = sardhan_hash(&local_hdr.to_bytes());
                                if hash_less_than_target(&h, &target) {
                                    stop_c.store(true, Ordering::Relaxed);
                                    return Some(local_hdr);
                                }
                                nonce += threads as u64;
                            }
                            None
                        }));
                    }
                    let mut winner = None;
                    for h in handles {
                        if let Ok(Some(res)) = h.join() {
                            if winner.is_none() {
                                winner = Some(res);
                            }
                        }
                    }
                    winner
                })
                .await
                .ok()
                .flatten();

                if let Some(winning_hdr) = found_header {
                    // Check chain didn't advance while hashing
                    let current_tip = m_ledger.read().await.best_block_hash;
                    if current_tip != tip_snapshot {
                        continue;
                    }
                    let block = Block {
                        header: winning_hdr.clone(),
                        transactions: txs,
                    };
                    let apply_ok = {
                        let mut l = m_ledger.write().await;
                        l.submit_block_at(block.clone(), current_unix_timestamp())
                    };
                    if apply_ok.is_ok() {
                        {
                            let l = m_ledger.read().await;
                            let mut m = m_mempool.write().await;
                            m.prune_against_ledger(&l);
                        }
                        println!(
                            "[MINER] Mined PoW Block #{} (hash: 0x{}..., nonce: {})",
                            winning_hdr.block_height,
                            &hex::encode(block.block_hash())[..16],
                            winning_hdr.pow_nonce
                        );
                        m_p2p.broadcast_block(&block).await;
                    }
                }
            }
        });
    }

    // Optional built-in PoS staking loop
    if opts.stake {
        let s_ledger = Arc::clone(&ledger_arc);
        let s_mempool = Arc::clone(&mempool_arc);
        let s_p2p = Arc::clone(&p2p);
        let staker_kp = node_key.clone();

        tokio::spawn(async move {
            println!("[STAKER] Built-in PoS validator loop active");
            loop {
                sleep(Duration::from_millis(800)).await;

                let candidate_block = {
                    let l = s_ledger.read().await;
                    let m = s_mempool.read().await;
                    if !l.can_append_channel(ConsensusType::ProofOfStake) {
                        None
                    } else {
                        let pk = staker_kp.public_key_bytes();
                        let mature_utxos = l.get_staking_utxos(&pk, l.best_height);
                        if mature_utxos.is_empty() {
                            None
                        } else {
                            let next_h = l.best_height + 1;
                            let pos_bits =
                                l.expected_next_bits(l.best_block_hash, ConsensusType::ProofOfStake);
                            let lsm = l.expected_lagging_modifier(l.best_block_hash, next_h);
                            let mtp = l.median_time_past_for_parent(l.best_block_hash);
                            let ts = current_unix_timestamp().max(mtp + 1);

                            let mut hit = None;
                            for (outpoint, val) in mature_utxos {
                                if evaluate_pos_kernel(
                                    &lsm,
                                    &outpoint.txid,
                                    outpoint.vout,
                                    ts,
                                    val,
                                    pos_bits,
                                ) {
                                    hit = Some((outpoint, val));
                                    break;
                                }
                            }

                            if let Some((kernel_op, kernel_val)) = hit {
                                let reward = calculate_block_reward(next_h);
                                let mut coinstake = Transaction {
                                    chain_id: l.config.chain_id,
                                    version: 1,
                                    recent_block_hash: l.best_block_hash,
                                    micro_pow_nonce: 0,
                                    lock_time: 0,
                                    inputs: vec![TxIn {
                                        previous_output: kernel_op,
                                        sequence: 0xffffffff,
                                    }],
                                    outputs: vec![TxOut {
                                        value: kernel_val + reward,
                                        recipient_pubkey: pk,
                                    }],
                                    witnesses: vec![],
                                };
                                let digest = coinstake.signature_digest(0);
                                let sig = staker_kp.sign(&digest);
                                coinstake.witnesses.push(TxWitness {
                                    signature: sig.to_vec(),
                                    public_key: pk,
                                });

                                let mempool_txs = m.get_block_template(MAX_BLOCK_BYTES - 1024);
                                let mut txs = Vec::with_capacity(1 + mempool_txs.len());
                                txs.push(coinstake);
                                txs.extend(mempool_txs);

                                let txids: Vec<[u8; 32]> = txs.iter().map(|t| t.txid()).collect();
                                let mut hdr = BlockHeader {
                                    version: 1,
                                    prev_block_hash: l.best_block_hash,
                                    merkle_root: compute_merkle_root(&txids),
                                    timestamp: ts,
                                    block_height: next_h,
                                    consensus_bits: pos_bits,
                                    consensus_type: ConsensusType::ProofOfStake,
                                    lagging_stake_modifier: lsm,
                                    pow_nonce: 0,
                                    reserved_flags: [0; 3],
                                    validator_signature: vec![0; 64],
                                };
                                hdr.validator_signature =
                                    staker_kp.sign(&hdr.pos_signature_preimage()).to_vec();
                                Some(Block {
                                    header: hdr,
                                    transactions: txs,
                                })
                            } else {
                                None
                            }
                        }
                    }
                };

                if let Some(block) = candidate_block {
                    let h = block.header.block_height;
                    let bhash = block.block_hash();
                    let res = {
                        let mut l = s_ledger.write().await;
                        l.submit_block_at(block.clone(), current_unix_timestamp())
                    };
                    if res.is_ok() {
                        {
                            let l = s_ledger.read().await;
                            let mut m = s_mempool.write().await;
                            m.prune_against_ledger(&l);
                        }
                        println!(
                            "[STAKER] Forged PoS Block #{} (hash: 0x{}...)",
                            h,
                            &hex::encode(bhash)[..16]
                        );
                        s_p2p.broadcast_block(&block).await;
                    }
                }
            }
        });
    }

    // Keep daemon running cleanly until Ctrl-C / SIGTERM
    let _ = tokio::signal::ctrl_c().await;
    println!("\n[SHUTDOWN] Flushing peer DB and stopping Sardhan node gracefully...");
    p2p.persist_peers().await;
}
