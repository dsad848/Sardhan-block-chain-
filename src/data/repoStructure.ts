import { RepoPackage } from '../types/protocol';

export const REPO_PACKAGES: RepoPackage[] = [
  {
    name: 'packages/crypto',
    path: '/packages/crypto',
    description: 'Cryptographic primitives for hashing, signatures, address encoding, and deterministic key derivation.',
    dependencies: ['None (Pure cryptographic primitives, system CSPRNG)'],
    files: [
      {
        name: 'ed25519.ts',
        description: 'RFC 8032 Ed25519 high-speed constant-time signature generation and batch verification.',
        keyInterfaces: ['sign(message: Uint8Array, privateKey: Uint8Array): Uint8Array', 'verify(signature: Uint8Array, message: Uint8Array, publicKey: Uint8Array): boolean']
      },
      {
        name: 'blake3.ts',
        description: 'BLAKE3 256-bit cryptographic tree hashing engine for fast transaction IDs and Merkle root calculation.',
        keyInterfaces: ['hash(data: Uint8Array): Uint8Array', 'keyedHash(key: Uint8Array, data: Uint8Array): Uint8Array']
      },
      {
        name: 'sardhan_hash.ts',
        description: 'Memory-light (~8 MiB) AES-NI & ARMv8 hardware-accelerated PoW hashing algorithm.',
        keyInterfaces: ['hashHeader(header144Bytes: Uint8Array): Uint8Array', 'verifyDifficulty(hash: Uint8Array, target: bigint): boolean']
      },
      {
        name: 'bech32m.ts',
        description: 'BIP-350 Bech32m checksummed encoder and decoder with human-readable prefix "sdr1".',
        keyInterfaces: ['encodeAddress(pubKeyHash: Uint8Array): string', 'decodeAddress(address: string): Uint8Array']
      },
      {
        name: 'bip39_44.ts',
        description: 'Deterministic 12/24-word mnemonic seed generator and BIP-44 key derivation path (m/44\'/9333\'/0\').',
        keyInterfaces: ['generateMnemonic(): string', 'deriveMasterKey(mnemonic: string): HDKey']
      }
    ]
  },
  {
    name: 'packages/primitives',
    path: '/packages/primitives',
    description: 'Immutable core data models, serialization protocols, and block/transaction data structures.',
    dependencies: ['packages/crypto'],
    files: [
      {
        name: 'block.ts',
        description: '144-byte BlockHeader structure and full Block container with transaction arrays.',
        keyInterfaces: ['serializeHeader(header: BlockHeader): Uint8Array', 'deserializeBlock(bytes: Uint8Array): Block']
      },
      {
        name: 'transaction.ts',
        description: 'UTXO Transaction model containing TxIn, TxOut, LockTime, and MicroPoW nonce fields.',
        keyInterfaces: ['computeTxId(tx: Transaction): Uint8Array', 'serializeTx(tx: Transaction): Uint8Array']
      },
      {
        name: 'merkle.ts',
        description: 'BLAKE3 incremental Merkle tree builder with inclusion proof verification.',
        keyInterfaces: ['computeMerkleRoot(txHashes: Uint8Array[]): Uint8Array', 'generateProof(index: number): MerkleProof']
      },
      {
        name: 'script.ts',
        description: 'Standard Pay-to-Public-Key-Hash (P2PKH) validation and locking script interpreter.',
        keyInterfaces: ['verifyScript(scriptSig: Uint8Array, scriptPubKey: Uint8Array, tx: Transaction, inputIndex: number): boolean']
      }
    ]
  },
  {
    name: 'packages/consensus',
    path: '/packages/consensus',
    description: 'Unified Proof of Power consensus engine, dual difficulty retargeting, and halving curves.',
    dependencies: ['packages/crypto', 'packages/primitives'],
    files: [
      {
        name: 'pow_validator.ts',
        description: 'Validates CPU/ARM Proof of Work headers against the dynamic compact target difficulty.',
        keyInterfaces: ['verifyPoWBlock(header: BlockHeader, currentTarget: bigint): boolean']
      },
      {
        name: 'pos_kernel.ts',
        description: 'Evaluates the staking kernel condition: BLAKE3(modifier || utxo || time) < target * stake.',
        keyInterfaces: ['evaluateKernel(stakeModifier: Uint8Array, utxo: UTXO, timestamp: number, target: bigint): boolean']
      },
      {
        name: 'daa_darkgravity.ts',
        description: 'DarkGravityWave v3 dual difficulty adjustment algorithm for 50% PoW and 50% PoS channels.',
        keyInterfaces: ['calculateNextTarget(channel: ConsensusType, history: BlockHeader[]): bigint']
      },
      {
        name: 'halving_calculator.ts',
        description: 'Mathematical 21M SDR halving decay calculation: floor((50 * 10^8) / 2^(h / 210000)).',
        keyInterfaces: ['getBlockReward(height: number): bigint', 'getCumulativeSupply(height: number): bigint']
      }
    ]
  },
  {
    name: 'packages/ledger',
    path: '/packages/ledger',
    description: 'Distributed ledger state machine, persistent UTXO index, and block undo storage.',
    dependencies: ['packages/primitives', 'packages/consensus'],
    files: [
      {
        name: 'utxo_set.ts',
        description: 'High-performance atomic key-value store for unspent transaction outputs with coin-age tracking.',
        keyInterfaces: ['applyBlock(block: Block): Promise<void>', 'getUTXO(outpoint: OutPoint): Promise<UTXO | null>']
      },
      {
        name: 'chain_state.ts',
        description: 'Tracks current best chain tip, accumulated work, and weak subjectivity checkpoints.',
        keyInterfaces: ['getBestTip(): ChainTip', 'isReorgPermitted(forkHeight: number): boolean']
      },
      {
        name: 'genesis.ts',
        description: 'Deterministic genesis block constructor with zero reward and strict 0% premine.',
        keyInterfaces: ['getGenesisBlock(): Block', 'verifyGenesisInvariants(): boolean']
      }
    ]
  },
  {
    name: 'packages/mempool',
    path: '/packages/mempool',
    description: 'Zero-fee transaction pool with client-side micro-PoW verification and dynamic congestion throttling.',
    dependencies: ['packages/primitives', 'packages/ledger'],
    files: [
      {
        name: 'micro_pow_verifier.ts',
        description: 'Validates client-side micro-PoW nonces in <100 microseconds per transaction.',
        keyInterfaces: ['verifyTxMicroPoW(tx: Transaction, requiredTarget: bigint): boolean']
      },
      {
        name: 'congestion_scaler.ts',
        description: 'Scales required micro-PoW difficulty quadratically as mempool fills above 50% capacity.',
        keyInterfaces: ['getDynamicTarget(currentBytes: number, maxBytes: number): bigint']
      },
      {
        name: 'eviction_queue.ts',
        description: 'Priority queue sorting transactions by micro-PoW density (difficulty / byte size).',
        keyInterfaces: ['addTx(tx: Transaction): boolean', 'getBlockTemplateTxList(maxBytes: number): Transaction[]']
      }
    ]
  },
  {
    name: 'packages/p2p',
    path: '/packages/p2p',
    description: 'Decentralized peer-to-peer gossip protocol, wire framing, and headers-first block synchronization.',
    dependencies: ['packages/primitives'],
    files: [
      {
        name: 'wire_protocol.ts',
        description: 'Binary framing (magic 0x53445201, command, length, BLAKE3 checksum, payload).',
        keyInterfaces: ['encodeMessage(cmd: string, payload: Uint8Array): Uint8Array', 'decodeMessage(bytes: Uint8Array): P2PMessage']
      },
      {
        name: 'peer_manager.ts',
        description: 'IP subnet bucketing (/16 IPv4, /32 IPv6), peer scoring, and Sybil attack defense.',
        keyInterfaces: ['addPeer(ip: string, port: number): void', 'banPeer(ip: string, durationMs: number): void']
      },
      {
        name: 'sync_engine.ts',
        description: 'Headers-first parallel blockchain download and validation pipeline.',
        keyInterfaces: ['requestHeaders(fromHeight: number): void', 'processBlockBatch(blocks: Block[]): void']
      }
    ]
  },
  {
    name: 'packages/rpc',
    path: '/packages/rpc',
    description: 'Standard JSON-RPC 2.0 interface for node management, wallet integrations, and miners.',
    dependencies: ['packages/node', 'packages/ledger', 'packages/mempool'],
    files: [
      {
        name: 'rpc_server.ts',
        description: 'HTTP JSON-RPC 2.0 listener running on port 9332.',
        keyInterfaces: ['handleRequest(jsonRpcReq: any): Promise<any>']
      },
      {
        name: 'blockchain_methods.ts',
        description: 'RPC handlers: getblockchaininfo, getblock, getbestblockhash, getrawtransaction.',
        keyInterfaces: ['getBlockchainInfo(): Promise<BlockchainInfoResponse>']
      },
      {
        name: 'mining_methods.ts',
        description: 'RPC handlers: getblocktemplate, submitblock, getmininginfo.',
        keyInterfaces: ['getBlockTemplate(): Promise<BlockTemplateResponse>', 'submitBlock(hex: string): Promise<boolean>']
      }
    ]
  },
  {
    name: 'packages/miner',
    path: '/packages/miner',
    description: 'Multi-threaded CPU miner and ARMv8 Android mining worker pool.',
    dependencies: ['packages/crypto', 'packages/primitives'],
    files: [
      {
        name: 'cpu_miner.ts',
        description: 'Multi-core worker pool utilizing native AES-NI instruction sets.',
        keyInterfaces: ['startMining(threads: number, coinbaseAddr: string): void', 'stopMining(): void']
      },
      {
        name: 'arm_optimizer.ts',
        description: 'ARMv8 Cryptographic Extension acceleration for Android mobile nodes.',
        keyInterfaces: ['isArmCryptoAvailable(): boolean', 'hashArmChunk(data: Uint8Array): Uint8Array']
      }
    ]
  },
  {
    name: 'packages/wallet',
    path: '/packages/wallet',
    description: 'Cryptographic HD wallet, transaction composer with micro-PoW solver, and background staker.',
    dependencies: ['packages/crypto', 'packages/primitives'],
    files: [
      {
        name: 'tx_builder.ts',
        description: 'Performs UTXO coin selection, generates Ed25519 signatures, and solves micro-PoW.',
        keyInterfaces: ['createTransaction(to: string, amount: bigint): Promise<Transaction>']
      },
      {
        name: 'staker_worker.ts',
        description: 'Iterates mature wallet UTXOs to test staking kernel every second for PoS block minting.',
        keyInterfaces: ['startStaking(): void', 'getStakingTelemetry(): StakingTelemetry']
      }
    ]
  }
];
