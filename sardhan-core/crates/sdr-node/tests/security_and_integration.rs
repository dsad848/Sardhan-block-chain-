//! Comprehensive Adversarial Security, Fuzzing, Performance, and 3-Node Integration Tests

use sdr_consensus::{
    calculate_block_reward, compact_to_target, evaluate_pos_kernel, hash_less_than_target,
    verify_channel_balance, ConsensusError,
};
use sdr_crypto::{sardhan_hash, Keypair};
use sdr_ledger::{current_unix_timestamp, Ledger, LedgerError};
use sdr_mempool::{Mempool, MempoolError};
use sdr_p2p::{P2pManager, WireMessage};
use sdr_primitives::{
    compute_merkle_root, Block, BlockHeader, ConsensusType, Network, OutPoint, Transaction, TxIn,
    TxOut, TxWitness, SARDHAN_MAINNET_MAGIC, SARDHAN_REGTEST_MAGIC, SARDHAN_TESTNET_MAGIC,
};
use sdr_rpc::{execute_rpc_method, RpcContext};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

fn mine_pow_block(ledger: &Ledger, coinbase_to: [u8; 32], extra_txs: Vec<Transaction>) -> Block {
    let h = ledger.best_height + 1;
    let reward = calculate_block_reward(h);
    let mut txs = vec![Transaction {
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
    }];
    txs.extend(extra_txs);

    let txids: Vec<[u8; 32]> = txs.iter().map(|t| t.txid()).collect();
    let bits = ledger.expected_next_bits(ledger.best_block_hash, ConsensusType::ProofOfWork);
    let lsm = ledger.expected_lagging_modifier(ledger.best_block_hash, h);
    let mtp = ledger.median_time_past_for_parent(ledger.best_block_hash);
    let ts = current_unix_timestamp().max(mtp + 1);

    let mut hdr = BlockHeader {
        version: 1,
        prev_block_hash: ledger.best_block_hash,
        merkle_root: compute_merkle_root(&txids),
        timestamp: ts,
        block_height: h,
        consensus_bits: bits,
        consensus_type: ConsensusType::ProofOfWork,
        lagging_stake_modifier: lsm,
        pow_nonce: 0,
        reserved_flags: [0; 3],
        validator_signature: vec![0; 64],
    };

    let target = compact_to_target(bits);
    for nonce in 1..200_000u64 {
        hdr.pow_nonce = nonce;
        let hash = sardhan_hash(&hdr.to_bytes());
        if hash_less_than_target(&hash, &target) {
            return Block {
                header: hdr,
                transactions: txs,
            };
        }
    }
    panic!("Failed mining regtest block");
}

fn forge_pos_block(
    ledger: &Ledger,
    staker: &Keypair,
    kernel_op: OutPoint,
    kernel_val: u64,
    extra_txs: Vec<Transaction>,
) -> Block {
    let h = ledger.best_height + 1;
    let reward = calculate_block_reward(h);
    let bits = ledger.expected_next_bits(ledger.best_block_hash, ConsensusType::ProofOfStake);
    let lsm = ledger.expected_lagging_modifier(ledger.best_block_hash, h);
    let mtp = ledger.median_time_past_for_parent(ledger.best_block_hash);
    let mut ts = current_unix_timestamp().max(mtp + 1);

    while !evaluate_pos_kernel(&lsm, &kernel_op.txid, kernel_op.vout, ts, kernel_val, bits) {
        ts += 1;
    }

    let mut coinstake = Transaction {
        chain_id: ledger.config.chain_id,
        version: 1,
        recent_block_hash: ledger.best_block_hash,
        micro_pow_nonce: 0,
        lock_time: 0,
        inputs: vec![TxIn {
            previous_output: kernel_op,
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            value: kernel_val + reward,
            recipient_pubkey: staker.public_key_bytes(),
        }],
        witnesses: vec![],
    };
    let sig = staker.sign(&coinstake.signature_digest(0));
    coinstake.witnesses.push(TxWitness {
        signature: sig.to_vec(),
        public_key: staker.public_key_bytes(),
    });

    let mut txs = vec![coinstake];
    txs.extend(extra_txs);
    let txids: Vec<[u8; 32]> = txs.iter().map(|t| t.txid()).collect();

    let mut hdr = BlockHeader {
        version: 1,
        prev_block_hash: ledger.best_block_hash,
        merkle_root: compute_merkle_root(&txids),
        timestamp: ts,
        block_height: h,
        consensus_bits: bits,
        consensus_type: ConsensusType::ProofOfStake,
        lagging_stake_modifier: lsm,
        pow_nonce: 0,
        reserved_flags: [0; 3],
        validator_signature: vec![0; 64],
    };
    hdr.validator_signature = staker.sign(&hdr.pos_signature_preimage()).to_vec();

    Block {
        header: hdr,
        transactions: txs,
    }
}

fn make_zero_fee_tx(
    ledger: &Ledger,
    mempool: &Mempool,
    sender: &Keypair,
    recipient: [u8; 32],
    amount: u64,
) -> Transaction {
    let utxos = ledger.get_address_utxos(&sender.public_key_bytes());
    let (op, entry) = utxos.first().expect("sender has UTXO").clone();
    let change = entry.output.value - amount;
    let mut outputs = vec![TxOut {
        value: amount,
        recipient_pubkey: recipient,
    }];
    if change > 0 {
        outputs.push(TxOut {
            value: change,
            recipient_pubkey: sender.public_key_bytes(),
        });
    }

    let mut tx = Transaction {
        chain_id: ledger.config.chain_id,
        version: 1,
        recent_block_hash: ledger.best_block_hash,
        micro_pow_nonce: 0,
        lock_time: 0,
        inputs: vec![TxIn {
            previous_output: op,
            sequence: 0xffffffff,
        }],
        outputs,
        witnesses: vec![],
    };
    let target = mempool.current_micro_pow_target();
    assert!(Mempool::solve_micro_pow(&mut tx, &target, 200_000));
    let sig = sender.sign(&tx.signature_digest(0));
    tx.witnesses.push(TxWitness {
        signature: sig.to_vec(),
        public_key: sender.public_key_bytes(),
    });
    tx
}

#[test]
fn test_adversarial_double_spend_in_block_and_mempool() {
    let mut ledger = Ledger::new_for_network(Network::Regtest);
    let mut mempool = Mempool::with_config(
        ledger.config.chain_id,
        ledger.config.micro_pow_base_bits,
        sdr_mempool::MAX_MEMPOOL_BYTES,
    );
    let alice = Keypair::generate();
    let bob = Keypair::generate();
    let carol = Keypair::generate();

    let b1 = mine_pow_block(&ledger, alice.public_key_bytes(), vec![]);
    ledger.apply_block(&b1.header, &b1.transactions).unwrap();

    let tx_ab = make_zero_fee_tx(&ledger, &mempool, &alice, bob.public_key_bytes(), 1_000_000_000);
    let tx_ac = make_zero_fee_tx(&ledger, &mempool, &alice, carol.public_key_bytes(), 1_000_000_000);

    // 1. Mempool admits first spend, rejects conflicting second spend
    assert!(mempool
        .insert_transaction_with_ledger(tx_ab.clone(), &ledger, None, current_unix_timestamp())
        .is_ok());
    let conflict = mempool.insert_transaction_with_ledger(
        tx_ac.clone(),
        &ledger,
        None,
        current_unix_timestamp(),
    );
    assert_eq!(conflict, Err(MempoolError::InFlightConflict));

    // 2. Block attempting to pack both conflicting spends is atomically rejected
    let bad_block = mine_pow_block(&ledger, alice.public_key_bytes(), vec![tx_ab, tx_ac]);
    assert_eq!(
        ledger.apply_block(&bad_block.header, &bad_block.transactions),
        Err(LedgerError::DoubleSpend)
    );
}

#[test]
fn test_adversarial_invalid_malleable_and_replay_transactions() {
    let mut ledger = Ledger::new_for_network(Network::Regtest);
    let mut mempool = Mempool::with_config(
        ledger.config.chain_id,
        ledger.config.micro_pow_base_bits,
        sdr_mempool::MAX_MEMPOOL_BYTES,
    );
    let alice = Keypair::generate();
    let bob = Keypair::generate();

    let b1 = mine_pow_block(&ledger, alice.public_key_bytes(), vec![]);
    ledger.apply_block(&b1.header, &b1.transactions).unwrap();

    // 1. Corrupt signature (non-canonical S scalar)
    let mut bad_sig_tx =
        make_zero_fee_tx(&ledger, &mempool, &alice, bob.public_key_bytes(), 500_000_000);
    bad_sig_tx.witnesses[0].signature[63] = 0xFF;
    assert_eq!(
        mempool.insert_transaction_with_ledger(
            bad_sig_tx,
            &ledger,
            None,
            current_unix_timestamp()
        ),
        Err(MempoolError::InvalidSignature(0))
    );

    // 2. Cross-network ChainID replay attempt (Mainnet ChainID submitted to Regtest/Testnet)
    let mut replay_tx =
        make_zero_fee_tx(&ledger, &mempool, &alice, bob.public_key_bytes(), 500_000_000);
    replay_tx.chain_id = SARDHAN_MAINNET_MAGIC;
    assert_eq!(
        mempool.insert_transaction_with_ledger(
            replay_tx.clone(),
            &ledger,
            None,
            current_unix_timestamp()
        ),
        Err(MempoolError::InvalidChainId(
            SARDHAN_MAINNET_MAGIC,
            SARDHAN_REGTEST_MAGIC
        ))
    );
    let mut testnet_replay = replay_tx;
    testnet_replay.chain_id = SARDHAN_TESTNET_MAGIC;
    let bad_blk = mine_pow_block(&ledger, alice.public_key_bytes(), vec![testnet_replay]);
    assert_eq!(
        ledger.apply_block(&bad_blk.header, &bad_blk.transactions),
        Err(LedgerError::InvalidChainId(
            SARDHAN_REGTEST_MAGIC,
            SARDHAN_TESTNET_MAGIC
        ))
    );
}

#[test]
fn test_adversarial_stake_grinding_and_double_signing_slashing() {
    let mut ledger = Ledger::new_for_network(Network::Regtest);
    let staker = Keypair::generate();

    // Mine 2 PoW blocks so staker UTXO matures past Regtest min_stake_confirmation_age (1)
    let b1 = mine_pow_block(&ledger, staker.public_key_bytes(), vec![]);
    ledger.apply_block(&b1.header, &b1.transactions).unwrap();
    let b2 = mine_pow_block(&ledger, Keypair::generate().public_key_bytes(), vec![]);
    ledger.apply_block(&b2.header, &b2.transactions).unwrap();

    let (kernel_op, kernel_entry) = ledger
        .get_address_utxos(&staker.public_key_bytes())
        .into_iter()
        .next()
        .unwrap();

    // 1. Stake grinding attempt: forge a PoS block with mutated lagging_stake_modifier
    let mut grinded = forge_pos_block(&ledger, &staker, kernel_op.clone(), kernel_entry.output.value, vec![]);
    grinded.header.lagging_stake_modifier = [0xAA; 32];
    grinded.header.validator_signature = staker.sign(&grinded.header.pos_signature_preimage()).to_vec();
    assert_eq!(
        ledger.submit_block_at(grinded, current_unix_timestamp() + 100),
        Err(LedgerError::Consensus(ConsensusError::StakeModifierMismatch))
    );

    // 2. Valid PoS block #3 accepted & logged
    let pos_a = forge_pos_block(&ledger, &staker, kernel_op.clone(), kernel_entry.output.value, vec![]);
    assert!(ledger
        .submit_block_at(pos_a.clone(), pos_a.header.timestamp + 10)
        .unwrap());

    // 3. Validator double-signs conflicting PoS block #3' at identical height 3 atop block #2!
    let mut tmp_ledger = Ledger::new_for_network(Network::Regtest);
    tmp_ledger.apply_block(&b1.header, &b1.transactions).unwrap();
    tmp_ledger.apply_block(&b2.header, &b2.transactions).unwrap();
    let mut pos_b = forge_pos_block(&tmp_ledger, &staker, kernel_op, kernel_entry.output.value, vec![]);
    // Ensure distinct block hash at same height
    pos_b.header.timestamp += 1;
    while !evaluate_pos_kernel(
        &pos_b.header.lagging_stake_modifier,
        &pos_b.transactions[0].inputs[0].previous_output.txid,
        pos_b.transactions[0].inputs[0].previous_output.vout,
        pos_b.header.timestamp,
        kernel_entry.output.value,
        pos_b.header.consensus_bits,
    ) {
        pos_b.header.timestamp += 1;
    }
    pos_b.header.validator_signature = staker.sign(&pos_b.header.pos_signature_preimage()).to_vec();

    let slash_err = ledger.submit_block_at(pos_b, current_unix_timestamp() + 500);
    assert!(matches!(slash_err, Err(LedgerError::DoubleSigningDetected(3, _))));
    assert!(ledger.slashed_validators.contains_key(&staker.public_key_bytes()));
}

#[test]
fn test_adversarial_long_range_reorg_and_timestamp_and_channel_balance() {
    let mut ledger = Ledger::new_for_network(Network::Regtest);
    // Test custom strict max_reorg_depth = 2 for deep-fork long-range attack test
    ledger.config.max_reorg_depth = 2;
    let alice = Keypair::generate();
    let bob = Keypair::generate();

    for _ in 0..3 {
        let b = mine_pow_block(&ledger, alice.public_key_bytes(), vec![]);
        ledger.apply_block(&b.header, &b.transactions).unwrap();
    }

    // Build competing 4-block branch from Genesis (reorg depth = 3 > limit 2)
    let mut fork = Ledger::new_for_network(Network::Regtest);
    let mut fork_blocks = Vec::new();
    for _ in 0..4 {
        let b = mine_pow_block(&fork, bob.public_key_bytes(), vec![]);
        fork.apply_block(&b.header, &b.transactions).unwrap();
        fork_blocks.push(b);
    }

    let now = current_unix_timestamp() + 200;
    for b in &fork_blocks[..3] {
        let _ = ledger.submit_block_at(b.clone(), now).unwrap();
    }
    let reorg_res = ledger.submit_block_at(fork_blocks[3].clone(), now);
    assert!(matches!(reorg_res, Err(LedgerError::ReorgDepthExceeded(3, 2))));

    // Timestamp manipulation: future drift > 7200s rejected
    let mut future_blk = mine_pow_block(&ledger, alice.public_key_bytes(), vec![]);
    future_blk.header.timestamp = current_unix_timestamp() + 10_000;
    assert!(matches!(
        ledger.submit_block_at(future_blk, current_unix_timestamp()),
        Err(LedgerError::Consensus(ConsensusError::InvalidTimestamp(_, _, _)))
    ));

    // Channel balancing invariant [24..40] over 64-block window
    let too_many_pow = vec![ConsensusType::ProofOfWork; 64];
    assert_eq!(
        verify_channel_balance(&too_many_pow),
        Err(ConsensusError::ChannelBalanceImbalance(64))
    );
}

#[test]
fn test_fuzz_wire_deserializers_never_panic() {
    let mut state: u64 = 0x534452_DEADBEEF;
    let mut next_u8 = || -> u8 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state & 0xFF) as u8
    };

    // Fuzz 2,000 arbitrary & mutated byte payloads across Tx, Block, Header, and WireMessage
    for i in 0..2_000 {
        let len = (i % 512) + 1;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(next_u8());
        }
        let _ = Transaction::from_bytes(&buf);
        let _ = Block::from_bytes(&buf);
        let _ = BlockHeader::from_bytes(&buf);
        let _ = WireMessage::from_bytes(&buf);
    }
}

#[test]
fn test_performance_serialization_and_validation_throughput() {
    let kp = Keypair::generate();
    let tx = Transaction {
        chain_id: SARDHAN_REGTEST_MAGIC,
        version: 1,
        recent_block_hash: [0x22; 32],
        micro_pow_nonce: 42,
        lock_time: 0,
        inputs: vec![TxIn {
            previous_output: OutPoint {
                txid: [0x11; 32],
                vout: 0,
            },
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            value: 50_000_000,
            recipient_pubkey: kp.public_key_bytes(),
        }],
        witnesses: vec![TxWitness {
            signature: kp.sign(b"bench").to_vec(),
            public_key: kp.public_key_bytes(),
        }],
    };

    let t0 = Instant::now();
    let mut total_bytes = 0usize;
    for _ in 0..5_000 {
        let bytes = tx.to_bytes();
        total_bytes += bytes.len();
        let dec = Transaction::from_bytes(&bytes).unwrap();
        assert_eq!(dec.version, 1);
    }
    let elapsed = t0.elapsed();
    assert!(elapsed.as_secs_f64() < 2.0, "5,000 tx roundtrips took {elapsed:?}");
    println!(
        "[PERF] 5,000 SegWit Tx roundtrips ({total_bytes} bytes) completed in {:.2} ms",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[tokio::test]
async fn test_three_node_hybrid_pow_pos_mempool_p2p_sync() {
    let staker_kp = Keypair::generate();
    let user_kp = Keypair::generate();

    let l1 = Arc::new(RwLock::new(Ledger::new_for_network(Network::Regtest)));
    let m1 = Arc::new(RwLock::new(Mempool::with_config(SARDHAN_REGTEST_MAGIC, 0x207fffff, 1024 * 1024)));
    let l2 = Arc::new(RwLock::new(Ledger::new_for_network(Network::Regtest)));
    let m2 = Arc::new(RwLock::new(Mempool::with_config(SARDHAN_REGTEST_MAGIC, 0x207fffff, 1024 * 1024)));
    let l3 = Arc::new(RwLock::new(Ledger::new_for_network(Network::Regtest)));
    let m3 = Arc::new(RwLock::new(Mempool::with_config(SARDHAN_REGTEST_MAGIC, 0x207fffff, 1024 * 1024)));

    let p1 = P2pManager::new(SARDHAN_REGTEST_MAGIC, "127.0.0.1:0".parse().unwrap(), Arc::clone(&l1), Arc::clone(&m1), None);
    let p2 = P2pManager::new(SARDHAN_REGTEST_MAGIC, "127.0.0.1:0".parse().unwrap(), Arc::clone(&l2), Arc::clone(&m2), None);
    let p3 = P2pManager::new(SARDHAN_REGTEST_MAGIC, "127.0.0.1:0".parse().unwrap(), Arc::clone(&l3), Arc::clone(&m3), None);

    let addr1 = p1.start().await.unwrap();
    let addr2 = p2.start().await.unwrap();
    let _addr3 = p3.start().await.unwrap();

    p2.connect_peer(&addr1.to_string()).await.unwrap();
    p3.connect_peer(&addr1.to_string()).await.unwrap();
    p3.connect_peer(&addr2.to_string()).await.unwrap();
    sleep(Duration::from_millis(200)).await;

    // 1. Node 1 mines 3 PoW blocks paying staker_kp -> broadcast to Node 2 & Node 3
    // (At height 3 with min_stake_confirmation_age=1, both block 1 and block 2 UTXOs are mature for staking)
    for _ in 0..3 {
        let blk = mine_pow_block(&*l1.read().await, staker_kp.public_key_bytes(), vec![]);
        l1.write().await.submit_block_at(blk.clone(), current_unix_timestamp()).unwrap();
        p1.broadcast_block(&blk).await;
    }

    for _ in 0..30 {
        if l2.read().await.best_height == 3 && l3.read().await.best_height == 3 {
            break;
        }
        sleep(Duration::from_millis(80)).await;
    }
    assert_eq!(l2.read().await.best_height, 3);
    assert_eq!(l3.read().await.best_height, 3);

    // 2. Node 3 sends a zero-fee transaction from staker_kp -> user_kp via RPC `sendrawtransaction`
    let rpc3 = RpcContext {
        ledger: Arc::clone(&l3),
        mempool: Arc::clone(&m3),
        p2p: Arc::clone(&p3),
        started_at: current_unix_timestamp(),
    };
    let tx = {
        let l = l3.read().await;
        let m = m3.read().await;
        make_zero_fee_tx(&l, &m, &staker_kp, user_kp.public_key_bytes(), 2_000_000_000)
    };
    let send_res = execute_rpc_method("sendrawtransaction", json!([hex::encode(tx.to_bytes())]), &rpc3).await;
    assert_eq!(send_res["result"]["accepted"], true);

    // Wait for mempool gossip propagation to Node 2
    for _ in 0..30 {
        if !m2.read().await.is_empty() {
            break;
        }
        sleep(Duration::from_millis(80)).await;
    }
    assert_eq!(m2.read().await.len(), 1);

    // 3. Node 2 forges PoS block #4 including the mempool zero-fee transaction & broadcasts
    let pos_blk = {
        let l = l2.read().await;
        let m = m2.read().await;
        let utxos = l.get_staking_utxos(&staker_kp.public_key_bytes(), l.best_height);
        // Pick a mature UTXO not being spent by the mempool tx
        let spent_op = &tx.inputs[0].previous_output;
        let (k_op, k_val) = utxos.into_iter().find(|(op, _)| op != spent_op).expect("mature unspent staking UTXO available");
        let mem_txs = m.get_block_template(1_000_000);
        forge_pos_block(&l, &staker_kp, k_op, k_val, mem_txs)
    };
    l2.write().await.submit_block_at(pos_blk.clone(), pos_blk.header.timestamp + 5).unwrap();
    m2.write().await.prune_against_ledger(&*l2.read().await);
    p2.broadcast_block(&pos_blk).await;

    // 4. All 3 nodes must converge on Height 4, PoS tip, user_kp balance = 20 SDR, and empty mempool!
    for _ in 0..40 {
        if l1.read().await.best_height == 4 && l3.read().await.best_height == 4 {
            break;
        }
        sleep(Duration::from_millis(80)).await;
    }
    assert_eq!(l1.read().await.best_height, 4);
    assert_eq!(l3.read().await.best_height, 4);
    assert_eq!(l1.read().await.best_block_hash, l2.read().await.best_block_hash);
    assert_eq!(l3.read().await.best_block_hash, l2.read().await.best_block_hash);
    assert_eq!(l1.read().await.get_balance(&user_kp.public_key_bytes()), 2_000_000_000);
    assert_eq!(l3.read().await.get_balance(&user_kp.public_key_bytes()), 2_000_000_000);
}
