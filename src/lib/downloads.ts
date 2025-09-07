import { writable } from 'svelte/store';
import type { Download } from './downloadManager';
import { programs } from './programs';

const DOWNLOADS_STORAGE_KEY = 'avelonia_downloads';

function loadDownloads(): Download[] {
  const initialDownloads = programs.map((program) => ({
    ...program,
    eta: 'N/A',
    status: 'available',
    progress: 0,
  }));

  if (typeof window !== 'undefined') {
    const storedDownloads = localStorage.getItem(DOWNLOADS_STORAGE_KEY);
    if (storedDownloads) {
      try {
        const parsed = JSON.parse(storedDownloads);
        if (Array.isArray(parsed)) {
          const storedMap = new Map(parsed.map((dl: Download) => [dl.id, dl]));

          return initialDownloads.map(initialDl => {
            const storedDl = storedMap.get(initialDl.id);
            if (storedDl) {
              let newStatus = storedDl.status;
              if (storedDl.status === 'downloading' || storedDl.status === 'pending') {
                newStatus = 'paused';
              } else if (storedDl.status === 'completed') {
                newStatus = 'available';
              }
              return {
                ...initialDl,
                ...storedDl,
                status: newStatus,
              };
            }
            return initialDl;
          });
        }
      } catch (error) {
        console.error("Error parsing downloads from localStorage", error);
        return initialDownloads;
      }
    }
  }

  return initialDownloads;
}

export const downloads = writable<Download[]>(loadDownloads());

downloads.subscribe((currentDownloads) => {
  if (typeof window !== 'undefined') {
    localStorage.setItem(DOWNLOADS_STORAGE_KEY, JSON.stringify(currentDownloads));
  }
});
