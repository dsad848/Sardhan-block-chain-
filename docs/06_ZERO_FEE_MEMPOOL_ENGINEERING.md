# Sardhan Chain (SDR) — Zero-Fee Mempool Engineering

**Document Version:** 1.0.0-RFC  
**Subject:** Deterministic Denial-of-Service Prevention in a Zero-Transaction-Fee Blockchain  
**Status:** Protocol Engineering Standard  

---

## 1. The Zero-Fee Paradox & Solution

In conventional blockchains (Bitcoin, Ethereum), transaction fees ($F > 0$) serve as the primary spam deterrent. If sending a transaction is free ($F = 0$), a malicious actor could generate millions of transactions per second, resulting in:
1. **Network Saturation:** P2P gossip links become overwhelmed by garbage transactions.
2. **Mempool RAM Exhaustion:** Node daemons crash due to Out-Of-Memory (OOM) exceptions.
3. **Ledger Bloat:** If miners include free spam transactions, the persistent blockchain disk footprint balloons uncontrollably.

### Sardhan Chain's Core Paradigm: Proof-of-Work As Currency for Bandwidth
Instead of spending financial capital ($SDR$), transaction senders spend **computational entropy** (client CPU cycles). This guarantees:
- **Zero Cost for Normal Users:** Sending 1 to 5 transactions per day costs a regular user $\approx 100\text{ms}$ of CPU time in the background—completely free and invisible.
- **Infinite Cost for Spammers:** Attempting to submit 1,000,000 spam transactions requires $>100,000$ seconds (~28 hours) of dedicated 8-core CPU computation.

---

## 2. Micro-PoW Specification & Algorithm

Every valid transaction $\text{Tx}$ has an 8-byte field:
`MicroPoWNonce: uint64`

The hash of the transaction is defined as:
$$\mathcal{H}_{\text{tx}} = \text{BLAKE3}\Big(\text{TxSerializeExceptNonce}(\text{Tx}) \parallel \text{MicroPoWNonce}\Big)$$

The transaction is accepted if and only if:
$$\mathcal{H}_{\text{tx}} < \mathcal{T}_{\text{mempool}}$$

Where $\mathcal{T}_{\text{mempool}}$ is the node's current acceptance threshold.

### 2.1 Dynamic Congestion Scaling Formula
Let $M_{\text{cur}}$ be the current mempool size in bytes, and $M_{\text{max}} = 67,108,864\text{ bytes}$ (64 MiB).

$$\mathcal{T}_{\text{mempool}}(M_{\text{cur}}) = \begin{cases}
\mathcal{T}_{\text{base}}, & \text{if } M_{\text{cur}} < 0.5 \times M_{\text{max}} \\
\mathcal{T}_{\text{base}} \times \left( \frac{M_{\text{max}} - M_{\text{cur}}}{0.5 \times M_{\text{max}}} \right)^2, & \text{if } 0.5 \times M_{\text{max}} \le M_{\text{cur}} < M_{\text{max}} \\
0, & \text{if } M_{\text{cur}} \ge M_{\text{max}} \quad \text{(Drop all new tx)}
\end{cases}$$

Notice that as $M_{\text{cur}} \to M_{\text{max}}$, $\mathcal{T}_{\text{mempool}}$ approaches zero, which means the difficulty target becomes exponentially harder. To get a transaction through during high load, the sender's client must grind significantly more hashes.

---

## 3. Mempool Priority & Block Packing

When miners construct a new block template:
1. Conventional PoW chains sort transactions by Satoshi fee per byte: $\text{FeeRate} = \frac{\text{Fee}}{\text{Size}}$.
2. **Sardhan Chain sorts transactions by Micro-PoW Density:**
   $$\text{Priority}(\text{Tx}) = \frac{\text{Difficulty}(\mathcal{H}_{\text{tx}})}{\text{ByteSize}(\text{Tx})}$$
   Where:
   $$\text{Difficulty}(\mathcal{H}_{\text{tx}}) = \frac{2^{256} - 1}{\mathcal{H}_{\text{tx}}}$$
3. Transactions with higher computational effort per serialized byte are prioritized first for block inclusion.

---

## 4. Dust Limit Invariant
To prevent creating millions of tiny UTXOs that bloat node RAM indexes:
- Minimum output value is strictly enforced:
  $$\forall \text{out} \in \text{Tx}.\text{Vout}: \text{out}.\text{Value} \ge 1,000\text{ Sardhans } (0.00001\text{ SDR})$$
- Transactions with outputs below 1,000 Sardhans are rejected at the deserialization stage as invalid syntax.
