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
import { 
  ShieldCheck, 
  Cpu, 
  Coins, 
  Zap, 
  GitBranch, 
  FileText, 
  Layers, 
  CheckCircle2, 
  Download,
  AlertCircle,
  Scale,
  Radio
} from 'lucide-react';

export default function App() {
  const [activeTab, setActiveTab] = useState<ProtocolSection>('overview');

  const currentDoc = PROTOCOL_DOCS.find(d => d.id === activeTab) || PROTOCOL_DOCS[0];

  const handleExportAll = () => {
    const combinedContent = [
      '# SARDHAN CHAIN (SDR) — PROTOCOL SPECIFICATION BUNDLE',
      `Generated: ${new Date().toISOString()}`,
      'Network: Sardhan Chain Layer-1',
      'Ticker: SDR',
      'Max Supply: 21,000,000 SDR (Strict Hard Cap)',
      'Premine: 0.00000000 SDR (0%)',
      'Consensus: Hybrid 50% PoW / 50% PoS (Proof of Power)',
      'Transaction Fees: Zero Fees (Secured via Client Micro-PoW)',
      '',
      '========================================================================',
      ''
    ];

    PROTOCOL_DOCS.forEach(doc => {
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
      {/* 3-Zone Top Navigation Contract */}
      <Header
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        onExportAll={handleExportAll}
      />

      {/* Main Content Area */}
      <main className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8">
        {/* Protocol Invariants Banner */}
        <div className="rounded-xl border border-neutral-800 bg-neutral-900/40 p-6">
          <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6">
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-xs font-mono text-neutral-400">
                <span className="text-amber-400 font-bold">PHASE 1 & 2 COMPLETED</span>
                <span aria-hidden="true">·</span>
                <span>INDEPENDENT LAYER-1 BLOCKCHAIN</span>
                <span aria-hidden="true">·</span>
                <span className="text-neutral-500">TICKER: SDR</span>
              </div>
              <h1 className="text-2xl sm:text-3xl font-extrabold text-neutral-100 tracking-tight">
                Sardhan Chain Protocol Architecture
              </h1>
              <p className="text-sm text-neutral-300 max-w-3xl leading-relaxed">
                A sovereign, zero-premine, zero-transaction-fee Layer-1 secured by unified 50/50 Proof of Power (CPU/ARM mining + community staking). In accordance with protocol engineering principles, full architecture, consensus design, and threat modeling are finalized for review before code implementation.
              </p>
            </div>

            {/* Quick Invariants Pill-Free Strip */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 shrink-0">
              <div className="p-3 rounded-lg bg-neutral-950 border border-neutral-800">
                <div className="text-[10px] font-mono text-neutral-500 uppercase">MAX SUPPLY</div>
                <div className="text-sm font-bold font-mono text-amber-400 tabular-nums">21M SDR</div>
                <div className="text-[11px] text-neutral-400">Zero Premine</div>
              </div>

              <div className="p-3 rounded-lg bg-neutral-950 border border-neutral-800">
                <div className="text-[10px] font-mono text-neutral-500 uppercase">CONSENSUS</div>
                <div className="text-sm font-bold font-mono text-neutral-200">50% PoW / 50% PoS</div>
                <div className="text-[11px] text-neutral-400">Proof of Power</div>
              </div>

              <div className="p-3 rounded-lg bg-neutral-950 border border-neutral-800">
                <div className="text-[10px] font-mono text-neutral-500 uppercase">TX FEES</div>
                <div className="text-sm font-bold font-mono text-emerald-400 tabular-nums">0.00 SDR</div>
                <div className="text-[11px] text-neutral-400">Client Micro-PoW</div>
              </div>

              <div className="p-3 rounded-lg bg-neutral-950 border border-neutral-800">
                <div className="text-[10px] font-mono text-neutral-500 uppercase">MINING TARGET</div>
                <div className="text-sm font-bold font-mono text-neutral-200">CPU & ARM Mobile</div>
                <div className="text-[11px] text-neutral-400">AES-NI / ARMv8</div>
              </div>
            </div>
          </div>

          {/* Operational Rule Notice */}
          <div className="mt-5 pt-4 border-t border-neutral-850 flex items-center gap-2.5 text-xs text-amber-300/90 font-mono">
            <AlertCircle className="w-4 h-4 text-amber-400 shrink-0" />
            <span>
              Mandate Enforced: Complete architecture, technical specification, consensus proposal, and security threat model generated. Foundation coding held pending stakeholder review.
            </span>
          </div>
        </div>

        {/* Section Navigation Tabs */}
        <div className="flex items-center gap-1.5 overflow-x-auto pb-2 border-b border-neutral-800 text-xs font-mono">
          <button
            onClick={() => setActiveTab('overview')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'overview'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <FileText className="w-3.5 h-3.5" />
            <span>Overview</span>
          </button>

          <button
            onClick={() => setActiveTab('testnet')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'testnet'
                ? 'bg-amber-950/60 text-amber-300 font-bold border border-amber-800'
                : 'text-amber-400/90 hover:text-amber-300 hover:bg-amber-950/20'
            }`}
          >
            <Radio className="w-3.5 h-3.5 text-emerald-400 animate-pulse" />
            <span>Live 4-Node Testnet</span>
          </button>

          <button
            onClick={() => setActiveTab('audit')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'audit'
                ? 'bg-rose-950/60 text-rose-300 font-bold border border-rose-850'
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
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Scale className="w-3.5 h-3.5 text-amber-400" />
            <span>Consensus Math (Task 1)</span>
          </button>

          <button
            onClick={() => setActiveTab('risk')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'risk'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <AlertCircle className="w-3.5 h-3.5 text-amber-400" />
            <span>Risk Register & Readiness</span>
          </button>

          <button
            onClick={() => setActiveTab('research')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'research'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <span>Research & Verus Analysis</span>
          </button>

          <button
            onClick={() => setActiveTab('specification')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'specification'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <span>Technical Spec (RFC-102)</span>
          </button>

          <button
            onClick={() => setActiveTab('consensus')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'consensus'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Cpu className="w-3.5 h-3.5" />
            <span>Consensus Proposal</span>
          </button>

          <button
            onClick={() => setActiveTab('threat-model')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'threat-model'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <ShieldCheck className="w-3.5 h-3.5" />
            <span>Threat Model & 51% Matrix</span>
          </button>

          <button
            onClick={() => setActiveTab('tokenomics')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'tokenomics'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <Coins className="w-3.5 h-3.5" />
            <span>Emission Simulator</span>
          </button>

          <button
            onClick={() => setActiveTab('mempool')}
            className={`px-3 py-1.5 rounded-lg whitespace-nowrap transition-colors flex items-center gap-1.5 ${
              activeTab === 'mempool'
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
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
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
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
                ? 'bg-neutral-800 text-amber-400 font-bold border border-neutral-750'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-900'
            }`}
          >
            <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
            <span>Review Checklist</span>
          </button>
        </div>

        {/* View Routing */}
        {activeTab === 'testnet' ? (
          <TestnetLiveCluster />
        ) : activeTab === 'audit' ? (
          <div className="space-y-8">
            <AuditReportViewer />
            <DocViewer doc={PROTOCOL_DOCS.find(d => d.id === 'audit')!} />
          </div>
        ) : activeTab === 'math' ? (
          <div className="space-y-8">
            <ConsensusMathVisualizer />
            <DocViewer doc={PROTOCOL_DOCS.find(d => d.id === 'math')!} />
          </div>
        ) : activeTab === 'risk' ? (
          <div className="space-y-8">
            <DocViewer doc={PROTOCOL_DOCS.find(d => d.id === 'risk')!} />
          </div>
        ) : activeTab === 'tokenomics' ? (
          <div className="space-y-8">
            <EmissionCalculator />
            <DocViewer doc={PROTOCOL_DOCS.find(d => d.id === 'tokenomics')!} />
          </div>
        ) : activeTab === 'threat-model' ? (
          <div className="space-y-8">
            <ThreatModelVisualizer />
            <DocViewer doc={PROTOCOL_DOCS.find(d => d.id === 'threat-model')!} />
          </div>
        ) : activeTab === 'mempool' ? (
          <div className="space-y-8">
            <ZeroFeeSimulator />
            <DocViewer doc={PROTOCOL_DOCS.find(d => d.id === 'mempool')!} />
          </div>
        ) : activeTab === 'repository' ? (
          <div className="space-y-8">
            <RepoExplorer />
            <DocViewer doc={PROTOCOL_DOCS.find(d => d.id === 'repository')!} />
          </div>
        ) : activeTab === 'checklist' ? (
          <ReviewChecklist />
        ) : (
          <DocViewer doc={currentDoc} />
        )}
      </main>

      {/* Protocol Architecture Footer */}
      <footer className="mt-auto border-t border-neutral-850 py-6 px-6 bg-neutral-950 text-xs text-neutral-500 font-mono">
        <div className="max-w-7xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-amber-500"></span>
            <span className="text-neutral-400 font-bold">Sardhan Protocol Core</span>
            <span>· MIT Permissive License</span>
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
