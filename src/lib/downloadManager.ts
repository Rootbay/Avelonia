import { get } from 'svelte/store';
import { downloads, removeDownloadsByIds } from './downloads';
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
  hash?: string;
  installFlags?: string;
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
  hash?: string;
  installFlags?: string;
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
    | 'installed'
    | 'verifying';
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
let processingQueue = false;

const reservedPaths = new Set<string>();
const downloadPathReservations = new Map<number, string>();
const usedPaths = new Set<string>();
const autoInstallTried = new Set<number>();
const installQueue: number[] = [];
let installBusy = false;
const postCompleteTimers = new Map<number, number>();
let installPresenceTimer: number | null = null;
let downloadDirPromise: Promise<string> | null = null;

type VerifyInstallResult = {
  verified: boolean;
  matched?: {
    display_name?: string;
    display_version?: string;
  };
};

function logIgnoredError(context: string, error: unknown) {
  pushLog('WARN', `Ignored error (${context}): ${String(error)}`, 'Downloader');
}
async function appLog(level: LogLevel, message: string) {
  try {
    pushLog(level, message, 'Downloader');
  } catch {
    /* ignore logging errors */
  }
}

async function maybeAutoInstall(id: number) {
  try {
    const { downloader } = get(settings);
    if (!downloader.autoInstall) return;
    if (autoInstallTried.has(id)) return;
    installQueue.push(id);
    void processInstallQueue();
  } catch (error: unknown) {
    await appLog('ERROR', `Auto-install trigger failed for ID ${id}: ${String(error)}`);
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

      // Transition to verifying / Installing state in the UI
      updateDownloadById(id, (draft) => {
        draft.status = 'verifying';
        draft.eta = 'Installing...';
      });

      let installSuccess = false;

      if (downloader.installMode === 'normal') {
        await appLog(
          'INFO',
          `Launching installer (interactive) for ${snap.name}${downloader.elevate ? ' [elevated]' : ''}`
        );
        try {
          await invoke('launch_installer', { id, path, elevate: !!downloader.elevate });
          await appLog('SUCCESS', `Installer launched: ${snap.name}`);
          installSuccess = true;
        } catch (installerLaunchError: unknown) {
          await appLog(
            'ERROR',
            `Failed to launch installer for ${snap.name}: ${String(installerLaunchError)}`
          );
        } finally {
          autoInstallTried.add(id);
        }
      } else {
        await appLog(
          'INFO',
          `Attempting silent install of ${snap.name} (${ext.toUpperCase()})${downloader.elevate ? ' [elevated]' : ''}`
        );
        try {
          await invoke('silent_install', {
            id,
            path,
            elevate: !!downloader.elevate,
            customFlags: snap.installFlags,
          });
          await appLog('SUCCESS', `Silent install completed: ${snap.name}`);
          autoInstallTried.add(id);
          installSuccess = true;
        } catch (silentInstallError: unknown) {
          await appLog(
            'WARN',
            `Silent install failed for ${snap.name}: ${String(silentInstallError)}`
          );
          autoInstallTried.add(id);
          if (downloader.fallbackOpen) {
            try {
              await appLog('INFO', `Opening installer normally for ${snap.name}`);
              await openPath(path);
              installSuccess = true;
            } catch (fallbackError: unknown) {
              await appLog(
                'ERROR',
                `Failed to open installer for ${snap.name}: ${String(fallbackError)}`
              );
            }
          }
        }
      }

      let verified = false;
      try {
        if (installSuccess && downloader.verifyInstall) {
          const result = (await invoke('verify_install', {
            displayNameHint: snap.name,
            timeoutMs: 30000,
          })) as VerifyInstallResult;
          if (result?.verified) {
            verified = true;
            const dn = result.matched?.display_name ?? snap.name;
            const dv = result.matched?.display_version ?? '';
            await appLog('SUCCESS', `Installed (verified): ${dn}${dv ? ' ' + dv : ''}`);
            updateDownloadById(id, (draft) => {
              draft.status = 'installed';
              draft.targetPath = undefined;
              draft.eta = 'Done';
            });
          } else {
            await appLog(
              'WARN',
              `Install not verified for ${snap.name} (no uninstall entry detected)`
            );
          }
        }
      } catch (error: unknown) {
        await appLog('ERROR', `Verification process failed for ${snap.name}: ${String(error)}`);
      }

      if (!verified) {
        updateDownloadById(id, (draft) => {
          draft.status = installSuccess ? 'completed' : 'failed';
          draft.eta = installSuccess ? 'Done' : 'Failed';
        });
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
  } catch (error: unknown) {
    await appLog('WARN', `Verification check failed: ${String(error)}`);
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
  } catch (error: unknown) {
    void appLog('ERROR', `Post-complete check failed: ${String(error)}`);
  }
}

function syncUsedPaths(list: Download[]) {
  usedPaths.clear();
  for (const entry of list) {
    if (entry.targetPath) {
      usedPaths.add(entry.targetPath);
    }
  }
  for (const reserved of reservedPaths) {
    usedPaths.add(reserved);
  }
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
    usedPaths.delete(reserved);
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
    const index = list.findIndex((item) => item.id === id);
    if (index === -1) {
      return list;
    }
    const draft = { ...list[index] };
    mutator(draft);
    const next = [...list];
    next[index] = draft;
    return next;
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
  if (processingQueue) return;
  processingQueue = true;
  try {
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
          draft.eta = 'Invalid Link';
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
  } finally {
    processingQueue = false;
  }
}

async function performDownload(download: Download): Promise<void> {
  try {
    const filePath = await getDownloadPath(download);
    if (!filePath) {
      throw new Error('Could not resolve a valid storage path');
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

    const currentSnap = getDownloadSnapshot(download.id);
    if (currentSnap && currentSnap.hash && currentSnap.status !== 'available') {
      updateDownloadById(download.id, (draft) => {
        draft.eta = 'Verifying...';
        draft.status = 'verifying';
      });
      const isValid = await invoke<boolean>('verify_hash', {
        path: filePath,
        expectedHash: currentSnap.hash,
      });
      if (!isValid) {
        throw new Error('Integrity check failed: File hash mismatch');
      }
      updateDownloadById(download.id, (draft) => {
        draft.status = 'completed';
        draft.progress = 100;
        draft.speed = '';
        draft.eta = 'Done';
      });
      await appLog(
        'SUCCESS',
        'Download completed: ' +
          currentSnap.name +
          (currentSnap.targetPath ? ' -> ' + currentSnap.targetPath : '')
      );
      void maybeAutoInstall(download.id);
      schedulePostCompleteCheck(download.id, 5000);
    }
  } catch (error: unknown) {
    if (activeDownloads.has(download.id)) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      await appLog('ERROR', `Download failed for ${download.name}: ${errorMessage}`);

      updateDownloadById(download.id, (draft) => {
        if (draft.status !== 'available') {
          draft.status = 'failed';
          draft.progress = 0;
          draft.speed = '';
          draft.eta = 'Failed';
          draft.targetPath = undefined;
        }
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

  void (async () => {
    try {
      const dir = await getBaseDownloadDir();
      const currentDownloads = get(downloads);
      const activePaths = currentDownloads
        .filter(
          (d) =>
            d.status === 'downloading' ||
            d.status === 'pending' ||
            d.status === 'queued' ||
            d.status === 'verifying'
        )
        .map((d) => (d.targetPath ? d.targetPath + '.part' : null))
        .filter((p): p is string => !!p);

      const count = await invoke<number>('cleanup_orphaned_downloads', {
        downloadDir: dir,
        activePaths,
      });
      if (count > 0) {
        await appLog('INFO', `Cleaned up ${count} orphaned partial download files.`);
      }
    } catch (error) {
      void appLog('WARN', `Orphaned cleanup failed: ${String(error)}`);
    }
  })();

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
    const snapshot = getDownloadSnapshot(id);
    if (!snapshot) return;

    const prevStatus = snapshot.status;
    if (prevStatus === 'available' || prevStatus === 'completed' || prevStatus === 'failed') {
      return;
    }

    const completed = total > 0 && downloaded >= total;
    const awaitingVerify = completed && !!snapshot.hash;
    const newStatus = completed ? (awaitingVerify ? 'verifying' : 'completed') : 'downloading';

    let updatedSpeed = false;
    let speedStr = '';
    let etaStr = '';

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
          speedStr = formatBytesPerSec(bps);
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
          updatedSpeed = true;
          lastUiEmit.set(id, now);
        }
      }
    }

    const shouldUpdateUi = !previous || prevStatus !== newStatus || updatedSpeed || completed;

    if (shouldUpdateUi) {
      updateDownloadById(id, (draft) => {
        draft.progress = total === 0 ? -1 : Math.min(100, (downloaded / total) * 100);
        draft.status = newStatus;
        if (newStatus === 'completed') {
          draft.progress = 100;
          draft.speed = '';
          draft.eta = 'Done';
        } else if (newStatus === 'verifying') {
          draft.progress = 100;
          draft.speed = '';
          draft.eta = 'Verifying...';
        } else if (updatedSpeed) {
          draft.speed = speedStr || draft.speed;
          draft.eta = etaStr || draft.eta;
        } else if (!previous && total === 0) {
          draft.eta = '—';
          draft.speed = '';
        }
      });
    }

    if (completed) {
      lastSample.delete(id);
      avgSpeedBps.delete(id);
      lastUiEmit.delete(id);
      if (!awaitingVerify) {
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
      }
      return;
    }

    lastSample.set(id, { bytes: downloaded, time: now });
  });
}

export async function disposeDownloadListener() {
  if (!progressUnlisten) return;
  try {
    const unlisten = await progressUnlisten;
    unlisten();
  } catch (error: unknown) {
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
          } catch (error: unknown) {
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
          } catch (error: unknown) {
            logIgnoredError('installPresenceWatch path_exists', error);
          }
        }
      }
    } catch (error: unknown) {
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

async function getBaseDownloadDir(): Promise<string> {
  if (!downloadDirPromise) {
    downloadDirPromise = (async () => {
      return await downloadDir();
    })();
  }
  return downloadDirPromise;
}

export async function getDownloadPath(dl: Download): Promise<string | null> {
  try {
    const downloadsPath = await getBaseDownloadDir();
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
  } catch (error: unknown) {
    logIgnoredError('getDownloadPath', error);
    return null;
  }
}

export function startDownload(id: number) {
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
    snapshot.status === 'queued' ||
    snapshot.status === 'verifying'
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
    } catch (error: unknown) {
      void appLog('WARN', `cancel_download failed: ${String(error)}`);
    }
  }

  updateDownloadById(id, (draft) => {
    draft.status = 'available';
    draft.progress = 0;
    draft.speed = '';
    draft.eta = 'N/A';
    draft.targetPath = undefined;
  });

  lastSample.delete(id);

  if (!wasActive) {
    finalizeDownload(id);
  }

  void (async () => {
    const snap = getDownloadSnapshot(id);
    if (snap) await appLog('WARN', 'Canceled download: ' + snap.name);
  })();
}

export function cancelAndRemoveDownloads(ids: number[]) {
  const idSet = new Set(ids);
  const list = get(downloads);
  list
    .filter((d) => idSet.has(d.id))
    .filter((d) => ['downloading', 'pending', 'queued', 'verifying'].includes(d.status))
    .forEach((d) => cancelDownload(d.id));
  removeDownloadsByIds(ids);
}

export function setDownloadRelease(id: number, releaseLabel: string) {
  const snap = getDownloadSnapshot(id);
  if (snap && ['downloading', 'pending', 'queued', 'verifying'].includes(snap.status)) {
    void cancelDownload(id);
  }
  updateDownloadById(id, (draft) => {
    const releases = draft.releases;
    if (!releases || releases.length === 0) return;
    const next = releases.find((release) => release.label === releaseLabel);
    if (!next) return;
    draft.selectedReleaseLabel = releaseLabel;
    draft.downloadLink = next.downloadLink;
    draft.size = next.size ?? 'N/A';
    draft.fileType = next.fileType ?? '';
    draft.hash = next.hash ?? undefined;
    draft.installFlags = next.installFlags ?? undefined;
    draft.status = 'available';
    draft.progress = 0;
    draft.speed = '';
    draft.eta = 'N/A';
    draft.targetPath = undefined;
  });
}

downloads.subscribe((list) => {
  syncUsedPaths(list);
});
