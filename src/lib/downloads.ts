import { writable } from 'svelte/store';
import type { Download } from './downloadManager';

const DOWNLOADS_STORAGE_KEY = 'avelonia_downloads';

function loadDownloads(): Download[] {
  // Start from persisted list only; no hardcoded defaults or test seeds
  if (typeof window === 'undefined') return [];
  const stored = localStorage.getItem(DOWNLOADS_STORAGE_KEY);
  if (!stored) return [];
  try {
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return [];
    // Normalize transient states back to available on load
    return parsed.map((d: Download) => {
      let status = d.status as Download['status'];
      if (status === 'downloading' || status === 'pending' || status === 'queued') {
        status = 'available';
      }
      return { ...d, status } as Download;
    });
  } catch (e) {
    console.error('Error parsing downloads from localStorage', e);
    return [];
  }
}

export const downloads = writable<Download[]>(loadDownloads());

downloads.subscribe((currentDownloads) => {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(DOWNLOADS_STORAGE_KEY, JSON.stringify(currentDownloads));
  } catch (error) {
    console.error('Failed to persist downloads state', error);
  }
});

export function nextDownloadId(list: Download[]): number {
  let max = 100;
  for (const d of list) max = Math.max(max, d.id);
  return max + 1;
}

export type NewDownloadEntry = {
  name: string;
  description?: string;
  size?: string;
  fileType?: string;
  category: string;
  tags?: string[];
  downloadLink: string;
};

export function addDownload(entry: NewDownloadEntry) {
  downloads.update((list) => {
    const id = nextDownloadId(list);
    const item: Download = {
      name: entry.name,
      description: entry.description ?? '',
      size: entry.size ?? 'N/A',
      fileType: entry.fileType ?? '',
      category: entry.category,
      tags: entry.tags && entry.tags.length ? entry.tags : [entry.category],
      downloadLink: entry.downloadLink,
      id,
      eta: 'N/A',
      status: 'available',
      progress: 0,
    };
    return [...list, item];
  });
}

export function removeDownloadsByIds(ids: number[]) {
  const set = new Set(ids);
  downloads.update((list) => list.filter((d) => !set.has(d.id)));
}
