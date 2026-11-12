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

// Standard Bech32m character set (BIP-0173 / BIP-0350)
const BECH32_CHARSET = 'qpzry9x8gf2tvdw0s3jn54khce6mua7l';
const BECH32M_CONST = 0x2bc830a3;

function bech32Polymod(values: number[]): number {
  let chk = 1;
  for (const v of values) {
    const top = chk >> 25;
    chk = ((chk & 0x1ffffff) << 5) ^ v;
    if ((top >> 0) & 1) chk ^= 0x3b6a57b2;
    if ((top >> 1) & 1) chk ^= 0x26508e6d;
    if ((top >> 2) & 1) chk ^= 0x1ea119fa;
    if ((top >> 3) & 1) chk ^= 0x3d4233dd;
    if ((top >> 4) & 1) chk ^= 0x2a1462b3;
  }
  return chk;
}

function hrpExpand(hrp: string): number[] {
  const ret: number[] = [];
  for (let i = 0; i < hrp.length; i++) ret.push(hrp.charCodeAt(i) >> 5);
  ret.push(0);
  for (let i = 0; i < hrp.length; i++) ret.push(hrp.charCodeAt(i) & 31);
  return ret;
}

function convertBits(data: Uint8Array, fromBits: number, toBits: number, pad: boolean): number[] {
  let acc = 0;
  let bits = 0;
  const ret: number[] = [];
  const maxv = (1 << toBits) - 1;
  for (let p = 0; p < data.length; ++p) {
    const value = data[p];
    acc = (acc << fromBits) | value;
    bits += fromBits;
    while (bits >= toBits) {
      bits -= toBits;
      ret.push((acc >> bits) & maxv);
    }
  }
  if (pad) {
    if (bits > 0) {
      ret.push((acc << (toBits - bits)) & maxv);
    }
  }
  return ret;
}

// Canonical Bech32m address encoding matching Rust sdr-crypto (Issue 7 & D Fix)
export function pubKeyToAddress(pubKeyBytes: Uint8Array): string {
  const hrp = 'sdr';
  const data5Bit = convertBits(pubKeyBytes, 8, 5, true);
  const values = hrpExpand(hrp).concat(data5Bit).concat([0, 0, 0, 0, 0, 0]);
  const mod = bech32Polymod(values) ^ BECH32M_CONST;
  const checksum: number[] = [];
  for (let p = 0; p < 6; ++p) {
    checksum.push((mod >> (5 * (5 - p))) & 31);
  }
  const combined = data5Bit.concat(checksum);
  let result = hrp + '1';
  for (const v of combined) {
    result += BECH32_CHARSET.charAt(v);
  }
  return result;
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

// Construct canonical 196-byte Block Header buffer and compute real BLAKE3 hash
export function serializeBlockHeader(
  version: number,
  prevBlockHashHex: string,
  merkleRootHex: string,
  timestamp: number,
  blockHeight: number,
  consensusBits: number,
  consensusType: 'PoW' | 'PoS',
  laggingStakeModifierHex: string,
  powNonce: number,
  validatorSigHex: string
): { headerBytes: Uint8Array; blockHashHex: string } {
  const buf = new Uint8Array(BLOCK_HEADER_SIZE);
  const view = new DataView(buf.buffer);

  // 1. Version (4B LE)
  view.setUint32(0, version, true);

  // 2. Prev Block Hash (32B)
  const prevBytes = hexToBytes(prevBlockHashHex);
  buf.set(prevBytes.slice(0, 32), 4);

  // 3. Merkle Root (32B)
  const merkleBytes = hexToBytes(merkleRootHex);
  buf.set(merkleBytes.slice(0, 32), 36);

  // 4. Timestamp (8B LE)
  view.setBigUint64(68, BigInt(timestamp), true);

  // 5. Block Height (8B LE)
  view.setBigUint64(76, BigInt(blockHeight), true);

  // 6. Consensus Bits (4B LE)
  view.setUint32(84, consensusBits, true);

  // 7. Consensus Type (1B: 0 = PoW, 1 = PoS)
  view.setUint8(88, consensusType === 'PoW' ? 0 : 1);

  // 8. Lagging Stake Modifier (32B)
  const lsmBytes = hexToBytes(laggingStakeModifierHex);
  buf.set(lsmBytes.slice(0, 32), 89);

  // 9. PoW Nonce (8B LE)
  view.setBigUint64(121, BigInt(powNonce), true);

  // 10. Reserved (3B)
  buf[129] = 0;
  buf[130] = 0;
  buf[131] = 0;

  // 11. Validator Signature (64B)
  const sigBytes = hexToBytes(validatorSigHex);
  buf.set(sigBytes.slice(0, 64), 132);

  // Cryptographic BLAKE3 hash of the full 196-byte header
  const blockHashBytes = blake3(buf);
  return {
    headerBytes: buf,
    blockHashHex: bytesToHex(blockHashBytes),
  };
}

// Create Genesis Block matching Rust sdr-ledger
export function createGenesisBlock(): TestnetBlock {
  const prevHash = '00'.repeat(32);
  const merkleRoot = bytesToHex(blake3(new TextEncoder().encode('Sardhan Chain: Zero Premine, Zero Fees, Sovereign Hybrid PoW/PoS')));
  const lsm = '53'.repeat(32);
  const nonce = 0x53445201;
  const sigHex = '00'.repeat(64);

  const { headerBytes, blockHashHex } = serializeBlockHeader(
    1,
    prevHash,
    merkleRoot,
    1772928000,
    0,
    0x1e0ffff0,
    'PoW',
    lsm,
    nonce,
    sigHex
  );

  return {
    headerBytesHex: bytesToHex(headerBytes),
    version: 1,
    prevBlockHash: prevHash,
    merkleRoot,
    timestamp: 1772928000,
    blockHeight: 0,
    consensusBits: 0x1e0ffff0,
    consensusType: 'PoW',
    laggingStakeModifier: lsm,
    powNonce: nonce,
    validatorSignatureHex: sigHex,
    blockHash: blockHashHex,
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
