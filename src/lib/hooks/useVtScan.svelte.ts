import { onMount, onDestroy } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '$lib/components/ui/sonner';
import { pushLog } from '$lib/logStore';
import { openUrl as openExternal } from '@tauri-apps/plugin-opener';
import { setVerdict, setVerdictFromReport } from '$lib/vtVerdicts.svelte';
import { beginScan, endScan, pushReport as pushScanReport } from '$lib/scanStatus.svelte';

export function useVtScan() {
  const unlistenFns: Array<() => void> = [];
  let vtBusy = $state(false);
  let destroyed = false;

  async function runVtScanNow() {
    try {
      vtBusy = true;
      pushLog('INFO', 'VT scan starting (manual).', 'Optimize');
      beginScan('manual');
      toast.message('VirusTotal scan started');
      const res = (await invoke('vt_scan_all', { limit: 50, force: true })) as [number, number];
      endScan({ startup: res?.[0], registry: res?.[1] });
      toast.success('VirusTotal scan completed');
      pushLog(
        'SUCCESS',
        `VT scan finished (manual): startup ${res?.[0] ?? 0}, registry ${res?.[1] ?? 0}.`,
        'Optimize'
      );
    } catch (e) {
      toast.error('VirusTotal scan failed (set API key?)');
      pushLog('ERROR', `VT scan failed (manual): ${String(e)}`, 'Optimize');
    } finally {
      vtBusy = false;
    }
  }

  async function initVt() {
    try {
      await invoke('vt_load_cache');
      const status = (await invoke('vt_get_status')) as { key_set?: boolean };
      if (status && status.key_set) {
        if (destroyed) return;
        try {
          await invoke('vt_auto_maybe_scan');
        } catch {
          /* noop */
        }
        const iv = setInterval(() => {
          void (async () => {
            try {
              await invoke('vt_auto_maybe_scan');
            } catch {
              /* noop */
            }
          })();
        }, 60_000);
        unlistenFns.push(() => clearInterval(iv));
      } else {
        pushLog('INFO', 'VT key not set. Reputation scans disabled.', 'Optimize');
      }
    } catch {
      /* noop */
    }
  }

  onMount(() => {
    initVt();

    const setupListeners = async () => {
      const unAutoStart = await listen<{ reason?: string }>('vt-autoscan-start', (ev) => {
        toast.message('VirusTotal scan started (auto)');
        pushLog('INFO', 'VT scan starting (auto): ' + (ev.payload?.reason || 'auto'), 'Optimize');
      });
      if (destroyed) {
        unAutoStart();
        return;
      }
      unlistenFns.push(unAutoStart);

      const unAutoDone = await listen<{ startup?: number; registry?: number }>(
        'vt-autoscan-done',
        (ev) => {
          const p = ev.payload;
          toast.success('VirusTotal scan completed (auto)');
          pushLog(
            'SUCCESS',
            'VT scan finished (auto): startup ' +
              (Number(p?.startup) || 0) +
              ', registry ' +
              (Number(p?.registry) || 0) +
              '.',
            'Optimize'
          );
        }
      );
      if (destroyed) {
        unAutoDone();
        return;
      }
      unlistenFns.push(unAutoDone);

      const unAlert = await listen<{
        subject?: string;
        verdict?: string;
        positives?: number;
        permalink?: string;
        source?: string;
      }>('vt-alert', (ev) => {
        const p = ev.payload;
        const name = p?.subject || 'Startup item';
        const src = (p?.source || 'startup').toString();
        const sev = (p?.verdict || '').toString().toUpperCase();
        const msg = `${sev === 'MALICIOUS' ? 'Malicious' : 'Suspicious'} ${src === 'registry' ? 'registry item' : 'startup item'}: ${name}`;
        toast.error(msg, {
          action: p?.permalink
            ? {
                label: 'Open VirusTotal',
                onClick: async () => {
                  try {
                    await openExternal(p.permalink as string);
                  } catch {
                    /* noop */
                  }
                },
              }
            : undefined,
        });
        const lvl = sev === 'MALICIOUS' ? 'ERROR' : 'WARN';
        const pos = typeof p?.positives === 'number' ? ` (${p?.positives} vendors)` : '';
        pushLog(
          lvl,
          `VT detection: ${name}${pos}. ${p?.permalink ? 'Report available.' : ''}`,
          'Optimize'
        );
        setVerdict(name, 'Sus');
      });
      if (destroyed) {
        unAlert();
        return;
      }
      unlistenFns.push(unAlert);

      const unReport = await listen<import('$lib/vtVerdicts.svelte').VtReport>(
        'vt-report',
        (ev) => {
          const rep = ev.payload;
        try {
          setVerdictFromReport(rep);
          pushScanReport(rep);
          const v = String(rep?.verdict || '').toUpperCase();
          const pos = typeof rep?.positives === 'number' ? ` (${rep?.positives} vendors)` : '';
          pushLog('INFO', `VT report: ${rep?.subject ?? 'item'} -> ${v}${pos}`, 'Optimize');
        } catch {
          /* noop */
        }
        }
      );
      if (destroyed) {
        unReport();
        return;
      }
      unlistenFns.push(unReport);
    };

    setupListeners();
  });

  onDestroy(() => {
    destroyed = true;
    unlistenFns.forEach((fn) => fn());
  });

  return {
    runVtScanNow,
    vtBusy: () => vtBusy,
  };
}
