import { writable } from 'svelte/store';

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

export const vtScan = writable<VtScanState>(initial);

export function beginScan(source: VtScanSource, expected?: { startup?: number; registry?: number }) {
  vtScan.set({
    phase: 'running',
    source,
    startedAt: Date.now(),
    expectedStartup: expected?.startup,
    expectedRegistry: expected?.registry,
    items: [],
  });
}

export function endScan(counts?: { startup?: number; registry?: number }) {
  vtScan.update((s) => ({
    ...s,
    phase: 'done',
    finishedAt: Date.now(),
    expectedStartup: counts?.startup ?? s.expectedStartup,
    expectedRegistry: counts?.registry ?? s.expectedRegistry,
  }));
}

export function pushReport(rep: { subject: string; source?: string; verdict?: string; positives?: number; malicious?: number; suspicious?: number; harmless?: number; undetected?: number; total_vendors?: number; reason?: string; permalink?: string }) {
  const src = (rep.source === 'registry' ? 'registry' : 'startup') as 'startup' | 'registry';
  vtScan.update((s) => {
    const key = rep.subject.trim().toLowerCase();
    const items = s.items.slice();
    const idx = items.findIndex((i) => i.subject.trim().toLowerCase() === key && i.source === src);
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
    if (idx >= 0) items[idx] = next; else items.push(next);
    return { ...s, items };
  });
}

export function resetScan() {
  vtScan.set(initial);
}
