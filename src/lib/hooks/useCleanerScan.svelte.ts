import { onMount, onDestroy } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '$lib/components/ui/sonner';
import {
  beginCleanerScan,
  endCleanerScan,
  incCleanerFound,
  setCleanerMessage,
  cleanerScan,
} from '$lib/cleanerScan.svelte';

export function useCleanerScan() {
  const unlistenFns: Array<() => void> = [];
  let cleanerToastShown = false;
  const cleanerToastId = 'cleaner-scan';
  let destroyed = false;

  onMount(() => {
    const setupListeners = async () => {
      const unScanProg = await listen<string>('scan_progress', (ev) => {
        try {
          const msg = ev.payload;
          setCleanerMessage(msg);
          if (cleanerToastShown) {
            toast.message(`${msg}`, { id: cleanerToastId, duration: Infinity });
          }
        } catch {
          /* noop */
        }
      });
      if (destroyed) {
        unScanProg();
        return;
      }
      unlistenFns.push(unScanProg);

      const unTempBatch = await listen<string[]>('cleaner-temp-batch', (ev) => {
        try {
          const arr = ev.payload;
          const n = Array.isArray(arr) ? arr.length : 0;
          if (n > 0) {
            beginCleanerScan();
            incCleanerFound(n);
            if (!cleanerToastShown) {
              cleanerToastShown = true;
              toast.message('Scanning temporary files…', {
                id: cleanerToastId,
                duration: Infinity,
                action: {
                  label: 'Stop',
                  onClick: async () => {
                    try {
                      await invoke('cancel_temp_scan');
                    } catch {
                      /* noop */
                    }
                  },
                },
              });
            }
            if (cleanerScan.phase === 'running') {
              const label =
                cleanerScan.message && cleanerScan.message.length > 0
                  ? cleanerScan.message
                  : 'Scanning temporary files…';
              toast.message(`${label} (${cleanerScan.found.toLocaleString()} found)`, {
                id: cleanerToastId,
                duration: Infinity,
              });
            }
          }
        } catch {
          /* noop */
        }
      });
      if (destroyed) {
        unTempBatch();
        return;
      }
      unlistenFns.push(unTempBatch);

      const unTempDone = await listen<{ total?: number }>('cleaner-temp-done', (ev) => {
        try {
          const total = Number(ev.payload?.total || 0);
          endCleanerScan(total);
          toast.success(
            `Temp scan complete • ${Number.isFinite(total) ? total.toLocaleString() : '0'} files`
          );
          cleanerToastShown = false;
          try {
            toast.dismiss(cleanerToastId);
          } catch {
            /* noop */
          }
        } catch {
          /* noop */
        }
      });
      if (destroyed) {
        unTempDone();
        return;
      }
      unlistenFns.push(unTempDone);
    };

    setupListeners();
  });

  onDestroy(() => {
    destroyed = true;
    unlistenFns.forEach((fn) => fn());
  });
}
