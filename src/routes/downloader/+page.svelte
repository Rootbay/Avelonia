<script lang="ts">
  import { downloads, addDownload, removeDownloadsByIds } from '$lib/downloads';
  import FilterPanel from '$lib/components/FilterPanel.svelte';
  import DownloadItem from '$lib/components/DownloadItem.svelte';
  import { startDownload, cancelDownload, getDownloadPath } from '$lib/downloadManager';
  import type { Download } from '$lib/downloadManager';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { pushLog as pushSystemLog, type LogLevel } from '$lib/logStore';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from '$lib/components/ui/card';
  import { Badge } from '$lib/components/ui/badge';
  import { Separator } from '$lib/components/ui/separator';
  import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
  import {
    Table,
    TableHeader,
    TableRow,
    TableHead,
    TableBody,
    TableCell,
  } from '$lib/components/ui/table';
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
  import { Play, XCircle, RefreshCcw, Clipboard as ClipboardIcon, FileDown, Eye, FolderOpen, Trash2 } from '@lucide/svelte';
  import { sanitizeFileName, normalizeExtension } from '$lib/downloadPath';
  import { prettifyDisplayName } from '$lib/name';
  import { tags as customTags, addTag, BUILT_IN_TAGS } from '$lib/tags';
  import { invoke } from '@tauri-apps/api/core';

  let searchTerm = $state('');
  let debouncedSearchTerm = $state('');
  $effect(() => {
    const t = setTimeout(() => {
      debouncedSearchTerm = searchTerm;
    }, 150);
    return () => clearTimeout(t);
  });
  let showFilters = $state(false);
  let filtersOpen = $state(false);
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

  type ButtonSnippetContext = { props?: Record<string, unknown> & { class?: string } };

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
  let tableEl: HTMLTableElement | null = null;
  let maxListHeight = $state(0);
  let addOpen = $state(false);
  let addUrl = $state('');
  let addCategory = $state('');
  let addName = $state('');
  let nameTouched = $state(false);
  const tagOptions = $derived([...new Set([...BUILT_IN_TAGS, ...$customTags])]);
  const guessed = $derived.by(() => guessFromUrl(addUrl));
  let probeName = $state('');
  let probeExt = $state('');
  let probeSize = $state(0);
  let probing = $state(false);
  const effectiveExt = $derived(probeExt || guessed.ext);
  const effectiveName = $derived(prettifyDisplayName(probeName || guessed.name, effectiveExt));

  // Keep suggested name in sync until user edits the field
  $effect(() => {
    if (!nameTouched) {
      addName = effectiveName;
    }
  });

  // Probe server headers for better filename/extension when URL looks valid
  $effect(() => {
    const u = addUrl.trim();
    if (!/^https?:\/\//i.test(u)) {
      probeName = '';
      probeExt = '';
      return;
    }
    const t = setTimeout(async () => {
      try {
        probing = true;
        const res = (await invoke('probe_download', { url: u })) as { filename?: string; ext?: string; size?: number };
        probeName = res?.filename || '';
        probeExt = res?.ext || '';
        probeSize = typeof res?.size === 'number' ? (res!.size as number) : 0;
      } catch {
        probeName = '';
        probeExt = '';
        probeSize = 0;
      } finally {
        probing = false;
      }
    }, 250);
    return () => clearTimeout(t);
  });

  function guessFromUrl(url: string): { name: string; ext: string } {
    try {
      const u = new URL(url);
      let last = decodeURIComponent(u.pathname.split('/').filter(Boolean).pop() || '');
      // Strip query-like suffixes sometimes embedded in filename
      last = last.replace(/[?#].*$/, '');
      if (!last) return { name: 'download', ext: '' };
      // Try multi-dot extensions first
      const lower = last.toLowerCase();
      const multi = ['.tar.gz', '.tar.bz2', '.tar.xz', '.tar.zst'];
      for (const m of multi) {
        if (lower.endsWith(m)) {
          const base = last.slice(0, -m.length);
          const ext = m.slice(1);
          return { name: sanitizeFileName(base), ext };
        }
      }
      const idx = last.lastIndexOf('.');
      if (idx > 0 && idx < last.length - 1) {
        const base = last.slice(0, idx);
        const ext = last.slice(idx + 1);
        return { name: sanitizeFileName(base), ext };
      }
      return { name: sanitizeFileName(last), ext: '' };
    } catch {
      return { name: 'download', ext: '' };
    }
  }

  function addNewDownload() {
    const url = addUrl.trim();
    if (!url) {
      toast.error('Enter a valid URL');
      return;
    }
    if (!/^https?:\/\//i.test(url)) {
      toast.error('Only http(s) URLs are supported');
      return;
    }
    const name = (addName || '').trim() || effectiveName;
    const ext = effectiveExt;
    const normExt = normalizeExtension(ext).replace(/^\./, '');
    addDownload({
      name,
      description: '',
      size: probeSize > 0 ? formatBytes(probeSize) : 'N/A',
      fileType: normExt,
      category: addCategory.trim() || 'General',
      tags: addCategory.trim() ? [addCategory.trim()] : ['General'],
      downloadLink: url,
    });
    const cat = addCategory.trim();
    if (cat && !tagOptions.includes(cat)) addTag(cat);
    addOpen = false;
    addUrl = '';
    addCategory = '';
    toast.success('Added to list');
  }

  const activeFilterCount = $derived.by(() => {
    let c = 0;
    if (debouncedSearchTerm.trim()) c += 1;
    if (filters.fileType) c += 1;
    if (filters.category) c += 1;
    if (filters.minSize) c += 1;
    if (filters.maxSize) c += 1;
    if (filters.eta) c += 1;
    if (filters.status) c += 1;
    if (statusGroup !== 'all') c += 1;
    return c;
  });

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

  // Uncheck items automatically when they complete
  $effect(() => {
    let changed = false;
    for (const d of $downloads) {
      if (d.status === 'completed' && selectedIds.has(d.id)) {
        selectedIds.delete(d.id);
        changed = true;
      }
    }
    if (changed) selectedIds = new Set(selectedIds);
  });

  // Utility to send app logs to Dashboard via Tauri event bus
  async function appLog(level: LogLevel, message: string) {
    try {
      // Reduce noise: only persist WARN/ERROR/SUCCESS; ignore INFO here
      if (level === 'INFO') return;
      pushSystemLog(level, message, 'Downloader');
    } catch {}
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

  function recomputeMaxHeight() {
    try {
      const header = tableEl?.querySelector('thead') as HTMLElement | null;
      const firstRow = tableEl?.querySelector('tbody tr') as HTMLElement | null;
      const headerH = header ? header.getBoundingClientRect().height : 44;
      const rowH = firstRow ? firstRow.getBoundingClientRect().height : 56;
      const cap = Math.ceil(headerH + rowH * 10 + 6); // header + 10 rows + small buffer
      maxListHeight = cap;
    } catch {
      maxListHeight = 0;
    }
  }

  $effect(() => {
    // Recalculate when the filtered list length changes
    const _len = filteredDownloads.length;
    // Next tick to allow DOM to render
    setTimeout(recomputeMaxHeight, 0);
  });

  onMount(() => {
    const onR = () => recomputeMaxHeight();
    window.addEventListener('resize', onR);
    // initial calculation
    setTimeout(onR, 0);
    return () => window.removeEventListener('resize', onR);
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

  // Action counts for nicer Bulk actions UI
  const startableAll = $derived($downloads.filter((d) => d.status === 'available' && !!d.downloadLink).length);
  const cancelableAll = $derived($downloads.filter((d) => d.status === 'downloading' || d.status === 'pending' || d.status === 'queued').length);
  const startableFiltered = $derived(filteredDownloads.filter((d) => d.status === 'available' && !!d.downloadLink).length);
  const cancelableFiltered = $derived(filteredDownloads.filter((d) => d.status === 'downloading' || d.status === 'pending' || d.status === 'queued').length);
  const startableSelected = $derived(filteredDownloads.filter((d) => selectedIds.has(d.id) && d.status === 'available' && !!d.downloadLink).length);
  const cancelableSelected = $derived(filteredDownloads.filter((d) => selectedIds.has(d.id) && (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued')).length);
  const failedFiltered = $derived(filteredDownloads.filter((d) => d.status === 'failed').length);
  const deletableSelected = $derived(filteredDownloads.filter((d) => selectedIds.has(d.id)).length);
  const deletableFiltered = $derived(filteredDownloads.length);

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
    void appLog('INFO', 'Queued all available downloads');
  }

  function cancelAllActive() {
    const list = get(downloads);
    let canceled = 0;
    for (const d of list) {
      if (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued') {
        cancelDownload(d.id);
        canceled += 1;
      }
    }
    if (canceled > 0) void appLog('INFO', `Canceled ${canceled} active downloads`);
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
      void appLog('INFO', `Queued ${queued} download(s) from current filter`);
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
      void appLog('INFO', `Canceled ${canceled} download(s) from current filter`);
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
      void appLog('INFO', `Queued ${queued} selected download(s)`);
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
      void appLog('INFO', `Canceled ${canceled} selected download(s)`);
    } else {
      toast.info('No active selected downloads to cancel');
    }
  }

  function deleteSelected() {
    const ids = filteredDownloads.filter((d) => selectedIds.has(d.id)).map((d) => d.id);
    if (ids.length === 0) {
      toast.info('No selected downloads to delete');
      return;
    }
    // Cancel active ones before removing
    for (const d of filteredDownloads) {
      if (selectedIds.has(d.id) && (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued')) {
        cancelDownload(d.id);
      }
    }
    removeDownloadsByIds(ids);
    for (const id of ids) selectedIds.delete(id);
    selectedIds = new Set(selectedIds);
    toast.success(`Deleted ${ids.length} download${ids.length === 1 ? '' : 's'}`);
    void appLog('WARN', `Deleted ${ids.length} selected download(s)`);
  }

  function deleteFiltered() {
    const ids = filteredDownloads.map((d) => d.id);
    if (ids.length === 0) {
      toast.info('No filtered downloads to delete');
      return;
    }
    for (const d of filteredDownloads) {
      if (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued') {
        cancelDownload(d.id);
      }
    }
    removeDownloadsByIds(ids);
    for (const id of ids) selectedIds.delete(id);
    selectedIds = new Set(selectedIds);
    toast.success(`Deleted ${ids.length} download${ids.length === 1 ? '' : 's'}`);
    void appLog('WARN', `Deleted ${ids.length} filtered download(s)`);
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
      void appLog('INFO', `Opened ${opened} completed download(s)`);
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
      void appLog('INFO', `Revealed ${revealed} completed download(s) in explorer`);
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
      void appLog('INFO', `Retrying ${retried} failed download(s) in current view`);
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
      void appLog('INFO', `Retrying ${retried} failed download(s)`);
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

  <Card class="bg-card/80 shadow-sm">
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

  <Dialog bind:open={addOpen}>
    <DialogContent>
      <DialogHeader>
        <DialogTitle>Add download</DialogTitle>
        <DialogDescription>Paste a direct download link and choose a category.</DialogDescription>
      </DialogHeader>
      <div class="grid gap-3">
        <div class="space-y-1">
          <label class="text-sm font-medium" for="add-url">URL</label>
          <Input id="add-url" placeholder="https://example.com/file.exe" bind:value={addUrl} />
        </div>
        <div class="space-y-1">
          <label class="text-sm font-medium" for="add-cat">Category</label>
          <Input id="add-cat" placeholder="e.g. Utilities" bind:value={addCategory} list="category-options" />
          <datalist id="category-options">
            {#each tagOptions as t}
              <option value={t}></option>
            {/each}
          </datalist>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1">
            <label class="text-sm font-medium" for="add-name">Name</label>
            <Input id="add-name" placeholder="App name" bind:value={addName} oninput={() => (nameTouched = true)} />
            {#if !nameTouched}
              <p class="text-xs text-muted-foreground">Suggested: {effectiveName}{#if probing} (detecting...){/if}</p>
            {/if}
          </div>
          <div class="space-y-1">
            <div class="text-sm font-medium">Type</div>
            <div class="text-sm text-foreground">{normalizeExtension(effectiveExt) || '—'}</div>
          </div>
        </div>
      </div>
      <DialogFooter>
        <Button type="button" variant="secondary" onclick={() => (addOpen = false)}>Cancel</Button>
        <Button type="button" onclick={addNewDownload}>Add</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>

  <Card class="bg-card/70 shadow-sm">
    <CardHeader class="gap-4 pb-2">
      <div class="flex flex-col gap-3">
          <div class="flex flex-wrap items-center gap-3">
            <div class="flex min-w-[220px] flex-1 items-center gap-2">
              <Input class="w-full" placeholder="Search downloads..." bind:value={searchTerm} />
              <Button
                type="button"
                variant="outline"
                onclick={() => (filtersOpen = true)}
              >
                <SlidersHorizontal class="size-4" />
                <span class="ml-2 hidden sm:inline">Filters</span>
              </Button>
              <Button type="button" variant="secondary" onclick={() => (addOpen = true)}>
                <span class="hidden sm:inline">Add</span>
                <span class="sm:hidden">+</span>
              </Button>
              {#snippet ActionsTrigger({ props }: ButtonSnippetContext)}
                {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
                {@const { class: propsClass, ...restWithoutClass } = rawProps}
                {@const restProps = restWithoutClass as Record<string, unknown>}
                <Button
                  {...restProps}
                  type="button"
                  variant="secondary"
                  class={propsClass}
                >
                  <ListChecks class="size-4" />
                  <span class="ml-2 hidden sm:inline">Actions</span>
                </Button>
              {/snippet}
            <Sheet bind:open={actionsOpen}>
              <SheetTrigger child={ActionsTrigger} />
              <SheetContent side="right" class="w-[340px] sm:w-[380px] p-4 sm:p-6">
                <SheetHeader class="space-y-1 p-0">
                  <SheetTitle>Bulk actions</SheetTitle>
                  <SheetDescription
                    >Apply actions to the current selection or filtered list.</SheetDescription
                  >
                </SheetHeader>
                <div class="mt-3 space-y-3">
                  <div class="space-y-2">
                    <p class="text-xs uppercase tracking-wide text-muted-foreground">All downloads</p>
                    <div class="grid gap-2">
                      <Button type="button" class="justify-between" onclick={() => { startAll(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><Play class="size-4" /> Start all</span>
                        {#if startableAll > 0}<Badge variant="secondary">{startableAll}</Badge>{/if}
                      </Button>
                      <Button type="button" variant="destructive" class="justify-between" onclick={() => { cancelAllActive(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><XCircle class="size-4" /> Cancel active</span>
                        {#if cancelableAll > 0}<Badge variant="secondary">{cancelableAll}</Badge>{/if}
                      </Button>
                    </div>
                  </div>
                  <Separator />
                  <div class="space-y-2">
                    <p class="text-xs uppercase tracking-wide text-muted-foreground">Current view</p>
                    <div class="grid gap-2">
                      <Button type="button" variant="outline" class="justify-between" disabled={startableFiltered === 0} onclick={() => { startAllFiltered(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><Play class="size-4" /> Start filtered</span>
                        {#if startableFiltered > 0}<Badge variant="secondary">{startableFiltered}</Badge>{/if}
                      </Button>
                      <Button type="button" variant="destructive" class="justify-between" disabled={cancelableFiltered === 0} onclick={() => { cancelAllFiltered(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><XCircle class="size-4" /> Cancel filtered</span>
                        {#if cancelableFiltered > 0}<Badge variant="secondary">{cancelableFiltered}</Badge>{/if}
                      </Button>
                      <Button type="button" variant="destructive" class="justify-between" disabled={deletableFiltered === 0} onclick={() => { deleteFiltered(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><Trash2 class="size-4" /> Delete filtered</span>
                        {#if deletableFiltered > 0}<Badge variant="secondary">{deletableFiltered}</Badge>{/if}
                      </Button>
                      <Button type="button" variant="outline" class="justify-between" disabled={failedFiltered === 0} onclick={() => { retryFailedFiltered(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><RefreshCcw class="size-4" /> Retry failed (filtered)</span>
                        {#if failedFiltered > 0}<Badge variant="secondary">{failedFiltered}</Badge>{/if}
                      </Button>
                      <Button type="button" variant="ghost" class="justify-between" onclick={() => { exportFilteredCSV(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><FileDown class="size-4" /> Export CSV</span>
                      </Button>
                    </div>
                  </div>
                  <Separator />
                  <div class="space-y-2">
                    <p class="text-xs uppercase tracking-wide text-muted-foreground">Selected</p>
                    <div class="grid gap-2">
                      <Button type="button" class="justify-between" disabled={startableSelected === 0} onclick={() => { startSelected(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><Play class="size-4" /> Start selected</span>
                        {#if startableSelected > 0}<Badge variant="secondary">{startableSelected}</Badge>{/if}
                      </Button>
                      <Button type="button" variant="destructive" class="justify-between" disabled={cancelableSelected === 0} onclick={() => { cancelSelected(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><XCircle class="size-4" /> Cancel selected</span>
                        {#if cancelableSelected > 0}<Badge variant="secondary">{cancelableSelected}</Badge>{/if}
                      </Button>
                      <Button type="button" variant="destructive" class="justify-between" disabled={deletableSelected === 0} onclick={() => { deleteSelected(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><Trash2 class="size-4" /> Delete selected</span>
                        {#if deletableSelected > 0}<Badge variant="secondary">{deletableSelected}</Badge>{/if}
                      </Button>
                      <div class="grid grid-cols-2 gap-2">
                        <Button type="button" variant="outline" class="justify-center" disabled={selectedCompletedCount === 0} onclick={() => { openSelectedCompleted(); actionsOpen = false; }}>
                          <Eye class="size-4" />
                          <span class="ml-2">Open</span>
                        </Button>
                        <Button type="button" variant="outline" class="justify-center" disabled={selectedCompletedCount === 0} onclick={() => { showSelectedCompleted(); actionsOpen = false; }}>
                          <FolderOpen class="size-4" />
                          <span class="ml-2">Show</span>
                        </Button>
                      </div>
                      <div class="grid grid-cols-2 gap-2">
                        <Button type="button" variant="ghost" class="justify-center" disabled={selectedIds.size === 0} onclick={() => { copySelectedLinks(); actionsOpen = false; }}>
                          <ClipboardIcon class="size-4" />
                          <span class="ml-2">Copy links</span>
                        </Button>
                        <Button type="button" variant="ghost" class="justify-center" onclick={() => { exportFilteredCSV(); actionsOpen = false; }}>
                          <FileDown class="size-4" />
                          <span class="ml-2">Export</span>
                        </Button>
                      </div>
                    </div>
                  </div>
                  <Separator />
                  <div class="space-y-2">
                    <p class="text-xs uppercase tracking-wide text-muted-foreground">Failed</p>
                    <div class="grid gap-2">
                      <Button type="button" variant="outline" class="justify-between" disabled={failedCount === 0} onclick={() => { retryAllFailed(); actionsOpen = false; }}>
                        <span class="flex items-center gap-2"><RefreshCcw class="size-4" /> Retry all failed</span>
                        {#if failedCount > 0}<Badge variant="secondary">{failedCount}</Badge>{/if}
                      </Button>
                    </div>
                  </div>
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

  <Sheet bind:open={filtersOpen}>
    <SheetContent side="right" class="sm:max-w-md">
      <SheetHeader>
        <SheetTitle>Filters</SheetTitle>
        <SheetDescription>Refine downloads list</SheetDescription>
      </SheetHeader>
      <div class="mt-4">
        <FilterPanel
          bind:searchTerm
          filters={filters}
          showFilters={true}
          onClearFilters={() => {
            searchTerm = '';
            filters = { fileType: '', category: '', minSize: '', maxSize: '', eta: '', status: '' };
          }}
        />
      </div>
    </SheetContent>
  </Sheet>

  <Card class="relative overflow-hidden border border-border/60">

    <CardContent class="p-0">
      <div class="overflow-auto" style:max-height={`${maxListHeight || ''}px`}>
        <Table ref={tableEl} class="min-w-[960px]">
          <TableHeader>
            <TableRow>
              <TableHead class="w-[40px]">
                <span class="flex justify-center">
                  <input
                    bind:this={selectAllCheckbox}
                    type="checkbox"
                    class="h-4 w-4 rounded border-border bg-background text-primary focus:ring-ring"
                    checked={filteredDownloads.length > 0 && filteredDownloads.every((d) => selectedIds.has(d.id))}
                    aria-checked={(() => {
                      const any = filteredDownloads.some((d) => selectedIds.has(d.id));
                      const all = any && filteredDownloads.every((d) => selectedIds.has(d.id));
                      return all ? 'true' : any ? 'mixed' : 'false';
                    })()}
                    onchange={(e) => {
                      const check = (e.target as HTMLInputElement).checked;
                      for (const d of filteredDownloads) toggleSelect(d.id, check);
                    }}
                    title="Select or deselect all filtered downloads"
                  />
                </span>
              </TableHead>
              <TableHead>
                <button
                  type="button"
                  class="flex items-center gap-1 text-left transition hover:text-foreground focus-visible:outline-none"
                  onclick={() => setSort('name')}
                  onkeydown={(event) => handleHeaderKey(event, 'name')}
                >
                  <span>Name</span>
                  {#if sortBy === 'name'}<ArrowUpDown class={`size-3 ${sortDirection === 'asc' ? 'rotate-180' : ''}`} />{/if}
                </button>
              </TableHead>
              <TableHead>
                <button type="button" class="flex items-center gap-1 text-left transition hover:text-foreground focus-visible:outline-none" onclick={() => setSort('size')} onkeydown={(event) => handleHeaderKey(event, 'size')}>
                  <span>Size</span>
                  {#if sortBy === 'size'}<ArrowUpDown class={`size-3 ${sortDirection === 'asc' ? 'rotate-180' : ''}`} />{/if}
                </button>
              </TableHead>
              <TableHead>
                <button type="button" class="flex items-center gap-1 text-left transition hover:text-foreground focus-visible:outline-none" onclick={() => setSort('fileType')} onkeydown={(event) => handleHeaderKey(event, 'fileType')}>
                  <span>File Type</span>
                  {#if sortBy === 'fileType'}<ArrowUpDown class={`size-3 ${sortDirection === 'asc' ? 'rotate-180' : ''}`} />{/if}
                </button>
              </TableHead>
              <TableHead>
                <button type="button" class="flex items-center gap-1 text-left transition hover:text-foreground focus-visible:outline-none" onclick={() => setSort('category')} onkeydown={(event) => handleHeaderKey(event, 'category')}>
                  <span>Category</span>
                  {#if sortBy === 'category'}<ArrowUpDown class={`size-3 ${sortDirection === 'asc' ? 'rotate-180' : ''}`} />{/if}
                </button>
              </TableHead>
              <TableHead>
                <button type="button" class="flex items-center gap-1 text-left transition hover:text-foreground focus-visible:outline-none" onclick={() => setSort('eta')} onkeydown={(event) => handleHeaderKey(event, 'eta')}>
                  <span>ETA</span>
                  {#if sortBy === 'eta'}<ArrowUpDown class={`size-3 ${sortDirection === 'asc' ? 'rotate-180' : ''}`} />{/if}
                </button>
              </TableHead>
              <TableHead class="w-[180px] pl-6 sm:pl-8">
                <button type="button" class="flex items-center gap-1 text-left transition hover:text-foreground focus-visible:outline-none" onclick={() => setSort('status')} onkeydown={(event) => handleHeaderKey(event, 'status')}>
                  <span>Status</span>
                  {#if sortBy === 'status'}<ArrowUpDown class={`size-3 ${sortDirection === 'asc' ? 'rotate-180' : ''}`} />{/if}
                </button>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {#each filteredDownloads as download, i (download.id)}
              <DownloadItem
                {download}
                {startDownload}
                {cancelDownload}
                selected={isSelected(download.id)}
                onToggleSelect={(payload: { checked: boolean; shiftKey: boolean }) => 
                  toggleSelectWithIndex(download.id, payload?.checked ?? false, i, !!payload?.shiftKey)}
              />
            {/each}
            {#if filteredDownloads.length === 0}
              <TableRow>
                <TableCell colspan={7} class="py-12 text-center text-sm text-muted-foreground">
                  No downloads match the current filters.
                </TableCell>
              </TableRow>
            {/if}
          </TableBody>
        </Table>
      </div>

      {#if selectedIds.size > 0}
        <div class="pointer-events-none">
          <div class="pointer-events-auto md:absolute md:bottom-4 md:right-4 fixed bottom-6 right-6 z-20 rounded-md border bg-card/95 backdrop-blur p-2 shadow-md flex items-center gap-2">
            <Badge variant="secondary">{selectedIds.size} selected</Badge>
            <Button size="sm" onclick={startSelected}>Start</Button>
            <Button size="sm" variant="destructive" onclick={cancelSelected}>Cancel</Button>
            <Button size="sm" variant="ghost" onclick={() => (actionsOpen = true)}>More</Button>
          </div>
        </div>
      {/if}

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














