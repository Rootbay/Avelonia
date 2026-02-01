export type VtScanSource = 'background' | 'optimize' | 'manual';
export type VtScanPhase = 'idle' | 'running' | 'done';

export type VtScanItem = {
  subject: string;
  source: 'startup' | 'registry';
  verdict?: string;
  positives?: number;
  malicious?: number;
  suspicious?: number;
  harmless?: number;
  undetected?: number;
  total_vendors?: number;
  reason?: string;
  permalink?: string;
};

export type VtScanState = {
  phase: VtScanPhase;
  source?: VtScanSource;
  startedAt?: number;
  finishedAt?: number;
  expectedStartup?: number;
  expectedRegistry?: number;
  items: VtScanItem[];
};

const initial: VtScanState = { phase: 'idle', items: [] };

export const vtScan = $state<VtScanState>(initial);

export function beginScan(
  source: VtScanSource,
  expected?: { startup?: number; registry?: number }
) {
  vtScan.phase = 'running';
  vtScan.source = source;
  vtScan.startedAt = Date.now();
  vtScan.expectedStartup = expected?.startup;
  vtScan.expectedRegistry = expected?.registry;
  vtScan.items = [];
}

export function endScan(counts?: { startup?: number; registry?: number }) {
  vtScan.phase = 'done';
  vtScan.finishedAt = Date.now();
  if (counts?.startup !== undefined) vtScan.expectedStartup = counts.startup;
  if (counts?.registry !== undefined) vtScan.expectedRegistry = counts.registry;
}

export function pushReport(rep: {
  subject: string;
  source?: string;
  verdict?: string;
  positives?: number;
  malicious?: number;
  suspicious?: number;
  harmless?: number;
  undetected?: number;
  total_vendors?: number;
  reason?: string;
  permalink?: string;
}) {
  const src = (rep.source === 'registry' ? 'registry' : 'startup') as 'startup' | 'registry';
  const key = rep.subject.trim().toLowerCase();
  const idx = vtScan.items.findIndex(
    (i) => i.subject.trim().toLowerCase() === key && i.source === src
  );

  const next: VtScanItem = {
    subject: rep.subject,
    source: src,
    verdict: rep.verdict,
    positives: rep.positives,
    malicious: rep.malicious,
    suspicious: rep.suspicious,
    harmless: rep.harmless,
    undetected: rep.undetected,
    total_vendors: rep.total_vendors,
    reason: rep.reason,
    permalink: rep.permalink,
  };

  if (idx >= 0) {
    vtScan.items[idx] = next;
  } else {
    vtScan.items.push(next);
  }
}

export function resetScan() {
  vtScan.phase = 'idle';
  vtScan.source = undefined;
  vtScan.startedAt = undefined;
  vtScan.finishedAt = undefined;
  vtScan.expectedStartup = undefined;
  vtScan.expectedRegistry = undefined;
  vtScan.items = [];
}
