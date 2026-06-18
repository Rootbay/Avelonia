export type CleanerPhase = 'idle' | 'running' | 'done';

export type FileEntry = { path: string; size?: number };
export type DuplicateGroup = { hash: string; size: number; files: string[] };

export type CleanerScanState = {
  phase: CleanerPhase;
  readonly found: number;
  startedAt?: number;
  finishedAt?: number;
  message?: string;
  
  tempFiles: FileEntry[];
  largeFiles: FileEntry[];
  duplicateFiles: FileEntry[];
  emptyFolders: FileEntry[];
  brokenShortcuts: FileEntry[];
  dupGroups: DuplicateGroup[];
  selectedPaths: Set<string>;
};

const initial: CleanerScanState = {
  phase: 'idle',
  get found() {
    return (
      (this.tempFiles?.length || 0) +
      (this.largeFiles?.length || 0) +
      (this.duplicateFiles?.length || 0) +
      (this.emptyFolders?.length || 0) +
      (this.brokenShortcuts?.length || 0)
    );
  },
  tempFiles: [],
  largeFiles: [],
  duplicateFiles: [],
  emptyFolders: [],
  brokenShortcuts: [],
  dupGroups: [],
  selectedPaths: new Set<string>(),
};

export const cleanerScan = $state<CleanerScanState>(initial);

export function beginCleanerScan() {
  cleanerScan.phase = 'running';
  cleanerScan.startedAt = Date.now();
  cleanerScan.message = '';
  
  cleanerScan.tempFiles = [];
  cleanerScan.largeFiles = [];
  cleanerScan.duplicateFiles = [];
  cleanerScan.emptyFolders = [];
  cleanerScan.brokenShortcuts = [];
  cleanerScan.dupGroups = [];
  cleanerScan.selectedPaths = new Set<string>();
}

export function setCleanerMessage(msg: string) {
  cleanerScan.message = msg;
}

export function endCleanerScan() {
  cleanerScan.phase = 'done';
  cleanerScan.finishedAt = Date.now();
  cleanerScan.message = '';
}

export function resetCleanerScan() {
  cleanerScan.phase = 'idle';
  cleanerScan.startedAt = undefined;
  cleanerScan.finishedAt = undefined;
  cleanerScan.message = undefined;
  
  cleanerScan.tempFiles = [];
  cleanerScan.largeFiles = [];
  cleanerScan.duplicateFiles = [];
  cleanerScan.emptyFolders = [];
  cleanerScan.brokenShortcuts = [];
  cleanerScan.dupGroups = [];
  cleanerScan.selectedPaths = new Set<string>();
}

export function clearAllScannedItems() {
  cleanerScan.tempFiles = [];
  cleanerScan.largeFiles = [];
  cleanerScan.duplicateFiles = [];
  cleanerScan.emptyFolders = [];
  cleanerScan.brokenShortcuts = [];
  cleanerScan.dupGroups = [];
  cleanerScan.selectedPaths = new Set<string>();
}
