import { downloads } from './downloads';

export interface Download {
  id: number;
  name: string;
  description: string;
  size: string; // Assuming size is a string like "100MB"
  fileType: string;
  category: string;
  eta: string;
  status: 'available' | 'pending' | 'downloading' | 'paused' | 'completed' | 'queued' | 'failed';
  progress: number;
  intervalId: ReturnType<typeof setInterval> | null;
  downloadStartTime: number;
  pausedTime: number;
  retryCount: number;
}

const MAX_CONCURRENT_DOWNLOADS = 3; // Example limit
const MAX_RETRIES = 3; // Max retries for a failed download
const downloadQueue: number[] = [];

function processQueue() {
  downloads.update((dlList) => {
    const activeDownloads = dlList.filter(d => d.status === 'downloading' || d.status === 'pending').length;
    while (activeDownloads < MAX_CONCURRENT_DOWNLOADS && downloadQueue.length > 0) {
      const nextDownloadId = downloadQueue.shift();
      if (nextDownloadId !== undefined) {
        const dl = dlList.find(d => d.id === nextDownloadId);
        if (dl && dl.status === 'queued') {
          // Directly call the internal start logic, bypassing the queue check
          // as it's already been queued and now being processed.
          startDownloadInternal(dl);
        }
      }
    }
    return dlList;
  });
}

// Internal function to start a download, used by processQueue and initial startDownload
function startDownloadInternal(dl: Download) {
  dl.status = 'pending';
  dl.eta = 'Calculating...';
  dl.progress = 0;
  dl.downloadStartTime = Date.now(); // Store start time
  dl.pausedTime = 0; // Reset paused time

  dl.intervalId = setInterval(() => {
    downloads.update((innerDlList) => {
      const innerDl = innerDlList.find((d) => d.id === dl.id);
      if (innerDl) {
        // Simulate a random failure
        if (Math.random() < 0.05 && innerDl.progress < 50) { // 5% chance to fail before 50% progress
          innerDl.status = 'failed';
          innerDl.eta = 'Failed';
          innerDl.retryCount++;
          if (innerDl.intervalId !== null) {
            clearInterval(innerDl.intervalId);
          }
          processQueue(); // Check queue after failure
          return innerDlList;
        }

        if (innerDl.progress < 100) {
          const simulatedSpeed = Math.random() * 5 + 5; // Simulate 5-10 units per second
          const elapsedTime = (Date.now() - innerDl.downloadStartTime - innerDl.pausedTime) / 1000; // Account for paused time
          const progressIncrement = (simulatedSpeed / 100) * (100 - innerDl.progress); // Simulate progress based on speed
          innerDl.progress = Math.min(100, innerDl.progress + progressIncrement);

          const totalTimeEstimate = (elapsedTime / innerDl.progress) * 100;
          const remainingTime = totalTimeEstimate - elapsedTime;

          if (innerDl.progress >= 100) {
            innerDl.progress = 100;
            innerDl.status = 'completed';
            innerDl.eta = 'Done';
            if (innerDl.intervalId !== null) {
              clearInterval(innerDl.intervalId);
            }
            processQueue(); // Check queue after completion
          } else if (innerDl.status === 'pending') {
            innerDl.status = 'downloading';
            innerDl.eta = formatEta(remainingTime);
          } else {
            innerDl.eta = formatEta(remainingTime);
          }
        }
      }
      return innerDlList;
    });
  }, 1000);
}

function formatEta(seconds: number): string {
  if (seconds < 60) {
    return `${Math.round(seconds)} sec`;
  } else if (seconds < 3600) {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.round(seconds % 60);
    return `${minutes} min${remainingSeconds > 0 ? ` ${remainingSeconds} sec` : ''}`;
  } else {
    const hours = Math.floor(seconds / 3600);
    const remainingMinutes = Math.round((seconds % 3600) / 60);
    return `${hours}h ${remainingMinutes}min`;
  }
}

export function startDownload(id: number) {
  downloads.update((dlList) => {
    const dl = dlList.find((d) => d.id === id);
    if (dl && dl.status === 'available') {
      const activeDownloads = dlList.filter(d => d.status === 'downloading' || d.status === 'pending').length;
      if (activeDownloads < MAX_CONCURRENT_DOWNLOADS) {
        startDownloadInternal(dl);
      } else {
        dl.status = 'queued';
        downloadQueue.push(id);
      }
    }
    return dlList;
  });
}

export function pauseDownload(id: number) {
  downloads.update((dlList) => {
    const dl = dlList.find((d) => d.id === id);
    if (dl && dl.status === 'downloading') {
      dl.status = 'paused';
      if (dl.intervalId !== null) {
        clearInterval(dl.intervalId);
      }
      dl.pausedTime += Date.now() - (dl.downloadStartTime + dl.pausedTime); // Accumulate paused time
    }
    return dlList;
  });
}

export function resumeDownload(id: number) {
  downloads.update((dlList) => {
    const dl = dlList.find((d) => d.id === id);
    if (dl && dl.status === 'paused') {
      dl.status = 'downloading';
      dl.downloadStartTime = Date.now() - (dl.progress / 100) * ((Date.now() - dl.downloadStartTime - dl.pausedTime) / (dl.progress > 0 ? dl.progress : 1)) * 1000; // Recalculate start time based on current progress and elapsed active time

      dl.intervalId = setInterval(() => {
        downloads.update((innerDlList) => {
          const innerDl = innerDlList.find((d) => d.id === id);
          if (innerDl && innerDl.progress < 100) {
            const simulatedSpeed = Math.random() * 5 + 5; // Simulate 5-10 units per second
            const elapsedTime = (Date.now() - innerDl.downloadStartTime - innerDl.pausedTime) / 1000;
            const progressIncrement = (simulatedSpeed / 100) * (100 - innerDl.progress);
            innerDl.progress = Math.min(100, innerDl.progress + progressIncrement);

            const totalTimeEstimate = (elapsedTime / innerDl.progress) * 100;
            const remainingTime = totalTimeEstimate - elapsedTime;

            if (innerDl.progress >= 100) {
              innerDl.progress = 100;
              innerDl.status = 'completed';
              innerDl.eta = 'Done';
              if (innerDl.intervalId !== null) {
                clearInterval(innerDl.intervalId);
              }
            } else {
              innerDl.eta = formatEta(remainingTime);
            }
          }
          return innerDlList;
        });
      }, 1000);
    }
    return dlList;
  });
}

export function cancelDownload(id: number) {
  downloads.update((dlList) => {
    const dl = dlList.find((d) => d.id === id);
    if (dl) {
      if (dl.intervalId !== null) {
        clearInterval(dl.intervalId);
      }
      dl.status = 'available';
      dl.progress = 0;
      dl.eta = 'N/A';
      dl.downloadStartTime = 0; // Reset start time
      dl.pausedTime = 0; // Reset paused time
      processQueue(); // Check queue after cancellation
    }
    return dlList;
  });
}

export function retryDownload(id: number) {
  downloads.update((dlList) => {
    const dl = dlList.find((d) => d.id === id);
    if (dl && dl.status === 'failed' && dl.retryCount < MAX_RETRIES) {
      dl.status = 'available'; // Reset to available to allow startDownload to pick it up
      dl.progress = 0;
      dl.eta = 'N/A';
      dl.downloadStartTime = 0;
      dl.pausedTime = 0;
      startDownload(id); // Attempt to start again
    }
    return dlList;
  });
}
