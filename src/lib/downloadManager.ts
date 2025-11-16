import { get } from 'svelte/store';
import { downloads } from './downloads';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { downloadDir, join } from '@tauri-apps/api/path';
import { candidateFileNames, normalizeExtension, sanitizeFileName } from './downloadPath';
import { pushLog, type LogLevel } from '$lib/logStore';
import { settings } from '$lib/settings';
import { openPath } from '@tauri-apps/plugin-opener';

export type DownloadRelease = {
  label: string;
  downloadLink: string;
  size?: string;
  fileType?: string;
};

export interface Download {
  id: number;
  name: string;
  description: string;
  size: string;
  fileType: string;
  category: string;
  tags?: string[];
  downloadLink: string;
  releases?: DownloadRelease[];
  selectedReleaseLabel?: string;
  eta: string;
  status:
    | 'available'
    | 'pending'
    | 'downloading'
    | 'paused'
    | 'completed'
    | 'queued'
    | 'failed'
    | 'installed';
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
const autoInstallTried = new Set<number>();
const installQueue: number[] = [];
let installBusy = false;
const postCompleteTimers = new Map<number, number>();
let installPresenceTimer: number | null = null;

type VerifyInstallResult = {
  verified: boolean;
  matched?: {
    display_name?: string;
    display_version?: string;
  };
};

function logIgnoredError(context: string, error: unknown) {
  console.debug(`[Downloader:${context}] Ignored error:`, error);
}
async function appLog(level: LogLevel, message: string) {
  try {
    pushLog(level, message, 'Downloader');
  } catch (error) {
    logIgnoredError('appLog', error);
  }
}

async function maybeAutoInstall(id: number) {
  try {
    const { downloader } = get(settings);
    if (!downloader.autoInstall) return;
    if (autoInstallTried.has(id)) return;
    installQueue.push(id);
    void processInstallQueue();
  } catch (error) {
    logIgnoredError('maybeAutoInstall', error);
  }
}

async function processInstallQueue() {
  if (installBusy) return;
  installBusy = true;
  try {
    while (installQueue.length > 0) {
      const id = installQueue.shift() as number;
      if (autoInstallTried.has(id)) continue;
      const { downloader } = get(settings);
      const snap = getDownloadSnapshot(id);
      if (!snap || snap.status !== 'completed' || !snap.targetPath) {
        continue;
      }
      const path = snap.targetPath;
      const ext = (path.split('.').pop() || '').toLowerCase();
      if (ext !== 'exe' && ext !== 'msi') {
        autoInstallTried.add(id);
        continue;
      }
      if (downloader.installMode === 'normal') {
        await appLog(
          'INFO',
          `Launching installer (interactive) for ${snap.name}${downloader.elevate ? ' [elevated]' : ''}`
        );
        try {
          await invoke('launch_installer', { path, elevate: !!downloader.elevate });
          await appLog('SUCCESS', `Installer launched: ${snap.name}`);
        } catch (e) {
          await appLog('ERROR', `Failed to launch installer for ${snap.name}: ${String(e)}`);
        } finally {
          autoInstallTried.add(id);
        }
      } else {
        await appLog(
          'INFO',
          `Attempting silent install of ${snap.name} (${ext.toUpperCase()})${downloader.elevate ? ' [elevated]' : ''}`
        );
        try {
          await invoke('silent_install', { path, elevate: !!downloader.elevate });
          await appLog('SUCCESS', `Silent install completed: ${snap.name}`);
          autoInstallTried.add(id);
        } catch (e) {
          await appLog('WARN', `Silent install failed for ${snap.name}: ${String(e)}`);
          autoInstallTried.add(id);
          if (downloader.fallbackOpen) {
            try {
              await appLog('INFO', `Opening installer normally for ${snap.name}`);
              await openPath(path);
            } catch (e2) {
              await appLog('ERROR', `Failed to open installer for ${snap.name}: ${String(e2)}`);
            }
          }
        }
      }

      try {
        if (downloader.verifyInstall) {
          const result = (await invoke('verify_install', {
            displayNameHint: snap.name,
            timeoutMs: 30000,
          })) as VerifyInstallResult;
          if (result?.verified) {
            const dn = result.matched?.display_name ?? snap.name;
            const dv = result.matched?.display_version ?? '';
            await appLog('SUCCESS', `Installed (verified): ${dn}${dv ? ' ' + dv : ''}`);
            updateDownloadById(id, (draft) => {
              draft.status = 'installed';
              draft.targetPath = undefined;
            });
          } else {
            await appLog(
              'WARN',
              `Install not verified for ${snap.name} (no uninstall entry detected)`
            );
          }
        }
      } catch (error) {
        logIgnoredError('processInstallQueue verify', error);
      }
    }
  } finally {
    installBusy = false;
  }
}

function isLikelyInstaller(dl: Download): boolean {
  const ext = (dl.fileType || '').toLowerCase();
  return ext === 'exe' || ext === 'msi';
}

async function checkAndMarkInstalled(id: number) {
  try {
    const s = get(settings);
    const dl = getDownloadSnapshot(id);
    if (!dl) return;
    if (!isLikelyInstaller(dl)) return;
    if (s.downloader.verifyInstall) {
      const ok = (await invoke('is_installed', { displayNameHint: dl.name })) as boolean;
      if (ok) {
        updateDownloadById(id, (draft) => {
          draft.status = 'installed';
          draft.targetPath = undefined;
        });
        await appLog('SUCCESS', `Installed (verified): ${dl.name}`);
      }
    }
  } catch (error) {
    logIgnoredError('checkAndMarkInstalled', error);
  }
}

function schedulePostCompleteCheck(id: number, delayMs = 5000) {
  try {
    if (postCompleteTimers.has(id)) return;
    const timer = setTimeout(() => {
      postCompleteTimers.delete(id);
      void checkAndMarkInstalled(id);
    }, delayMs) as unknown as number;
    postCompleteTimers.set(id, timer);
  } catch (error) {
    logIgnoredError('schedulePostCompleteCheck', error);
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
        } catch (innerError) {
          logIgnoredError('performDownload appLog', innerError);
        }
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
          void maybeAutoInstall(id);
          schedulePostCompleteCheck(id, 5000);
        }
      })();
      return;
    }

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
          const speedStr = formatBytesPerSec(bps);
          let etaStr = '';
          if (total > 0 && bps > 1) {
            const remaining = Math.max(0, total - downloaded);
            const etaSec = Math.max(0, Math.round(remaining / bps));
            const hrs = Math.floor(etaSec / 3600);
            const mins = Math.floor((etaSec % 3600) / 60);
            const secs = etaSec % 60;
            etaStr =
              hrs > 0
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
  } catch (error) {
    logIgnoredError('disposeDownloadListener', error);
  } finally {
    progressUnlisten = null;
  }
}

export function startInstallPresenceWatch(intervalMs = 20000) {
  stopInstallPresenceWatch();
  installPresenceTimer = setInterval(async () => {
    try {
      const s = get(settings);
      if (!s.downloader.verifyInstall) return;
      const list = get(downloads);
      for (const d of list) {
        if (d.status === 'installed') {
          try {
            const ok = (await invoke('is_installed', { displayNameHint: d.name })) as boolean;
            if (!ok) {
              updateDownloadById(d.id, (draft) => {
                draft.status = 'available';
                draft.progress = 0;
                draft.speed = '';
                draft.eta = 'N/A';
              });
              await appLog('INFO', `Uninstalled detected: ${d.name}`);
            }
          } catch (error) {
            logIgnoredError('installPresenceWatch is_installed', error);
          }
        } else if (d.status === 'completed' && isLikelyInstaller(d)) {
          try {
            const p = d.targetPath;
            let exists = false;
            if (typeof p === 'string' && p) {
              exists = !!(await invoke('path_exists', { path: p }));
            }
            if (!exists) {
              updateDownloadById(d.id, (draft) => {
                draft.status = 'available';
                draft.progress = 0;
                draft.speed = '';
                draft.eta = 'N/A';
                draft.targetPath = undefined;
              });
              await appLog('INFO', `Installer file missing; reset to available: ${d.name}`);
            }
          } catch (error) {
            logIgnoredError('installPresenceWatch path_exists', error);
          }
        }
      }
    } catch (error) {
      logIgnoredError('installPresenceWatch', error);
    }
  }, intervalMs) as unknown as number;
}

export function stopInstallPresenceWatch() {
  if (installPresenceTimer !== null) {
    clearInterval(installPresenceTimer as unknown as number);
    installPresenceTimer = null;
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
  } catch (error) {
    logIgnoredError('getDownloadPath', error);
    return null;
  }
}

export function startDownload(id: number) {
  seedUsedPaths();

  const snapshot = getDownloadSnapshot(id);
  if (!snapshot || !snapshot.downloadLink) {
    return;
  }

  autoInstallTried.delete(id);

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
  void (async () => {
    const snap = getDownloadSnapshot(id);
    if (snap) await appLog('WARN', 'Canceled download: ' + snap.name);
  })();
}

export function setDownloadRelease(id: number, releaseLabel: string) {
  updateDownloadById(id, (draft) => {
    const releases = draft.releases;
    if (!releases || releases.length === 0) return;
    const next = releases.find((release) => release.label === releaseLabel);
    if (!next) return;
    draft.selectedReleaseLabel = releaseLabel;
    draft.downloadLink = next.downloadLink;
    if (next.size) {
      draft.size = next.size;
    }
    if (next.fileType) {
      draft.fileType = next.fileType;
    }
  });
}
