import { downloads } from './downloads';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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
}

listen('download-progress', (event) => {
  const { id, downloaded, total } = event.payload as { id: number; downloaded: number; total: number };
  downloads.update((dlList) => {
    const dl = dlList.find((d) => d.id === id);
    if (dl && (dl.status === 'downloading' || dl.status === 'pending')) {
      dl.progress = (downloaded / total) * 100;
      dl.status = 'downloading';
      if (downloaded === total) {
        dl.status = 'completed';
        dl.progress = 100;
      }
    }
    return dlList;
  });
});

export function startDownload(id: number) {
  
  let dlToDownload: Download | undefined;

  downloads.update((dlList) => {
    const dl = dlList.find((d) => d.id === id);
    if (dl && dl.downloadLink && dl.status !== 'downloading' && dl.status !== 'pending') {
      dl.status = 'pending';
      dl.progress = 0;
      dlToDownload = dl;
    }
    return dlList;
  });

  if (dlToDownload) {
    
    const dl = dlToDownload;
    (async () => {
      try {
        const downloadsPath = await downloadDir();
        const fileName = `${dl.name.replace(/[^a-zA-Z0-9._-]/g, '_')}.${dl.fileType}`;
        const filePath = await join(downloadsPath, fileName);
        
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
            }
            return dlList;
        });
      }
    })();
  }
}

export async function cancelDownload(id: number) {
    await invoke('cancel_download', { id });
    downloads.update((dlList) => {
        const dl = dlList.find((d) => d.id === id);
        if (dl) {
          dl.status = 'available';
          dl.progress = 0;
        }
        return dlList;
    });
}