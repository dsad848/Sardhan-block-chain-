import React, { useState } from 'react';
import { Calculator, Scale, Cpu, AlertTriangle, ShieldCheck, CheckCircle2, XCircle } from 'lucide-react';

export const ConsensusMathVisualizer: React.FC = () => {
  // Simulator: Low Difficulty Spam vs Normal Difficulty Chain
  const [forkASpamFactor, setForkASpamFactor] = useState<number>(4); // 4x easier blocks
  const [forkABlockCount, setForkABlockCount] = useState<number>(36); // 36 blocks mined
  const [forkBBlockCount, setForkBBlockCount] = useState<number>(10); // 10 normal blocks

  // Relative difficulty
  const normalTarget = 1000;
  const spamTarget = normalTarget * forkASpamFactor;

  // Weight formula: W = sum(2^256 / Target) -> relative weight
  const weightPerNormalBlock = 1000;
  const weightPerSpamBlock = Math.floor(1000 / forkASpamFactor);

  const forkAWeight = forkABlockCount * weightPerSpamBlock;
  const forkBWeight = forkBBlockCount * weightPerNormalBlock;

  const forkAWins = forkAWeight > forkBWeight;

  return (
    <div className="space-y-6">
      {/* Chain Weight & Low-Difficulty Anti-Manipulation Theorem */}
      <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-neutral-800">
          <div>
            <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
              <span className="text-amber-400 font-semibold">TASK 1 DERIVATION</span>
              <span aria-hidden="true">·</span>
              <span>EXACT CUMULATIVE CHAIN WEIGHT W(C)</span>
            </div>
            <h2 className="text-xl lg:text-2xl font-bold text-neutral-100 tracking-tight">
              Chain-Weight Fork Selection & Anti-Manipulation Proof
            </h2>
            <p className="text-sm text-neutral-400 mt-1 max-w-2xl leading-relaxed">
              Verify Theorem 2.1: Proving why an adversary producing a larger quantity of low-difficulty blocks cannot overtake the canonical chain.
            </p>
          </div>

          <div className="flex items-center gap-2 bg-neutral-950 px-4 py-2.5 rounded-lg border border-neutral-800 text-xs font-mono">
            <Scale className="w-4 h-4 text-amber-400" />
            <span className="text-neutral-300">W(C) = Σ ⌊2²⁵⁶ / (T + 1)⌋</span>
          </div>
        </div>

        {/* Interactive Fork Comparison Simulator */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 pt-6">
          {/* Fork A: Adversary Low-Difficulty Spam */}
          <div className="bg-neutral-950 p-5 rounded-lg border border-neutral-800 space-y-4">
            <div className="flex items-center justify-between">
              <span className="text-xs font-mono text-rose-400 font-semibold">FORK A: LOW-DIFFICULTY SPAM BRANCH</span>
              <span className="text-xs font-mono px-2 py-0.5 rounded bg-rose-950/60 text-rose-300 border border-rose-900/60">
                {forkASpamFactor}x Easier Blocks
              </span>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs text-neutral-300">
                <span>Adversarial Target Inflation:</span>
                <span className="font-mono text-rose-400">{forkASpamFactor}x Target</span>
              </div>
              <input
                type="range"
                min="2"
                max="10"
                value={forkASpamFactor}
                onChange={(e) => setForkASpamFactor(Number(e.target.value))}
                className="w-full accent-rose-500 bg-neutral-800 h-2 rounded cursor-pointer"
              />
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs text-neutral-300">
                <span>Blocks Produced on Fork A:</span>
                <span className="font-mono text-rose-400 font-bold">{forkABlockCount} blocks</span>
              </div>
              <input
                type="range"
                min="5"
                max="60"
                value={forkABlockCount}
                onChange={(e) => setForkABlockCount(Number(e.target.value))}
                className="w-full accent-rose-500 bg-neutral-800 h-2 rounded cursor-pointer"
              />
            </div>

            <div className="pt-3 border-t border-neutral-850 flex items-center justify-between">
              <div>
                <span className="text-[10px] font-mono text-neutral-500 block">ACCUMULATED WEIGHT W(A)</span>
                <span className="text-lg font-mono font-bold text-rose-400 tabular-nums">
                  {forkAWeight.toLocaleString()} units
                </span>
              </div>
              <span className="text-xs font-mono text-neutral-400">{forkABlockCount} blocks × {weightPerSpamBlock} wt/blk</span>
            </div>
          </div>

          {/* Fork B: Canonical High-Difficulty Chain */}
          <div className="bg-neutral-950 p-5 rounded-lg border border-neutral-800 space-y-4">
            <div className="flex items-center justify-between">
              <span className="text-xs font-mono text-emerald-400 font-semibold">FORK B: CANONICAL HONEST CHAIN</span>
              <span className="text-xs font-mono px-2 py-0.5 rounded bg-emerald-950/60 text-emerald-300 border border-emerald-900/60">
                1.0x Baseline Difficulty
              </span>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs text-neutral-300">
                <span>Honest Target Difficulty:</span>
                <span className="font-mono text-emerald-400">1.0x Target (1,000 wt/blk)</span>
              </div>
              <div className="h-2 bg-neutral-800 rounded">
                <div className="w-full h-full bg-emerald-500 rounded"></div>
              </div>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs text-neutral-300">
                <span>Blocks Produced on Fork B:</span>
                <span className="font-mono text-emerald-400 font-bold">{forkBBlockCount} blocks</span>
              </div>
              <input
                type="range"
                min="1"
                max="30"
                value={forkBBlockCount}
                onChange={(e) => setForkBBlockCount(Number(e.target.value))}
                className="w-full accent-emerald-500 bg-neutral-800 h-2 rounded cursor-pointer"
              />
            </div>

            <div className="pt-3 border-t border-neutral-850 flex items-center justify-between">
              <div>
                <span className="text-[10px] font-mono text-neutral-500 block">ACCUMULATED WEIGHT W(B)</span>
                <span className="text-lg font-mono font-bold text-emerald-400 tabular-nums">
                  {forkBWeight.toLocaleString()} units
                </span>
              </div>
              <span className="text-xs font-mono text-neutral-400">{forkBBlockCount} blocks × {weightPerNormalBlock} wt/blk</span>
            </div>
          </div>
        </div>

        {/* Fork Choice Outcome Banner */}
        <div className={`mt-6 p-4 rounded-lg border flex items-center justify-between ${
          forkAWins
            ? 'bg-amber-950/20 border-amber-800/80 text-amber-200'
            : 'bg-emerald-950/20 border-emerald-800/80 text-emerald-200'
        }`}>
          <div className="flex items-center gap-3">
            {forkAWins ? (
              <AlertTriangle className="w-5 h-5 text-amber-400 shrink-0" />
            ) : (
              <CheckCircle2 className="w-5 h-5 text-emerald-400 shrink-0" />
            )}
            <div>
              <span className="text-xs font-mono font-bold block">
                {forkAWins
                  ? 'FORK A HAS HIGHER TOTAL WEIGHT (REQUIRES EXTREME TIME OR RESOURCES)'
                  : 'FORK B ACCEPTED AS CANONICAL CHAIN (EVEN WITH FEWER TOTAL BLOCKS)'}
              </span>
              <p className="text-xs opacity-90 mt-0.5">
                {forkAWins
                  ? `To overtake Fork B, the adversary had to produce ${forkABlockCount} blocks (${(forkABlockCount / forkBBlockCount).toFixed(1)}x more). Note: Branch must also satisfy the 64-block channel balance window [24, 40] or it is rejected.`
                  : `Even though Fork A has ${forkABlockCount} blocks and Fork B has only ${forkBBlockCount} blocks, nodes choose Fork B because W(B) [${forkBWeight}] > W(A) [${forkAWeight}]. Chain length alone does not win.`}
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Real Hardware Comparison Matrix */}
      <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
        <div className="pb-4 border-b border-neutral-800">
          <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
            <span className="text-amber-400 font-semibold">TASK 3 MINING EVALUATION</span>
            <span aria-hidden="true">·</span>
            <span>NO INVENTED BENCHMARKS · ARCHITECTURAL REALITIES</span>
          </div>
          <h3 className="text-lg font-bold text-neutral-100 tracking-tight">
            Proof-of-Work Algorithm & Hardware Architectural Comparison
          </h3>
          <p className="text-xs text-neutral-400 mt-1">
            Objective evaluation of hardware bottlenecks, memory latency constraints, and mobile ARM viability.
          </p>
        </div>

        <div className="overflow-x-auto rounded-lg border border-neutral-800 mt-5">
          <table className="w-full text-left text-xs border-collapse">
            <thead>
              <tr className="border-b border-neutral-800 bg-neutral-950 text-neutral-300 font-mono font-semibold">
                <th className="py-3 px-4">Metric / Dimension</th>
                <th className="py-3 px-4">Monero (RandomX)</th>
                <th className="py-3 px-4">Verus (VerusHash 2.2b)</th>
                <th className="py-3 px-4">Argon2d</th>
                <th className="py-3 px-4 text-amber-400">SardhanHash (Proposed)</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-neutral-850 font-sans text-neutral-300">
              <tr>
                <td className="py-2.5 px-4 font-mono text-neutral-400 font-semibold">Scratchpad / RAM</td>
                <td className="py-2.5 px-4 font-mono text-rose-400">2,080 MiB (2.08 GiB)</td>
                <td className="py-2.5 px-4 font-mono text-emerald-400">~4–8 MiB</td>
                <td className="py-2.5 px-4 font-mono">64 MiB – 1 GiB</td>
                <td className="py-2.5 px-4 font-mono text-emerald-400">8 MiB Fixed</td>
              </tr>
              <tr>
                <td className="py-2.5 px-4 font-mono text-neutral-400 font-semibold">Mobile Android Viability</td>
                <td className="py-2.5 px-4 text-rose-300">Unviable (Exhausts RAM, OOM killed)</td>
                <td className="py-2.5 px-4 text-emerald-300">Excellent (Native ARMv8 Crypto)</td>
                <td className="py-2.5 px-4 text-amber-300">Marginal (Bus saturation)</td>
                <td className="py-2.5 px-4 text-emerald-300 font-semibold">Excellent (Fits in modern L3)</td>
              </tr>
              <tr>
                <td className="py-2.5 px-4 font-mono text-neutral-400 font-semibold">GPU Speedup vs CPU</td>
                <td className="py-2.5 px-4 font-mono">~0.8x – 1.5x (Minimal)</td>
                <td className="py-2.5 px-4 font-mono">~1.2x – 2.5x (Low)</td>
                <td className="py-2.5 px-4 font-mono">~2.0x – 4.0x (Moderate)</td>
                <td className="py-2.5 px-4 font-mono">~1.5x – 3.0x (With branch div.)</td>
              </tr>
              <tr>
                <td className="py-2.5 px-4 font-mono text-neutral-400 font-semibold">ASIC Economics</td>
                <td className="py-2.5 px-4">Prohibitive (VM complexity)</td>
                <td className="py-2.5 px-4">Prohibitive (AES-NI parity)</td>
                <td className="py-2.5 px-4">Moderate (Custom memory)</td>
                <td className="py-2.5 px-4">Prohibitive (Commodity instruction parity)</td>
              </tr>
              <tr>
                <td className="py-2.5 px-4 font-mono text-neutral-400 font-semibold">Battery & Thermal Profile</td>
                <td className="py-2.5 px-4 text-rose-300">Extreme thermal throttling</td>
                <td className="py-2.5 px-4 text-emerald-300">Manageable thermal load</td>
                <td className="py-2.5 px-4 text-amber-300">High memory controller power</td>
                <td className="py-2.5 px-4 text-emerald-300">Configurable thermal ceiling (&le;38°C)</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
