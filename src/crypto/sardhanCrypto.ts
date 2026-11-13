import { ed25519 } from '@noble/curves/ed25519.js';
import { blake3 } from '@noble/hashes/blake3.js';
import { sha256 } from '@noble/hashes/sha2.js';

export const SARDHAN_MAINNET_MAGIC = 0x53445201;
export const SARDHAN_TESTNET_MAGIC = 0x53445202;
export const SARDHAN_REGTEST_MAGIC = 0x53445203;
export const SARDHANS_PER_SDR = 100_000_000n;
export const MIN_TRANSACTION_OUTPUT_VALUE = 1000n;

export type SdrNetworkName = 'testnet' | 'mainnet' | 'regtest';

export function networkHrp(network: SdrNetworkName): string {
  switch (network) {
    case 'mainnet':
      return 'sdr';
    case 'testnet':
      return 'tsdr';
    case 'regtest':
      return 'rsdr';
  }
}

export function networkChainId(network: SdrNetworkName): number {
  switch (network) {
    case 'mainnet':
      return SARDHAN_MAINNET_MAGIC;
    case 'testnet':
      return SARDHAN_TESTNET_MAGIC;
    case 'regtest':
      return SARDHAN_REGTEST_MAGIC;
  }
}

export function bytesToHex(bytes: Uint8Array): string {
  let out = '';
  for (let i = 0; i < bytes.length; i++) {
    out += bytes[i].toString(16).padStart(2, '0');
  }
  return out;
}

export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  if (clean.length % 2 !== 0) {
    throw new Error('Hex string must have even length');
  }
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) {
    const byte = Number.parseInt(clean.slice(i * 2, i * 2 + 2), 16);
    if (Number.isNaN(byte)) {
      throw new Error('Invalid hex character');
    }
    out[i] = byte;
  }
  return out;
}

// Exact 256-word dictionary matching Rust `sdr-crypto` MNEMONIC_WORDS
export const MNEMONIC_WORDS: readonly string[] = [
  'abandon', 'ability', 'able', 'about', 'above', 'absent', 'absorb', 'abstract',
  'absurd', 'access', 'account', 'accuse', 'achieve', 'acid', 'acoustic', 'acquire',
  'across', 'action', 'actor', 'actual', 'adapt', 'address', 'adjust', 'admit',
  'adult', 'advance', 'advice', 'aerobic', 'affair', 'afford', 'afraid', 'again',
  'agent', 'agree', 'ahead', 'aiming', 'airway', 'aisle', 'alarm', 'album',
  'alcohol', 'alert', 'alien', 'allied', 'allow', 'almost', 'alone', 'alpha',
  'already', 'also', 'alter', 'always', 'amateur', 'amazing', 'amber', 'among',
  'amount', 'amused', 'anchor', 'ancient', 'anger', 'angle', 'animal', 'ankle',
  'announce', 'annual', 'another', 'answer', 'antenna', 'antique', 'anvil', 'apart',
  'apology', 'appear', 'apple', 'approve', 'april', 'arch', 'arctic', 'arena',
  'argue', 'arise', 'armor', 'army', 'around', 'arrange', 'arrest', 'arrive',
  'arrow', 'artery', 'artist', 'artwork', 'aspect', 'assault', 'asset', 'assist',
  'assume', 'asthma', 'athlete', 'atom', 'attack', 'attend', 'attitude', 'attract',
  'auction', 'audit', 'august', 'aunt', 'author', 'auto', 'autumn', 'average',
  'avocado', 'avoid', 'awake', 'aware', 'awesome', 'awful', 'awkward', 'axis',
  'bacon', 'badge', 'balance', 'balcony', 'bamboo', 'banana', 'banner', 'bargain',
  'barrel', 'base', 'basic', 'basket', 'battle', 'beacon', 'beauty', 'because',
  'become', 'before', 'begin', 'behave', 'behind', 'believe', 'below', 'belt',
  'bench', 'benefit', 'beyond', 'bicycle', 'bitter', 'black', 'blade', 'blame',
  'blanket', 'blast', 'bleak', 'bless', 'blind', 'blood', 'blossom', 'blouse',
  'blue', 'blur', 'blush', 'board', 'boast', 'border', 'borrow', 'bottom',
  'bounce', 'bracket', 'brain', 'brand', 'brass', 'brave', 'breeze', 'brick',
  'bridge', 'brief', 'bright', 'bring', 'brisk', 'broken', 'bronze', 'brother',
  'brown', 'brush', 'bubble', 'bucket', 'budget', 'buffalo', 'build', 'bullet',
  'bundle', 'bunker', 'burden', 'burger', 'burst', 'butter', 'buyer', 'cabin',
  'cable', 'cactus', 'cage', 'cake', 'calm', 'camera', 'camp', 'canal',
  'cancel', 'candy', 'cannon', 'canoe', 'canvas', 'canyon', 'capable', 'capital',
  'captain', 'carbon', 'cargo', 'carpet', 'carry', 'cart', 'castle', 'casual',
  'catalog', 'catch', 'causal', 'caution', 'cave', 'ceiling', 'celery', 'cement',
  'census', 'century', 'ceramic', 'certain', 'chair', 'chalk', 'champion', 'change',
  'chaos', 'chapter', 'charge', 'chase', 'cheap', 'check', 'cheese', 'cherry',
  'chest', 'chief', 'child', 'chimney', 'choice', 'choose', 'chronic', 'cigar',
];

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

function convertBits(data: Uint8Array | number[], fromBits: number, toBits: number, pad: boolean): number[] {
  let acc = 0;
  let bits = 0;
  const ret: number[] = [];
  const maxv = (1 << toBits) - 1;
  const maxAcc = (1 << (fromBits + toBits - 1)) - 1;
  for (let p = 0; p < data.length; p++) {
    const value = data[p];
    if (value < 0 || value >> fromBits !== 0) {
      throw new Error('Invalid convertBits input');
    }
    acc = ((acc << fromBits) | value) & maxAcc;
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
  } else if (bits >= fromBits || ((acc << (toBits - bits)) & maxv) !== 0) {
    throw new Error('Invalid padding bits in Bech32m data');
  }
  return ret;
}

export function encodeAddressForHrp(hrp: string, pubKeyBytes: Uint8Array): string {
  if (pubKeyBytes.length !== 32) {
    throw new Error('Public key must be 32 bytes');
  }
  const data5Bit = convertBits(pubKeyBytes, 8, 5, true);
  const values = hrpExpand(hrp).concat(data5Bit).concat([0, 0, 0, 0, 0, 0]);
  const mod = bech32Polymod(values) ^ BECH32M_CONST;
  const checksum: number[] = [];
  for (let p = 0; p < 6; p++) {
    checksum.push((mod >> (5 * (5 - p))) & 31);
  }
  const combined = data5Bit.concat(checksum);
  let result = `${hrp}1`;
  for (const v of combined) {
    result += BECH32_CHARSET.charAt(v);
  }
  return result;
}

export function decodeAddressWithHrp(address: string): { hrp: string; pubkey: Uint8Array } {
  const lower = address.trim().toLowerCase();
  const sep = lower.lastIndexOf('1');
  if (sep < 1 || sep + 7 > lower.length) {
    throw new Error('Malformed Bech32m address');
  }
  const hrp = lower.slice(0, sep);
  if (hrp !== 'sdr' && hrp !== 'tsdr' && hrp !== 'rsdr') {
    throw new Error(`Unsupported address HRP '${hrp}'`);
  }
  const dataPart = lower.slice(sep + 1);
  const decoded: number[] = [];
  for (let i = 0; i < dataPart.length; i++) {
    const idx = BECH32_CHARSET.indexOf(dataPart[i]);
    if (idx === -1) {
      throw new Error('Invalid Bech32m character');
    }
    decoded.push(idx);
  }
  if (bech32Polymod(hrpExpand(hrp).concat(decoded)) !== BECH32M_CONST) {
    throw new Error('Invalid Bech32m checksum');
  }
  const payload = decoded.slice(0, decoded.length - 6);
  const bytes8 = convertBits(payload, 5, 8, false);
  if (bytes8.length !== 32) {
    throw new Error('Invalid decoded public key length');
  }
  return { hrp, pubkey: new Uint8Array(bytes8) };
}

export function decodeAddressForHrp(address: string, expectedHrp: string): Uint8Array {
  const { hrp, pubkey } = decodeAddressWithHrp(address);
  if (hrp !== expectedHrp) {
    throw new Error(`Network mismatch: expected '${expectedHrp}1...', got '${hrp}1...'`);
  }
  return pubkey;
}

export function generateMnemonic(wordCount: 18 | 24 = 18): string {
  const entropyLen = wordCount === 18 ? 16 : 23;
  const checksumLen = wordCount === 18 ? 2 : 1;
  const entropy = new Uint8Array(entropyLen);
  crypto.getRandomValues(entropy);
  const checksum = sha256(entropy);
  const all = new Uint8Array(entropyLen + checksumLen);
  all.set(entropy, 0);
  all.set(checksum.slice(0, checksumLen), entropyLen);
  return Array.from(all, (b) => MNEMONIC_WORDS[b]).join(' ');
}

export function validateMnemonic(phrase: string): Uint8Array {
  const words = phrase.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length !== 18 && words.length !== 24) {
    throw new Error(`Expected 18 or 24 mnemonic words, got ${words.length}`);
  }
  const entropyLen = words.length === 18 ? 16 : 23;
  const checksumLen = words.length === 18 ? 2 : 1;
  const bytes = new Uint8Array(words.length);
  for (let i = 0; i < words.length; i++) {
    const idx = MNEMONIC_WORDS.indexOf(words[i]);
    if (idx === -1) {
      throw new Error(`Unknown mnemonic word "${words[i]}"`);
    }
    bytes[i] = idx;
  }
  const entropy = bytes.slice(0, entropyLen);
  const expected = sha256(entropy);
  for (let i = 0; i < checksumLen; i++) {
    if (bytes[entropyLen + i] !== expected[i]) {
      throw new Error('Mnemonic checksum verification failed');
    }
  }
  return entropy;
}

export function deriveSeedFromMnemonic(phrase: string, passphrase = ''): Uint8Array {
  const entropy = validateMnemonic(phrase);
  const enc = new TextEncoder();
  const passBytes = enc.encode(passphrase);

  const prefix = enc.encode('SardhanSeedDerivationV1:');
  const colon = enc.encode(':');
  const initBuf = new Uint8Array(prefix.length + entropy.length + colon.length + passBytes.length);
  initBuf.set(prefix, 0);
  initBuf.set(entropy, prefix.length);
  initBuf.set(colon, prefix.length + entropy.length);
  initBuf.set(passBytes, prefix.length + entropy.length + colon.length);

  let state: Uint8Array = sha256(initBuf);
  const roundBuf = new Uint8Array(32 + 4 + passBytes.length);
  const view = new DataView(roundBuf.buffer);
  for (let round = 0; round < 2048; round++) {
    roundBuf.set(state, 0);
    view.setUint32(32, round, true);
    roundBuf.set(passBytes, 36);
    state = sha256(roundBuf);
  }
  return state;
}

export function deriveChildSecret(masterSeed: Uint8Array, account = 0, index = 0): Uint8Array {
  const ctx = new TextEncoder().encode('SardhanHDKeyV1');
  const payload = new Uint8Array(ctx.length + 8);
  payload.set(ctx, 0);
  const view = new DataView(payload.buffer);
  view.setUint32(ctx.length, account, true);
  view.setUint32(ctx.length + 4, index, true);
  return blake3(payload, { key: masterSeed });
}

export interface LocalWalletAccount {
  mnemonic: string;
  network: SdrNetworkName;
  hrp: string;
  chainId: number;
  secretKeyHex: string;
  publicKeyHex: string;
  address: string;
}

export function deriveWalletFromMnemonic(
  mnemonic: string,
  network: SdrNetworkName = 'testnet',
  passphrase = ''
): LocalWalletAccount {
  const seed = deriveSeedFromMnemonic(mnemonic, passphrase);
  const secretKey = deriveChildSecret(seed, 0, 0);
  const publicKey = ed25519.getPublicKey(secretKey);
  const hrp = networkHrp(network);
  return {
    mnemonic: mnemonic.trim().toLowerCase().split(/\s+/).join(' '),
    network,
    hrp,
    chainId: networkChainId(network),
    secretKeyHex: bytesToHex(secretKey),
    publicKeyHex: bytesToHex(publicKey),
    address: encodeAddressForHrp(hrp, publicKey),
  };
}

/**
 * Exact integer decimal SDR parser (1 SDR = 100,000,000 Sardhans)
 */
export function parseSdrToSardhans(amountStr: string): bigint {
  const trimmed = amountStr.trim();
  if (!trimmed || trimmed.startsWith('-')) {
    throw new Error('Amount must be positive');
  }
  const parts = trimmed.split('.');
  if (parts.length > 2) {
    throw new Error('Invalid decimal amount');
  }
  const wholeStr = parts[0] === '' ? '0' : parts[0];
  if (!/^\d+$/.test(wholeStr)) {
    throw new Error('Invalid whole SDR digits');
  }
  const fracRaw = parts[1] ?? '';
  if (fracRaw.length > 8 || (fracRaw.length > 0 && !/^\d+$/.test(fracRaw))) {
    throw new Error('Up to 8 decimal digits allowed');
  }
  const fracStr = fracRaw.padEnd(8, '0');
  return BigInt(wholeStr) * SARDHANS_PER_SDR + BigInt(fracStr);
}

export function formatSardhansToSdr(sardhans: bigint | number): string {
  const val = typeof sardhans === 'bigint' ? sardhans : BigInt(Math.trunc(sardhans));
  const whole = val / SARDHANS_PER_SDR;
  const frac = (val % SARDHANS_PER_SDR).toString().padStart(8, '0').replace(/0+$/, '');
  return frac.length > 0 ? `${whole.toString()}.${frac}` : whole.toString();
}

export interface UnsignedInput {
  txidHex: string;
  vout: number;
  sequence?: number;
}

export interface TxOutputSpec {
  valueSardhans: bigint;
  recipientPubKey: Uint8Array;
}

function hashLessThanTarget(hash: Uint8Array, target: Uint8Array): boolean {
  for (let i = 0; i < 32; i++) {
    if (hash[i] < target[i]) return true;
    if (hash[i] > target[i]) return false;
  }
  return false;
}

function computeTxIdBytes(
  chainId: number,
  version: number,
  recentBlockHash: Uint8Array,
  microPowNonce: bigint,
  lockTime: number,
  inputs: { txid: Uint8Array; vout: number; sequence: number }[],
  outputs: TxOutputSpec[]
): Uint8Array {
  const totalLen =
    4 + 4 + 32 + 8 + 4 + 4 + inputs.length * 40 + 4 + outputs.length * 40;
  const buf = new Uint8Array(totalLen);
  const view = new DataView(buf.buffer);
  let off = 0;
  view.setUint32(off, chainId, true); off += 4;
  view.setUint32(off, version, true); off += 4;
  buf.set(recentBlockHash, off); off += 32;
  view.setBigUint64(off, microPowNonce, true); off += 8;
  view.setUint32(off, lockTime, true); off += 4;
  view.setUint32(off, inputs.length, true); off += 4;
  for (const inp of inputs) {
    buf.set(inp.txid, off); off += 32;
    view.setUint32(off, inp.vout, true); off += 4;
    view.setUint32(off, inp.sequence, true); off += 4;
  }
  view.setUint32(off, outputs.length, true); off += 4;
  for (const out of outputs) {
    view.setBigUint64(off, out.valueSardhans, true); off += 8;
    buf.set(out.recipientPubKey, off); off += 32;
  }
  return blake3(buf);
}

function computeSignatureDigest(
  chainId: number,
  version: number,
  recentBlockHash: Uint8Array,
  lockTime: number,
  inputIndex: number,
  input: { txid: Uint8Array; vout: number; sequence: number },
  outputs: TxOutputSpec[]
): Uint8Array {
  const totalLen = 4 + 4 + 32 + 4 + 4 + 40 + 4 + outputs.length * 40;
  const buf = new Uint8Array(totalLen);
  const view = new DataView(buf.buffer);
  let off = 0;
  view.setUint32(off, chainId, true); off += 4;
  view.setUint32(off, version, true); off += 4;
  buf.set(recentBlockHash, off); off += 32;
  view.setUint32(off, lockTime, true); off += 4;
  view.setUint32(off, inputIndex, true); off += 4;
  buf.set(input.txid, off); off += 32;
  view.setUint32(off, input.vout, true); off += 4;
  view.setUint32(off, input.sequence, true); off += 4;
  view.setUint32(off, outputs.length, true); off += 4;
  for (const out of outputs) {
    view.setBigUint64(off, out.valueSardhans, true); off += 8;
    buf.set(out.recipientPubKey, off); off += 32;
  }
  return blake3(buf);
}

export interface SignedTransactionBuild {
  rawTxHex: string;
  txidHex: string;
  microPowNonce: number;
  byteSize: number;
}

/**
 * Construct, solve client micro-PoW, sign locally with Ed25519, and serialize binary Transaction
 */
export function buildAndSignZeroFeeTransaction(params: {
  chainId: number;
  recentBlockHashHex: string;
  microPowTargetHex: string;
  inputs: UnsignedInput[];
  outputs: TxOutputSpec[];
  secretKeyHex: string;
  maxIterations?: number;
}): SignedTransactionBuild {
  const secretKey = hexToBytes(params.secretKeyHex);
  const publicKey = ed25519.getPublicKey(secretKey);
  const recentBlockHash = hexToBytes(params.recentBlockHashHex);
  const target = hexToBytes(params.microPowTargetHex);
  const version = 1;
  const lockTime = 0;

  const normInputs = params.inputs.map((inp) => ({
    txid: hexToBytes(inp.txidHex),
    vout: inp.vout,
    sequence: inp.sequence ?? 0xffffffff,
  }));

  const maxIter = params.maxIterations ?? 500_000;
  let solvedNonce = -1;
  let txidBytes: Uint8Array = new Uint8Array(32);

  for (let nonce = 1; nonce <= maxIter; nonce++) {
    const candidate = computeTxIdBytes(
      params.chainId,
      version,
      recentBlockHash,
      BigInt(nonce),
      lockTime,
      normInputs,
      params.outputs
    );
    if (hashLessThanTarget(candidate, target)) {
      solvedNonce = nonce;
      txidBytes = candidate;
      break;
    }
  }

  if (solvedNonce < 0) {
    throw new Error('Could not solve anti-spam micro-PoW within iteration budget');
  }

  const witnesses: { signature: Uint8Array; publicKey: Uint8Array }[] = [];
  for (let i = 0; i < normInputs.length; i++) {
    const digest = computeSignatureDigest(
      params.chainId,
      version,
      recentBlockHash,
      lockTime,
      i,
      normInputs[i],
      params.outputs
    );
    const sig = ed25519.sign(digest, secretKey);
    witnesses.push({ signature: sig, publicKey });
  }

  const bodyLen =
    4 +
    4 +
    32 +
    8 +
    4 +
    4 +
    normInputs.length * 40 +
    4 +
    params.outputs.length * 40 +
    4 +
    witnesses.length * (4 + 64 + 32);
  const raw = new Uint8Array(bodyLen);
  const view = new DataView(raw.buffer);
  let off = 0;
  view.setUint32(off, params.chainId, true); off += 4;
  view.setUint32(off, version, true); off += 4;
  raw.set(recentBlockHash, off); off += 32;
  view.setBigUint64(off, BigInt(solvedNonce), true); off += 8;
  view.setUint32(off, lockTime, true); off += 4;

  view.setUint32(off, normInputs.length, true); off += 4;
  for (const inp of normInputs) {
    raw.set(inp.txid, off); off += 32;
    view.setUint32(off, inp.vout, true); off += 4;
    view.setUint32(off, inp.sequence, true); off += 4;
  }

  view.setUint32(off, params.outputs.length, true); off += 4;
  for (const out of params.outputs) {
    view.setBigUint64(off, out.valueSardhans, true); off += 8;
    raw.set(out.recipientPubKey, off); off += 32;
  }

  view.setUint32(off, witnesses.length, true); off += 4;
  for (const w of witnesses) {
    view.setUint32(off, 64, true); off += 4;
    raw.set(w.signature, off); off += 64;
    raw.set(w.publicKey, off); off += 32;
  }

  return {
    rawTxHex: bytesToHex(raw),
    txidHex: bytesToHex(txidBytes),
    microPowNonce: solvedNonce,
    byteSize: raw.length,
  };
}

/**
 * ISO-18004 QR Code Encoder (Version 4-L, 33x33 modules, Byte mode, up to 78 bytes, GF(256) ECC)
 * Matches `sdr-wallet` native CLI encoder.
 */
function rsEncode(data: number[], ecLen: number): number[] {
  const exp = new Uint8Array(512);
  const log = new Uint8Array(256);
  let x = 1;
  for (let i = 0; i < 255; i++) {
    exp[i] = x;
    log[x] = i;
    x <<= 1;
    if ((x & 0x100) !== 0) {
      x ^= 0x11d;
    }
  }
  for (let i = 255; i < 512; i++) {
    exp[i] = exp[i - 255];
  }
  const gfMul = (a: number, b: number): number =>
    a === 0 || b === 0 ? 0 : exp[log[a] + log[b]];

  let gen = [1];
  for (let i = 0; i < ecLen; i++) {
    const next = new Array(gen.length + 1).fill(0);
    for (let j = 0; j < gen.length; j++) {
      next[j] ^= gen[j];
      next[j + 1] ^= gfMul(gen[j], exp[i]);
    }
    gen = next;
  }

  const msg = new Array(data.length + ecLen).fill(0);
  for (let i = 0; i < data.length; i++) msg[i] = data[i];
  for (let i = 0; i < data.length; i++) {
    const coef = msg[i];
    if (coef !== 0) {
      for (let j = 0; j < gen.length; j++) {
        msg[i + j] ^= gfMul(gen[j], coef);
      }
    }
  }
  return msg.slice(data.length);
}

export function encodeQrVersion4L(text: string): boolean[][] | null {
  const data = new TextEncoder().encode(text);
  if (data.length > 78) return null;

  const bits: boolean[] = [];
  const pushBits = (val: number, count: number) => {
    for (let i = count - 1; i >= 0; i--) {
      bits.push(((val >> i) & 1) === 1);
    }
  };
  pushBits(0b0100, 4);
  pushBits(data.length, 8);
  for (const b of data) pushBits(b, 8);

  const totalDataBits = 80 * 8;
  const termLen = Math.min(totalDataBits - bits.length, 4);
  for (let i = 0; i < termLen; i++) bits.push(false);
  while (bits.length % 8 !== 0) bits.push(false);

  const codewords: number[] = [];
  for (let i = 0; i < bits.length; i += 8) {
    let byte = 0;
    for (let j = 0; j < 8; j++) {
      byte = (byte << 1) | (bits[i + j] ? 1 : 0);
    }
    codewords.push(byte);
  }
  const padBytes = [0xec, 0x11];
  let pIdx = 0;
  while (codewords.length < 80) {
    codewords.push(padBytes[pIdx % 2]);
    pIdx++;
  }
  codewords.push(...rsEncode(codewords, 20));

  const size = 33;
  const grid: boolean[][] = Array.from({ length: size }, () => Array(size).fill(false));
  const reserved: boolean[][] = Array.from({ length: size }, () => Array(size).fill(false));

  const placeFinder = (row: number, col: number) => {
    for (let dy = 0; dy < 7; dy++) {
      for (let dx = 0; dx < 7; dx++) {
        const dark =
          dy === 0 || dy === 6 || dx === 0 || dx === 6 || (dy >= 2 && dy <= 4 && dx >= 2 && dx <= 4);
        grid[row + dy][col + dx] = dark;
        reserved[row + dy][col + dx] = true;
      }
    }
  };
  placeFinder(0, 0);
  placeFinder(0, size - 7);
  placeFinder(size - 7, 0);

  for (let i = 0; i < 8; i++) {
    reserved[7][i] = true;
    reserved[i][7] = true;
    reserved[7][size - 1 - i] = true;
    reserved[i][size - 8] = true;
    reserved[size - 8][i] = true;
    reserved[size - 1 - i][7] = true;
  }

  for (let i = 8; i < size - 8; i++) {
    const dark = i % 2 === 0;
    grid[6][i] = dark;
    reserved[6][i] = true;
    grid[i][6] = dark;
    reserved[i][6] = true;
  }

  for (let dy = 0; dy < 5; dy++) {
    for (let dx = 0; dx < 5; dx++) {
      const dark = dy === 0 || dy === 4 || dx === 0 || dx === 4 || (dy === 2 && dx === 2);
      grid[24 + dy][24 + dx] = dark;
      reserved[24 + dy][24 + dx] = true;
    }
  }

  grid[size - 8][8] = true;
  reserved[size - 8][8] = true;
  for (let i = 0; i < 9; i++) {
    if (i !== 6) {
      reserved[8][i] = true;
      reserved[i][8] = true;
    }
  }
  for (let i = 0; i < 8; i++) {
    reserved[8][size - 1 - i] = true;
    reserved[size - 1 - i][8] = true;
  }

  const bitStream: boolean[] = [];
  for (const cw of codewords) {
    for (let b = 7; b >= 0; b--) {
      bitStream.push(((cw >> b) & 1) === 1);
    }
  }

  let bitIdx = 0;
  let col = size - 1;
  let upward = true;
  while (col > 0) {
    if (col === 6) col -= 1;
    for (let step = 0; step < size; step++) {
      const r = upward ? size - 1 - step : step;
      for (let dx = 0; dx < 2; dx++) {
        const c = col - dx;
        if (!reserved[r][c]) {
          const rawBit = bitIdx < bitStream.length ? bitStream[bitIdx] : false;
          bitIdx++;
          grid[r][c] = rawBit !== ((r + c) % 2 === 0);
        }
      }
    }
    upward = !upward;
    col -= 2;
  }

  const fmt = 0x77c4;
  for (let i = 0; i < 15; i++) {
    const dark = ((fmt >> (14 - i)) & 1) === 1;
    if (i < 6) grid[8][i] = dark;
    else if (i === 6) grid[8][7] = dark;
    else if (i === 7) grid[8][8] = dark;
    else if (i === 8) grid[7][8] = dark;
    else grid[14 - i][8] = dark;

    if (i < 7) grid[size - 1 - i][8] = dark;
    else grid[8][size - 15 + i] = dark;
  }

  return grid;
}
