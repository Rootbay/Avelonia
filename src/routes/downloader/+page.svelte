<script lang="ts">
  import { downloads } from '$lib/downloads';
  import {
    cancelAndRemoveDownloads,
    cancelDownload,
    getDownloadPath,
    startDownload,
  } from '$lib/downloadManager';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Card } from '$lib/components/ui/card';
  import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogClose,
  } from '$lib/components/ui/dialog';
  import { CircleX, ListChecks, Keyboard } from '@lucide/svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import AddDownloadDialog from '$lib/components/downloader/AddDownloadDialog.svelte';
  import BulkActionsSheet from '$lib/components/downloader/BulkActionsSheet.svelte';
  import DownloaderOptionsSheet from '$lib/components/downloader/DownloaderOptionsSheet.svelte';
  import DownloadsTable from '$lib/components/downloader/DownloadsTable.svelte';
  import type { Download } from '$lib/downloadManager';
  import { toast } from '$lib/components/ui/sonner';
  import { i18n } from '$lib/i18n.svelte';

  let searchTerm = $state('');
  let debouncedSearchTerm = $state('');
  let actionsOpen = $state(false);
  let optionsOpen = $state(false);
  let addOpen = $state(false);
  let showHelp = $state(false);
  let showInstallInfo = $state(false);
  let showVerifyInfo = $state(false);
  let initialLoading = $state(true);

  let filters = $state({
    fileType: '',
    category: '',
    minSize: '',
    maxSize: '',
    eta: '',
    status: '',
  });

  type StatusGroup = 'all' | 'available' | 'active' | 'completed' | 'failed';
  let statusGroup = $state<StatusGroup>('all');
  type SortKey = 'name' | 'size' | 'fileType' | 'category' | 'eta' | 'status';
  let sortBy = $state<SortKey>('name');
  let sortDirection = $state<'asc' | 'desc'>('asc');
  let selectedIds = new SvelteSet<number>();

  const statusFilters = $derived<Array<{ value: StatusGroup; label: string }>>([
    { value: 'all', label: i18n.t('downloader.category_all') },
    { value: 'available', label: i18n.t('dashboard.status_available') },
    { value: 'active', label: i18n.t('common.active') },
    { value: 'completed', label: i18n.t('dashboard.status_completed') },
    { value: 'failed', label: i18n.t('dashboard.status_failed') },
  ]);

  $effect(() => {
    const t = setTimeout(() => {
      debouncedSearchTerm = searchTerm;
    }, 150);
    return () => clearTimeout(t);
  });

  onMount(() => {
    const t = setTimeout(() => (initialLoading = false), 350);
    return () => clearTimeout(t);
  });

  function parseSpeedToBytes(speedStr: string | undefined): number {
    if (!speedStr) return 0;
    const s = speedStr.trim().toUpperCase();
    if (s === 'N/A' || s === '0 B/S' || !s) return 0;
    const match = s.match(/^(\d+(?:\.\d+)?)\s*([KMG]?B\/S)$/);
    if (!match) {
      const numOnly = parseFloat(s);
      return isNaN(numOnly) ? 0 : numOnly;
    }
    const val = parseFloat(match[1]);
    const unit = match[2];
    if (unit === 'KB/S') return val * 1024;
    if (unit === 'MB/S') return val * 1024 * 1024;
    if (unit === 'GB/S') return val * 1024 * 1024 * 1024;
    return val;
  }

  function formatSpeed(bytesPerSec: number): string {
    if (bytesPerSec === 0) return '0 B/s';
    const k = 1024;
    const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    const i = Math.floor(Math.log(bytesPerSec) / Math.log(k));
    const val = bytesPerSec / Math.pow(k, i);
    return `${val.toFixed(1)} ${sizes[i]}`;
  }

  const totalSpeedBytes = $derived.by(() => {
    let sum = 0;
    for (const d of $downloads) {
      if (['downloading', 'pending', 'queued', 'verifying'].includes(d.status)) {
        sum += parseSpeedToBytes(d.speed);
      }
    }
    return sum;
  });

  let speedHistory = $state<number[]>([]);

  onMount(() => {
    speedHistory = Array(30).fill(0);
    const interval = setInterval(() => {
      speedHistory = [...speedHistory.slice(1), totalSpeedBytes];
    }, 1000);
    return () => clearInterval(interval);
  });

  const maxSpeed = $derived(Math.max(...speedHistory, 1024 * 1024));
  const pathData = $derived.by(() => {
    if (speedHistory.length === 0) return '';
    return speedHistory
      .map((h, i) => {
        const x = (i / (speedHistory.length - 1)) * 300;
        const y = 55 - (h / maxSpeed) * 45;
        return `${i === 0 ? 'M' : 'L'} ${x} ${y}`;
      })
      .join(' ');
  });

  const areaPathData = $derived(pathData ? `${pathData} L 300 60 L 0 60 Z` : '');

  function toBytes(val: string | number | undefined | null): number {
    if (val === undefined || val === null) return 0;
    const s = String(val).trim();
    if (!s) return 0;
    const m = s.match(/^(\d+(?:\.\d+)?)\s*([kKmMgGtTpP]?[bB])?$/);
    if (!m) return parseFloat(s) || 0;
    const num = parseFloat(m[1]);
    const unit = (m[2] || 'B').toUpperCase();
    const map: Record<string, number> = {
      B: 1,
      KB: 1024,
      MB: 1024 ** 2,
      GB: 1024 ** 3,
      TB: 1024 ** 4,
      PB: 1024 ** 5,
    };
    return num * (map[unit] ?? 1);
  }

  const statusWeight: Record<string, number> = {
    available: 1,
    pending: 2,
    queued: 3,
    downloading: 4,
    verifying: 5,
    paused: 6,
    completed: 7,
    installed: 7,
    failed: 8,
  };

  const filteredDownloads = $derived(
    $downloads
      .filter((download) => {
        const matchesSearchTerm = download.name
          .toLowerCase()
          .includes(debouncedSearchTerm.toLowerCase());
        const matchesFileType = filters.fileType
          ? download.fileType.toLowerCase().includes(filters.fileType.toLowerCase())
          : true;
        const matchesCategory = filters.category
          ? download.category.toLowerCase().includes(filters.category.toLowerCase())
          : true;
        const downloadSizeBytes = toBytes(download.size);
        const minSizeBytes = filters.minSize !== '' ? toBytes(filters.minSize) : undefined;
        const maxSizeBytes = filters.maxSize !== '' ? toBytes(filters.maxSize) : undefined;
        const matchesMinSize =
          minSizeBytes !== undefined ? downloadSizeBytes >= minSizeBytes : true;
        const matchesMaxSize =
          maxSizeBytes !== undefined ? downloadSizeBytes <= maxSizeBytes : true;
        const matchesETA = filters.eta
          ? download.eta.toLowerCase().includes(filters.eta.toLowerCase())
          : true;
        const matchesStatus = filters.status
          ? download.status.toLowerCase() === filters.status.toLowerCase()
          : true;
        const matchesGroup = (() => {
          switch (statusGroup) {
            case 'available':
              return download.status === 'available';
            case 'completed':
              return download.status === 'completed' || download.status === 'installed';
            case 'failed':
              return download.status === 'failed';
            case 'active':
              return ['downloading', 'pending', 'queued', 'verifying'].includes(download.status);
            default:
              return true;
          }
        })();
        return (
          matchesSearchTerm &&
          matchesFileType &&
          matchesCategory &&
          matchesMinSize &&
          matchesMaxSize &&
          matchesETA &&
          matchesStatus &&
          matchesGroup
        );
      })
      .sort((a, b) => {
        let valA: Download[SortKey] | string | number | undefined = a[sortBy];
        let valB: Download[SortKey] | string | number | undefined = b[sortBy];
        if (sortBy === 'size') {
          valA = toBytes(valA);
          valB = toBytes(valB);
        }
        if (sortBy === 'status') {
          valA = statusWeight[String(valA)?.toLowerCase()] ?? 999;
          valB = statusWeight[String(valB)?.toLowerCase()] ?? 999;
        }
        if (typeof valA === 'string') valA = valA.toLowerCase();
        if (typeof valB === 'string') valB = valB.toLowerCase();
        if (valA < valB) return sortDirection === 'asc' ? -1 : 1;
        if (valA > valB) return sortDirection === 'asc' ? 1 : -1;
        return 0;
      })
  );

  const globalStats = $derived.by(() => {
    const list = $downloads;
    let active = 0,
      completed = 0,
      failed = 0,
      startable = 0,
      cancelable = 0;
    for (const d of list) {
      if (d.status === 'available' && d.downloadLink) startable++;
      if (['downloading', 'pending', 'queued', 'verifying'].includes(d.status)) {
        active++;
        if (d.status !== 'verifying') cancelable++;
      }
      if (d.status === 'completed' || d.status === 'installed') completed++;
      if (d.status === 'failed') failed++;
    }
    return { total: list.length, active, completed, failed, startable, cancelable };
  });

  const filteredStats = $derived.by(() => {
    let startable = 0,
      cancelable = 0,
      failed = 0;
    for (const d of filteredDownloads) {
      if (d.status === 'available' && d.downloadLink) startable++;
      if (['downloading', 'pending', 'queued'].includes(d.status)) cancelable++;
      if (d.status === 'failed') failed++;
    }
    return {
      count: filteredDownloads.length,
      startable,
      cancelable,
      failed,
      deletable: filteredDownloads.length,
    };
  });

  const selectedStats = $derived.by(() => {
    const selected = $downloads.filter((d) => selectedIds.has(d.id));
    let startable = 0,
      cancelable = 0,
      completed = 0;
    for (const d of selected) {
      if (
        (d.status === 'available' || d.status === 'completed' || d.status === 'installed') &&
        d.downloadLink
      )
        startable++;
      if (['downloading', 'pending', 'queued'].includes(d.status)) cancelable++;
      if (d.status === 'completed' || d.status === 'installed') completed++;
    }
    return {
      count: selectedIds.size,
      startable,
      cancelable,
      completed,
      deletable: selectedIds.size,
    };
  });

  function handleBulkAction(action: string) {
    const all = get(downloads);
    const selected = all.filter((d) => selectedIds.has(d.id));

    switch (action) {
      case 'startAll':
        all
          .filter((d) => d.status === 'available' && d.downloadLink)
          .forEach((d) => startDownload(d.id));
        break;
      case 'cancelAllActive':
        all
          .filter((d) => ['downloading', 'pending', 'queued'].includes(d.status))
          .forEach((d) => cancelDownload(d.id));
        break;
      case 'startAllFiltered':
        filteredDownloads
          .filter((d) => d.status === 'available' && d.downloadLink)
          .forEach((d) => startDownload(d.id));
        break;
      case 'cancelAllFiltered':
        filteredDownloads
          .filter((d) => ['downloading', 'pending', 'queued'].includes(d.status))
          .forEach((d) => cancelDownload(d.id));
        break;
      case 'deleteFiltered': {
        const filteredIds = filteredDownloads.map((d) => d.id);
        cancelAndRemoveDownloads(filteredIds);
        selectedIds.clear();
        break;
      }
      case 'startSelected':
        selected
          .filter(
            (d) =>
              (d.status === 'available' || d.status === 'completed' || d.status === 'installed') &&
              d.downloadLink
          )
          .forEach((d) => startDownload(d.id));
        break;
      case 'cancelSelected':
        selected
          .filter((d) => ['downloading', 'pending', 'queued'].includes(d.status))
          .forEach((d) => cancelDownload(d.id));
        break;
      case 'deleteSelected': {
        const ids = Array.from(selectedIds);
        cancelAndRemoveDownloads(ids);
        selectedIds.clear();
        break;
      }
      case 'openSelectedCompleted':
        selected
          .filter((d) => d.status === 'completed' || d.status === 'installed')
          .forEach(async (d) => {
            const p = await getDownloadPath(d);
            if (p) openPath(p);
          });
        break;
      case 'showSelectedCompleted':
        selected
          .filter((d) => d.status === 'completed' || d.status === 'installed')
          .forEach(async (d) => {
            const p = await getDownloadPath(d);
            if (p) revealItemInDir(p);
          });
        break;
      case 'retryFailedFiltered':
        filteredDownloads.filter((d) => d.status === 'failed').forEach((d) => startDownload(d.id));
        break;
      case 'retryAllFailed':
        all.filter((d) => d.status === 'failed').forEach((d) => startDownload(d.id));
        break;
      case 'copySelectedLinks': {
        const links = selected
          .map((d) => d.downloadLink)
          .filter(Boolean)
          .join('\n');
        if (links) {
          navigator.clipboard
            .writeText(links)
            .then(() => toast.success(i18n.t('downloader.toast_copied')))
            .catch(() => toast.error(i18n.t('downloader.toast_copy_failed')));
        }
        break;
      }
      case 'exportFilteredCSV': {
        const headers = 'Name,Size,Type,Category,ETA,Status,Link\n';
        const rows = filteredDownloads
          .map(
            (d) =>
              `"${d.name}","${d.size}","${d.fileType}","${d.category}","${d.eta}","${d.status}","${d.downloadLink}"`
          )
          .join('\n');
        const blob = new Blob([headers + rows], { type: 'text/csv;charset=utf-8;' });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.setAttribute('href', url);
        link.setAttribute('download', 'avelonia-downloads.csv');
        link.style.visibility = 'hidden';
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        toast.success(i18n.t('downloader.toast_exported_csv'));
        break;
      }
    }
  }
</script>

<div class="space-y-6">
  <div class="grid gap-4 md:grid-cols-3">
    <!-- Header info card -->
    <Card
      class="glass-card md:col-span-2 bg-card/80 shadow-sm p-4 flex flex-col justify-between transition-all duration-300 hover:scale-[1.005]"
    >
      <div>
        <h2 class="text-lg font-bold font-heading mb-1 text-foreground">
          {i18n.t('downloader.title')}
        </h2>
        <p class="text-xs text-muted-foreground leading-relaxed mb-3">
          {i18n.t('downloader.desc')}
        </p>
      </div>
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1.5 text-[11px] font-semibold">
        <div
          class="flex items-center gap-1 bg-muted/40 border border-border/30 rounded px-2 py-0.5 text-muted-foreground"
        >
          <span class="size-1.5 rounded-full bg-border"></span>
          <span
            >{i18n.t('downloader.showing_count', {
              filtered: filteredStats.count,
              total: globalStats.total,
            })}</span
          >
        </div>
        <div
          class="flex items-center gap-1 bg-primary/10 border border-primary/20 rounded px-2 py-0.5 text-primary"
        >
          <span class="size-1.5 rounded-full bg-primary animate-pulse"></span>
          <span>{i18n.t('downloader.active_count', { count: globalStats.active })}</span>
        </div>
        <div
          class="flex items-center gap-1 bg-emerald-500/10 border border-emerald-500/20 rounded px-2 py-0.5 text-emerald-500"
        >
          <span class="size-1.5 rounded-full bg-emerald-500"></span>
          <span>{i18n.t('downloader.done_count', { count: globalStats.completed })}</span>
        </div>
      </div>
    </Card>

    <!-- Speed History Sparkline -->
    <Card
      class="glass-card md:col-span-1 bg-card/80 shadow-sm p-4 flex flex-col justify-between overflow-hidden relative transition-all duration-300 hover:scale-[1.005] glow-blue"
    >
      <div class="z-10">
        <h3 class="text-[10px] font-bold uppercase tracking-wider text-muted-foreground mb-0.5">
          {i18n.t('downloader.download_speed')}
        </h3>
        <div class="flex items-baseline gap-1">
          <span class="text-2xl font-extrabold tracking-tight font-heading">
            {formatSpeed(totalSpeedBytes)}
          </span>
          <span class="text-[10px] text-muted-foreground font-medium"
            >{i18n.t('downloader.total_speed')}</span
          >
        </div>
      </div>

      <!-- Sparkline chart -->
      <div class="h-10 w-full mt-2 absolute bottom-0 left-0 right-0 z-0">
        {#if totalSpeedBytes > 0 || speedHistory.some((h) => h > 0)}
          <svg class="w-full h-full" viewBox="0 0 300 60" preserveAspectRatio="none">
            <defs>
              <linearGradient id="speedGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="var(--color-chart-1)" stop-opacity="0.2" />
                <stop offset="100%" stop-color="var(--color-chart-1)" stop-opacity="0.0" />
              </linearGradient>
            </defs>
            <!-- Area under path -->
            <path d={areaPathData} fill="url(#speedGrad)" class="transition-all duration-300" />
            <!-- Speed line path -->
            <path
              d={pathData}
              fill="none"
              stroke="var(--color-chart-1)"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="transition-all duration-300"
            />
          </svg>
        {:else}
          <div
            class="flex items-center justify-center h-full text-[10px] text-muted-foreground/30 font-medium pb-1 select-none"
          >
            {i18n.t('dashboard.no_active_downloads')}
          </div>
        {/if}
      </div>
    </Card>
  </div>

  <div class="flex flex-wrap items-center gap-3">
    <Input
      class="flex-1 min-w-65"
      placeholder={i18n.t('downloader.search_downloads')}
      bind:value={searchTerm}
    />
    <div class="flex items-center gap-2">
      <Button size="sm" variant="outline" onclick={() => (addOpen = true)}
        >{i18n.t('downloader.btn_add')}</Button
      >
      <Button size="sm" variant="outline" onclick={() => (actionsOpen = true)}>
        <ListChecks class="size-4 mr-2" />
        {i18n.t('cleaner.actions')}
      </Button>
      <Button size="sm" variant="outline" onclick={() => (optionsOpen = true)}
        >{i18n.t('downloader.btn_options')}</Button
      >
      <Button variant="ghost" size="icon" onclick={() => (showHelp = true)}>
        <Keyboard class="size-4" />
      </Button>
    </div>
  </div>

  <div class="flex flex-wrap items-center gap-2">
    {#each statusFilters as filter (filter.value)}
      <Button
        variant={statusGroup === filter.value ? 'default' : 'outline'}
        size="sm"
        onclick={() => (statusGroup = filter.value)}
      >
        {filter.label}
      </Button>
    {/each}
  </div>

  <DownloadsTable
    downloads={filteredDownloads}
    {initialLoading}
    {selectedIds}
    bind:sortBy
    bind:sortDirection
    onStart={startDownload}
    onCancel={cancelDownload}
  />

  {#if selectedIds.size > 0}
    <div class="fixed bottom-6 right-6 z-50 animate-in fade-in slide-in-from-bottom-4 duration-300">
      <div
        class="flex items-center gap-3 rounded-xl border bg-card/95 p-3 shadow-2xl backdrop-blur-md ring-1 ring-border/50"
      >
        <div class="flex flex-col border-r pr-3">
          <span class="text-[10px] font-bold uppercase tracking-wider text-muted-foreground"
            >{i18n.t('downloader.selection')}</span
          >
          <span class="text-sm font-semibold"
            >{i18n.t('downloader.items_count', { count: selectedIds.size })}</span
          >
        </div>
        <div class="flex items-center gap-2">
          <Button size="sm" onclick={() => handleBulkAction('startSelected')}
            >{i18n.t('downloader.btn_start')}</Button
          >
          <Button size="sm" variant="outline" onclick={() => handleBulkAction('cancelSelected')}
            >{i18n.t('common.cancel')}</Button
          >
          <Button size="sm" variant="ghost" onclick={() => (actionsOpen = true)}
            ><ListChecks class="size-4" /></Button
          >
          <Button size="sm" variant="ghost" onclick={() => selectedIds.clear()}
            ><CircleX class="size-4 opacity-50" /></Button
          >
        </div>
      </div>
    </div>
  {/if}

  <AddDownloadDialog bind:open={addOpen} />
  <BulkActionsSheet
    bind:open={actionsOpen}
    {globalStats}
    {filteredStats}
    {selectedStats}
    onAction={handleBulkAction}
  />
  <DownloaderOptionsSheet
    bind:open={optionsOpen}
    onShowInfo={(type) => {
      if (type === 'install') showInstallInfo = true;
      else showVerifyInfo = true;
    }}
  />

  <Dialog bind:open={showInstallInfo}>
    <DialogContent
      ><DialogHeader
        ><DialogTitle>{i18n.t('downloader.silent_install_title')}</DialogTitle></DialogHeader
      >
      <DialogDescription>{i18n.t('downloader.silent_install_desc')}</DialogDescription>
      <DialogFooter
        ><DialogClose><Button>{i18n.t('downloader.btn_got_it')}</Button></DialogClose></DialogFooter
      >
    </DialogContent>
  </Dialog>

  <Dialog bind:open={showVerifyInfo}>
    <DialogContent
      ><DialogHeader
        ><DialogTitle>{i18n.t('downloader.verify_install_title')}</DialogTitle></DialogHeader
      >
      <DialogDescription>{i18n.t('downloader.verify_install_desc')}</DialogDescription>
      <DialogFooter
        ><DialogClose><Button>{i18n.t('downloader.btn_got_it')}</Button></DialogClose></DialogFooter
      >
    </DialogContent>
  </Dialog>

  <Dialog bind:open={showHelp}>
    <DialogContent
      ><DialogHeader><DialogTitle>{i18n.t('downloader.shortcuts_title')}</DialogTitle></DialogHeader
      >
      <div class="grid gap-2 text-sm">
        <p><strong>Ctrl/Cmd + A:</strong> {i18n.t('downloader.shortcut_select_all')}</p>
        <p><strong>Esc:</strong> {i18n.t('downloader.help_clear_selection')}</p>
      </div>
      <DialogFooter
        ><Button onclick={() => (showHelp = false)}>{i18n.t('common.close')}</Button></DialogFooter
      >
    </DialogContent>
  </Dialog>
</div>
