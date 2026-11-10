import React from 'react';
import { ProtocolSection } from '../types/protocol';
import { Download } from 'lucide-react';

interface HeaderProps {
  activeTab: ProtocolSection;
  setActiveTab: (tab: ProtocolSection) => void;
  onExportAll: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  activeTab,
  setActiveTab,
  onExportAll
}) => {
  return (
    <header className="sticky top-0 z-40 bg-neutral-950/90 backdrop-blur-md border-b border-neutral-800 flex items-center justify-between gap-8 px-6 py-4">
      {/* Zone 1: Single text element wordmark */}
      <div 
        onClick={() => setActiveTab('overview')}
        className="cursor-pointer text-base font-bold tracking-tight text-neutral-100 flex items-center gap-2.5 whitespace-nowrap shrink-0"
      >
        <span className="w-2.5 h-2.5 rounded-sm bg-amber-500 inline-block shadow-[0_0_10px_rgba(245,158,11,0.5)]"></span>
        <span className="text-amber-400 font-extrabold tracking-wider">SARDHAN CHAIN</span>
        <span className="text-neutral-500 font-mono text-xs">SDR · RFC-100</span>
      </div>

      {/* Zone 2: 4–5 clean single-line text navigation links */}
      <nav className="hidden lg:flex items-center gap-6 text-sm font-medium text-neutral-400">
        <button
          onClick={() => setActiveTab('overview')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'overview' ? 'text-amber-400 font-semibold' : ''
          }`}
        >
          Overview
        </button>
        <button
          onClick={() => setActiveTab('testnet')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'testnet' ? 'text-amber-400 font-semibold flex items-center gap-1.5' : 'flex items-center gap-1.5'
          }`}
        >
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
          <span>Live Testnet</span>
        </button>
        <button
          onClick={() => setActiveTab('audit')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'audit' ? 'text-rose-400 font-semibold' : ''
          }`}
        >
          Security Audit
        </button>
        <button
          onClick={() => setActiveTab('math')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'math' ? 'text-amber-400 font-semibold' : ''
          }`}
        >
          Consensus Math
        </button>
        <button
          onClick={() => setActiveTab('risk')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'risk' ? 'text-amber-400 font-semibold' : ''
          }`}
        >
          Risk Register
        </button>
        <button
          onClick={() => setActiveTab('tokenomics')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'tokenomics' ? 'text-amber-400 font-semibold' : ''
          }`}
        >
          Tokenomics
        </button>
        <button
          onClick={() => setActiveTab('repository')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'repository' ? 'text-amber-400 font-semibold' : ''
          }`}
        >
          Repository
        </button>
        <button
          onClick={() => setActiveTab('checklist')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'checklist' ? 'text-amber-400 font-semibold' : ''
          }`}
        >
          Review Checklist
        </button>
      </nav>

      {/* Zone 3: 1 primary action */}
      <div className="flex items-center gap-3 shrink-0">
        <button
          onClick={onExportAll}
          className="flex items-center gap-2 px-3.5 py-1.5 text-xs font-semibold text-neutral-950 bg-amber-400 rounded-lg hover:bg-amber-300 transition-colors whitespace-nowrap shrink-0 shadow-sm"
        >
          <Download className="w-3.5 h-3.5" />
          <span>Export Specifications</span>
        </button>
      </div>
    </header>
  );
};
