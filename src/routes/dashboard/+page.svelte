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
} from '@lucide/svelte';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';

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
      // best-effort cache
    }
  }

  const cachedSystemInfo = loadSystemInfoSnapshot();
  let cpuUsage = $state(cachedSystemInfo?.cpuUsage ?? 0);
  let usedMemory = $state(cachedSystemInfo?.usedMemory ?? 0);
  let totalMemory = $state(cachedSystemInfo?.totalMemory ?? 0);
  let totalDiskSpace = $state(cachedSystemInfo?.totalDiskSpace ?? 0);
  let availableDiskSpace = $state(cachedSystemInfo?.availableDiskSpace ?? 0);

  const downloadStatusLabels: Record<Download['status'], string> = {
    available: 'Available',
    pending: 'Preparing',
    downloading: 'Downloading',
    paused: 'Paused',
    completed: 'Completed',
    installed: 'Installed',
    queued: 'Queued',
    failed: 'Failed',
  };

  function formatBytes(bytes: number, decimals = 2) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
  }

  function getTimestamp() {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  }

  function levelBadgeClass(level: LogLevel) {
    if (level === 'SUCCESS') return 'border-green-500/20 text-green-700 bg-green-500/10';
    if (level === 'INFO') return 'border-blue-500/20 text-blue-700 bg-blue-500/10';
    if (level === 'WARN') return 'border-yellow-500/20 text-yellow-700 bg-yellow-500/10';
    return 'border-red-500/20 text-red-700 bg-red-500/10';
  }

  function readableDownloadStatus(status: Download['status']) {
    return downloadStatusLabels[status] ?? status;
  }

  function getProgressValue(progress: number) {
    if (!Number.isFinite(progress) || progress < 0) return null;
    return Math.max(0, Math.min(100, Math.floor(progress)));
  }

  function describeDownloadTransition(
    dl: Download,
    previousStatus: Download['status'] | undefined
  ): { level: LogLevel; message: string } | null {
    switch (dl.status) {
      case 'queued':
      case 'pending':
      case 'downloading':
      case 'completed':
      case 'available':
        return null;
      case 'failed':
        return { level: 'ERROR', message: `Download ${dl.name} failed.` };
      case 'paused':
        return { level: 'WARN', message: `Download ${dl.name} paused.` };
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
        console.error('Failed to fetch system info:', error);
        if (!systemInfoErrorLogged) {
          const reason = error instanceof Error ? error.message : String(error);
          pushLog('ERROR', `Failed to fetch system info: ${reason}`);
          systemInfoErrorLogged = true;
        }
      }
    };

    fetchData();
    const intervalId = setInterval(fetchData, 5000);

    return () => {
      fetchAbort = true;
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
          dl.status === 'downloading' || dl.status === 'pending' || dl.status === 'queued'
      )
      .map((dl) => ({
        data: dl,
        progressValue: getProgressValue(dl.progress),
      }))
  );

  const LOG_ROW_PX = 32;
  const LOG_MAX_DOM = 600;
  const LOG_VIEW_CHUNK = 100;
  const LOG_SCROLL_THRESHOLD_PX = 120;
  let logsStart = $state(0);
  let logsVisible = $state(100);
  let logsSkeleton = $state(new Set<number>());
  let initialLogLoading = $state(true);
  let logsScrollEl: HTMLDivElement | null = null;
  let logsSentinel: HTMLDivElement | null = null;
  let _logsTick = $state(false);
  let prevLogLen = $state(0);

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

  function timeToSec(ts: string): number {
    const p = (ts || '').split(':').map((x) => parseInt(x, 10));
    if (p.length !== 3 || p.some((n) => Number.isNaN(n))) return 0;
    return p[0] * 3600 + p[1] * 60 + p[2];
  }

  type VtGroup = { header: number; indices: number[] };
  const vtGroups = $derived.by(() => {
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
    const m = new SvelteMap<number, VtGroup>();
    for (const g of vtGroups) {
      for (const k of g.indices) m.set(k, g);
    }
    return m;
  });

  const vtHeaderSet = $derived(new SvelteSet(vtGroups.map((g) => g.header)));
  let vtCollapsed = $state(new Set<number>());
  let vtKnownHeaders = $state(new Set<number>());
  
  $effect(() => {
    const starts = new SvelteSet(vtGroups.map((g) => g.header));
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

  function toggleVtGroup(headerIndex: number) {
    if (vtCollapsed.has(headerIndex)) vtCollapsed.delete(headerIndex);
    else vtCollapsed.add(headerIndex);
    vtCollapsed = new SvelteSet(vtCollapsed);
  }

  const logsAfter = $derived(Math.max(0, $logStore.length - (logsStart + windowedLogs.length)));

  function markLogSkeletonRange(startIndex: number, endIndex: number) {
    try {
      for (let i = startIndex; i < endIndex; i++) logsSkeleton.add(i);
      logsSkeleton = new SvelteSet(logsSkeleton);
      setTimeout(() => {
        for (let i = startIndex; i < endIndex; i++) logsSkeleton.delete(i);
        logsSkeleton = new SvelteSet(logsSkeleton);
      }, 350);
    } catch { /* noop */ }
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
    const t = setTimeout(() => (initialLogLoading = false), 350);
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
      logsVisible = Math.min(total, Math.max(logsVisible + delta, 100));
      logsStart = 0;
      try {
        if (logsScrollEl && logsScrollEl.scrollTop <= 4) {
          setTimeout(() => {
            try {
              if (logsScrollEl) logsScrollEl.scrollTop = 0;
            } catch { /* noop */ }
          }, 0);
        }
      } catch { /* noop */ }
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
  <Card>
    <CardHeader class="space-y-1">
      <CardTitle class="text-2xl">Welcome back!</CardTitle>
      <CardDescription>Your system status at a glance.</CardDescription>
    </CardHeader>
  </Card>

  <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">CPU Usage</CardTitle>
        <Cpu class="size-5 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="flex items-baseline gap-2">
          <span class="text-3xl font-semibold">{cpuUsage.toFixed(1)}</span>
          <span class="text-muted-foreground">%</span>
        </div>
        <p class="mt-1 text-sm text-muted-foreground">Current performance</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">Memory Usage</CardTitle>
        <MemoryStick class="size-5 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="flex items-baseline gap-2">
          <span class="text-3xl font-semibold">{formatBytes(usedMemory)}</span>
        </div>
        <p class="mt-1 text-sm text-muted-foreground">Used of {formatBytes(totalMemory)}</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">Disk Space</CardTitle>
        <HardDrive class="size-5 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="flex items-baseline gap-2">
          <span class="text-3xl font-semibold">{formatBytes(availableDiskSpace)}</span>
        </div>
        <p class="mt-1 text-sm text-muted-foreground">Available of {formatBytes(totalDiskSpace)}</p>
      </CardContent>
    </Card>
  </div>

  <div class="grid gap-4 lg:grid-cols-2">
    <Card>
      <CardHeader class="space-y-1">
        <div class="flex items-center gap-2">
          <DownloadIcon class="size-5 text-muted-foreground" />
          <CardTitle>Active Downloads</CardTitle>
        </div>
        <CardDescription>Items currently downloading or queued.</CardDescription>
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
                        <span>Speed {entry.data.speed}</span>
                      {/if}
                      {#if entry.data.eta && entry.data.eta !== 'N/A'}
                        <span>ETA {entry.data.eta}</span>
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
          <p class="text-sm text-muted-foreground">No active downloads.</p>
        {/if}
      </CardContent>
    </Card>

    <Card>
      <CardHeader class="space-y-1">
        <CardTitle>System Logs</CardTitle>
        <CardDescription>Live application and system events.</CardDescription>
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
                <TableHead class="w-20 text-xs text-muted-foreground">Time</TableHead>
                <TableHead class="w-20 text-xs text-muted-foreground">Level</TableHead>
                <TableHead class="text-xs text-muted-foreground">Message</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {#if initialLogLoading}
                {#each Array.from({ length: 6 }) as _, ii}
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
                    No activity recorded yet.
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
                {#each windowedLogs as log, i (logsStart + i)}
                  {#if logsSkeleton.has(logsStart + i)}
                    <TableRow class="border-0!" aria-hidden="true">
                      <TableCell class="w-20"
                        ><Skeleton class="h-3 w-14" aria-hidden="true" /></TableCell
                      >
                      <TableCell class="w-20"
                        ><Skeleton class="h-3 w-12" aria-hidden="true" /></TableCell
                      >
                      <TableCell><Skeleton class="h-3 w-3/4" aria-hidden="true" /></TableCell>
                    </TableRow>
                  {:else if vtHeaderSet.has(i)}
                    {@const g = vtGroupIndexMap.get(i) as VtGroup}
                    <TableRow
                      class="border-0! bg-muted/20 hover:bg-muted/30 cursor-pointer"
                      onclick={() => toggleVtGroup(i)}
                      role="button"
                      aria-expanded={!vtCollapsed.has(i)}
                    >
                      <TableCell class="font-mono text-[11px] text-muted-foreground pr-4"
                        >{log.timestamp}</TableCell
                      >
                      <TableCell class="pr-4" colspan={2}>
                        <div class="flex items-center gap-2">
                          <Badge variant="secondary" class="text-[11px]">VirusTotal activity</Badge>
                          <span class="text-xs text-muted-foreground">{g.indices.length} items</span
                          >
                          <span class="ml-auto inline-flex items-center">
                            {#if vtCollapsed.has(i)}
                              <ChevronRight class="size-4 text-muted-foreground" />
                            {:else}
                              <ChevronDown class="size-4 text-muted-foreground" />
                            {/if}
                          </span>
                        </div>
                      </TableCell>
                    </TableRow>
                    {#if !vtCollapsed.has(i)}
                      {#each g.indices as gi}
                        <TableRow class="border-0! hover:bg-muted/30">
                          <TableCell class="font-mono text-[11px] text-muted-foreground pr-4"
                            >{windowedLogs[gi].timestamp}</TableCell
                          >
                          <TableCell class="pr-4">
                            <Badge
                              variant="outline"
                              class={'text-[11px] ' + levelBadgeClass(windowedLogs[gi].level)}
                              >{windowedLogs[gi].level}</Badge
                            >
                          </TableCell>
                          <TableCell class="text-sm leading-snug"
                            >{windowedLogs[gi].message}</TableCell
                          >
                        </TableRow>
                      {/each}
                    {/if}
                  {:else if vtGroupIndexMap.has(i)}
                    <!-- Inside a group (collapsed or expanded): skip individual row; items render under header -->
                  {:else}
                    <TableRow class="border-0! hover:bg-muted/30">
                      <TableCell class="font-mono text-[11px] text-muted-foreground pr-4"
                        >{log.timestamp}</TableCell
                      >
                      <TableCell class="pr-4">
                        <Badge variant="outline" class={'text-[11px] ' + levelBadgeClass(log.level)}
                          >{log.level}</Badge
                        >
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
