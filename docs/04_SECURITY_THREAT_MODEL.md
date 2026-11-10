# Sardhan Chain (SDR) — Security Threat Model

**Document Version:** 1.0.0  
**Phase:** Security Architecture & Threat Analysis  
**Classification:** Core Protocol Security Standard  
**Status:** Complete / Ready for Review  

---

## 1. Threat Matrix & Mitigation Summary

| Attack Vector | Threat Level | Target Subsystem | Sardhan Chain Mitigation Strategy |
| :--- | :--- | :--- | :--- |
| **51% PoW Hashrate Attack** | Critical | Consensus / Finality | **Immune:** PoW blocks alone cannot finalize without interleaved PoS validator signatures. Attacker requires $>50\%$ hashrate AND $>50\%$ of circulating stake. |
| **51% Staking Cartel Attack** | Critical | Consensus / Staking | **Immune:** Stakers cannot forge blocks without matching physical PoW solutions on alternate blocks. |
| **Zero-Fee Mempool DoS / Flood** | High | P2P / Mempool | **Mitigated:** Client-side Micro-PoW required for every transaction; dynamic difficulty scaling when mempool occupancy $\ge 50\%$; deterministic eviction. |
| **Nothing-at-Stake (PoS Forking)**| High | Consensus / Fork Choice | **Mitigated:** Physical PoW checkpointing; dual-signing slashing where validator coin outputs on conflicting branches are forfeit. |
| **Long-Range Reorg Attack** | High | Consensus / History | **Mitigated:** Dynamic Finality Checkpoints every 288 blocks (~12h); reorganization deeper than 288 blocks is strictly rejected by full nodes (Weak Subjectivity). |
| **Sybil Network Isolation** | Medium | P2P Overlay | **Mitigated:** Subnet bucketing (max 4 connections per `/16` IPv4 or `/32` IPv6); deterministic peer scoring and eviction of misbehaving peers. |
| **Eclipse Attack** | Medium | P2P Routing | **Mitigated:** Diverse anchor peers, hardcoded decentralized seed nodes, outbound connection pinning to multiple ASNs. |
| **Time-Warp Attack** | Medium | DAA / Retargeting | **Mitigated:** MedianTimePast(11) strict monotonicity; timestamps restricted within $[-15\text{s}, +120\text{s}]$ of host clock. |
| **Selfish Mining** | Medium | Mining / PoW | **Mitigated:** Unrevealed private PoW chains fall behind public PoS blocks; inability of miner to predict future stake modifiers $\mathcal{M}_h$. |
| **Key Extraction / Quantum Risk** | Low–Medium | Cryptography | **Mitigated:** High-security Ed25519 signatures; addresses use Bech32m hashes rather than raw public keys until spend time. |

---

## 2. In-Depth Threat Analysis

### 2.1 The 51% Attack Resistance Proof (Proof of Power)

In standard Nakamoto PoW (Bitcoin), an adversary with fraction $\alpha > 0.5$ of total network hashrate can reliably produce a chain faster than the honest network:
$$P(\text{adversary wins reorg}) = 1.0 \quad \text{as } k \to \infty$$

In Sardhan Chain's Unified Proof of Power:
Let:
- $\alpha \in [0, 1]$ be the adversary's fraction of global CPU/ARM mining hashrate.
- $\beta \in [0, 1]$ be the adversary's fraction of mature circulating SDR stake.

For an adversary to successfully reorganize a branch of length $k$:
They must mine the PoW blocks ($\approx k/2$) AND mint the PoS blocks ($\approx k/2$). The probability that the adversary produces both is bounded by:

$$P(\text{Adversary Chain Speed} > \text{Honest Chain Speed}) \implies \frac{\alpha \cdot \text{Rate}_{\text{PoW}} + \beta \cdot \text{Rate}_{\text{PoS}}}{\text{Target Rate}} > 1.0$$

Because $\text{Rate}_{\text{PoW}} = \text{Rate}_{\text{PoS}} = 0.5$:
$$\alpha + \beta > 1.0$$

Furthermore, because PoW blocks and PoS blocks alternate or interleave, an adversary with $\alpha = 0.9$ (90% hashrate) but $\beta = 0.05$ (5% stake) will inevitably stall on every PoS block slot, because their likelihood of minting the PoS kernel is only 5%. Meanwhile, the 95% honest stakers will mint the honest chain, rendering the attacker's 90% hashrate useless for deep reorganization.

$$\text{Minimum Attacker Cost} = \text{CapEx}_{\text{Hash}}(\alpha \ge 0.5) + \text{CapEx}_{\text{Stake}}(\beta \ge 0.5)$$

This dual requirement makes attacking Sardhan Chain orders of magnitude more expensive than any single-consensus network.

---

### 2.2 Zero-Fee Spam & Mempool Exhaustion Defense

Because users pay $0\text{ SDR}$ in transaction fees, conventional economic fee-rate prioritization cannot protect nodes from malicious flooding. Sardhan Chain implements a four-tier defense-in-depth architecture:

#### Tier 1: Client-Side Micro-PoW
Every transaction must supply an 8-byte nonce such that:
$$\text{BLAKE3}(\text{TxBytes} \parallel \text{Nonce}) < \text{Target}_{\text{anti-spam}}$$
- Cost per honest transaction: $\approx 100\text{ms}$ CPU time.
- Cost to flood network with 100,000 spam transactions:
  $$100,000 \times 0.1\text{s} = 10,000\text{ CPU-seconds} \approx 2.77\text{ hours of 100\% 8-core CPU maxout}$$
- While the spammer burns substantial electrical energy, nodes verify each transaction's BLAKE3 hash in $<1\text{ microsecond}$ (a $100,000:1$ verification asymmetry in favor of the network).

#### Tier 2: Dynamic Congestion Target Scaling
If a node's mempool memory usage exceeds 32 MiB (50% capacity):
$$\text{RequiredTarget}_{\text{dynamic}} = \text{Target}_{\text{base}} \times \left(1 - \frac{\text{CurrentBytes}}{67,108,864}\right)^2$$
The micro-PoW difficulty scales quadratically. As spam increases, the computational cost to inject additional transactions explodes, choking off the attacker.

#### Tier 3: Peer Reputation & Subnet Throttling
- Nodes track peer unconfirmed transaction relay rates.
- Any peer transmitting more than 10 unconfirmed tx/second is rate-limited.
- Any peer transmitting transactions with invalid micro-PoW is banned for 24 hours.

#### Tier 4: Dust Limit Output Enforcement
To prevent "dust attacks" (bloating the persistent UTXO database with microscopic balances), all transaction outputs must be $\ge 1,000\text{ Sardhans}$ ($0.00001\text{ SDR}$).

---

### 2.3 Nothing-at-Stake & Fork-Choice Rule

In pure PoS, validators vote on every available fork because voting costs zero energy.
Sardhan Chain eliminates this through:
1. **Hybrid Checkpointing:** A PoS block can only build on a valid prior block (which is frequently a PoW block backed by real thermodynamic energy).
2. **Dual-Sign Slashing Proof:** If a validator signs two conflicting PoS block headers at the same block height:
   $$\text{Header}_A \neq \text{Header}_B \quad\text{with}\quad \text{Height}_A = \text{Height}_B$$
   Any peer can submit a `SlashingEvidence` transaction. The network confiscates the validator's mature staking UTXO, burning 50% of the coins and paying 50% to the reporter.

---

### 2.4 Long-Range Attack & Weak Subjectivity

An attacker who purchases abandoned private keys from years past might attempt to generate an alternate long chain from the genesis block.
**Mitigation:**
- Sardhan Chain enforces **Weak Subjectivity** with a maximum reorganization depth of $\Omega = 288$ blocks (~12 hours).
- Any reorganization attempt that forks from a height $h < h_{\text{best}} - 288$ is unconditionally rejected by all full nodes without processing.
- New nodes synchronizing from scratch bootstrap using trusted genesis and hardcoded release checkpoints.
