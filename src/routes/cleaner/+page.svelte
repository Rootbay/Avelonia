<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
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
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { toast } from '$lib/components/ui/sonner';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
  } from '$lib/components/ui/dropdown-menu';
  import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
  import { saveCleanerCache } from '$lib/cleanerCache';
  import { pushLog, type LogLevel } from '$lib/logStore';
  import { Trash2, Scan, Eraser, Ellipsis } from '@lucide/svelte';
  import { i18n } from '$lib/i18n.svelte';

  import { cleanerScan, beginCleanerScan } from '$lib/cleanerScan.svelte';

  let isDeferredLoaded = $state(false);
  onMount(() => {
    const t = setTimeout(() => {
      isDeferredLoaded = true;
    }, 120);
    return () => clearTimeout(t);
  });

  const tempFiles = $derived(isDeferredLoaded ? cleanerScan.tempFiles : []);
  const largeFiles = $derived(isDeferredLoaded ? cleanerScan.largeFiles : []);
  const duplicateFiles = $derived(isDeferredLoaded ? cleanerScan.duplicateFiles : []);
  const emptyFolders = $derived(isDeferredLoaded ? cleanerScan.emptyFolders : []);
  const brokenShortcuts = $derived(isDeferredLoaded ? cleanerScan.brokenShortcuts : []);
  const dupGroups = $derived(isDeferredLoaded ? cleanerScan.dupGroups : []);
  const selectedPaths = $derived(
    isDeferredLoaded ? cleanerScan.selectedPaths : new SvelteSet<string>()
  );

  let totalDiskSpace = $state(0);
  let availableDiskSpace = $state(0);

  const tempSize = $derived(tempFiles.reduce((acc, f) => acc + (f.size || 0), 0));
  const largeSize = $derived(largeFiles.reduce((acc, f) => acc + (f.size || 0), 0));
  const duplicateSize = $derived(duplicateFiles.reduce((acc, f) => acc + (f.size || 0), 0));
  const totalClutterSize = $derived(tempSize + largeSize + duplicateSize);

  const clutterRatio = $derived(totalDiskSpace > 0 ? (totalClutterSize / totalDiskSpace) * 100 : 0);
  const freeRatio = $derived(totalDiskSpace > 0 ? (availableDiskSpace / totalDiskSpace) * 100 : 0);
  const otherUsedRatio = $derived(
    totalDiskSpace > 0 ? Math.max(0, 100 - clutterRatio - freeRatio) : 0
  );

  const tempClutterPct = $derived(totalClutterSize > 0 ? (tempSize / totalClutterSize) * 100 : 0);
  const largeClutterPct = $derived(totalClutterSize > 0 ? (largeSize / totalClutterSize) * 100 : 0);
  const dupClutterPct = $derived(
    totalClutterSize > 0 ? (duplicateSize / totalClutterSize) * 100 : 0
  );

  let message = $state('');
  const progressMessage = $derived(cleanerScan.message || '');
  let isLoading = $state(false);
  const scanning = $derived(cleanerScan.phase === 'running');
  let eraserPasses = $state(1);
  let isErasing = $state(false);
  let showConfirmationModal = $state(false);
  let filesToDelete = $state<string[]>([]);
  type Kind = 'temp' | 'large' | 'duplicate' | 'empty' | 'shortcut';
  let q = $state('');
  let qDeb = $state('');
  let filterKind = $state<'all' | Kind>('all');
  let largeMinMB = $state(100);
  let showSettings = $state(false);
  let exclusions = $state<string[]>([]);
  let normalizedExclusions = $derived(
    Array.from(
      new Set(
        exclusions.map((ex) => (ex || '').trim().toLowerCase()).filter((value) => Boolean(value))
      )
    )
  );
  const EXC_KEY = 'avelonia_cleaner_exclusions_v1';
  function logCleanerError(context: string, error: unknown, level: LogLevel = 'WARN') {
    pushLog(level, `${context}: ${String(error)}`, 'Cleaner');
  }

  function loadExclusions() {
    try {
      const raw = localStorage.getItem(EXC_KEY);
      exclusions = raw ? JSON.parse(raw) : [];
    } catch (error) {
      exclusions = [];
      logCleanerError('Load exclusions failed', error);
    }
  }

  function saveExclusions() {
    try {
      localStorage.setItem(
        EXC_KEY,
        JSON.stringify(Array.from(new Set(exclusions.map((s) => s.trim()).filter(Boolean))))
      );
    } catch (error) {
      logCleanerError('Save exclusions failed', error);
    }
  }

  onMount(async () => {
    loadExclusions();
    try {
      const [total, avail] = await invoke<[number, number]>('get_drive_info');
      totalDiskSpace = total;
      availableDiskSpace = avail;
    } catch (error) {
      logCleanerError('Failed to fetch drive info', error);
    }
  });

  function addExclusion(pattern: string) {
    const p = (pattern || '').trim();
    if (!p) return;
    exclusions = Array.from(new Set([...exclusions, p]));
    saveExclusions();
  }
  function removeExclusion(pattern: string) {
    exclusions = exclusions.filter((s) => s !== pattern);
    saveExclusions();
  }

  let _cacheSaveTimer: number | null = null;
  function saveCacheSoon() {
    try {
      if (_cacheSaveTimer) clearTimeout(_cacheSaveTimer as unknown as number);
    } catch (error) {
      logCleanerError('Clear cache timer failed', error);
    }
    _cacheSaveTimer = setTimeout(() => {
      try {
        saveCleanerCache({
          tempFiles: cleanerScan.tempFiles,
          largeFiles: cleanerScan.largeFiles,
          duplicateFiles: cleanerScan.duplicateFiles,
          emptyFolders: cleanerScan.emptyFolders,
          brokenShortcuts: cleanerScan.brokenShortcuts,
          dupGroups: cleanerScan.dupGroups,
          timestamp: Date.now(),
        });
      } catch (error) {
        logCleanerError('Save cleaner cache failed', error);
      }
    }, 500) as unknown as number;
  }

  $effect(() => {
    if (!isDeferredLoaded) return;
    void cleanerScan.tempFiles;
    void cleanerScan.largeFiles;
    void cleanerScan.duplicateFiles;
    void cleanerScan.emptyFolders;
    void cleanerScan.brokenShortcuts;
    void cleanerScan.dupGroups;
    saveCacheSoon();
  });

  $effect(() => {
    const t = setTimeout(() => (qDeb = q), 150);
    return () => clearTimeout(t);
  });

  function matchesExclusion(p: string): boolean {
    try {
      const normalizedPath = (p || '').toLowerCase();
      for (const pattern of normalizedExclusions) {
        if (pattern && normalizedPath.includes(pattern)) return true;
      }
    } catch (error) {
      logCleanerError('Match exclusion failed', error);
    }
    return false;
  }

  type CleanerItem = { path: string; size?: number; kind: Kind; groupId?: string };

  let unifiedCap = $state(500000);
  const UNIFIED_BUILD_STEP = 5000;
  const allItems = $derived.by<CleanerItem[]>(() => {
    const cap = unifiedCap;
    const items: CleanerItem[] = [];
    const seen = new SvelteSet<string>();
    const term = qDeb.trim().toLowerCase();
    function tryPush(it: CleanerItem) {
      const p = it.path;
      if (matchesExclusion(p)) return;
      if (filterKind !== 'all' && it.kind !== filterKind) return;
      if (term && !p.toLowerCase().includes(term)) return;
      if (seen.has(p)) return;
      seen.add(p);
      items.push(it);
    }
    for (const f of tempFiles) {
      tryPush({ path: f.path, size: f.size, kind: 'temp' });
      if (items.length >= cap) break;
    }
    for (const f of largeFiles) {
      tryPush({ path: f.path, size: f.size, kind: 'large' });
      if (items.length >= cap) break;
    }
    for (const f of duplicateFiles) {
      tryPush({ path: f.path, size: f.size, kind: 'duplicate' });
      if (items.length >= cap) break;
    }
    for (const f of emptyFolders) {
      tryPush({ path: f.path, size: f.size, kind: 'empty' });
      if (items.length >= cap) break;
    }
    for (const f of brokenShortcuts) {
      tryPush({ path: f.path, size: f.size, kind: 'shortcut' });
      if (items.length >= cap) break;
    }
    if (dupGroups.length > 0) {
      for (const g of dupGroups) {
        for (const p of g.files) {
          tryPush({ path: p, size: g.size, kind: 'duplicate', groupId: g.hash });
          if (items.length >= cap) break;
        }
      }
    }
    return items;
  });

  const selectedCount = $derived(selectedPaths.size);
  const selectedSize = $derived.by(() => {
    let sum = 0;
    const s = selectedPaths;
    for (const it of allItems) {
      if (s.has(it.path)) sum += it.size ?? 0;
    }
    return sum;
  });

  const MAX_TEMP_ITEMS = 20000;

  let unifiedContainer = $state<HTMLDivElement | null>(null);
  let _unifiedLastScrollTop = 0;
  let _unifiedLastDir: 'up' | 'down' | null = null;
  let UNIFIED_ROW_PX = $state(40);
  const UNIFIED_PREBUFFER = 3;
  const UNIFIED_MAX_DOM = 600;
  let unifiedAutoFallback = $state(false);
  let unifiedVirtualize = $derived(!unifiedAutoFallback && (scanning || allItems.length > 1500));

  let unifiedStart = $state(0);
  const unifiedRowsInView = $derived(
    Math.ceil((unifiedContainer?.clientHeight ?? 480) / UNIFIED_ROW_PX) + 20
  );

  const unifiedDisplayed = $derived.by(() => {
    if (!unifiedVirtualize) return allItems;
    const items = allItems;
    const visibleRows = Math.min(unifiedRowsInView, UNIFIED_MAX_DOM);
    const end = Math.min(items.length, unifiedStart + visibleRows);
    return items.slice(unifiedStart, end);
  });

  let unifiedDisplayList = $derived(unifiedDisplayed);

  let unifiedPadPx = $state(0);
  const unifiedTopPad = $derived(unifiedVirtualize ? unifiedPadPx : 0);
  const unifiedBottomPad = $derived(
    unifiedVirtualize
      ? Math.max(
          0,
          allItems.length * UNIFIED_ROW_PX -
            (unifiedTopPad + unifiedDisplayed.length * UNIFIED_ROW_PX)
        )
      : 0
  );

  $effect(() => {
    if (!unifiedVirtualize) {
      unifiedStart = 0;
      return;
    }
    const maxStart = Math.max(0, allItems.length - Math.min(unifiedRowsInView, UNIFIED_MAX_DOM));
    if (unifiedStart > maxStart) unifiedStart = maxStart;
    if (unifiedStart < 0) unifiedStart = 0;
  });

  $effect(() => {
    void filterKind;
    void qDeb;
    void exclusions.length;
    unifiedStart = 0;
    _unifiedLastScrollTop = 0;
  });

  $effect(() => {
    const _len = allItems.length;
    void _len;
    setTimeout(() => {
      try {
        const el = unifiedContainer as HTMLElement | null;
        if (!el) return;
        const rowsVis = Math.ceil((el.clientHeight || 0) / UNIFIED_ROW_PX) + 5;
        const needsVirtualization = _len > rowsVis;
        const hasScrollSpace = el.scrollHeight > el.clientHeight + 1;
        const shouldFallback = needsVirtualization && !hasScrollSpace;
        if (unifiedAutoFallback !== shouldFallback) {
          unifiedAutoFallback = shouldFallback;
        }
      } catch {
        /* ignore layout measurement errors */
      }
    }, 0);
  });

  function measureUnifiedRowHeight() {
    try {
      const el = unifiedContainer as HTMLElement | null;
      if (!el) return;
      const row = el.querySelector('tbody tr.border-t') as HTMLElement | null;
      const h = Math.round(row?.getBoundingClientRect()?.height ?? 0);
      if (h && isFinite(h) && h > 8 && h < 200) {
        UNIFIED_ROW_PX = h;
      }
    } catch {
      /* ignore layout measurement errors */
    }
  }

  onMount(() => {
    setTimeout(measureUnifiedRowHeight, 0);
  });

  $effect(() => {
    void unifiedDisplayed.length;
    setTimeout(measureUnifiedRowHeight, 0);
  });

  let _unifiedScrollTick = false;
  function onUnifiedScroll(_event: Event) {
    if (_unifiedScrollTick) return;
    _unifiedScrollTick = true;
    requestAnimationFrame(() => {
      const el = unifiedContainer as HTMLElement | null;
      if (!el) {
        _unifiedScrollTick = false;
        return;
      }
      const top = el.scrollTop;
      _unifiedLastDir =
        top < _unifiedLastScrollTop ? 'up' : top > _unifiedLastScrollTop ? 'down' : _unifiedLastDir;
      _unifiedLastScrollTop = top;
      if (unifiedVirtualize) {
        const approx = Math.floor(top / UNIFIED_ROW_PX);
        const first = approx - UNIFIED_PREBUFFER;
        unifiedStart = Math.max(0, first);
        unifiedPadPx = Math.max(0, top - UNIFIED_PREBUFFER * UNIFIED_ROW_PX);
      }
      if (unifiedVirtualize) {
        try {
          const movingDown = _unifiedLastDir === 'down';
          if (movingDown && el.scrollTop + el.clientHeight >= el.scrollHeight - 400) {
            unifiedCap = Math.min(unifiedCap + UNIFIED_BUILD_STEP, unifiedCap + 20000);
          }
        } catch {
          /* ignore scroll measurement errors */
        }
      }
      _unifiedScrollTick = false;
    });
  }

  async function scanAll() {
    if (cleanerScan.phase === 'running' || isLoading) return;
    beginCleanerScan();
    message = '';
    try {
      const minBytes = Math.max(1, largeMinMB) * 1024 * 1024;
      void invoke('start_cleaner_scan', {
        min_size_bytes: minBytes,
        max_temp: MAX_TEMP_ITEMS,
        exclusions: normalizedExclusions,
      });
    } catch (e) {
      logCleanerError('Scan failed to start', e, 'ERROR');
      toast.error('Scan failed to start');
      cleanerScan.phase = 'idle';
    }
  }

  function toggleSelectUnified(p: string) {
    if (cleanerScan.selectedPaths.has(p)) {
      cleanerScan.selectedPaths.delete(p);
    } else {
      cleanerScan.selectedPaths.add(p);
    }
  }

  function clearSelectionUnified() {
    cleanerScan.selectedPaths.clear();
  }

  function setSelectionForKind(kind: 'all' | Kind) {
    for (const it of allItems) {
      if (kind === 'all' || it.kind === kind) {
        cleanerScan.selectedPaths.add(it.path);
      }
    }
  }

  async function deleteSelectedUnified() {
    const files = Array.from(cleanerScan.selectedPaths);
    if (files.length === 0) return;
    filesToDelete = files;
    showConfirmationModal = true;
  }

  async function moveSelectedUnified() {
    const files = Array.from(cleanerScan.selectedPaths);
    if (files.length === 0) return;
    try {
      const dest = await open({ directory: true });
      if (!dest || typeof dest !== 'string') return;
      isLoading = true;
      message = 'Moving selected...';
      const moved = (await invoke('move_files', { files, destination: dest })) as number;
      cleanerScan.tempFiles = cleanerScan.tempFiles.filter(
        (f) => !cleanerScan.selectedPaths.has(f.path)
      );
      cleanerScan.largeFiles = cleanerScan.largeFiles.filter(
        (f) => !cleanerScan.selectedPaths.has(f.path)
      );
      cleanerScan.duplicateFiles = cleanerScan.duplicateFiles.filter(
        (f) => !cleanerScan.selectedPaths.has(f.path)
      );
      cleanerScan.emptyFolders = cleanerScan.emptyFolders.filter(
        (f) => !cleanerScan.selectedPaths.has(f.path)
      );
      cleanerScan.brokenShortcuts = cleanerScan.brokenShortcuts.filter(
        (f) => !cleanerScan.selectedPaths.has(f.path)
      );

      const nextGroups = cleanerScan.dupGroups
        .map((g) => {
          return {
            ...g,
            files: g.files.filter((p) => !cleanerScan.selectedPaths.has(p)),
          };
        })
        .filter((g) => g.files.length > 1);
      cleanerScan.dupGroups = nextGroups;

      clearSelectionUnified();
      toast.success(`Moved ${moved} item(s)`);
    } catch (e) {
      logCleanerError('Move failed', e, 'ERROR');
      toast.error('Move failed');
    } finally {
      isLoading = false;
    }
  }

  async function secureEraseSelectedUnified() {
    const files = Array.from(cleanerScan.selectedPaths);
    if (files.length === 0) return;
    try {
      isErasing = true;
      await invoke('secure_erase', { files, passes: eraserPasses });
      toast.success('Secure erase done');
      clearSelectionUnified();
    } catch (e) {
      logCleanerError('Secure erase failed', e, 'ERROR');
      toast.error('Secure erase failed');
    } finally {
      isErasing = false;
    }
  }

  function autoSelectDuplicatesKeepOne() {
    if (cleanerScan.dupGroups.length === 0) return;
    for (const g of cleanerScan.dupGroups) {
      if (!g.files || g.files.length < 2) continue;
      const sorted = [...g.files].sort();
      for (let i = 1; i < sorted.length; i++) {
        cleanerScan.selectedPaths.add(sorted[i]);
      }
    }
    filterKind = 'duplicate';
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

  async function stopScan() {
    try {
      await invoke('cancel_temp_scan');
      await invoke('cancel_cleaner_scan');
    } catch (error) {
      logCleanerError('Cancel scan failed', error);
    }
    toast.warning('Scan cancelled');
  }

  async function executeDeletion() {
    showConfirmationModal = false;
    isLoading = true;
    message = '';
    try {
      message = `Moving ${filesToDelete.length} item(s) to Trash...`;
      const deletedCount: number = await invoke('move_to_trash', { files: filesToDelete });
      message = `Moved ${deletedCount} item(s) to Trash.`;

      cleanerScan.tempFiles = cleanerScan.tempFiles.filter((f) => !filesToDelete.includes(f.path));
      cleanerScan.largeFiles = cleanerScan.largeFiles.filter(
        (f) => !filesToDelete.includes(f.path)
      );
      cleanerScan.duplicateFiles = cleanerScan.duplicateFiles.filter(
        (f) => !filesToDelete.includes(f.path)
      );
      cleanerScan.emptyFolders = cleanerScan.emptyFolders.filter(
        (f) => !filesToDelete.includes(f.path)
      );
      cleanerScan.brokenShortcuts = cleanerScan.brokenShortcuts.filter(
        (f) => !filesToDelete.includes(f.path)
      );

      const nextGroups = cleanerScan.dupGroups
        .map((g) => {
          return {
            ...g,
            files: g.files.filter((p) => !filesToDelete.includes(p)),
          };
        })
        .filter((g) => g.files.length > 1);
      cleanerScan.dupGroups = nextGroups;

      filesToDelete = [];
      clearSelectionUnified();
    } catch (error) {
      message = `Error deleting files: ${error}`;
      logCleanerError('Delete failed', error, 'ERROR');
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  function cancelDeletion() {
    showConfirmationModal = false;
    filesToDelete = [];
  }
</script>

<div class="space-y-6 text-foreground">
  <Card>
    <CardHeader>
      <CardTitle class="text-2xl">{i18n.t('cleaner.title')}</CardTitle>
      <div class="flex items-baseline justify-between">
        <CardDescription>{i18n.t('cleaner.desc')}</CardDescription>
        {#if !isDeferredLoaded}
          <div class="flex items-center gap-2">
            <Skeleton class="h-3 w-16" />
            <Skeleton class="h-3 w-20" />
            <Skeleton class="h-3 w-14" />
          </div>
        {:else}
          <span class="text-xs text-muted-foreground"
            >{i18n.t('cleaner.category_temp')}: {tempFiles.length}, {i18n.t(
              'cleaner.category_large'
            )}: {largeFiles.length}, {i18n.t('cleaner.category_dup')}: {dupGroups.length},
            {i18n.t('cleaner.category_empty')}: {emptyFolders.length}, {i18n.t(
              'cleaner.category_shortcuts'
            )}: {brokenShortcuts.length}</span
          >
        {/if}
      </div>
    </CardHeader>
  </Card>

  <!-- SOTA Storage Analyzer Visualizer -->
  {#if !isDeferredLoaded}
    <div class="grid gap-4 md:grid-cols-2">
      <!-- Drive storage skeleton -->
      <Card
        class="glass-card p-4 flex flex-col justify-between transition-all duration-300 hover:scale-[1.01] glow-purple"
      >
        <div>
          <h3
            class="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2 font-heading"
          >
            {i18n.t('cleaner.drive_analyzer')}
          </h3>
          <div class="flex items-baseline gap-1.5 mb-3">
            <Skeleton class="h-7 w-28" />
            <Skeleton class="h-3 w-20" />
          </div>

          <Skeleton class="h-3.5 w-full rounded-full mb-4" />

          <!-- Legend skeleton -->
          <div class="grid grid-cols-3 gap-1.5 text-[11px]">
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-purple-500/50 animate-pulse"></span>
              <div class="flex flex-col gap-1">
                <Skeleton class="h-3 w-10" />
                <Skeleton class="h-2 w-12" />
              </div>
            </div>
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-zinc-400/50 dark:bg-zinc-600/50 animate-pulse"
              ></span>
              <div class="flex flex-col gap-1">
                <Skeleton class="h-3 w-16" />
                <Skeleton class="h-2 w-12" />
              </div>
            </div>
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-emerald-500/50 animate-pulse"></span>
              <div class="flex flex-col gap-1">
                <Skeleton class="h-3 w-14" />
                <Skeleton class="h-2 w-12" />
              </div>
            </div>
          </div>
        </div>
      </Card>

      <!-- Clutter breakdown skeleton -->
      <Card
        class="glass-card p-4 flex flex-col justify-between transition-all duration-300 hover:scale-[1.01] glow-emerald"
      >
        <div>
          <h3
            class="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2 font-heading"
          >
            {i18n.t('cleaner.clutter_breakdown')}
          </h3>
          <div class="flex items-baseline gap-1.5 mb-3">
            <Skeleton class="h-7 w-28" />
            <Skeleton class="h-3 w-20" />
          </div>

          <Skeleton class="h-3.5 w-full rounded-full mb-4" />

          <!-- Legend skeleton -->
          <div class="grid grid-cols-3 gap-1.5 text-[11px]">
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-violet-500/50 animate-pulse"></span>
              <div class="flex flex-col gap-1">
                <Skeleton class="h-3 w-12" />
                <Skeleton class="h-2 w-14" />
              </div>
            </div>
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-blue-500/50 animate-pulse"></span>
              <div class="flex flex-col gap-1">
                <Skeleton class="h-3 w-12" />
                <Skeleton class="h-2 w-14" />
              </div>
            </div>
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-amber-500/50 animate-pulse"></span>
              <div class="flex flex-col gap-1">
                <Skeleton class="h-3 w-12" />
                <Skeleton class="h-2 w-14" />
              </div>
            </div>
          </div>
        </div>
      </Card>
    </div>
  {:else if totalClutterSize > 0 || (totalDiskSpace > 0 && availableDiskSpace > 0)}
    <div class="grid gap-4 md:grid-cols-2">
      <!-- Drive health / total distribution -->
      <Card
        class="glass-card p-4 flex flex-col justify-between transition-all duration-300 hover:scale-[1.01] glow-purple"
      >
        <div>
          <h3
            class="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2 font-heading"
          >
            {i18n.t('cleaner.drive_analyzer')}
          </h3>
          <div class="flex items-baseline gap-1.5 mb-3">
            <span class="text-2xl font-extrabold tracking-tight font-heading"
              >{formatBytes(totalDiskSpace - availableDiskSpace)}</span
            >
            <span class="text-[10px] text-muted-foreground"
              >{i18n.t('cleaner.used_of', { total: formatBytes(totalDiskSpace) })}</span
            >
          </div>

          <div
            class="h-3.5 w-full rounded-full bg-muted overflow-hidden flex border border-border/30 mb-4"
          >
            {#if clutterRatio > 0}
              <div
                style="width: {clutterRatio}%"
                class="bg-gradient-to-r from-purple-500 to-indigo-500 transition-all duration-500"
                title="Scanned Clutter"
              ></div>
            {/if}
            {#if otherUsedRatio > 0}
              <div
                style="width: {otherUsedRatio}%"
                class="bg-zinc-400 dark:bg-zinc-600 transition-all duration-500"
                title="System / Other files"
              ></div>
            {/if}
            {#if freeRatio > 0}
              <div
                style="width: {freeRatio}%"
                class="bg-gradient-to-r from-emerald-500 to-teal-500 transition-all duration-500"
                title="Free Space"
              ></div>
            {/if}
          </div>

          <!-- Legend -->
          <div class="grid grid-cols-3 gap-1.5 text-[11px]">
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-purple-500"></span>
              <div class="flex flex-col">
                <span class="font-semibold text-foreground/80 leading-none"
                  >{i18n.t('cleaner.clutter')}</span
                >
                <span class="text-[9px] text-muted-foreground mt-0.5"
                  >{formatBytes(totalClutterSize)}</span
                >
              </div>
            </div>
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-zinc-400 dark:bg-zinc-600"></span>
              <div class="flex flex-col">
                <span class="font-semibold text-foreground/80 leading-none"
                  >{i18n.t('cleaner.system_other')}</span
                >
                <span class="text-[9px] text-muted-foreground mt-0.5"
                  >{formatBytes(
                    Math.max(0, totalDiskSpace - availableDiskSpace - totalClutterSize)
                  )}</span
                >
              </div>
            </div>
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-emerald-500"></span>
              <div class="flex flex-col">
                <span class="font-semibold text-foreground/80 leading-none"
                  >{i18n.t('cleaner.free_space')}</span
                >
                <span class="text-[9px] text-muted-foreground mt-0.5"
                  >{formatBytes(availableDiskSpace)}</span
                >
              </div>
            </div>
          </div>
        </div>
      </Card>

      <!-- Clutter category breakdown -->
      <Card
        class="glass-card p-4 flex flex-col justify-between transition-all duration-300 hover:scale-[1.01] glow-emerald"
      >
        <div>
          <h3
            class="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2 font-heading"
          >
            {i18n.t('cleaner.clutter_breakdown')}
          </h3>
          <div class="flex items-baseline gap-1.5 mb-3">
            <span class="text-2xl font-extrabold tracking-tight font-heading"
              >{formatBytes(totalClutterSize)}</span
            >
            <span class="text-[10px] text-muted-foreground"
              >{i18n.t('cleaner.scanned_removable')}</span
            >
          </div>

          <div
            class="h-3.5 w-full rounded-full bg-muted overflow-hidden flex border border-border/30 mb-4"
          >
            {#if tempClutterPct > 0}
              <div
                style="width: {tempClutterPct}%"
                class="bg-gradient-to-r from-violet-500 to-indigo-500 transition-all duration-500"
                title="Temp Files"
              ></div>
            {/if}
            {#if largeClutterPct > 0}
              <div
                style="width: {largeClutterPct}%"
                class="bg-gradient-to-r from-blue-500 to-cyan-500 transition-all duration-500"
                title="Large Files"
              ></div>
            {/if}
            {#if dupClutterPct > 0}
              <div
                style="width: {dupClutterPct}%"
                class="bg-gradient-to-r from-amber-500 to-orange-500 transition-all duration-500"
                title="Duplicate Files"
              ></div>
            {/if}
          </div>

          <!-- Legend -->
          <div class="grid grid-cols-3 gap-1.5 text-[11px]">
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-violet-500"></span>
              <div class="flex flex-col">
                <span class="font-semibold text-foreground/80 leading-none"
                  >{i18n.t('cleaner.category_temp')}</span
                >
                <span class="text-[9px] text-muted-foreground mt-0.5"
                  >{formatBytes(tempSize)} ({tempFiles.length})</span
                >
              </div>
            </div>
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-blue-500"></span>
              <div class="flex flex-col">
                <span class="font-semibold text-foreground/80 leading-none"
                  >{i18n.t('cleaner.category_large')}</span
                >
                <span class="text-[9px] text-muted-foreground mt-0.5"
                  >{formatBytes(largeSize)} ({largeFiles.length})</span
                >
              </div>
            </div>
            <div class="flex items-center gap-1.5">
              <span class="size-2 rounded-full bg-amber-500"></span>
              <div class="flex flex-col">
                <span class="font-semibold text-foreground/80 leading-none"
                  >{i18n.t('cleaner.category_dup')}</span
                >
                <span class="text-[9px] text-muted-foreground mt-0.5"
                  >{formatBytes(duplicateSize)} ({duplicateFiles.length})</span
                >
              </div>
            </div>
          </div>
        </div>
      </Card>
    </div>
  {/if}

  <div class="space-y-6 text-foreground">
    <Card>
      <CardContent>
        <div class="flex flex-col gap-3">
          <div class="flex flex-wrap items-center gap-2">
            <Input placeholder={i18n.t('cleaner.search_path')} bind:value={q} class="w-70" />
            <div class="flex items-center gap-2">
              <Label for="kind">{i18n.t('cleaner.type')}</Label>
              <Select type="single" bind:value={filterKind}>
                <SelectTrigger id="kind" class="w-30">
                  <p class="truncate">
                    {filterKind === 'all'
                      ? i18n.t('cleaner.all')
                      : filterKind === 'temp'
                        ? i18n.t('cleaner.category_temp')
                        : filterKind === 'large'
                          ? i18n.t('cleaner.category_large')
                          : filterKind === 'duplicate'
                            ? i18n.t('cleaner.category_dup')
                            : filterKind === 'empty'
                              ? i18n.t('cleaner.category_empty')
                              : filterKind === 'shortcut'
                                ? i18n.t('cleaner.category_shortcuts')
                                : filterKind}
                  </p>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">{i18n.t('cleaner.all')}</SelectItem>
                  <SelectItem value="temp">{i18n.t('cleaner.category_temp')}</SelectItem>
                  <SelectItem value="large">{i18n.t('cleaner.category_large')}</SelectItem>
                  <SelectItem value="duplicate">{i18n.t('cleaner.category_dup')}</SelectItem>
                  <SelectItem value="empty">{i18n.t('cleaner.category_empty')}</SelectItem>
                  <SelectItem value="shortcut">{i18n.t('cleaner.category_shortcuts')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div class="ml-auto flex items-center gap-2">
              <Button onclick={scanAll} disabled={isLoading}
                ><Scan class="h-4 w-4" />{i18n.t('cleaner.scan_all')}</Button
              >
              {#if scanning}
                <Button variant="secondary" onclick={stopScan} title="Stop scanning"
                  >{i18n.t('cleaner.stop')}</Button
                >
              {/if}
              <Button
                variant="secondary"
                onclick={() => (showSettings = true)}
                aria-label="Settings">{i18n.t('cleaner.settings')}</Button
              >
              <Button
                variant="secondary"
                onclick={autoSelectDuplicatesKeepOne}
                title="Auto-select duplicate copies"
                >{i18n.t('cleaner.auto_select_duplicates')}</Button
              >
            </div>
          </div>
          <div class="flex items-center gap-2">
            <Button
              variant="destructive"
              disabled={selectedPaths.size === 0 || isLoading}
              onclick={deleteSelectedUnified}
              ><Trash2 class="h-4 w-4" />{i18n.t('cleaner.delete_selected', {
                count: selectedCount,
              })}</Button
            >
            <Button
              variant="secondary"
              disabled={selectedPaths.size === 0 || isLoading}
              onclick={moveSelectedUnified}>{i18n.t('cleaner.move_selected')}</Button
            >
            <Button
              variant="secondary"
              disabled={selectedPaths.size === 0 || isErasing}
              onclick={secureEraseSelectedUnified}
              ><Eraser class="h-4 w-4" />{i18n.t('cleaner.secure_erase')}</Button
            >
            <span class="text-xs text-muted-foreground"
              >{i18n.t('cleaner.selected_size', { size: formatBytes(selectedSize) })}</span
            >
            <Button
              variant="ghost"
              size="sm"
              onclick={() => {
                clearSelectionUnified();
              }}>{i18n.t('cleaner.clear_selection')}</Button
            >
          </div>
          <div
            class="rounded-md border h-[60vh] overflow-auto"
            style="overscroll-behavior: contain; overflow-anchor: none;"
            bind:this={unifiedContainer}
            onscroll={onUnifiedScroll}
          >
            <table class="w-full text-sm text-foreground">
              <thead class="bg-muted/40 text-xs">
                <tr>
                  <th class="px-3 py-2 text-left w-9">
                    <Checkbox
                      checked={selectedPaths.size > 0 &&
                        selectedPaths.size === allItems.length &&
                        allItems.length > 0}
                      onCheckedChange={() =>
                        setSelectionForKind(filterKind === 'all' ? 'all' : filterKind)}
                    />
                  </th>
                  <th class="px-3 py-2 text-left">{i18n.t('cleaner.path')}</th>
                  <th class="px-3 py-2 text-left">{i18n.t('cleaner.type')}</th>
                  <th class="px-3 py-2 text-left w-30">{i18n.t('common.size')}</th>
                  <th class="px-3 py-2 text-left w-40">{i18n.t('cleaner.actions')}</th>
                </tr>
              </thead>
              <tbody>
                {#if !isDeferredLoaded}
                  {#each Array.from({ length: 12 }) as _, i (i)}
                    <tr class="border-t">
                      <td class="px-3 py-2.5" colspan="5"
                        ><Skeleton class="h-4 w-full opacity-60" aria-hidden="true" /></td
                      >
                    </tr>
                  {/each}
                {:else if allItems.length === 0}
                  {#if scanning || isLoading}
                    {#each Array.from({ length: 16 }) as _, i (i)}
                      <tr class="border-t">
                        <td class="px-3 py-2" colspan="5"
                          ><Skeleton class="h-4 w-full" aria-hidden="true" /></td
                        >
                      </tr>
                    {/each}
                  {:else}
                    <tr>
                      <td colspan="5" class="px-3 py-6 text-center text-muted-foreground"
                        >{i18n.t('cleaner.no_items')}</td
                      >
                    </tr>
                  {/if}
                {:else}
                  {#if unifiedVirtualize && unifiedTopPad > 0}
                    <tr
                      ><td colspan="5" style={`height:${unifiedTopPad}px; overflow-anchor: none;`}
                      ></td></tr
                    >
                  {/if}
                  {#each unifiedDisplayList as it (it.path)}
                    <tr class="border-t h-10 align-middle">
                      <td class="px-3 py-2"
                        ><Checkbox
                          checked={selectedPaths.has(it.path)}
                          onCheckedChange={() => toggleSelectUnified(it.path)}
                        /></td
                      >
                      <td class="px-3 py-2"
                        ><span class="block truncate max-w-[60ch]" title={it.path}>{it.path}</span
                        ></td
                      >
                      <td class="px-3 py-2"
                        ><span
                          class="inline-flex items-center rounded border px-2 py-0.5 text-xs capitalize"
                          >{it.kind === 'temp'
                            ? i18n.t('cleaner.category_temp')
                            : it.kind === 'large'
                              ? i18n.t('cleaner.category_large')
                              : it.kind === 'duplicate'
                                ? i18n.t('cleaner.category_dup')
                                : it.kind === 'empty'
                                  ? i18n.t('cleaner.category_empty')
                                  : it.kind === 'shortcut'
                                    ? i18n.t('cleaner.category_shortcuts')
                                    : it.kind}</span
                        ></td
                      >
                      <td class="px-3 py-2">{it.size ? formatBytes(it.size) : '-'}</td>
                      <td class="px-3 py-2">
                        <DropdownMenu>
                          <DropdownMenuTrigger>
                            <Button type="button" variant="ghost" size="sm" aria-label="Details">
                              <Ellipsis class="size-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent
                            align="end"
                            onclick={(e: MouseEvent) => e.stopPropagation()}
                          >
                            <DropdownMenuItem
                              onclick={async (e: MouseEvent) => {
                                e.stopPropagation();
                                try {
                                  const { revealItemInDir } =
                                    await import('@tauri-apps/plugin-opener');
                                  await revealItemInDir(it.path);
                                } catch (error) {
                                  logCleanerError('Reveal item failed', error);
                                }
                              }}>{i18n.t('cleaner.reveal')}</DropdownMenuItem
                            >
                            <DropdownMenuItem
                              onclick={(e: MouseEvent) => {
                                e.stopPropagation();
                                addExclusion(it.path);
                              }}>{i18n.t('cleaner.exclude')}</DropdownMenuItem
                            >
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </td>
                    </tr>
                  {/each}
                  {#if unifiedVirtualize && unifiedBottomPad > 0}
                    <tr
                      ><td
                        colspan="5"
                        style={`height:${unifiedBottomPad}px; overflow-anchor: none;`}
                      ></td></tr
                    >
                  {/if}
                {/if}
              </tbody>
            </table>
          </div>
          {#if message || progressMessage}
            <p class="mt-1 text-xs text-muted-foreground">{progressMessage || message}</p>
          {/if}
        </div>
      </CardContent>
    </Card>
  </div>
</div>

<Dialog open={showSettings} onOpenChange={(v) => (showSettings = !!v)}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>{i18n.t('cleaner.cleaner_settings')}</DialogTitle>
      <DialogDescription>{i18n.t('cleaner.exclude_desc')}</DialogDescription>
    </DialogHeader>
    <div class="space-y-3">
      <div class="flex items-center gap-2">
        <Input
          id="ex-add"
          placeholder={i18n.t('cleaner.add_placeholder')}
          onkeydown={(e: KeyboardEvent) => {
            const input = e.currentTarget as HTMLInputElement;
            if (e.key === 'Enter') {
              addExclusion(input.value);
              input.value = '';
            }
          }}
        />
        <Button
          size="sm"
          onclick={() => {
            const el = document.getElementById('ex-add') as HTMLInputElement | null;
            if (el) {
              addExclusion(el.value);
              el.value = '';
            }
          }}>{i18n.t('cleaner.add')}</Button
        >
      </div>
      <div class="rounded border">
        <ul class="max-h-48 overflow-auto text-sm">
          {#if exclusions.length === 0}
            <li class="px-3 py-2 text-muted-foreground">{i18n.t('cleaner.no_exclusions')}</li>
          {:else}
            {#each exclusions as ex (ex)}
              <li class="flex items-center justify-between gap-2 border-b px-3 py-2">
                <span class="truncate" title={ex}>{ex}</span>
                <Button variant="ghost" size="sm" onclick={() => removeExclusion(ex)}
                  >{i18n.t('cleaner.remove')}</Button
                >
              </li>
            {/each}
          {/if}
        </ul>
      </div>
    </div>
    <DialogFooter>
      <Button variant="secondary" onclick={() => (showSettings = false)}
        >{i18n.t('common.close')}</Button
      >
    </DialogFooter>
  </DialogContent>
</Dialog>

<AlertDialog open={showConfirmationModal} onOpenChange={(v) => (showConfirmationModal = v)}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>{i18n.t('cleaner.delete_selected_title')}</AlertDialogTitle>
      <AlertDialogDescription>
        {i18n.t('cleaner.delete_selected_desc', { count: filesToDelete.length })}
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={cancelDeletion}>{i18n.t('common.cancel')}</AlertDialogCancel>
      <AlertDialogAction
        class="bg-destructive text-destructive-foreground hover:opacity-90"
        onclick={executeDeletion}
      >
        {i18n.t('cleaner.delete')}
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
        {progressMessage ||
          (isErasing ? i18n.t('cleaner.securely_erasing') : i18n.t('cleaner.working'))}
      </p>
    </div>
  </div>
{/if}
