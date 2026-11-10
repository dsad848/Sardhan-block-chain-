import { AuditFinding } from '../types/protocol';

export const AUDIT_FINDINGS: AuditFinding[] = [
  {
    id: 'SEC-CRIT-01',
    domain: 'Zero-Fee Economics',
    title: 'Offline Micro-PoW Pre-Mining & Botnet Burst Flood',
    severity: 'Critical',
    affectedDocs: ['RFC-102 (Sec 4.2)', 'RFC-107'],
    description: 'The v1 micro-PoW check BLAKE3(TxData || Nonce) < Target has no temporal binding to recent blockchain state. An attacker can calculate millions of valid nonces weeks in advance.',
    exploitScenario: 'An attacker uses cloud instances or botnets to pre-calculate valid micro-PoW nonces for 1,000,000 zero-fee transactions offline over 14 days. They suddenly flood the P2P network in a 3-second burst. Full nodes cannot invalidate them based on age, causing severe mempool OOM crashes.',
    remediation: 'Implement Block-Bound Micro-PoW: BLAKE3(RecentBlockHash || TxPayload || Nonce) < Target, where RecentBlockHash must reside in the canonical chain within the last 16 blocks (~40 minutes). Older transactions are rejected instantly.',
    testCase: 'TEST-SEC-01: Craft a valid transaction with micro-PoW bound to Block (Tip - 17). Verify node rejects with ERR_MICRO_POW_EXPIRED.',
    mathematicalProof: 'Offline pre-mining utility drops to zero because all pre-computed hashes become invalid after 40 minutes: Total Hashes = 500,000 * 2^18 = 1.31e11. Required burst compute rate is 54.6 MH/s, preventing offline storage attacks.'
  },
  {
    id: 'SEC-CRIT-02',
    domain: 'Cryptography & Wire',
    title: 'Missing ChainID in Signature Preimage (Cross-Network Replay)',
    severity: 'Critical',
    affectedDocs: ['RFC-102 (Sec 4.1)', 'RFC-105'],
    description: 'The transaction serialization format lacks a ChainID or network identifier in both its binary wire payload and the Ed25519 signature digest preimage.',
    exploitScenario: 'A user broadcasts a transaction on Sardhan Testnet (ChainID 0x53445202). An adversary intercepts the raw hex and replays it verbatim on Mainnet (ChainID 0x53445201). Because the keys and UTXOs originate identically from bootstrap, Mainnet nodes accept the signature and transfer real user funds.',
    remediation: 'Mandate explicit 32-bit ChainID field in the transaction header and require ChainID in the signature digest: SigDigest = BLAKE3(ChainID || TxCanonicalPayload).',
    testCase: 'TEST-SEC-02: Sign transaction using Testnet ChainID and broadcast to a Mainnet node. Verify node rejects with ERR_SIG_INVALID_CHAIN_ID.'
  },
  {
    id: 'SEC-CRIT-03',
    domain: 'Consensus',
    title: 'Stake Grinding via Immediate Parent Hash Manipulation',
    severity: 'Critical',
    affectedDocs: ['RFC-103 (Sec 2.2)'],
    description: 'The Stake Modifier updates every block using the immediate previous block hash: M_h = BLAKE3(M_{h-1} || B_{h-1}.Hash). Miners can grind timestamps and nonces on block h-1 to bias future staking lotteries.',
    exploitScenario: 'A miner with 15% stake grinds candidate solutions on block h-1 until they produce a block hash that deterministically selects their own UTXOs for blocks h and h+1, effectively monopolizing block production.',
    remediation: 'Implement a Lagging Stake Modifier with a 100-block buffer: M_h = BLAKE3(M_{h-100} || B_{h-100}.Hash). Manipulating block h-1 cannot influence block h because the entropy was locked ~4 hours earlier.',
    testCase: 'TEST-SEC-03: Modify timestamp and nonce on Block H. Verify StakeModifier(H+1) does not change, confirming entropy isolation from parent block.'
  },
  {
    id: 'SEC-HIGH-01',
    domain: 'Consensus',
    title: 'Undefined Fork-Choice Weight Metric & Channel Starvation',
    severity: 'High',
    affectedDocs: ['RFC-103 (Sec 1-2)'],
    description: 'The v1 proposal states "50% PoW and 50% PoS" but omits the exact scalar chain weight formula W(C) used by nodes to select the canonical chain during forks.',
    exploitScenario: 'During a difficulty drop on the PoW channel, an attacker mines a rapid sequence of low-difficulty PoW blocks. If nodes compare simple chain length (block count), the attacker chain displaces the honest high-difficulty chain.',
    remediation: 'Define Unified Proof-of-Power Weight: W(C) = sum(2^256 / Target(B)). Enforce a Channel Balance Invariant: in any sliding window of 32 blocks, count of either type must be between 8 and 24 (25%-75%).',
    testCase: 'TEST-SEC-04: Construct two competing branches: Branch A (30 blocks, lower difficulty) vs Branch B (25 blocks, high cumulative difficulty). Verify nodes choose Branch B.'
  },
  {
    id: 'SEC-HIGH-02',
    domain: 'Mining Architecture',
    title: 'GPU L2 Cache Fit & Lack of Branch Divergence in SardhanHash',
    severity: 'High',
    affectedDocs: ['RFC-103 (Sec 4)'],
    description: 'An 8 MiB scratchpad fits entirely within the on-chip L2/L3 cache of modern GPUs (e.g. RTX 4090 has 72 MB L2 cache). Linear execution without dynamic thread branching allows GPUs to heavily outperform mobile ARM chips.',
    exploitScenario: 'GPU mining farms dominate SardhanHash, achieving 500x the efficiency of consumer Android smartphones and destroying the democratic mobile mining objective.',
    remediation: 'Incorporate data-dependent dynamic branching and instruction randomization (VerusHash 2.2b / Argon2d style) into SardhanHash to penalize GPU SIMD lockstep execution while running at full speed on CPU/ARM branch predictors.',
    testCase: 'TEST-SEC-05: Benchmark SardhanHash on CUDA with 32-thread warps. Verify high warp divergence penalty (>80% execution serialization under heavy branching).'
  },
  {
    id: 'SEC-HIGH-03',
    domain: 'Cryptography',
    title: 'Transaction Signature Malleability & Non-Canonical S Values',
    severity: 'High',
    affectedDocs: ['RFC-102 (Sec 5)', 'RFC-105'],
    description: 'Ed25519 signatures (R, S) are malleable if non-canonical S values (S >= L) are accepted by deserializers, altering the TxID while keeping signature verification valid.',
    exploitScenario: 'An eavesdropping attacker intercepts an unconfirmed payment, replaces S with S + L, and broadcasts. The modified transaction confirms with a different TxID, breaking dependent unconfirmed child transactions.',
    remediation: 'Enforce strict canonical S < L check in the deserializer. Implement Segregated Witness: TxID is computed over canonical transaction inputs and outputs excluding signature witnesses.',
    testCase: 'TEST-SEC-06: Construct an Ed25519 signature with S_mod = S + L. Verify node deserializer rejects with ERR_ED25519_NON_CANONICAL_S.'
  },
  {
    id: 'SEC-HIGH-04',
    domain: 'Network & Synchronization',
    title: 'PoS Header Sync DoS (Unverified State Dependency)',
    severity: 'High',
    affectedDocs: ['RFC-102 (Sec 6)', 'RFC-105'],
    description: 'During headers-first sync, a node receives headers before transactions. PoS header validation requires evaluating the staking kernel against the spent UTXO, which the syncing node does not yet possess.',
    exploitScenario: 'A malicious peer sends 200,000 fabricated PoS headers. The syncing node cannot verify whether the staked UTXO is mature or real without the full UTXO database, resulting in memory exhaustion or accepting an invalid chain.',
    remediation: 'Every PoS block header must include a Compact Merkle Proof of Stake Maturity referencing an earlier confirmed block UTXO root, allowing stateless verification of the staking kernel during headers-first sync.',
    testCase: 'TEST-SEC-07: Send fabricated PoS headers without a valid UTXO Merkle root proof. Verify syncing node rejects during header validation without allocating block storage.'
  },
  {
    id: 'SEC-HIGH-05',
    domain: 'Tokenomics',
    title: 'Long-Term Validator Sustainability with Zero Transaction Fees',
    severity: 'High',
    affectedDocs: ['RFC-106', 'RFC-107'],
    description: 'In year 10+, block rewards drop below 0.09 SDR and eventually to 0. Since transaction fees are permanently 0, node operators and validators have no economic incentive to run hardware.',
    exploitScenario: 'Validators shut down community nodes due to zero fee revenue and minimal block rewards in late epochs, making the network vulnerable to hostile centralization.',
    remediation: 'Formulate an Inactive UTXO Demurrage policy and a permanent baseline protocol security budget (minimum 0.05 SDR tail reward after Epoch 10, or micro-bond community validator incentives).',
    testCase: 'TEST-SEC-08: Simulate economic model at Epoch 15 with zero transaction fees. Verify validator yield remains positive under the revised baseline security budget.'
  },
  {
    id: 'SEC-MED-01',
    domain: 'Binary Implementation',
    title: 'Block Header Size Arithmetic Mismatch (144 bytes vs 194 bytes)',
    severity: 'Medium',
    affectedDocs: ['RFC-102 (Sec 3.1)'],
    description: 'RFC-102 claimed a 144-byte block header, but summing the specified fields results in 194 bytes (with 64B Ed25519 signature) or 130 bytes (without signature).',
    exploitScenario: 'P2P wire parsers expecting a fixed 144-byte buffer crash or panic with buffer underflow/overflow errors upon receiving blocks.',
    remediation: 'Establish a canonical fixed-size 196-byte header: Version(4) + PrevBlock(32) + MerkleRoot(32) + Timestamp(8) + Height(8) + Bits(4) + Type(1) + Modifier(32) + Nonce(8) + Flags(3) + ValidatorSig(64) = 196 bytes.',
    testCase: 'TEST-SEC-09: Serialize and deserialize PoW and PoS block headers. Verify length is strictly 196 bytes in both cases.'
  },
  {
    id: 'SEC-MED-02',
    domain: 'Mining Architecture',
    title: 'Mobile Thermal Throttling & Android OOM Daemon Kills',
    severity: 'Medium',
    affectedDocs: ['RFC-101', 'RFC-103'],
    description: 'Continuous background CPU mining on Android devices without thermal and power management triggers OS lowmemorykiller and thermal throttling.',
    exploitScenario: 'Users attempt to mine on Android, but the smartphone overheats within 5 minutes, battery drains rapidly, and Android OS terminates the node process.',
    remediation: 'Specify mandatory mobile battery and thermal constraints in sdr-miner: pause mining if battery temp > 38C or battery < 20%, and run in non-intrusive foreground service with wake-lock.',
    testCase: 'TEST-SEC-10: Trigger simulated battery temperature of 39C. Verify mobile miner pauses immediately and resumes when temperature drops to 35C.'
  },
  {
    id: 'SEC-MED-03',
    domain: 'Network Security',
    title: 'Eclipse Vulnerability on Low-Degree Peer Bootstraps',
    severity: 'Medium',
    affectedDocs: ['RFC-104 (Sec 2.3)'],
    description: 'If a node relies only on a small hardcoded seed list during initial bootstrap, an attacker controlling adjacent IP subnets can isolate the node from the honest network.',
    exploitScenario: 'An attacker monitors peer announcements and fills a new node\'s 8 outbound connection slots with attacker-controlled Sybil IPs.',
    remediation: 'Mandate Autonomous System Number (ASN) diversification: outbound connections must connect to at least 4 distinct ASNs, and nodes must persist anchor peers in ~/.sardhan/anchors.dat.',
    testCase: 'TEST-SEC-11: Attempt to fill node outbound slots with IPs from a single /16 subnet. Verify peer manager rejects excess connections after slot 2.'
  },
  {
    id: 'SEC-MED-04',
    domain: 'Cryptography',
    title: 'Public Key Hash vs Full Key Address Collision Space',
    severity: 'Medium',
    affectedDocs: ['RFC-102 (Sec 5)'],
    description: 'If address derivation truncates BLAKE3 output to 20 bytes (160 bits), birthday collision search complexity is 2^80, which is vulnerable to nation-state supercomputing clusters.',
    exploitScenario: 'An attacker uses high-throughput GPU clusters to find a collision between an honest address and a malicious address.',
    remediation: 'Enforce full 32-byte (256-bit) Bech32m address encoding: sdr1 + Bech32m(BLAKE3(PublicKey)), providing full 128-bit collision resistance and 256-bit preimage resistance.',
    testCase: 'TEST-SEC-12: Generate Bech32m address from a 32-byte Ed25519 public key. Verify address length is exactly 59 characters and decodes to full 32 bytes.'
  }
];
