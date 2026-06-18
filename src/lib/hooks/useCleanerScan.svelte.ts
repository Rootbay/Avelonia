import { onMount, onDestroy } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '$lib/components/ui/sonner';
import { endCleanerScan, setCleanerMessage, cleanerScan } from '$lib/cleanerScan.svelte';
import { loadCleanerCache, saveCleanerCache } from '$lib/cleanerCache';

export function useCleanerScan() {
  const unlistenFns: Array<() => void> = [];
  let cleanerToastShown = false;
  const cleanerToastId = 'cleaner-scan';
  let destroyed = false;

  type FilePair = [string, number];

  // Queues and flushes for throttling updates
  let tempQueue: FilePair[] = [];
  let tempFlushRaf: number | null = null;
  function scheduleTempFlush() {
    if (tempFlushRaf !== null) return;
    const run = () => {
      tempFlushRaf = null;
      if (tempQueue.length === 0) return;
      const take = tempQueue.splice(0, Math.min(2500, tempQueue.length));
      const next = take.map(([p, s]) => ({ path: p, size: s }));
      if (next.length) {
        const remaining = Math.max(0, 20000 - cleanerScan.tempFiles.length);
        if (remaining > 0) {
          cleanerScan.tempFiles = [...cleanerScan.tempFiles, ...next.slice(0, remaining)];
        }
      }
      if (tempQueue.length > 0) scheduleTempFlush();
    };
    tempFlushRaf = window.requestIdleCallback?.(run, { timeout: 120 }) ?? window.setTimeout(run, 0);
  }

  let largeQueue: FilePair[] = [];
  let largeFlushRaf: number | null = null;
  function scheduleLargeFlush() {
    if (largeFlushRaf !== null) return;
    const run = () => {
      largeFlushRaf = null;
      if (largeQueue.length === 0) return;
      const take = largeQueue.splice(0, Math.min(2000, largeQueue.length));
      const next = take.map(([p, s]) => ({ path: p, size: s }));
      if (next.length) {
        cleanerScan.largeFiles = [...cleanerScan.largeFiles, ...next];
      }
      if (largeQueue.length > 0) scheduleLargeFlush();
    };
    largeFlushRaf =
      window.requestIdleCallback?.(run, { timeout: 120 }) ?? window.setTimeout(run, 0);
  }

  let dupGroupsQueue: Array<{ hash: string; size: number; files: string[] }> = [];
  let dupFlushRaf: number | null = null;
  function scheduleDupFlush() {
    if (dupFlushRaf !== null) return;
    const run = () => {
      dupFlushRaf = null;
      if (dupGroupsQueue.length === 0) return;
      const take = dupGroupsQueue.splice(0, Math.min(500, dupGroupsQueue.length));
      if (take.length) {
        cleanerScan.dupGroups = [...cleanerScan.dupGroups, ...take];
        const flat = take.flatMap((g) =>
          (g.files || []).map((p) => ({ path: p as string, size: g.size }))
        );
        if (flat.length) {
          cleanerScan.duplicateFiles = [...cleanerScan.duplicateFiles, ...flat];
        }
      }
      if (dupGroupsQueue.length > 0) scheduleDupFlush();
    };
    dupFlushRaf = window.requestIdleCallback?.(run, { timeout: 120 }) ?? window.setTimeout(run, 0);
  }

  let emptyQueue: string[] = [];
  let emptyFlushRaf: number | null = null;
  function scheduleEmptyFlush() {
    if (emptyFlushRaf !== null) return;
    const run = () => {
      emptyFlushRaf = null;
      if (emptyQueue.length === 0) return;
      const take = emptyQueue.splice(0, Math.min(2000, emptyQueue.length));
      const next = take.map((p) => ({ path: p }));
      if (next.length) {
        cleanerScan.emptyFolders = [...cleanerScan.emptyFolders, ...next];
      }
      if (emptyQueue.length > 0) scheduleEmptyFlush();
    };
    emptyFlushRaf =
      window.requestIdleCallback?.(run, { timeout: 120 }) ?? window.setTimeout(run, 0);
  }

  let shortcutQueue: string[] = [];
  let shortcutFlushRaf: number | null = null;
  function scheduleShortcutFlush() {
    if (shortcutFlushRaf !== null) return;
    const run = () => {
      shortcutFlushRaf = null;
      if (shortcutQueue.length === 0) return;
      const take = shortcutQueue.splice(0, Math.min(2000, shortcutQueue.length));
      const next = take.map((p) => ({ path: p }));
      if (next.length) {
        cleanerScan.brokenShortcuts = [...cleanerScan.brokenShortcuts, ...next];
      }
      if (shortcutQueue.length > 0) scheduleShortcutFlush();
    };
    shortcutFlushRaf =
      window.requestIdleCallback?.(run, { timeout: 120 }) ?? window.setTimeout(run, 0);
  }

  function flushAllQueues() {
    // 1. Temp Queue
    if (tempQueue.length > 0) {
      const next = tempQueue.map(([p, s]) => ({ path: p, size: s }));
      const remaining = Math.max(0, 20000 - cleanerScan.tempFiles.length);
      if (remaining > 0) {
        cleanerScan.tempFiles = [...cleanerScan.tempFiles, ...next.slice(0, remaining)];
      }
      tempQueue = [];
      if (tempFlushRaf) {
        if (typeof window !== 'undefined' && 'cancelIdleCallback' in window) {
          (window as unknown as { cancelIdleCallback: (id: number) => void }).cancelIdleCallback(
            tempFlushRaf
          );
        } else {
          clearTimeout(tempFlushRaf);
        }
        tempFlushRaf = null;
      }
    }

    // 2. Large Queue
    if (largeQueue.length > 0) {
      const next = largeQueue.map(([p, s]) => ({ path: p, size: s }));
      cleanerScan.largeFiles = [...cleanerScan.largeFiles, ...next];
      largeQueue = [];
      if (largeFlushRaf) {
        if (typeof window !== 'undefined' && 'cancelIdleCallback' in window) {
          (window as unknown as { cancelIdleCallback: (id: number) => void }).cancelIdleCallback(
            largeFlushRaf
          );
        } else {
          clearTimeout(largeFlushRaf);
        }
        largeFlushRaf = null;
      }
    }

    // 3. Duplicate Queue
    if (dupGroupsQueue.length > 0) {
      cleanerScan.dupGroups = [...cleanerScan.dupGroups, ...dupGroupsQueue];
      const flat = dupGroupsQueue.flatMap((g) =>
        (g.files || []).map((p) => ({ path: p as string, size: g.size }))
      );
      if (flat.length) {
        cleanerScan.duplicateFiles = [...cleanerScan.duplicateFiles, ...flat];
      }
      dupGroupsQueue = [];
      if (dupFlushRaf) {
        if (typeof window !== 'undefined' && 'cancelIdleCallback' in window) {
          (window as unknown as { cancelIdleCallback: (id: number) => void }).cancelIdleCallback(
            dupFlushRaf
          );
        } else {
          clearTimeout(dupFlushRaf);
        }
        dupFlushRaf = null;
      }
    }

    // 4. Empty Queue
    if (emptyQueue.length > 0) {
      const next = emptyQueue.map((p) => ({ path: p }));
      cleanerScan.emptyFolders = [...cleanerScan.emptyFolders, ...next];
      emptyQueue = [];
      if (emptyFlushRaf) {
        if (typeof window !== 'undefined' && 'cancelIdleCallback' in window) {
          (window as unknown as { cancelIdleCallback: (id: number) => void }).cancelIdleCallback(
            emptyFlushRaf
          );
        } else {
          clearTimeout(emptyFlushRaf);
        }
        emptyFlushRaf = null;
      }
    }

    // 5. Shortcut Queue
    if (shortcutQueue.length > 0) {
      const next = shortcutQueue.map((p) => ({ path: p }));
      cleanerScan.brokenShortcuts = [...cleanerScan.brokenShortcuts, ...next];
      shortcutQueue = [];
      if (shortcutFlushRaf) {
        if (typeof window !== 'undefined' && 'cancelIdleCallback' in window) {
          (window as unknown as { cancelIdleCallback: (id: number) => void }).cancelIdleCallback(
            shortcutFlushRaf
          );
        } else {
          clearTimeout(shortcutFlushRaf);
        }
        shortcutFlushRaf = null;
      }
    }
  }

  onMount(() => {
    // Load cache on init
    try {
      const cache = loadCleanerCache();
      if (cache) {
        cleanerScan.tempFiles = cache.tempFiles || [];
        cleanerScan.largeFiles = cache.largeFiles || [];
        cleanerScan.duplicateFiles = cache.duplicateFiles || [];
        cleanerScan.emptyFolders = cache.emptyFolders || [];
        cleanerScan.brokenShortcuts = cache.brokenShortcuts || [];
        cleanerScan.dupGroups = cache.dupGroups || [];
        cleanerScan.phase = 'done';
      }
    } catch {}

    const setupListeners = async () => {
      const unScanProg = await listen<string>('scan_progress', (ev) => {
        try {
          const msg = ev.payload;
          setCleanerMessage(msg);
          if (cleanerToastShown) {
            toast.message(`${msg}`, { id: cleanerToastId, duration: Infinity });
          }
        } catch {}
      });
      if (destroyed) {
        unScanProg();
        return;
      }
      unlistenFns.push(unScanProg);

      // Listen for temp batch
      const unTempBatch = await listen<[string, number][]>('cleaner-temp-batch', (ev) => {
        try {
          const arr = ev.payload;
          if (Array.isArray(arr) && arr.length) {
            for (const it of arr) {
              if (cleanerScan.tempFiles.length + tempQueue.length >= 20000) {
                break;
              }
              tempQueue.push([String(it[0]), Number(it[1])]);
            }
            scheduleTempFlush();

            if (!cleanerToastShown) {
              cleanerToastShown = true;
              toast.message('Scanning temporary files...', {
                id: cleanerToastId,
                duration: Infinity,
                action: {
                  label: 'Stop',
                  onClick: async () => {
                    try {
                      await invoke('cancel_temp_scan');
                      await invoke('cancel_cleaner_scan');
                    } catch {}
                  },
                },
              });
            }
            if (cleanerScan.phase === 'running') {
              const label = cleanerScan.message || 'Scanning temporary files...';
              toast.message(`${label} (${cleanerScan.tempFiles.length.toLocaleString()} found)`, {
                id: cleanerToastId,
                duration: Infinity,
              });
            }
          }
        } catch {}
      });
      if (destroyed) {
        unTempBatch();
        return;
      }
      unlistenFns.push(unTempBatch);

      // Listen for temp done
      const unTempDone = await listen<{ total?: number }>('cleaner-temp-done', (_ev) => {
        try {
          toast.message(`Temporary files scan complete.`, {
            id: cleanerToastId,
            duration: 3000,
          });
          cleanerToastShown = false;
        } catch {}
      });
      if (destroyed) {
        unTempDone();
        return;
      }
      unlistenFns.push(unTempDone);

      // Listen for large batch
      const unLargeBatch = await listen<[string, number][]>('cleaner-large-batch', (ev) => {
        try {
          const arr = ev.payload;
          if (Array.isArray(arr) && arr.length) {
            for (const it of arr) {
              largeQueue.push([String(it[0]), Number(it[1])]);
            }
            scheduleLargeFlush();
          }
        } catch {}
      });
      if (destroyed) {
        unLargeBatch();
        return;
      }
      unlistenFns.push(unLargeBatch);

      // Listen for duplicate groups batch
      const unDupBatch = await listen<Array<{ hash: string; size: number; files: string[] }>>(
        'cleaner-dup-groups-batch',
        (ev) => {
          try {
            const groups = ev.payload;
            if (Array.isArray(groups) && groups.length) {
              for (const g of groups) {
                dupGroupsQueue.push(g);
              }
              scheduleDupFlush();
            }
          } catch {}
        }
      );
      if (destroyed) {
        unDupBatch();
        return;
      }
      unlistenFns.push(unDupBatch);

      // Listen for empty folders batch
      const unEmptyBatch = await listen<string[]>('cleaner-empty-batch', (ev) => {
        try {
          const arr = ev.payload;
          if (Array.isArray(arr) && arr.length) {
            for (const p of arr) {
              emptyQueue.push(String(p));
            }
            scheduleEmptyFlush();
          }
        } catch {}
      });
      if (destroyed) {
        unEmptyBatch();
        return;
      }
      unlistenFns.push(unEmptyBatch);

      // Listen for shortcuts batch
      const unShortcutBatch = await listen<string[]>('cleaner-shortcut-batch', (ev) => {
        try {
          const arr = ev.payload;
          if (Array.isArray(arr) && arr.length) {
            for (const p of arr) {
              shortcutQueue.push(String(p));
            }
            scheduleShortcutFlush();
          }
        } catch {}
      });
      if (destroyed) {
        unShortcutBatch();
        return;
      }
      unlistenFns.push(unShortcutBatch);

      // Listen for cleaner stopped
      const unStopped = await listen('cleaner-stopped', () => {
        cleanerScan.phase = 'done';
        cleanerScan.message = '';
        cleanerToastShown = false;
        toast.dismiss(cleanerToastId);
        toast.warning('Scan stopped by user');
      });
      if (destroyed) {
        unStopped();
        return;
      }
      unlistenFns.push(unStopped);

      // Listen for cleaner error
      const unError = await listen<string>('cleaner-error', (ev) => {
        cleanerScan.phase = 'done';
        cleanerScan.message = '';
        cleanerToastShown = false;
        toast.dismiss(cleanerToastId);
        toast.error(`Scan error: ${ev.payload}`);
      });
      if (destroyed) {
        unError();
        return;
      }
      unlistenFns.push(unError);

      // Listen for cleaner done
      const unDone = await listen<{ scope?: string }>('cleaner-done', (ev) => {
        try {
          const scope = ev.payload?.scope || 'all';
          if (scope === 'all') {
            flushAllQueues();
            endCleanerScan();
            cleanerToastShown = false;
            toast.dismiss(cleanerToastId);
            toast.success('System scan completed successfully!');

            saveCleanerCache({
              tempFiles: cleanerScan.tempFiles,
              largeFiles: cleanerScan.largeFiles,
              duplicateFiles: cleanerScan.duplicateFiles,
              emptyFolders: cleanerScan.emptyFolders,
              brokenShortcuts: cleanerScan.brokenShortcuts,
              dupGroups: cleanerScan.dupGroups,
              timestamp: Date.now(),
            });
          }
        } catch {}
      });
      if (destroyed) {
        unDone();
        return;
      }
      unlistenFns.push(unDone);
    };

    setupListeners();
  });

  onDestroy(() => {
    destroyed = true;
    unlistenFns.forEach((fn) => fn());

    // Clear timeouts/RAF timers
    if (tempFlushRaf) clearTimeout(tempFlushRaf);
    if (largeFlushRaf) clearTimeout(largeFlushRaf);
    if (dupFlushRaf) clearTimeout(dupFlushRaf);
    if (emptyFlushRaf) clearTimeout(emptyFlushRaf);
    if (shortcutFlushRaf) clearTimeout(shortcutFlushRaf);
  });
}
