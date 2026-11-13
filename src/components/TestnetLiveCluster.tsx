import React, { useState, useEffect, useCallback } from 'react';
import {
  Server,
  Cpu,
  Shield,
  Search,
  Layers,
  Database,
  RefreshCw,
  WifiOff,
  Wifi,
  ChevronRight,
  Lock,
  Pause,
  Play,
} from 'lucide-react';

interface BlockchainInfo {
  chain: string;
  chain_id: number;
  mainnet_enabled: boolean;
  blocks: number;
  bestblockhash: string;
  difficulty_pow: number;
  difficulty_pos: number;
  difficulty_pow_hex: string;
  difficulty_pos_hex: string;
  network_hashrate_hps: number;
  circulating_supply_sardhans: number;
  circulating_supply: number;
  max_supply: number;
  halving_epoch: number;
  block_reward_sdr: number;
  cumulative_weight: string;
  pow_blocks_64: number;
  pos_blocks_64: number;
  mempool_txs: number;
  utxo_count: number;
  slashed_validators: number;
}

interface NodeHealth {
  status: string;
  uptime_secs: number;
  best_height: number;
  best_block_hash: string;
  peer_count: number;
  mempool_txs: number;
  mempool_bytes: number;
  utxo_count: number;
  mainnet_enabled: boolean;
  chain: string;
}

interface RpcPeerSnapshot {
  addr: string;
  inbound: boolean;
  best_height: number;
  score: number;
  connected_at: number;
  user_agent: string;
}

interface ClusterNodeStatus {
  index: number;
  id: string;
  name: string;
  role: string;
  p2pPort: number;
  rpcPort: number;
  online: boolean;
  error?: string;
  info?: BlockchainInfo;
  health?: NodeHealth;
  peers?: RpcPeerSnapshot[];
  mempoolInfo?: {
    size: number;
    bytes: number;
    max_bytes: number;
    micro_pow_target_hex: string;
    recent_window_blocks: number;
    zero_fee: boolean;
  };
  logs: string[];
}

interface RpcTxJson {
  txid: string;
  chain_id: number;
  version: number;
  recent_block_hash: string;
  micro_pow_nonce: number;
  byte_size: number;
  is_reward_tx: boolean;
  fee_sardhans: number;
  total_output_sdr: number;
  block_height: number | null;
  block_hash: string | null;
  confirmed: boolean;
  inputs: { txid: string; vout: number; sequence: number }[];
  outputs: { vout: number; value_sardhans: number; value_sdr: number; address: string }[];
}

interface RpcBlockJson {
  hash: string;
  height: number;
  version: number;
  prev_block_hash: string;
  merkle_root: string;
  timestamp: number;
  consensus_bits_hex: string;
  consensus_type: 'PoW' | 'PoS';
  lagging_stake_modifier: string;
  pow_nonce: number;
  cumulative_weight: string;
  tx_count: number;
  size_bytes: number;
  transactions: RpcTxJson[];
}

interface ClusterStatusPayload {
  network: string;
  timestamp: string;
  nodes: ClusterNodeStatus[];
  latestBlocks: RpcBlockJson[];
  rawMempool: RpcTxJson[];
}

type InspectorResult =
  | { kind: 'block'; data: RpcBlockJson }
  | { kind: 'tx'; data: RpcTxJson }
  | {
      kind: 'address';
      data: {
        address: string;
        balance_sdr: number;
        spendable_sdr: number;
        utxos: { txid: string; vout: number; value_sdr: number; block_height: number }[];
        history: { block_height: number; direction: string; amount_sardhans: number; txid: string }[];
      };
    };

async function rpcPost(nodeIndex: number, method: string, params: unknown[] = []) {
  const resp = await fetch(`/api/rpc/${nodeIndex}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ method, params }),
  });
  const payload = await resp.json();
  if (payload.error) throw new Error(payload.error.message || 'RPC Error');
  return payload.result;
}

export const TestnetLiveCluster: React.FC = () => {
  const [cluster, setCluster] = useState<ClusterStatusPayload | null>(null);
  const [autoPoll, setAutoPoll] = useState(true);
  const [loading, setLoading] = useState(true);
  const [actionBusy, setActionBusy] = useState(false);
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  const [searchQuery, setSearchQuery] = useState('');
  const [searchError, setSearchError] = useState<string | null>(null);
  const [inspector, setInspector] = useState<InspectorResult | null>(null);

  const fetchCluster = useCallback(async () => {
    try {
      const resp = await fetch('/api/cluster/status');
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const data = (await resp.json()) as ClusterStatusPayload;
      setCluster(data);
    } catch (err) {
      setActionMsg(err instanceof Error ? err.message : 'Failed polling live node cluster');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchCluster();
  }, [fetchCluster]);

  useEffect(() => {
    if (!autoPoll) return;
    const t = setInterval(fetchCluster, 2500);
    return () => clearInterval(t);
  }, [autoPoll, fetchCluster]);

  const handlePartitionNode3 = async () => {
    setActionBusy(true);
    setActionMsg(null);
    try {
      const resp = await fetch('/api/cluster/partition', { method: 'POST' });
      const data = await resp.json();
      if (!resp.ok || !data.ok) throw new Error(data.error || 'Partition command failed');
      setActionMsg('Node 3 (sdrd-staker-03) disconnected from P2P peers via real `disconnectnode` RPC.');
      await fetchCluster();
    } catch (err) {
      setActionMsg(err instanceof Error ? err.message : 'Partition error');
    } finally {
      setActionBusy(false);
    }
  };

  const handleReconnectNode3 = async () => {
    setActionBusy(true);
    setActionMsg(null);
    try {
      const resp = await fetch('/api/cluster/reconnect', { method: 'POST' });
      const data = await resp.json();
      if (!resp.ok || !data.ok) throw new Error(data.error || 'Reconnect command failed');
      setActionMsg('Node 3 reconnected to sdrd-seed-01 & sdrd-miner-02 via `addnode` RPC; header/block sync active.');
      await fetchCluster();
    } catch (err) {
      setActionMsg(err instanceof Error ? err.message : 'Reconnect error');
    } finally {
      setActionBusy(false);
    }
  };

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    const q = searchQuery.trim();
    if (!q) return;
    setSearchError(null);
    setInspector(null);

    try {
      if (/^\d+$/.test(q)) {
        const blk = (await rpcPost(1, 'getblock', [Number(q)])) as RpcBlockJson;
        setInspector({ kind: 'block', data: blk });
      } else if (q.startsWith('tsdr1') || q.startsWith('sdr1') || q.startsWith('rsdr1')) {
        const addr = await rpcPost(1, 'getaddressinfo', [q]);
        setInspector({ kind: 'address', data: addr });
      } else {
        const clean = q.replace(/^0x/, '');
        try {
          const blk = (await rpcPost(1, 'getblock', [clean])) as RpcBlockJson;
          setInspector({ kind: 'block', data: blk });
        } catch {
          const tx = (await rpcPost(1, 'getrawtransaction', [clean])) as RpcTxJson;
          setInspector({ kind: 'tx', data: tx });
        }
      }
    } catch (err) {
      setSearchError(err instanceof Error ? err.message : 'Record not found on node RPC');
    }
  };

  const primaryNode = cluster?.nodes.find((n) => n.online);
  const primaryInfo = primaryNode?.info;
  const blocks = cluster?.latestBlocks ?? [];
  const mempool = cluster?.rawMempool ?? [];

  return (
    <div className="space-y-6">
      {/* Real RPC Summary Header */}
      <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-5">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4 border-b border-neutral-800 pb-5">
          <div className="space-y-1">
            <div className="flex flex-wrap items-center gap-2 text-xs font-mono text-neutral-400">
              <span className="text-emerald-400 font-semibold">LIVE 3-NODE RUST DAEMON CLUSTER</span>
              <span aria-hidden="true">·</span>
              <span>TESTNET MAGIC 0x53445202</span>
              <span aria-hidden="true">·</span>
              <span className="text-amber-400">
                MAINNET_ENABLED = {String(primaryInfo?.mainnet_enabled ?? false).toUpperCase()}
              </span>
            </div>
            <h2 className="text-xl lg:text-2xl font-bold text-neutral-100 tracking-tight">
              Sardhan Live Explorer & P2P Testnet Cluster
            </h2>
            <p className="text-sm text-neutral-400 max-w-3xl">
              Direct JSON-RPC telemetry from three locally running <code className="text-amber-400 font-mono">sdrd</code> daemons communicating over TCP P2P (ports 19333, 19343, 19353). Zero simulated data.
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2 shrink-0 font-mono text-xs">
            <button
              onClick={() => setAutoPoll((v) => !v)}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-neutral-950 border border-neutral-800 text-neutral-300 hover:text-white"
            >
              {autoPoll ? <Pause className="w-3.5 h-3.5 text-amber-400" /> : <Play className="w-3.5 h-3.5 text-emerald-400" />}
              <span>{autoPoll ? 'Pause Poll' : 'Resume Poll'}</span>
            </button>
            <button
              onClick={fetchCluster}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-neutral-950 border border-neutral-800 text-neutral-300 hover:text-white"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
              <span>Refresh</span>
            </button>
            <button
              onClick={handlePartitionNode3}
              disabled={actionBusy}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-rose-950/40 border border-rose-900/60 text-rose-300 hover:bg-rose-900/40 disabled:opacity-50"
            >
              <WifiOff className="w-3.5 h-3.5" />
              <span>Partition Node 3</span>
            </button>
            <button
              onClick={handleReconnectNode3}
              disabled={actionBusy}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-emerald-500 text-neutral-950 font-semibold hover:bg-emerald-400 disabled:opacity-50"
            >
              <Wifi className="w-3.5 h-3.5" />
              <span>Reconnect & Sync</span>
            </button>
          </div>
        </div>

        {actionMsg && (
          <div className="rounded-lg border border-neutral-800 bg-neutral-950 px-4 py-2.5 text-xs font-mono text-amber-300">
            {actionMsg}
          </div>
        )}

        {/* Zero-Pill Telemetry Strip */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 border border-neutral-800 rounded-lg divide-y md:divide-y-0 md:divide-x divide-neutral-800 bg-neutral-950">
          <div className="p-4">
            <div className="text-[10px] font-mono text-neutral-500 uppercase">BEST HEIGHT</div>
            <div className="text-lg font-bold font-mono text-amber-400 tabular-nums mt-1">
              #{primaryInfo?.blocks ?? 0}
            </div>
            <div className="text-[11px] font-mono text-neutral-400 tabular-nums mt-0.5 truncate">
              Tip: 0x{primaryInfo?.bestblockhash?.slice(0, 12) ?? '...'}
            </div>
          </div>

          <div className="p-4">
            <div className="text-[10px] font-mono text-neutral-500 uppercase">SDR SUPPLY</div>
            <div className="text-lg font-bold font-mono text-neutral-100 tabular-nums mt-1">
              {(primaryInfo?.circulating_supply ?? 0).toLocaleString()} SDR
            </div>
            <div className="text-[11px] font-mono text-neutral-400 tabular-nums mt-0.5">
              Cap: 21,000,000 · Reward: {primaryInfo?.block_reward_sdr ?? 50} SDR
            </div>
          </div>

          <div className="p-4">
            <div className="text-[10px] font-mono text-neutral-500 uppercase">MINING & DIFFICULTY</div>
            <div className="text-lg font-bold font-mono text-neutral-100 tabular-nums mt-1">
              {(primaryInfo?.network_hashrate_hps ?? 0).toFixed(2)} H/s
            </div>
            <div className="text-[11px] font-mono text-neutral-400 tabular-nums mt-0.5">
              PoW: {primaryInfo?.difficulty_pow_hex ?? '—'} · PoS: {primaryInfo?.difficulty_pos_hex ?? '—'}
            </div>
          </div>

          <div className="p-4">
            <div className="text-[10px] font-mono text-neutral-500 uppercase">64-BLOCK CHANNEL BALANCE</div>
            <div className="text-lg font-bold font-mono text-emerald-400 tabular-nums mt-1">
              {primaryInfo?.pow_blocks_64 ?? 0} PoW / {primaryInfo?.pos_blocks_64 ?? 0} PoS
            </div>
            <div className="text-[11px] font-mono text-neutral-400 mt-0.5">
              In-window bound [24..40]
            </div>
          </div>

          <div className="p-4">
            <div className="text-[10px] font-mono text-neutral-500 uppercase">LEDGER & SAFETY</div>
            <div className="text-lg font-bold font-mono text-neutral-100 tabular-nums mt-1">
              {primaryInfo?.utxo_count ?? 0} UTXOs
            </div>
            <div className="text-[11px] font-mono text-neutral-400 tabular-nums mt-0.5">
              Mempool: {mempool.length} · Slashed: {primaryInfo?.slashed_validators ?? 0}
            </div>
          </div>
        </div>

        {/* 3 Live Running `sdrd` Nodes */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {(cluster?.nodes ?? []).map((node) => {
            const Icon = node.index === 1 ? Server : node.index === 2 ? Cpu : Shield;
            const peerCount = node.health?.peer_count ?? 0;
            const isolated = node.online && peerCount === 0;
            return (
              <div
                key={node.id}
                className="p-4 rounded-lg bg-neutral-950 border border-neutral-800 space-y-3"
              >
                <div className="flex items-center justify-between border-b border-neutral-800/80 pb-2.5">
                  <div className="flex items-center gap-2">
                    <Icon className="w-4 h-4 text-amber-400" />
                    <div>
                      <div className="text-xs font-mono font-bold text-neutral-100">{node.name}</div>
                      <div className="text-[10px] font-mono text-neutral-500">{node.role}</div>
                    </div>
                  </div>
                  <div className="text-right font-mono text-[11px]">
                    <div
                      className={
                        !node.online
                          ? 'text-rose-400 font-semibold'
                          : isolated
                          ? 'text-amber-400 font-semibold'
                          : 'text-emerald-400 font-semibold'
                      }
                    >
                      {!node.online ? 'OFFLINE' : isolated ? 'ISOLATED (0 PEERS)' : `SYNCED (${peerCount} PEERS)`}
                    </div>
                    <div className="text-neutral-500 text-[10px] tabular-nums">
                      P2P :{node.p2pPort} · RPC :{node.rpcPort}
                    </div>
                  </div>
                </div>

                {node.online && node.info && node.health ? (
                  <div className="text-[11px] font-mono space-y-1.5 text-neutral-400">
                    <div className="flex justify-between">
                      <span>Best Height</span>
                      <span className="text-amber-400 font-bold tabular-nums">#{node.info.blocks}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Tip Hash</span>
                      <span className="text-neutral-200 tabular-nums">
                        0x{node.info.bestblockhash.slice(0, 14)}...
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span>Mempool / UTXOs</span>
                      <span className="text-neutral-200 tabular-nums">
                        {node.health.mempool_txs} tx / {node.info.utxo_count} utxo
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span>Node Uptime</span>
                      <span className="text-neutral-300 tabular-nums">{node.health.uptime_secs}s</span>
                    </div>
                  </div>
                ) : (
                  <div className="text-xs font-mono text-rose-400 py-2">
                    {node.error || 'Waiting for sdrd RPC readiness...'}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Live Explorer Search & Inspector */}
      <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-4">
        <form onSubmit={handleSearch} className="flex flex-col sm:flex-row gap-3">
          <div className="relative flex-1">
            <Search className="w-4 h-4 text-neutral-500 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search live ledger by Block Height (e.g. 1), Block Hash, TxID, or Bech32m Address (tsdr1...)"
              className="w-full rounded-lg bg-neutral-950 border border-neutral-800 pl-9 pr-3 py-2 text-xs font-mono text-neutral-200 focus:outline-none focus:border-amber-500"
            />
          </div>
          <button
            type="submit"
            className="px-4 py-2 rounded-lg bg-amber-400 text-neutral-950 text-xs font-bold hover:bg-amber-300 transition-colors shrink-0"
          >
            Inspect On-Chain Data
          </button>
        </form>

        {searchError && (
          <div className="rounded-lg border border-rose-800/70 bg-rose-950/30 px-4 py-2.5 text-xs font-mono text-rose-300">
            {searchError}
          </div>
        )}

        {inspector && (
          <div className="rounded-lg border border-neutral-700 bg-neutral-950 p-5 space-y-3">
            <div className="flex items-center justify-between border-b border-neutral-800 pb-2">
              <span className="text-xs font-mono font-bold text-amber-400 uppercase">
                {inspector.kind === 'block'
                  ? `Block #${inspector.data.height} (${inspector.data.consensus_type})`
                  : inspector.kind === 'tx'
                  ? `Transaction 0x${inspector.data.txid.slice(0, 16)}...`
                  : `Address ${inspector.data.address}`}
              </span>
              <button
                onClick={() => setInspector(null)}
                className="text-xs font-mono text-neutral-400 hover:text-white"
              >
                Close
              </button>
            </div>

            {inspector.kind === 'block' && (
              <div className="space-y-3 text-xs font-mono">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-neutral-400">
                  <div className="truncate">Hash: <span className="text-neutral-200">0x{inspector.data.hash}</span></div>
                  <div className="truncate">Prev: <span className="text-neutral-200">0x{inspector.data.prev_block_hash}</span></div>
                  <div className="truncate">Merkle: <span className="text-neutral-200">0x{inspector.data.merkle_root}</span></div>
                  <div className="truncate">Lagging Modifier (H-100): <span className="text-neutral-200">0x{inspector.data.lagging_stake_modifier}</span></div>
                  <div>Bits: <span className="text-neutral-200">{inspector.data.consensus_bits_hex}</span> · Nonce: <span className="text-neutral-200">{inspector.data.pow_nonce}</span></div>
                  <div>Size: <span className="text-neutral-200">{inspector.data.size_bytes} B</span> · Weight: <span className="text-neutral-200">{inspector.data.cumulative_weight}</span></div>
                </div>
                <div className="border-t border-neutral-800 pt-2 space-y-1.5">
                  <div className="text-neutral-500 uppercase text-[10px]">
                    Transactions ({inspector.data.transactions.length})
                  </div>
                  {inspector.data.transactions.map((tx) => (
                    <div key={tx.txid} className="flex items-center justify-between bg-neutral-900/60 px-3 py-1.5 rounded">
                      <span className="text-neutral-300 truncate max-w-md">
                        {tx.is_reward_tx ? '[COINBASE/STAKE] ' : '[ZERO-FEE TX] '}0x{tx.txid}
                      </span>
                      <span className="text-emerald-400 tabular-nums shrink-0">
                        {tx.total_output_sdr.toFixed(4)} SDR · Fee {tx.fee_sardhans}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {inspector.kind === 'tx' && (
              <div className="space-y-2 text-xs font-mono text-neutral-300">
                <div>TxID: 0x{inspector.data.txid}</div>
                <div>
                  ChainID: 0x{inspector.data.chain_id.toString(16)} · Micro-PoW Nonce: {inspector.data.micro_pow_nonce} · Fee: {inspector.data.fee_sardhans} Sardhans
                </div>
                <div>
                  Status: {inspector.data.confirmed ? `Confirmed in Block #${inspector.data.block_height}` : 'Unconfirmed in Mempool'}
                </div>
                <div className="border-t border-neutral-800 pt-2 space-y-1">
                  {inspector.data.outputs.map((o) => (
                    <div key={o.vout} className="flex justify-between">
                      <span className="text-neutral-400">Output #{o.vout} → {o.address}</span>
                      <span className="text-emerald-400 tabular-nums">{o.value_sdr.toFixed(4)} SDR</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {inspector.kind === 'address' && (
              <div className="space-y-2 text-xs font-mono text-neutral-300">
                <div>
                  Balance: <span className="text-amber-400 font-bold">{inspector.data.balance_sdr.toFixed(4)} SDR</span>{' '}
                  (Spendable: {inspector.data.spendable_sdr.toFixed(4)} SDR · UTXOs: {inspector.data.utxos.length})
                </div>
                <div className="border-t border-neutral-800 pt-2 space-y-1 max-h-48 overflow-y-auto">
                  {inspector.data.history.map((h, i) => (
                    <div key={`${h.txid}-${i}`} className="flex justify-between">
                      <span>#{h.block_height} · {h.direction} · 0x{h.txid.slice(0, 16)}...</span>
                      <span className="text-emerald-400 tabular-nums">{(h.amount_sardhans / 1e8).toFixed(4)} SDR</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Latest Blocks & Zero-Fee Mempool */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-7 rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-4">
          <div className="flex items-center justify-between border-b border-neutral-800 pb-3">
            <div className="flex items-center gap-2">
              <Layers className="w-4 h-4 text-amber-400" />
              <h3 className="text-xs font-mono font-bold text-neutral-100 uppercase">
                Latest Canonical Blocks ({blocks.length} shown)
              </h3>
            </div>
            <span className="text-[11px] font-mono text-neutral-500">196-Byte Headers · Click to inspect</span>
          </div>

          <div className="divide-y divide-neutral-800/70 max-h-[420px] overflow-y-auto">
            {blocks.map((b) => (
              <button
                key={b.hash}
                onClick={() => setInspector({ kind: 'block', data: b })}
                className="w-full text-left py-2.5 px-2 hover:bg-neutral-950/80 transition-colors flex items-center justify-between gap-4 text-xs font-mono"
              >
                <div className="min-w-0 space-y-0.5">
                  <div className="flex items-center gap-2">
                    <span className="text-amber-400 font-bold tabular-nums">#{b.height}</span>
                    <span
                      className={`font-semibold ${
                        b.consensus_type === 'PoW' ? 'text-amber-300' : 'text-emerald-400'
                      }`}
                    >
                      {b.consensus_type}
                    </span>
                    <span className="text-neutral-500">· {b.tx_count} tx · {b.size_bytes} B</span>
                  </div>
                  <div className="text-[11px] text-neutral-400 truncate max-w-md">
                    0x{b.hash}
                  </div>
                </div>
                <ChevronRight className="w-4 h-4 text-neutral-600 shrink-0" />
              </button>
            ))}
          </div>
        </div>

        <div className="lg:col-span-5 space-y-6">
          <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-3">
            <div className="flex items-center justify-between border-b border-neutral-800 pb-3">
              <div className="flex items-center gap-2 text-xs font-mono font-bold text-neutral-100">
                <Database className="w-4 h-4 text-emerald-400" />
                <span>ZERO-FEE MEMPOOL ({mempool.length})</span>
              </div>
              <span className="text-[11px] font-mono text-neutral-500">Max 64 MiB</span>
            </div>

            {primaryNode?.mempoolInfo && (
              <div className="text-[11px] font-mono text-neutral-400 space-y-1 pb-2 border-b border-neutral-800/70">
                <div>
                  Micro-PoW Target: 0x{primaryNode.mempoolInfo.micro_pow_target_hex.slice(0, 16)}...
                </div>
                <div>
                  Memory Usage: {primaryNode.mempoolInfo.bytes} / {primaryNode.mempoolInfo.max_bytes} bytes · Window: {primaryNode.mempoolInfo.recent_window_blocks} blocks
                </div>
              </div>
            )}

            {mempool.length === 0 ? (
              <p className="text-xs font-mono text-neutral-500 py-3">
                Mempool is currently clear — incoming transactions from CLI `sdr-wallet` or Web Wallet are mined within ~1.5–3s.
              </p>
            ) : (
              <div className="space-y-1.5 max-h-56 overflow-y-auto">
                {mempool.map((tx) => (
                  <button
                    key={tx.txid}
                    onClick={() => setInspector({ kind: 'tx', data: tx })}
                    className="w-full text-left p-2.5 bg-neutral-950 rounded border border-neutral-800 hover:border-neutral-700 text-xs font-mono flex items-center justify-between"
                  >
                    <span className="text-amber-400 truncate max-w-[160px]">0x{tx.txid.slice(0, 16)}...</span>
                    <span className="text-emerald-400 tabular-nums">{tx.total_output_sdr.toFixed(4)} SDR</span>
                    <span className="text-neutral-500 text-[10px]">Nonce {tx.micro_pow_nonce}</span>
                  </button>
                ))}
              </div>
            )}
          </div>

          <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-5 space-y-2">
            <div className="flex items-center gap-2 text-xs font-mono font-bold text-amber-400">
              <Lock className="w-3.5 h-3.5" />
              <span>MAINNET RELEASE GATE ENFORCEMENT</span>
            </div>
            <p className="text-xs text-neutral-400 leading-relaxed">
              <code className="text-neutral-200 font-mono">MAINNET_ENABLED = false</code> in <code className="text-neutral-200 font-mono">sdr-primitives</code>. Public Mainnet genesis launch remains blocked until all Section-12 security release gates and hardware GPU resistance benchmarks pass.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};
