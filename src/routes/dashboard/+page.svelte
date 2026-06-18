<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { downloads } from '$lib/downloads';
  import { systemLogs as logStore, pushLog, type LogLevel, type LogEntry } from '$lib/logStore';
  import type { Download } from '$lib/downloadManager';
  import {
    Card,
    CardHeader,
    CardTitle,
    CardDescription,
    CardContent,
  } from '$lib/components/ui/card';
  import { Separator } from '$lib/components/ui/separator';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import { Badge } from '$lib/components/ui/badge';
  import { Progress } from '$lib/components/ui/progress';
  import { Button } from '$lib/components/ui/button';
  import {
    Table,
    TableHeader,
    TableRow,
    TableHead,
    TableBody,
    TableCell,
  } from '$lib/components/ui/table';
  import {
    Cpu,
    MemoryStick,
    HardDrive,
    DownloadIcon,
    ChevronRight,
    ChevronDown,
    ShieldCheck,
    Activity,
    Zap,
    Loader2,
  } from '@lucide/svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import { toast } from '$lib/components/ui/sonner';
  import { cleanerScan, beginCleanerScan, clearAllScannedItems } from '$lib/cleanerScan.svelte';
  import { i18n } from '$lib/i18n.svelte';

  // Read cleaner cache
  const totalCleanerFiles = $derived.by(() => {
    return (
      (cleanerScan.tempFiles?.length || 0) +
      (cleanerScan.largeFiles?.length || 0) +
      (cleanerScan.duplicateFiles?.length || 0) +
      (cleanerScan.emptyFolders?.length || 0) +
      (cleanerScan.brokenShortcuts?.length || 0)
    );
  });

  // Read tweaks status
  let activeTweaksCount = $state(0);
  let totalTweaksCount = $state(0);

  async function fetchTweaksCount() {
    try {
      const status = (await invoke('get_tweaks_status')) as Record<string, boolean>;
      const keys = Object.keys(status);
      totalTweaksCount = keys.length;
      activeTweaksCount = keys.filter((k) => status[k]).length;
    } catch {
      totalTweaksCount = 20;
      activeTweaksCount = 10;
    }
  }

  onMount(() => {
    void fetchTweaksCount();
  });

  const SYSTEM_INFO_CACHE_KEY = 'avelonia_dashboard_system_info_v1';

  type SystemInfoSnapshot = {
    cpuUsage: number;
    usedMemory: number;
    totalMemory: number;
    totalDiskSpace: number;
    availableDiskSpace: number;
  };

  function loadSystemInfoSnapshot(): SystemInfoSnapshot | null {
    if (typeof window === 'undefined') return null;
    try {
      const raw = localStorage.getItem(SYSTEM_INFO_CACHE_KEY);
      if (!raw) return null;
      const parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== 'object') return null;
      const asNumber = (value: unknown) =>
        typeof value === 'number' && Number.isFinite(value) ? value : 0;
      return {
        cpuUsage: asNumber(parsed.cpuUsage),
        usedMemory: asNumber(parsed.usedMemory),
        totalMemory: asNumber(parsed.totalMemory),
        totalDiskSpace: asNumber(parsed.totalDiskSpace),
        availableDiskSpace: asNumber(parsed.availableDiskSpace),
      };
    } catch {
      return null;
    }
  }

  function saveSystemInfoSnapshot(snapshot: SystemInfoSnapshot) {
    if (typeof window === 'undefined') return;
    try {
      localStorage.setItem(SYSTEM_INFO_CACHE_KEY, JSON.stringify(snapshot));
    } catch {
      // ignore
    }
  }

  const cachedSystemInfo = loadSystemInfoSnapshot();
  let cpuUsage = $state(cachedSystemInfo?.cpuUsage ?? 0);
  let usedMemory = $state(cachedSystemInfo?.usedMemory ?? 0);
  let totalMemory = $state(cachedSystemInfo?.totalMemory ?? 0);
  let totalDiskSpace = $state(cachedSystemInfo?.totalDiskSpace ?? 0);
  let availableDiskSpace = $state(cachedSystemInfo?.availableDiskSpace ?? 0);
  // Calculate Health Score dynamically
  const healthScore = $derived.by(() => {
    let score = 100;

    // CPU penalty (max -15)
    if (cpuUsage > 80) score -= 15;
    else if (cpuUsage > 50) score -= 8;
    else if (cpuUsage > 25) score -= 3;

    // Memory penalty (max -20)
    if (totalMemory > 0) {
      const memRatio = usedMemory / totalMemory;
      if (memRatio > 0.85) score -= 20;
      else if (memRatio > 0.65) score -= 10;
      else if (memRatio > 0.45) score -= 5;
    }

    // Disk space penalty (max -20)
    if (totalDiskSpace > 0) {
      const diskFreeRatio = availableDiskSpace / totalDiskSpace;
      if (diskFreeRatio < 0.1) score -= 20;
      else if (diskFreeRatio < 0.2) score -= 10;
      else if (diskFreeRatio < 0.35) score -= 5;
    }

    // Cleaner penalty (max -25)
    const filesCount = cleanerScan.phase === 'running' ? cleanerScan.found : totalCleanerFiles;
    if (filesCount > 5000) score -= 25;
    else if (filesCount > 1000) score -= 15;
    else if (filesCount > 200) score -= 8;
    else if (filesCount > 20) score -= 3;

    // Optimizations penalty (max -20)
    if (totalTweaksCount > 0) {
      const activeRatio = activeTweaksCount / totalTweaksCount;
      if (activeRatio < 0.3) score -= 20;
      else if (activeRatio < 0.6) score -= 10;
      else if (activeRatio < 0.8) score -= 5;
    }

    return Math.max(0, Math.min(100, Math.round(score)));
  });

  const healthStatus = $derived.by(() => {
    if (healthScore >= 80) {
      return {
        label: i18n.t('dashboard.status_excellent'),
        desc: i18n.t('dashboard.desc_excellent'),
        color: 'text-emerald-500',
        bg: 'bg-emerald-500/10',
        border: 'border-emerald-500/20',
        stroke: 'oklch(0.696 0.17 162.48)',
      };
    }
    if (healthScore >= 50) {
      return {
        label: i18n.t('dashboard.status_good'),
        desc: i18n.t('dashboard.desc_good'),
        color: 'text-amber-500',
        bg: 'bg-amber-500/10',
        border: 'border-amber-500/20',
        stroke: 'oklch(0.769 0.188 70.08)',
      };
    }
    return {
      label: i18n.t('dashboard.status_attention'),
      desc: i18n.t('dashboard.desc_attention'),
      color: 'text-rose-500',
      bg: 'bg-rose-500/10',
      border: 'border-rose-500/20',
      stroke: 'oklch(0.577 0.245 27.325)',
    };
  });

  const healthScoreLabel = $derived(i18n.t('dashboard.health_score'));

  let diagnosticState = $state<'idle' | 'scanning' | 'done'>('idle');
  let diagnosticMessage = $state('');

  async function startDiagnosticScan() {
    if (diagnosticState === 'scanning') return;
    diagnosticState = 'scanning';
    diagnosticMessage = i18n.t('dashboard.diagnostics_cpu_ram');
    await new Promise((r) => setTimeout(r, 600));

    diagnosticMessage = i18n.t('dashboard.diagnostics_temp');
    try {
      let exclusions: string[] = [];
      try {
        const raw = localStorage.getItem('avelonia_cleaner_exclusions_v1');
        if (raw) {
          exclusions = JSON.parse(raw);
        }
      } catch {
        /* noop */
      }
      beginCleanerScan();
      await invoke('start_cleaner_scan', {
        min_size_bytes: 100 * 1024 * 1024,
        max_temp: 5000,
        exclusions,
      });
    } catch (err) {
      pushLog('ERROR', `Diagnostic scan error: ${String(err)}`, 'General');
    }

    let attempts = 0;
    while (cleanerScan.phase === 'running' && attempts < 40) {
      await new Promise((r) => setTimeout(r, 800));
      attempts++;
    }

    diagnosticMessage = i18n.t('dashboard.diagnostics_tweaks');
    await fetchTweaksCount();
    await new Promise((r) => setTimeout(r, 600));

    diagnosticState = 'done';
    diagnosticMessage = i18n.t('dashboard.diagnostics_done');
    toast.success(i18n.t('dashboard.toast_diagnostics_success'));
  }

  let isFixing = $state(false);
  async function runQuickFix() {
    if (isFixing) return;
    isFixing = true;
    toast.info(i18n.t('dashboard.toast_quick_fix'));

    try {
      // 1. Quick clear user & system temp
      await invoke('quick_clear_user_temp');
      await invoke('quick_clear_system_temp');
      await invoke('quick_clear_prefetch');
      await invoke('quick_clear_recent');
      pushLog('SUCCESS', i18n.t('dashboard.log_cleared_temp'), 'Cleaner');

      // Clear the local sessionStorage cleaner cache so count updates immediately
      if (typeof window !== 'undefined') {
        sessionStorage.removeItem('avelonia_cleaner_cache_v1');
      }

      // 2. Apply all recommended Standard tweaks
      const standardProfileTweaks = [
        'disable_consumer_features',
        'disable_telemetry',
        'disable_activity_history',
        'disable_recall',
        'disable_explorer_discovery',
        'disable_gamedvr',
        'enable_end_task',
        'dark_theme',
        'show_file_extensions',
        'center_taskbar_items',
        'snap_window',
        'disable_chat',
        'disable_task_view',
        'disable_widgets',
        'disable_search',
      ];

      const changes = standardProfileTweaks.map((id) => ({ id, enabled: true }));
      await invoke('apply_tweaks_state_batch', { changes });
      pushLog('SUCCESS', i18n.t('dashboard.log_applied_standard'), 'Optimize');

      toast.success(i18n.t('dashboard.toast_quick_fix_success'));

      diagnosticState = 'done';
      diagnosticMessage = i18n.t('dashboard.diagnostics_done');

      clearAllScannedItems();
      await fetchTweaksCount();
    } catch (err) {
      toast.error('Failed to run all optimizations');
      pushLog('ERROR', `Quick fix failed: ${String(err)}`, 'General');
    } finally {
      isFixing = false;
    }
  }

  function formatBytes(bytes: number, decimals = 2) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
  }

  function levelBadgeClass(level: LogLevel) {
    if (level === 'SUCCESS') return 'border-green-500/20 text-green-700 bg-green-500/10';
    if (level === 'INFO') return 'border-blue-500/20 text-blue-700 bg-blue-500/10';
    if (level === 'WARN') return 'border-yellow-500/20 text-yellow-700 bg-yellow-500/10';
    return 'border-red-500/20 text-red-700 bg-red-500/10';
  }

  function readableDownloadStatus(status: Download['status']) {
    switch (status) {
      case 'available':
        return i18n.t('dashboard.status_available');
      case 'pending':
        return i18n.t('dashboard.status_pending');
      case 'downloading':
        return i18n.t('dashboard.status_downloading');
      case 'paused':
        return i18n.t('dashboard.status_paused');
      case 'completed':
        return i18n.t('dashboard.status_completed');
      case 'installed':
        return i18n.t('dashboard.status_installed');
      case 'queued':
        return i18n.t('dashboard.status_queued');
      case 'failed':
        return i18n.t('dashboard.status_failed');
      case 'verifying':
        return i18n.t('dashboard.status_verifying');
      default:
        return status;
    }
  }

  function getProgressValue(progress: number) {
    if (!Number.isFinite(progress) || progress < 0) return null;
    return Math.max(0, Math.min(100, Math.floor(progress)));
  }

  function describeDownloadTransition(
    dl: Download,
    _previousStatus: Download['status'] | undefined
  ): { level: LogLevel; message: string } | null {
    switch (dl.status) {
      case 'queued':
      case 'pending':
      case 'downloading':
      case 'completed':
      case 'available':
        return null;
      case 'failed':
        return {
          level: 'ERROR',
          message: i18n.t('dashboard.log_download_failed', { name: dl.name }),
        };
      case 'paused':
        return {
          level: 'WARN',
          message: i18n.t('dashboard.log_download_paused', { name: dl.name }),
        };
      default:
        return null;
    }
  }

  let systemInfoErrorLogged = false;

  $effect(() => {
    let fetchAbort = false;

    const fetchData = async () => {
      try {
        const [cpu, used, totalMem, [totalDisk, availDisk]] = await Promise.all([
          invoke<number>('get_cpu_usage'),
          invoke<number>('get_memory_usage'),
          invoke<number>('get_total_memory'),
          invoke<[number, number]>('get_drive_info'),
        ]);
        if (fetchAbort) return;
        cpuUsage = cpu;
        usedMemory = used;
        totalMemory = totalMem;
        totalDiskSpace = totalDisk;
        availableDiskSpace = availDisk;
        saveSystemInfoSnapshot({
          cpuUsage: cpu,
          usedMemory: used,
          totalMemory: totalMem,
          totalDiskSpace: totalDisk,
          availableDiskSpace: availDisk,
        });
        systemInfoErrorLogged = false;
      } catch (error) {
        if (!systemInfoErrorLogged) {
          const reason = error instanceof Error ? error.message : String(error);
          pushLog('ERROR', i18n.t('dashboard.log_fetch_sys_info_failed', { reason }));
          systemInfoErrorLogged = true;
        }
      }
    };

    const initialTimer = setTimeout(fetchData, 600);
    const intervalId = setInterval(fetchData, 5000);

    return () => {
      fetchAbort = true;
      clearTimeout(initialTimer);
      clearInterval(intervalId);
    };
  });

  const trackedDownloads = new SvelteMap<number, Download['status']>();
  let downloadsSnapshotReady = false;

  $effect(() => {
    const current = $downloads;
    if (!downloadsSnapshotReady) {
      trackedDownloads.clear();
      for (const dl of current) {
        trackedDownloads.set(dl.id, dl.status);
      }
      downloadsSnapshotReady = true;
      return;
    }

    const seen = new SvelteSet<number>();
    for (const dl of current) {
      seen.add(dl.id);
      const previousStatus = trackedDownloads.get(dl.id);
      if (previousStatus !== dl.status) {
        const log = describeDownloadTransition(dl, previousStatus);
        if (log) {
          pushLog(log.level, log.message);
        }
      }
      trackedDownloads.set(dl.id, dl.status);
    }

    for (const id of [...trackedDownloads.keys()]) {
      if (!seen.has(id)) {
        trackedDownloads.delete(id);
      }
    }
  });

  const activeDownloads = $derived(
    $downloads
      .filter(
        (dl: Download) =>
          dl.status === 'downloading' ||
          dl.status === 'pending' ||
          dl.status === 'queued' ||
          dl.status === 'verifying'
      )
      .map((dl) => ({
        data: dl,
        progressValue: getProgressValue(dl.progress),
      }))
  );

  const LOG_ROW_PX = 32;
  const LOG_MAX_DOM = 600;
  const LOG_VIEW_CHUNK = 40;
  const LOG_SCROLL_THRESHOLD_PX = 120;
  const INITIAL_LOGS_VISIBLE = LOG_VIEW_CHUNK;
  let logsStart = $state(0);
  let logsVisible = $state(INITIAL_LOGS_VISIBLE);
  let logsSkeleton = $state(new Set<number>());
  let initialLogLoading = $state(true);
  let logsScrollEl: HTMLDivElement | null = null;
  let logsSentinel: HTMLDivElement | null = null;
  let _logsTick = $state(false);
  let prevLogLen = $state(0);
  let logsReady = $state(false);

  const windowedLogs = $derived(
    $logStore.slice(logsStart, Math.min(logsVisible, $logStore.length))
  );

  function isVtLog(log: LogEntry): boolean {
    if ((log.category || '') !== 'Optimize') return false;
    const m = log.message || '';
    return m.startsWith('VT ') || /^VirusTotal\b/i.test(m) || /^Security alert:/i.test(m);
  }

  function isVtStart(log: LogEntry): boolean {
    const m = log.message || '';
    return m.startsWith('VT scan starting') || /^VirusTotal scan started\.?/i.test(m);
  }

  function isVtEnd(log: LogEntry): boolean {
    const m = log.message || '';
    return (
      m.startsWith('VT scan finished') ||
      m.startsWith('VT scan failed') ||
      m.startsWith('VT scan skipped') ||
      /^VirusTotal scan completed/i.test(m) ||
      /^VirusTotal up to date — scan skipped\.?/i.test(m) ||
      /^VirusTotal scan failed\.?/i.test(m)
    );
  }

  function isTweakLog(log: LogEntry): boolean {
    if ((log.category || '') !== 'Optimize') return false;
    const m = (log.message || '').toLowerCase();
    return (
      m.includes('transitioned setting') ||
      m.includes('transition setting') ||
      m.includes('tweaks batch') ||
      m.includes('settings batch') ||
      m.includes('update profile') ||
      m.includes('windows explorer') ||
      m.includes('fix action') ||
      m.includes('watermark removal') ||
      m.includes('activation status query') ||
      m.includes('tweak') ||
      m.includes('setting')
    );
  }

  function timeToSec(ts: string): number {
    const p = (ts || '').split(':').map((x) => parseInt(x, 10));
    if (p.length !== 3 || p.some((n) => Number.isNaN(n))) return 0;
    return p[0] * 3600 + p[1] * 60 + p[2];
  }

  type VtGroup = { header: number; indices: number[] };
  const vtGroups = $derived.by(() => {
    if (!logsReady) return [];
    const list = windowedLogs as LogEntry[];
    const used = new SvelteSet<number>();
    const out: VtGroup[] = [];
    const THRESH = 5 * 60;
    let current: { header: number; indices: number[]; headerSec: number } | null = null;
    for (let i = 0; i < list.length; i++) {
      const log = list[i];
      if (!isVtLog(log)) continue;
      if (current) {
        const sec = timeToSec(log.timestamp || '');
        if (isVtStart(log)) {
          const idx = current.indices.concat([i]);
          if (idx.length >= 2) {
            out.push({ header: current.header, indices: idx });
            for (const k of idx) used.add(k);
          }
          current = null;
        } else if (isVtEnd(log)) {
          if (current.indices.length >= 2) {
            out.push({ header: current.header, indices: current.indices.slice() });
            for (const k of current.indices) used.add(k);
          }
          current = { header: i, indices: [i], headerSec: timeToSec(log.timestamp || '') };
          continue;
        } else if (current.headerSec && sec > 0 && current.headerSec - sec > THRESH) {
          if (current.indices.length >= 2) {
            out.push({ header: current.header, indices: current.indices.slice() });
            for (const k of current.indices) used.add(k);
          }
          current = null;
        } else {
          current.indices.push(i);
        }
        continue;
      }
      if (isVtEnd(log)) {
        current = { header: i, indices: [i], headerSec: timeToSec(log.timestamp || '') };
        continue;
      }
    }
    if (current) {
      if (current.indices.length >= 2) {
        out.push({ header: current.header, indices: current.indices.slice() });
        for (const k of current.indices) used.add(k);
      }
      current = null;
    }
    let i = 0;
    while (i < list.length) {
      if (used.has(i) || !isVtLog(list[i]) || isVtStart(list[i]) || isVtEnd(list[i])) {
        i++;
        continue;
      }
      const head = i;
      const headSec = timeToSec(list[i].timestamp || '');
      const idx: number[] = [i];
      let j = i + 1;
      while (
        j < list.length &&
        !used.has(j) &&
        isVtLog(list[j]) &&
        !isVtStart(list[j]) &&
        !isVtEnd(list[j])
      ) {
        const sec = timeToSec(list[j].timestamp || '');
        if (headSec > 0 && sec > 0 && headSec - sec <= THRESH) {
          idx.push(j);
          j++;
        } else break;
      }
      if (idx.length >= 2) {
        out.push({ header: head, indices: idx });
        for (const k of idx) used.add(k);
      }
      i = j;
    }
    return out;
  });
  const vtGroupIndexMap = $derived.by(() => {
    if (!logsReady) return new SvelteMap<number, VtGroup>();
    const m = new SvelteMap<number, VtGroup>();
    for (const g of vtGroups) {
      for (const k of g.indices) m.set(k, g);
    }
    return m;
  });

  const vtHeaderSet = $derived(new SvelteSet(vtGroups.map((g) => g.header)));
  let vtCollapsed = $state(new Set<string>());
  let vtKnownHeaders = $state(new Set<string>());

  $effect(() => {
    const starts = new SvelteSet(
      vtGroups
        .map((g) => {
          const log = windowedLogs[g.header];
          return log ? log.timestamp + '|' + log.message : '';
        })
        .filter(Boolean)
    );
    let changedCollapsed = false;
    let changedKnown = false;
    for (const s of starts) {
      if (!vtKnownHeaders.has(s)) {
        vtKnownHeaders.add(s);
        changedKnown = true;
        if (!vtCollapsed.has(s)) {
          vtCollapsed.add(s);
          changedCollapsed = true;
        }
      }
    }
    for (const s of Array.from(vtCollapsed)) {
      if (!starts.has(s)) {
        vtCollapsed.delete(s);
        changedCollapsed = true;
      }
    }
    for (const s of Array.from(vtKnownHeaders)) {
      if (!starts.has(s)) {
        vtKnownHeaders.delete(s);
        changedKnown = true;
      }
    }
    if (changedCollapsed) vtCollapsed = new SvelteSet(vtCollapsed);
    if (changedKnown) vtKnownHeaders = new SvelteSet(vtKnownHeaders);
  });

  function toggleVtGroup(key: string) {
    if (vtCollapsed.has(key)) vtCollapsed.delete(key);
    else vtCollapsed.add(key);
    vtCollapsed = new SvelteSet(vtCollapsed);
  }

  type TweakGroup = { header: number; indices: number[] };
  const tweakGroups = $derived.by(() => {
    if (!logsReady) return [];
    const list = windowedLogs as LogEntry[];
    const used = new SvelteSet<number>();
    const out: TweakGroup[] = [];
    const THRESH = 15; // 15 seconds threshold for tweak adjustments

    let i = 0;
    while (i < list.length) {
      if (used.has(i) || !isTweakLog(list[i])) {
        i++;
        continue;
      }

      const head = i;
      const headSec = timeToSec(list[i].timestamp || '');
      const idx: number[] = [i];
      let j = i + 1;
      while (j < list.length && !used.has(j) && isTweakLog(list[j])) {
        const sec = timeToSec(list[j].timestamp || '');
        if (headSec > 0 && sec > 0 && Math.abs(headSec - sec) <= THRESH) {
          idx.push(j);
          j++;
        } else break;
      }

      if (idx.length >= 2) {
        out.push({ header: head, indices: idx });
        for (const k of idx) used.add(k);
      }
      i = j;
    }
    return out;
  });

  const tweakGroupIndexMap = $derived.by(() => {
    if (!logsReady) return new SvelteMap<number, TweakGroup>();
    const m = new SvelteMap<number, TweakGroup>();
    for (const g of tweakGroups) {
      for (const k of g.indices) m.set(k, g);
    }
    return m;
  });

  const tweakHeaderSet = $derived(new SvelteSet(tweakGroups.map((g) => g.header)));
  let tweakCollapsed = $state(new Set<string>());
  let tweakKnownHeaders = $state(new Set<string>());

  $effect(() => {
    const starts = new SvelteSet(
      tweakGroups
        .map((g) => {
          const log = windowedLogs[g.header];
          return log ? log.timestamp + '|' + log.message : '';
        })
        .filter(Boolean)
    );
    let changedCollapsed = false;
    let changedKnown = false;
    for (const s of starts) {
      if (!tweakKnownHeaders.has(s)) {
        tweakKnownHeaders.add(s);
        changedKnown = true;
        if (!tweakCollapsed.has(s)) {
          tweakCollapsed.add(s);
          changedCollapsed = true;
        }
      }
    }
    for (const s of Array.from(tweakCollapsed)) {
      if (!starts.has(s)) {
        tweakCollapsed.delete(s);
        changedCollapsed = true;
      }
    }
    for (const s of Array.from(tweakKnownHeaders)) {
      if (!starts.has(s)) {
        tweakKnownHeaders.delete(s);
        changedKnown = true;
      }
    }
    if (changedCollapsed) tweakCollapsed = new SvelteSet(tweakCollapsed);
    if (changedKnown) tweakKnownHeaders = new SvelteSet(tweakKnownHeaders);
  });

  function toggleTweakGroup(key: string) {
    if (tweakCollapsed.has(key)) tweakCollapsed.delete(key);
    else tweakCollapsed.add(key);
    tweakCollapsed = new SvelteSet(tweakCollapsed);
  }

  function markLogSkeletonRange(startIndex: number, endIndex: number) {
    try {
      for (let i = startIndex; i < endIndex; i++) logsSkeleton.add(i);
      logsSkeleton = new SvelteSet(logsSkeleton);
      setTimeout(() => {
        for (let i = startIndex; i < endIndex; i++) logsSkeleton.delete(i);
        logsSkeleton = new SvelteSet(logsSkeleton);
      }, 350);
    } catch {
      /* noop */
    }
  }

  function onLogsScroll(event: Event) {
    if (_logsTick) return;
    _logsTick = true;
    const el = (event.currentTarget as HTMLElement) || logsScrollEl;
    if (!el) {
      _logsTick = false;
      return;
    }
    requestAnimationFrame(() => {
      const nearBottomPx =
        el.scrollTop + el.clientHeight >= el.scrollHeight - LOG_SCROLL_THRESHOLD_PX;
      const ratio = (el.scrollTop + el.clientHeight) / Math.max(1, el.scrollHeight);
      const nearBottomRatio = ratio >= 0.8;
      if (nearBottomPx || nearBottomRatio) {
        const prev = logsVisible;
        const next = Math.min(prev + LOG_VIEW_CHUNK, $logStore.length);
        if (next > prev) {
          markLogSkeletonRange(prev, next);
          logsVisible = next;
          if (logsVisible - logsStart > LOG_MAX_DOM) {
            logsStart = Math.max(0, logsVisible - LOG_MAX_DOM);
          }
        }
      }
      _logsTick = false;
    });
  }

  onMount(() => {
    const t = setTimeout(() => {
      initialLogLoading = false;
      logsReady = true;
    }, 800);
    return () => clearTimeout(t);
  });

  onMount(() => {
    const io = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const prev = logsVisible;
          const next = Math.min(prev + LOG_VIEW_CHUNK, $logStore.length);
          if (next > prev) {
            markLogSkeletonRange(prev, next);
            logsVisible = next;
            if (logsVisible - logsStart > LOG_MAX_DOM) {
              logsStart = Math.max(0, logsVisible - LOG_MAX_DOM);
            }
          }
        }
      },
      { root: logsScrollEl, rootMargin: '0px', threshold: 0.1 }
    );
    if (logsSentinel) io.observe(logsSentinel);
    return () => io.disconnect();
  });

  $effect(() => {
    const total = $logStore.length;
    const delta = total - prevLogLen;
    if (delta > 0) {
      logsVisible = Math.min(total, Math.max(logsVisible + delta, INITIAL_LOGS_VISIBLE));
      logsStart = 0;
      try {
        if (logsScrollEl && logsScrollEl.scrollTop <= 4) {
          setTimeout(() => {
            try {
              if (logsScrollEl) logsScrollEl.scrollTop = 0;
            } catch {
              /* noop */
            }
          }, 0);
        }
      } catch {
        /* noop */
      }
    }
    prevLogLen = total;
    if (logsVisible > total) logsVisible = total;
    if (logsVisible - logsStart > LOG_MAX_DOM) {
      logsStart = Math.max(0, logsVisible - LOG_MAX_DOM);
    }
    if (logsStart > logsVisible) logsStart = 0;
  });
</script>

<div class="space-y-6 text-foreground">
  <div class="grid gap-6 md:grid-cols-3">
    <!-- Health Score Circular Gauge -->
    <Card
      class="glass-card md:col-span-1 flex flex-col items-center justify-center p-4 text-center transition-all duration-300 hover:scale-[1.01] glow-blue"
    >
      <CardHeader class="pb-1 w-full p-2">
        <CardTitle class="text-xs font-semibold tracking-wider uppercase text-muted-foreground"
          >{i18n.t('dashboard.system_health_score')}</CardTitle
        >
      </CardHeader>
      <CardContent class="flex flex-col items-center justify-center w-full relative p-2">
        <div class="relative flex items-center justify-center w-24 h-24">
          <svg class="w-full h-full transform -rotate-90" viewBox="0 0 80 80">
            <circle
              cx="40"
              cy="40"
              r="34"
              fill="transparent"
              stroke="color-mix(in oklch, var(--border) 60%, transparent)"
              stroke-width="6"
            />
            <circle
              cx="40"
              cy="40"
              r="34"
              fill="transparent"
              stroke={healthStatus.stroke}
              stroke-width="6"
              stroke-dasharray="214"
              stroke-dashoffset={214 - (214 * healthScore) / 100}
              stroke-linecap="round"
              class="transition-[stroke-dashoffset,stroke] duration-1000 ease-out"
            />
            <text
              x="40"
              y="38"
              class="fill-foreground font-extrabold tracking-tight font-heading"
              text-anchor="middle"
              transform="rotate(90 40 40)"
              font-size="20"
            >
              {healthScore}
            </text>
            <text
              x="40"
              y="52"
              class="fill-muted-foreground font-bold tracking-wider"
              text-anchor="middle"
              transform="rotate(90 40 40)"
              font-size={Math.max(5.0, Math.min(8.2, 90 / healthScoreLabel.length))}
            >
              {healthScoreLabel}
            </text>
          </svg>
        </div>

        <div class="mt-2.5">
          <Badge
            variant="outline"
            class="font-semibold text-[10px] tracking-wide uppercase px-2 py-0.5 rounded {healthStatus.color} {healthStatus.bg} {healthStatus.border}"
          >
            {healthStatus.label}
          </Badge>
        </div>
      </CardContent>
    </Card>

    <!-- Diagnostics and One-Click Quick Clean/Optimize -->
    <Card
      class="glass-card md:col-span-2 p-4 flex flex-col justify-between transition-all duration-300 hover:scale-[1.01]"
    >
      <div>
        <div class="flex items-center gap-2 mb-1">
          <h2 class="text-lg font-bold font-heading">
            {i18n.t('dashboard.system_diagnostics_fix')}
          </h2>
        </div>
        <p class="text-xs text-muted-foreground leading-relaxed mb-3">
          {healthStatus.desc}
        </p>

        <!-- Current scan status -->
        <div
          class="space-y-2.5 bg-muted/20 border border-border/30 rounded-lg p-3 mb-3 text-xs font-medium"
        >
          <div class="flex items-center gap-2.5">
            {#if diagnosticState === 'scanning'}
              <Loader2 class="size-3.5 animate-spin text-primary" />
              <span class="text-muted-foreground">{diagnosticMessage}</span>
            {:else if diagnosticState === 'done'}
              <ShieldCheck class="size-3.5 text-emerald-500" />
              <span>{diagnosticMessage}</span>
            {:else}
              <Activity class="size-3.5 text-blue-500" />
              <span class="text-muted-foreground">{i18n.t('dashboard.ready_message')}</span>
            {/if}
          </div>

          {#if cleanerScan.phase === 'running'}
            <div class="space-y-1">
              <div class="flex justify-between text-[10px] text-muted-foreground">
                <span>{i18n.t('dashboard.scanning_files')}</span>
                <span>{cleanerScan.found} items</span>
              </div>
              <Progress value={Math.min(100, (cleanerScan.found / 1000) * 100)} class="h-1" />
            </div>
          {/if}

          <!-- Diagnostic Metrics -->
          <div
            class="grid grid-cols-2 gap-x-4 gap-y-1.5 pt-2 border-t border-border/20 text-[11px] text-muted-foreground"
          >
            <div class="flex justify-between">
              <span>{i18n.t('dashboard.clutter_items')}</span>
              <span class="font-mono text-foreground font-semibold">
                {cleanerScan.phase === 'running' ? cleanerScan.found : totalCleanerFiles}
              </span>
            </div>
            <div class="flex justify-between">
              <span>{i18n.t('dashboard.applied_tweaks')}</span>
              <span class="font-mono text-foreground font-semibold"
                >{activeTweaksCount} / {totalTweaksCount}</span
              >
            </div>
          </div>
        </div>
      </div>

      <div class="flex flex-wrap gap-2 justify-end">
        <Button
          variant="outline"
          size="sm"
          onclick={startDiagnosticScan}
          disabled={diagnosticState === 'scanning'}
          class="min-w-28 text-xs h-8"
        >
          {#if diagnosticState === 'scanning'}
            <Loader2 class="size-3.5 mr-1 animate-spin" /> {i18n.t('dashboard.btn_scanning')}
          {:else}
            {i18n.t('dashboard.btn_scan')}
          {/if}
        </Button>
        <Button
          onclick={runQuickFix}
          size="sm"
          disabled={diagnosticState === 'scanning' || isFixing}
          class="min-w-28 text-xs h-8 bg-primary text-primary-foreground hover:bg-primary/95 shadow-md flex items-center gap-1.5"
        >
          {#if isFixing}
            <Loader2 class="size-3.5 animate-spin" /> {i18n.t('dashboard.btn_optimizing')}
          {:else}
            <Zap class="size-3.5 fill-current" /> {i18n.t('dashboard.btn_quick_optimize')}
          {/if}
        </Button>
      </div>
    </Card>
  </div>

  <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">{i18n.t('dashboard.cpu_usage')}</CardTitle>
        <Cpu class="size-5 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="flex items-baseline gap-2">
          <span class="text-3xl font-semibold">{cpuUsage.toFixed(1)}</span>
          <span class="text-muted-foreground">%</span>
        </div>
        <p class="mt-1 text-sm text-muted-foreground">{i18n.t('dashboard.current_performance')}</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">{i18n.t('dashboard.memory_usage')}</CardTitle>
        <MemoryStick class="size-5 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="flex items-baseline gap-2">
          <span class="text-3xl font-semibold">{formatBytes(usedMemory)}</span>
        </div>
        <p class="mt-1 text-sm text-muted-foreground">
          {i18n.t('dashboard.used_of', { total: formatBytes(totalMemory) })}
        </p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">{i18n.t('dashboard.disk_space')}</CardTitle>
        <HardDrive class="size-5 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="flex items-baseline gap-2">
          <span class="text-3xl font-semibold">{formatBytes(availableDiskSpace)}</span>
        </div>
        <p class="mt-1 text-sm text-muted-foreground">
          {i18n.t('dashboard.available_of', { total: formatBytes(totalDiskSpace) })}
        </p>
      </CardContent>
    </Card>
  </div>

  <div class="grid gap-4 lg:grid-cols-2">
    <Card>
      <CardHeader>
        <div class="flex items-center gap-2">
          <DownloadIcon class="size-5 text-muted-foreground" />
          <CardTitle>{i18n.t('dashboard.active_downloads')}</CardTitle>
        </div>
        <CardDescription>{i18n.t('dashboard.active_downloads_desc')}</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        {#if activeDownloads.length > 0}
          <div class="space-y-4">
            {#each activeDownloads as entry, index (entry.data.id)}
              <div class="space-y-3">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <p class="text-sm font-medium leading-none">{entry.data.name}</p>
                    <div
                      class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground"
                    >
                      <span>{readableDownloadStatus(entry.data.status)}</span>
                      {#if entry.data.speed}
                        <span>{i18n.t('dashboard.speed', { speed: entry.data.speed })}</span>
                      {/if}
                      {#if entry.data.eta && entry.data.eta !== 'N/A'}
                        <span>{i18n.t('dashboard.eta', { eta: entry.data.eta })}</span>
                      {/if}
                    </div>
                  </div>
                  <span class="text-xs text-muted-foreground">
                    {#if entry.progressValue !== null}
                      {entry.progressValue}%
                    {:else}
                      --
                    {/if}
                  </span>
                </div>
                {#if entry.progressValue !== null}
                  <Progress
                    value={entry.progressValue}
                    aria-label={`Download progress for ${entry.data.name}`}
                  />
                {:else}
                  <div class="h-2 w-full rounded-full bg-muted" aria-hidden="true"></div>
                {/if}
              </div>
              {#if index < activeDownloads.length - 1}
                <Separator />
              {/if}
            {/each}
          </div>
        {:else}
          <p class="text-sm text-muted-foreground">
            {i18n.t('dashboard.no_active_downloads_text')}
          </p>
        {/if}
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <CardTitle>{i18n.t('dashboard.system_logs')}</CardTitle>
        <CardDescription>{i18n.t('dashboard.system_logs_desc')}</CardDescription>
      </CardHeader>
      <CardContent>
        <div
          class="h-64 rounded-md bg-muted/10 overflow-auto"
          bind:this={logsScrollEl}
          onscroll={onLogsScroll}
        >
          <Table class="w-full">
            <TableHeader
              class="sticky top-0 bg-card/80 backdrop-blur supports-backdrop-filter:bg-card/70"
            >
              <TableRow class="border-0!">
                <TableHead class="w-20 text-xs text-muted-foreground"
                  >{i18n.t('dashboard.log_time')}</TableHead
                >
                <TableHead class="w-20 text-xs text-muted-foreground"
                  >{i18n.t('dashboard.log_level')}</TableHead
                >
                <TableHead class="text-xs text-muted-foreground"
                  >{i18n.t('dashboard.log_message')}</TableHead
                >
              </TableRow>
            </TableHeader>
            <TableBody>
              {#if initialLogLoading}
                {#each Array.from({ length: 6 }) as _, ii (ii)}
                  <TableRow class="border-0!">
                    <TableCell class="w-20"
                      ><Skeleton class="h-3 w-14" aria-hidden="true" /></TableCell
                    >
                    <TableCell class="w-20"
                      ><Skeleton class="h-3 w-12" aria-hidden="true" /></TableCell
                    >
                    <TableCell><Skeleton class="h-3 w-3/4" aria-hidden="true" /></TableCell>
                  </TableRow>
                {/each}
              {:else if $logStore.length === 0}
                <TableRow class="border-0!">
                  <TableCell colspan={3} class="py-6 text-center text-xs text-muted-foreground">
                    {i18n.t('dashboard.no_activity')}
                  </TableCell>
                </TableRow>
              {:else}
                {#if logsStart > 0}
                  <tr aria-hidden="true">
                    <td
                      colspan="3"
                      style={`height:${logsStart * LOG_ROW_PX}px; padding:0; border:0;`}
                    ></td>
                  </tr>
                {/if}
                {#each windowedLogs as log, i (log.timestamp + log.message + i)}
                  {#if logsSkeleton.has(logsStart + i)}
                    <TableRow class="border-0!" aria-hidden="true">
                      <TableCell class="w-20">
                        <Skeleton class="h-3 w-14" aria-hidden="true" />
                      </TableCell>
                      <TableCell class="w-20">
                        <Skeleton class="h-3 w-12" aria-hidden="true" />
                      </TableCell>
                      <TableCell><Skeleton class="h-3 w-3/4" aria-hidden="true" /></TableCell>
                    </TableRow>
                  {:else if vtHeaderSet.has(i)}
                    {@const g = vtGroupIndexMap.get(i) as VtGroup}
                    {@const stableKey = log.timestamp + '|' + log.message}
                    <TableRow
                      class="border-0! bg-blue-500/5 hover:bg-blue-500/10 cursor-pointer transition-colors"
                      onclick={() => toggleVtGroup(stableKey)}
                      role="button"
                      aria-expanded={!vtCollapsed.has(stableKey)}
                    >
                      <TableCell
                        class="font-mono text-[11px] text-muted-foreground pr-4 border-l-2 border-blue-500"
                      >
                        {log.timestamp}
                      </TableCell>
                      <TableCell class="pr-4" colspan={2}>
                        <div class="flex items-center gap-2">
                          <Badge
                            variant="secondary"
                            class="text-[11px] bg-blue-500/10 text-blue-700 dark:text-blue-300 border-blue-500/20"
                          >
                            {i18n.t('dashboard.log_vt_activity')}
                          </Badge>
                          <span class="text-xs font-medium text-muted-foreground"
                            >({g.indices.length} items)</span
                          >
                          <span
                            class="ml-auto inline-flex items-center text-xs text-muted-foreground/60 gap-1"
                          >
                            {#if vtCollapsed.has(stableKey)}
                              <span>{i18n.t('dashboard.expand')}</span>
                              <ChevronRight class="size-4" />
                            {:else}
                              <span>{i18n.t('dashboard.collapse')}</span>
                              <ChevronDown class="size-4" />
                            {/if}
                          </span>
                        </div>
                      </TableCell>
                    </TableRow>
                    {#if !vtCollapsed.has(stableKey)}
                      {#each g.indices as gi, idx (gi)}
                        {@const isLast = idx === g.indices.length - 1}
                        <TableRow class="border-0! bg-muted/5 hover:bg-muted/10 transition-colors">
                          <TableCell
                            class="font-mono text-[11px] text-muted-foreground pr-4 pl-6 border-l-2 border-blue-500/30"
                          >
                            <span class="inline-flex items-center gap-1.5">
                              <span class="text-muted-foreground/30 font-semibold"
                                >{isLast ? '└' : '├'}</span
                              >
                              {windowedLogs[gi].timestamp}
                            </span>
                          </TableCell>
                          <TableCell class="pr-4">
                            <Badge
                              variant="outline"
                              class={'text-[11px] ' + levelBadgeClass(windowedLogs[gi].level)}
                              >{windowedLogs[gi].level}</Badge
                            >
                          </TableCell>
                          <TableCell class="text-sm leading-snug text-muted-foreground/90">
                            {windowedLogs[gi].message}
                          </TableCell>
                        </TableRow>
                      {/each}
                    {/if}
                  {:else if vtGroupIndexMap.has(i)}
                    <!-- Inside a group (collapsed or expanded): skip individual row; items render under header -->
                  {:else if tweakHeaderSet.has(i)}
                    {@const g = tweakGroupIndexMap.get(i) as TweakGroup}
                    {@const stableKey = log.timestamp + '|' + log.message}
                    <TableRow
                      class="border-0! bg-emerald-500/5 hover:bg-emerald-500/10 cursor-pointer transition-colors"
                      onclick={() => toggleTweakGroup(stableKey)}
                      role="button"
                      aria-expanded={!tweakCollapsed.has(stableKey)}
                    >
                      <TableCell
                        class="font-mono text-[11px] text-muted-foreground pr-4 border-l-2 border-emerald-500"
                      >
                        {log.timestamp}
                      </TableCell>
                      <TableCell class="pr-4" colspan={2}>
                        <div class="flex items-center gap-2">
                          <Badge
                            variant="secondary"
                            class="text-[11px] bg-emerald-500/10 text-emerald-700 dark:text-emerald-300 border-emerald-500/20"
                          >
                            {i18n.t('dashboard.log_tweak_activity')}
                          </Badge>
                          <span class="text-xs font-medium text-muted-foreground"
                            >({g.indices.length} items)</span
                          >
                          <span
                            class="ml-auto inline-flex items-center text-xs text-muted-foreground/60 gap-1"
                          >
                            {#if tweakCollapsed.has(stableKey)}
                              <span>{i18n.t('dashboard.expand')}</span>
                              <ChevronRight class="size-4" />
                            {:else}
                              <span>{i18n.t('dashboard.collapse')}</span>
                              <ChevronDown class="size-4" />
                            {/if}
                          </span>
                        </div>
                      </TableCell>
                    </TableRow>
                    {#if !tweakCollapsed.has(stableKey)}
                      {#each g.indices as gi, idx (gi)}
                        {@const isLast = idx === g.indices.length - 1}
                        <TableRow class="border-0! bg-muted/5 hover:bg-muted/10 transition-colors">
                          <TableCell
                            class="font-mono text-[11px] text-muted-foreground pr-4 pl-6 border-l-2 border-emerald-500/30"
                          >
                            <span class="inline-flex items-center gap-1.5">
                              <span class="text-muted-foreground/30 font-semibold"
                                >{isLast ? '└' : '├'}</span
                              >
                              {windowedLogs[gi].timestamp}
                            </span>
                          </TableCell>
                          <TableCell class="pr-4">
                            <Badge
                              variant="outline"
                              class={'text-[11px] ' + levelBadgeClass(windowedLogs[gi].level)}
                              >{windowedLogs[gi].level}</Badge
                            >
                          </TableCell>
                          <TableCell class="text-sm leading-snug text-muted-foreground/90">
                            {windowedLogs[gi].message}
                          </TableCell>
                        </TableRow>
                      {/each}
                    {/if}
                  {:else if tweakGroupIndexMap.has(i)}
                    <!-- Inside a group (collapsed or expanded): skip individual row; items render under header -->
                  {:else}
                    <TableRow class="border-0! hover:bg-muted/30">
                      <TableCell class="font-mono text-[11px] text-muted-foreground pr-4"
                        >{log.timestamp}</TableCell
                      >
                      <TableCell class="pr-4">
                        <Badge
                          variant="outline"
                          class={'text-[11px] ' + levelBadgeClass(log.level)}
                        >
                          {log.level}
                        </Badge>
                      </TableCell>
                      <TableCell class="text-sm leading-snug">{log.message}</TableCell>
                    </TableRow>
                  {/if}
                {/each}
              {/if}
            </TableBody>
          </Table>
          <div bind:this={logsSentinel} class="h-0" aria-hidden="true"></div>
        </div>
      </CardContent>
    </Card>
  </div>
</div>
