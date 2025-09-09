import { downloads } from './downloads';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { downloadDir, join } from '@tauri-apps/api/path';

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
  // Derived/runtime fields (not required by initial seed)
  speed?: string; // e.g. "1.2 MB/s"
  targetPath?: string; // absolute path where file is saved
}

let progressUnlisten: Promise<UnlistenFn> | null = null;

// Track last progress sample per download to compute speed/ETA
const lastSample = new Map<number, { bytes: number; time: number }>();

function formatBytesPerSec(bps: number): string {
  if (!isFinite(bps) || bps <= 0) return '';
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s', 'TB/s'];
  let idx = 0;
  while (bps >= 1024 && idx < units.length - 1) {
    bps /= 1024;
    idx++;
  }
  return `${bps.toFixed(bps >= 100 ? 0 : 1)} ${units[idx]}`;
}

export function initDownloadListener() {
  if (!progressUnlisten) {
    progressUnlisten = listen('download-progress', (event) => {
      const { id, downloaded, total } = event.payload as { id: number; downloaded: number; total: number };
      downloads.update((dlList) => {
        const dl = dlList.find((d) => d.id === id);
        if (dl && (dl.status === 'downloading' || dl.status === 'pending')) {
          const now = Date.now();
          // progress
          dl.progress = total === 0 ? -1 : (downloaded / total) * 100;

          // speed/eta
          const prev = lastSample.get(id);
          if (prev) {
            const deltaBytes = downloaded - prev.bytes;
            const deltaTime = (now - prev.time) / 1000; // seconds
            if (deltaTime > 0 && deltaBytes >= 0) {
              const bps = deltaBytes / deltaTime;
              dl.speed = formatBytesPerSec(bps);
              if (total > 0 && downloaded <= total && bps > 0) {
                const remain = total - downloaded;
                const etaSec = Math.max(0, Math.round(remain / bps));
                const mm = Math.floor(etaSec / 60).toString().padStart(2, '0');
                const ss = (etaSec % 60).toString().padStart(2, '0');
                dl.eta = `${mm}:${ss}`;
              } else if (total === 0) {
                dl.eta = 'Preparing…';
              }
            }
          }
          lastSample.set(id, { bytes: downloaded, time: now });

          dl.status = 'downloading';
          if (total > 0 && downloaded === total) {
            dl.status = 'completed';
            dl.progress = 100;
            dl.speed = '';
            dl.eta = 'Done';
            lastSample.delete(id);
          }
        }
        return dlList;
      });
    });
  }
}

export async function disposeDownloadListener() {
  if (progressUnlisten) {
    try {
      const unlisten = await progressUnlisten;
      unlisten();
    } catch {
      // ignore
    } finally {
      progressUnlisten = null;
    }
  }
}

// Compute a stable on-disk path for a download. Returns null on error.
export async function getDownloadPath(dl: Download): Promise<string | null> {
  try {
    if (dl.targetPath) return dl.targetPath;
    const downloadsPath = await downloadDir();
    const fileName = `${(dl.name || '').replace(/[^a-zA-Z0-9._-]/g, '_')}.${dl.fileType}`;
    return await join(downloadsPath, fileName);
  } catch {
    return null;
  }
}

export function startDownload(id: number) {
  
  let dlToDownload: Download | undefined;

  downloads.update((dlList) => {
    const dl = dlList.find((d) => d.id === id);
    if (dl && dl.downloadLink && dl.status !== 'downloading' && dl.status !== 'pending') {
      dl.status = 'pending';
      dl.progress = 0;
      dl.speed = '';
      dl.eta = 'Preparing…';
      dlToDownload = dl;
    }
    return dlList;
  });

  if (dlToDownload) {
    
    const dl = dlToDownload;
    (async () => {
      try {
        const filePath = await getDownloadPath(dl);
        if (!filePath) throw new Error('Failed to resolve download path');
        downloads.update(list => {
          const cur = list.find(d => d.id === dl.id);
          if (cur) cur.targetPath = filePath;
          return list;
        });
        
        await invoke('download_file', {
          id: dl.id,
          url: dl.downloadLink,
          path: filePath,
        });
      } catch (error) {
        console.error(`Failed to start download for ${dl.name}:`, error);
        downloads.update(dlList => {
            const failedDl = dlList.find(d => d.id === id);
            if(failedDl) {
                failedDl.status = 'failed';
                failedDl.progress = 0;
                failedDl.speed = '';
                failedDl.eta = 'Failed';
            }
            return dlList;
        });
      }
    })();
  }
}

export async function cancelDownload(id: number) {
  try {
    await invoke('cancel_download', { id });
  } catch (error) {
    // Swallow cancel errors to keep UI responsive; update state regardless
    console.warn('cancel_download failed', error);
  } finally {
    downloads.update((dlList) => {
      const dl = dlList.find((d) => d.id === id);
      if (dl) {
        dl.status = 'available';
        dl.progress = 0;
        dl.speed = '';
        dl.eta = 'N/A';
        lastSample.delete(id);
      }
      return dlList;
    });
  }
}


