# Sardhan Chain (SDR) — Revised Technical Specification (v2.0-Audit-Ratified)

**Document Version:** 2.0.0-PROPOSED-RATIFICATION  
**Predecessor:** RFC-102 (v1.0.0-RFC)  
**Security Status:** Hardened Against Audit SEC-CRIT-01 through SEC-MED-04  
**Author:** Sardhan Core Security & Protocol Architecture Working Group  

---

## 1. Summary of Protocol Hardening Changes

| Feature | RFC-102 (v1.0) | RFC-102 (v2.0 Ratified) | Security Rationale |
| :--- | :--- | :--- | :--- |
| **Block Header Size** | Mismatched (claimed 144B, actual 130B/194B) | **Fixed 196 bytes** | Eliminates deserialization panic (SEC-MED-01) |
| **Micro-PoW Binding** | Static ($\text{BLAKE3}(\text{TxData} \parallel \text{Nonce})$) | **Block-Bound ($\text{BLAKE3}(\text{RecentBlock} \parallel \dots)$)** | Defeats offline botnet pre-mining (SEC-CRIT-01) |
| **Cross-Chain Replay** | None | **Mandatory 32-bit `ChainID` in Preimage** | Prevents Testnet/Mainnet replay theft (SEC-CRIT-02) |
| **Stake Entropy** | Immediate parent block hash ($H-1$) | **Lagging Stake Modifier ($H-100$)** | Prevents stake grinding attacks (SEC-CRIT-03) |
| **Fork Choice Rule** | Undefined ("50/50 balance") | **Unified Weight $\sum (2^{256} / \mathcal{T})$ + Window Bound** | Resolves chain splits deterministically (SEC-HIGH-01) |
| **Signature Safety** | Unspecified Ed25519 canonical checks | **Strict $S < L$ check & Segregated Witness** | Prevents TxID malleability (SEC-HIGH-03) |
| **PoS Sync Security** | Unverified state dependency | **Compact UTXO Merkle Proof in Header** | Prevents PoS header sync flood DoS (SEC-HIGH-04) |

---

## 2. Revised Block Header Binary Layout (Strictly 196 Bytes Fixed)

```
Offset (Bytes) | Field Name            | Type     | Description
---------------+-----------------------+----------+---------------------------------------------------
0x00 - 0x03    | Version               | uint32   | Block protocol version (0x00000002)
0x04 - 0x23    | PrevBlockHash         | uint256  | 32-byte hash of previous canonical block header
0x24 - 0x43    | MerkleRoot            | uint256  | 32-byte BLAKE3 root of transaction IDs
0x44 - 0x4B    | Timestamp             | uint64   | Unix epoch timestamp (seconds)
0x4C - 0x53    | BlockHeight           | uint64   | Strict sequential block height
0x54 - 0x57    | ConsensusBits         | uint32   | Compact representation of target difficulty
0x58           | ConsensusType         | uint8    | 0x01 = PoW Block, 0x02 = PoS Block
0x59 - 0x78    | LaggingStakeModifier  | uint256  | Entropy derived from block (Height - 100)
0x79 - 0x80    | PoWNonce              | uint64   | Miner search nonce (Used in PoW, zeroed in PoS)
0x81 - 0x83    | ReservedFlags         | uint24   | Reserved alignment flags (0x000000)
0x84 - 0xC3    | ValidatorSignature    | uint512  | 64-byte Ed25519 signature (Zeroed in PoW blocks)
---------------+-----------------------+----------+---------------------------------------------------
Total Size: Exactly 196 bytes (Little-Endian integer encoding, Big-Endian hash displays)
```

---

## 3. Revised Transaction Wire Format & Block-Bound Micro-PoW

### 3.1 Binary Structure
```
Field Name            | Type         | Size (Bytes) | Description
----------------------+--------------+--------------+---------------------------------------------------
ChainID               | uint32       | 4            | 0x53445201 (Mainnet) or 0x53445202 (Testnet)
Version               | uint32       | 4            | Transaction format version
RecentBlockHash       | uint256      | 32           | Must be within [BestHeight - 16, BestHeight]
MicroPoWNonce         | uint64       | 8            | Proof-of-work anti-spam nonce
LockTime              | uint32       | 4            | Block height or Unix timestamp lock
TxIn Count            | CompactSize  | 1 - 9        | Number of inputs
  TxIn[i]             | Input Struct | Variable     | PrevTxID, VoutIndex, Sequence (No signatures!)
TxOut Count           | CompactSize  | 1 - 9        | Number of outputs
  TxOut[i]            | Output Struct| Variable     | Value (Sardhans >= 1000), ScriptPubKey
Witness Count         | CompactSize  | 1 - 9        | Number of witness signature vectors
  Witness[i]          | Witness Data | 64 + 32      | Ed25519 Sig (S < L enforced) + 32B Public Key
```

### 3.2 Block-Bound Anti-Spam Condition
A node will accept a zero-fee transaction into its mempool if and only if:
1. `RecentBlockHash` is present in the node's best active chain at height $H_{\text{recent}}$ such that:
   $$H_{\text{best}} - 16 \le H_{\text{recent}} \le H_{\text{best}}$$
2. The transaction hash satisfies the dynamic congestion threshold:
   $$\text{BLAKE3}\Big(\text{RecentBlockHash} \parallel \text{SerializeTxPayload}(\text{Tx}) \parallel \text{MicroPoWNonce}\Big) < \mathcal{T}_{\text{mempool}}$$
3. Every signature witness $(R, S)$ satisfies strict canonical bounds:
   $$S < 2^{252} + 27742317777372353535851937790883648493$$

---

## 4. Formal Proof of Power Fork-Choice Rule

When a node discovers two competing chain tips $\mathcal{C}_A$ and $\mathcal{C}_B$:
1. The node computes the **Cumulative Proof-of-Power Weight**:
   $$\mathcal{W}(\mathcal{C}) = \sum_{B \in \mathcal{C}} \frac{2^{256}}{\text{CompactToTarget}(B.\text{ConsensusBits})}$$
2. The node verifies the **Channel Interleaving Invariant**:
   For every sliding window of 32 blocks:
   $$8 \le \sum_{i=0}^{31} \mathbb{I}(B_{h-i}.\text{Type} == \text{PoW}) \le 24$$
3. The valid chain with strictly higher $\mathcal{W}(\mathcal{C})$ is accepted as the canonical tip. In the event of an exact weight tie, the branch with the lower timestamp on the tip block prevails.
