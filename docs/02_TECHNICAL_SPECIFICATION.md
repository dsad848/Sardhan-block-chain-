# Sardhan Chain (SDR) — Technical Specification

**Version:** 1.0.0-RFC  
**Phase:** Phase 2 (Complete Technical Specification)  
**Status:** Complete / Proposed for Architectural Sign-Off  
**Author:** Sardhan Core Protocol Engineering Group  

---

## 1. Protocol Constants & Global Invariants

| Parameter | Value | Definition / Invariant Rationale |
| :--- | :--- | :--- |
| **Network Identifier** | `sdr-mainnet` / `sdr-testnet` | Magic byte domain separator |
| **Ticker Symbol** | **SDR** | Native currency unit |
| **Smallest Divisible Unit** | **1 Sardhan = $10^{-8}$ SDR** | 8 decimal places ($100,000,000$ Sardhans per 1 SDR) |
| **Maximum Total Supply** | **21,000,000.00000000 SDR** | Strict invariant: $\sum_{h=0}^{\infty} \text{Reward}(h) \le 21 \times 10^{14}$ Sardhans |
| **Premine / Founder Pool** | **0.00000000 SDR (0%)** | Genesis block contains no circulating tokens |
| **Target Block Interval** | **150 seconds (2.5 minutes)** | Balances fast confirmation latency with low orphan rate |
| **Blocks Per Day** | **576 blocks / day** | $86400 / 150 = 576$ blocks daily |
| **Halving Epoch Interval** | **210,000 blocks** | Exactly $364.58$ days (~1 year per epoch) |
| **Initial Base Block Reward** | **50.00000000 SDR** | Split between PoW miner and PoS staker |
| **Transaction Fees** | **0.00000000 SDR** | Protocol-enforced fee = 0; anti-spam via client micro-PoW |
| **Max Block Payload Size** | **2,097,152 bytes (2 MiB)** | Maximum serialization size of valid block |
| **Max Mempool Size** | **67,108,864 bytes (64 MiB)** | Eviction by micro-PoW density when exceeded |
| **Min Stake Confirmation Age**| **288 blocks (~12 hours)** | UTXO must be 288 blocks deep before eligible for staking |
| **Max Stake Coin Age** | **17,280 blocks (~30 days)** | Prevents long-offline hoarded coins from dominating |
| **Address Prefix** | `sdr1` (Bech32m) | Human-readable Bech32m checksummed addresses |

---

## 2. Emission Schedule & Halving Mechanics

The total supply of 21,000,000 SDR is emitted strictly through block rewards according to a geometric halving decay function:

$$\text{BlockReward}(h) = \left\lfloor \frac{50 \times 10^8}{2^{\lfloor h / 210,000 \rfloor}} \right\rfloor \quad \text{Sardhans}$$

For any block height $h \ge 210,000 \times 33 = 6,930,000$, $\text{BlockReward}(h) = 0$.

### 2.1 Reward Allocation per Block
In Sardhan Chain's Unified Proof of Power architecture:
- **Proof-of-Work Blocks (50%):** Mined via CPU/ARM hardware. Reward = $100\%$ of $\text{BlockReward}(h)$ awarded to the miner's coinbase output.
- **Proof-of-Stake Blocks (50%):** Minted by a staking validator who signs a coinstake kernel transaction. Reward = $100\%$ of $\text{BlockReward}(h)$ awarded to the coinstake output.
- Over time, total minted coins partition equally: **10,500,000 SDR to PoW CPU miners** and **10,500,000 SDR to PoS community stakers**.

### 2.2 Epoch Breakdown

| Epoch | Block Range | Reward per Block | Epoch Total Minted | Cumulative SDR Minted | % of Max Cap |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **1** | $0 \dots 209,999$ | **50.0 SDR** | 10,500,000 SDR | 10,500,000 SDR | 50.00% |
| **2** | $210,000 \dots 419,999$ | **25.0 SDR** | 5,250,000 SDR | 15,750,000 SDR | 75.00% |
| **3** | $420,000 \dots 629,999$ | **12.5 SDR** | 2,625,000 SDR | 18,375,000 SDR | 87.50% |
| **4** | $630,000 \dots 839,999$ | **6.25 SDR** | 1,312,500 SDR | 19,687,500 SDR | 93.75% |
| **5** | $840,000 \dots 1,049,999$ | **3.125 SDR** | 656,250 SDR | 20,343,750 SDR | 96.88% |
| **6** | $1,050,000 \dots 1,259,999$ | **1.5625 SDR** | 328,125 SDR | 20,671,875 SDR | 98.44% |
| **7** | $1,260,000 \dots 1,469,999$ | **0.78125 SDR** | 164,062.5 SDR | 20,835,937.5 SDR | 99.22% |
| $\dots$ | $\dots$ | $\dots$ | $\dots$ | $\dots$ | $\dots$ |
| **$\ge 33$** | $\ge 6,930,000$ | **0.00000000 SDR** | 0 SDR | **21,000,000.0 SDR** | **100.00%** |

---

## 3. Block Structure & Binary Wire Framing

A Sardhan Chain block consists of an 144-byte Block Header followed by a variable-length transaction list.

### 3.1 Block Header Binary Layout (144 bytes)

```
+-----------------------------------------------------------------------+
| Version (uint32, 4 bytes)                                             |
+-----------------------------------------------------------------------+
| PrevBlockHash (uint256, 32 bytes)                                     |
+-----------------------------------------------------------------------+
| MerkleRoot (uint256, 32 bytes)                                        |
+-----------------------------------------------------------------------+
| Timestamp (uint64, 8 bytes, Unix epoch seconds)                       |
+-----------------------------------------------------------------------+
| BlockHeight (uint64, 8 bytes)                                         |
+-----------------------------------------------------------------------+
| ConsensusBits (uint32, 4 bytes, compact target difficulty)            |
+-----------------------------------------------------------------------+
| ConsensusType (uint8, 1 byte: 0x01 = PoW Block, 0x02 = PoS Block)    |
+-----------------------------------------------------------------------+
| StakeModifier (uint256, 32 bytes, entropy accumulator for PoS)        |
+-----------------------------------------------------------------------+
| Nonce (uint64, 8 bytes, PoW miner search nonce)                       |
+-----------------------------------------------------------------------+
| ValidatorSignatureLength (uint8, 1 byte: 0x00 for PoW, 0x40 for PoS) |
+-----------------------------------------------------------------------+
| ValidatorSignature (0 or 64 bytes: Ed25519 signature for PoS blocks)  |
+-----------------------------------------------------------------------+
```

### 3.2 Block Verification Pipeline
When a node receives a block $B$:
1. Check size: $\text{len}(\text{Serialize}(B)) \le 2,097,152 \text{ bytes}$.
2. Check timestamp: $B.\text{Timestamp} > \text{MedianTimePast}(11) \quad \text{and} \quad B.\text{Timestamp} \le \text{CurrentTime} + 120\text{s}$.
3. Verify Merkle Root matches the root of all transactions:
   $$\text{MerkleRoot} = \text{ComputeMerkle}(\text{tx}_0, \text{tx}_1, \dots, \text{tx}_{N-1})$$
4. **If ConsensusType == 0x01 (PoW):**
   - Check that $\text{SardhanHash}(\text{Header}) < \text{CompactToTarget}(B.\text{ConsensusBits})$.
   - First transaction must be a valid Coinbase transaction (no inputs, output $\le \text{BlockReward}(B.\text{Height})$).
5. **If ConsensusType == 0x02 (PoS):**
   - First transaction must be an empty dummy input; second transaction must be a **CoinStake** transaction.
   - CoinStake spends a valid mature UTXO ($\ge 288$ confirmations).
   - Check Staking Kernel condition:
     $$\text{BLAKE3}(B.\text{StakeModifier} \parallel \text{UTXO}.\text{TxID} \parallel \text{UTXO}.\text{Index} \parallel B.\text{Timestamp}) < \text{Target}_{\text{PoS}} \times \text{UTXO}.\text{Value}$$
   - Verify `ValidatorSignature` using the public key in the spent UTXO.
6. Verify all transactions, prevent double spends, and update the UTXO ledger.

---

## 4. Transaction Architecture & Zero-Fee Anti-Spam (Micro-PoW)

Sardhan Chain uses an unspent transaction output (UTXO) model. Because transaction fees are zero, every transaction embeds a client-side **Micro-PoW Header** to prevent spam.

### 4.1 Transaction Serialization Format

```
+-----------------------------------------------------------------------+
| Version (uint32, 4 bytes)                                             |
+-----------------------------------------------------------------------+
| MicroPoWNonce (uint64, 8 bytes, client anti-spam proof)               |
+-----------------------------------------------------------------------+
| LockTime (uint32, 4 bytes)                                            |
+-----------------------------------------------------------------------+
| TxIn Count (CompactSize uint)                                         |
|   - PrevTxHash (32 bytes)                                             |
|   - PrevTxIndex (uint32, 4 bytes)                                     |
|   - ScriptSig / Signature Witness (variable bytes)                    |
|   - Sequence (uint32, 4 bytes)                                        |
+-----------------------------------------------------------------------+
| TxOut Count (CompactSize uint)                                        |
|   - Value (uint64, 8 bytes in Sardhans)                               |
|   - ScriptPubKey / Recipient Lock (P2PKH Ed25519)                     |
+-----------------------------------------------------------------------+
```

### 4.2 Anti-Spam Micro-PoW Specification
A transaction is valid for mempool acceptance and block inclusion if and only if:

$$\text{TxHash}(\text{Tx}) = \text{BLAKE3}(\text{TxData} \parallel \text{MicroPoWNonce})$$
$$\text{TxHash}(\text{Tx}) < \text{AntiSpamTarget}$$

- Baseline anti-spam difficulty requires on average $\approx 2^{16}$ to $2^{18}$ BLAKE3 hash calculations (~50–200 milliseconds on a single CPU/ARM core).
- **Dynamic Congestion Scaling:** If a node's mempool fills above 50% capacity (32 MiB), the node dynamically raises its required `AntiSpamTarget` according to:
  $$\text{RequiredTarget} = \text{BaseTarget} \times \left(1 - \frac{\text{MempoolSize}}{\text{MaxMempoolSize}}\right)^2$$
- Transactions below the dynamic target are dropped or rejected at the P2P border without propagation.

---

## 5. Cryptographic Primitives

1. **Signatures & Keys:**
   - **Ed25519 (RFC 8032):** High-speed, constant-time, collision-resistant signature scheme. Public keys are 32 bytes; signatures are 64 bytes.
   - Address format: Bech32m checksummed with prefix `sdr1` (e.g. `sdr1q7x4n...`).
2. **Fast Hashing & Merkle Trees:**
   - **BLAKE3:** 256-bit cryptographic digest. 4x to 10x faster than SHA-256 and MD5, tree-hashable, impervious to length-extension attacks. Used for TxIDs, Merkle trees, and anti-spam micro-PoW.
3. **Proof-of-Work Hash Function (SardhanHash):**
   - Memory-light (~8 MiB) AES-NI & ARMv8-accelerated hash pipeline. Prevents ASIC specialization while running natively on x86-64 and ARMv8 Android architectures.

---

## 6. Peer-to-Peer Protocol

- **Default Port:** `9333` (Mainnet), `19333` (Testnet).
- **Protocol Magic Bytes:**
  - Mainnet: `0x53 0x44 0x52 0x01` ("SDR\x01")
  - Testnet: `0x53 0x44 0x52 0xF1` ("SDR\xF1")
- **Framing Structure:**
  - Magic (4 bytes) + Command (12 bytes ASCII null-padded) + PayloadLength (4 bytes uint32) + Checksum (4 bytes, first 4 bytes of BLAKE3(Payload)) + Payload.
- **Core Messages:**
  - `version`, `verack`: Connection negotiation and capabilities.
  - `addr`, `getaddr`: Decentralized peer gossip.
  - `inv`, `getdata`: Inventory advertisement for blocks and transactions.
  - `tx`: Broadcast validated zero-fee transactions with micro-PoW.
  - `block`, `getblocks`, `getheaders`: Blockchain synchronization (headers-first).
  - `reject`: Malformed or invalid block/transaction notification.

---

## 7. JSON-RPC 2.0 Node Interface

Standard JSON-RPC over HTTP/Unix Domain Socket on port `9332`:
- `getblockchaininfo`: Returns chain height, best block hash, PoW difficulty, PoS difficulty, halving epoch, total circulating supply.
- `getblock <hash|height> [verbosity]`: Returns decoded block header, transactions, and consensus metadata.
- `getrawtransaction <txid>`: Decodes raw UTXO inputs and outputs.
- `sendrawtransaction <hexstring>`: Validates micro-PoW and relays transaction to network mempool.
- `getmininginfo`: Reports PoW hashrate estimate, difficulty, target, and mining status.
- `getstakinginfo`: Reports mature staking balance, PoS difficulty, estimated time to next stake block.
- `getmempoolinfo`: Size, byte count, and current dynamic micro-PoW difficulty threshold.
- `validateaddress <address>`: Validates Bech32m checksum and decodes Ed25519 public key hash.
