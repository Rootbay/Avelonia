import { writable } from 'svelte/store';
import type { Download } from './downloadManager';

const DOWNLOADS_STORAGE_KEY = 'avelonia_downloads';

function loadDownloads(): Download[] {
  // Start from persisted list only; no hardcoded default entries
  if (typeof window !== 'undefined') {
    const storedDownloads = localStorage.getItem(DOWNLOADS_STORAGE_KEY);
    if (storedDownloads) {
      try {
        const parsed = JSON.parse(storedDownloads);
        if (Array.isArray(parsed)) {
          return parsed.map((storedDl: Download) => {
            let newStatus = storedDl.status as Download['status'];
            if (
              storedDl.status === 'downloading' ||
              storedDl.status === 'pending' ||
              storedDl.status === 'queued'
            ) {
              newStatus = 'available';
            }
            return { ...storedDl, status: newStatus } as Download;
          });
        }
      } catch (error) {
        console.error('Error parsing downloads from localStorage', error);
      }
    }
  }
  return [];
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
