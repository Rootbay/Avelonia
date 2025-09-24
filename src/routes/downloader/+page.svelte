<script lang="ts">
  import { downloads } from '$lib/downloads';
  import FilterPanel from '$lib/components/FilterPanel.svelte';
  import DownloadItem from '$lib/components/DownloadItem.svelte';
  import { startDownload, cancelDownload, getDownloadPath } from '$lib/downloadManager';
  import type { Download } from '$lib/downloadManager';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from '$lib/components/ui/card';
  import { Separator } from '$lib/components/ui/separator';
  import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
  import {
    Sheet,
    SheetContent,
    SheetDescription,
    SheetHeader,
    SheetTitle,
    SheetTrigger,
  } from '$lib/components/ui/sheet';
  import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
  } from '$lib/components/ui/dialog';
  import { toast } from '$lib/components/ui/sonner';
  import { ArrowUpDown, ListChecks, SlidersHorizontal, Keyboard } from '@lucide/svelte';

  let searchTerm = $state('');
  let debouncedSearchTerm = $state('');
  $effect(() => {
    const t = setTimeout(() => {
      debouncedSearchTerm = searchTerm;
    }, 150);
    return () => clearTimeout(t);
  });
  let showFilters = $state(false);
  let actionsOpen = $state(false);
  let filters = $state({
    fileType: '',
    category: '',
    minSize: '',
    maxSize: '',
    eta: '',
    status: '',
  });
  type StatusGroup = 'all' | 'available' | 'active' | 'completed' | 'failed';
  type SortKey = 'name' | 'size' | 'status' | 'eta' | 'fileType' | 'category';

  const statusFilters: Array<{ value: StatusGroup; label: string }> = [
    { value: 'all', label: 'All' },
    { value: 'available', label: 'Available' },
    { value: 'active', label: 'Active' },
    { value: 'completed', label: 'Completed' },
    { value: 'failed', label: 'Failed' },
  ];

  const sortOptions: Array<{ value: SortKey; label: string }> = [
    { value: 'name', label: 'Name' },
    { value: 'size', label: 'Size' },
    { value: 'fileType', label: 'File Type' },
    { value: 'category', label: 'Category' },
    { value: 'eta', label: 'ETA' },
    { value: 'status', label: 'Status' },
  ];

  let statusGroup = $state<StatusGroup>('all');
  let sortBy = $state<SortKey>('name');
  let sortDirection = $state<'asc' | 'desc'>('asc');
  let selectedIds = $state(new Set<number>());
  const isSelected = (id: number) => selectedIds.has(id);
  let selectAllCheckbox: HTMLInputElement | null = null;
  let announce = $state('');
  let showHelp = $state(false);
  let lastSelectedIndex: number | null = null;

  function toggleSelect(id: number, value?: boolean) {
    if (value === undefined) {
      if (selectedIds.has(id)) selectedIds.delete(id);
      else selectedIds.add(id);
    } else {
      if (value) selectedIds.add(id);
      else selectedIds.delete(id);
    }
    selectedIds = new Set(selectedIds);
  }
  function clearSelection() {
    selectedIds = new Set();
  }

  const STORAGE_KEY = 'avelonia_downloader_ui_v1';
  onMount(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const s = JSON.parse(raw);
        if (typeof s?.searchTerm === 'string') searchTerm = s.searchTerm;
        if (s?.filters && typeof s.filters === 'object') {
          filters.fileType = s.filters.fileType ?? '';
          filters.category = s.filters.category ?? '';
          filters.minSize = s.filters.minSize ?? '';
          filters.maxSize = s.filters.maxSize ?? '';
          filters.eta = s.filters.eta ?? '';
          filters.status = s.filters.status ?? '';
        }
        if (typeof s?.sortBy === 'string') sortBy = s.sortBy;
        if (s?.sortDirection === 'asc' || s?.sortDirection === 'desc')
          sortDirection = s.sortDirection;
        if (s?.statusGroup) statusGroup = s.statusGroup;
      }
    } catch {}

    const keyHandler = (e: KeyboardEvent) => {
      const meta = e.ctrlKey || e.metaKey;
      if (meta && (e.key === 'a' || e.key === 'A')) {
        e.preventDefault();
        for (const d of filteredDownloads) selectedIds.add(d.id);
        selectedIds = new Set(selectedIds);
        return;
      }
      if (e.key === 'Escape') {
        clearSelection();
        return;
      }
      if (e.key === 'Delete' || e.key === 'Backspace') {
        cancelSelected();
        return;
      }
      if (e.key === 'Enter') {
        startSelected();
        return;
      }
    };
    window.addEventListener('keydown', keyHandler);
    return () => {
      window.removeEventListener('keydown', keyHandler);
    };
  });
  $effect(() => {
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ searchTerm, filters, sortBy, sortDirection, statusGroup })
      );
    } catch {}
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
    paused: 5,
    completed: 6,
    failed: 7,
  };

  function sortDownloads(a: Download, b: Download) {
    let valA: string | number;
    let valB: string | number;

    switch (sortBy) {
      case 'name':
        valA = a.name.toLowerCase();
        valB = b.name.toLowerCase();
        break;
      case 'size':
        valA = toBytes(a.size);
        valB = toBytes(b.size);
        break;
      case 'status':
        valA = statusWeight[a.status?.toLowerCase?.()] ?? 999;
        valB = statusWeight[b.status?.toLowerCase?.()] ?? 999;
        break;
      case 'eta':
        valA = a.eta.toLowerCase();
        valB = b.eta.toLowerCase();
        break;
      case 'fileType':
        valA = a.fileType.toLowerCase();
        valB = b.fileType.toLowerCase();
        break;
      case 'category':
        valA = a.category.toLowerCase();
        valB = b.category.toLowerCase();
        break;
      default:
        valA = a.id;
        valB = b.id;
    }

    if (valA < valB) {
      return sortDirection === 'asc' ? -1 : 1;
    } else if (valA > valB) {
      return sortDirection === 'asc' ? 1 : -1;
    } else {
      return 0;
    }
  }
  function setSort(key: SortKey) {
    if (sortBy === key) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = key;
      sortDirection = 'asc';
    }
  }

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
              return download.status === 'completed';
            case 'failed':
              return download.status === 'failed';
            case 'active':
              return (
                download.status === 'downloading' ||
                download.status === 'pending' ||
                download.status === 'queued'
              );
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
      .sort(sortDownloads)
  );

  const totalDownloads = $derived($downloads.length);
  const availableDownloads = $derived(filteredDownloads.length);
  const activeCount = $derived(
    $downloads.filter(
      (d) => d.status === 'downloading' || d.status === 'pending' || d.status === 'queued'
    ).length
  );
  const completedCount = $derived($downloads.filter((d) => d.status === 'completed').length);
  const failedCount = $derived($downloads.filter((d) => d.status === 'failed').length);
  const selectedCompletedCount = $derived(
    filteredDownloads.filter((d) => selectedIds.has(d.id) && d.status === 'completed').length
  );

  $effect(() => {
    const filteredIds = new Set(filteredDownloads.map((d) => d.id));
    let selectedInFilter = 0;
    for (const id of filteredIds) if (selectedIds.has(id)) selectedInFilter++;
    if (selectAllCheckbox) {
      const total = filteredIds.size;
      const allSelected = total > 0 && selectedInFilter === total;
      const noneSelected = selectedInFilter === 0;
      selectAllCheckbox.checked = allSelected;
      selectAllCheckbox.indeterminate = !allSelected && !noneSelected && total > 0;
    }
  });

  function formatBytes(bytes: number): string {
    if (!isFinite(bytes) || bytes <= 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
    let i = 0;
    while (bytes >= 1024 && i < units.length - 1) {
      bytes /= 1024;
      i++;
    }
    return `${bytes.toFixed(bytes >= 100 ? 0 : 1)} ${units[i]}`;
  }
  const filteredTotalBytes = $derived(
    filteredDownloads.reduce((sum, d) => sum + toBytes(d.size), 0)
  );

  function handleClearFilters() {
    searchTerm = '';
    filters.fileType = '';
    filters.category = '';
    filters.minSize = '';
    filters.maxSize = '';
    filters.eta = '';
    filters.status = '';
  }

  function startAll() {
    const list = get(downloads);
    for (const d of list) {
      if (d.status === 'available' && d.downloadLink) startDownload(d.id);
    }
  }

  function cancelAllActive() {
    const list = get(downloads);
    for (const d of list) {
      if (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued') {
        cancelDownload(d.id);
      }
    }
  }

  function startAllFiltered() {
    let queued = 0;
    for (const d of filteredDownloads) {
      if (d.status === 'available' && d.downloadLink) {
        startDownload(d.id);
        queued += 1;
      }
    }
    if (queued > 0) {
      toast.success(`Queued ${queued} download${queued === 1 ? '' : 's'}`);
    } else {
      toast.info('No available downloads to start');
    }
  }
  function cancelAllFiltered() {
    let canceled = 0;
    for (const d of filteredDownloads) {
      if (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued') {
        cancelDownload(d.id);
        canceled += 1;
      }
    }
    if (canceled > 0) {
      toast.success(`Canceled ${canceled} download${canceled === 1 ? '' : 's'}`);
    } else {
      toast.info('No active downloads to cancel');
    }
  }
  function startSelected() {
    let queued = 0;
    for (const d of filteredDownloads) {
      if (selectedIds.has(d.id) && d.status === 'available' && d.downloadLink) {
        startDownload(d.id);
        queued += 1;
      }
    }
    if (queued > 0) {
      toast.success(`Queued ${queued} selected download${queued === 1 ? '' : 's'}`);
    } else {
      toast.info('Select an available download first');
    }
  }
  function cancelSelected() {
    let canceled = 0;
    for (const d of filteredDownloads) {
      if (
        selectedIds.has(d.id) &&
        (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued')
      ) {
        cancelDownload(d.id);
        canceled += 1;
      }
    }
    if (canceled > 0) {
      toast.success(`Canceled ${canceled} selected download${canceled === 1 ? '' : 's'}`);
    } else {
      toast.info('No active selected downloads to cancel');
    }
  }

  async function openSelectedCompleted() {
    const items = filteredDownloads.filter(
      (d) => selectedIds.has(d.id) && d.status === 'completed'
    );
    if (items.length === 0) {
      toast.info('Select a completed download first');
      return;
    }
    let opened = 0;
    let failures = 0;
    for (const d of items) {
      try {
        const p = await getDownloadPath(d);
        if (p) {
          await openPath(p);
          opened += 1;
        } else {
          failures += 1;
        }
      } catch (error) {
        console.error('openSelectedCompleted failed', error);
        failures += 1;
      }
    }
    if (opened > 0) {
      toast.success(`Opened ${opened} completed download${opened === 1 ? '' : 's'}`);
    }
    if (failures > 0) {
      toast.error('Some items could not be opened');
    }
  }

  async function showSelectedCompleted() {
    const items = filteredDownloads.filter(
      (d) => selectedIds.has(d.id) && d.status === 'completed'
    );
    if (items.length === 0) {
      toast.info('Select a completed download first');
      return;
    }
    let revealed = 0;
    let failures = 0;
    for (const d of items) {
      try {
        const p = await getDownloadPath(d);
        if (p) {
          await revealItemInDir(p);
          revealed += 1;
        } else {
          failures += 1;
        }
      } catch (error) {
        console.error('showSelectedCompleted failed', error);
        failures += 1;
      }
    }
    if (revealed > 0) {
      toast.success(
        `Revealed ${revealed} download${revealed === 1 ? '' : 's'} in the file explorer`
      );
    }
    if (failures > 0) {
      toast.error('Some items could not be shown');
    }
  }

  function retryFailedFiltered() {
    let retried = 0;
    for (const d of filteredDownloads) {
      if (d.status === 'failed' && d.downloadLink) {
        startDownload(d.id);
        retried += 1;
      }
    }
    if (retried > 0) {
      toast.success(`Retrying ${retried} failed download${retried === 1 ? '' : 's'}`);
    } else {
      toast.info('No failed downloads in view to retry');
    }
  }
  function retryAllFailed() {
    const list = get(downloads);
    let retried = 0;
    for (const d of list) {
      if (d.status === 'failed' && d.downloadLink) {
        startDownload(d.id);
        retried += 1;
      }
    }
    if (retried > 0) {
      toast.success(`Retrying ${retried} failed download${retried === 1 ? '' : 's'}`);
    } else {
      toast.info('No failed downloads to retry');
    }
  }

  // Range selection with Shift
  function toggleSelectRange(currentIndex: number, value: boolean) {
    if (lastSelectedIndex === null) {
      const id = filteredDownloads[currentIndex]?.id;
      if (id !== undefined) {
        if (value) selectedIds.add(id);
        else selectedIds.delete(id);
      }
    } else {
      const start = Math.min(lastSelectedIndex, currentIndex);
      const end = Math.max(lastSelectedIndex, currentIndex);
      for (let i = start; i <= end; i++) {
        const id = filteredDownloads[i]?.id;
        if (id !== undefined) {
          if (value) selectedIds.add(id);
          else selectedIds.delete(id);
        }
      }
    }
    selectedIds = new Set(selectedIds);
    lastSelectedIndex = currentIndex;
  }

  function toggleSelectWithIndex(id: number, value: boolean, index: number, shiftKey: boolean) {
    if (shiftKey) {
      toggleSelectRange(index, value);
      announce = `${selectedIds.size} selected`;
      setTimeout(() => (announce = ''), 1200);
      return;
    }
    if (value) selectedIds.add(id);
    else selectedIds.delete(id);
    selectedIds = new Set(selectedIds);
    lastSelectedIndex = index;
  }

  function invertSelection() {
    const next = new Set<number>();
    const visible = new Set(filteredDownloads.map((d) => d.id));
    for (const id of visible) {
      if (!selectedIds.has(id)) next.add(id);
    }
    selectedIds = next;
  }

  function handleHeaderKey(e: KeyboardEvent, key: SortKey) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setSort(key);
    }
  }

  async function copySelectedLinks() {
    const links = filteredDownloads
      .filter((d) => selectedIds.has(d.id) && d.downloadLink)
      .map((d) => d.downloadLink);
    if (links.length === 0) {
      toast.info('Select at least one download first');
      return;
    }
    try {
      await navigator.clipboard.writeText(links.join('\n'));
      announce = `Copied ${links.length} link${links.length === 1 ? '' : 's'} to clipboard`;
      toast.success(announce);
      setTimeout(() => (announce = ''), 2000);
    } catch (error) {
      console.error('copySelectedLinks failed', error);
      announce = 'Copy failed';
      toast.error('Failed to copy download links');
      setTimeout(() => (announce = ''), 2000);
    }
  }

  function exportFilteredCSV() {
    const rows = [
      ['ID', 'Name', 'Size', 'File Type', 'Category', 'ETA', 'Status', 'Link'],
      ...filteredDownloads.map((d) => [
        String(d.id),
        d.name ?? '',
        String(d.size ?? ''),
        d.fileType ?? '',
        d.category ?? '',
        d.eta ?? '',
        d.status ?? '',
        d.downloadLink ?? '',
      ]),
    ];
    const csv = rows
      .map((r) =>
        r
          .map((v) => {
            const s = String(v ?? '');
            return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
          })
          .join(',')
      )
      .join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'downloads.csv';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    announce = 'Exported filtered list as CSV';
    toast.success(announce);
    setTimeout(() => (announce = ''), 2000);
  }
</script>

<div class="space-y-6">
  <div class="sr-only" aria-live="polite">{announce}</div>

  <Card class="border border-border/60 bg-card/80 shadow-sm">
    <CardHeader>
      <CardTitle class="text-2xl font-semibold">Downloader</CardTitle>
      <CardDescription>Search, filter, and manage app downloads.</CardDescription>
    </CardHeader>
    <CardContent class="mt-4 flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
      <span>Showing {availableDownloads} / {totalDownloads}</span>
      <Separator orientation="vertical" class="hidden h-4 md:flex" />
      <span>Size: {formatBytes(filteredTotalBytes)}</span>
      <Separator orientation="vertical" class="hidden h-4 md:flex" />
      <span class="text-primary">Active: {activeCount}</span>
      <span class="text-emerald-500">Completed: {completedCount}</span>
      <span class="text-destructive">Failed: {failedCount}</span>
    </CardContent>
  </Card>

  <Card class="border border-border/60 bg-card/70 shadow-sm">
    <CardHeader class="gap-4 pb-2">
      <div class="flex flex-col gap-3">
        <div class="flex flex-wrap items-center gap-3">
          <div class="flex min-w-[220px] flex-1 items-center gap-2">
            <Input class="w-full" placeholder="Search downloads..." bind:value={searchTerm} />
            <Button
              type="button"
              variant="outline"
              onclick={() => (showFilters = !showFilters)}
              aria-expanded={showFilters}
            >
              <SlidersHorizontal class="size-4" />
              <span class="ml-2 hidden sm:inline">Filters</span>
            </Button>
            <Sheet bind:open={actionsOpen}>
              <SheetTrigger>
                <Button type="button" variant="secondary">
                  <ListChecks class="size-4" />
                  <span class="ml-2 hidden sm:inline">Actions</span>
                </Button>
              </SheetTrigger>
              <SheetContent side="right" class="w-[320px] sm:w-[360px]">
                <SheetHeader>
                  <SheetTitle>Bulk actions</SheetTitle>
                  <SheetDescription
                    >Apply actions to the current selection or filtered list.</SheetDescription
                  >
                </SheetHeader>
                <div class="mt-4 grid gap-3">
                  <Button
                    type="button"
                    onclick={() => {
                      startAll();
                      actionsOpen = false;
                    }}>Start all</Button
                  >
                  <Button
                    type="button"
                    variant="destructive"
                    onclick={() => {
                      cancelAllActive();
                      actionsOpen = false;
                    }}
                  >
                    Cancel active
                  </Button>
                  <Separator class="my-1" />
                  <Button
                    type="button"
                    variant="outline"
                    onclick={() => {
                      startAllFiltered();
                      actionsOpen = false;
                    }}
                  >
                    Start filtered
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    onclick={() => {
                      cancelAllFiltered();
                      actionsOpen = false;
                    }}
                  >
                    Cancel filtered
                  </Button>
                  <Separator class="my-1" />
                  <Button
                    type="button"
                    disabled={selectedIds.size === 0}
                    onclick={() => {
                      startSelected();
                      actionsOpen = false;
                    }}
                  >
                    Start selected
                  </Button>
                  <Button
                    type="button"
                    variant="destructive"
                    disabled={selectedIds.size === 0}
                    onclick={() => {
                      cancelSelected();
                      actionsOpen = false;
                    }}
                  >
                    Cancel selected
                  </Button>
                  <Separator class="my-1" />
                  <Button
                    type="button"
                    variant="outline"
                    disabled={selectedIds.size === 0}
                    onclick={() => {
                      copySelectedLinks();
                      actionsOpen = false;
                    }}
                  >
                    Copy selected links
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    onclick={() => {
                      exportFilteredCSV();
                      actionsOpen = false;
                    }}
                  >
                    Export CSV
                  </Button>
                  <Separator class="my-1" />
                  <Button
                    type="button"
                    variant="outline"
                    onclick={() => {
                      retryFailedFiltered();
                      actionsOpen = false;
                    }}
                  >
                    Retry failed (filtered)
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    onclick={() => {
                      retryAllFailed();
                      actionsOpen = false;
                    }}
                  >
                    Retry all failed
                  </Button>
                </div>
              </SheetContent>
            </Sheet>
          </div>

          <div class="flex items-center gap-2">
            <Select type="single" bind:value={sortBy}>
              <SelectTrigger class="w-36" placeholder="Sort by" />
              <SelectContent>
                {#each sortOptions as option}
                  <SelectItem value={option.value}>{option.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
            <Button
              type="button"
              variant="outline"
              size="icon"
              onclick={() => (sortDirection = sortDirection === 'asc' ? 'desc' : 'asc')}
              aria-label={`Sort direction: ${sortDirection === 'asc' ? 'ascending' : 'descending'}`}
            >
              <ArrowUpDown class="size-4" />
            </Button>
            <Button type="button" variant="ghost" size="icon" onclick={() => (showHelp = true)}>
              <Keyboard class="size-4" />
              <span class="sr-only">Keyboard shortcuts</span>
            </Button>
          </div>
        </div>

        <div
          class="flex flex-wrap items-center gap-2"
          role="group"
          aria-label="Quick status filters"
        >
          {#each statusFilters as filter}
            <Button
              type="button"
              variant={statusGroup === filter.value ? 'default' : 'outline'}
              size="sm"
              onclick={() => (statusGroup = filter.value)}
            >
              {filter.label}
            </Button>
          {/each}
        </div>
      </div>
    </CardHeader>
  </Card>

  <FilterPanel bind:searchTerm bind:showFilters bind:filters onClearFilters={handleClearFilters} />

  <Card class="overflow-hidden border border-border/60">
    {#if selectedIds.size > 0}
      <div
        class="flex flex-wrap items-center gap-2 border-b border-border/60 bg-muted/40 px-4 py-3 text-sm"
      >
        <span><strong>{selectedIds.size}</strong> selected</span>
        <div class="flex flex-wrap items-center gap-2">
          <Button type="button" size="sm" onclick={startSelected}>Start</Button>
          <Button type="button" size="sm" variant="destructive" onclick={cancelSelected}
            >Cancel</Button
          >
          <Button type="button" size="sm" variant="outline" onclick={copySelectedLinks}
            >Copy links</Button
          >
          <Button type="button" size="sm" variant="outline" onclick={exportFilteredCSV}
            >Export CSV</Button
          >
          <Button
            type="button"
            size="sm"
            variant="outline"
            onclick={openSelectedCompleted}
            disabled={selectedCompletedCount === 0}
          >
            Open
          </Button>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onclick={showSelectedCompleted}
            disabled={selectedCompletedCount === 0}
          >
            Show
          </Button>
          <Button type="button" size="sm" variant="ghost" onclick={invertSelection}>Invert</Button>
          <Button type="button" size="sm" variant="ghost" onclick={clearSelection}>Clear</Button>
        </div>
      </div>
    {/if}

    <CardContent class="p-0">
      <div
        class="hidden border-b border-border/60 bg-muted/40 px-4 py-3 text-xs font-medium text-muted-foreground md:grid md:grid-cols-[auto,minmax(220px,1fr),repeat(4,minmax(120px,0.6fr)),minmax(160px,0.8fr)] md:items-center md:gap-3"
      >
        <span class="flex justify-center">
          <input
            bind:this={selectAllCheckbox}
            type="checkbox"
            class="h-4 w-4 rounded border-border bg-background text-primary focus:ring-ring"
            onchange={(e) => {
              const check = (e.target as HTMLInputElement).checked;
              for (const d of filteredDownloads) toggleSelect(d.id, check);
            }}
            title="Select or deselect all filtered downloads"
          />
        </span>
        {#each sortOptions as option (option.value)}
          <button
            class="flex items-center gap-1 text-left transition hover:text-foreground"
            aria-sort={sortBy === option.value
              ? sortDirection === 'asc'
                ? 'ascending'
                : 'descending'
              : 'none'}
            onclick={() => setSort(option.value)}
            onkeydown={(event) => handleHeaderKey(event, option.value)}
          >
            <span>{option.label}</span>
            {#if sortBy === option.value}
              <ArrowUpDown class="size-3 transition rotate-180={sortDirection === 'asc'}" />
            {/if}
          </button>
        {/each}
      </div>

      <div class="divide-y divide-border/60">
        {#each filteredDownloads as download, i (download.id)}
          <DownloadItem
            {download}
            {startDownload}
            {cancelDownload}
            selected={isSelected(download.id)}
            onToggleSelect={(payload) =>
              toggleSelectWithIndex(download.id, payload?.checked ?? false, i, !!payload?.shiftKey)}
          />
        {/each}
        {#if filteredDownloads.length === 0}
          <div class="space-y-3 px-6 py-12 text-center text-sm text-muted-foreground">
            <p>No downloads match the current filters.</p>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onclick={() => {
                handleClearFilters();
                statusGroup = 'all';
              }}
            >
              Reset filters
            </Button>
          </div>
        {/if}
      </div>
    </CardContent>
  </Card>

  <Dialog bind:open={showHelp}>
    <DialogContent>
      <DialogHeader>
        <DialogTitle>Keyboard shortcuts</DialogTitle>
        <DialogDescription>Speed up triage with quick actions.</DialogDescription>
      </DialogHeader>
      <div class="grid gap-2 text-sm">
        <p><strong>Ctrl/Cmd + A:</strong> Select all filtered downloads</p>
        <p><strong>Shift + Click:</strong> Range select</p>
        <p><strong>Enter:</strong> Start or cancel the focused row</p>
        <p><strong>Delete / Backspace:</strong> Cancel selected active downloads</p>
        <p><strong>Esc:</strong> Clear selection</p>
      </div>
      <DialogFooter>
        <Button type="button" variant="secondary" onclick={() => (showHelp = false)}>Close</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</div>
