import { writable } from 'svelte/store';
import type { Download } from './downloadManager';
import { programs } from './programs';

const DOWNLOADS_STORAGE_KEY = 'avelonia_downloads';

function loadDownloads(): Download[] {
  if (typeof window !== 'undefined') {
    const storedDownloads = localStorage.getItem(DOWNLOADS_STORAGE_KEY);
    if (storedDownloads) {
      const parsed = JSON.parse(storedDownloads);
      return parsed.map((dl: Download) => ({
        ...dl,
        intervalId: null,
        status: (dl.status === 'downloading' || dl.status === 'pending') ? 'paused' : dl.status,
      }));
    }
  }
  return programs.map((program) => ({
    ...program,
    eta: 'N/A',
    status: 'available',
    progress: 0,
    intervalId: null,
    downloadStartTime: 0,
    pausedTime: 0,
    retryCount: 0,
  }));
}

export const downloads = writable<Download[]>(loadDownloads());

downloads.subscribe((currentDownloads) => {
  if (typeof window !== 'undefined') {
    const serializableDownloads = currentDownloads.map(dl => {
      const { intervalId, ...rest } = dl;
      return rest;
    });
    localStorage.setItem(DOWNLOADS_STORAGE_KEY, JSON.stringify(serializableDownloads));
  }
});