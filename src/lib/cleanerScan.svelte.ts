export type CleanerPhase = 'idle' | 'running' | 'done';

export type CleanerScanState = {
  phase: CleanerPhase;
  found: number;
  startedAt?: number;
  finishedAt?: number;
  message?: string;
};

const initial: CleanerScanState = { phase: 'idle', found: 0 };

export const cleanerScan = $state<CleanerScanState>(initial);

export function beginCleanerScan() {
  cleanerScan.phase = 'running';
  cleanerScan.found = 0;
  cleanerScan.startedAt = Date.now();
  cleanerScan.message = '';
}

export function incCleanerFound(n: number) {
  cleanerScan.found += Math.max(0, n);
}

export function setCleanerMessage(msg: string) {
  cleanerScan.message = msg;
}

export function endCleanerScan(total?: number) {
  cleanerScan.phase = 'done';
  if (typeof total === 'number') {
    cleanerScan.found = total;
  }
  cleanerScan.finishedAt = Date.now();
  cleanerScan.message = '';
}

export function resetCleanerScan() {
  cleanerScan.phase = 'idle';
  cleanerScan.found = 0;
  cleanerScan.startedAt = undefined;
  cleanerScan.finishedAt = undefined;
  cleanerScan.message = undefined;
}
