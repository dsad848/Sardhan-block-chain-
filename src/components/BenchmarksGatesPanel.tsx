import React, { useState, useEffect } from 'react';
import { Cpu, ShieldCheck, AlertTriangle, RefreshCw, CheckCircle2, Lock } from 'lucide-react';

interface BenchmarkPayload {
  timestamp: string;
  output: string;
}

const RELEASE_GATES = [
  { gate: '1. Critical & High Vulnerabilities Resolved', status: 'Passed (In-Repo)', note: 'SEC-CRIT-01..03, SEC-HIGH-01..04, canonical Ed25519 S<L, OOM bounds fixed & tested' },
  { gate: '2. Consensus Rules Fully Specified', status: 'Passed', note: 'Unified U256 chain weight, Dual-DAA [0.25x..4.0x], H-100 lagging modifier, MTP-11' },
  { gate: '3. Independent External Security Review', status: 'Pending Sign-Off', note: 'Requires third-party external cryptographic & protocol audit before mainnet flag flip' },
  { gate: '4. Supply & Integer Emission Verified', status: 'Passed', note: 'Exact 20,999,999.9769 SDR cap via bit-shift halving; zero floats; 0% genesis premine' },
  { gate: '5. Three-Node Testnet Operational', status: 'Passed (Live)', note: '3 independent sdrd TCP peers mining PoW, staking PoS, & settling zero-fee tx' },
  { gate: '6. Node Restart & State Persistence', status: 'Passed', note: 'Checksummed append-only blocks.bin + atomic slashing.json + peers.json verified' },
  { gate: '7. End-to-End Wallet Transactions', status: 'Passed', note: 'sdr-wallet CLI + Web Wallet sign locally, solve micro-PoW & broadcast zero-fee tx' },
  { gate: '8. PoW Mining & PoS Staking Active', status: 'Passed', note: 'Multi-threaded SardhanHash CPU miner + thermal/battery guard + PoS kernel validator' },
  { gate: '9. Zero Simulated/Fake Functionality', status: 'Passed', note: 'Frontend Explorer, Wallet, and Benchmark panels backed by real sdrd & sdr-miner binaries' },
  { gate: '10. Reproducible Builds & GPU Benchmarks', status: 'Pending Hardware', note: 'Cargo.lock pinned; physical CUDA/OpenCL/Metal GPU resistance benchmarks still required' },
  { gate: '11. MAINNET_ENABLED Gate Locked', status: 'Enforced (false)', note: 'sdrd exits with code 2 if launched on --network mainnet without release gate unlock' },
];

export const BenchmarksGatesPanel: React.FC = () => {
  const [benchmark, setBenchmark] = useState<BenchmarkPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const runBench = async (refresh = false) => {
    setLoading(true);
    setError(null);
    try {
      const resp = await fetch(`/api/miner/benchmark${refresh ? '?refresh=1' : ''}`);
      const data = await resp.json();
      if (!resp.ok) throw new Error(data.error || 'Benchmark execution failed');
      setBenchmark(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed running sdr-miner --benchmark');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    runBench(false);
  }, []);

  return (
    <div className="space-y-6">
      {/* Live Host CPU Benchmark Panel */}
      <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-neutral-800 pb-4">
          <div className="space-y-1">
            <div className="flex items-center gap-2 text-xs font-mono text-neutral-400">
              <Cpu className="w-3.5 h-3.5 text-amber-400" />
              <span className="text-amber-400 font-semibold">EMPIRICAL HOST MEASUREMENTS</span>
              <span aria-hidden="true">·</span>
              <span>EXECUTED VIA `sdr-miner --benchmark`</span>
            </div>
            <h2 className="text-xl font-bold text-neutral-100">
              SardhanHash vs Reference CPU Workloads
            </h2>
          </div>

          <button
            onClick={() => runBench(true)}
            disabled={loading}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-lg bg-amber-400 text-neutral-950 text-xs font-bold hover:bg-amber-300 disabled:opacity-50 shrink-0"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
            <span>{loading ? 'Benchmarking Host...' : 'Re-Run Host Benchmark'}</span>
          </button>
        </div>

        <div className="rounded-lg border border-amber-900/60 bg-amber-950/20 p-4 flex items-start gap-3 text-xs text-amber-200">
          <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
          <div className="space-y-1 leading-relaxed">
            <div className="font-mono font-bold text-amber-300">
              GPU RESISTANCE INTEGRITY STATEMENT
            </div>
            <p className="text-neutral-300">
              Measurements below reflect actual CPU timings on this Linux host executing release-compiled <code className="font-mono text-amber-300">sdr-miner</code> alongside pure-Rust RandomX-style (2 MiB VM) and Verus-style (ARX lane mixing) simulations. Because SardhanHash currently spends ~11 ms filling 8 MiB via BLAKE3 XOF with 64 branching reads afterward, true GPU/ASIC resistance cannot be claimed until benchmarked on physical CUDA/OpenCL/Metal GPU hardware and tuned prior to Mainnet activation.
            </p>
          </div>
        </div>

        {error ? (
          <div className="rounded-lg border border-rose-800/70 bg-rose-950/30 p-4 text-xs font-mono text-rose-300">
            {error}
          </div>
        ) : (
          <pre className="rounded-lg border border-neutral-800 bg-neutral-950 p-4 text-xs font-mono text-neutral-200 overflow-x-auto leading-relaxed">
            {benchmark?.output || 'Running release-mode `sdr-miner --benchmark 1 --threads 2` on host...'}
          </pre>
        )}
      </div>

      {/* Section 12 Mainnet Release Gates */}
      <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-neutral-800 pb-4">
          <div className="flex items-center gap-2">
            <ShieldCheck className="w-4 h-4 text-emerald-400" />
            <h3 className="text-base font-bold text-neutral-100">
              Section 12 — Mainnet Security Release Gates
            </h3>
          </div>
          <div className="flex items-center gap-1.5 text-xs font-mono text-amber-400 font-bold">
            <Lock className="w-3.5 h-3.5" />
            <span>MAINNET_ENABLED = false (Release Candidate Blocked)</span>
          </div>
        </div>

        <div className="divide-y divide-neutral-800/70">
          {RELEASE_GATES.map((item) => {
            const isPending = item.status.startsWith('Pending');
            return (
              <div
                key={item.gate}
                className="py-3 flex flex-col sm:flex-row sm:items-center justify-between gap-2 text-xs"
              >
                <div className="space-y-0.5">
                  <div className="font-semibold text-neutral-200 flex items-center gap-2">
                    {isPending ? (
                      <AlertTriangle className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                    ) : (
                      <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                    )}
                    <span>{item.gate}</span>
                  </div>
                  <div className="text-neutral-400 font-mono text-[11px] pl-5">{item.note}</div>
                </div>
                <span
                  className={`font-mono font-semibold shrink-0 pl-5 sm:pl-0 ${
                    isPending ? 'text-amber-400' : 'text-emerald-400'
                  }`}
                >
                  {item.status}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
