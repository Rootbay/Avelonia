<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { downloads } from '$lib/downloads';
  import type { Download } from '$lib/downloadManager';

  let cpuUsage = 0;
  let usedMemory = 0;
  let totalMemory = 0;
  let totalDiskSpace = 0;
  let availableDiskSpace = 0;

  interface LogEntry {
    timestamp: string;
    level: 'INFO' | 'WARN' | 'ERROR';
    message: string;
  }
  let systemLogs: LogEntry[] = [];

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
    if (systemLogs.length > 100) {
      systemLogs = systemLogs.slice(systemLogs.length - 100);
    }
  }

  onMount(() => {
    const fetchData = async () => {
      try {
        cpuUsage = await invoke('get_cpu_usage');
        usedMemory = await invoke('get_memory_usage');
        totalMemory = await invoke('get_total_memory');
        const [total, available]: [number, number] = await invoke('get_drive_info');
        totalDiskSpace = total;
        availableDiskSpace = available;
      } catch (error) {
        console.error('Failed to fetch system info:', error);
      }
    };

    fetchData();
    const intervalId = setInterval(fetchData, 5000);
    const logIntervalId = setInterval(generateMockLog, 3000);
    return () => {
      clearInterval(intervalId);
      clearInterval(logIntervalId);
    };
  });

  $: activeDownloads = $downloads.filter((dl: Download) =>
    dl.status === 'downloading' || dl.status === 'pending' || dl.status === 'queued'
  );
</script>

<div class="main-content">
  <div class="header-card">
    <h1>Welcome back!</h1>
    <p>Your system status at a glance.</p>
  </div>

  <div class="metrics-grid">
    <div class="metric-card">
      <h3 class="metric-title">CPU Usage</h3>
      <div class="metric-value-container">
        <span class="metric-value">{cpuUsage.toFixed(1)}</span>
        <span class="metric-unit">%</span>
      </div>
      <p class="metric-subtitle">Current performance</p>
    </div>

    <div class="metric-card">
      <h3 class="metric-title">Memory Usage</h3>
      <div class="metric-value-container">
        <span class="metric-value">{formatBytes(usedMemory)}</span>
      </div>
      <p class="metric-subtitle">Used of {formatBytes(totalMemory)}</p>
    </div>

    <div class="metric-card">
      <h3 class="metric-title">Disk Space</h3>
      <div class="metric-value-container">
        <span class="metric-value">{formatBytes(availableDiskSpace)}</span>
      </div>
      <p class="metric-subtitle">Available of {formatBytes(totalDiskSpace)}</p>
    </div>
  </div>

  <div class="panels">
    <section class="panel">
      <h2>Active Downloads</h2>
      {#if activeDownloads.length > 0}
        <ul>
          {#each activeDownloads as dl (dl.id)}
            <li>{dl.name} — {dl.status}{dl.progress > 0 ? ` (${Math.floor(dl.progress)}%)` : ''}</li>
          {/each}
        </ul>
      {:else}
        <p class="muted">No active downloads.</p>
      {/if}
    </section>

    <section class="panel">
      <h2>System Logs</h2>
      <div class="logs" aria-live="polite">
        {#each systemLogs as log, i (i)}
          <div class="log-row {log.level.toLowerCase()}">
            <span class="ts">{log.timestamp}</span>
            <span class="lvl">{log.level}</span>
            <span class="msg">{log.message}</span>
          </div>
        {/each}
      </div>
    </section>
  </div>
</div>

<style>
  .main-content { display: flex; flex-direction: column; gap: 20px; color: var(--avelonia-text); }
  .header-card { background: var(--avelonia-card); border: 1px solid var(--avelonia-border); border-radius: 12px; padding: 16px; }
  .metrics-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 12px; }
  .metric-card { background: var(--avelonia-card); border: 1px solid var(--avelonia-border); border-radius: 12px; padding: 16px; }
  .metric-title { margin: 0 0 8px; color: var(--avelonia-text); }
  .metric-value-container { display: flex; align-items: baseline; gap: 6px; }
  .metric-value { font-size: 1.8rem; font-weight: 600; }
  .metric-unit { color: var(--avelonia-text-muted); }
  .metric-subtitle { margin: 6px 0 0; color: var(--avelonia-text-muted); font-size: 0.9rem; }
  .panels { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .panel { background: var(--avelonia-card); border: 1px solid var(--avelonia-border); border-radius: 12px; padding: 16px; min-height: 200px; }
  .logs { display: grid; gap: 6px; max-height: 260px; overflow: auto; border: 1px solid var(--avelonia-border); border-radius: 8px; padding: 8px; }
  .log-row { display: grid; grid-template-columns: 70px 60px 1fr; gap: 8px; align-items: center; }
  .log-row .ts { color: var(--avelonia-text-muted); font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace; }
  .log-row .lvl { font-weight: 600; }
  .log-row.info .lvl { color: var(--avelonia-blue); }
  .log-row.warn .lvl { color: var(--avelonia-warning); }
  .log-row.error .lvl { color: var(--avelonia-danger); }
  .muted { color: var(--avelonia-text-muted); }
  @media (max-width: 1024px) { .panels { grid-template-columns: 1fr; } }
</style>

