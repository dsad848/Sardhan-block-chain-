import React, { useState, useMemo } from 'react';
import { EpochData } from '../types/protocol';
import { Calculator, ArrowRight, ShieldCheck, Flame, Coins, Cpu } from 'lucide-react';

const TOTAL_CAP = 21000000;
const BLOCKS_PER_HALVING = 210000;
const TARGET_BLOCK_TIME_SEC = 150; // 2.5 min

export const EmissionCalculator: React.FC = () => {
  const [targetHeight, setTargetHeight] = useState<number>(105000);

  // Precompute epoch breakdown
  const epochs = useMemo<EpochData[]>(() => {
    const list: EpochData[] = [];
    let cumulative = 0;

    for (let e = 0; e <= 10; e++) {
      const reward = 50 / Math.pow(2, e);
      const epochTotal = reward * BLOCKS_PER_HALVING;
      cumulative += epochTotal;
      const pct = (cumulative / TOTAL_CAP) * 100;

      list.push({
        epoch: e,
        blockStart: e * BLOCKS_PER_HALVING,
        blockEnd: (e + 1) * BLOCKS_PER_HALVING - 1,
        sdrPerBlock: reward,
        epochTotalSdr: epochTotal,
        cumulativeSdr: cumulative,
        percentEmitted: Math.min(100, pct),
        powShareSdr: epochTotal * 0.5,
        posShareSdr: epochTotal * 0.5,
      });
    }
    return list;
  }, []);

  // Compute exact metrics for selected block height
  const currentMetrics = useMemo(() => {
    const height = Math.max(0, Math.min(6930000, targetHeight));
    const currentEpoch = Math.floor(height / BLOCKS_PER_HALVING);
    
    // Exact integer math in satoshis / sardhans
    let rewardSardhans = 0n;
    if (currentEpoch < 33) {
      const baseSardhans = 5000000000n; // 50 * 10^8
      rewardSardhans = baseSardhans >> BigInt(currentEpoch);
    }
    const rewardSdr = Number(rewardSardhans) / 100000000;

    // Calculate cumulative emission up to this block height
    let totalSardhans = 0n;
    for (let e = 0; e < currentEpoch; e++) {
      const epReward = 5000000000n >> BigInt(e);
      totalSardhans += epReward * BigInt(BLOCKS_PER_HALVING);
    }
    const blocksIntoEpoch = height - (currentEpoch * BLOCKS_PER_HALVING);
    totalSardhans += rewardSardhans * BigInt(blocksIntoEpoch);

    const cumulativeSdr = Number(totalSardhans) / 100000000;
    const remainingSdr = TOTAL_CAP - cumulativeSdr;
    const percentEmitted = (cumulativeSdr / TOTAL_CAP) * 100;
    
    // Time calculations
    const totalSeconds = height * TARGET_BLOCK_TIME_SEC;
    const daysElapsed = totalSeconds / 86400;
    const yearsElapsed = daysElapsed / 365.25;

    return {
      height,
      currentEpoch,
      rewardSdr,
      rewardSardhans: rewardSardhans.toString(),
      cumulativeSdr,
      remainingSdr,
      percentEmitted,
      daysElapsed: Math.round(daysElapsed),
      yearsElapsed: yearsElapsed.toFixed(2),
      powCumulative: cumulativeSdr * 0.5,
      posCumulative: cumulativeSdr * 0.5,
      nextHalvingBlock: (currentEpoch + 1) * BLOCKS_PER_HALVING,
      blocksUntilHalving: (currentEpoch + 1) * BLOCKS_PER_HALVING - height
    };
  }, [targetHeight]);

  return (
    <div className="space-y-6">
      {/* Header card */}
      <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-neutral-800">
          <div>
            <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
              <span className="text-amber-400 font-semibold">MATHEMATICAL VERIFICATION</span>
              <span aria-hidden="true">·</span>
              <span>INVARIANT: 21,000,000 SDR CAP</span>
              <span aria-hidden="true">·</span>
              <span>ZERO PREMINE</span>
            </div>
            <h2 className="text-xl lg:text-2xl font-bold text-neutral-100 tracking-tight">
              Deterministic Emission & Halving Simulator
            </h2>
            <p className="text-sm text-neutral-400 mt-1 max-w-2xl leading-relaxed">
              Verify the exact mathematical token emission curve, integer division truncation, and the 50/50 Proof of Power block reward split across any block height.
            </p>
          </div>

          <div className="flex items-center gap-4 bg-neutral-950 px-4 py-3 rounded-lg border border-neutral-800/80">
            <Coins className="w-5 h-5 text-amber-400 shrink-0" />
            <div>
              <div className="text-xs text-neutral-500 font-mono">MAX HARD SUPPLY</div>
              <div className="text-lg font-bold font-mono text-neutral-100 tabular-nums">21,000,000.00 SDR</div>
            </div>
          </div>
        </div>

        {/* Interactive Block Height Control */}
        <div className="pt-6">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 mb-3">
            <label className="text-sm font-semibold text-neutral-200 flex items-center gap-2">
              <Calculator className="w-4 h-4 text-amber-400" />
              <span>Simulate Block Height</span>
            </label>
            <div className="flex items-center gap-2">
              <span className="text-xs font-mono text-neutral-400">Jump to Epoch:</span>
              <button
                onClick={() => setTargetHeight(0)}
                className="px-2 py-0.5 text-xs font-mono bg-neutral-800 hover:bg-neutral-700 text-neutral-300 rounded"
              >
                Genesis
              </button>
              <button
                onClick={() => setTargetHeight(210000)}
                className="px-2 py-0.5 text-xs font-mono bg-neutral-800 hover:bg-neutral-700 text-neutral-300 rounded"
              >
                1st Halving
              </button>
              <button
                onClick={() => setTargetHeight(420000)}
                className="px-2 py-0.5 text-xs font-mono bg-neutral-800 hover:bg-neutral-700 text-neutral-300 rounded"
              >
                2nd Halving
              </button>
              <button
                onClick={() => setTargetHeight(1050000)}
                className="px-2 py-0.5 text-xs font-mono bg-neutral-800 hover:bg-neutral-700 text-neutral-300 rounded"
              >
                5th Halving
              </button>
            </div>
          </div>

          <div className="space-y-3">
            <input
              type="range"
              min="0"
              max="2100000"
              step="1000"
              value={targetHeight}
              onChange={(e) => setTargetHeight(Number(e.target.value))}
              className="w-full accent-amber-500 bg-neutral-800 h-2 rounded-lg cursor-pointer"
            />
            <div className="flex items-center justify-between">
              <span className="text-xs font-mono text-neutral-500">Block 0 (Genesis)</span>
              <div className="flex items-center gap-2">
                <span className="text-xs text-neutral-400 font-mono">Current Height:</span>
                <input
                  type="number"
                  min="0"
                  max="6930000"
                  value={targetHeight}
                  onChange={(e) => setTargetHeight(Math.max(0, Number(e.target.value)))}
                  className="w-32 bg-neutral-950 border border-neutral-800 rounded px-2 py-1 text-sm font-mono text-amber-400 text-right focus:outline-none focus:border-amber-500"
                />
              </div>
              <span className="text-xs font-mono text-neutral-500">Block 2,100,000 (Epoch 10)</span>
            </div>
          </div>
        </div>

        {/* Dynamic Computed Metrics Grid */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mt-8">
          <div className="p-4 rounded-lg bg-neutral-950/80 border border-neutral-800/80">
            <div className="text-xs font-mono text-neutral-400">BLOCK REWARD</div>
            <div className="text-xl font-bold font-mono text-amber-400 mt-1 tabular-nums">
              {currentMetrics.rewardSdr.toFixed(8)} <span className="text-xs text-neutral-500 font-sans">SDR</span>
            </div>
            <div className="text-xs text-neutral-500 font-mono mt-1">
              {currentMetrics.rewardSardhans} Sardhans
            </div>
          </div>

          <div className="p-4 rounded-lg bg-neutral-950/80 border border-neutral-800/80">
            <div className="text-xs font-mono text-neutral-400">CIRCULATING SUPPLY</div>
            <div className="text-xl font-bold font-mono text-neutral-100 mt-1 tabular-nums">
              {currentMetrics.cumulativeSdr.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </div>
            <div className="text-xs text-neutral-500 font-mono mt-1">
              {currentMetrics.percentEmitted.toFixed(2)}% of 21M Cap
            </div>
          </div>

          <div className="p-4 rounded-lg bg-neutral-950/80 border border-neutral-800/80">
            <div className="text-xs font-mono text-neutral-400">HALVING EPOCH</div>
            <div className="text-xl font-bold font-mono text-neutral-100 mt-1 tabular-nums">
              Epoch {currentMetrics.currentEpoch}
            </div>
            <div className="text-xs text-neutral-500 font-mono mt-1">
              Next in {currentMetrics.blocksUntilHalving.toLocaleString()} blocks
            </div>
          </div>

          <div className="p-4 rounded-lg bg-neutral-950/80 border border-neutral-800/80">
            <div className="text-xs font-mono text-neutral-400">ESTIMATED NETWORK AGE</div>
            <div className="text-xl font-bold font-mono text-neutral-100 mt-1 tabular-nums">
              ~{currentMetrics.yearsElapsed} <span className="text-xs text-neutral-500 font-sans">years</span>
            </div>
            <div className="text-xs text-neutral-500 font-mono mt-1">
              {currentMetrics.daysElapsed.toLocaleString()} days since genesis
            </div>
          </div>
        </div>

        {/* 50/50 Mining & Staking Split Visualization */}
        <div className="mt-8 p-5 rounded-lg bg-neutral-950 border border-neutral-800/80">
          <div className="text-xs font-mono text-neutral-400 mb-3 flex items-center justify-between">
            <span className="font-semibold text-neutral-300">UNIFIED PROOF OF POWER: 50% MINING / 50% STAKING SPLIT</span>
            <span>Cumulative Distribution to Height {currentMetrics.height.toLocaleString()}</span>
          </div>

          <div className="w-full bg-neutral-800 h-3 rounded-full overflow-hidden flex">
            <div className="bg-amber-500 h-full w-1/2" title="50% CPU/ARM Miners"></div>
            <div className="bg-emerald-500 h-full w-1/2" title="50% Community Stakers"></div>
          </div>

          <div className="grid grid-cols-2 gap-4 mt-3">
            <div className="flex items-center gap-2">
              <Cpu className="w-4 h-4 text-amber-400" />
              <div>
                <span className="text-xs text-neutral-400 block font-sans">PoW CPU/ARM Mining Allocation</span>
                <span className="text-sm font-mono font-bold text-neutral-200 tabular-nums">
                  {currentMetrics.powCumulative.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })} SDR (50%)
                </span>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <ShieldCheck className="w-4 h-4 text-emerald-400" />
              <div>
                <span className="text-xs text-neutral-400 block font-sans">PoS Community Staking Allocation</span>
                <span className="text-sm font-mono font-bold text-neutral-200 tabular-nums">
                  {currentMetrics.posCumulative.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })} SDR (50%)
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Epochs Schedule Table */}
      <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
        <h3 className="text-lg font-bold text-neutral-100 tracking-tight mb-4">
          Scheduled Halving Table (Epochs 0 through 10)
        </h3>
        <div className="overflow-x-auto rounded-lg border border-neutral-800">
          <table className="w-full text-left text-xs border-collapse">
            <thead>
              <tr className="border-b border-neutral-800 bg-neutral-950 text-neutral-400 font-semibold font-mono">
                <th className="py-3 px-4">Epoch</th>
                <th className="py-3 px-4">Block Range</th>
                <th className="py-3 px-4 text-right">Reward (SDR)</th>
                <th className="py-3 px-4 text-right">Epoch Total</th>
                <th className="py-3 px-4 text-right">Cumulative SDR</th>
                <th className="py-3 px-4 text-right">% Emitted</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-neutral-850">
              {epochs.map((ep) => {
                const isSelected = ep.epoch === currentMetrics.currentEpoch;
                return (
                  <tr
                    key={ep.epoch}
                    onClick={() => setTargetHeight(ep.blockStart + 1000)}
                    className={`cursor-pointer transition-colors ${
                      isSelected
                        ? 'bg-amber-500/10 text-amber-200 font-semibold'
                        : 'hover:bg-neutral-800/40 text-neutral-300'
                    }`}
                  >
                    <td className="py-2.5 px-4 font-mono">
                      {isSelected ? '▶ Epoch ' + ep.epoch : 'Epoch ' + ep.epoch}
                    </td>
                    <td className="py-2.5 px-4 font-mono text-neutral-400">
                      {ep.blockStart.toLocaleString()} – {ep.blockEnd.toLocaleString()}
                    </td>
                    <td className="py-2.5 px-4 font-mono text-right tabular-nums text-amber-400">
                      {ep.sdrPerBlock.toFixed(4)}
                    </td>
                    <td className="py-2.5 px-4 font-mono text-right tabular-nums">
                      {ep.epochTotalSdr.toLocaleString()} SDR
                    </td>
                    <td className="py-2.5 px-4 font-mono text-right tabular-nums">
                      {ep.cumulativeSdr.toLocaleString()} SDR
                    </td>
                    <td className="py-2.5 px-4 font-mono text-right tabular-nums">
                      {ep.percentEmitted.toFixed(2)}%
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
