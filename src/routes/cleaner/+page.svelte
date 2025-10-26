<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { slide } from 'svelte/transition';

  import { Button } from '$lib/components/ui/button';
  import {
    Card,
    CardHeader,
    CardTitle,
    CardDescription,
    CardContent,
  } from '$lib/components/ui/card';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Label } from '$lib/components/ui/label';
  import { Input } from '$lib/components/ui/input';
  import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogDescription,
    DialogFooter,
  } from '$lib/components/ui/dialog';
  import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
  } from '$lib/components/ui/alert-dialog';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import { Separator } from '$lib/components/ui/separator';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { toast } from '$lib/components/ui/sonner';
  import { Tabs, TabsList, TabsTrigger, TabsContent } from '$lib/components/ui/tabs';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';

  import {
    Trash2,
    RefreshCw,
    Scan,
    HardDrive,
    FolderOpen,
    Files as FilesIcon,
    Eraser,
    MoreHorizontal,
  } from '@lucide/svelte';

  interface FileEntry {
    path: string;
    size?: number;
  }

  let tempFiles = $state<FileEntry[]>([]);
  let largeFiles = $state<FileEntry[]>([]);
  let duplicateFiles = $state<FileEntry[]>([]);
  let emptyFolders = $state<FileEntry[]>([]);
  let brokenShortcuts = $state<FileEntry[]>([]);

  let selectedTempFiles = $state<string[]>([]);
  let selectedLargeFiles = $state<string[]>([]);
  let selectedDuplicateFiles = $state<string[]>([]);
  let selectedEmptyFolders = $state<string[]>([]);
  let selectedBrokenShortcuts = $state<string[]>([]);

  let message = $state('');
  let progressMessage = $state('');
  let isLoading = (false);
  let scanning = $state(false);

  let eraserSelectedFiles = $state<string[]>([]);
  let eraserPasses = $state(1);
  let isErasing = $state(false);
  let eraserMessage = $state('');

  let showConfirmationModal = $state(false);
  let showRecycleBinConfirmationModal = $state(false);
  let filesToDelete = $state<string[]>([]);

  let totalDiskSpace = $state<number | null>(null);
  let availableDiskSpace = $state<number | null>(null);

  // Unified view state and settings
  type Kind = 'temp' | 'large' | 'duplicate' | 'empty' | 'shortcut';
  let view = $state<'unified' | 'tabs'>('unified');
  let q = $state('');
  let filterKind = $state<'all' | Kind>('all');
  let largeMinMB = $state(100);
  let dupGroups = $state<Array<{ hash: string; size: number; files: string[] }>>([]);
  let selectedPaths = $state(new Set<string>());
  let showSettings = $state(false);
  let exclusions = $state<string[]>([]);
  const EXC_KEY = 'avelonia_cleaner_exclusions_v1';
  function loadExclusions() {
    try { const raw = localStorage.getItem(EXC_KEY); exclusions = raw ? JSON.parse(raw) : []; } catch { exclusions = []; }
  }
  function saveExclusions() {
    try { localStorage.setItem(EXC_KEY, JSON.stringify(Array.from(new Set(exclusions.map((s)=>s.trim()).filter(Boolean))))); } catch {}
  }
  onMount(() => { loadExclusions(); });
  function addExclusion(pattern: string) {
    const p = (pattern||'').trim(); if (!p) return; exclusions = Array.from(new Set([...exclusions, p])); saveExclusions();
  }
  function removeExclusion(pattern: string) { exclusions = exclusions.filter((s)=>s!==pattern); saveExclusions(); }

  function matchesExclusion(p: string): boolean {
    try {
      for (const ex of exclusions) { if (!ex) continue; if (p.toLowerCase().includes(ex.toLowerCase())) return true; }
    } catch {}
    return false;
  }

  type CleanerItem = { path: string; size?: number; kind: Kind; groupId?: string };
  const allItems = $derived.by<CleanerItem[]>(() => {
    const items: CleanerItem[] = [];
    for (const f of tempFiles) items.push({ path: f.path, size: f.size, kind: 'temp' });
    for (const f of largeFiles) items.push({ path: f.path, size: f.size, kind: 'large' });
    for (const f of duplicateFiles) items.push({ path: f.path, size: f.size, kind: 'duplicate' });
    for (const f of emptyFolders) items.push({ path: f.path, size: f.size, kind: 'empty' });
    for (const f of brokenShortcuts) items.push({ path: f.path, size: f.size, kind: 'shortcut' });
    // integrate groups if present (ensure duplicates existing list contains them)
    if (dupGroups.length > 0) {
      for (const g of dupGroups) {
        for (const p of g.files) items.push({ path: p, size: g.size, kind: 'duplicate', groupId: g.hash });
      }
    }
    const term = q.trim().toLowerCase();
    return items.filter((it) => {
      if (matchesExclusion(it.path)) return false;
      if (filterKind !== 'all' && it.kind !== filterKind) return false;
      if (term && !it.path.toLowerCase().includes(term)) return false;
      return true;
    });
  });

  const selectedCount = $derived(selectedPaths.size);
  const selectedSize = $derived.by(() => {
    let sum = 0; const s = selectedPaths; for (const it of allItems) { if (s.has(it.path)) sum += it.size ?? 0; } return sum;
  });

  // Streaming + truncation for temp list
  const MAX_TEMP_ITEMS = 20000;
  let tempTruncated = $state(false);
  let tempReportedTotal = $state(0);
  let tempQueue: string[] = [];
  let tempFlushRaf: number | null = null;
  function scheduleTempFlush() {
    if (tempFlushRaf !== null) return;
    tempFlushRaf = requestAnimationFrame(() => {
      tempFlushRaf = null;
      if (tempQueue.length === 0) return;
      const take = tempQueue.splice(0, Math.min(800, tempQueue.length));
      const next = take.filter((p) => !matchesExclusion(p)).map((p) => ({ path: p }));
      if (next.length) {
        const remaining = Math.max(0, MAX_TEMP_ITEMS - tempFiles.length);
        if (remaining <= 0) {
          tempTruncated = true;
        } else {
          const append = next.slice(0, remaining);
          tempFiles = [...tempFiles, ...append];
          if (append.length < next.length) tempTruncated = true;
        }
      }
      if (tempQueue.length > 0) scheduleTempFlush();
    });
  }

  // Virtualization for unified table
  let unifiedContainer: HTMLDivElement | null = null;
  const UNIFIED_ROW_PX = 40;
  const UNIFIED_MAX_DOM = 600;
  let unifiedStart = $state(0);
  const unifiedRowsInView = $derived(() => {
    const h = unifiedContainer?.clientHeight ?? 480;
    return Math.ceil(h / UNIFIED_ROW_PX) + 20;
  });
  const unifiedDisplayed = $derived(() => {
    const end = Math.min(allItems.length, unifiedStart + Math.min(unifiedRowsInView, UNIFIED_MAX_DOM));
    return allItems.slice(unifiedStart, end);
  });
  const unifiedTopPad = $derived(unifiedStart * UNIFIED_ROW_PX);
  const unifiedBottomPad = $derived(Math.max(0, (allItems.length - (unifiedStart + unifiedDisplayed.length)) * UNIFIED_ROW_PX));
  function onUnifiedScroll(event: Event) {
    const el = event.currentTarget as HTMLElement;
    const first = Math.floor(el.scrollTop / UNIFIED_ROW_PX) - 3; // small lookahead, avoid jumpiness
    unifiedStart = Math.max(0, first);
  }

  // Tabs view: make the tab selection controlled so clicking the triggers actually switches content
  type CleanerTab = 'quick' | 'temp' | 'large' | 'dup' | 'empty' | 'shortcuts' | 'erase' | 'disk';
  let activeTab = $state<CleanerTab>('quick');

  async function statTempSizes() {
    try {
      const paths = tempFiles.map((f) => f.path);
      if (paths.length === 0) return;
      const res = (await invoke('stat_paths', { paths })) as [string, number][];
      const map = new Map(res);
      tempFiles = tempFiles.map((f) => ({ path: f.path, size: map.get(f.path) ?? f.size }));
    } catch {}
  }

  async function scanAll() {
    scanning = true; progressMessage = ''; message = '';
    try {
      const minBytes = Math.max(1, largeMinMB) * 1024 * 1024;
      const tasks = [
        (async () => { tempFiles = []; selectedTempFiles = []; await invoke('get_temp_files_stream', { batch_size: 250, max: MAX_TEMP_ITEMS }); await statTempSizes(); return 'temp:ok'; })(),
        (async () => { const r = (await invoke('find_large_files_min', { min_size_bytes: minBytes })) as [string, number][]; largeFiles = r.map(([p,s])=>({ path: p, size: s })); return 'large:ok'; })(),
        (async () => { const g = (await invoke('find_duplicate_groups')) as Array<{ hash: string; size: number; files: string[] }>; dupGroups = g; duplicateFiles = []; return 'dup:ok'; })(),
        (async () => { const r = (await invoke('find_empty_folders')) as string[]; emptyFolders = r.map((p)=>({ path: p })); return 'empty:ok'; })(),
        (async () => { const r = (await invoke('find_broken_shortcuts')) as string[]; brokenShortcuts = r.map((p)=>({ path: p })); return 'shortcuts:ok'; })(),
      ];
      const results = await Promise.allSettled(tasks);
      const failed = results.filter((r) => r.status === 'rejected').length;
      if (failed > 0) {
        toast.warning(`Scan complete with ${failed} error(s)`);
      } else {
        toast.success('Scan complete');
      }
      // Unified view: show all results by default after a full scan
      if (view === 'unified') {
        filterKind = 'all';
      }
    } catch (e) {
      console.error(e); toast.error('Scan failed');
    } finally { scanning = false; progressMessage = ''; }
  }

  function toggleSelectUnified(p: string) {
    const next = new Set(selectedPaths); if (next.has(p)) next.delete(p); else next.add(p); selectedPaths = next;
  }

  function clearSelectionUnified() { selectedPaths = new Set(); }

  function setSelectionForKind(kind: 'all' | Kind) {
    const next = new Set(selectedPaths);
    for (const it of allItems) { if (kind === 'all' || it.kind === kind) next.add(it.path); }
    selectedPaths = next;
  }

  async function deleteSelectedUnified() {
    const files = Array.from(selectedPaths); if (files.length === 0) return; filesToDelete = files; showConfirmationModal = true;
  }

  async function moveSelectedUnified() {
    const files = Array.from(selectedPaths); if (files.length === 0) return;
    try {
      const dest = await open({ directory: true });
      if (!dest || typeof dest !== 'string') return;
      isLoading = true; message = 'Moving selected...';
      const moved = (await invoke('move_files', { files, destination: dest })) as number;
      // remove moved from lists
      tempFiles = tempFiles.filter((f)=>!selectedPaths.has(f.path));
      largeFiles = largeFiles.filter((f)=>!selectedPaths.has(f.path));
      duplicateFiles = duplicateFiles.filter((f)=>!selectedPaths.has(f.path));
      emptyFolders = emptyFolders.filter((f)=>!selectedPaths.has(f.path));
      brokenShortcuts = brokenShortcuts.filter((f)=>!selectedPaths.has(f.path));
      for (const g of dupGroups) { g.files = g.files.filter((p)=>!selectedPaths.has(p)); }
      dupGroups = dupGroups.filter((g)=>g.files.length>1);
      clearSelectionUnified();
      toast.success(`Moved ${moved} item(s)`);
    } catch (e) {
      console.error(e); toast.error('Move failed');
    } finally { isLoading = false; }
  }

  async function secureEraseSelectedUnified() {
    const files = Array.from(selectedPaths); if (files.length === 0) return;
    try { isErasing = true; await invoke('secure_erase', { files, passes: eraserPasses }); toast.success('Secure erase done'); clearSelectionUnified(); }
    catch(e){ console.error(e); toast.error('Secure erase failed'); }
    finally { isErasing = false; }
  }

  function autoSelectDuplicatesKeepOne() {
    if (dupGroups.length === 0) return;
    const next = new Set(selectedPaths);
    for (const g of dupGroups) {
      if (!g.files || g.files.length < 2) continue;
      const sorted = [...g.files].sort();
      for (let i = 1; i < sorted.length; i++) next.add(sorted[i]); // keep first, select rest
    }
    selectedPaths = next;
    filterKind = 'duplicate';
  }

  onMount(() => {
    const unsubs: Array<() => void> = [];

    listen('scan_progress', (event) => {
      if ((window.performance?.now?.() ?? Date.now()) - (window.__lastProg||0) > 300) { progressMessage = event.payload as string; (window as any).__lastProg = window.performance?.now?.() ?? Date.now(); }
    }).then((fn) => unsubs.push(fn)).catch(() => {});

    listen('cleaner-temp-batch', (event) => {
      try {
        const arr = (event.payload as string[]) || [];
        if (Array.isArray(arr) && arr.length) {
          for (const p of arr) {
            if (tempFiles.length + tempQueue.length >= MAX_TEMP_ITEMS) { tempTruncated = true; break; }
            tempQueue.push(String(p));
          }
          scheduleTempFlush();
        }
      } catch {}
    }).then((fn) => unsubs.push(fn)).catch(() => {});

        listen('cleaner-temp-done', (event) => {
      try { const payload = event.payload as { total?: number }; tempReportedTotal = Number(payload?.total||0); } catch { tempReportedTotal = 0; }
      // Final synchronous drain in case rAF never got a chance to flush the queue
      if (tempQueue.length > 0) {
        while (tempQueue.length > 0) {
          const take = tempQueue.splice(0, Math.min(2000, tempQueue.length));
          const next = take.filter((p) => !matchesExclusion(p)).map((p) => ({ path: p }));
          if (next.length) {
            const remaining = Math.max(0, MAX_TEMP_ITEMS - tempFiles.length);
            if (remaining <= 0) { tempTruncated = true; break; }
            const append = next.slice(0, remaining);
            tempFiles = [...tempFiles, ...append];
            if (append.length < next.length) { tempTruncated = true; break; }
          }
        }
      }
      void statTempSizes();
      scanning = false;
      if (tempTruncated) {
        toast.warning(`Showing first ${tempFiles.length} items (truncated). Refine filters.`);
      } else {
        toast.success(`Found ${tempReportedTotal || tempFiles.length} temporary files.`);
      }
    }).then((fn) => unsubs.push(fn)).catch(() => {});

    getDiskInfo();

    return () => { for (const u of unsubs) { try { u(); } catch {} } };
  });

  async function getDiskInfo() {
    try {
      const [total, available]: [number, number] = await invoke('get_drive_info');
      totalDiskSpace = total;
      availableDiskSpace = available;
    } catch (error) {
      console.error('Error getting disk info:', error);
    }
  }

  async function eraserPickFiles() {
    const result = await open({ multiple: true });
    if (Array.isArray(result)) {
      eraserSelectedFiles = result as string[];
    } else if (typeof result === 'string') {
      eraserSelectedFiles = [result];
    }
  }

  async function secureErase() {
    if (eraserSelectedFiles.length === 0) return;
    isErasing = true;
    eraserMessage = '';
    try {
      const count: number = await invoke('secure_erase', {
        files: eraserSelectedFiles,
        passes: eraserPasses,
      });
      eraserMessage = `Securely erased ${count} item(s).`;
      eraserSelectedFiles = [];
    } catch (e) {
      console.error(e);
      eraserMessage = `Failed: ${e}`;
    } finally {
      isErasing = false;
    }
  }

  function formatBytes(bytes: number, decimals = 2) {
    if (!Number.isFinite(bytes) || bytes < 0) return '-';
    if (bytes === 0) return '0 B';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    const value = bytes / Math.pow(k, i);
    return `${parseFloat(value.toFixed(dm))} ${sizes[i]}`;
  }

  async function getTempFiles() {
    scanning = true;
    message = '';
    progressMessage = '';
    try {
      tempFiles = [];
      selectedTempFiles = [];
      tempQueue = []; tempTruncated = false; tempReportedTotal = 0;
      message = 'Scanning for temporary files...';
      await invoke('get_temp_files_stream', { batch_size: 250, max: MAX_TEMP_ITEMS });
    } catch (error) {
      message = `Error scanning temporary files: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      // scanning flag cleared by done event
      progressMessage = '';
    }
  }

  async function stopScan() {
    try {
      await invoke('cancel_temp_scan');
      scanning = false;
      tempQueue = [];
      toast.warning('Scan cancelled');
    } catch {}
  }

  async function deleteSelectedTempFiles() {
    if (selectedTempFiles.length === 0) return;
    confirmDeletion(selectedTempFiles);
  }

  function confirmEmptyRecycleBin() {
    showRecycleBinConfirmationModal = true;
  }

  async function emptyRecycleBin() {
    showRecycleBinConfirmationModal = false;
    isLoading = true;
    message = '';
    try {
      message = 'Emptying recycle bin...';
      await invoke('empty_recycle_bin');
      message = 'Recycle bin emptied successfully.';
      toast.success(message);
    } catch (error) {
      message = `Error emptying recycle bin: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  async function findLargeFiles() {
    isLoading = true;
    message = '';
    progressMessage = '';
    try {
      message = 'Scanning for large files...';
      const result: [string, number][] = await invoke('find_large_files');
      largeFiles = result.map(([path, size]) => ({ path, size }));
      message = `Found ${largeFiles.length} large files.`;
      toast.success(message);
      // If user is in unified view, switch filter to show large results immediately
      if (view === 'unified') {
        filterKind = 'large';
      }
    } catch (error) {
      message = `Error finding large files: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedLargeFiles() {
    if (selectedLargeFiles.length === 0) return;
    confirmDeletion(selectedLargeFiles);
  }

  async function findDuplicateFiles() {
    isLoading = true;
    message = '';
    progressMessage = '';
    try {
      message = 'Scanning for duplicate files...';
      const result: [string, number][] = await invoke('find_duplicate_files');
      duplicateFiles = result.map(([path, size]) => ({ path, size }));
      message = `Found ${duplicateFiles.length} sets of duplicate files.`;
      toast.success(message);
    } catch (error) {
      message = `Error finding duplicate files: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedDuplicateFiles() {
    if (selectedDuplicateFiles.length === 0) return;
    confirmDeletion(selectedDuplicateFiles);
  }

  async function findEmptyFolders() {
    isLoading = true;
    message = '';
    progressMessage = '';
    try {
      message = 'Scanning for empty folders...';
      const result: string[] = await invoke('find_empty_folders');
      emptyFolders = result.map((path) => ({ path }));
      message = `Found ${emptyFolders.length} empty folders.`;
      toast.success(message);
    } catch (error) {
      message = `Error finding empty folders: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedEmptyFolders() {
    if (selectedEmptyFolders.length === 0) return;
    confirmDeletion(selectedEmptyFolders);
  }

  async function findBrokenShortcuts() {
    isLoading = true;
    message = '';
    progressMessage = '';
    try {
      message = 'Scanning for broken shortcuts...';
      const result: string[] = await invoke('find_broken_shortcuts');
      brokenShortcuts = result.map((path) => ({ path }));
      message = `Found ${brokenShortcuts.length} broken shortcuts.`;
      toast.success(message);
    } catch (error) {
      message = `Error finding broken shortcuts: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedBrokenShortcuts() {
    if (selectedBrokenShortcuts.length === 0) return;
    confirmDeletion(selectedBrokenShortcuts);
  }

  function handleFileSelection(
    file: string,
    type: 'temp' | 'large' | 'duplicate' | 'empty' | 'broken_shortcut'
  ) {
    switch (type) {
      case 'temp':
        selectedTempFiles = selectedTempFiles.includes(file)
          ? selectedTempFiles.filter((f) => f !== file)
          : [...selectedTempFiles, file];
        break;
      case 'large':
        selectedLargeFiles = selectedLargeFiles.includes(file)
          ? selectedLargeFiles.filter((f) => f !== file)
          : [...selectedLargeFiles, file];
        break;
      case 'duplicate':
        selectedDuplicateFiles = selectedDuplicateFiles.includes(file)
          ? selectedDuplicateFiles.filter((f) => f !== file)
          : [...selectedDuplicateFiles, file];
        break;
      case 'empty':
        selectedEmptyFolders = selectedEmptyFolders.includes(file)
          ? selectedEmptyFolders.filter((f) => f !== file)
          : [...selectedEmptyFolders, file];
        break;
      case 'broken_shortcut':
        selectedBrokenShortcuts = selectedBrokenShortcuts.includes(file)
          ? selectedBrokenShortcuts.filter((f) => f !== file)
          : [...selectedBrokenShortcuts, file];
        break;
    }
  }

  function toggleSelectAll(type: 'temp' | 'large' | 'duplicate' | 'empty' | 'broken_shortcut') {
    switch (type) {
      case 'temp':
        selectedTempFiles =
          selectedTempFiles.length === tempFiles.length ? [] : tempFiles.map((f) => f.path);
        break;
      case 'large':
        selectedLargeFiles =
          selectedLargeFiles.length === largeFiles.length ? [] : largeFiles.map((f) => f.path);
        break;
      case 'duplicate':
        selectedDuplicateFiles =
          selectedDuplicateFiles.length === duplicateFiles.length
            ? []
            : duplicateFiles.map((f) => f.path);
        break;
      case 'empty':
        selectedEmptyFolders =
          selectedEmptyFolders.length === emptyFolders.length
            ? []
            : emptyFolders.map((f) => f.path);
        break;
      case 'broken_shortcut':
        selectedBrokenShortcuts =
          selectedBrokenShortcuts.length === brokenShortcuts.length
            ? []
            : brokenShortcuts.map((f) => f.path);
        break;
    }
  }

  function confirmDeletion(files: string[]) {
    filesToDelete = files;
    showConfirmationModal = true;
  }

  async function executeDeletion() {
    showConfirmationModal = false;
    isLoading = true;
    message = '';
    try {
      message = `Moving ${filesToDelete.length} item(s) to Trash...`;
      const deletedCount: number = await invoke('move_to_trash', { files: filesToDelete });
      message = `Moved ${deletedCount} item(s) to Trash.`;
      toast.success(message);

      tempFiles = tempFiles.filter((f) => !filesToDelete.includes(f.path));
      largeFiles = largeFiles.filter((f) => !filesToDelete.includes(f.path));
      duplicateFiles = duplicateFiles.filter((f) => !filesToDelete.includes(f.path));
      emptyFolders = emptyFolders.filter((f) => !filesToDelete.includes(f.path));
      brokenShortcuts = brokenShortcuts.filter((f) => !filesToDelete.includes(f.path));

      selectedTempFiles = [];
      selectedLargeFiles = [];
      selectedDuplicateFiles = [];
      selectedEmptyFolders = [];
      selectedBrokenShortcuts = [];
      filesToDelete = [];
      selectedPaths = new Set();
    } catch (error) {
      message = `Error deleting files: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  function cancelDeletion() {
    showConfirmationModal = false;
    filesToDelete = [];
  }

  async function clearUserTemp() {
    isLoading = true;
    message = '';
    try {
      const res: any = await invoke('quick_clear_user_temp');
      message = `Cleared user temp: ${res.files_deleted} files (${formatBytes(res.bytes_deleted)}).`;
      toast.success(message);
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  async function clearSystemTemp() {
    isLoading = true;
    message = '';
    try {
      const res: any = await invoke('quick_clear_system_temp');
      message = `Cleared system temp: ${res.files_deleted} files (${formatBytes(res.bytes_deleted)}).`;
      toast.success(message);
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  async function clearPrefetch() {
    isLoading = true;
    message = '';
    try {
      const res: any = await invoke('quick_clear_prefetch');
      message = `Cleared Prefetch: ${res.files_deleted} files (${formatBytes(res.bytes_deleted)}).`;
      toast.success(message);
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  async function clearRecent() {
    isLoading = true;
    message = '';
    try {
      const res: any = await invoke('quick_clear_recent');
      message = `Cleared Recent shortcuts: ${res.files_deleted} items (${formatBytes(res.bytes_deleted)}).`;
      toast.success(message);
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="space-y-6 text-foreground">
  <Card>
    <CardHeader>
      <div class="flex items-center justify-between gap-2">
        <div>
          <CardTitle class="text-2xl">Cleaner</CardTitle>
          <CardDescription>Scan, review and clean safely.</CardDescription>
        </div>
        <div class="flex items-center gap-2">
          <Label for="view">View</Label>
          <select id="view" bind:value={view} class="h-9 rounded-md border bg-background px-2 text-sm">
            <option value="unified">Unified</option>
            <option value="tabs">Tabs</option>
          </select>
        </div>
            <span class="text-xs text-muted-foreground">Counts ? Temp: {tempFiles.length}, Large: {largeFiles.length}, Dups: {dupGroups.length}, Empty: {emptyFolders.length}, Shortcuts: {brokenShortcuts.length}</span>
      </div>
    </CardHeader>
    <CardContent>
      {#if view === 'unified'}
        <div class="flex flex-col gap-3">
          <div class="flex flex-wrap items-center gap-2">
            <Input placeholder="Search path..." bind:value={q} class="w-[280px]" />
            <div class="flex items-center gap-2">
              <Label for="kind">Type</Label>
              <select id="kind" bind:value={filterKind} class="h-9 rounded-md border bg-background px-2 text-sm">
                <option value="all">All</option>
                <option value="temp">Temp</option>
                <option value="large">Large</option>
                <option value="duplicate">Duplicates</option>
                <option value="empty">Empty</option>
                <option value="shortcut">Shortcuts</option>
              </select>
            </div>
            <div class="flex items-center gap-2">
              <Label for="minmb">Large &gt;=</Label>
              <Input id="minmb" type="number" min="1" class="w-24" bind:value={largeMinMB} />
              <span class="text-xs text-muted-foreground">MB</span>
            </div>
            <div class="ml-auto flex items-center gap-2">
              <Button onclick={scanAll} disabled={isLoading}><Scan class="h-4 w-4" />Scan All</Button>
              {#if scanning}
                <Button variant="secondary" onclick={stopScan} title="Stop scanning">Stop</Button>
              {/if}
              <Button variant="secondary" onclick={() => (showSettings = true)} aria-label="Settings">Settings</Button>
              <Button variant="secondary" onclick={autoSelectDuplicatesKeepOne} title="Auto-select duplicate copies">Auto-select Duplicates</Button>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <Button variant="destructive" disabled={selectedPaths.size===0 || isLoading} onclick={deleteSelectedUnified}><Trash2 class="h-4 w-4" />Delete Selected ({selectedCount})</Button>
            <Button variant="secondary" disabled={selectedPaths.size===0 || isLoading} onclick={moveSelectedUnified}>Move Selected</Button>
            <Button variant="secondary" disabled={selectedPaths.size===0 || isErasing} onclick={secureEraseSelectedUnified}><Eraser class="h-4 w-4" />Secure Erase</Button>
            <span class="text-xs text-muted-foreground">Selected size: {formatBytes(selectedSize)}</span>
            <Button variant="ghost" size="sm" onclick={() => { selectedPaths = new Set(); }}>Clear selection</Button>
          </div>

          <!-- close actions row, then table container -->
          <div class="rounded-md border h-[60vh] overflow-auto" bind:this={unifiedContainer} onscroll={onUnifiedScroll}>
            <table class="w-full text-sm text-foreground">
              <thead class="bg-muted/40 text-xs">
                <tr>
                  <th class="px-3 py-2 text-left w-[36px]">
                    <Checkbox checked={selectedPaths.size>0 && selectedPaths.size===allItems.length && allItems.length>0} onCheckedChange={() => setSelectionForKind(filterKind==='all'?'all':filterKind)} />
                  </th>
                  <th class="px-3 py-2 text-left">Path</th>
                  <th class="px-3 py-2 text-left">Type</th>
                  <th class="px-3 py-2 text-left w-[120px]">Size</th>
                  <th class="px-3 py-2 text-left w-[160px]">Actions</th>
                </tr>
              </thead>
              <tbody>
                {#if allItems.length === 0}
                  <tr>
                    <td colspan="5" class="px-3 py-6 text-center text-muted-foreground">No items. Click Scan All.</td>
                  </tr>
                {:else}
                  {#if unifiedTopPad > 0}
                    <tr><td colspan="5" style={`height:${unifiedTopPad}px`}></td></tr>
                  {/if}
                  {#each (unifiedDisplayed.length > 0 ? unifiedDisplayed : allItems.slice(0, Math.min(allItems.length, UNIFIED_MAX_DOM))) as it (it.path)}
                    <tr class="border-t">
                      <td class="px-3 py-2"><Checkbox checked={selectedPaths.has(it.path)} onCheckedChange={() => toggleSelectUnified(it.path)} /></td>
                      <td class="px-3 py-2"><span class="block truncate max-w-[60ch]" title={it.path}>{it.path}</span></td>
                      <td class="px-3 py-2"><span class="inline-flex items-center rounded border px-2 py-0.5 text-xs capitalize">{it.kind}</span></td>
                      <td class="px-3 py-2">{it.size ? formatBytes(it.size) : '-'}</td>
                      <td class="px-3 py-2">
                        <DropdownMenu>
                          <DropdownMenuTrigger>
                            <Button type="button" variant="ghost" size="sm" aria-label="Details">
                              <MoreHorizontal class="size-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end" onclick={(e: MouseEvent) => e.stopPropagation()}>
                            <DropdownMenuItem onclick={async (e: MouseEvent) => { e.stopPropagation(); try { const { revealItemInDir } = await import('@tauri-apps/plugin-opener'); await revealItemInDir(it.path); } catch {} }}>Reveal</DropdownMenuItem>
                            <DropdownMenuItem onclick={(e: MouseEvent) => { e.stopPropagation(); addExclusion(it.path); }}>Exclude</DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </td>
                    </tr>
                  {/each}
                  {#if unifiedBottomPad > 0}
                    <tr><td colspan="5" style={`height:${unifiedBottomPad}px`}></td></tr>
                  {/if}
                {/if}
              </tbody>
            </table>
          </div>

          {#if message || progressMessage}
            <p class="mt-1 text-xs text-muted-foreground">{progressMessage || message}</p>
          {/if}
        </div>
      {:else}
        <Tabs value={activeTab} onValueChange={(v) => (activeTab = v as CleanerTab)}>
        <TabsList class="flex flex-wrap gap-2">
          <TabsTrigger value="quick">Quick</TabsTrigger>
          <TabsTrigger value="temp">Temp</TabsTrigger>
          <TabsTrigger value="large">Large</TabsTrigger>
          <TabsTrigger value="dup">Duplicates</TabsTrigger>
          <TabsTrigger value="empty">Empty</TabsTrigger>
          <TabsTrigger value="shortcuts">Shortcuts</TabsTrigger>
          <TabsTrigger value="erase">Eraser</TabsTrigger>
          <TabsTrigger value="disk">Disk</TabsTrigger>
        </TabsList>

        <TabsContent value="quick" class="mt-4">
          <div class="flex flex-wrap items-center gap-2">
            <Button onclick={clearUserTemp} disabled={isLoading}>
              <Trash2 class="h-4 w-4" />
              Clear User Temp
            </Button>
            <Button variant="secondary" onclick={clearSystemTemp} disabled={isLoading}>
              <Trash2 class="h-4 w-4" />
              Clear System Temp
            </Button>
            <Button variant="secondary" onclick={clearPrefetch} disabled={isLoading}>
              <RefreshCw class="h-4 w-4" />
              Clear Prefetch
            </Button>
            <Button variant="secondary" onclick={clearRecent} disabled={isLoading}>
              <RefreshCw class="h-4 w-4" />
              Clear Recent
            </Button>
            <Button variant="destructive" onclick={confirmEmptyRecycleBin} disabled={isLoading}>
              <Trash2 class="h-4 w-4" />
              Empty Recycle Bin
            </Button>
          </div>
        </TabsContent>

        <TabsContent value="temp" class="mt-4 space-y-3">
          <div class="flex items-center gap-2">
            <Button onclick={getTempFiles} disabled={isLoading}>
              <Scan class="h-4 w-4" />
              Scan
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Button variant="outline" size="icon" aria-label="More">
                  <MoreHorizontal class="size-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start">
                <DropdownMenuLabel>Temp files</DropdownMenuLabel>
                <DropdownMenuItem onclick={() => getTempFiles()}>Rescan</DropdownMenuItem>
                <DropdownMenuItem onclick={() => toggleSelectAll('temp')}>
                  {selectedTempFiles.length === tempFiles.length && tempFiles.length > 0
                    ? 'Unselect all'
                    : 'Select all'}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={selectedTempFiles.length === 0 || isLoading} onclick={deleteSelectedTempFiles}>
                  Delete selected ({selectedTempFiles.length})
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          {#if tempFiles.length > 0}
            <ScrollArea orientation="both" class="h-64 rounded border">
              <ul class="text-sm">
                {#each tempFiles as file (file.path)}
                  <li class="flex items-center gap-2 px-2 py-1">
                    <Checkbox
                      checked={selectedTempFiles.includes(file.path)}
                      onCheckedChange={() => handleFileSelection(file.path, 'temp')}
                    />
                    <span class="truncate">{file.path}</span>
                  </li>
                {/each}
              </ul>
            </ScrollArea>
          {:else}
            <p class="text-sm text-muted-foreground">No results. Click Scan to search.</p>
          {/if}
        </TabsContent>

        <TabsContent value="large" class="mt-4 space-y-3">
          <div class="flex items-center gap-2">
            <Button onclick={findLargeFiles} disabled={isLoading}>
              <Scan class="h-4 w-4" />
              Scan
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Button variant="outline" size="icon" aria-label="More">
                  <MoreHorizontal class="size-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start">
                <DropdownMenuLabel>Large files</DropdownMenuLabel>
                <DropdownMenuItem onclick={() => findLargeFiles()}>Rescan</DropdownMenuItem>
                <DropdownMenuItem onclick={() => toggleSelectAll('large')}>
                  {selectedLargeFiles.length === largeFiles.length && largeFiles.length > 0
                    ? 'Unselect all'
                    : 'Select all'}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={selectedLargeFiles.length === 0 || isLoading} onclick={deleteSelectedLargeFiles}>
                  Delete selected ({selectedLargeFiles.length})
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          {#if largeFiles.length > 0}
            <ScrollArea orientation="both" class="h-64 rounded border">
              <ul class="text-sm">
                {#each largeFiles as file (file.path)}
                  <li class="flex items-center justify-between gap-2 px-2 py-1">
                    <div class="flex items-center gap-2">
                      <Checkbox
                        checked={selectedLargeFiles.includes(file.path)}
                        onCheckedChange={() => handleFileSelection(file.path, 'large')}
                      />
                      <span class="truncate">{file.path}</span>
                    </div>
                    {#if file.size}
                      <span class="text-xs opacity-70 whitespace-nowrap">{formatBytes(file.size)}</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            </ScrollArea>
          {:else}
            <p class="text-sm text-muted-foreground">No results. Click Scan to search.</p>
          {/if}
        </TabsContent>

        <TabsContent value="dup" class="mt-4 space-y-3">
          <div class="flex items-center gap-2">
            <Button onclick={findDuplicateFiles} disabled={isLoading}>
              <FilesIcon class="h-4 w-4" />
              Scan
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Button variant="outline" size="icon" aria-label="More">
                  <MoreHorizontal class="size-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start">
                <DropdownMenuLabel>Duplicates</DropdownMenuLabel>
                <DropdownMenuItem onclick={() => findDuplicateFiles()}>Rescan</DropdownMenuItem>
                <DropdownMenuItem onclick={() => toggleSelectAll('duplicate')}>
                  {selectedDuplicateFiles.length === duplicateFiles.length && duplicateFiles.length > 0
                    ? 'Unselect all'
                    : 'Select all'}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={selectedDuplicateFiles.length === 0 || isLoading} onclick={deleteSelectedDuplicateFiles}>
                  Delete selected ({selectedDuplicateFiles.length})
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          {#if duplicateFiles.length > 0}
            <ScrollArea orientation="both" class="h-64 rounded border">
              <ul class="text-sm">
                {#each duplicateFiles as file (file.path)}
                  <li class="flex items-center gap-2 px-2 py-1">
                    <Checkbox
                      checked={selectedDuplicateFiles.includes(file.path)}
                      onCheckedChange={() => handleFileSelection(file.path, 'duplicate')}
                    />
                    <span class="truncate">{file.path}</span>
                  </li>
                {/each}
              </ul>
            </ScrollArea>
          {:else}
            <p class="text-sm text-muted-foreground">No results. Click Scan to search.</p>
          {/if}
        </TabsContent>

        <TabsContent value="empty" class="mt-4 space-y-3">
          <div class="flex items-center gap-2">
            <Button onclick={findEmptyFolders} disabled={isLoading}>
              <FolderOpen class="h-4 w-4" />
              Scan
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Button variant="outline" size="icon" aria-label="More">
                  <MoreHorizontal class="size-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start">
                <DropdownMenuLabel>Empty folders</DropdownMenuLabel>
                <DropdownMenuItem onclick={() => findEmptyFolders()}>Rescan</DropdownMenuItem>
                <DropdownMenuItem onclick={() => toggleSelectAll('empty')}>
                  {selectedEmptyFolders.length === emptyFolders.length && emptyFolders.length > 0
                    ? 'Unselect all'
                    : 'Select all'}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={selectedEmptyFolders.length === 0 || isLoading} onclick={deleteSelectedEmptyFolders}>
                  Delete selected ({selectedEmptyFolders.length})
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          {#if emptyFolders.length > 0}
            <ScrollArea orientation="both" class="h-64 rounded border">
              <ul class="text-sm">
                {#each emptyFolders as folder (folder.path)}
                  <li class="flex items-center gap-2 px-2 py-1">
                    <Checkbox
                      checked={selectedEmptyFolders.includes(folder.path)}
                      onCheckedChange={() => handleFileSelection(folder.path, 'empty')}
                    />
                    <span class="truncate">{folder.path}</span>
                  </li>
                {/each}
              </ul>
            </ScrollArea>
          {:else}
            <p class="text-sm text-muted-foreground">No results. Click Scan to search.</p>
          {/if}
        </TabsContent>

        <TabsContent value="shortcuts" class="mt-4 space-y-3">
          <div class="flex items-center gap-2">
            <Button onclick={findBrokenShortcuts} disabled={isLoading}>
              <FolderOpen class="h-4 w-4" />
              Scan
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Button variant="outline" size="icon" aria-label="More">
                  <MoreHorizontal class="size-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start">
                <DropdownMenuLabel>Broken shortcuts</DropdownMenuLabel>
                <DropdownMenuItem onclick={() => findBrokenShortcuts()}>Rescan</DropdownMenuItem>
                <DropdownMenuItem onclick={() => toggleSelectAll('broken_shortcut')}>
                  {selectedBrokenShortcuts.length === brokenShortcuts.length && brokenShortcuts.length > 0
                    ? 'Unselect all'
                    : 'Select all'}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={selectedBrokenShortcuts.length === 0 || isLoading} onclick={deleteSelectedBrokenShortcuts}>
                  Delete selected ({selectedBrokenShortcuts.length})
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          {#if brokenShortcuts.length > 0}
            <ScrollArea orientation="both" class="h-64 rounded border">
              <ul class="text-sm">
                {#each brokenShortcuts as shortcut (shortcut.path)}
                  <li class="flex items-center gap-2 px-2 py-1">
                    <Checkbox
                      checked={selectedBrokenShortcuts.includes(shortcut.path)}
                      onCheckedChange={() => handleFileSelection(shortcut.path, 'broken_shortcut')}
                    />
                    <span class="truncate">{shortcut.path}</span>
                  </li>
                {/each}
              </ul>
            </ScrollArea>
          {:else}
            <p class="text-sm text-muted-foreground">No results. Click Scan to search.</p>
          {/if}
        </TabsContent>

        <TabsContent value="erase" class="mt-4 space-y-3">
          <div class="flex flex-wrap items-center gap-3">
            <Button onclick={eraserPickFiles} disabled={isErasing}>
              <Eraser class="h-4 w-4" />
              Choose Files
            </Button>
            <div class="flex items-center gap-2">
              <Label for="passes">Passes:</Label>
              <Input id="passes" type="number" min="1" max="7" bind:value={eraserPasses} class="w-24" />
            </div>
            <Button variant="destructive" onclick={secureErase} disabled={isErasing || eraserSelectedFiles.length === 0}>
              {#if isErasing}
                Erasing...
              {:else}
                <Eraser class="h-4 w-4" />
                Secure Erase
              {/if}
            </Button>
          </div>
          {#if eraserSelectedFiles.length > 0}
            <ScrollArea orientation="both" class="h-48 rounded border">
              <ul class="text-sm p-2">
                {#each eraserSelectedFiles as f (f)}
                  <li class="truncate py-1">{f}</li>
                {/each}
              </ul>
            </ScrollArea>
          {/if}
          {#if eraserMessage}
            <Separator />
            <p class="text-sm">{eraserMessage}</p>
          {/if}
        </TabsContent>

        <TabsContent value="disk" class="mt-4 space-y-2">
          {#if totalDiskSpace !== null}
            <div class="flex items-center gap-2"><HardDrive class="h-4 w-4" /><p>Total: {formatBytes(totalDiskSpace)}</p></div>
          {/if}
          {#if availableDiskSpace !== null}
            <div class="flex items-center gap-2"><HardDrive class="h-4 w-4" /><p>Available: {formatBytes(availableDiskSpace)}</p></div>
          {/if}
          <Button onclick={getDiskInfo} disabled={isLoading} class="mt-2">
            <RefreshCw class="h-4 w-4" />
            Refresh Disk Info
          </Button>
        </TabsContent>
        </Tabs>
        {#if message || progressMessage}
          <p class="mt-4 text-sm text-muted-foreground">{progressMessage || message}</p>
        {/if}
      {/if}
    </CardContent>
  </Card>
</div>

<Dialog open={showSettings} onOpenChange={(v)=> (showSettings = !!v)}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Cleaner Settings</DialogTitle>
      <DialogDescription>Exclude paths containing these patterns from results.</DialogDescription>
    </DialogHeader>
    <div class="space-y-3">
      <div class="flex items-center gap-2">
        <Input id="ex-add" placeholder="Add path substring, e.g. C:\\Games" onkeydown={(e: KeyboardEvent) => {
          const input = e.currentTarget as HTMLInputElement;
          if (e.key === 'Enter') { addExclusion(input.value); input.value = ''; }
        }} />
        <Button size="sm" onclick={() => {
          const el = document.getElementById('ex-add') as HTMLInputElement | null; if (el) { addExclusion(el.value); el.value=''; }
        }}>Add</Button>
      </div>
      <div class="rounded border">
        <ul class="max-h-48 overflow-auto text-sm">
          {#if exclusions.length === 0}
            <li class="px-3 py-2 text-muted-foreground">No exclusions</li>
          {:else}
            {#each exclusions as ex (ex)}
              <li class="flex items-center justify-between gap-2 border-b px-3 py-2">
                <span class="truncate" title={ex}>{ex}</span>
                <Button variant="ghost" size="sm" onclick={() => removeExclusion(ex)}>Remove</Button>
              </li>
            {/each}
          {/if}
        </ul>
      </div>
    </div>
    <DialogFooter>
      <Button variant="secondary" onclick={() => (showSettings = false)}>Close</Button>
    </DialogFooter>
  </DialogContent>
  
</Dialog>

<AlertDialog open={showConfirmationModal} onOpenChange={(v) => (showConfirmationModal = v)}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Delete selected?</AlertDialogTitle>
      <AlertDialogDescription>
        This moves {filesToDelete.length} item(s) to the Recycle Bin.
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={cancelDeletion}>Cancel</AlertDialogCancel>
      <AlertDialogAction class="bg-destructive text-destructive-foreground hover:opacity-90" onclick={executeDeletion}>
        Delete
      </AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>

<AlertDialog open={showRecycleBinConfirmationModal} onOpenChange={(v) => (showRecycleBinConfirmationModal = v)}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Empty Recycle Bin?</AlertDialogTitle>
      <AlertDialogDescription>This cannot be undone.</AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={() => (showRecycleBinConfirmationModal = false)}>Cancel</AlertDialogCancel>
      <AlertDialogAction class="bg-destructive text-destructive-foreground hover:opacity-90" onclick={emptyRecycleBin}>
        Empty
      </AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>

{#if isLoading || isErasing}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm"
    role="status"
    aria-live="polite"
    aria-busy="true"
  >
    <div class="w-[min(32rem,calc(100%-2rem))] space-y-4 rounded-lg bg-card p-6 shadow-lg">
      <div class="flex items-center gap-3">
        <Skeleton class="size-12 rounded-full" aria-hidden="true" />
        <div class="flex-1 space-y-2">
          <Skeleton class="h-4 w-3/4" aria-hidden="true" />
          <Skeleton class="h-3 w-1/2" aria-hidden="true" />
        </div>
      </div>
      <div class="space-y-2">
        <Skeleton class="h-3 w-full" aria-hidden="true" />
        <Skeleton class="h-3 w-5/6" aria-hidden="true" />
        <Skeleton class="h-3 w-4/6" aria-hidden="true" />
      </div>
      <p class="text-sm text-muted-foreground">
        {progressMessage || (isErasing ? 'Securely erasing...' : 'Working...')}
      </p>
    </div>
  </div>
{/if}
