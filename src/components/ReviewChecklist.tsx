import React, { useState } from 'react';
import { ChecklistItem } from '../types/protocol';
import { CheckCircle2, Clock, HelpCircle, FileCheck, Download } from 'lucide-react';

const INITIAL_CHECKLIST: ChecklistItem[] = [
  {
    id: 'tok-1',
    category: 'Tokenomics',
    title: '21,000,000 SDR Supply Hard Cap',
    description: 'Strict mathematical invariant floor((50 * 10^8) / 2^epoch) ensuring total minted supply terminates at 20,999,999.9769 SDR.',
    status: 'Approved',
    notes: 'Verified via geometric sum in RFC-106.'
  },
  {
    id: 'tok-2',
    category: 'Tokenomics',
    title: 'Strict 0% Premine & 0% Founder Tax',
    description: 'Genesis block contains 0 SDR reward and no founder allocations. All coins originate strictly from CPU mining and staking.',
    status: 'Approved',
    notes: 'Genesis block parameters confirmed in RFC-102.'
  },
  {
    id: 'con-1',
    category: 'Consensus',
    title: 'Hybrid 50% PoW / 50% PoS (Proof of Power)',
    description: 'Miners and stakers mint alternating blocks. Reorganizing the chain requires both >50% hashrate and >50% active coin stake.',
    status: 'Approved',
    notes: 'Eliminates standalone PoW hash rental attacks.'
  },
  {
    id: 'con-2',
    category: 'Consensus',
    title: 'SardhanHash CPU & Android ARM Mining',
    description: 'Hardware-accelerated AES-NI & ARMv8 cryptographic instructions with ~8 MiB scratchpad to democratize mining on phones and desktop CPUs.',
    status: 'Approved',
    notes: 'Protects against ASIC centralization without exceeding mobile RAM constraints.'
  },
  {
    id: 'mem-1',
    category: 'Mempool',
    title: 'Zero Transaction Fees via Client Micro-PoW',
    description: 'Transactions are completely free of financial fees. Anti-spam is achieved via client-side BLAKE3 micro-PoW (~100ms CPU compute per tx).',
    status: 'Approved',
    notes: 'Verified 100,000:1 verification asymmetry in favor of full nodes.'
  },
  {
    id: 'mem-2',
    category: 'Mempool',
    title: 'Quadratic Dynamic Congestion Scaling',
    description: 'When mempool exceeds 50% capacity (32 MiB), required micro-PoW difficulty scales quadratically, making flood attacks mathematically impossible.',
    status: 'Approved',
    notes: 'Dynamic formula RFC-107.'
  },
  {
    id: 'sec-1',
    category: 'Security',
    title: 'Weak Subjectivity & 288-Block Reorg Horizon',
    description: 'Deep reorgs (>288 blocks / ~12 hours) are rejected unconditionally, eliminating historical long-range attacks.',
    status: 'Approved',
    notes: 'Standard security checkpoint policy.'
  },
  {
    id: 'sec-2',
    category: 'Security',
    title: 'Dual-Signing Slashing for Stakers',
    description: 'Validators who sign conflicting blocks at the same height forfeit their mature staking UTXO (50% burn, 50% reporter bounty).',
    status: 'Approved',
    notes: 'Solves Nothing-at-Stake problem.'
  },
  {
    id: 'arch-1',
    category: 'Architecture',
    title: 'Modular Monorepo Structure',
    description: 'Separation into packages: crypto, primitives, consensus, ledger, mempool, p2p, rpc, node, wallet, miner, explorer.',
    status: 'Approved',
    notes: 'Clear dependency boundaries prevent cyclic dependencies.'
  },
  {
    id: 'audit-ratify-1',
    category: 'Security',
    title: 'Ratify Block-Bound Micro-PoW (SEC-CRIT-01)',
    description: 'Binds transaction micro-PoW nonces to a recent block hash within 16 blocks to permanently eliminate offline botnet burst pre-mining.',
    status: 'Requires Clarification',
    notes: 'Audit recommendation pending stakeholder vote.'
  },
  {
    id: 'audit-ratify-2',
    category: 'Security',
    title: 'Ratify ChainID Replay Guard (SEC-CRIT-02)',
    description: 'Embeds 32-bit ChainID in wire serialization and Ed25519 signature preimage to prevent cross-network Testnet/Mainnet replay attacks.',
    status: 'Requires Clarification',
    notes: 'Audit recommendation pending stakeholder vote.'
  },
  {
    id: 'audit-ratify-3',
    category: 'Consensus',
    title: 'Ratify Lagging Stake Modifier & Weight Fork Choice (SEC-CRIT-03 & SEC-HIGH-01)',
    description: '100-block lagging stake modifier to eliminate stake grinding, paired with scalar cumulative Proof-of-Power weight for fork choice.',
    status: 'Requires Clarification',
    notes: 'Audit recommendation pending stakeholder vote.'
  },
  {
    id: 'audit-ratify-4',
    category: 'Architecture',
    title: 'Ratify Fixed 196-Byte Block Header (SEC-MED-01)',
    description: 'Standardizes exact 196-byte fixed-size binary header layout to eliminate deserialization buffer mismatches.',
    status: 'Requires Clarification',
    notes: 'Audit recommendation pending stakeholder vote.'
  }
];

export const ReviewChecklist: React.FC = () => {
  const [items, setItems] = useState<ChecklistItem[]>(INITIAL_CHECKLIST);

  const toggleStatus = (id: string) => {
    setItems((prev) =>
      prev.map((item) => {
        if (item.id === id) {
          const nextStatus =
            item.status === 'Approved'
              ? 'Requires Clarification'
              : item.status === 'Requires Clarification'
              ? 'Pending'
              : 'Approved';
          return { ...item, status: nextStatus };
        }
        return item;
      })
    );
  };

  const updateNotes = (id: string, notes: string) => {
    setItems((prev) =>
      prev.map((item) => (item.id === id ? { ...item, notes } : item))
    );
  };

  const handleExportReview = () => {
    const lines = [
      '# Sardhan Chain (SDR) — Phase 1 & 2 Protocol Review Sign-Off Record',
      `Date: ${new Date().toISOString()}`,
      `Total Invariants Evaluated: ${items.length}`,
      `Approved: ${items.filter(i => i.status === 'Approved').length}`,
      `Requires Clarification: ${items.filter(i => i.status === 'Requires Clarification').length}`,
      `Pending: ${items.filter(i => i.status === 'Pending').length}`,
      '',
      '---',
      ''
    ];

    items.forEach(item => {
      lines.push(`### [${item.status.toUpperCase()}] ${item.category}: ${item.title}`);
      lines.push(`Description: ${item.description}`);
      lines.push(`Reviewer Notes: ${item.notes || 'None'}`);
      lines.push('');
    });

    const blob = new Blob([lines.join('\n')], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sardhan_protocol_review_signoff_${Date.now()}.md`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const approvedCount = items.filter((i) => i.status === 'Approved').length;

  return (
    <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8 space-y-6">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-neutral-800">
        <div>
          <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
            <span className="text-amber-400 font-semibold">GOVERNANCE & AUDIT</span>
            <span aria-hidden="true">·</span>
            <span>PHASE 1 & 2 SIGN-OFF DASHBOARD</span>
          </div>
          <h2 className="text-xl lg:text-2xl font-bold text-neutral-100 tracking-tight">
            Protocol Engineering Review Checklist
          </h2>
          <p className="text-sm text-neutral-400 mt-1 max-w-2xl leading-relaxed">
            Per the project rule: <strong className="text-neutral-200">"Do not start coding until these are reviewed."</strong> Review each protocol component, adjust status, and annotate notes before authorizing Phase 4 implementation.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <div className="bg-neutral-950 px-3.5 py-2 rounded-lg border border-neutral-800 text-xs font-mono">
            <span className="text-neutral-400 block text-[10px]">APPROVAL PROGRESS</span>
            <span className="text-amber-400 font-bold">{approvedCount} / {items.length} Ready</span>
          </div>

          <button
            onClick={handleExportReview}
            className="flex items-center gap-2 px-3.5 py-2 text-xs font-semibold text-neutral-950 bg-amber-400 rounded-lg hover:bg-amber-300 transition-colors shrink-0 shadow-sm"
          >
            <Download className="w-3.5 h-3.5" />
            <span>Export Sign-Off</span>
          </button>
        </div>
      </div>

      {/* Checklist Items */}
      <div className="space-y-3 pt-2">
        {items.map((item) => (
          <div
            key={item.id}
            className="p-4 rounded-lg bg-neutral-950 border border-neutral-800/80 hover:border-neutral-700/60 transition-colors space-y-3"
          >
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
              <div className="flex items-start gap-3">
                <button
                  onClick={() => toggleStatus(item.id)}
                  className="mt-0.5 shrink-0"
                  title="Click to cycle status"
                >
                  {item.status === 'Approved' ? (
                    <CheckCircle2 className="w-5 h-5 text-emerald-400" />
                  ) : item.status === 'Requires Clarification' ? (
                    <HelpCircle className="w-5 h-5 text-amber-400" />
                  ) : (
                    <Clock className="w-5 h-5 text-neutral-500" />
                  )}
                </button>
                <div>
                  <div className="flex items-center gap-2 text-xs font-mono text-neutral-400">
                    <span className="text-amber-400 font-semibold">{item.category}</span>
                    <span aria-hidden="true">·</span>
                    <span className="text-neutral-500">ID: {item.id}</span>
                  </div>
                  <h4 className="text-sm font-bold text-neutral-100 mt-0.5">
                    {item.title}
                  </h4>
                  <p className="text-xs text-neutral-300 mt-1 leading-relaxed">
                    {item.description}
                  </p>
                </div>
              </div>

              <div className="shrink-0 flex items-center gap-2">
                <button
                  onClick={() => toggleStatus(item.id)}
                  className={`px-2.5 py-1 text-xs font-mono rounded font-semibold transition-colors ${
                    item.status === 'Approved'
                      ? 'bg-emerald-950/60 text-emerald-300 border border-emerald-800/60'
                      : item.status === 'Requires Clarification'
                      ? 'bg-amber-950/60 text-amber-300 border border-amber-800/60'
                      : 'bg-neutral-800 text-neutral-400 border border-neutral-700/60'
                  }`}
                >
                  {item.status}
                </button>
              </div>
            </div>

            <div className="pt-2 border-t border-neutral-850 flex items-center gap-2">
              <span className="text-[11px] font-mono text-neutral-500 shrink-0">Notes:</span>
              <input
                type="text"
                value={item.notes}
                onChange={(e) => updateNotes(item.id, e.target.value)}
                placeholder="Add reviewer notes or clarification requests..."
                className="w-full bg-neutral-900 border border-neutral-800 rounded px-2.5 py-1 text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-amber-500 font-sans"
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
