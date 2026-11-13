# Sardhan Chain Core (`sardhan-core`)

Independent Rust Layer-1 implementation of the **Sardhan Chain (SDR)** blockchain:

- **Native Currency**: `SDR` (`1 SDR = 100,000,000 Sardhans`)
- **Hard Supply Cap**: `21,000,000 SDR` exact integer halving schedule (`50 SDR` initial reward, `210,000` blocks/epoch, `0%` Genesis premine, total max `20,999,999.97690000 SDR`, no perpetual tail emission)
- **Zero Transaction Fees**: `total_in == total_out`, defended against spam via block-bound `BLAKE3` micro-PoW (`RecentBlockHash` within last 16 blocks + quadratic mempool scaling)
- **Hybrid 50/50 Consensus ("Proof of Power")**:
  - **PoW**: `SardhanHash` (8 MiB thread-local scratchpad + 64 data-dependent branching rounds)
  - **PoS**: Scale-invariant staking kernel anchored to `H - 100` lagging stake modifier + strict canonical Ed25519 signature (`S < L`) + double-signing slashing evidence verification
  - **Fork Choice**: Exact 256-bit `U256` cumulative weight `floor(2^256 / (Target + 1))`
  - **Dual-DAA**: Independent PoW & PoS difficulty retargeting clamped `[0.25x, 4.0x]`
  - **Channel Balancing**: `[24, 40]` PoW blocks invariant per 64-block sliding window

## Workspace Crates

| Crate | Purpose |
| :--- | :--- |
| `sdr-crypto` | Canonical Ed25519 (`S < L`), BLAKE3, SHA-256, `SardhanHash`, Bech32m (`sdr1`/`tsdr1`/`rsdr1`), 18/24-word mnemonics & HD derivation, `HardwareSigner` trait |
| `sdr-primitives` | Network configs (`Mainnet`, `Testnet`, `Regtest`), `MAINNET_ENABLED = false` gate, 196-byte canonical `BlockHeader`, SegWit `Transaction`, OOM-bounded wire decoders |
| `sdr-consensus` | Exact `U256` weight & Dual-DAA retargeting, `SardhanHash` / PoS kernel verification, MTP-11 timestamp checks, 64-block channel balance, slashing proof verification |
| `sdr-ledger` | Persistent checksummed append-only `blocks.bin`, `slashing.json`, atomic UTXO state & `BlockUndo` reorg engine (`MAX_REORG_DEPTH = 288`), address & tx indexes |
| `sdr-mempool` | Zero-fee enforcement, quadratic micro-PoW target scaling, 16-block expiry pruning, bounded 64 MiB memory pool, per-peer rate limiter |
| `sdr-p2p` | Async multi-peer TCP mesh (`tokio`), persistent `peers.json`, `/16` Eclipse attack guard, peer scoring & banning, partition header/block sync |
| `sdr-rpc` | Async HTTP/1.1 JSON-RPC 2.0 server (`getblockchaininfo`, `getblock`, `getlatestblocks`, `getrawtransaction`, `sendrawtransaction`, `getaddressinfo`, `getmempoolinfo`, `getmininginfo`, `getblocktemplate`, `submitblock`, `getstakinginfo`, `getpeerinfo`, `getnetworkinfo`, `getnodehealth`, `addnode`, `disconnectnode`) |
| `sdr-node` (`sdrd`) | Full node daemon orchestrating storage, P2P, JSON-RPC, CPU miner & PoS validator |
| `sdr-miner` | Standalone multi-threaded CPU/ARM64 miner with thermal/battery throttling, pause/resume & empirical CPU benchmarks |
| `sdr-wallet` | Non-custodial CLI wallet (18/24-word seed creation & recovery, local Ed25519 signing, local micro-PoW proving, ISO-18004 QR renderer) |

## Reproducible Build & Verification

```bash
cd sardhan-core
cargo test --locked --workspace
cargo build --locked --release --workspace
sha256sum target/release/sdrd target/release/sdr-miner target/release/sdr-wallet
```

## Network Configuration Parameters

| Parameter | Mainnet (Locked) | Public Testnet (Active) | Regtest (Local Dev) |
| :--- | :--- | :--- | :--- |
| ChainID / Magic | `0x53445201` | `0x53445202` | `0x53445203` |
| Address Prefix (Bech32m) | `sdr1...` | `tsdr1...` | `rsdr1...` |
| Default P2P Port | `9333` | `19333` | `29333` |
| Default JSON-RPC Port | `9332` | `19332` | `29332` |
| Target Block Spacing | `150s` | `15s` | `5s` |
| Min Stake Age | `288 blocks` | `2 blocks` | `1 block` |
| Coinstake Maturity | `576 blocks` | `4 blocks` | `2 blocks` |
| Max Reorg Depth | `288 blocks` | `288 blocks` | `288 blocks` |

## Community 3-Node Testnet Quickstart

Run three independent nodes in separate terminal windows:

```bash
# Node 1 (Seed + CPU Miner)
./target/release/sdrd --network testnet --datadir ./node1 --p2p-port 19333 --rpc-port 19332 --mine --mine-threads 2

# Node 2 (Peer + CPU Miner)
./target/release/sdrd --network testnet --datadir ./node2 --p2p-port 19343 --rpc-port 19342 --addnode 127.0.0.1:19333 --mine

# Node 3 (Peer + PoS Validator + RPC)
./target/release/sdrd --network testnet --datadir ./node3 --p2p-port 19353 --rpc-port 19352 --addnode 127.0.0.1:19333 --addnode 127.0.0.1:19343 --mine --stake
```

Wallet & Mining CLI examples:

```bash
# Create local non-custodial Testnet wallet
./target/release/sdr-wallet --network testnet --wallet ./alice.json create --words 18

# Mine 1 real block directly to Alice's address
./target/release/sdr-miner --rpc 127.0.0.1:19332 --address tsdr1... --once

# Query balance and send a zero-fee transaction
./target/release/sdr-wallet --network testnet --wallet ./alice.json balance
./target/release/sdr-wallet --network testnet --wallet ./alice.json send --to tsdr1... --amount 10.5

# Run honest host CPU benchmark
./target/release/sdr-miner --benchmark 2
```

## Security Release Gate Notice

`MAINNET_ENABLED` is locked to `false` in `sdr-primitives`. Running `sdrd --network mainnet` exits immediately with status code `2` until independent external security review and physical GPU/ASIC resistance hardware benchmarks are completed and ratified.
