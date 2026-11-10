import React, { useState } from 'react';
import { AUDIT_FINDINGS } from '../data/auditFindings';
import { AuditFinding } from '../types/protocol';
import { 
  AlertOctagon, 
  AlertTriangle, 
  ShieldAlert, 
  CheckCircle2, 
  Terminal, 
  FileText, 
  Filter, 
  PauseCircle,
  ChevronRight
} from 'lucide-react';

export const AuditReportViewer: React.FC = () => {
  const [selectedSeverity, setSelectedSeverity] = useState<'All' | 'Critical' | 'High' | 'Medium'>('All');
  const [selectedFinding, setSelectedFinding] = useState<AuditFinding>(AUDIT_FINDINGS[0]);

  const filteredFindings = AUDIT_FINDINGS.filter(f => 
    selectedSeverity === 'All' ? true : f.severity === selectedSeverity
  );

  const criticalCount = AUDIT_FINDINGS.filter(f => f.severity === 'Critical').length;
  const highCount = AUDIT_FINDINGS.filter(f => f.severity === 'High').length;
  const mediumCount = AUDIT_FINDINGS.filter(f => f.severity === 'Medium').length;

  return (
    <div className="space-y-6">
      {/* Audit Banner */}
      <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6 pb-6 border-b border-neutral-800">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-xs font-mono text-neutral-400">
              <span className="text-rose-400 font-bold">SECURITY REVIEW COMPLETE</span>
              <span aria-hidden="true">·</span>
              <span>INDEPENDENT PROTOCOL AUDIT</span>
              <span aria-hidden="true">·</span>
              <span className="text-amber-400">ACTION: PROTOCOL HOLD</span>
            </div>
            <h2 className="text-xl lg:text-2xl font-extrabold text-neutral-100 tracking-tight">
              Protocol Vulnerability Assessment & Security Audit
            </h2>
            <p className="text-sm text-neutral-300 max-w-3xl leading-relaxed">
              In strict compliance with the project rule (<strong className="text-rose-300">"DO NOT WRITE BLOCKCHAIN CODE. Do not claim that the blockchain is secure without evidence."</strong>), an independent security audit was performed across all 7 protocol specifications.
            </p>
          </div>

          <div className="flex items-center gap-2 p-3 bg-rose-950/30 border border-rose-900/60 rounded-lg text-rose-300 text-xs font-mono shrink-0">
            <PauseCircle className="w-5 h-5 text-rose-400 shrink-0" />
            <div>
              <span className="font-bold block text-rose-200">IMPLEMENTATION GATE ACTIVE</span>
              <span>Awaiting Protocol Approval</span>
            </div>
          </div>
        </div>

        {/* Severity Count Summary Strip */}
        <div className="grid grid-cols-1 sm:grid-cols-4 gap-4 pt-6">
          <div className="p-4 rounded-lg bg-neutral-950 border border-neutral-800">
            <div className="text-xs font-mono text-neutral-500">TOTAL FINDINGS</div>
            <div className="text-2xl font-bold font-mono text-neutral-100 mt-1 tabular-nums">
              {AUDIT_FINDINGS.length}
            </div>
            <div className="text-xs text-neutral-400 mt-1">Across 7 Protocol Domains</div>
          </div>

          <div className="p-4 rounded-lg bg-rose-950/20 border border-rose-900/50">
            <div className="text-xs font-mono text-rose-400 font-semibold">CRITICAL RISK</div>
            <div className="text-2xl font-bold font-mono text-rose-300 mt-1 tabular-nums">
              {criticalCount}
            </div>
            <div className="text-xs text-rose-400/80 mt-1">Immediate protocol break</div>
          </div>

          <div className="p-4 rounded-lg bg-amber-950/20 border border-amber-900/50">
            <div className="text-xs font-mono text-amber-400 font-semibold">HIGH RISK</div>
            <div className="text-2xl font-bold font-mono text-amber-300 mt-1 tabular-nums">
              {highCount}
            </div>
            <div className="text-xs text-amber-400/80 mt-1">Network split / centralizing</div>
          </div>

          <div className="p-4 rounded-lg bg-neutral-950 border border-neutral-800">
            <div className="text-xs font-mono text-neutral-400">MEDIUM RISK</div>
            <div className="text-2xl font-bold font-mono text-neutral-300 mt-1 tabular-nums">
              {mediumCount}
            </div>
            <div className="text-xs text-neutral-500 mt-1">Specification discrepancies</div>
          </div>
        </div>
      </div>

      {/* Filter Tabs */}
      <div className="flex items-center gap-2 border-b border-neutral-800 pb-3">
        <span className="text-xs font-mono text-neutral-400 flex items-center gap-1.5 mr-2">
          <Filter className="w-3.5 h-3.5 text-amber-400" />
          <span>Filter Severity:</span>
        </span>
        {(['All', 'Critical', 'High', 'Medium'] as const).map(sev => (
          <button
            key={sev}
            onClick={() => setSelectedSeverity(sev)}
            className={`px-3 py-1 rounded-md text-xs font-mono font-medium transition-colors ${
              selectedSeverity === sev
                ? 'bg-amber-400 text-neutral-950 font-bold'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800'
            }`}
          >
            {sev}
          </button>
        ))}
      </div>

      {/* Audit Findings Browser */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Findings List */}
        <div className="lg:col-span-5 space-y-2">
          {filteredFindings.map(finding => {
            const isSelected = finding.id === selectedFinding.id;
            return (
              <button
                key={finding.id}
                onClick={() => setSelectedFinding(finding)}
                className={`w-full text-left p-3.5 rounded-lg border transition-all flex items-center justify-between ${
                  isSelected
                    ? 'bg-amber-500/10 border-amber-500/60 text-amber-200'
                    : 'bg-neutral-950/60 border-neutral-800 text-neutral-300 hover:bg-neutral-800/40'
                }`}
              >
                <div>
                  <div className="flex items-center gap-2">
                    <span className={`text-[10px] font-mono px-1.5 py-0.5 rounded font-bold ${
                      finding.severity === 'Critical'
                        ? 'bg-rose-950 text-rose-300 border border-rose-800/60'
                        : finding.severity === 'High'
                        ? 'bg-amber-950 text-amber-300 border border-amber-800/60'
                        : 'bg-neutral-800 text-neutral-300 border border-neutral-700'
                    }`}>
                      {finding.severity}
                    </span>
                    <span className="text-xs font-mono text-neutral-400">{finding.domain}</span>
                  </div>
                  <div className="text-sm font-semibold mt-1.5 text-neutral-100">
                    {finding.title}
                  </div>
                  <div className="text-xs font-mono text-neutral-500 mt-0.5">
                    ID: {finding.id}
                  </div>
                </div>
                <ChevronRight className={`w-4 h-4 shrink-0 transition-transform ${isSelected ? 'translate-x-1 text-amber-400' : 'text-neutral-600'}`} />
              </button>
            );
          })}
        </div>

        {/* Selected Finding Detail Sheet */}
        <div className="lg:col-span-7 bg-neutral-950 border border-neutral-800 rounded-lg p-6 space-y-5">
          <div className="border-b border-neutral-850 pb-4">
            <div className="flex items-center justify-between gap-2 mb-2">
              <span className={`text-xs font-mono px-2 py-0.5 rounded font-bold ${
                selectedFinding.severity === 'Critical'
                  ? 'bg-rose-950 text-rose-300 border border-rose-800'
                  : selectedFinding.severity === 'High'
                  ? 'bg-amber-950 text-amber-300 border border-amber-800'
                  : 'bg-neutral-800 text-neutral-300'
              }`}>
                {selectedFinding.severity.toUpperCase()} RISK
              </span>
              <span className="text-xs font-mono text-neutral-400">
                AFFECTS: {selectedFinding.affectedDocs.join(', ')}
              </span>
            </div>
            <h3 className="text-lg font-bold text-neutral-100">{selectedFinding.title}</h3>
            <p className="text-xs text-neutral-400 font-mono mt-1">Domain: {selectedFinding.domain}</p>
          </div>

          {/* Description */}
          <div>
            <span className="text-xs font-mono text-neutral-400 uppercase tracking-wider block mb-1">
              VULNERABILITY DESCRIPTION
            </span>
            <p className="text-sm text-neutral-300 leading-relaxed font-sans">
              {selectedFinding.description}
            </p>
          </div>

          {/* Exploit Scenario */}
          <div className="p-3.5 bg-rose-950/20 border border-rose-900/40 rounded-lg">
            <div className="flex items-center gap-2 text-xs font-mono text-rose-400 font-semibold mb-1.5">
              <AlertOctagon className="w-4 h-4" />
              <span>CONCRETE EXPLOIT SCENARIO</span>
            </div>
            <p className="text-xs text-rose-200/90 leading-relaxed">
              {selectedFinding.exploitScenario}
            </p>
          </div>

          {/* Proposed Remediation */}
          <div className="p-3.5 bg-emerald-950/20 border border-emerald-900/40 rounded-lg">
            <div className="flex items-center gap-2 text-xs font-mono text-emerald-400 font-semibold mb-1.5">
              <CheckCircle2 className="w-4 h-4" />
              <span>SPECIFICATION REMEDIATION</span>
            </div>
            <p className="text-xs text-emerald-200/90 leading-relaxed">
              {selectedFinding.remediation}
            </p>
          </div>

          {/* Mathematical Proof */}
          {selectedFinding.mathematicalProof && (
            <div className="p-3.5 bg-neutral-900/60 border border-neutral-800 rounded-lg font-mono text-xs">
              <span className="text-neutral-400 block mb-1 font-sans font-semibold">
                MATHEMATICAL VERIFICATION / BOUND:
              </span>
              <span className="text-amber-300/90 leading-relaxed block">
                {selectedFinding.mathematicalProof}
              </span>
            </div>
          )}

          {/* Automated Test Case */}
          <div className="p-3.5 bg-neutral-900/90 border border-neutral-800 rounded-lg font-mono text-xs space-y-1">
            <div className="flex items-center gap-2 text-amber-400 font-semibold font-sans mb-1">
              <Terminal className="w-4 h-4" />
              <span>REQUIRED TEST CASE SPECIFICATION:</span>
            </div>
            <div className="text-neutral-300 bg-neutral-950 p-2.5 rounded border border-neutral-850">
              {selectedFinding.testCase}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
