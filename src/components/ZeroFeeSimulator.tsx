import React, { useState, useMemo } from 'react';
import { Cpu, ShieldCheck, Zap, AlertOctagon, Activity } from 'lucide-react';

export const ZeroFeeSimulator: React.FC = () => {
  const [mempoolSizeMb, setMempoolSizeMb] = useState<number>(18); // 18MB current occupancy
  const [spamTxCount, setSpamTxCount] = useState<number>(50000); // 50,000 tx attack

  const maxMempoolMb = 64;

  const simulation = useMemo(() => {
    const occupancyRatio = mempoolSizeMb / maxMempoolMb;
    
    // Dynamic difficulty multiplier: base is 1.0; if >50% full, scales quadratically
    let difficultyMultiplier = 1.0;
    if (occupancyRatio >= 0.5) {
      const remainingRatio = (maxMempoolMb - mempoolSizeMb) / (maxMempoolMb * 0.5);
      difficultyMultiplier = remainingRatio > 0.01 ? 1 / Math.pow(remainingRatio, 2) : 10000;
    }

    // Base difficulty takes ~0.10s (100ms) on a standard 4-core CPU
    const userComputeTimeSec = 0.10 * difficultyMultiplier;
    
    // Attacker total CPU computation time in seconds
    const attackerTotalCpuSec = spamTxCount * 0.10 * difficultyMultiplier;
    const attackerHours = attackerTotalCpuSec / 3600;
    const attacker8CoreDays = attackerHours / (24 * 8); // On an 8-core machine

    // Node verification time: 1 BLAKE3 hash check takes ~0.5 microseconds (0.0000005s)
    const nodeVerificationTotalSec = (spamTxCount * 0.0000005);

    // Asymmetry ratio
    const asymmetryFactor = attackerTotalCpuSec / Math.max(0.001, nodeVerificationTotalSec);

    return {
      occupancyRatio: (occupancyRatio * 100).toFixed(1),
      difficultyMultiplier: difficultyMultiplier.toFixed(2),
      userComputeTimeMs: Math.round(userComputeTimeSec * 1000),
      attackerTotalHours: attackerHours.toFixed(1),
      attacker8CoreDays: attacker8CoreDays.toFixed(2),
      nodeVerificationSec: nodeVerificationTotalSec.toFixed(3),
      asymmetryFactor: Math.round(asymmetryFactor).toLocaleString()
    };
  }, [mempoolSizeMb, spamTxCount]);

  return (
    <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8 space-y-6">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-neutral-800">
        <div>
          <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
            <span className="text-amber-400 font-semibold">ANTI-DOS ARCHITECTURE</span>
            <span aria-hidden="true">·</span>
            <span>ZERO-FEE DEFENSE-IN-DEPTH</span>
          </div>
          <h2 className="text-xl lg:text-2xl font-bold text-neutral-100 tracking-tight">
            Client Micro-PoW & Dynamic Congestion Scaler
          </h2>
          <p className="text-sm text-neutral-400 mt-1 max-w-2xl leading-relaxed">
            In Sardhan Chain, transaction fees are zero. Instead of paying SDR tokens, clients solve a lightweight BLAKE3 micro-PoW nonce. This calculator demonstrates how dynamic congestion scaling chokes off automated spam attacks while keeping regular transactions instant.
          </p>
        </div>

        <div className="flex items-center gap-3 bg-neutral-950 px-4 py-2.5 rounded-lg border border-neutral-800 text-xs font-mono">
          <Zap className="w-4 h-4 text-amber-400" />
          <div>
            <span className="text-neutral-500 block text-[10px]">VERIFICATION ASYMMETRY</span>
            <span className="text-neutral-200 font-bold">{simulation.asymmetryFactor} : 1</span>
          </div>
        </div>
      </div>

      {/* Control Sliders */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 pt-2">
        <div className="bg-neutral-950 p-5 rounded-lg border border-neutral-800/80 space-y-3">
          <div className="flex items-center justify-between">
            <label className="text-xs font-mono text-neutral-300 font-semibold">
              MEMPOOL MEMORY OCCUPANCY (OUT OF 64 MiB)
            </label>
            <span className="text-sm font-mono font-bold text-amber-400 tabular-nums">
              {mempoolSizeMb} MiB ({simulation.occupancyRatio}%)
            </span>
          </div>
          <input
            type="range"
            min="0"
            max="63"
            value={mempoolSizeMb}
            onChange={(e) => setMempoolSizeMb(Number(e.target.value))}
            className="w-full accent-amber-500 bg-neutral-800 h-2 rounded-lg cursor-pointer"
          />
          <div className="flex items-center justify-between text-xs text-neutral-500 font-mono">
            <span>0 MiB (Idle)</span>
            <span>32 MiB (50% Scaling Trigger)</span>
            <span>64 MiB (Max Cap)</span>
          </div>
          <p className="text-xs text-neutral-400">
            {mempoolSizeMb < 32 
              ? '✓ Normal load. Base difficulty target applies.'
              : `⚠️ Congestion threshold reached! Required micro-PoW difficulty is scaled ${simulation.difficultyMultiplier}x.`}
          </p>
        </div>

        <div className="bg-neutral-950 p-5 rounded-lg border border-neutral-800/80 space-y-3">
          <div className="flex items-center justify-between">
            <label className="text-xs font-mono text-neutral-300 font-semibold">
              SIMULATED SPAM BURST VOLUME
            </label>
            <span className="text-sm font-mono font-bold text-rose-400 tabular-nums">
              {spamTxCount.toLocaleString()} Transactions
            </span>
          </div>
          <input
            type="range"
            min="1000"
            max="250000"
            step="1000"
            value={spamTxCount}
            onChange={(e) => setSpamTxCount(Number(e.target.value))}
            className="w-full accent-rose-500 bg-neutral-800 h-2 rounded-lg cursor-pointer"
          />
          <div className="flex items-center justify-between text-xs text-neutral-500 font-mono">
            <span>1,000 Tx</span>
            <span>100,000 Tx (Flood)</span>
            <span>250,000 Tx (Extreme)</span>
          </div>
          <p className="text-xs text-neutral-400">
            Simulate the energy and computing hardware required for an attacker to construct this volume of zero-fee spam.
          </p>
        </div>
      </div>

      {/* Outcome Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="p-4 rounded-lg bg-neutral-950 border border-neutral-800">
          <div className="flex items-center gap-2 text-xs font-mono text-emerald-400 mb-1">
            <Cpu className="w-4 h-4" />
            <span>HONEST USER COST</span>
          </div>
          <div className="text-2xl font-bold font-mono text-neutral-100 tabular-nums">
            {simulation.userComputeTimeMs} <span className="text-xs font-sans text-neutral-400">ms CPU</span>
          </div>
          <p className="text-xs text-neutral-400 mt-2 leading-relaxed">
            Time required for an everyday wallet to solve the BLAKE3 micro-nonce. Completely imperceptible to a human user; $0 financial cost.
          </p>
        </div>

        <div className="p-4 rounded-lg bg-neutral-950 border border-neutral-800">
          <div className="flex items-center gap-2 text-xs font-mono text-rose-400 mb-1">
            <AlertOctagon className="w-4 h-4" />
            <span>SPAMMER COMPUTATION TIME</span>
          </div>
          <div className="text-2xl font-bold font-mono text-rose-300 tabular-nums">
            {simulation.attackerTotalHours} <span className="text-xs font-sans text-neutral-400">CPU-hours</span>
          </div>
          <p className="text-xs text-neutral-400 mt-2 leading-relaxed">
            Requires continuous compute on an 8-core CPU for <strong className="text-neutral-200">{simulation.attacker8CoreDays} days</strong> to construct {spamTxCount.toLocaleString()} transactions.
          </p>
        </div>

        <div className="p-4 rounded-lg bg-neutral-950 border border-neutral-800">
          <div className="flex items-center gap-2 text-xs font-mono text-amber-400 mb-1">
            <Activity className="w-4 h-4" />
            <span>NETWORK NODE VERIFICATION</span>
          </div>
          <div className="text-2xl font-bold font-mono text-amber-300 tabular-nums">
            {simulation.nodeVerificationSec} <span className="text-xs font-sans text-neutral-400">seconds</span>
          </div>
          <p className="text-xs text-neutral-400 mt-2 leading-relaxed">
            Full nodes verify all {spamTxCount.toLocaleString()} transactions in a fraction of a second due to the 200,000:1 hash verification asymmetry.
          </p>
        </div>
      </div>
    </div>
  );
};
