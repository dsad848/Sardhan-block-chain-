# Sardhan Chain (SDR) — Mainnet Release Candidate & Installation Guide

**Release Version:** `v0.2.0-rc1`  
**Mainnet Activation Gate:** `MAINNET_ENABLED = false` (Locked by Security Release Gate)

---

## 1. Network Configurations

| Parameter | Mainnet (Gated) | Public Testnet (Active) | Regtest (Local Dev) |
| :--- | :--- | :--- | :--- |
| **Network Name** | `sdr-mainnet` | `sdr-testnet` | `sdr-regtest` |
| **ChainID / Magic** | `0x53445201` (`1396986369`) | `0x53445202` (`1396986370`) | `0x53445203` (`1396986371`) |
| **Address Prefix (Bech32m)** | `sdr1...` | `tsdr1...` | `rsdr1...` |
| **P2P TCP Port** | `9333` | `19333` | `29333` |
| **JSON-RPC HTTP Port** | `9332` | `19332` | `29332` |
| **Target Block Spacing** | `150s` | `15s` | `5s` |
| **Min Stake Maturity** | `288 blocks` | `2 blocks` | `1 block` |
| **Coinstake Lock Window** | `576 blocks` | `4 blocks` | `2 blocks` |
| **Max Reorg Depth** | `288 blocks` | `288 blocks` | `288 blocks` |

## 2. Genesis Block Invariants

- **Genesis Timestamp:** `1772928000`
- **Genesis Reward / Premine:** `0.00000000 SDR` (Zero Premine, zero outputs at height 0)
- **Genesis Merkle Root:** `BLAKE3("Sardhan Chain: Zero Premine, Zero Fees, Sovereign Hybrid PoW/PoS")`
- **Genesis Lagging Stake Modifier:** `[0x53; 32]`
- **Consensus Type:** `ProofOfWork` (`nonce = 0x53445201`)

## 3. Reproducible Release Build & Checksums

Build release binaries strictly using the pinned `Cargo.lock`:

```bash
cd sardhan-core
cargo test --locked --workspace
cargo build --locked --release --workspace
sha256sum -c SHA256SUMS.txt
```

Verified Linux `x86_64` release binary checksums:

- `sdrd`: `be1316c29006e1a18cfacdc3675f8046f028e178184defeb0beb0154ced6ce7c`
- `sdr-miner`: `f2d18eee5f552f15d5506c85e03a1602aacb39ee034607a6ebaf1b118cd431ef`
- `sdr-wallet`: `92f6e901a09319a309690f6a0f016f0e10f1b01e7545878d6b755d4ca2be5969`

## 4. Running the Full-Stack Explorer, Wallet & 3-Node Testnet

```bash
npm install
npm run dev
```

This starts Express (`server.ts`) on `http://localhost:3000`, spawns 3 independent `sdrd` TCP peers in `.sdr-testnet/node{1,2,3}` (P2P ports `19333`, `19343`, `19353`; RPC ports `19332`, `19342`, `19352`), and serves the React Block Explorer, non-custodial Web Wallet, and live host Benchmark view.

## 5. Honest Empirical Benchmarks & Remaining Gates Before Mainnet

Measured on `x86_64` Linux (2 cores) using `sdr-miner --benchmark 2`:

| Workload | Measured Throughput | Avg Latency | Memory |
| :--- | :--- | :--- | :--- |
| `SardhanHash (1T actual)` | `88.44 H/s` | `11.307 ms` | `8 MiB` |
| `SardhanHash (2T actual)` | `146.96 H/s` | `6.805 ms` | `8 MiB/thr` |
| `RandomX-style 2MiB VM sim (1T)` | `21,946.56 H/s` | `0.0456 ms` | `2 MiB` |
| `Verus-style ARX mix sim (1T)` | `3,172,218.91 H/s` | `0.00032 ms` | Stateless |
| `BLAKE3-256 Header Hash (1T)` | `2,553,717.98 H/s` | `0.000392 ms` | `0 B` |

**Unresolved Mainnet Release Gates:**
1. Independent external cryptographic and consensus audit sign-off.
2. Physical GPU (CUDA / OpenCL / Metal) hardware benchmarking and optimization of SardhanHash mixing-to-scratchpad fill ratio before claiming GPU resistance.
3. Flipping `MAINNET_ENABLED` to `true` in `sardhan-core/crates/sdr-primitives/src/lib.rs` ONLY after gates 1–11 pass.
