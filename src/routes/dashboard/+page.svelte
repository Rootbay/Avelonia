<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  // events not used here to reduce log duplication
  import { downloads } from '$lib/downloads';
  import { systemLogs as logStore, pushLog, type LogLevel } from '$lib/logStore';
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
  import { Cpu, MemoryStick, HardDrive, DownloadIcon } from '@lucide/svelte';

  let cpuUsage = $state(0);
  let usedMemory = $state(0);
  let totalMemory = $state(0);
  let totalDiskSpace = $state(0);
  let availableDiskSpace = $state(0);

  // System logs and pushLog are provided via $lib/logStore

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
      // Suppress noisy transitions; these are logged centrally by the manager if needed
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

  const trackedDownloads = new Map<number, Download['status']>();
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

    const seen = new Set<number>();
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

  // Only show Cleaner scan progress here if desired; comment out to reduce noise
  // Currently suppressed to keep logs focused on issues and errors.

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

  // System Logs: lazy loading + skeletons (similar to Downloader/Optimize)
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
  let _logsTick = false;

  const windowedLogs = $derived(
    $logStore.slice(logsStart, Math.min(logsVisible, $logStore.length))
  );
  const logsAfter = $derived(
    Math.max(0, $logStore.length - (logsStart + windowedLogs.length))
  );

  function markLogSkeletonRange(startIndex: number, endIndex: number) {
    try {
      for (let i = startIndex; i < endIndex; i++) logsSkeleton.add(i);
      logsSkeleton = new Set(logsSkeleton);
      setTimeout(() => {
        for (let i = startIndex; i < endIndex; i++) logsSkeleton.delete(i);
        logsSkeleton = new Set(logsSkeleton);
      }, 350);
    } catch {}
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
      const nearBottomPx = el.scrollTop + el.clientHeight >= el.scrollHeight - LOG_SCROLL_THRESHOLD_PX;
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
    // Disable initial skeleton shortly after mount
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

  // Keep DOM window bounded when logs change
  $effect(() => {
    const total = $logStore.length;
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
        <div class="h-64 rounded-md bg-muted/10 overflow-auto" bind:this={logsScrollEl} onscroll={onLogsScroll}>
          <Table class="w-full">
            <TableHeader class="sticky top-0 bg-card/80 backdrop-blur supports-[backdrop-filter]:bg-card/70">
              <TableRow class="!border-0">
                <TableHead class="w-[80px] text-xs text-muted-foreground">Time</TableHead>
                <TableHead class="w-[80px] text-xs text-muted-foreground">Level</TableHead>
                <TableHead class="text-xs text-muted-foreground">Message</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {#if initialLogLoading}
                {#each Array.from({ length: 6 }) as _, ii}
                  <TableRow class="!border-0">
                    <TableCell class="w-[80px]"><Skeleton class="h-3 w-14" aria-hidden="true" /></TableCell>
                    <TableCell class="w-[80px]"><Skeleton class="h-3 w-12" aria-hidden="true" /></TableCell>
                    <TableCell><Skeleton class="h-3 w-3/4" aria-hidden="true" /></TableCell>
                  </TableRow>
                {/each}
              {:else if $logStore.length === 0}
                <TableRow class="!border-0">
                  <TableCell colspan={3} class="py-6 text-center text-xs text-muted-foreground">
                    No activity recorded yet.
                  </TableCell>
                </TableRow>
              {:else}
                {#if logsStart > 0}
                  <tr aria-hidden="true">
                    <td colspan="3" style={`height:${logsStart * LOG_ROW_PX}px; padding:0; border:0;`}></td>
                  </tr>
                {/if}
                {#each windowedLogs as log, i (logsStart + i)}
                  {#if logsSkeleton.has(logsStart + i)}
                    <TableRow class="!border-0" aria-hidden="true">
                      <TableCell class="w-[80px]"><Skeleton class="h-3 w-14" aria-hidden="true" /></TableCell>
                      <TableCell class="w-[80px]"><Skeleton class="h-3 w-12" aria-hidden="true" /></TableCell>
                      <TableCell><Skeleton class="h-3 w-3/4" aria-hidden="true" /></TableCell>
                    </TableRow>
                  {:else}
                    <TableRow class="!border-0 hover:bg-muted/30">
                      <TableCell class="font-mono text-[11px] text-muted-foreground pr-4">{log.timestamp}</TableCell>
                      <TableCell class="pr-4">
                        <Badge variant="outline" class={'text-[11px] ' + levelBadgeClass(log.level)}>{log.level}</Badge>
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
