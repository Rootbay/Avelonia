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
  import { Card, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
  import { Separator } from '$lib/components/ui/separator';
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

  const statusFilters: Array<{ value: StatusGroup; label: string }> = [
    { value: 'all', label: 'All' },
    { value: 'available', label: 'Available' },
    { value: 'active', label: 'Active' },
    { value: 'completed', label: 'Completed' },
    { value: 'failed', label: 'Failed' },
  ];

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
      if ((d.status === 'available' || d.status === 'completed' || d.status === 'installed') && d.downloadLink) startable++;
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
          .filter((d) => (d.status === 'available' || d.status === 'completed' || d.status === 'installed') && d.downloadLink)
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
        filteredDownloads
          .filter((d) => d.status === 'failed')
          .forEach((d) => startDownload(d.id));
        break;
      case 'retryAllFailed':
        all
          .filter((d) => d.status === 'failed')
          .forEach((d) => startDownload(d.id));
        break;
      case 'copySelectedLinks': {
        const links = selected.map((d) => d.downloadLink).filter(Boolean).join('\n');
        if (links) {
          navigator.clipboard.writeText(links)
            .then(() => toast.success('Copied download links to clipboard'))
            .catch(() => toast.error('Failed to copy to clipboard'));
        }
        break;
      }
      case 'exportFilteredCSV': {
        const headers = 'Name,Size,Type,Category,ETA,Status,Link\n';
        const rows = filteredDownloads.map(d => 
          `"${d.name}","${d.size}","${d.fileType}","${d.category}","${d.eta}","${d.status}","${d.downloadLink}"`
        ).join('\n');
        const blob = new Blob([headers + rows], { type: 'text/csv;charset=utf-8;' });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.setAttribute('href', url);
        link.setAttribute('download', 'avelonia-downloads.csv');
        link.style.visibility = 'hidden';
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        toast.success('Exported filtered downloads to CSV');
        break;
      }
    }
  }
</script>

<div class="space-y-6">
  <Card class="bg-card/80 shadow-sm">
    <CardHeader>
      <CardTitle class="text-2xl font-semibold">Downloader</CardTitle>
      <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <CardDescription>Search, filter, and manage app downloads.</CardDescription>
        <div
          class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs font-medium text-muted-foreground"
        >
          <div class="flex items-center gap-1.5">
            <span class="size-1.5 rounded-full bg-border"></span>
            <span>Showing {filteredStats.count} / {globalStats.total}</span>
          </div>
          <Separator orientation="vertical" class="hidden h-3 md:flex" />
          <div class="flex items-center gap-1.5">
            <span class="size-1.5 rounded-full bg-primary"></span>
            <span class="text-primary">{globalStats.active} Active</span>
          </div>
          <div class="flex items-center gap-1.5">
            <span class="size-1.5 rounded-full bg-emerald-500"></span>
            <span class="text-emerald-500">{globalStats.completed} Done</span>
          </div>
        </div>
      </div>
    </CardHeader>
  </Card>

  <div class="flex flex-wrap items-center gap-3">
    <Input class="flex-1 min-w-65" placeholder="Search downloads..." bind:value={searchTerm} />
    <div class="flex items-center gap-2">
      <Button size="sm" variant="outline" onclick={() => (addOpen = true)}>Add</Button>
      <Button size="sm" variant="outline" onclick={() => (actionsOpen = true)}>
        <ListChecks class="size-4 mr-2" /> Actions
      </Button>
      <Button size="sm" variant="outline" onclick={() => (optionsOpen = true)}>Options</Button>
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
            >Selection</span
          >
          <span class="text-sm font-semibold">{selectedIds.size} items</span>
        </div>
        <div class="flex items-center gap-2">
          <Button size="sm" onclick={() => handleBulkAction('startSelected')}>Start</Button>
          <Button size="sm" variant="outline" onclick={() => handleBulkAction('cancelSelected')}
            >Cancel</Button
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
      ><DialogHeader><DialogTitle>Silent install</DialogTitle></DialogHeader>
      <DialogDescription
        >Tries to install supported installers silently using common flags.</DialogDescription
      >
      <DialogFooter><DialogClose><Button>Got it</Button></DialogClose></DialogFooter>
    </DialogContent>
  </Dialog>

  <Dialog bind:open={showVerifyInfo}>
    <DialogContent
      ><DialogHeader><DialogTitle>Installation verification</DialogTitle></DialogHeader>
      <DialogDescription
        >Checks for entries in Windows "Programs and Features" after install.</DialogDescription
      >
      <DialogFooter><DialogClose><Button>Got it</Button></DialogClose></DialogFooter>
    </DialogContent>
  </Dialog>

  <Dialog bind:open={showHelp}>
    <DialogContent
      ><DialogHeader><DialogTitle>Keyboard shortcuts</DialogTitle></DialogHeader>
      <div class="grid gap-2 text-sm">
        <p><strong>Ctrl/Cmd + A:</strong> Select all</p>
        <p><strong>Esc:</strong> Clear selection</p>
      </div>
      <DialogFooter><Button onclick={() => (showHelp = false)}>Close</Button></DialogFooter>
    </DialogContent>
  </Dialog>
</div>
