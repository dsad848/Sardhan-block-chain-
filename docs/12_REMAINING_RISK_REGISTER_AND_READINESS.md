# Sardhan Chain (SDR) — Remaining Risk Register & Implementation Readiness

**Document Version:** 2.0.0-RISK-ASSESSMENT  
**Classification:** Formal Protocol Security Risk Audit  
**Auditing Group:** Independent Blockchain Security Auditing Team  
**Status:** Canonical Security Baseline  

---

## 1. Exhaustive Remaining Risk Register

The second security audit has thoroughly examined all protocol domains. Below is the honest, unvarnished disclosure of all remaining structural risks and operational trade-offs inherent in the Sardhan Chain design.

| Risk ID | Vulnerability Domain | Description | Severity | Likelihood | Residual Mitigation Strategy |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **RISK-01** | Bootstrap Hashrate | **Low Initial Hashrate Vulnerability (Blocks 0–288):** During the first 12 hours before coin maturity activates PoS, the chain operates as pure PoW. A malicious actor with modest CPU/cloud power could reorganise early blocks. | High | Medium | Coordinate community miner launch with hardcoded genesis and trusted bootstrap seed nodes. |
| **RISK-02** | Mobile Architecture | **Android OS Doze Mode & Battery Management:** When an Android device is unplugged or screen is off, aggressive OS power-saving kills background tasks, halting mobile mining threads. | Medium | High | Require foreground service with sticky notification (`FOREGROUND_SERVICE_DATA_SYNC`), pause during battery < 20%, and encourage overnight plugged-in mining. |
| **RISK-03** | Long-Term Economics | **Zero-Fee Security Budget Post-Epoch 10:** In Year 10+, block rewards drop to negligible amounts. Because transaction fees are zero, validators have no transaction fee income. | High | Low (Year 10+) | Formal governance choice between a perpetual 0.05 SDR baseline tail emission or inactive UTXO state demurrage recycling. |
| **RISK-04** | Network Partitions | **Partition $> 288$ Blocks Requires Manual Reconciliation:** If a catastrophic continental internet severance splits the network for $> 12$ hours, both sides finalize their chains past the weak subjectivity boundary. | High | Low | By design, weak subjectivity prevents automated deep reorgs. Reconnection after multi-day partitions requires manual validator social consensus. |
| **RISK-05** | Mempool Memory | **Local Mempool Desynchronization under Extreme Spikes:** Under massive network congestion, different nodes may calculate slightly different dynamic micro-PoW thresholds $\mathcal{T}_{\text{mempool}}$ due to arrival time jitter. | Medium | Medium | The P2P protocol allows a 15% difficulty tolerance window before peer disconnection, and miners pack blocks strictly by micro-PoW density. |
| **RISK-06** | Staking Pooling | **Staking Pool Centralization:** Stakers with small balances may be tempted to pool custody with custodial services, leading to custodial validator concentration. | Medium | Medium | Direct in-wallet UTXO staking with zero lockup penalty makes solo staking viable even for 100 SDR balances. |

---

## 2. Invariant Checklist: Resolved vs Gated

- [x] **21,000,000 SDR Cap Invariant:** Mathematically verified via geometric series sum ($\sum R_k = 20,999,999.9769\text{ SDR} \le 21\text{M}$).
- [x] **0% Premine Invariant:** Genesis block contains 0 SDR reward and zero pre-allocated outputs.
- [x] **Fixed Binary Header Invariant:** Canonical 196-byte header fixed length verified.
- [x] **Cross-Chain Replay Invariant:** 32-bit `ChainID` enforced in transaction payload and signature preimage.
- [x] **Anti-Botnet Invariant:** Block-bound micro-PoW enforces 16-block validity window, eliminating offline pre-mining.
- [x] **Stake Grinding Invariant:** 100-block lagging stake modifier completely decouples parent block hash manipulation from current staking lotteries.
- [x] **Low-Difficulty Manipulation Invariant:** Unified cumulative weight $\mathcal{W}(\mathcal{C}) = \sum (2^{256} / \mathcal{T})$ cancels out low-difficulty volume manipulation.
- [ ] **Mobile Background Runtime Limits:** Requires native Android foreground service wrapper during Phase 7 (Wallet/Miner development).
- [ ] **Mainnet Long-Term Tail Emission Ratification:** To be ratified via community vote prior to Epoch 5.

---

## 3. Final Implementation-Readiness Recommendation

Based on the completion of the second independent security audit and the mathematical proofs provided:

1. **PHASE 3 (Repository Setup & Core Cryptographic Foundation):**
   - **RECOMMENDATION: CONDITIONALLY APPROVED FOR COMMENCEMENT.**
   - The team is cleared to build the monorepo workspace, setup `packages/crypto` (Ed25519, BLAKE3, SardhanHash, Bech32m), and `packages/primitives` (196-byte Header, Segregated Transaction format).
   - Automated unit test vectors (`TEST-SEC-01` through `TEST-SEC-12`) must be implemented alongside each package.

2. **PHASE 4 & 5 (Consensus Engine & P2P Wire Protocol):**
   - **RECOMMENDATION: GATED PENDING FOUNDATION TESTS.**
   - Do not activate consensus block production until all cryptographic and primitive unit tests achieve 100% pass rate.

3. **PHASE 8 (Private Testnet):**
   - Must run a 7-day multi-node simulation with simulated 51% hashrate attacks, botnet micro-PoW floods, and network partition healings before initiating public testnet.
