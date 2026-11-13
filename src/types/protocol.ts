export type ProtocolSection = 
  | 'overview'
  | 'testnet'
  | 'wallet'
  | 'benchmarks'
  | 'audit'
  | 'math'
  | 'risk'
  | 'research'
  | 'specification'
  | 'consensus'
  | 'threat-model'
  | 'repository'
  | 'tokenomics'
  | 'mempool'
  | 'checklist';

export interface AuditFinding {
  id: string;
  domain: string;
  title: string;
  severity: 'Critical' | 'High' | 'Medium';
  affectedDocs: string[];
  description: string;
  exploitScenario: string;
  remediation: string;
  testCase: string;
  mathematicalProof?: string;
}

export interface ProtocolDoc {
  id: ProtocolSection;
  title: string;
  badge: string;
  rfcNumber: string;
  summary: string;
  markdownContent: string;
}

export interface EpochData {
  epoch: number;
  blockStart: number;
  blockEnd: number;
  sdrPerBlock: number;
  epochTotalSdr: number;
  cumulativeSdr: number;
  percentEmitted: number;
  powShareSdr: number;
  posShareSdr: number;
}

export interface ThreatVector {
  id: string;
  name: string;
  severity: 'Critical' | 'High' | 'Medium' | 'Low';
  subsystem: string;
  description: string;
  purePowOutcome: string;
  purePosOutcome: string;
  sardhanMitigation: string;
  mathematicalGuarantee: string;
}

export interface RepoPackage {
  name: string;
  path: string;
  description: string;
  dependencies: string[];
  files: {
    name: string;
    description: string;
    keyInterfaces: string[];
  }[];
}

export interface ChecklistItem {
  id: string;
  category: 'Tokenomics' | 'Consensus' | 'Security' | 'Architecture' | 'Mempool' | 'Governance';
  title: string;
  description: string;
  status: 'Pending' | 'Approved' | 'Requires Clarification';
  notes: string;
}
