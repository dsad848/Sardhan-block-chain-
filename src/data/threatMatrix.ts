import { ThreatVector } from '../types/protocol';

export const THREAT_VECTORS: ThreatVector[] = [
  {
    id: 'attack-51-pow',
    name: '51% Mining Hashrate Attack',
    severity: 'Critical',
    subsystem: 'Consensus / Finality',
    description: 'An attacker controls a majority of CPU mining power (e.g. via rental or botnet) and attempts to secretly mine a private branch to rewrite history and double spend.',
    purePowOutcome: 'Catastrophic failure. Pure PoW chains inevitably succumb to deep reorganizations and double spends once an adversary exceeds 50% hashrate.',
    purePosOutcome: 'Not applicable (pure PoS does not use hashrate).',
    sardhanMitigation: 'Immune. PoW blocks cannot finalize without interleaved PoS blocks. An attacker with 90% hashrate but 5% stake stalls on PoS slots and cannot outpace the honest chain.',
    mathematicalGuarantee: 'P(Reorg) requires α (hashrate share) + β (stake share) > 1.0. A hashrate monopoly without matching coin stake produces an invalid chain.'
  },
  {
    id: 'attack-51-pos',
    name: '51% Staking Cartel Attack',
    severity: 'Critical',
    subsystem: 'Consensus / Staking',
    description: 'A wealthy cartel acquires >50% of circulating coins and attempts to monopolize block production or arbitrarily censor peer transactions.',
    purePowOutcome: 'Not applicable (pure PoW ignores coin balances for consensus).',
    purePosOutcome: 'Catastrophic permanent takeover. A 51% stake cartel permanently controls block production and can never be dethroned.',
    sardhanMitigation: 'Immune. Stakers can only mint PoS blocks (50% of the timeline). They cannot forge PoW blocks, which require thermodynamic CPU energy expenditure.',
    mathematicalGuarantee: 'Honest miners continue producing PoW blocks, anchoring the canonical chain and distributing new coins away from the staking cartel.'
  },
  {
    id: 'attack-mempool-flood',
    name: 'Zero-Fee Mempool Flooding (Denial of Service)',
    severity: 'High',
    subsystem: 'P2P / Mempool',
    description: 'Because transaction fees are zero, an adversary broadcasts millions of garbage transactions to exhaust node RAM and partition the network.',
    purePowOutcome: 'Nodes crash from OOM if fees are zero; or fees spike to exorbitant levels in fee-market chains.',
    purePosOutcome: 'Same as PoW: requires high base fees to mitigate.',
    sardhanMitigation: 'Client-Side Micro-PoW. Every transaction must embed a valid BLAKE3 nonce (~100ms CPU work). Mempool difficulty scales quadratically if occupancy exceeds 50%.',
    mathematicalGuarantee: 'Flooding 100k transactions requires ~28 CPU-hours for the attacker, but full nodes verify all 100k hashes in under 100 milliseconds.'
  },
  {
    id: 'attack-nothing-at-stake',
    name: 'Nothing-at-Stake Fork Creation',
    severity: 'High',
    subsystem: 'Consensus / Fork Choice',
    description: 'In proof-of-stake, signing conflicting blocks on multiple forks costs zero energy, incentivizing validators to vote on all forks simultaneously.',
    purePowOutcome: 'Not applicable (miners must split physical energy between forks).',
    purePosOutcome: 'Consensus instability and inability to resolve chain splits cleanly without complex slashing or subjective finality.',
    sardhanMitigation: 'Dual anchor: PoS blocks must reference real PoW blocks. Dual-signing evidence triggers automatic protocol slashing of the mature staking UTXO.',
    mathematicalGuarantee: 'Slashing confiscates 100% of the offending staking UTXO (50% burned permanently, 50% paid as bounty to the reporting peer).'
  },
  {
    id: 'attack-long-range',
    name: 'Long-Range Historical Reorganization',
    severity: 'High',
    subsystem: 'Consensus / History',
    description: 'An attacker purchases historic private keys that held large balances months ago and mines a secret long chain starting from genesis or deep in the past.',
    purePowOutcome: 'Mitigated in pure PoW by cumulative work requirement (re-mining months of PoW is prohibitively expensive).',
    purePosOutcome: 'Severe vulnerability. Costless historical simulation allows trivial alternate history generation.',
    sardhanMitigation: 'Weak Subjectivity with hard 288-block (~12 hour) reorg horizon. Full nodes reject any reorganization attempt deeper than 288 blocks.',
    mathematicalGuarantee: 'Nodes enforce immutable local checkpoints every 288 blocks. Any fork attempt prior to height (best_height - 288) is dropped immediately.'
  },
  {
    id: 'attack-sybil-eclipse',
    name: 'P2P Sybil & Eclipse Isolation Attack',
    severity: 'Medium',
    subsystem: 'Networking / Gossip',
    description: 'An attacker spins up hundreds of cloud nodes to surround a victim node, isolating it from honest peer blocks and feeding it an artificial chain.',
    purePowOutcome: 'Victim can be eclipsed if all inbound/outbound connections belong to the adversary.',
    purePosOutcome: 'Same vulnerability.',
    sardhanMitigation: 'Subnet bucketing (maximum 4 peer connections per /16 IPv4 or /32 IPv6 subnet), outbound anchor pinning, and hardcoded seed nodes.',
    mathematicalGuarantee: 'Eclipsing requires compromising distinct autonomous systems (ASNs) and IP blocks across independent geographic regions.'
  },
  {
    id: 'attack-time-warp',
    name: 'Difficulty Time-Warp Manipulation',
    severity: 'Medium',
    subsystem: 'Difficulty Adjustment (DAA)',
    description: 'Miners forge block timestamps to make blocks appear to take longer than reality, tricking the difficulty algorithm into lowering target difficulty.',
    purePowOutcome: 'Historical vulnerability on early Bitcoin-derived chains without strict timestamp guards.',
    purePosOutcome: 'Can disrupt staking kernel time intervals.',
    sardhanMitigation: 'Strict MedianTimePast(11) rule: block timestamp must be strictly greater than the median of the past 11 blocks, and within [-15s, +120s] of host time.',
    mathematicalGuarantee: 'Timestamp manipulation is bounded to negligible margins, preventing DAA distortion.'
  }
];
