# Sardhan Chain (SDR) — Repository Structure & Module Blueprint

**Document Version:** 1.0.0-BLUEPRINT  
**Phase:** Phase 3 (Repository Structure & Modular Blueprint)  
**Status:** Complete / Ready for Review  
**Architecture:** Multi-Package Clean Modular Monorepo  

---

## 1. Directory Tree & Modular Architecture

To ensure strict separation of concerns, testability, and deterministic builds, the Sardhan Chain repository follows a clean modular monorepo architecture:

```
sardhan-chain/
├── docs/                                 # Architectural Specifications & RFCs
│   ├── 01_RESEARCH_VERUS_AND_COMPARATIVE_ANALYSIS.md
│   ├── 02_TECHNICAL_SPECIFICATION.md
│   ├── 03_HYBRID_CONSENSUS_PROPOSAL.md
│   ├── 04_SECURITY_THREAT_MODEL.md
│   ├── 05_REPOSITORY_STRUCTURE_AND_MODULE_BLUEPRINT.md
│   ├── 06_ZERO_FEE_MEMPOOL_ENGINEERING.md
│   └── 07_TOKENOMICS_AND_EMISSION_SCHEDULE.md
├── packages/
│   ├── crypto/                           # Low-level Cryptographic Primitives
│   │   ├── src/
│   │   │   ├── ed25519.ts                # Curve25519/Ed25519 signing & verification
│   │   │   ├── blake3.ts                 # BLAKE3 256-bit hashing & tree hashing
│   │   │   ├── sardhan_hash.ts           # CPU & ARMv8 optimized PoW hashing function
│   │   │   ├── bech32m.ts                # Checksummed 'sdr1' address encoding
│   │   │   └── bip39_44.ts               # Mnemonic derivation & HD keypaths
│   │   └── tests/
│   ├── primitives/                       # Core Blockchain Data Types
│   │   ├── src/
│   │   │   ├── block.ts                  # Block & BlockHeader serialization (144 bytes)
│   │   │   ├── transaction.ts            # TxIn, TxOut, OutPoint, LockTime, MicroPoW
│   │   │   ├── script.ts                 # Standard P2PKH script execution & validation
│   │   │   ├── merkle.ts                 # Incremental Merkle Tree calculation
│   │   │   └── serializer.ts             # Deterministic binary byte buffers (LE/BE)
│   │   └── tests/
│   ├── consensus/                        # Hybrid Consensus & Validation Engine
│   │   ├── src/
│   │   │   ├── pow_validator.ts          # PoW difficulty verification & target math
│   │   │   ├── pos_kernel.ts             # Staking kernel evaluation & modifier entropy
│   │   │   ├── daa_darkgravity.ts        # DarkGravityWave v3 dual difficulty adjustment
│   │   │   ├── halving_calculator.ts     # 21M SDR mathematical emission & halving curve
│   │   │   └── fork_choice.ts            # Accumulated Proof-of-Power chain weight rule
│   │   └── tests/
│   ├── ledger/                           # Distributed UTXO Ledger & State Engine
│   │   ├── src/
│   │   │   ├── utxo_set.ts               # Persistent UTXO key-value store (LevelDB/RocksDB)
│   │   │   ├── chain_state.ts            # Best chain tip, height, total work accumulator
│   │   │   ├── block_undo.ts             # Reorganization undo records for rollback
│   │   │   └── genesis.ts                # Deterministic genesis block constructor
│   │   └── tests/
│   ├── mempool/                          # Zero-Fee Anti-Spam Transaction Pool
│   │   ├── src/
│   │   │   ├── micro_pow_verifier.ts     # Client-side micro-PoW validation (<100µs)
│   │   │   ├── congestion_scaler.ts      # Quadratic dynamic difficulty adjustment
│   │   │   ├── eviction_queue.ts         # PoW density-sorted priority queue
│   │   │   └── double_spend_guard.ts     # In-flight input conflict rejector
│   │   └── tests/
│   ├── p2p/                              # Peer-to-Peer Networking Engine
│   │   ├── src/
│   │   │   ├── wire_protocol.ts          # Framing, magic bytes (0x53445201), checksums
│   │   │   ├── peer_manager.ts           # IP subnet bucketing, reputation scoring
│   │   │   ├── sync_engine.ts            # Headers-first parallel block synchronization
│   │   │   ├── gossip.ts                 # Inventory relay (inv/getdata)
│   │   │   └── seeds.ts                  # Hardcoded decentralized DNS/IP seed nodes
│   │   └── tests/
│   ├── rpc/                              # JSON-RPC 2.0 Server Interface
│   │   ├── src/
│   │   │   ├── rpc_server.ts             # HTTP/IPC JSON-RPC 2.0 listener (port 9332)
│   │   │   ├── blockchain_methods.ts     # getblock, getbestblockhash, gettxout
│   │   │   ├── mining_methods.ts         # getblocktemplate, submitblock, getmininginfo
│   │   │   └── staking_methods.ts        # getstakinginfo, getstakecandidates
│   │   └── tests/
│   ├── node/                             # Full Node Daemon (`sdrd`)
│   │   ├── src/
│   │   │   ├── daemon.ts                 # Lifecycle orchestrator & event bus
│   │   │   ├── config.ts                 # CLI flags, config file parser (~/.sardhan/sdr.conf)
│   │   │   └── signal_handler.ts         # Clean database flush on SIGINT/SIGTERM
│   │   └── tests/
│   ├── wallet/                           # Cryptographic Wallet Engine & CLI (`sdr-cli`)
│   │   ├── src/
│   │   │   ├── hd_keyring.ts             # BIP-39 mnemonic seed phrase & derivation
│   │   │   ├── tx_builder.ts             # Coin selection, signature creation, micro-PoW solver
│   │   │   ├── staker_worker.ts          # Background staking engine iterating UTXOs
│   │   │   └── cli_runner.ts             # Interactive command-line client
│   │   └── tests/
│   ├── miner/                            # Multi-Threaded CPU & ARM Mining Engine
│   │   ├── src/
│   │   │   ├── cpu_miner.ts              # Multi-threaded worker pool utilizing AES-NI
│   │   │   ├── arm_optimizer.ts          # ARMv8 Cryptographic Extension acceleration
│   │   │   └── stratum_client.ts         # Decentralized stratum mining protocol client
│   │   └── tests/
│   └── explorer/                         # Public Blockchain Explorer Service
│       ├── src/
│       │   ├── indexer.ts                # Real-time block & transaction indexing
│       │   ├── search_engine.ts          # Address, TxID, Block hash search lookup
│       │   └── web_api.ts                # REST & WebSocket live stream
│       └── tests/
└── tests/                                # System & End-to-End Integration Tests
    ├── vectors/                          # Known consensus test vectors (BLAKE3, Ed25519)
    ├── e2e_bootstrap.test.ts             # 100-block mining & staking bootstrap test
    ├── e2e_reorg_resistance.test.ts      # 51% PoW attacker isolation verification
    └── e2e_zero_fee_flood.test.ts        # 10,000 tx spam flood eviction test
```

---

## 2. Module Boundaries & Dependency Invariants

1. **`crypto` has zero internal dependencies:** It relies exclusively on pure mathematics, system entropy (`crypto.getRandomValues`), and native hardware instructions.
2. **`primitives` depends ONLY on `crypto`:** Defines immutable data models with deterministic binary encode/decode methods.
3. **`consensus` depends on `primitives` and `crypto`:** Completely decoupled from database I/O, networking, and filesystem access. This ensures consensus can be fuzz-tested in memory with millions of synthetic blocks.
4. **`ledger` depends on `primitives` and `consensus`:** Encapsulates state persistence and atomicity (ACID).
5. **`mempool` depends on `primitives` and `ledger`:** Inspects UTXO availability and validates micro-PoW before holding transactions in RAM.
6. **`p2p` and `rpc` are transport adapters:** They interact with the node core purely through well-typed method calls and event streams.

---

## 3. Strict Coding & Cryptographic Invariants

- **No Simulation or Fake Mining:** Every hash calculation, signature generation, and difficulty check must run actual cryptographic algorithms (real Ed25519, real BLAKE3, real SardhanHash).
- **Constant-Time Verification:** All signature checks and private key handlings must execute in constant time to prevent side-channel timing attacks.
- **Strict Big-Endian / Little-Endian Standards:** Hashes are displayed in reverse byte order (RPC big-endian convention like Bitcoin), while numeric fields in binary wire format are standard Little-Endian.
- **Zero Premine Verification:** The genesis block configuration hardcodes reward to 0 and contains no initial UTXOs.
