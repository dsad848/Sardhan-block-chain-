import React, { useState } from 'react';
import { REPO_PACKAGES } from '../data/repoStructure';
import { RepoPackage } from '../types/protocol';
import { FolderTree, FileCode, Layers, Search, CheckCircle2 } from 'lucide-react';

export const RepoExplorer: React.FC = () => {
  const [selectedPkg, setSelectedPkg] = useState<RepoPackage>(REPO_PACKAGES[0]);
  const [searchTerm, setSearchTerm] = useState('');

  const filteredPackages = REPO_PACKAGES.filter((pkg) =>
    pkg.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    pkg.description.toLowerCase().includes(searchTerm.toLowerCase()) ||
    pkg.files.some(f => f.name.toLowerCase().includes(searchTerm.toLowerCase()))
  );

  return (
    <div className="space-y-6">
      <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-neutral-800">
          <div>
            <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
              <span className="text-amber-400 font-semibold">PHASE 3 BLUEPRINT</span>
              <span aria-hidden="true">·</span>
              <span>MODULAR MONOREPO ARCHITECTURE</span>
            </div>
            <h2 className="text-xl lg:text-2xl font-bold text-neutral-100 tracking-tight">
              Repository Structure & Module Blueprints
            </h2>
            <p className="text-sm text-neutral-400 mt-1 max-w-2xl leading-relaxed">
              Explore the independent package interfaces, data flow invariants, and module isolation boundaries for Sardhan Chain before commencement of implementation.
            </p>
          </div>

          <div className="relative">
            <Search className="w-4 h-4 text-neutral-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              placeholder="Search packages, modules..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-9 pr-4 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs font-mono text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-amber-500 w-64"
            />
          </div>
        </div>

        {/* Package Selector & Content */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 pt-6">
          {/* Packages Sidebar */}
          <div className="lg:col-span-4 space-y-2">
            <div className="text-xs font-mono text-neutral-400 px-1 mb-2 flex items-center gap-1.5">
              <FolderTree className="w-3.5 h-3.5 text-amber-500" />
              <span>CORE PACKAGES ({REPO_PACKAGES.length})</span>
            </div>
            <div className="space-y-1.5">
              {filteredPackages.map((pkg) => {
                const isSelected = pkg.name === selectedPkg.name;
                return (
                  <button
                    key={pkg.name}
                    onClick={() => setSelectedPkg(pkg)}
                    className={`w-full text-left p-3 rounded-lg border transition-all ${
                      isSelected
                        ? 'bg-amber-500/10 border-amber-500/60 text-amber-200'
                        : 'bg-neutral-950/60 border-neutral-800/80 text-neutral-300 hover:bg-neutral-800/40'
                    }`}
                  >
                    <div className="font-mono text-xs font-bold flex items-center justify-between">
                      <span className={isSelected ? 'text-amber-400' : 'text-neutral-200'}>
                        {pkg.name}
                      </span>
                      <span className="text-[10px] text-neutral-500 font-normal">
                        {pkg.files.length} files
                      </span>
                    </div>
                    <p className="text-xs text-neutral-400 mt-1 line-clamp-1">
                      {pkg.description}
                    </p>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Selected Package Details */}
          <div className="lg:col-span-8 bg-neutral-950 border border-neutral-800 rounded-lg p-6 space-y-6">
            <div>
              <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
                <span className="text-amber-400 font-bold">{selectedPkg.path}</span>
                <span aria-hidden="true">·</span>
                <span>CLEAN ISOLATED MODULE</span>
              </div>
              <h3 className="text-lg font-bold text-neutral-100 font-mono">
                {selectedPkg.name}
              </h3>
              <p className="text-sm text-neutral-300 mt-1 leading-relaxed">
                {selectedPkg.description}
              </p>
            </div>

            {/* Inbound Dependencies */}
            <div className="p-3 bg-neutral-900/60 rounded border border-neutral-800">
              <span className="text-xs font-mono text-neutral-400 block mb-1">
                PACKAGE DEPENDENCIES:
              </span>
              <div className="flex flex-wrap gap-2">
                {selectedPkg.dependencies.map((dep, idx) => (
                  <span
                    key={idx}
                    className="text-xs font-mono px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 border border-neutral-700/60"
                  >
                    {dep}
                  </span>
                ))}
              </div>
            </div>

            {/* File List */}
            <div>
              <div className="text-xs font-mono text-neutral-400 mb-3 flex items-center gap-1.5">
                <FileCode className="w-3.5 h-3.5 text-amber-400" />
                <span>SOURCE FILES & KEY PUBLIC INTERFACES:</span>
              </div>

              <div className="space-y-3">
                {selectedPkg.files.map((file) => (
                  <div
                    key={file.name}
                    className="p-3.5 bg-neutral-900/40 rounded-lg border border-neutral-800/80 space-y-2"
                  >
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-mono font-bold text-amber-300">
                        {file.name}
                      </span>
                    </div>
                    <p className="text-xs text-neutral-300 leading-relaxed font-sans">
                      {file.description}
                    </p>

                    <div className="pt-2 border-t border-neutral-850 space-y-1">
                      <span className="text-[10px] font-mono text-neutral-500 uppercase tracking-wider block">
                        Core Method Signatures:
                      </span>
                      {file.keyInterfaces.map((iface, iIdx) => (
                        <div
                          key={iIdx}
                          className="text-[11px] font-mono text-neutral-300 bg-neutral-950 px-2.5 py-1 rounded border border-neutral-850 overflow-x-auto"
                        >
                          {iface}
                        </div>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
