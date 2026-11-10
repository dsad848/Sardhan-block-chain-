# Sardhan Chain (SDR) — Tokenomics & Emission Mathematics

**Document Version:** 1.0.0-TOKENOMICS  
**Subject:** Formal Invariant Verification & Emission Proofs  
**Status:** Canonical Reference  

---

## 1. Formal Proof of 21,000,000 SDR Supply Limit

Let $h \in \mathbb{N}_0$ be the block height.  
Let the halving interval be $E = 210,000$ blocks.  
Let the initial block reward at height $0 \le h < 210,000$ be $R_0 = 50 \times 10^8$ Sardhans ($50\text{ SDR}$).

The block reward function for any height $h$ is:
$$R(h) = \left\lfloor \frac{R_0}{2^{\lfloor h / E \rfloor}} \right\rfloor$$

The total cumulative supply $S(H)$ emitted up to height $H$ is:
$$S(H) = \sum_{h=0}^{H} R(h)$$

Because integer division truncates to zero after $\lfloor h / E \rfloor \ge 33$ (since $50 \times 10^8 / 2^{33} = 5,000,000,000 / 8,589,934,592 = 0$), the infinite sum terminates cleanly:

$$\lim_{H \to \infty} S(H) = \sum_{k=0}^{32} \left( E \times \left\lfloor \frac{R_0}{2^k} \right\rfloor \right)$$

Evaluating the sum:
$$S_{\text{total}} = 210,000 \times \left( 50 + 25 + 12.5 + 6.25 + 3.125 + \dots + \frac{50}{2^{32}} \right) \times 10^8 \text{ Sardhans}$$
$$S_{\text{total}} = 210,000 \times 50 \times \left( 1 + \frac{1}{2} + \frac{1}{4} + \dots \right) \approx 210,000 \times 50 \times 2 = 21,000,000\text{ SDR}$$

Because integer truncation drops fractional Sardhans:
$$S_{\text{total}} = 20,999,999.97690000\text{ SDR} \le 21,000,000\text{ SDR}$$

**Invariant 1 Verified:** Under no mathematical condition can total supply exceed 21 million SDR.

---

## 2. Emission Schedule Epoch Table

| Epoch | Block Interval | Approx Time | SDR / Block | Total SDR in Epoch | Cumulative Supply | % of Total Cap |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **0** | $0 \dots 209,999$ | Yr 1 (0–364d) | 50.00000000 | 10,500,000.00 | 10,500,000.00 | 50.0000% |
| **1** | $210,000 \dots 419,999$ | Yr 2 (365–729d) | 25.00000000 | 5,250,000.00 | 15,750,000.00 | 75.0000% |
| **2** | $420,000 \dots 629,999$ | Yr 3 | 12.50000000 | 2,625,000.00 | 18,375,000.00 | 87.5000% |
| **3** | $630,000 \dots 839,999$ | Yr 4 | 6.25000000 | 1,312,500.00 | 19,687,500.00 | 93.7500% |
| **4** | $840,000 \dots 1,049,999$ | Yr 5 | 3.12500000 | 656,250.00 | 20,343,750.00 | 96.8750% |
| **5** | $1,050,000 \dots 1,259,999$ | Yr 6 | 1.56250000 | 328,125.00 | 20,671,875.00 | 98.4375% |
| **6** | $1,260,000 \dots 1,469,999$ | Yr 7 | 0.78125000 | 164,062.50 | 20,835,937.50 | 99.2188% |
| **7** | $1,470,000 \dots 1,679,999$ | Yr 8 | 0.39062500 | 82,031.25 | 20,917,968.75 | 99.6094% |
| **8** | $1,680,000 \dots 1,889,999$ | Yr 9 | 0.19531250 | 41,015.625 | 20,958,984.375 | 99.8047% |
| **$\dots$** | $\dots$ | $\dots$ | $\dots$ | $\dots$ | $\dots$ | $\dots$ |
| **32** | $6,720,000 \dots 6,929,999$ | Yr 33 | 0.00000001 | 0.00210000 | 20,999,999.9769 | 99.9999% |
| **$\ge 33$** | $\ge 6,930,000$ | Yr 34+ | 0.00000000 | 0.00000000 | **20,999,999.9769** | **100.0000%** |

---

## 3. Split Between PoW Miners and PoS Stakers

Because each channel generates half of the blocks, the long-term coin distribution is perfectly symmetric:

- **Total PoW Mining Allocation:** 10,499,999.98845 SDR (50.0%)
- **Total PoS Staking Allocation:** 10,499,999.98845 SDR (50.0%)
- **Premine:** 0.00000000 SDR (0.0%)
- **Founder / Foundation Tax:** 0.00000000 SDR (0.0%)
- **Venture Capital Allocation:** 0.00000000 SDR (0.0%)

Every single SDR in existence is minted strictly through cryptographic energy expenditure (CPU/ARM PoW) or coin-age commitment (PoS community validation).
