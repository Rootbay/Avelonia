import { get } from 'svelte/store';
import { downloads } from './downloads';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { downloadDir, join } from '@tauri-apps/api/path';
import { candidateFileNames, normalizeExtension, sanitizeFileName } from './downloadPath';

export interface Download {
  id: number;
  name: string;
  description: string;
  size: string;
  fileType: string;
  category: string;
  downloadLink: string;
  eta: string;
  status: 'available' | 'pending' | 'downloading' | 'paused' | 'completed' | 'queued' | 'failed';
  progress: number;
  speed?: string;
  targetPath?: string;
}

const MAX_CONCURRENT_DOWNLOADS = 2;
const MAX_FILENAME_ATTEMPTS = 50;

let progressUnlisten: Promise<UnlistenFn> | null = null;

const lastSample = new Map<number, { bytes: number; time: number }>();
const activeDownloads = new Set<number>();
const pendingQueue: number[] = [];

const reservedPaths = new Set<string>();
const downloadPathReservations = new Map<number, string>();
const usedPaths = new Set<string>();
let usedPathsSeeded = false;

function seedUsedPaths() {
  if (usedPathsSeeded) return;
  for (const entry of get(downloads)) {
    if (entry.targetPath) {
      usedPaths.add(entry.targetPath);
    }
  }
  usedPathsSeeded = true;
}

function reservePath(id: number, path: string) {
  reservedPaths.add(path);
  downloadPathReservations.set(id, path);
  usedPaths.add(path);
}

function releasePath(id: number) {
  const reserved = downloadPathReservations.get(id);
  if (reserved) {
    reservedPaths.delete(reserved);
    downloadPathReservations.delete(id);
  }
}

function formatBytesPerSec(bps: number): string {
  if (!isFinite(bps) || bps <= 0) return '';
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s', 'TB/s'];
  let idx = 0;
  while (bps >= 1024 && idx < units.length - 1) {
    bps /= 1024;
    idx += 1;
  }
  return `${bps.toFixed(bps >= 100 ? 0 : 1)} ${units[idx]}`;
}

function updateDownloadById(id: number, mutator: (draft: Download) => void) {
  downloads.update((list) => {
    let changed = false;
    const next = list.map((item) => {
      if (item.id !== id) {
        return item;
      }
      changed = true;
      const draft = { ...item };
      mutator(draft);
      return draft;
    });
    return changed ? next : list;
  });
}

function getDownloadSnapshot(id: number): Download | undefined {
  return get(downloads).find((dl) => dl.id === id);
}

function removeFromQueue(id: number) {
  let index = pendingQueue.indexOf(id);
  while (index !== -1) {
    pendingQueue.splice(index, 1);
    index = pendingQueue.indexOf(id);
  }
}

function processQueue() {
  while (activeDownloads.size < MAX_CONCURRENT_DOWNLOADS && pendingQueue.length > 0) {
    const nextId = pendingQueue.shift();
    if (typeof nextId !== 'number') break;

    if (activeDownloads.has(nextId)) {
      continue;
    }

    const snapshot = getDownloadSnapshot(nextId);
    if (!snapshot || !snapshot.downloadLink) {
      updateDownloadById(nextId, (draft) => {
        draft.status = 'failed';
        draft.progress = 0;
        draft.speed = '';
        draft.eta = 'Failed';
        draft.targetPath = undefined;
      });
      continue;
    }

    activeDownloads.add(nextId);
    updateDownloadById(nextId, (draft) => {
      draft.status = 'pending';
      draft.progress = 0;
      draft.speed = '';
      draft.eta = 'Preparing.';
    });

    void performDownload({ ...snapshot });
  }
}

async function performDownload(download: Download): Promise<void> {
  try {
    const filePath = await getDownloadPath(download);
    if (!filePath) {
      throw new Error('Failed to resolve download path');
    }

    if (!activeDownloads.has(download.id)) {
      return;
    }

    releasePath(download.id);
    reservePath(download.id, filePath);
    updateDownloadById(download.id, (draft) => {
      draft.targetPath = filePath;
    });

    await invoke('download_file', {
      id: download.id,
      url: download.downloadLink,
      path: filePath,
    });
  } catch (error) {
    if (activeDownloads.has(download.id)) {
      console.error(`Failed to start download for ${download.name}:`, error);
      updateDownloadById(download.id, (draft) => {
        draft.status = 'failed';
        draft.progress = 0;
        draft.speed = '';
        draft.eta = 'Failed';
        draft.targetPath = undefined;
      });
    }
  } finally {
    finalizeDownload(download.id);
  }
}

function finalizeDownload(id: number) {
  releasePath(id);
  activeDownloads.delete(id);
  lastSample.delete(id);
  removeFromQueue(id);
  processQueue();
}

export function initDownloadListener() {
  if (progressUnlisten) return;

  progressUnlisten = listen('download-progress', (event) => {
    const { id, downloaded, total } = event.payload as {
      id: number;
      downloaded: number;
      total: number;
    };

    if (!activeDownloads.has(id)) {
      return;
    }

    const now = Date.now();
    const previous = lastSample.get(id);

    updateDownloadById(id, (draft) => {
      draft.progress = total === 0 ? -1 : Math.min(100, (downloaded / total) * 100);

      if (previous) {
        const deltaBytes = downloaded - previous.bytes;
        const deltaTime = (now - previous.time) / 1000;
        if (deltaTime > 0 && deltaBytes >= 0) {
          const bps = deltaBytes / deltaTime;
          draft.speed = formatBytesPerSec(bps);
          if (total > 0 && downloaded <= total && bps > 0) {
            const remaining = total - downloaded;
            const etaSec = Math.max(0, Math.round(remaining / bps));
            const mm = Math.floor(etaSec / 60)
              .toString()
              .padStart(2, '0');
            const ss = (etaSec % 60).toString().padStart(2, '0');
            draft.eta = `${mm}:${ss}`;
          } else if (total === 0) {
            draft.eta = 'Preparing.';
          }
        }
      } else if (total === 0) {
        draft.eta = 'Preparing.';
        draft.speed = '';
      }

      if (total > 0 && downloaded >= total) {
        draft.status = 'completed';
        draft.progress = 100;
        draft.speed = '';
        draft.eta = 'Done';
      } else {
        draft.status = 'downloading';
      }
    });

    if (total > 0 && downloaded >= total) {
      lastSample.delete(id);
    } else {
      lastSample.set(id, { bytes: downloaded, time: now });
    }
  });
}

export async function disposeDownloadListener() {
  if (!progressUnlisten) return;
  try {
    const unlisten = await progressUnlisten;
    unlisten();
  } catch {
    /* noop */
  } finally {
    progressUnlisten = null;
  }
}

export async function getDownloadPath(dl: Download): Promise<string | null> {
  try {
    seedUsedPaths();

    const downloadsPath = await downloadDir();
    if (!downloadsPath) {
      return null;
    }
    const baseName = sanitizeFileName(dl.name);
    const extension = normalizeExtension(dl.fileType);

    const snapshot = get(downloads);
    const takenByOthers = new Set<string>();
    for (const entry of snapshot) {
      if (entry.id !== dl.id && entry.targetPath) {
        takenByOthers.add(entry.targetPath);
      }
    }

    const existingPath = typeof dl.targetPath === 'string' ? dl.targetPath : null;

    if (
      existingPath &&
      !reservedPaths.has(existingPath) &&
      !takenByOthers.has(existingPath) &&
      !usedPaths.has(existingPath)
    ) {
      return existingPath;
    }

    let attempts = 0;
    for (const candidateName of candidateFileNames(baseName, extension)) {
      const fullPath = await join(downloadsPath, candidateName);
      if (
        !reservedPaths.has(fullPath) &&
        !takenByOthers.has(fullPath) &&
        !usedPaths.has(fullPath)
      ) {
        return fullPath;
      }
      attempts += 1;
      if (attempts >= MAX_FILENAME_ATTEMPTS) {
        break;
      }
    }

    const fallback = await join(downloadsPath, `${baseName}-${Date.now()}${extension}`);
    return fallback;
  } catch {
    return null;
  }
}

export function startDownload(id: number) {
  seedUsedPaths();

  const snapshot = getDownloadSnapshot(id);
  if (!snapshot || !snapshot.downloadLink) {
    return;
  }

  if (activeDownloads.has(id) || pendingQueue.includes(id)) {
    return;
  }

  if (
    snapshot.status === 'downloading' ||
    snapshot.status === 'pending' ||
    snapshot.status === 'queued'
  ) {
    return;
  }

  if (snapshot.targetPath) {
    usedPaths.add(snapshot.targetPath);
  }

  lastSample.delete(id);

  const willStartImmediately = activeDownloads.size < MAX_CONCURRENT_DOWNLOADS;

  updateDownloadById(id, (draft) => {
    draft.status = willStartImmediately ? 'pending' : 'queued';
    draft.progress = 0;
    draft.speed = '';
    draft.eta = 'Preparing.';
    draft.targetPath = undefined;
  });

  pendingQueue.push(id);
  processQueue();
}

export async function cancelDownload(id: number) {
  const wasActive = activeDownloads.has(id);
  removeFromQueue(id);

  if (wasActive) {
    try {
      await invoke('cancel_download', { id });
    } catch (error) {
      console.warn('cancel_download failed', error);
    }
  }

  releasePath(id);

  updateDownloadById(id, (draft) => {
    draft.status = 'available';
    draft.progress = 0;
    draft.speed = '';
    draft.eta = 'N/A';
    draft.targetPath = undefined;
  });

  lastSample.delete(id);
  finalizeDownload(id);
}
