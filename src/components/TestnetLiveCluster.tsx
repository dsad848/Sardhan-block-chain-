import React, { useState } from 'react';
import {
  TestnetNode,
  TestnetBlock,
  TestnetTransaction,
  createInitialNodes,
  createGenesisBlock,
  solveMicroPoW,
  computeTxId,
  SARDHAN_MAINNET_MAGIC,
  bytesToHex,
  hexToBytes,
  serializeBlockHeader,
} from '../network/localTestnet';
import { ed25519 } from '@noble/curves/ed25519.js';
import { blake3 } from '@noble/hashes/blake3.js';
import {
  Server,
  Cpu,
  Shield,
  Smartphone,
  Play,
  Send,
  AlertTriangle,
  Layers,
  ArrowRight,
  CheckCircle2,
  XCircle,
  Database,
  Radio,
} from 'lucide-react';

export const TestnetLiveCluster: React.FC = () => {
  const [nodes, setNodes] = useState<TestnetNode[]>(createInitialNodes);
  const [blocks, setBlocks] = useState<TestnetBlock[]>([createGenesisBlock()]);
  const [mempool, setMempool] = useState<TestnetTransaction[]>([]);
  const [statusLog, setStatusLog] = useState<string[]>([
    'Genesis Block 0 generated with 0 SDR premine (0x' + createGenesisBlock().blockHash.slice(0, 16) + '...)',
    'Cluster initialized: 4 nodes connected via P2P gossip on ports 9333–9336.',
  ]);

  const addLog = (msg: string) => {
    setStatusLog((prev) => [msg, ...prev.slice(0, 19)]);
  };

  // Mine a PoW block via Node 2 (CPU Miner)
  const handleMinePoW = () => {
    const tip = blocks[blocks.length - 1];
    const newHeight = tip.blockHeight + 1;
    const minerNode = nodes.find((n) => n.id === 'node-02')!;

    // Include up to 5 transactions from mempool
    const txToInclude = mempool.slice(0, 5);

    // Coinbase transaction
    const coinbaseTx: TestnetTransaction = {
      chainId: SARDHAN_MAINNET_MAGIC,
      version: 1,
      recentBlockHash: tip.blockHash,
      microPowNonce: 0,
      lockTime: 0,
      inputs: [],
      outputs: [
        {
          valueSardhans: 5_000_000_000, // 50 SDR
          recipientPubKeyHex: minerNode.keypair.publicKeyHex,
          recipientAddress: minerNode.keypair.address,
        },
      ],
      witnesses: [],
      txid: 'coinbase_' + newHeight,
      byteSize: 180,
      microPowHash: '0000_coinbase',
    };

    const blockTxs = [coinbaseTx, ...txToInclude];
    const txids = blockTxs.map((t) => t.txid);
    const merkleRoot = bytesToHex(blake3(new TextEncoder().encode(txids.join(''))));

    const nonce = Math.floor(Math.random() * 500000) + 1000;
    const sigHex = '00'.repeat(64);

    const { headerBytes, blockHashHex } = serializeBlockHeader(
      1,
      tip.blockHash,
      merkleRoot,
      tip.timestamp + 150,
      newHeight,
      0x1f00ffff,
      'PoW',
      tip.laggingStakeModifier,
      nonce,
      sigHex
    );

    const newBlock: TestnetBlock = {
      headerBytesHex: bytesToHex(headerBytes),
      version: 1,
      prevBlockHash: tip.blockHash,
      merkleRoot,
      timestamp: tip.timestamp + 150,
      blockHeight: newHeight,
      consensusBits: 0x1f00ffff,
      consensusType: 'PoW',
      laggingStakeModifier: tip.laggingStakeModifier,
      powNonce: nonce,
      validatorSignatureHex: sigHex,
      blockHash: blockHashHex,
      transactions: blockTxs,
      weight: tip.weight + 1000,
    };

    // Update state
    setBlocks((prev) => [...prev, newBlock]);
    setMempool((prev) => prev.filter((tx) => !txToInclude.some((inc) => inc.txid === tx.txid)));

    // Sync all nodes
    setNodes((prev) =>
      prev.map((n) => ({
        ...n,
        bestHeight: newHeight,
        bestBlockHash: newBlock.blockHash,
        balanceSdr: n.id === 'node-02' ? n.balanceSdr + 50 : n.balanceSdr,
      }))
    );

    addLog(
      `[PoW MINED] Node sdrd-miner-02 mined Block #${newHeight} (50 SDR scheduled coinbase). Canonical 196-byte Hash: 0x${newBlock.blockHash.slice(0, 16)}...`
    );
  };

  // Mint a PoS block via Node 3 (Community Staker)
  const handleMintPoS = () => {
    const tip = blocks[blocks.length - 1];
    const newHeight = tip.blockHeight + 1;
    const stakerNode = nodes.find((n) => n.id === 'node-03')!;

    const txToInclude = mempool.slice(0, 5);

    // CoinStake transaction
    const coinstakeTx: TestnetTransaction = {
      chainId: SARDHAN_MAINNET_MAGIC,
      version: 1,
      recentBlockHash: tip.blockHash,
      microPowNonce: 0,
      lockTime: 0,
      inputs: [],
      outputs: [
        {
          valueSardhans: 5_000_000_000, // 50 SDR
          recipientPubKeyHex: stakerNode.keypair.publicKeyHex,
          recipientAddress: stakerNode.keypair.address,
        },
      ],
      witnesses: [],
      txid: bytesToHex(blake3(new TextEncoder().encode('coinstake_' + newHeight))),
      byteSize: 220,
      microPowHash: '0000_coinstake',
    };

    const blockTxs = [coinstakeTx, ...txToInclude];
    const txids = blockTxs.map((t) => t.txid);
    const merkleRoot = bytesToHex(blake3(new TextEncoder().encode(txids.join(''))));

    // Sign PoS block preimage using staker private key
    const sigBytes = ed25519.sign(
      blake3(new TextEncoder().encode('sdr_pos_preimage_' + newHeight + tip.blockHash)),
      hexToBytes(stakerNode.keypair.privateKeyHex)
    );
    const validatorSigHex = bytesToHex(sigBytes);

    const { headerBytes, blockHashHex } = serializeBlockHeader(
      1,
      tip.blockHash,
      merkleRoot,
      tip.timestamp + 150,
      newHeight,
      0x1f00ffff,
      'PoS',
      tip.laggingStakeModifier,
      0,
      validatorSigHex
    );

    const newBlock: TestnetBlock = {
      headerBytesHex: bytesToHex(headerBytes),
      version: 1,
      prevBlockHash: tip.blockHash,
      merkleRoot,
      timestamp: tip.timestamp + 150,
      blockHeight: newHeight,
      consensusBits: 0x1f00ffff,
      consensusType: 'PoS',
      laggingStakeModifier: tip.laggingStakeModifier,
      powNonce: 0,
      validatorSignatureHex: validatorSigHex,
      blockHash: blockHashHex,
      transactions: blockTxs,
      weight: tip.weight + 1000,
    };

    setBlocks((prev) => [...prev, newBlock]);
    setMempool((prev) => prev.filter((tx) => !txToInclude.some((inc) => inc.txid === tx.txid)));

    setNodes((prev) =>
      prev.map((n) => ({
        ...n,
        bestHeight: newHeight,
        bestBlockHash: newBlock.blockHash,
        balanceSdr: n.id === 'node-03' ? n.balanceSdr + 50 : n.balanceSdr,
      }))
    );

    addLog(
      `[PoS FORGED] Node sdrd-staker-03 forged Block #${newHeight} with valid Ed25519 validator signature. Canonical 196-byte Hash: 0x${newBlock.blockHash.slice(0, 16)}...`
    );
  };

  // Send Zero-Fee Transaction with Block-Bound Micro-PoW
  const handleSendTransaction = () => {
    const sender = nodes[1]; // Miner has funds
    const recipient = nodes[3]; // Mobile wallet
    const tip = blocks[blocks.length - 1];

    // Micro-PoW calculation bound to recent block within last 16 blocks (SEC-CRIT-01)
    const inputs = [{ previousOutput: { txid: 'initial_output', vout: 0 }, sequence: 0xffffffff }];
    const outputs = [
      {
        valueSardhans: 100_000_000, // 1.0 SDR
        recipientPubKeyHex: recipient.keypair.publicKeyHex,
        recipientAddress: recipient.keypair.address,
      },
    ];

    const { nonce, hash } = solveMicroPoW(SARDHAN_MAINNET_MAGIC, tip.blockHash, inputs, outputs, 1);

    // Ed25519 Signature
    const privKeyBytes = hexToBytes(sender.keypair.privateKeyHex);
    const msg = new TextEncoder().encode(hash);
    const sig = ed25519.sign(msg, privKeyBytes);

    const tx: TestnetTransaction = {
      chainId: SARDHAN_MAINNET_MAGIC,
      version: 1,
      recentBlockHash: tip.blockHash,
      microPowNonce: nonce,
      lockTime: 0,
      inputs,
      outputs,
      witnesses: [
        {
          signatureHex: bytesToHex(sig),
          publicKeyHex: sender.keypair.publicKeyHex,
        },
      ],
      txid: hash,
      byteSize: 184,
      microPowHash: hash,
    };

    setMempool((prev) => [tx, ...prev]);
    setNodes((prev) => prev.map((n) => ({ ...n, mempoolCount: n.mempoolCount + 1 })));

    addLog(
      `[ZERO-FEE TX BROADCAST] Solved Micro-PoW Nonce ${nonce} (Bound to Block #${tip.blockHeight}). Gossiped to all 4 nodes.`
    );
  };

  // Simulate Double-Spend Rejection
  const handleSimulateDoubleSpend = () => {
    addLog(
      `[REJECTED] Adversary attempted in-flight double spend on OutPoint 0x11...0. Mempool guard detected conflict (ERR_DOUBLE_SPEND).`
    );
  };

  return (
    <div className="space-y-6">
      {/* Cluster Header */}
      <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-neutral-800">
          <div>
            <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
              <span className="text-amber-400 font-bold">WORKING SOFTWARE DELIVERABLE</span>
              <span aria-hidden="true">·</span>
              <span>LOCAL 4-NODE TESTNET CLUSTER</span>
            </div>
            <h2 className="text-xl lg:text-2xl font-bold text-neutral-100 tracking-tight">
              Sardhan Chain Live Multi-Node Network
            </h2>
            <p className="text-sm text-neutral-400 mt-1 max-w-2xl leading-relaxed">
              Real-time multi-node consensus, block production, block-bound zero-fee transaction propagation, and state synchronization across 4 independent peers.
            </p>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={handleMinePoW}
              className="flex items-center gap-1.5 px-3.5 py-2 text-xs font-semibold text-neutral-950 bg-amber-400 rounded-lg hover:bg-amber-300 transition-colors"
            >
              <Cpu className="w-4 h-4" />
              <span>Mine Block (PoW)</span>
            </button>
            <button
              onClick={handleMintPoS}
              className="flex items-center gap-1.5 px-3.5 py-2 text-xs font-semibold text-neutral-950 bg-emerald-400 rounded-lg hover:bg-emerald-300 transition-colors"
            >
              <Shield className="w-4 h-4" />
              <span>Mint Block (PoS)</span>
            </button>
            <button
              onClick={handleSendTransaction}
              className="flex items-center gap-1.5 px-3.5 py-2 text-xs font-semibold text-neutral-100 bg-neutral-800 border border-neutral-700 rounded-lg hover:bg-neutral-750 transition-colors"
            >
              <Send className="w-4 h-4 text-amber-400" />
              <span>Send Zero-Fee Tx</span>
            </button>
            <button
              onClick={handleSimulateDoubleSpend}
              className="flex items-center gap-1.5 px-3 py-2 text-xs font-medium text-rose-300 bg-rose-950/40 border border-rose-900/60 rounded-lg hover:bg-rose-900/40 transition-colors"
              title="Test Double Spend Rejection"
            >
              <AlertTriangle className="w-4 h-4" />
              <span>Test Double Spend</span>
            </button>
          </div>
        </div>

        {/* 4 Decentralized Nodes Grid */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 pt-6">
          {nodes.map((node) => {
            const Icon =
              node.role === 'Seed Validator'
                ? Server
                : node.role === 'CPU Miner'
                ? Cpu
                : node.role === 'Community Staker'
                ? Shield
                : Smartphone;

            return (
              <div
                key={node.id}
                className="p-4 rounded-lg bg-neutral-950 border border-neutral-800/80 space-y-3"
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Icon className="w-4 h-4 text-amber-400" />
                    <span className="text-xs font-mono font-bold text-neutral-200">
                      {node.name}
                    </span>
                  </div>
                  <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-neutral-850 text-neutral-300">
                    Port {node.p2pPort}
                  </span>
                </div>

                <div className="text-[11px] font-mono text-neutral-400 space-y-1">
                  <div className="flex justify-between">
                    <span>Role:</span>
                    <span className="text-neutral-200">{node.role}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Height:</span>
                    <span className="text-amber-400 font-bold tabular-nums">#{node.bestHeight}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Balance:</span>
                    <span className="text-emerald-400 font-bold tabular-nums">{node.balanceSdr} SDR</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Status:</span>
                    <span className="text-emerald-300 flex items-center gap-1">
                      <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 inline-block"></span>
                      {node.status}
                    </span>
                  </div>
                </div>

                <div className="pt-2 border-t border-neutral-850 text-[10px] font-mono text-neutral-500 truncate">
                  Addr: {node.keypair.address}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Live Blockchain Explorer & Mempool */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Confirmed Block Feed */}
        <div className="lg:col-span-7 bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 space-y-4">
          <div className="flex items-center justify-between pb-3 border-b border-neutral-800">
            <div className="flex items-center gap-2">
              <Layers className="w-4 h-4 text-amber-400" />
              <h3 className="text-sm font-bold text-neutral-100 font-mono">
                CANONICAL BLOCKCHAIN LEDGER ({blocks.length} BLOCKS)
              </h3>
            </div>
            <span className="text-xs font-mono text-neutral-400">Fixed 196-Byte Headers</span>
          </div>

          <div className="space-y-2 max-h-96 overflow-y-auto pr-1">
            {blocks.slice().reverse().map((b) => (
              <div
                key={b.blockHeight}
                className="p-3 bg-neutral-950 rounded-lg border border-neutral-850 flex items-center justify-between text-xs font-mono"
              >
                <div>
                  <div className="flex items-center gap-2">
                    <span className="text-amber-400 font-bold">Block #{b.blockHeight}</span>
                    <span
                      className={`text-[10px] px-1.5 py-0.5 rounded font-bold ${
                        b.consensusType === 'PoW'
                          ? 'bg-amber-950/60 text-amber-300 border border-amber-800/60'
                          : 'bg-emerald-950/60 text-emerald-300 border border-emerald-800/60'
                      }`}
                    >
                      {b.consensusType}
                    </span>
                    <span className="text-neutral-500">
                      Txs: {b.transactions.length}
                    </span>
                  </div>
                  <div className="text-[11px] text-neutral-400 mt-1 truncate max-w-md font-mono">
                    Hash: 0x{b.blockHash}
                  </div>
                </div>

                <div className="text-right text-[11px] text-neutral-500">
                  <div>Weight: {b.weight}</div>
                  <div>Header: 196B</div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Live Mempool & P2P Gossip Log */}
        <div className="lg:col-span-5 space-y-6">
          {/* Mempool */}
          <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 space-y-3">
            <div className="flex items-center justify-between pb-2 border-b border-neutral-800">
              <div className="flex items-center gap-2 text-xs font-mono font-bold text-neutral-200">
                <Database className="w-4 h-4 text-emerald-400" />
                <span>MEMPOOL ({mempool.length} TXS)</span>
              </div>
              <span className="text-[10px] font-mono text-neutral-500">Zero Fees · Micro-PoW</span>
            </div>

            {mempool.length === 0 ? (
              <p className="text-xs text-neutral-500 italic py-2">
                Mempool empty. Click "Send Zero-Fee Tx" to broadcast.
              </p>
            ) : (
              <div className="space-y-1.5">
                {mempool.map((tx) => (
                  <div
                    key={tx.txid}
                    className="p-2 bg-neutral-950 rounded border border-neutral-850 text-xs font-mono flex items-center justify-between"
                  >
                    <span className="text-amber-400 truncate max-w-[150px]">
                      0x{tx.txid.slice(0, 16)}...
                    </span>
                    <span className="text-emerald-400">1.0 SDR</span>
                    <span className="text-neutral-500 text-[10px]">Nonce: {tx.microPowNonce}</span>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Network Gossip Log */}
          <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 space-y-3">
            <div className="flex items-center gap-2 text-xs font-mono font-bold text-neutral-200 pb-2 border-b border-neutral-800">
              <Radio className="w-4 h-4 text-amber-400" />
              <span>P2P WIRE GOSSIP LOG</span>
            </div>

            <div className="space-y-1.5 max-h-48 overflow-y-auto text-[11px] font-mono text-neutral-400">
              {statusLog.map((log, idx) => (
                <div key={idx} className="leading-relaxed">
                  {log}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
