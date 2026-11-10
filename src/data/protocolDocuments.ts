import { ProtocolDoc } from '../types/protocol';

export const PROTOCOL_DOCS: ProtocolDoc[] = [
  {
    id: 'overview',
    title: 'Protocol Architecture Executive Overview',
    badge: 'PHASE 1 & 2',
    rfcNumber: 'RFC-100',
    summary: 'High-level synthesis of Sardhan Chain (SDR): a decentralized, zero-premine, zero-fee Layer-1 secured by 50/50 Proof of Power.',
    markdownContent: `# Sardhan Chain (SDR) — Layer-1 Architecture Overview

**Network:** Sardhan Chain  
**Ticker:** SDR  
**Max Supply:** 21,000,000.00000000 SDR  
**Premine:** 0.00000000 SDR (Strictly Zero)  
**Consensus:** Hybrid 50% Proof of Work / 50% Proof of Stake (Proof of Power)  
**Transaction Fees:** 0.00000000 SDR (Zero Transaction Fees)  
**Anti-Spam Mechanism:** Client-Side Proof-of-Work (Micro-PoW nonces) & Dynamic Mempool Difficulty Scaling  
**Mining Target:** CPU Mining initially, native Android ARMv8 optimization  
**Target Block Interval:** 150 seconds (2.5 minutes)  
**Halving Cadence:** Every 210,000 blocks (~364.58 days)  

---

### The Core Problem Solved by Sardhan Chain

1. **Vulnerability of Small PoW Networks:** Standalone PoW chains can be easily 51% attacked via hash rental (NiceHash, cloud clusters).
2. **Plutocracy of Pure PoS Networks:** Standalone PoS networks concentrate voting weight in early token holders, suffer from "Nothing-at-Stake" fork attacks, and lack real-world thermodynamic anchors.
3. **The Fee Extortion Barrier:** High transaction fees on Bitcoin and Ethereum prevent everyday micro-payments and user autonomy.
4. **Hardware Centralization:** SHA-256 and Scrypt networks are monopolized by capital-intensive ASIC manufacturing cartels.

---

### The Sardhan Solution Matrix

- **Dual-Anchor Security (Proof of Power):** Mined blocks and Staked blocks interleave. To reorganize the ledger, an attacker must possess both >50% global hashrate and >50% mature circulating stake simultaneously.
- **Democratized CPU/ARM Mining:** SardhanHash utilizes native hardware AES-NI and ARMv8 Cryptographic Extensions present on standard consumer CPUs and Android smartphones, neutralizing ASIC advantages.
- **Zero-Fee Sovereignty:** Financial transaction fees are replaced with lightweight client-side micro-PoW (BLAKE3 nonces requiring ~100ms CPU compute), keeping network access completely free for human users while economically ruining automated spam bots.`
  },
  {
    id: 'audit',
    title: 'Protocol Security Audit & Vulnerability Assessment',
    badge: 'SECURITY AUDIT',
    rfcNumber: 'RFC-108',
    summary: 'Independent security review uncovering 3 Critical, 5 High, and 4 Medium vulnerabilities with mathematical proofs, fixes, and test vectors.',
    markdownContent: `# Independent Protocol Security Audit Report

**Status:** Complete — Implementation on Hold Pending Protocol Ratification  
**Findings:** 3 Critical, 5 High-Risk, 4 Medium-Risk Vulnerabilities Identified  

---

### Audit Finding Highlights

1. **[CRITICAL] SEC-CRIT-01: Offline Micro-PoW Pre-Mining Floods**
   - *Weakness:* Client micro-PoW had no temporal binding, allowing attackers to pre-mine 1,000,000 valid zero-fee nonces offline and burst-flood the network.
   - *Fix:* Block-Bound Micro-PoW binding nonces to a block hash from within the last 16 blocks (~40 min window).

2. **[CRITICAL] SEC-CRIT-02: Cross-Network Transaction Replay**
   - *Weakness:* Missing ChainID in transaction header and signature preimage allowed Testnet signatures to be replayed on Mainnet.
   - *Fix:* Mandatory 32-bit ChainID embedded in wire format and signature preimage.

3. **[CRITICAL] SEC-CRIT-03: Stake Grinding via Immediate Parent Hash Manipulation**
   - *Weakness:* StakeModifier updated every block using the immediate previous block hash, allowing miners to grind nonces to win future staking blocks.
   - *Fix:* 100-block lagging stake modifier buffer M_h = BLAKE3(M_{h-100} || B_{h-100}.Hash).

4. **[HIGH] SEC-HIGH-01: Undefined Fork-Choice Weight & Channel Starvation**
   - *Fix:* Formulated cumulative weight W(C) = sum(2^256 / Target(B)) and a 32-block sliding window channel balance invariant (25%-75%).

5. **[MEDIUM] SEC-MED-01: Block Header Size Arithmetic Mismatch**
   - *Fix:* Standardized canonical header to exactly 196 bytes fixed-size.`
  },
  {
    id: 'math',
    title: 'Consensus Mathematics & Formal Invariant Derivations',
    badge: 'FORMAL MATH',
    rfcNumber: 'RFC-109',
    summary: 'Exact scalar chain-weight function, channel balancing window proofs, low-difficulty manipulation immunity, and finality probability distributions.',
    markdownContent: `# Consensus Mathematics & Formal Invariant Derivations

### 1. Cumulative Chain Weight Formulation
The fork-choice metric is the scalar sum:
W(C) = sum(floor(2^256 / (Target_i + 1))).
Each block contributes weight inversely proportional to its target difficulty, eliminating advantage from mining rapid low-difficulty blocks.

### 2. Channel Balancing Sliding Window
In any 64-block window:
24 <= Count(PoW) <= 40  and  24 <= Count(PoS) <= 40.
Branches failing this invariant are dropped unconditionally by full nodes.

### 3. Staking Probability & Scale Invariance
The probability of a wallet minting a PoS block at second t is P = (Target_PoS * Balance) / 2^256. Staking yield is perfectly scale-invariant.`
  },
  {
    id: 'risk',
    title: 'Remaining Risk Register & Readiness Assessment',
    badge: 'RISK AUDIT',
    rfcNumber: 'RFC-110',
    summary: 'Honest evaluation of unresolved risks (bootstrap period, Android Doze mode, partition >288 blocks) and Phase 3 readiness recommendations.',
    markdownContent: `# Remaining Risk Register & Readiness Assessment

### 1. Transparent Disclosure of Residual Risks
- **RISK-01 (Bootstrap Hashrate):** Blocks 0–288 operate as pure PoW before coin maturity. Mitigated via coordinated launch and trusted seed nodes.
- **RISK-02 (Android OS Power Management):** Doze mode kills background threads. Mitigated via foreground service with sticky notification.
- **RISK-03 (Late-Epoch Validator Incentives):** Zero fees require ratification of a 0.05 SDR baseline security budget prior to Epoch 10.
- **RISK-04 (Partitions > 288 Blocks):** Weak subjectivity prevents automatic reorg after 12 hours. Manual validator consensus required.

### 2. Implementation Clearance Verdict
- **PHASE 3 (Foundation & Primitives):** CONDITIONALLY APPROVED.
- **PHASE 4 & 5 (Consensus & P2P):** GATED PENDING 100% UNIT TEST PASS.`
  },
  {
    id: 'research',
    title: 'Research & Verus Comparative Analysis',
    badge: 'PHASE 1 RESEARCH',
    rfcNumber: 'RFC-101',
    summary: 'Deep architectural comparison between Verus Proof of Power, Decred ticket voting, Monero RandomX, and Bitcoin Nakamoto consensus.',
    markdownContent: `# Research & Comparative Analysis

### 1. The Lessons of Verus Proof of Power (VerusPoP)
Verus pioneered the 50% PoW / 50% PoS hybrid model. In VerusPoP, an attacker possessing 80% of the world's mining hashrate cannot reorg the chain without also holding a majority of the active staking coins. This eliminates the existential threat of hash rental attacks.

### 2. Decred Hybrid Voting Mechanism vs Sardhan Chain
Decred uses PoW miners to find blocks, but requires 3 to 5 PoS ticket holders to vote on each block before acceptance. While effective, Decred's ticket pool requires locking tokens for weeks, creates ticket price volatility, and increases mempool complexity. Sardhan Chain adopts a direct UTXO coin-age kernel without the friction of ticket lotteries.

### 3. Monero RandomX vs SardhanHash
Monero's RandomX executes random code in a 2GB scratchpad. While ASIC-resistant, this prevents low-RAM devices (e.g., standard Android smartphones with 2GB–3GB RAM where the OS uses 1.5GB) from mining. SardhanHash uses ~8MB scratchpad with hardware AES/SHA instructions, enabling native mobile ARM mining.

### 4. Zero-Fee Economic Modeling
Traditional blockchains rely on fees to meter block space. Sardhan Chain replaces economic fee bids with computational micro-PoW bids, ensuring fair access without requiring users to purchase tokens merely to cover gas.`
  },
  {
    id: 'specification',
    title: 'Complete Technical Specification',
    badge: 'PHASE 2 SPECIFICATION',
    rfcNumber: 'RFC-102',
    summary: 'Exhaustive protocol specification: binary layouts, 144-byte block headers, Ed25519 addresses (sdr1...), and P2P wire framing.',
    markdownContent: `# Complete Technical Specification

### 1. Global Invariants
- Maximum Supply: 21,000,000.00000000 SDR (2,100,000,000,000,000 Sardhans).
- Target Block Time: 150 seconds.
- Halving Epoch: 210,000 blocks.
- Minimum Coin Age for Staking: 288 blocks (~12 hours).
- Maximum Coin Age for Staking: 17,280 blocks (~30 days).
- Address Prefix: Bech32m 'sdr1...'.

### 2. Block Header Binary Layout (144 bytes)
- Version: uint32 (4 bytes)
- PrevBlockHash: uint256 (32 bytes)
- MerkleRoot: uint256 (32 bytes)
- Timestamp: uint64 (8 bytes, Unix epoch)
- BlockHeight: uint64 (8 bytes)
- ConsensusBits: uint32 (4 bytes compact target)
- ConsensusType: uint8 (1 byte: 0x01=PoW, 0x02=PoS)
- StakeModifier: uint256 (32 bytes entropy)
- Nonce: uint64 (8 bytes PoW nonce)
- ValidatorSigLen: uint8 (1 byte)
- ValidatorSignature: 64 bytes (Ed25519 signature for PoS)

### 3. Wire Protocol & Networking
- TCP Port: 9333 (Mainnet), 19333 (Testnet).
- RPC Port: 9332 (Mainnet), 19332 (Testnet).
- Magic Bytes: 0x53 0x44 0x52 0x01 ('SDR\\x01').`
  },
  {
    id: 'consensus',
    title: 'Hybrid Consensus Proposal (SDR-PoP)',
    badge: 'PHASE 2 CONSENSUS',
    rfcNumber: 'RFC-103',
    summary: 'Sardhan Proof of Power: dual-channel PoW/PoS mechanics, independent DarkGravityWave v3 DAA, and staking kernel math.',
    markdownContent: `# Hybrid Consensus Proposal (Sardhan Proof of Power)

### 1. The Dual-Channel Engine
Sardhan Chain features two block minting channels targeting 50/50 balance:
- **PoW Channel (0x01):** Valid when SardhanHash(Header) < Target_PoW.
- **PoS Channel (0x02):** Valid when BLAKE3(StakeModifier || TxID || Vout || Timestamp) < Target_PoS * UTXO_Value.

### 2. Difficulty Adjustment (Dual DGWv3)
Each channel maintains its own independent difficulty adjustment algorithm using a 144-block moving window. Each channel targets an individual 300-second interval, yielding a combined network block interval of 150 seconds.

### 3. Bootstrap Sequence
- Blocks 0–288: Mined exclusively via CPU PoW to bootstrap the initial coin supply.
- Block 288 onwards: Staking automatically activates as the first mined coinbase UTXOs reach maturity.`
  },
  {
    id: 'threat-model',
    title: 'Security Threat Model & Attack Matrix',
    badge: 'SECURITY AUDIT',
    rfcNumber: 'RFC-104',
    summary: 'Formal threat analysis covering 51% hashrate attacks, nothing-at-stake, long-range forks, time-warp, and zero-fee spam flood.',
    markdownContent: `# Security Threat Model & Attack Matrix

### 1. The 51% PoW Hash Attack Proof
In a pure PoW chain, an attacker with >50% hashrate can reliably execute deep reorganizations. In Sardhan Chain, PoW blocks cannot finalize without interleaved PoS staker signatures. An attacker with 90% hashrate but only 5% of staked coins will stall on PoS blocks, making deep reorgs impossible.

### 2. Zero-Fee Spam Flooding
Mitigated via mandatory Client-Side Micro-PoW (BLAKE3 nonces requiring ~100ms CPU per transaction). If the mempool exceeds 50% capacity (32MB), the required difficulty scales quadratically, making mass spam mathematically impossible.

### 3. Weak Subjectivity & Long-Range Reorgs
Nodes enforce a hard 288-block maximum reorganization depth. Any branch forking deeper than 288 blocks (~12 hours) is rejected unconditionally.`
  },
  {
    id: 'repository',
    title: 'Modular Repository Structure & Blueprints',
    badge: 'PHASE 3 BLUEPRINT',
    rfcNumber: 'RFC-105',
    summary: 'Clean multi-package architecture: crypto, primitives, consensus, ledger, mempool, p2p, rpc, node, wallet, miner, explorer.',
    markdownContent: `# Modular Repository Architecture

The Sardhan Chain codebase is organized into isolated, independently testable packages:
- **packages/crypto:** Ed25519, BLAKE3, SardhanHash, Bech32m, BIP-39/44.
- **packages/primitives:** Block, BlockHeader, Transaction, TxIn, TxOut, OutPoint, MerkleTree.
- **packages/consensus:** PoW Validator, PoS Kernel, DGWv3 DAA, Halving Calculator.
- **packages/ledger:** UTXO Set, ChainState, Undo Blocks, Genesis Block.
- **packages/mempool:** Micro-PoW Verifier, Dynamic Scaler, Eviction Queue.
- **packages/p2p:** Wire Protocol, Peer Manager, Sync Engine, Gossip.
- **packages/rpc:** JSON-RPC 2.0 Server (getblockchaininfo, getblock, submitblock).
- **packages/node:** Full node daemon orchestrator ('sdrd').
- **packages/wallet:** Deterministic HD wallet, Tx builder, background staker ('sdr-cli').
- **packages/miner:** Multi-threaded CPU & ARMv8 miner worker ('sdr-miner').
- **packages/explorer:** Lightweight block and transaction explorer ('sdr-explorer').`
  },
  {
    id: 'tokenomics',
    title: 'Tokenomics & Emission Mathematics',
    badge: 'TOKENOMICS',
    rfcNumber: 'RFC-106',
    summary: 'Mathematical verification of the 21,000,000 SDR supply cap, halving epoch schedules, and equal 50/50 mining/staking split.',
    markdownContent: `# Tokenomics & Emission Mathematics

### 1. Invariant: 21,000,000 SDR Hard Cap
The supply is governed by:
Reward(h) = floor((50 * 10^8) / 2^(floor(h / 210,000))) Sardhans.
Summing over all 33 halving epochs yields exactly 20,999,999.97690000 SDR, strictly below the 21,000,000 cap.

### 2. Fair Distribution
- 0% Premine
- 0% Founder/Developer Tax
- 10,500,000 SDR to CPU/ARM Miners
- 10,500,000 SDR to Community Stakers`
  },
  {
    id: 'mempool',
    title: 'Zero-Fee Mempool & Micro-PoW Engineering',
    badge: 'MEMPOOL RFC',
    rfcNumber: 'RFC-107',
    summary: 'Anti-DoS engineering: BLAKE3 micro-nonces, quadratic dynamic congestion scaling, and micro-PoW density sorting.',
    markdownContent: `# Zero-Fee Mempool Engineering

### 1. Proof-of-Work As Bandwidth Currency
Rather than paying SDR coins, transactions spend CPU entropy. A human sending a payment computes ~100ms of BLAKE3 hashes. A spammer attempting to send 100,000 transactions must spend ~28 hours of continuous CPU compute.

### 2. Dynamic Congestion Scaling
When mempool memory exceeds 32MB, required micro-PoW difficulty scales quadratically:
RequiredTarget = BaseTarget * ((64MB - CurrentBytes) / 32MB)^2.
Transactions in the mempool are sorted and packed into blocks by Micro-PoW Density (Difficulty / ByteSize).`
  }
];
