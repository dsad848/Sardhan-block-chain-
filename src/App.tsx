import React, { useState } from 'react';
import { ProtocolSection } from './types/protocol';
import { PROTOCOL_DOCS } from './data/protocolDocuments';
import { Header } from './components/Header';
import { DocViewer } from './components/DocViewer';
import { EmissionCalculator } from './components/EmissionCalculator';
import { ThreatModelVisualizer } from './components/ThreatModelVisualizer';
import { ZeroFeeSimulator } from './components/ZeroFeeSimulator';
import { RepoExplorer } from './components/RepoExplorer';
import { ReviewChecklist } from './components/ReviewChecklist';
import { AuditReportViewer } from './components/AuditReportViewer';
import { ConsensusMathVisualizer } from './components/ConsensusMathVisualizer';
import { TestnetLiveCluster } from './components/TestnetLiveCluster';
import { WebWallet } from './components/WebWallet';
import { BenchmarksGatesPanel } from './components/BenchmarksGatesPanel';
import {
  ShieldCheck,
  Cpu,
  Coins,
  Zap,
  FileText,
  Layers,
  CheckCircle2,
  AlertCircle,
  Scale,
  Radio,
  Wallet,
  Gauge,
} from 'lucide-react';

export default function App() {
  const [activeTab, setActiveTab] = useState<ProtocolSection>('testnet');

  const currentDoc = PROTOCOL_DOCS.find((d) => d.id === activeTab) || PROTOCOL_DOCS[0];

  const handleExportAll = () => {
    const combinedContent = [
      '# SARDHAN CHAIN (SDR) — PROTOCOL SPECIFICATION BUNDLE',
      `Generated: ${new Date().toISOString()}`,
      'Network: Sardhan Chain Layer-1',
      'Ticker: SDR',
      'Max Supply: 21,000,000 SDR (Strict Hard Cap · 20,999,999.9769 SDR via integer bit-shift halving)',
      'Premine: 0.00000000 SDR (0%)',
      'Consensus: Hybrid 50% PoW (SardhanHash) / 50% PoS (Proof of Power)',
      'Transaction Fees: Zero Fees (Secured via Block-Bound Client Micro-PoW)',
      'Release Gate: MAINNET_ENABLED = false (Testnet Active · Pending External Audit & GPU Benchmarks)',
      '',
      '========================================================================',
      '',
    ];

    PROTOCOL_DOCS.forEach((doc) => {
      combinedContent.push(`\n\n## [${doc.rfcNumber}] ${doc.title.toUpperCase()}\n\n`);
      combinedContent.push(doc.markdownContent);
      combinedContent.push('\n\n---');
    });

    const blob = new Blob([combinedContent.join('\n')], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sardhan_chain_protocol_rfc_bundle_${Date.now()}.md`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  return (
    <div className="min-h-screen bg-neutral-950 text-neutral-100 flex flex-col font-sans">
      <Header activeTab={activeTab} setActiveTab={setActiveTab} onExportAll={handleExportAll} />

      <main className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8">
        {/* Protocol Invariants Banner */}
        <div className="rounded-xl border border-neutral-800 bg-neutral-900/40 p-6">
          <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6">
            <div className="space-y-2">
              <div className="flex flex-wrap items-center gap-2 text-xs font-mono text-neutral-400">
                <span className="text-emerald-400 font-bold">RUST LAYER-1 CORE & 3-NODE TESTNET LIVE</span>
                <span aria-hidden="true">·</span>
                <span>TICKER: SDR</span>
                <span aria-hidden="true">·</span>
                <span className="text-amber-400">MAINNET_ENABLED = false</span>
              </div>
              <h1 className="text-2xl sm:text-3xl font-extrabold text-neutral-100 tracking-tight">
                Sardhan Chain Layer-1 Network & Release Candidate
              </h1>
              <p className="text-sm text-neutral-300 max-w-3xl leading-relaxed">
                Independent Layer-1 blockchain in Rust (<code className="font-mono text-amber-400">sardhan-core</code>) with persistent block/UTXO storage, hybrid 50/50 PoW+PoS consensus, zero mandatory transaction fees secured by block-bound micro-PoW, multi-peer TCP networking, and non-custodial Ed25519 wallets.
              </p>
            </div>

            <div className="grid grid-cols-2 sm:grid-cols-4 border border-neutral-800 rounded-lg divide-y sm:divide-y-0 sm:divide-x divide-neutral-800 bg-neutral-950 shrink-0">
              <div className="p-3">
                <div className="text-[10px] font-mono text-neutral-500 uppercase">MAX SUPPLY</div>
                <div className="text-sm font-bold font-mono text-amber-400 tabular-nums">21M SDR</div>
                <div className="text-[11px] text-neutral-400">0% Premine</div>
              </div>

              <div className="p-3">
                <div className="text-[10px] font-mono text-neutral-500 uppercase">CONSENSUS</div>
                <div className="text-sm font-bold font-mono text-neutral-200">50% PoW / 50% PoS</div>
                <div className="text-[11px] text-neutral-400">Proof of Power</div>
              </div>

              <div className="p-3">
                <div className="text-[10px] font-mono text-neutral-500 uppercase">TX FEES</div>
                <div className="text-sm font-bold font-mono text-emerald-400 tabular-nums">0.00 SDR</div>
                <div className="text-[11px] text-neutral-400">Micro-PoW Bound</div>
              </div>

              <div className="p-3">
                <div className="text-[10px] font-mono text-neutral-500 uppercase">POW KERNEL</div>
                <div className="text-sm font-bold font-mono text-neutral-200">SardhanHash</div>
                <div className="text-[11px] text-neutral-400">8 MiB Scratchpad</div>
              </div>
            </div>
          </div>

          <div className="mt-5 pt-4 border-t border-neutral-800 flex items-center gap-2.5 text-xs text-amber-300/90 font-mono">
            <AlertCircle className="w-4 h-4 text-amber-400 shrink-0" />
            <span>
              Security Release Gate Active: Public Mainnet remains disabled (`MAINNET_ENABLED = false`) until external third-party security audit and physical GPU resistance hardware benchmarks finish.
            </span>
          </div>
        </div>

        {/* Section Navigation Tabs */}
        <div className="flex items-center gap-1.5 overflow-x-auto pb-2 border-b border-neutral-800 text-xs font-mono">
          <button
            onClick={() => setActiveTab('testnet')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'testnet'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Radio className="w-3.5 h-3.5 text-emerald-400" />
            <span>Live 3-Node Explorer</span>
          </button>

          <button
            onClick={() => setActiveTab('wallet')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'wallet'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Wallet className="w-3.5 h-3.5 text-amber-400" />
            <span>SDR Web Wallet</span>
          </button>

          <button
            onClick={() => setActiveTab('benchmarks')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'benchmarks'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Gauge className="w-3.5 h-3.5 text-amber-400" />
            <span>Benchmarks & Release Gates</span>
          </button>

          <button
            onClick={() => setActiveTab('overview')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'overview'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <FileText className="w-3.5 h-3.5" />
            <span>Architecture Overview</span>
          </button>

          <button
            onClick={() => setActiveTab('audit')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'audit'
                ? 'bg-rose-950/60 text-rose-300 font-bold border border-rose-800'
                : 'text-rose-400/80 hover:text-rose-300 hover:bg-rose-950/20'
            }`}
          >
            <ShieldCheck className="w-3.5 h-3.5 text-rose-400" />
            <span>Audit Findings</span>
          </button>

          <button
            onClick={() => setActiveTab('math')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'math'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Scale className="w-3.5 h-3.5 text-amber-400" />
            <span>Consensus Math</span>
          </button>

          <button
            onClick={() => setActiveTab('risk')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'risk'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <AlertCircle className="w-3.5 h-3.5 text-amber-400" />
            <span>Risk Register</span>
          </button>

          <button
            onClick={() => setActiveTab('specification')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'specification'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <span>RFC-102 Spec</span>
          </button>

          <button
            onClick={() => setActiveTab('consensus')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'consensus'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Cpu className="w-3.5 h-3.5" />
            <span>Consensus Design</span>
          </button>

          <button
            onClick={() => setActiveTab('threat-model')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'threat-model'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <ShieldCheck className="w-3.5 h-3.5" />
            <span>51% Threat Matrix</span>
          </button>

          <button
            onClick={() => setActiveTab('tokenomics')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'tokenomics'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Coins className="w-3.5 h-3.5" />
            <span>Tokenomics</span>
          </button>

          <button
            onClick={() => setActiveTab('mempool')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'mempool'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Zap className="w-3.5 h-3.5" />
            <span>Zero-Fee Mempool</span>
          </button>

          <button
            onClick={() => setActiveTab('repository')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'repository'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Layers className="w-3.5 h-3.5" />
            <span>Repository Blueprint</span>
          </button>

          <button
            onClick={() => setActiveTab('checklist')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'checklist'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
            <span>Checklist</span>
          </button>
        </div>

        {/* View Routing */}
        {activeTab === 'testnet' ? (
          <TestnetLiveCluster />
        ) : activeTab === 'wallet' ? (
          <WebWallet />
        ) : activeTab === 'benchmarks' ? (
          <BenchmarksGatesPanel />
        ) : activeTab === 'audit' ? (
          <div className="space-y-8">
            <AuditReportViewer />
            <DocViewer doc={PROTOCOL_DOCS.find((d) => d.id === 'audit')!} />
          </div>
        ) : activeTab === 'math' ? (
          <div className="space-y-8">
            <ConsensusMathVisualizer />
            <DocViewer doc={PROTOCOL_DOCS.find((d) => d.id === 'math')!} />
          </div>
        ) : activeTab === 'risk' ? (
          <div className="space-y-8">
            <DocViewer doc={PROTOCOL_DOCS.find((d) => d.id === 'risk')!} />
          </div>
        ) : activeTab === 'tokenomics' ? (
          <div className="space-y-8">
            <EmissionCalculator />
            <DocViewer doc={PROTOCOL_DOCS.find((d) => d.id === 'tokenomics')!} />
          </div>
        ) : activeTab === 'threat-model' ? (
          <div className="space-y-8">
            <ThreatModelVisualizer />
            <DocViewer doc={PROTOCOL_DOCS.find((d) => d.id === 'threat-model')!} />
          </div>
        ) : activeTab === 'mempool' ? (
          <div className="space-y-8">
            <ZeroFeeSimulator />
            <DocViewer doc={PROTOCOL_DOCS.find((d) => d.id === 'mempool')!} />
          </div>
        ) : activeTab === 'repository' ? (
          <div className="space-y-8">
            <RepoExplorer />
            <DocViewer doc={PROTOCOL_DOCS.find((d) => d.id === 'repository')!} />
          </div>
        ) : activeTab === 'checklist' ? (
          <ReviewChecklist />
        ) : (
          <DocViewer doc={currentDoc} />
        )}
      </main>

      <footer className="mt-auto border-t border-neutral-800 py-6 px-6 bg-neutral-950 text-xs text-neutral-500 font-mono">
        <div className="max-w-7xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-2">
            <span className="text-neutral-300 font-bold">Sardhan Protocol Core</span>
            <span>· MIT License</span>
            <span>· 21,000,000 SDR Sovereign Hard Cap</span>
          </div>

          <div className="flex items-center gap-4 text-neutral-400">
            <span>Proof of Power (PoW + PoS)</span>
            <span>·</span>
            <span>Zero Transaction Fees</span>
            <span>·</span>
            <span>No Premine</span>
          </div>
        </div>
      </footer>
    </div>
  );
}
