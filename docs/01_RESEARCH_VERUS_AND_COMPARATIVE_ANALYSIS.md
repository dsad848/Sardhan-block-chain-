# Sardhan Chain (SDR) — Research & Comparative Analysis

**Document Version:** 1.0.0  
**Phase:** Phase 1 (Protocol Research & Literature Review)  
**Status:** Complete / Ready for Review  
**Subject:** Deep Analysis of Verus, Decred, Bitcoin, Monero, and Hybrid Consensus Architectures  

---

## 1. Executive Summary

Sardhan Chain (SDR) is conceived as an independent Layer-1 distributed ledger adhering to strict tokenomic invariants (21,000,000 SDR cap, no premine, scheduled halving, and zero transaction fees) secured by a hybrid Proof of Work (PoW) and Proof of Stake (PoS) consensus mechanism. To establish an uncompromised cryptographic foundation, this research evaluates mature open-source implementations—with particular emphasis on the Verus protocol (Verus Proof of Power) alongside Decred, Monero, and Satoshi-era Bitcoin.

This research establishes why a naive single-consensus model fails the Sardhan Chain mission and why a modernized Proof-of-Power hybrid model provides the required 51% hash-attack immunity, CPU/ARM mobile accessibility, and deterministic ledger finality.

---

## 2. Comparative Matrix: Mature Blockchain Protocols

| Dimension | Bitcoin Core | Verus Coin (VRSC) | Decred (DCR) | Monero (XMR) | **Sardhan Chain (SDR)** |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Consensus Model** | Pure PoW (Nakamoto) | Hybrid 50% PoW / 50% PoS (Proof of Power) | Hybrid PoW + PoS ticket voting | Pure PoW (RandomX) | **Hybrid 50% PoW / 50% PoS (Unified Proof of Power)** |
| **Mining Target** | ASIC-dominant (SHA-256d) | CPU/ARM-friendly (VerusHash 2.2b) | ASIC PoW (Blake-256) + PoS tickets | CPU-only (RandomX) | **CPU & ARMv8 Mobile (SardhanHash / VerusHash derivative)** |
| **51% Hash Attack Resistance** | 0% (Vulnerable to 51% hashrate) | **100% (Requires 51% Hash + 51% Staked Coin)** | High (PoS tickets validate PoW blocks) | High (due to memory-hard RandomX) | **Complete (Dual 51% threshold: Hash + Stake simultaneously)** |
| **Transaction Fees** | Dynamic fee market | Low fee market | Dynamic fee market | Dynamic fee market | **Zero Transaction Fees (Defended via Client-side PoW micro-tickets)** |
| **Maximum Supply** | 21,000,000 BTC | 83,540,184 VRSC | 21,000,000 DCR | Tail emission (~18.4M + 0.6 XMR/block) | **21,000,000 SDR (Strictly Hard-Capped)** |
| **Premine / Dev Tax** | None | 0% Premine / 0% Dev Tax | 8% Premine (Dev fund + airdrop) | None | **0% Premine, 0% Founder Allocation** |
| **Target Block Time** | 600s (10 min) | 60s (1 min) | ~300s (5 min) | 120s (2 min) | **150s (2.5 minutes)** |
| **Halving Interval** | 210,000 blocks (~4 yrs) | Halvings with smoothed decay | Reduction every 6144 blocks (~21 days) | Continuous reduction | **210,000 blocks (~1.0 yr at 150s block time)** |
| **Open Source License**| MIT License | MIT License | ISC License | BSD 3-Clause | **MIT License (Permissive & fully open)** |

---

## 3. Deep-Dive: Verus Protocol & Proof of Power

### 3.1 The Verus Proof of Power (VerusPoP) Paradigm
Verus introduced a consensus mechanism addressing the fundamental vulnerabilities of both standalone Proof of Work and standalone Proof of Stake:

1. **Weakness of Standalone PoW:** A hostile miner who rents or controls $>50\%$ of the network hashrate can unilaterally reorganize the chain, double-spend, and censor transactions. In young or niche blockchains, hash rental services (e.g., NiceHash) make 51% attacks trivial and inexpensive.
2. **Weakness of Standalone PoS:** The "Nothing-at-Stake" attack allows validators to vote on multiple conflicting chain forks at zero marginal computational cost, impeding fork resolution and inviting long-range history manipulation.
3. **The Proof of Power Solution:** Verus generates approximately 50% of blocks via PoW and 50% via PoS. Crucially, PoW blocks and PoS blocks are interleaved in the same consensus chain. To execute a successful 51% rewrite attack, an attacker must simultaneously acquire:
   $$\text{Attack Cost} = \text{Hashrate}_{>50\%} \quad\text{AND}\quad \text{Circulating Staked SDR}_{>50\%}$$
   If an attacker has 80% of the world's mining power but only 5% of the coins, their PoW blocks cannot finalize without valid PoS stakers co-validating the timeline. Conversely, a wealthy coin hoarder cannot forge blocks without the accompanying cryptographic proof-of-work.

### 3.2 VerusHash 2.2b Architectural Insights
Traditional PoW algorithms (Scrypt, Equihash, Ethash) were eventually outcompeted by custom FPGA and ASIC silicon. Verus solved this with **VerusHash**:
- Leverages the hardware AES-NI instructions present on standard consumer x86-64 CPUs and ARMv8 microarchitectures (modern Android phones, Raspberry Pi, Apple Silicon, server ARM cores).
- Uses quantum-resistant Haraka-512 v2 primitives interleaved with dynamic bit-manipulation and hardware acceleration.
- Because consumer CPUs and ARM system-on-chips already possess high-speed dedicated cryptographic execution units, building an ASIC offers minimal efficiency gain over standard mobile/desktop hardware. This guarantees widespread democratization and prevents industrial mining centralization.

---

## 4. Decred Hybrid Voting Mechanism vs VerusPoP

Decred employs a ticket-based hybrid consensus where PoW miners find blocks, but each block requires approval from 3 to 5 pseudo-randomly chosen PoS ticket holders.
- **Advantage:** PoS holders can invalidate a miner's block before it reaches the chain.
- **Drawback:** Requires an elaborate ticket pool, locked staking capital with fluctuating ticket prices, and an on-chain lottery. This introduces high protocol complexity, staking friction for small holders, and complex mempool logic.
- **Decision for Sardhan Chain:** Sardhan Chain adopts the **Verus Proof of Power model**, where staking is accessible directly from standard wallet UTXOs with minimum coin age, avoiding the need for an exotic ticket market.

---

## 5. Monero RandomX Analysis

Monero's RandomX executes random instruction streams inside a virtual machine (VM) with a 2GB scratchpad to enforce CPU dominance.
- **Strengths:** Utterly impervious to ASICs; preserves CPU decentralization.
- **Weaknesses:** Requires minimum 2.08 GiB of dedicated RAM per mining thread. This makes native mining on low-end ARM Android devices (2GB–3GB RAM phones where the OS consumes 1.5GB) unstable or impossible.
- **Decision for Sardhan Chain:** RandomX is rejected in favor of **SardhanHash** (derived from VerusHash 2.2b / Argon2d-light), which maintains an ultra-low memory footprint (~4MB–8MB per thread) with hardware AES/SHA instructions, allowing lightweight background mining on Android devices without thermal throttling or RAM exhaustion.

---

## 6. The Zero-Fee Challenge & Innovation

Every conventional blockchain (Bitcoin, Ethereum, Verus, Monero) relies on transaction fees ($F > 0$) as an anti-spam economic hurdle. Sardhan Chain mandates **Zero Transaction Fees**. Without transaction fees, an attacker could broadcast millions of transactions at zero financial cost, causing mempool saturation, network partition, and node disk exhaustion.

To solve this without sacrificing zero user fees, Sardhan Chain adopts a **Client-Side Proof-of-Work (Micro-PoW) & Dynamic Rate-Limiting Protocol**:
1. **Micro-PoW Nonce per Transaction:** Every transaction must include a cryptographic header with a valid nonce satisfying a dynamic difficulty target:
   $$\text{BLAKE3}(\text{TxData} \parallel \text{Nonce}) < \text{Target}_{\text{anti-spam}}$$
   On standard hardware, generating this takes 50–200 milliseconds of CPU time—imperceptible to a legitimate user sending a transaction, but computationally prohibitive ($10^6 \times 200\text{ms} = 55.5 \text{ hours}$ of continuous computing) for a spam bot.
2. **UTXO-Age & Nonce Multiplier:** Transactions spending UTXOs with greater confirmation depth require less computational micro-PoW, while rapid-fire broadcasts require exponentially higher micro-PoW.
3. **Deterministic Mempool Eviction:** If the mempool exceeds 64 MB, transactions are prioritized strictly by their client-side Micro-PoW difficulty density (hash difficulty per byte), dropping low-effort spam deterministically.

---

## 7. Conclusions for Sardhan Chain Architecture

1. **Consensus:** Sardhan Chain will implement a unified 50/50 Proof of Power (PoW + PoS) consensus engine.
2. **Algorithm:** SardhanHash optimized for x86-64 AES-NI and ARMv8 Cryptographic Extension, making Android mining a native first-class capability.
3. **Supply & Ledger:** Strict 21,000,000 SDR supply, UTXO ledger model, 0% premine, halving every 210,000 blocks.
4. **Anti-Spam:** Zero-fee architecture secured by client-side micro-PoW and peer-scoring rate limits.
5. **Licensing:** Permissive MIT License to ensure trustless, community-owned adoption.
