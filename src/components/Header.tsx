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
      {/* Zone 1: Single brand wordmark */}
      <button
        onClick={() => setActiveTab('testnet')}
        className="text-base font-extrabold tracking-wider text-amber-400 whitespace-nowrap shrink-0 hover:text-amber-300 transition-colors"
      >
        SARDHAN CHAIN
      </button>

      {/* Zone 2: 5 single-line text navigation links */}
      <nav className="hidden lg:flex items-center gap-6 text-sm font-medium text-neutral-400">
        <button
          onClick={() => setActiveTab('testnet')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'testnet' ? 'text-amber-400 font-semibold' : ''
          }`}
        >
          Live Explorer
        </button>
        <button
          onClick={() => setActiveTab('wallet')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'wallet' ? 'text-amber-400 font-semibold' : ''
          }`}
        >
          SDR Wallet
        </button>
        <button
          onClick={() => setActiveTab('benchmarks')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            activeTab === 'benchmarks' ? 'text-amber-400 font-semibold' : ''
          }`}
        >
          Benchmarks & Gates
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
          onClick={() => setActiveTab('overview')}
          className={`transition-colors whitespace-nowrap shrink-0 hover:text-neutral-100 ${
            !['testnet', 'wallet', 'benchmarks', 'audit'].includes(activeTab)
              ? 'text-amber-400 font-semibold'
              : ''
          }`}
        >
          Specifications
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
