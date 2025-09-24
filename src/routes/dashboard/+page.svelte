<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { downloads } from '$lib/downloads';
  import type { Download } from '$lib/downloadManager';

  import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '$lib/components/ui/card';
  import { Separator } from '$lib/components/ui/separator';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import { Badge } from '$lib/components/ui/badge';
  import { Progress } from '$lib/components/ui/progress';
  import { Table, TableHeader, TableRow, TableHead, TableBody, TableCell } from '$lib/components/ui/table';
  import { Cpu, MemoryStick, HardDrive, DownloadIcon } from '@lucide/svelte';

  let cpuUsage = $state(0);
  let usedMemory = $state(0);
  let totalMemory = $state(0);
  let totalDiskSpace = $state(0);
  let availableDiskSpace = $state(0);

  interface LogEntry {
    timestamp: string;
    level: 'INFO' | 'WARN' | 'ERROR';
    message: string;
  }
  let systemLogs = $state<LogEntry[]>([]);

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

  function generateMockLog() {
    const levels: LogEntry['level'][] = ['INFO', 'WARN', 'ERROR'];
    const messages = [
      'Scheduled task ran successfully',
      'Cache warmed: 24 entries',
      'Connection pool at capacity (8/8)',
      'Background sync completed',
      'Auth token refreshed',
      'High memory usage detected',
      'Disk IO throttled temporarily',
      'API rate limit nearing threshold',
      'Configuration reloaded',
      'Retrying message delivery to queue'
    ];
    const level = levels[Math.floor(Math.random() * levels.length)];
    const msg = messages[Math.floor(Math.random() * messages.length)];
    systemLogs = [...systemLogs, { timestamp: getTimestamp(), level, message: msg }];
    if (systemLogs.length > 100) systemLogs = systemLogs.slice(systemLogs.length - 100);
  }

  function levelBadgeClass(level: LogEntry['level']) {
    if (level === 'INFO') return 'border-blue-500/20 text-blue-700 bg-blue-500/10';
    if (level === 'WARN') return 'border-yellow-500/20 text-yellow-700 bg-yellow-500/10';
    return 'border-red-500/20 text-red-700 bg-red-500/10';
  }

  $effect(() => {
    let fetchAbort = false;

    const fetchData = async () => {
      try {
        const [cpu, used, totalMem, [totalDisk, availDisk]] = await Promise.all([
          invoke<number>('get_cpu_usage'),
          invoke<number>('get_memory_usage'),
          invoke<number>('get_total_memory'),
          invoke<[number, number]>('get_drive_info')
        ]);
        if (fetchAbort) return;
        cpuUsage = cpu;
        usedMemory = used;
        totalMemory = totalMem;
        totalDiskSpace = totalDisk;
        availableDiskSpace = availDisk;
      } catch (error) {
        console.error('Failed to fetch system info:', error);
      }
    };

    fetchData();
    const intervalId = setInterval(fetchData, 5000);
    const logIntervalId = setInterval(generateMockLog, 3000);

    return () => {
      fetchAbort = true;
      clearInterval(intervalId);
      clearInterval(logIntervalId);
    };
  });

  const activeDownloads = $derived(
    $downloads.filter(
      (dl: Download) => dl.status === 'downloading' || dl.status === 'pending' || dl.status === 'queued'
    )
  );
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
          {#each activeDownloads as dl (dl.id)}
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <div>
                  <p class="text-sm font-medium leading-none">{dl.name}</p>
                  <p class="text-xs text-muted-foreground capitalize">{dl.status}</p>
                </div>
                {#if dl.progress > 0}
                  <span class="text-xs text-muted-foreground">{Math.floor(dl.progress)}%</span>
                {/if}
              </div>
              {#if dl.progress > 0}
                <Progress value={Math.floor(dl.progress)} />
              {/if}
              <Separator />
            </div>
          {/each}
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
        <ScrollArea class="h-64 rounded-md border">
          <Table class="w-full">
            <TableHeader class="sticky top-0 bg-card">
              <TableRow>
                <TableHead class="w-[80px]">Time</TableHead>
                <TableHead class="w-[80px]">Level</TableHead>
                <TableHead>Message</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {#each systemLogs as log, i (i)}
                <TableRow>
                  <TableCell class="font-mono text-xs text-muted-foreground">{log.timestamp}</TableCell>
                  <TableCell>
                    <Badge variant="outline" class={'text-xs ' + levelBadgeClass(log.level)}>{log.level}</Badge>
                  </TableCell>
                  <TableCell class="text-sm">{log.message}</TableCell>
                </TableRow>
              {/each}
            </TableBody>
          </Table>
        </ScrollArea>
      </CardContent>
    </Card>
  </div>
</div>
