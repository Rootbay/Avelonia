import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { downloads } from './downloads';
import type { Download } from './downloadManager';

let timer: number | null = null;
let running = false;
const recentlyMissing = new Set<number>();

export async function verifyDownloadsOnce(): Promise<void> {
  if (running) return;
  running = true;
  try {
    const snapshot = get(downloads);
    const candidates = snapshot.filter((d) => d.status === 'completed' && !!d.targetPath);
    if (candidates.length === 0) return;

    const results = await Promise.all(
      candidates.map(async (d) => {
        try {
          const ok = await invoke<boolean>('path_exists', { path: d.targetPath as string });
          return { id: d.id, exists: ok };
        } catch {
          // If check fails, don't flip status; treat as unknown
          return { id: d.id, exists: true };
        }
      })
    );

    const missing = new Set(results.filter((r) => !r.exists).map((r) => r.id));
    if (missing.size === 0) return;
    // mark for downstream consumers (e.g., dashboard logs)
    for (const id of missing) recentlyMissing.add(id);

    downloads.update((list) =>
      list.map((d) => {
        if (!missing.has(d.id)) return d;
        const next: Download = { ...d } as Download;
        next.status = 'available';
        next.progress = 0;
        next.speed = '';
        next.eta = 'N/A';
        // Keep targetPath for reference? Clear to avoid confusion in UI
        next.targetPath = undefined;
        return next;
      })
    );
  } finally {
    running = false;
  }
}

export function startDownloadIntegrityWatch(intervalMs = 20000): void {
  stopDownloadIntegrityWatch();
  // Kick off immediately, then at interval
  void verifyDownloadsOnce();
  // In browsers, setInterval returns number
  timer = setInterval(() => void verifyDownloadsOnce(), intervalMs) as unknown as number;
}

export function stopDownloadIntegrityWatch(): void {
  if (timer !== null) {
    clearInterval(timer as unknown as number);
    timer = null;
  }
}

export function consumeRecentlyMissing(id: number): boolean {
  if (recentlyMissing.has(id)) {
    recentlyMissing.delete(id);
    return true;
  }
  return false;
}
