<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
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
  import { loadCleanerCache, saveCleanerCache } from '$lib/cleanerCache';
  import { Trash2, Scan, Eraser, Ellipsis } from '@lucide/svelte';
  import { SvelteSet } from 'svelte/reactivity';

  interface FileEntry {
    path: string;
    size?: number;
  }

  let tempFiles = $state<FileEntry[]>([]);
  let largeFiles = $state<FileEntry[]>([]);
  let duplicateFiles = $state<FileEntry[]>([]);
  let emptyFolders = $state<FileEntry[]>([]);
  let brokenShortcuts = $state<FileEntry[]>([]);
  let message = $state('');
  let progressMessage = $state('');
  let isLoading = $state(false);
  let scanning = $state(false);
  let eraserPasses = $state(1);
  let isErasing = $state(false);
  let showConfirmationModal = $state(false);
  let filesToDelete = $state<string[]>([]);
  type Kind = 'temp' | 'large' | 'duplicate' | 'empty' | 'shortcut';
  let q = $state('');
  let qDeb = $state('');
  let filterKind = $state<'all' | Kind>('all');
  let largeMinMB = $state(100);
  let dupGroups = $state<Array<{ hash: string; size: number; files: string[] }>>([]);
  let selectedPaths = new SvelteSet<string>();
  let showSettings = $state(false);
  let exclusions = $state<string[]>([]);
  const EXC_KEY = 'avelonia_cleaner_exclusions_v1';

  function loadExclusions() {
    try {
      const raw = localStorage.getItem(EXC_KEY);
      exclusions = raw ? JSON.parse(raw) : [];
    } catch {
      exclusions = [];
    }
  }

  function saveExclusions() {
    try {
      localStorage.setItem(
        EXC_KEY,
        JSON.stringify(Array.from(new Set(exclusions.map((s) => s.trim()).filter(Boolean))))
      );
    } catch { /* noop */ }
  }

  onMount(() => {
    loadExclusions();
  });

  onMount(() => {
    try {
      const cache = loadCleanerCache();
      if (cache) {
        tempFiles = Array.isArray(cache.tempFiles) ? cache.tempFiles : [];
        largeFiles = Array.isArray(cache.largeFiles) ? cache.largeFiles : [];
        duplicateFiles = Array.isArray(cache.duplicateFiles) ? cache.duplicateFiles : [];
        emptyFolders = Array.isArray(cache.emptyFolders) ? cache.emptyFolders : [];
        brokenShortcuts = Array.isArray(cache.brokenShortcuts) ? cache.brokenShortcuts : [];
        dupGroups = Array.isArray(cache.dupGroups) ? cache.dupGroups : [];
      }
    } catch { /* noop */ }
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
    } catch { /* noop */ }
    _cacheSaveTimer = setTimeout(() => {
      try {
        saveCleanerCache({
          tempFiles,
          largeFiles,
          duplicateFiles,
          emptyFolders,
          brokenShortcuts,
          dupGroups,
          timestamp: Date.now(),
        });
      } catch { /* noop */ }
    }, 500) as unknown as number;
  }

  $effect(() => {
    void tempFiles;
    void largeFiles;
    void duplicateFiles;
    void emptyFolders;
    void brokenShortcuts;
    void dupGroups;
    saveCacheSoon();
  });

  $effect(() => {
    const t = setTimeout(() => (qDeb = q), 150);
    return () => clearTimeout(t);
  });

  function matchesExclusion(p: string): boolean {
    try {
      for (const ex of exclusions) {
        if (!ex) continue;
        if (p.toLowerCase().includes(ex.toLowerCase())) return true;
      }
    } catch { /* noop */ }
    return false;
  }

  type CleanerItem = { path: string; size?: number; kind: Kind; groupId?: string };

  function resolveCleanerItems(value: CleanerItem[] | (() => CleanerItem[])) {
    return typeof value === 'function' ? (value as () => CleanerItem[])() : value;
  }
  let unifiedCap = $state(3000);
  const UNIFIED_BUILD_STEP = 2000;
  const allItems = $derived.by<CleanerItem[]>(() => {
    const cap = Math.max(500, unifiedCap);
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
      if (items.length >= cap) return items;
    }
    for (const f of largeFiles) {
      tryPush({ path: f.path, size: f.size, kind: 'large' });
      if (items.length >= cap) return items;
    }
    for (const f of duplicateFiles) {
      tryPush({ path: f.path, size: f.size, kind: 'duplicate' });
      if (items.length >= cap) return items;
    }
    for (const f of emptyFolders) {
      tryPush({ path: f.path, size: f.size, kind: 'empty' });
      if (items.length >= cap) return items;
    }
    for (const f of brokenShortcuts) {
      tryPush({ path: f.path, size: f.size, kind: 'shortcut' });
      if (items.length >= cap) return items;
    }
    if (dupGroups.length > 0) {
      for (const g of dupGroups) {
        for (const p of g.files) {
          tryPush({ path: p, size: g.size, kind: 'duplicate', groupId: g.hash });
          if (items.length >= cap) return items;
        }
      }
    }
    return items;
  });

  const getAllItemsList = () => resolveCleanerItems(allItems);

  $effect(() => {
    const _fk = filterKind;
    const _q = qDeb;
    const _exc = exclusions.length;
    unifiedCap = 3000;
  });

  const selectedCount = $derived(selectedPaths.size);
  const selectedSize = $derived.by(() => {
    let sum = 0;
    const s = selectedPaths;
    for (const it of getAllItemsList()) {
      if (s.has(it.path)) sum += it.size ?? 0;
    }
    return sum;
  });

  const MAX_TEMP_ITEMS = 20000;
  let tempTruncated = $state(false);
  let tempReportedTotal = $state(0);
  let tempQueue: string[] = [];
  let tempFlushRaf: number | null = null;
  const TEMP_FLUSH_IDLE_MS = 350;

  function scheduleTempFlush() {
    if (tempFlushRaf !== null) return;
    const run = () => {
      tempFlushRaf = null;
      if (tempQueue.length === 0) return;
      try {
        const now = Date.now();
        const unifiedActive = now - _unifiedLastScrollTs < TEMP_FLUSH_IDLE_MS;
        const unifiedUp = _unifiedLastDir === 'up' && unifiedActive;
        if (unifiedActive || unifiedUp) {
          tempFlushRaf = setTimeout(run, TEMP_FLUSH_IDLE_MS) as unknown as number;
          return;
        }
      } catch { /* noop */ }

      const take = tempQueue.splice(0, Math.min(scanning ? 80 : 400, tempQueue.length));
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
    };
    tempFlushRaf = ('requestIdleCallback' in window
      ? (window as any).requestIdleCallback(run, { timeout: 120 })
      : setTimeout(run, 0)) as unknown as number;
  }

  type FilePair = [string, number];
  let largeQueue: FilePair[] = [];
  let largeFlushRaf: number | null = null;
  function scheduleLargeFlush() {
    if (largeFlushRaf !== null) return;
    const run = () => {
      largeFlushRaf = null;
      if (largeQueue.length === 0) return;
      try {
        const now = Date.now();
        const userActive = now - _unifiedLastScrollTs < 600;
        const scrollingUp = _unifiedLastDir === 'up' && userActive;
        if (scrollingUp) {
          largeFlushRaf = setTimeout(run, 300) as unknown as number;
          return;
        }
      } catch { /* noop */ }
      const take = largeQueue.splice(0, Math.min(800, largeQueue.length));
      const next = take.map(([p, s]) => ({ path: p, size: s }));
      if (next.length) largeFiles = [...largeFiles, ...next];
      if (largeQueue.length > 0) scheduleLargeFlush();
    };
    largeFlushRaf = ('requestIdleCallback' in window
      ? (window as any).requestIdleCallback(run, { timeout: 120 })
      : setTimeout(run, 0)) as unknown as number;
  }

  let dupGroupsQueue: Array<{ hash: string; size: number; files: string[] }> = [];
  let dupFlushRaf: number | null = null;
  function scheduleDupFlush() {
    if (dupFlushRaf !== null) return;
    const run = () => {
      dupFlushRaf = null;
      if (dupGroupsQueue.length === 0) return;
      try {
        const now = Date.now();
        const userActive = now - _unifiedLastScrollTs < 600;
        const scrollingUp = _unifiedLastDir === 'up' && userActive;
        if (scrollingUp) {
          dupFlushRaf = setTimeout(run, 300) as unknown as number;
          return;
        }
      } catch { /* noop */ }
      const take = dupGroupsQueue.splice(0, Math.min(60, dupGroupsQueue.length));
      if (take.length) {
        dupGroups = [...dupGroups, ...take];
        const flat = take.flatMap((g) =>
          (g.files || []).map((p) => ({ path: p as string, size: g.size }))
        );
        if (flat.length) duplicateFiles = [...duplicateFiles, ...flat];
      }
      if (dupGroupsQueue.length > 0) scheduleDupFlush();
    };
    dupFlushRaf = ('requestIdleCallback' in window
      ? (window as any).requestIdleCallback(run, { timeout: 120 })
      : setTimeout(run, 0)) as unknown as number;
  }

  let emptyQueue: string[] = [];
  let emptyFlushRaf: number | null = null;
  function scheduleEmptyFlush() {
    if (emptyFlushRaf !== null) return;
    const run = () => {
      emptyFlushRaf = null;
      if (emptyQueue.length === 0) return;
      try {
        const now = Date.now();
        const userActive = now - _unifiedLastScrollTs < 600;
        const scrollingUp = _unifiedLastDir === 'up' && userActive;
        if (scrollingUp) {
          emptyFlushRaf = setTimeout(run, 300) as unknown as number;
          return;
        }
      } catch { /* noop */ }
      const take = emptyQueue.splice(0, Math.min(1200, emptyQueue.length));
      const next = take.map((p) => ({ path: p }));
      if (next.length) emptyFolders = [...emptyFolders, ...next];
      if (emptyQueue.length > 0) scheduleEmptyFlush();
    };
    emptyFlushRaf = ('requestIdleCallback' in window
      ? (window as any).requestIdleCallback(run, { timeout: 120 })
      : setTimeout(run, 0)) as unknown as number;
  }

  let shortcutQueue: string[] = [];
  let shortcutFlushRaf: number | null = null;
  function scheduleShortcutFlush() {
    if (shortcutFlushRaf !== null) return;
    const run = () => {
      shortcutFlushRaf = null;
      if (shortcutQueue.length === 0) return;
      try {
        const now = Date.now();
        const userActive = now - _unifiedLastScrollTs < 600;
        const scrollingUp = _unifiedLastDir === 'up' && userActive;
        if (scrollingUp) {
          shortcutFlushRaf = setTimeout(run, 300) as unknown as number;
          return;
        }
      } catch { /* noop */ }
      const take = shortcutQueue.splice(0, Math.min(1200, shortcutQueue.length));
      const next = take.map((p) => ({ path: p }));
      if (next.length) brokenShortcuts = [...brokenShortcuts, ...next];
      if (shortcutQueue.length > 0) scheduleShortcutFlush();
    };
    shortcutFlushRaf = ('requestIdleCallback' in window
      ? (window as any).requestIdleCallback(run, { timeout: 120 })
      : setTimeout(run, 0)) as unknown as number;
  }

  let unifiedContainer = $state<HTMLDivElement | null>(null);
  let _unifiedLastScrollTs = 0;
  let _unifiedLastScrollTop = 0;
  let _unifiedLastDir: 'up' | 'down' | null = null;
  let UNIFIED_ROW_PX = $state(40);
  const UNIFIED_PREBUFFER = 3;
  const UNIFIED_MAX_DOM = 600;
  let unifiedVirtualize = $state(true);
  let unifiedAutoFallback = $state(false);

  $effect(() => {
    unifiedVirtualize = !unifiedAutoFallback && (scanning || getAllItemsList().length > 1500);
  });

  let unifiedStart = $state(0);
  const unifiedRowsInView = $derived(() => {
    const h = unifiedContainer?.clientHeight ?? 480;
    return Math.ceil(h / UNIFIED_ROW_PX) + 20;
  });

  const unifiedDisplayed = $derived(() => {
    if (!unifiedVirtualize) return getAllItemsList();
    const items = getAllItemsList();
    const visibleRows = Math.min(unifiedRowsInView(), UNIFIED_MAX_DOM);
    const end = Math.min(items.length, unifiedStart + visibleRows);
    return items.slice(unifiedStart, end);
  });

  const getUnifiedDisplayList = () => resolveCleanerItems(unifiedDisplayed);

  let unifiedPadPx = $state(0);
  const unifiedTopPad = $derived(unifiedVirtualize ? unifiedPadPx : 0);
  const unifiedBottomPad = $derived(
    unifiedVirtualize
      ? Math.max(
          0,
          getAllItemsList().length * UNIFIED_ROW_PX -
            (unifiedTopPad + unifiedDisplayed.length * UNIFIED_ROW_PX)
        )
      : 0
  );

  $effect(() => {
    if (!unifiedVirtualize) {
      unifiedStart = 0;
      return;
    }
    const maxStart = Math.max(
      0,
      getAllItemsList().length - Math.min(unifiedRowsInView(), UNIFIED_MAX_DOM)
    );
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
    const _len = getAllItemsList().length;
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
      } catch { /* noop */ }
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
    } catch { /* noop */ }
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
      const now = Date.now();
      const top = el.scrollTop;
      _unifiedLastDir =
        top < _unifiedLastScrollTop ? 'up' : top > _unifiedLastScrollTop ? 'down' : _unifiedLastDir;
      _unifiedLastScrollTop = top;
      _unifiedLastScrollTs = now;
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
        } catch { /* noop */ }
      }
      _unifiedScrollTick = false;
    });
  }

  async function statTempSizes() {
    try {
      const paths = tempFiles.map((f) => f.path);
      if (paths.length === 0) return;
      const res = (await invoke('stat_paths', { paths })) as [string, number][];
      const map = new Map(res);
      tempFiles = tempFiles.map((f) => ({ path: f.path, size: map.get(f.path) ?? f.size }));
    } catch { /* noop */ }
  }

  async function scanAll() {
    if (scanning || isLoading) return;
    tempFiles = [];
    largeFiles = [];
    duplicateFiles = [];
    emptyFolders = [];
    brokenShortcuts = [];
    dupGroups = [];
    selectedPaths = new SvelteSet();
    tempQueue = [];
    tempTruncated = false;
    tempReportedTotal = 0;
    scanning = true;
    progressMessage = '';
    message = '';
    try {
      const minBytes = Math.max(1, largeMinMB) * 1024 * 1024;
      void invoke('start_cleaner_scan', { min_size_bytes: minBytes, max_temp: MAX_TEMP_ITEMS });
    } catch (e) {
      console.error(e);
      toast.error('Scan failed to start');
      scanning = false;
    }
  }

  function toggleSelectUnified(p: string) {
    const next = new SvelteSet(selectedPaths);
    if (next.has(p)) next.delete(p);
    else next.add(p);
    selectedPaths = next;
  }

  function clearSelectionUnified() {
    selectedPaths = new SvelteSet();
  }

  function setSelectionForKind(kind: 'all' | Kind) {
    const next = new SvelteSet(selectedPaths);
    for (const it of getAllItemsList()) {
      if (kind === 'all' || it.kind === kind) next.add(it.path);
    }
    selectedPaths = next;
  }

  async function deleteSelectedUnified() {
    const files = Array.from(selectedPaths);
    if (files.length === 0) return;
    filesToDelete = files;
    showConfirmationModal = true;
  }

  async function moveSelectedUnified() {
    const files = Array.from(selectedPaths);
    if (files.length === 0) return;
    try {
      const dest = await open({ directory: true });
      if (!dest || typeof dest !== 'string') return;
      isLoading = true;
      message = 'Moving selected...';
      const moved = (await invoke('move_files', { files, destination: dest })) as number;
      tempFiles = tempFiles.filter((f) => !selectedPaths.has(f.path));
      largeFiles = largeFiles.filter((f) => !selectedPaths.has(f.path));
      duplicateFiles = duplicateFiles.filter((f) => !selectedPaths.has(f.path));
      emptyFolders = emptyFolders.filter((f) => !selectedPaths.has(f.path));
      brokenShortcuts = brokenShortcuts.filter((f) => !selectedPaths.has(f.path));
      for (const g of dupGroups) {
        g.files = g.files.filter((p) => !selectedPaths.has(p));
      }
      dupGroups = dupGroups.filter((g) => g.files.length > 1);
      clearSelectionUnified();
      toast.success(`Moved ${moved} item(s)`);
    } catch (e) {
      console.error(e);
      toast.error('Move failed');
    } finally {
      isLoading = false;
    }
  }

  async function secureEraseSelectedUnified() {
    const files = Array.from(selectedPaths);
    if (files.length === 0) return;
    try {
      isErasing = true;
      await invoke('secure_erase', { files, passes: eraserPasses });
      toast.success('Secure erase done');
      clearSelectionUnified();
    } catch (e) {
      console.error(e);
      toast.error('Secure erase failed');
    } finally {
      isErasing = false;
    }
  }

  function autoSelectDuplicatesKeepOne() {
    if (dupGroups.length === 0) return;
    const next = new SvelteSet(selectedPaths);
    for (const g of dupGroups) {
      if (!g.files || g.files.length < 2) continue;
      const sorted = [...g.files].sort();
      for (let i = 1; i < sorted.length; i++) next.add(sorted[i]);
    }
    selectedPaths = next;
    filterKind = 'duplicate';
  }

  onMount(() => {
    const unsubs: Array<() => void> = [];

    listen('scan_progress', (event) => {
      const typedWindow = window as Window & { __lastProg?: number };
      if ((window.performance?.now?.() ?? Date.now()) - (typedWindow.__lastProg || 0) > 300) {
        progressMessage = event.payload as string;
        typedWindow.__lastProg = window.performance?.now?.() ?? Date.now();
      }
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});

    listen('cleaner-temp-batch', (event) => {
      try {
        const arr = (event.payload as string[]) || [];
        if (Array.isArray(arr) && arr.length) {
          for (const p of arr) {
            if (tempFiles.length + tempQueue.length >= MAX_TEMP_ITEMS) {
              tempTruncated = true;
              break;
            }
            tempQueue.push(String(p));
          }
          scheduleTempFlush();
        }
      } catch { /* noop */ }
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});

    listen('cleaner-temp-done', (event) => {
      try {
        const payload = event.payload as { total?: number };
        tempReportedTotal = Number(payload?.total || 0);
      } catch {
        tempReportedTotal = 0;
      }
      if (tempQueue.length > 0) {
        while (tempQueue.length > 0) {
          const take = tempQueue.splice(0, Math.min(2000, tempQueue.length));
          const next = take.filter((p) => !matchesExclusion(p)).map((p) => ({ path: p }));
          if (next.length) {
            const remaining = Math.max(0, MAX_TEMP_ITEMS - tempFiles.length);
            if (remaining <= 0) {
              tempTruncated = true;
              break;
            }
            const append = next.slice(0, remaining);
            tempFiles = [...tempFiles, ...append];
            if (append.length < next.length) {
              tempTruncated = true;
              break;
            }
          }
        }
      }
      if (tempFiles.length > 0 && tempFiles.length <= 2000) {
        void statTempSizes();
      }
      scanning = false;
      if (tempTruncated) {
        toast.warning(`Showing first ${tempFiles.length} items (truncated). Refine filters.`);
      } else {
        toast.success(`Found ${tempReportedTotal || tempFiles.length} temporary files.`);
      }
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});

    listen('cleaner-large-batch', (event) => {
      try {
        const arr = (event.payload as [string, number][]) || [];
        if (Array.isArray(arr) && arr.length) {
          for (const it of arr) largeQueue.push([String(it[0]), Number(it[1])]);
          scheduleLargeFlush();
        }
      } catch { /* noop */ }
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});

    listen('cleaner-dup-groups-batch', (event) => {
      try {
        const groups =
          (event.payload as Array<{ hash: string; size: number; files: string[] }>) || [];
        if (Array.isArray(groups) && groups.length) {
          for (const g of groups) dupGroupsQueue.push(g);
          scheduleDupFlush();
        }
      } catch { /* noop */ }
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});

    listen('cleaner-empty-batch', (event) => {
      try {
        const arr = (event.payload as string[]) || [];
        if (Array.isArray(arr) && arr.length) {
          for (const p of arr) emptyQueue.push(String(p));
          scheduleEmptyFlush();
        }
      } catch { /* noop */ }
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});

    listen('cleaner-shortcut-batch', (event) => {
      try {
        const arr = (event.payload as string[]) || [];
        if (Array.isArray(arr) && arr.length) {
          for (const p of arr) shortcutQueue.push(String(p));
          scheduleShortcutFlush();
        }
      } catch { /* noop */ }
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});

    listen('cleaner-done', (event) => {
      try {
        const scope = (event.payload as any)?.scope as string | undefined;
      } catch { /* noop */ }
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});
    listen('cleaner-stopped', () => {
      scanning = false;
      isLoading = false;
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});
    listen('cleaner-error', (ev) => {
      try {
        toast.error(String(ev.payload || 'Scan error'));
      } catch { /* noop */ }
    })
      .then((fn) => unsubs.push(fn))
      .catch(() => {});

    return () => {
      for (const u of unsubs) {
        try {
          u();
        } catch { /* noop */ }
      }
    };
  });

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
    } catch { /* noop */ }
    scanning = false;
    tempQueue = [];
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

      tempFiles = tempFiles.filter((f) => !filesToDelete.includes(f.path));
      largeFiles = largeFiles.filter((f) => !filesToDelete.includes(f.path));
      duplicateFiles = duplicateFiles.filter((f) => !filesToDelete.includes(f.path));
      emptyFolders = emptyFolders.filter((f) => !filesToDelete.includes(f.path));
      brokenShortcuts = brokenShortcuts.filter((f) => !filesToDelete.includes(f.path));

      filesToDelete = [];
      selectedPaths = new SvelteSet();
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
</script>

<div class="space-y-6 text-foreground">
  <Card>
    <CardHeader>
      <div class="flex items-center justify-between gap-2">
        <div>
          <CardTitle class="text-2xl">Cleaner</CardTitle>
          <CardDescription>Scan, review and clean safely.</CardDescription>
        </div>

        <span class="text-xs text-muted-foreground"
          >Counts ? Temp: {tempFiles.length}, Large: {largeFiles.length}, Dups: {dupGroups.length},
          Empty: {emptyFolders.length}, Shortcuts: {brokenShortcuts.length}</span
        >
      </div>
    </CardHeader>
    <CardContent>
      <div class="flex flex-col gap-3">
        <div class="flex flex-wrap items-center gap-2">
          <Input placeholder="Search path..." bind:value={q} class="w-[280px]" />
          <div class="flex items-center gap-2">
            <Label for="kind">Type</Label>
            <Select
              type="single"
              bind:value={filterKind}
            >
              <SelectTrigger id="kind" class="w-[120px]">
                <p class="truncate">
                  {filterKind === 'all'
                    ? 'All'
                    : filterKind === 'temp'
                    ? 'Temp'
                    : filterKind === 'large'
                    ? 'Large'
                    : filterKind === 'duplicate'
                    ? 'Duplicates'
                    : filterKind === 'empty'
                    ? 'Empty'
                    : filterKind === 'shortcut'
                    ? 'Shortcuts'
                    : filterKind}
                </p>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All</SelectItem>
                <SelectItem value="temp">Temp</SelectItem>
                <SelectItem value="large">Large</SelectItem>
                <SelectItem value="duplicate">Duplicates</SelectItem>
                <SelectItem value="empty">Empty</SelectItem>
                <SelectItem value="shortcut">Shortcuts</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="ml-auto flex items-center gap-2">
            <Button onclick={scanAll} disabled={isLoading}><Scan class="h-4 w-4" />Scan All</Button>
            {#if scanning}
              <Button variant="secondary" onclick={stopScan} title="Stop scanning">Stop</Button>
            {/if}
            <Button variant="secondary" onclick={() => (showSettings = true)} aria-label="Settings"
              >Settings</Button
            >
            <Button
              variant="secondary"
              onclick={autoSelectDuplicatesKeepOne}
              title="Auto-select duplicate copies">Auto-select Duplicates</Button
            >
          </div>
        </div>
        <div class="flex items-center gap-2">
          <Button
            variant="destructive"
            disabled={selectedPaths.size === 0 || isLoading}
            onclick={deleteSelectedUnified}
            ><Trash2 class="h-4 w-4" />Delete Selected ({selectedCount})</Button
          >
          <Button
            variant="secondary"
            disabled={selectedPaths.size === 0 || isLoading}
            onclick={moveSelectedUnified}>Move Selected</Button
          >
          <Button
            variant="secondary"
            disabled={selectedPaths.size === 0 || isErasing}
            onclick={secureEraseSelectedUnified}><Eraser class="h-4 w-4" />Secure Erase</Button
          >
          <span class="text-xs text-muted-foreground"
            >Selected size: {formatBytes(selectedSize)}</span
          >
          <Button
            variant="ghost"
            size="sm"
            onclick={() => {
              selectedPaths = new SvelteSet();
            }}>Clear selection</Button
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
                      selectedPaths.size === getAllItemsList().length &&
                      getAllItemsList().length > 0}
                    onCheckedChange={() =>
                      setSelectionForKind(filterKind === 'all' ? 'all' : filterKind)}
                  />
                </th>
                <th class="px-3 py-2 text-left">Path</th>
                <th class="px-3 py-2 text-left">Type</th>
                <th class="px-3 py-2 text-left w-30">Size</th>
                <th class="px-3 py-2 text-left w-40">Actions</th>
              </tr>
            </thead>
            <tbody>
              {#if getAllItemsList().length === 0}
                {#if scanning || isLoading}
                  {#each Array.from({ length: 16 }) as _, i}
                    <tr class="border-t">
                      <td class="px-3 py-2" colspan="5"
                        ><Skeleton class="h-4 w-full" aria-hidden="true" /></td
                      >
                    </tr>
                  {/each}
                {:else}
                  <tr>
                    <td colspan="5" class="px-3 py-6 text-center text-muted-foreground"
                      >No items. Click Scan All.</td
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
                {#each (
                  getUnifiedDisplayList().length > 0
                    ? getUnifiedDisplayList()
                    : getAllItemsList().slice(
                        0,
                        Math.min(getAllItemsList().length, UNIFIED_MAX_DOM)
                      )
                ) as it (it.path)}
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
                        >{it.kind}</span
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
                            onclick={async (e: MouseEvent) => { e.stopPropagation(); try { const { revealItemInDir } = await import('@tauri-apps/plugin-opener'); await revealItemInDir(it.path); } catch { /* noop */ } }}
                            >Reveal</DropdownMenuItem
                          >
                          <DropdownMenuItem
                            onclick={(e: MouseEvent) => {
                              e.stopPropagation();
                              addExclusion(it.path);
                            }}>Exclude</DropdownMenuItem
                          >
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </td>
                  </tr>
                {/each}
                {#if unifiedVirtualize && unifiedBottomPad > 0}
                  <tr
                    ><td colspan="5" style={`height:${unifiedBottomPad}px; overflow-anchor: none;`}
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

<Dialog open={showSettings} onOpenChange={(v) => (showSettings = !!v)}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Cleaner Settings</DialogTitle>
      <DialogDescription>Exclude paths containing these patterns from results.</DialogDescription>
    </DialogHeader>
    <div class="space-y-3">
      <div class="flex items-center gap-2">
        <Input
          id="ex-add"
          placeholder="Add path substring, e.g. C:\\Games"
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
          }}>Add</Button
        >
      </div>
      <div class="rounded border">
        <ul class="max-h-48 overflow-auto text-sm">
          {#if exclusions.length === 0}
            <li class="px-3 py-2 text-muted-foreground">No exclusions</li>
          {:else}
            {#each exclusions as ex (ex)}
              <li class="flex items-center justify-between gap-2 border-b px-3 py-2">
                <span class="truncate" title={ex}>{ex}</span>
                <Button variant="ghost" size="sm" onclick={() => removeExclusion(ex)}>Remove</Button
                >
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
      <AlertDialogAction
        class="bg-destructive text-destructive-foreground hover:opacity-90"
        onclick={executeDeletion}
      >
        Delete
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
