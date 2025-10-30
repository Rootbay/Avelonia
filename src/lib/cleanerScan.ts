import { writable } from 'svelte/store';

export type CleanerPhase = 'idle' | 'running' | 'done';

export type CleanerScanState = {
  phase: CleanerPhase;
  found: number;
  startedAt?: number;
  finishedAt?: number;
  message?: string;
};

const initial: CleanerScanState = { phase: 'idle', found: 0 };

export const cleanerScan = writable<CleanerScanState>(initial);

export function beginCleanerScan() {
  cleanerScan.set({ phase: 'running', found: 0, startedAt: Date.now() });
}

export function incCleanerFound(n: number) {
  cleanerScan.update((s) => ({ ...s, found: (s.found || 0) + Math.max(0, n) }));
}

export function setCleanerMessage(msg: string) {
  cleanerScan.update((s) => ({ ...s, message: msg }));
}

export function endCleanerScan(total?: number) {
  cleanerScan.update((s) => ({ phase: 'done', found: typeof total === 'number' ? total : (s.found||0), startedAt: s.startedAt, finishedAt: Date.now(), message: '' }));
}

export function resetCleanerScan() {
  cleanerScan.set(initial);
}

