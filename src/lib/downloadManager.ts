import { get } from 'svelte/store';
import { downloads } from './downloads';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { downloadDir, join } from '@tauri-apps/api/path';
import { candidateFileNames, normalizeExtension, sanitizeFileName } from './downloadPath';
import { pushLog, type LogLevel } from '$lib/logStore';

export interface Download {
  id: number;
  name: string;
  description: string;
  size: string;
  fileType: string;
  category: string;
  tags?: string[];
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
const avgSpeedBps = new Map<number, number>();
const lastUiEmit = new Map<number, number>();
const UI_UPDATE_MS = 700;
const EMA_ALPHA = 0.25;
const activeDownloads = new Set<number>();
const pendingQueue: number[] = [];

const reservedPaths = new Set<string>();
const downloadPathReservations = new Map<number, string>();
const usedPaths = new Set<string>();
let usedPathsSeeded = false;
async function appLog(level: LogLevel, message: string) {
  try {
    pushLog(level, message, 'Downloader');
  } catch {
    // ignore logging errors
  }
}

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
      void (async () => {
        try {
          await appLog('ERROR', 'Download failed to start: ' + download.name);
        } catch {}
      })();
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
  avgSpeedBps.delete(id);
  lastUiEmit.delete(id);
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

    // Always update progress and status promptly
    updateDownloadById(id, (draft) => {
      draft.progress = total === 0 ? -1 : Math.min(100, (downloaded / total) * 100);
      draft.status = total > 0 && downloaded >= total ? 'completed' : 'downloading';
      if (draft.status === 'completed') {
        draft.progress = 100;
        draft.speed = '';
        draft.eta = 'Done';
      }
    });

    if (total > 0 && downloaded >= total) {
      lastSample.delete(id);
      avgSpeedBps.delete(id);
      lastUiEmit.delete(id);
      void (async () => {
        const snap = getDownloadSnapshot(id);
        if (snap) {
          await appLog(
            'SUCCESS',
            'Download completed: ' + snap.name + (snap.targetPath ? ' -> ' + snap.targetPath : '')
          );
        }
      })();
      return;
    }

    // Compute smoothed speed (EMA) and throttle UI updates
    if (previous) {
      const deltaBytes = downloaded - previous.bytes;
      const deltaTime = (now - previous.time) / 1000;
      if (deltaTime > 0 && deltaBytes >= 0) {
        const instBps = deltaBytes / deltaTime;
        const prevAvg = avgSpeedBps.get(id) ?? instBps;
        const nextAvg = EMA_ALPHA * instBps + (1 - EMA_ALPHA) * prevAvg;
        avgSpeedBps.set(id, nextAvg);

        const lastEmit = lastUiEmit.get(id) ?? 0;
        const shouldEmit = now - lastEmit >= UI_UPDATE_MS;
        if (shouldEmit) {
          const bps = nextAvg;
          let speedStr = formatBytesPerSec(bps);
          let etaStr = '';
          if (total > 0 && bps > 1) {
            const remaining = Math.max(0, total - downloaded);
            const etaSec = Math.max(0, Math.round(remaining / bps));
            const hrs = Math.floor(etaSec / 3600);
            const mins = Math.floor((etaSec % 3600) / 60);
            const secs = etaSec % 60;
            etaStr = hrs > 0
              ? `${hrs}:${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
              : `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
          } else if (total === 0) {
            etaStr = '—';
          }

          updateDownloadById(id, (draft) => {
            if (speedStr) draft.speed = speedStr;
            if (etaStr) draft.eta = etaStr;
          });
          lastUiEmit.set(id, now);
        }
      }
    } else {
      // First sample: show placeholder ETA
      if (total === 0) {
        updateDownloadById(id, (draft) => {
          draft.eta = '—';
          draft.speed = '';
        });
      }
    }

    lastSample.set(id, { bytes: downloaded, time: now });
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

    // Prefer an already assigned path for this item when available
    if (existingPath && !reservedPaths.has(existingPath) && !takenByOthers.has(existingPath)) {
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

  // Log start/queue action for dashboard
  void (async () => {
    const snap = getDownloadSnapshot(id);
    if (!snap) return;
    if (willStartImmediately) {
      await appLog('INFO', 'Starting download: ' + snap.name);
    } else {
      await appLog('INFO', 'Queued download: ' + snap.name);
    }
  })();

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
  void (async () => { const snap = getDownloadSnapshot(id); if (snap) await appLog('WARN', 'Canceled download: ' + snap.name); })();
}










