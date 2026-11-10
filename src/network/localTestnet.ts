import { ed25519 } from '@noble/curves/ed25519.js';
import { blake3 } from '@noble/hashes/blake3.js';
// Self-contained hex helper utilities to avoid ESM export issues across bundlers
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  const bytes = new Uint8Array(Math.floor(clean.length / 2));
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.substring(i * 2, i * 2 + 2), 16) || 0;
  }
  return bytes;
}

export const SARDHAN_MAINNET_MAGIC = 0x53445201;
export const BLOCK_HEADER_SIZE = 196;

export interface TestnetOutPoint {
  txid: string;
  vout: number;
}

export interface TestnetTxIn {
  previousOutput: TestnetOutPoint;
  sequence: number;
}

export interface TestnetTxOut {
  valueSardhans: number;
  recipientPubKeyHex: string;
  recipientAddress: string;
}

export interface TestnetTxWitness {
  signatureHex: string;
  publicKeyHex: string;
}

export interface TestnetTransaction {
  chainId: number;
  version: number;
  recentBlockHash: string;
  microPowNonce: number;
  lockTime: number;
  inputs: TestnetTxIn[];
  outputs: TestnetTxOut[];
  witnesses: TestnetTxWitness[];
  txid: string;
  byteSize: number;
  microPowHash: string;
}

export interface TestnetBlock {
  headerBytesHex: string;
  version: number;
  prevBlockHash: string;
  merkleRoot: string;
  timestamp: number;
  blockHeight: number;
  consensusBits: number;
  consensusType: 'PoW' | 'PoS';
  laggingStakeModifier: string;
  powNonce: number;
  validatorSignatureHex: string;
  blockHash: string;
  transactions: TestnetTransaction[];
  weight: number;
}

export interface TestnetNode {
  id: string;
  name: string;
  role: 'Seed Validator' | 'CPU Miner' | 'Community Staker' | 'Mobile Light Node';
  p2pPort: number;
  bestHeight: number;
  bestBlockHash: string;
  mempoolCount: number;
  utxoCount: number;
  balanceSdr: number;
  status: 'Online' | 'Mining' | 'Staking' | 'Synchronizing' | 'Partitioned';
  keypair: {
    privateKeyHex: string;
    publicKeyHex: string;
    address: string;
  };
}

// Helper to encode Bech32m-like address
export function pubKeyToAddress(pubKeyBytes: Uint8Array): string {
  const hash = blake3(pubKeyBytes);
  return 'sdr1' + bytesToHex(hash.slice(0, 16));
}

// Generate new keypair
export function generateWalletKeypair() {
  const privKey = ed25519.utils.randomSecretKey();
  const pubKey = ed25519.getPublicKey(privKey);
  return {
    privateKeyHex: bytesToHex(privKey),
    publicKeyHex: bytesToHex(pubKey),
    address: pubKeyToAddress(pubKey)
  };
}

// Compute transaction ID
export function computeTxId(
  chainId: number,
  recentBlockHash: string,
  microPowNonce: number,
  inputs: TestnetTxIn[],
  outputs: TestnetTxOut[]
): string {
  const payload = JSON.stringify({ chainId, recentBlockHash, microPowNonce, inputs, outputs });
  const hash = blake3(new TextEncoder().encode(payload));
  return bytesToHex(hash);
}

// Solve client-side micro-PoW (~50ms)
export function solveMicroPoW(
  chainId: number,
  recentBlockHash: string,
  inputs: TestnetTxIn[],
  outputs: TestnetTxOut[],
  difficultyZeroes = 1
): { nonce: number; hash: string } {
  let nonce = 1;
  while (nonce < 100000) {
    const hashHex = computeTxId(chainId, recentBlockHash, nonce, inputs, outputs);
    if (hashHex.startsWith('0'.repeat(difficultyZeroes))) {
      return { nonce, hash: hashHex };
    }
    nonce++;
  }
  return { nonce: 1, hash: computeTxId(chainId, recentBlockHash, 1, inputs, outputs) };
}

// Create Genesis Block
export function createGenesisBlock(): TestnetBlock {
  const prevHash = '0000000000000000000000000000000000000000000000000000000000000000';
  const root = bytesToHex(blake3(new TextEncoder().encode('Sardhan Chain: Zero Premine Genesis')));
  const blockHash = bytesToHex(blake3(new TextEncoder().encode('SDR_GENESIS_BLOCK_HASH')));

  return {
    headerBytesHex: '53445201' + '00'.repeat(192),
    version: 1,
    prevBlockHash: prevHash,
    merkleRoot: root,
    timestamp: 1772928000,
    blockHeight: 0,
    consensusBits: 0x1e0ffff0,
    consensusType: 'PoW',
    laggingStakeModifier: '53535353'.repeat(8),
    powNonce: 0x53445201,
    validatorSignatureHex: '00'.repeat(64),
    blockHash,
    transactions: [],
    weight: 1000,
  };
}

// Initial 4 Multi-Nodes
export function createInitialNodes(): TestnetNode[] {
  const genesis = createGenesisBlock();
  
  return [
    {
      id: 'node-01',
      name: 'sdrd-seed-01',
      role: 'Seed Validator',
      p2pPort: 9333,
      bestHeight: 0,
      bestBlockHash: genesis.blockHash,
      mempoolCount: 0,
      utxoCount: 0,
      balanceSdr: 0,
      status: 'Online',
      keypair: generateWalletKeypair(),
    },
    {
      id: 'node-02',
      name: 'sdrd-miner-02',
      role: 'CPU Miner',
      p2pPort: 9334,
      bestHeight: 0,
      bestBlockHash: genesis.blockHash,
      mempoolCount: 0,
      utxoCount: 0,
      balanceSdr: 0,
      status: 'Mining',
      keypair: generateWalletKeypair(),
    },
    {
      id: 'node-03',
      name: 'sdrd-staker-03',
      role: 'Community Staker',
      p2pPort: 9335,
      bestHeight: 0,
      bestBlockHash: genesis.blockHash,
      mempoolCount: 0,
      utxoCount: 0,
      balanceSdr: 0,
      status: 'Staking',
      keypair: generateWalletKeypair(),
    },
    {
      id: 'node-04',
      name: 'sdr-mobile-04',
      role: 'Mobile Light Node',
      p2pPort: 9336,
      bestHeight: 0,
      bestBlockHash: genesis.blockHash,
      mempoolCount: 0,
      utxoCount: 0,
      balanceSdr: 0,
      status: 'Online',
      keypair: generateWalletKeypair(),
    },
  ];
}
