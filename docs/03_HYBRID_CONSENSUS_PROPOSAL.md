# Sardhan Chain (SDR) — Hybrid Consensus Proposal

**Document Version:** 1.0.0-PROPOSAL  
**Consensus Name:** Sardhan Proof of Power (SDR-PoP)  
**Target:** Hybrid Proof of Work (CPU/ARM) + Proof of Stake (Community Validators)  
**Status:** Under Protocol Review  

---

## 1. Consensus Motivation & Design Goals

Pure Proof of Work and pure Proof of Stake each harbor fatal systemic vulnerabilities:
1. **Pure PoW Centralization & 51% Hash Vulnerability:** Small-to-medium PoW chains are routinely 51% attacked via hashpower rental or specialized ASIC cartels.
2. **Pure PoS Plutocracy & "Nothing-at-Stake":** Pure PoS chains concentrate control among early large token holders and lack an external physical cost anchor (energy/entropy), leading to costless fork generation and subjective long-range reorganization vulnerabilities.

**The Sardhan Proof of Power (SDR-PoP) Solution:**
Sardhan Chain couples Proof of Work and Proof of Stake into a single unified consensus architecture where:
- Every block is either a **PoW Block** (mined via CPU/ARM energy expenditure) or a **PoS Block** (minted via mature community stake).
- The network enforces a continuous target ratio of **50% PoW blocks and 50% PoS blocks** through dual, independent difficulty adjustment loops.
- Neither miners nor stakers can independently rewrite or reorganize the ledger: a reorg requires overwhelming both the physical hashrate **AND** the active coin stake.

---

## 2. Mathematical Definition of Consensus Channels

The blockchain ledger state $\mathcal{S}_h$ at block height $h$ progresses via one of two valid state transitions:

$$\text{ValidBlock}(B) = \begin{cases} 
\text{ValidPoW}(B), & \text{if } B.\text{ConsensusType} = 0\text{x}01 \\
\text{ValidPoS}(B), & \text{if } B.\text{ConsensusType} = 0\text{x}02 
\end{cases}$$

### 2.1 Channel 1: Proof of Work (CPU/ARM Mining)
A candidate PoW block header $H$ is valid if and only if:

$$\text{SardhanHash}(H) < \mathcal{T}_{\text{PoW}}(h)$$

Where:
- $\text{SardhanHash}$ is a memory-light, AES-NI & ARMv8 Cryptographic accelerated hashing function.
- $\mathcal{T}_{\text{PoW}}(h)$ is the 256-bit scalar target difficulty calculated by the PoW difficulty adjustment algorithm.
- $H.\text{ValidatorSignatureLength} = 0$ (no staking signature required).
- The coinbase transaction pays 100% of the block reward to the miner's specified address.

### 2.2 Channel 2: Proof of Stake (Community Validators)
A candidate PoS block is minted by finding an unspent output $\text{UTXO}_i$ in the validator's wallet that satisfies the **Staking Kernel Condition**:

$$\text{BLAKE3}(\mathcal{M}_h \parallel \text{UTXO}_i.\text{TxID} \parallel \text{UTXO}_i.\text{Vout} \parallel t) < \mathcal{T}_{\text{PoS}}(h) \times \text{UTXO}_i.\text{Value}$$

Where:
- $\mathcal{M}_h$ is the **Stake Modifier**, a deterministic pseudorandom entropy accumulator updated with every block:
  $$\mathcal{M}_h = \text{BLAKE3}(\mathcal{M}_{h-1} \parallel B_{h-1}.\text{Hash} \parallel B_{h-1}.\text{ConsensusType})$$
- $\text{UTXO}_i.\text{Value}$ is the coin balance in Sardhans. The higher the balance, the higher the probability of satisfying the inequality (linearly proportional to stake).
- $t$ is the current block timestamp, strictly restricted to 1-second intervals ($t > B_{h-1}.\text{Timestamp}$ and $t \le \text{CurrentTime} + 15$).
- $\text{UTXO}_i.\text{Confirmations} \ge 288$ (~12 hours of block depth).
- The validator signs the block header using the private key controlling $\text{UTXO}_i$:
  $$B.\text{ValidatorSignature} = \text{Ed25519-Sign}(\text{PrivKey}_i, \text{SerializeHeaderWithoutSig}(B))$$

---

## 3. Dual Difficulty Adjustment Algorithm (Dual-DAA)

Because PoW hashrate and PoS staking weight fluctuate independently, Sardhan Chain executes two independent difficulty adjustments, each targeting an average block interval of $300$ seconds (such that together they produce a block every $150$ seconds).

We adapt **DarkGravityWave v3 (DGWv3)** with an exponential smoothing window of $N = 144$ blocks per channel:

$$\mathcal{T}_{\text{channel}}(h) = \mathcal{T}_{\text{channel}}(h-1) \times \frac{\sum_{i=1}^{N} \Delta t_i}{N \times T_{\text{target}}}$$

Where:
- $T_{\text{target}} = 300\text{ seconds}$ for each respective channel.
- Max adjustment per step is bounded between $0.25 \times \mathcal{T}$ and $4.0 \times \mathcal{T}$ to prevent malicious timestamp attacks ("warp attacks").

---

## 4. Hardware Democratization: The SardhanHash Specification

### 4.1 Design Imperatives
1. **CPU & Mobile Native:** Must execute with peak energy efficiency on standard Intel/AMD x86-64 processors and ARM Cortex-A55 / A76 / A78 / X-series processors found in Android smartphones.
2. **ASIC & FPGA Resistance:** By heavily utilizing built-in hardware instructions (AES-NI on x86, ARMv8 Cryptography Extensions) and variable cache memory round-trips, custom ASICs gain no significant cost or performance multiplier over mass-produced consumer chips.
3. **Low Memory Footprint:** Requires $\le 8\text{ MiB}$ of scratchpad per thread, allowing background mining on Android devices without causing thermal distress, UI stutter, or OS out-of-memory kills.

### 4.2 Algorithm Pipeline
```
Input: 144-byte Block Header
  │
  ▼
Step 1: BLAKE3 Pre-Hash (Initial 256-bit digest)
  │
  ▼
Step 2: Haraka-512 / AES-NI S-Box Round Transformation
  - 16 rounds of hardware-accelerated AES encrypt/mix
  - Employs native _mm_aesenc_si128 (x86) / vaeseq_u8 (ARM)
  │
  ▼
Step 3: Cache-Line Scramble (8 MiB scratchpad indexing)
  - Memory-hard indexing using intermediate state as pointer
  │
  ▼
Step 4: Final BLAKE3 Squeeze -> 256-bit Proof-of-Work Output Hash
```

---

## 5. Genesis Block & Bootstrap Parameters

- **Genesis Timestamp:** `1772928000` (Deterministic protocol epoch).
- **Genesis Nonce:** Valid PoW solution at initial target difficulty `0x1e0ffff0`.
- **Genesis CoinBase Data:** `"Sardhan Chain: A Decentralized, Zero-Fee, Hybrid Sovereign Layer-1"`.
- **Genesis Reward:** `0 SDR` (Enforcing **Strict 0% Premine**).
- **Genesis Stake Modifier:** `0x0000000000000000000000000000000000000000000000000000000053445231`.
- **Block 1 to 288 (PoW Bootstrap):** Because initial coin distribution is zero, the first 288 blocks (~12 hours) must be mined exclusively via CPU PoW. After block 288, the first mined coins mature and PoS staking automatically activates.
