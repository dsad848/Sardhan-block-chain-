# Sardhan Chain (SDR) — Consensus Mathematics & Formal Proofs

**Document Version:** 2.0.0-MATH  
**Classification:** Core Mathematical Specification & Invariant Proofs  
**Status:** Authoritative Mathematical Specification  

---

## 1. Unified Proof of Power Cumulative Weight Formulation

### 1.1 Weight Definition
Let $\mathcal{C} = (B_0, B_1, \dots, B_n)$ be a valid blockchain candidate from genesis $B_0$ to tip $B_n$.  
The cumulative weight $\mathcal{W}(\mathcal{C})$ is defined as:

$$\mathcal{W}(\mathcal{C}) = \sum_{i=1}^{n} \Delta \mathcal{W}(B_i)$$

Where the incremental weight of block $B_i$ is:

$$\Delta \mathcal{W}(B_i) = \left\lfloor \frac{2^{256}}{\mathcal{T}_i + 1} \right\rfloor$$

- $\mathcal{T}_i \in [1, 2^{256} - 1]$ is the 256-bit target difficulty encoded in compact 32-bit `ConsensusBits`.
- For Proof-of-Work blocks ($B_i.\text{Type} = 0\text{x}01$), $\mathcal{T}_i = \mathcal{T}_{\text{PoW}}(i)$.
- For Proof-of-Stake blocks ($B_i.\text{Type} = 0\text{x}02$), $\mathcal{T}_i = \mathcal{T}_{\text{PoS}}(i)$.

### 1.2 Channel Balancing Sliding Window Invariant
To ensure that an attacker cannot monopolize chain growth through one channel (e.g. mining PoW blocks without holding coin stake, or forging PoS blocks without thermodynamic work), every valid block at height $h \ge 64$ must satisfy the **Channel Balancing Invariant**:

$$\forall h \ge 64: \quad 24 \le \sum_{j=0}^{63} \mathbb{I}(B_{h-j}.\text{ConsensusType} == 0\text{x}01) \le 40$$

Where $\mathbb{I}(\cdot)$ is the indicator function.  
If the number of PoW blocks in any contiguous 64-block window falls outside the range $[24, 40]$ (representing $37.5\%$ to $62.5\%$), the block is **unconditionally invalid** and rejected by consensus rules, regardless of accumulated weight.

---

## 2. Invariant Proof: Manipulation via Low-Difficulty Blocks

**Theorem 2.1:** *An attacker cannot overtake the canonical chain by artificially depressing target difficulty and producing a larger volume of blocks.*

**Proof:**  
Let the honest chain have average target difficulty $\mathcal{T}_{\text{honest}}$ and produce blocks at rate $\lambda = \frac{1}{T_{\text{target}}} = \frac{1}{150\text{ s}}$.  
In time $\Delta t$, the expected weight accumulated by the honest chain is:
$$\mathbb{E}[\mathcal{W}_{\text{honest}}(\Delta t)] = \frac{\Delta t}{T_{\text{target}}} \times \frac{2^{256}}{\mathcal{T}_{\text{honest}}}$$

Now assume an adversary attempts to mine an isolated private fork $\mathcal{C}_{\text{adv}}$.  
To reduce difficulty, the adversary must manipulate timestamps or exploit reduced competition so that the target difficulty expands to:
$$\mathcal{T}_{\text{adv}} = k \cdot \mathcal{T}_{\text{honest}} \quad (k > 1)$$

Under this inflated target $\mathcal{T}_{\text{adv}}$, finding a hash or staking kernel is $k$ times easier. The adversary's block production rate increases by a factor of $k$:
$$\lambda_{\text{adv}} = k \cdot \lambda_{\text{honest}} \cdot \alpha$$
Where $\alpha \in [0, 1]$ is the adversary's fraction of network resources.

However, each adversarial block now contributes a reduced weight:
$$\Delta \mathcal{W}(B^{\text{adv}}) = \frac{2^{256}}{\mathcal{T}_{\text{adv}}} = \frac{2^{256}}{k \cdot \mathcal{T}_{\text{honest}}} = \frac{1}{k} \Delta \mathcal{W}(B^{\text{honest}})$$

The expected weight accumulated by the adversary in time $\Delta t$ is:
$$\mathbb{E}[\mathcal{W}_{\text{adv}}(\Delta t)] = (\lambda_{\text{adv}} \cdot \Delta t) \times \Delta \mathcal{W}(B^{\text{adv}}) = (k \cdot \lambda_{\text{honest}} \cdot \alpha \cdot \Delta t) \times \left(\frac{1}{k} \cdot \frac{2^{256}}{\mathcal{T}_{\text{honest}}}\right)$$
$$\mathbb{E}[\mathcal{W}_{\text{adv}}(\Delta t)] = \alpha \times \left( \frac{\Delta t}{T_{\text{target}}} \cdot \frac{2^{256}}{\mathcal{T}_{\text{honest}}} \right) = \alpha \cdot \mathbb{E}[\mathcal{W}_{\text{honest}}(\Delta t)]$$

Notice that the factor $k$ cancels out completely.  
For any adversary with resource share $\alpha < 1.0$:
$$\mathbb{E}[\mathcal{W}_{\text{adv}}(\Delta t)] < \mathbb{E}[\mathcal{W}_{\text{honest}}(\Delta t)]$$

The adversary accumulates weight at a rate strictly proportional to their physical resource fraction $\alpha$, regardless of how low they drive the difficulty target. $\blacksquare$

---

## 3. Staking Kernel Probability Distribution

Let $V$ be the value of a mature UTXO in Sardhans ($1\text{ SDR} = 10^8\text{ Sardhans}$).  
Let $\mathcal{T}_{\text{PoS}}$ be the current 256-bit scalar staking target.  
At each 1-second timestamp step $t$, the staking kernel computes:
$$X = \text{BLAKE3}(\mathcal{M}_h \parallel \text{UTXO}.\text{TxID} \parallel \text{UTXO}.\text{Vout} \parallel t)$$
Where $X \sim \text{Uniform}(0, 2^{256} - 1)$.

The condition for minting a valid PoS block is:
$$X < \mathcal{T}_{\text{PoS}} \times V$$

The probability $p_i(t)$ of UTXO $i$ satisfying the kernel condition at second $t$ is:
$$p_i(t) = \frac{\mathcal{T}_{\text{PoS}} \times V_i}{2^{256}}$$

For a wallet holding $M$ mature UTXOs with total balance $S_{\text{wallet}} = \sum_{i=1}^{M} V_i$, the trials are independent:
$$P(\text{Wallet mints at second } t) = 1 - \prod_{i=1}^{M} (1 - p_i(t)) \approx \sum_{i=1}^{M} p_i(t) = \frac{\mathcal{T}_{\text{PoS}} \times S_{\text{wallet}}}{2^{256}}$$

The time $T$ until the wallet successfully mints a block follows a Geometric distribution (approximated as Exponential in continuous time):
$$\mathbb{E}[T_{\text{mint}}] = \frac{2^{256}}{\mathcal{T}_{\text{PoS}} \times S_{\text{wallet}}} \text{ seconds}$$

This proves that:
1. Expected time to stake is strictly inversely proportional to coin balance.
2. Staking yields are perfectly scale-invariant: a single 10,000 SDR UTXO has the exact same expected time to stake as ten 1,000 SDR UTXOs.

---

## 4. Probabilistic Finality Horizon

Let an honest transaction receive $k$ confirmations.  
Suppose an attacker controlling fraction $q = \frac{\alpha + \beta}{2} < 0.5$ attempts a double-spend reorg from $k$ blocks back.  
The probability $P(k)$ that the attacker ever succeeds in overtaking the honest chain is bounded by Nakamoto's random walk theorem:

$$P(k) = \sum_{m=0}^{\infty} \left( \frac{\lambda^m e^{-\lambda}}{m!} \right) \times \min\left(1, \left(\frac{q}{p}\right)^{\max(0, k - m)}\right)$$

Where $p = 1 - q > 0.5$ is the honest fraction, and $\lambda = k \frac{q}{p}$.

### Confirmation Threshold Table for $q = 0.30$ (Attacker controls 30% combined power):

| Confirmations ($k$) | Time Elapsed | Attack Success Probability $P(k)$ | Finality Status |
| :---: | :---: | :---: | :---: |
| **1 block** | 2.5 minutes | $0.2045$ ($20.45\%$) | Soft / Unconfirmed |
| **3 blocks** | 7.5 minutes | $0.0211$ ($2.11\%$) | Standard Retail |
| **6 blocks** | 15.0 minutes | $0.0006$ ($0.06\%$) | High-Value Settlement |
| **12 blocks** | 30.0 minutes | $0.0000004$ ($4 \times 10^{-7}$) | Exchange / Institutional |
| **$\ge 288$ blocks** | $\ge 12.0$ hours | **$0.0000000$ (Weak Subjectivity Bound)** | **Absolute Finality** |

Beyond 288 confirmations, the probability of reorganization is mathematically zero because full nodes enforce the Weak Subjectivity reorg limit.
