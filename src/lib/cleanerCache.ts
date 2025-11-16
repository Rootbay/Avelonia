export type FileEntry = { path: string; size?: number };
export type DuplicateGroup = { hash: string; size: number; files: string[] };

export type CleanerCache = {
  tempFiles: FileEntry[];
  largeFiles: FileEntry[];
  duplicateFiles: FileEntry[];
  emptyFolders: FileEntry[];
  brokenShortcuts: FileEntry[];
  dupGroups: DuplicateGroup[];
  timestamp: number;
};

const STORAGE_KEY = 'avelonia_cleaner_cache_v1';

export function loadCleanerCache(): CleanerCache | null {
  if (typeof window === 'undefined') return null;
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return null;
    const ensureArr = (v: unknown) => (Array.isArray(v) ? v : []);
    return {
      tempFiles: ensureArr(parsed.tempFiles),
      largeFiles: ensureArr(parsed.largeFiles),
      duplicateFiles: ensureArr(parsed.duplicateFiles),
      emptyFolders: ensureArr(parsed.emptyFolders),
      brokenShortcuts: ensureArr(parsed.brokenShortcuts),
      dupGroups: ensureArr(parsed.dupGroups),
      timestamp: Number(parsed.timestamp) || Date.now(),
    } as CleanerCache;
  } catch (error) {
    console.warn('[CleanerCache] unable to load cache', error);
    return null;
  }
}

export function saveCleanerCache(cache: CleanerCache): void {
  if (typeof window === 'undefined') return;
  try {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(cache));
  } catch (error) {
    console.warn('[CleanerCache] unable to persist cache', error);
  }
}
