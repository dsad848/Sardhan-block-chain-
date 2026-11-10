import React, { useState } from 'react';
import { THREAT_VECTORS } from '../data/threatMatrix';
import { ThreatVector } from '../types/protocol';
import { Shield, AlertTriangle, CheckCircle2, ChevronRight, XCircle } from 'lucide-react';

export const ThreatModelVisualizer: React.FC = () => {
  const [selectedThreat, setSelectedThreat] = useState<ThreatVector>(THREAT_VECTORS[0]);
  const [attackerHashrate, setAttackerHashrate] = useState<number>(65); // 65% hashrate
  const [attackerStake, setAttackerStake] = useState<number>(10); // 10% stake

  // Calculate 51% attack feasibility under different consensus mechanisms
  const purePowVulnerable = attackerHashrate > 50;
  const purePosVulnerable = attackerStake > 50;
  
  // Under Sardhan Proof of Power, attack power is (alpha + beta) / 2
  // Reorg requires attacker speed > honest speed: alpha + beta > 1.0
  const sardhanSum = (attackerHashrate + attackerStake) / 100;
  const sardhanVulnerable = sardhanSum > 1.0;

  return (
    <div className="space-y-6">
      {/* 51% Attack Resilience Interactive Simulator */}
      <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-neutral-800">
          <div>
            <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
              <span className="text-amber-400 font-semibold">SECURITY THEOREM</span>
              <span aria-hidden="true">·</span>
              <span>PROOF OF POWER DUAL-THRESHOLD INVARIANT</span>
            </div>
            <h2 className="text-xl lg:text-2xl font-bold text-neutral-100 tracking-tight">
              51% Attack Resilience & Cost Simulator
            </h2>
            <p className="text-sm text-neutral-400 mt-1 max-w-2xl leading-relaxed">
              Model adversarial scenarios by varying attacker CPU hashrate vs active coin stake. Demonstrates why renting 51% hashrate fails against Sardhan Chain without simultaneously capturing 51% of circulating coin stake.
            </p>
          </div>

          <div className="flex items-center gap-2 bg-neutral-950 px-4 py-2.5 rounded-lg border border-neutral-800 text-xs font-mono">
            <span className="text-neutral-400">Security Rule:</span>
            <span className="text-amber-400 font-bold">α (Hash) + β (Stake) &gt; 1.0</span>
          </div>
        </div>

        {/* Sliders Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 pt-6">
          <div className="bg-neutral-950 p-5 rounded-lg border border-neutral-800/80 space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-xs font-mono text-neutral-300 font-semibold">
                ATTACKER CPU HASHRATE (α)
              </label>
              <span className="text-sm font-mono font-bold text-amber-400 tabular-nums">
                {attackerHashrate}%
              </span>
            </div>
            <input
              type="range"
              min="0"
              max="100"
              value={attackerHashrate}
              onChange={(e) => setAttackerHashrate(Number(e.target.value))}
              className="w-full accent-amber-500 bg-neutral-800 h-2 rounded-lg cursor-pointer"
            />
            <div className="flex items-center justify-between text-xs text-neutral-500 font-mono">
              <span>0% (Honest Network)</span>
              <span>50% (Standard PoW threshold)</span>
              <span>100% (Monopoly)</span>
            </div>
            <p className="text-xs text-neutral-400 pt-1">
              {attackerHashrate > 50
                ? '⚠️ Attacker controls majority hashrate. Standalone PoW chains (Bitcoin/Monero) would be fully reorganized.'
                : '✓ Honest miners control majority CPU hashrate.'}
            </p>
          </div>

          <div className="bg-neutral-950 p-5 rounded-lg border border-neutral-800/80 space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-xs font-mono text-neutral-300 font-semibold">
                ATTACKER MATURE COIN STAKE (β)
              </label>
              <span className="text-sm font-mono font-bold text-emerald-400 tabular-nums">
                {attackerStake}%
              </span>
            </div>
            <input
              type="range"
              min="0"
              max="100"
              value={attackerStake}
              onChange={(e) => setAttackerStake(Number(e.target.value))}
              className="w-full accent-emerald-500 bg-neutral-800 h-2 rounded-lg cursor-pointer"
            />
            <div className="flex items-center justify-between text-xs text-neutral-500 font-mono">
              <span>0% (No SDR held)</span>
              <span>50% (Standard PoS threshold)</span>
              <span>100% (Total Cartel)</span>
            </div>
            <p className="text-xs text-neutral-400 pt-1">
              {attackerStake > 50
                ? '⚠️ Attacker controls majority stake. Standalone PoS chains would be permanently captured.'
                : '✓ Honest community validators control majority active stake.'}
            </p>
          </div>
        </div>

        {/* Comparative Attack Outcome Matrix */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mt-6">
          {/* Bitcoin / Pure PoW */}
          <div className={`p-4 rounded-lg border ${
            purePowVulnerable 
              ? 'bg-rose-950/20 border-rose-900/60 text-rose-200' 
              : 'bg-neutral-950 border-neutral-800 text-neutral-300'
          }`}>
            <div className="text-xs font-mono text-neutral-400">PURE POW (BITCOIN / NAKAMOTO)</div>
            <div className="flex items-center gap-2 mt-2">
              {purePowVulnerable ? (
                <XCircle className="w-5 h-5 text-rose-500 shrink-0" />
              ) : (
                <CheckCircle2 className="w-5 h-5 text-emerald-400 shrink-0" />
              )}
              <span className="text-sm font-bold">
                {purePowVulnerable ? 'NETWORK COMPROMISED' : 'REORG RESISTANT'}
              </span>
            </div>
            <p className="text-xs mt-2 opacity-80 leading-relaxed font-sans">
              {purePowVulnerable 
                ? `Attacker has ${attackerHashrate}% hashrate (>50%). Can rewrite chain history and execute double spends.`
                : 'Attacker hashrate is below 50% threshold.'}
            </p>
          </div>

          {/* Pure PoS */}
          <div className={`p-4 rounded-lg border ${
            purePosVulnerable 
              ? 'bg-rose-950/20 border-rose-900/60 text-rose-200' 
              : 'bg-neutral-950 border-neutral-800 text-neutral-300'
          }`}>
            <div className="text-xs font-mono text-neutral-400">PURE POS (PROOF OF STAKE)</div>
            <div className="flex items-center gap-2 mt-2">
              {purePosVulnerable ? (
                <XCircle className="w-5 h-5 text-rose-500 shrink-0" />
              ) : (
                <CheckCircle2 className="w-5 h-5 text-emerald-400 shrink-0" />
              )}
              <span className="text-sm font-bold">
                {purePosVulnerable ? 'NETWORK COMPROMISED' : 'CARTEL RESISTANT'}
              </span>
            </div>
            <p className="text-xs mt-2 opacity-80 leading-relaxed font-sans">
              {purePosVulnerable 
                ? `Attacker holds ${attackerStake}% of coins (>50%). Can permanently control block production and censor peers.`
                : 'Attacker coin stake is below 50% threshold.'}
            </p>
          </div>

          {/* Sardhan Hybrid PoP */}
          <div className={`p-4 rounded-lg border ${
            sardhanVulnerable 
              ? 'bg-amber-950/20 border-amber-800 text-amber-200' 
              : 'bg-emerald-950/20 border-emerald-800/80 text-emerald-200'
          }`}>
            <div className="text-xs font-mono text-neutral-400">SARDHAN CHAIN (PROOF OF POWER)</div>
            <div className="flex items-center gap-2 mt-2">
              {sardhanVulnerable ? (
                <AlertTriangle className="w-5 h-5 text-amber-400 shrink-0" />
              ) : (
                <CheckCircle2 className="w-5 h-5 text-emerald-400 shrink-0" />
              )}
              <span className="text-sm font-bold">
                {sardhanVulnerable ? 'THEORETICAL THRESHOLD' : 'IMMUNE TO 51% REORGANIZATION'}
              </span>
            </div>
            <p className="text-xs mt-2 opacity-80 leading-relaxed font-sans">
              {sardhanVulnerable
                ? `Combined attack weight α (${attackerHashrate}%) + β (${attackerStake}%) = ${(sardhanSum * 100).toFixed(0)}% (>100%).`
                : `Combined α + β = ${(sardhanSum * 100).toFixed(0)}% (<= 100%). Attacker stalls on alternate consensus channels and cannot reorg.`}
            </p>
          </div>
        </div>
      </div>

      {/* Threat Vector Inspection Matrix */}
      <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
        <h3 className="text-lg font-bold text-neutral-100 tracking-tight mb-2">
          Protocol Threat Model & Attack Matrix
        </h3>
        <p className="text-xs text-neutral-400 mb-6">
          Select an attack vector to inspect the systemic impact on standalone architectures vs Sardhan Chain’s cryptographic defenses.
        </p>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          {/* List of threats */}
          <div className="lg:col-span-5 space-y-2">
            {THREAT_VECTORS.map((tv) => {
              const isSelected = tv.id === selectedThreat.id;
              return (
                <button
                  key={tv.id}
                  onClick={() => setSelectedThreat(tv)}
                  className={`w-full text-left p-3.5 rounded-lg border transition-all flex items-center justify-between ${
                    isSelected
                      ? 'bg-amber-500/10 border-amber-500/60 text-amber-200'
                      : 'bg-neutral-950/60 border-neutral-800 text-neutral-300 hover:bg-neutral-800/40'
                  }`}
                >
                  <div>
                    <div className="flex items-center gap-2">
                      <span className={`text-[10px] font-mono px-1.5 py-0.5 rounded font-semibold ${
                        tv.severity === 'Critical'
                          ? 'bg-rose-950 text-rose-300 border border-rose-800/60'
                          : tv.severity === 'High'
                          ? 'bg-amber-950 text-amber-300 border border-amber-800/60'
                          : 'bg-neutral-800 text-neutral-300'
                      }`}>
                        {tv.severity}
                      </span>
                      <span className="text-xs font-mono text-neutral-400">{tv.subsystem}</span>
                    </div>
                    <div className="text-sm font-semibold mt-1 text-neutral-100">
                      {tv.name}
                    </div>
                  </div>
                  <ChevronRight className={`w-4 h-4 shrink-0 transition-transform ${isSelected ? 'translate-x-1 text-amber-400' : 'text-neutral-500'}`} />
                </button>
              );
            })}
          </div>

          {/* Detailed view of selected threat */}
          <div className="lg:col-span-7 bg-neutral-950 border border-neutral-800 rounded-lg p-6 space-y-5">
            <div>
              <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
                <span className="text-amber-400">{selectedThreat.subsystem}</span>
                <span aria-hidden="true">·</span>
                <span>SEVERITY: {selectedThreat.severity.toUpperCase()}</span>
              </div>
              <h4 className="text-lg font-bold text-neutral-100">{selectedThreat.name}</h4>
              <p className="text-sm text-neutral-300 mt-2 leading-relaxed">
                {selectedThreat.description}
              </p>
            </div>

            <div className="space-y-3 pt-3 border-t border-neutral-850">
              <div className="p-3 bg-neutral-900/60 rounded border border-neutral-800">
                <span className="text-xs font-mono text-rose-400 font-semibold block mb-1">
                  VULNERABILITY IN PURE POW / POS
                </span>
                <p className="text-xs text-neutral-300 leading-relaxed">
                  {selectedThreat.purePowOutcome}
                </p>
              </div>

              <div className="p-3 bg-emerald-950/20 rounded border border-emerald-900/50">
                <span className="text-xs font-mono text-emerald-400 font-semibold block mb-1">
                  SARDHAN CHAIN MITIGATION
                </span>
                <p className="text-xs text-neutral-200 leading-relaxed">
                  {selectedThreat.sardhanMitigation}
                </p>
              </div>

              <div className="p-3 bg-neutral-900/40 rounded border border-neutral-800 font-mono text-xs">
                <span className="text-neutral-400 block mb-1 font-sans font-semibold">
                  MATHEMATICAL INVARIANT / GUARANTEE
                </span>
                <span className="text-amber-300/90 leading-relaxed block">
                  {selectedThreat.mathematicalGuarantee}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
