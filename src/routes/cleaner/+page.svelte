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
  import { loadCleanerCache, saveCleanerCache } from '$lib/cleanerCache';

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
  let isLoading = $state(false);
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
  let qDeb = $state('');
  $effect(() => { const t = setTimeout(() => (qDeb = q), 150); return () => clearTimeout(t); });
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
  // Restore previous scan results when entering Cleaner
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
        if (view === 'unified') filterKind = 'all';
      }
    } catch {}
  });
  function addExclusion(pattern: string) {
    const p = (pattern||'').trim(); if (!p) return; exclusions = Array.from(new Set([...exclusions, p])); saveExclusions();
  }
  function removeExclusion(pattern: string) { exclusions = exclusions.filter((s)=>s!==pattern); saveExclusions(); }

  // Debounced cache saves so streaming updates donG??t spam storage
  let _cacheSaveTimer: number | null = null;
  function saveCacheSoon() {
    try { if (_cacheSaveTimer) clearTimeout(_cacheSaveTimer as unknown as number); } catch {}
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
      } catch {}
    }, 500) as unknown as number;
  }
  $effect(() => {
    // track array state and persist lazily
    void tempFiles; void largeFiles; void duplicateFiles; void emptyFolders; void brokenShortcuts; void dupGroups;
    saveCacheSoon();
  });

  function matchesExclusion(p: string): boolean {
    try {
      for (const ex of exclusions) { if (!ex) continue; if (p.toLowerCase().includes(ex.toLowerCase())) return true; }
    } catch {}
    return false;
  }

  type CleanerItem = { path: string; size?: number; kind: Kind; groupId?: string };
  let unifiedCap = $state(3000);
  const UNIFIED_BUILD_STEP = 2000;
  const allItems = $derived.by<CleanerItem[]>(() => {
    const cap = Math.max(500, unifiedCap);
    const items: CleanerItem[] = [];
    const term = qDeb.trim().toLowerCase();
    function tryPush(it: CleanerItem) {
      if (matchesExclusion(it.path)) return;
      if (filterKind !== 'all' && it.kind !== filterKind) return;
      if (term && !it.path.toLowerCase().includes(term)) return;
      items.push(it);
    }
    // Push per-category, stop early once cap reached
    for (const f of tempFiles) { tryPush({ path: f.path, size: f.size, kind: 'temp' }); if (items.length >= cap) return items; }
    for (const f of largeFiles) { tryPush({ path: f.path, size: f.size, kind: 'large' }); if (items.length >= cap) return items; }
    for (const f of duplicateFiles) { tryPush({ path: f.path, size: f.size, kind: 'duplicate' }); if (items.length >= cap) return items; }
    for (const f of emptyFolders) { tryPush({ path: f.path, size: f.size, kind: 'empty' }); if (items.length >= cap) return items; }
    for (const f of brokenShortcuts) { tryPush({ path: f.path, size: f.size, kind: 'shortcut' }); if (items.length >= cap) return items; }
    if (dupGroups.length > 0) {
      for (const g of dupGroups) {
        for (const p of g.files) { tryPush({ path: p, size: g.size, kind: 'duplicate', groupId: g.hash }); if (items.length >= cap) return items; }
      }
    }
    return items;
  });

  // Grow cap when user scrolls near bottom (only on downward scroll)
  // Reset cap when filters/search/view change significantly
  $effect(() => {
    const _fk = filterKind; const _q = qDeb; const _exc = exclusions.length; const _v = view;
    unifiedCap = 3000;
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
  const TEMP_FLUSH_IDLE_MS = 350; // wait for user to be idle before mutating the list
  // Track temp-list user scroll activity to avoid fighting with the user
  let _tempLastScrollTs = 0
  function scheduleTempFlush() {
    if (tempFlushRaf !== null) return;
    const run = () => {
      tempFlushRaf = null;
      if (tempQueue.length === 0) return;
      try {
        const now = Date.now();
        const inUnified = view === 'unified';
        const unifiedActive = now - _unifiedLastScrollTs < TEMP_FLUSH_IDLE_MS;
        const unifiedUp = _unifiedLastDir === 'up' && unifiedActive;
        const tempActive = now - _tempLastScrollTs < TEMP_FLUSH_IDLE_MS;
        if ((inUnified && (unifiedActive || unifiedUp)) || (!inUnified && tempActive)) {
          tempFlushRaf = setTimeout(run, TEMP_FLUSH_IDLE_MS) as unknown as number;
          return;
        }
      } catch {}

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
    // Use a microtask to allow layout to settle, then run (or get delayed by idle gate above)
    tempFlushRaf = (('requestIdleCallback' in window) ? (window as any).requestIdleCallback(run, { timeout: 120 }) : setTimeout(run, 0)) as unknown as number;
  }

  // Queues for other categories (throttled to avoid UI thrash)
  type FilePair = [string, number];
  let largeQueue: FilePair[] = [];
  let largeFlushRaf: number | null = null;
  function scheduleLargeFlush() {
    if (largeFlushRaf !== null) return;
    const run = () => {
      largeFlushRaf = null;
      if (largeQueue.length === 0) return;
      // Pause while user scrollar upp i unified
      try {
        const now = Date.now();
        const userActive = now - _unifiedLastScrollTs < 600;
        const scrollingUp = _unifiedLastDir === 'up' && userActive;
        if (view === 'unified' && scrollingUp) { largeFlushRaf = setTimeout(run, 300) as unknown as number; return; }
      } catch {}
      const take = largeQueue.splice(0, Math.min(800, largeQueue.length));
      const next = take.map(([p, s]) => ({ path: p, size: s }));
      if (next.length) largeFiles = [...largeFiles, ...next];
      if (largeQueue.length > 0) scheduleLargeFlush();
    };
    largeFlushRaf = (('requestIdleCallback' in window) ? (window as any).requestIdleCallback(run, { timeout: 120 }) : setTimeout(run, 0)) as unknown as number;
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
        if (view === 'unified' && scrollingUp) { dupFlushRaf = setTimeout(run, 300) as unknown as number; return; }
      } catch {}
      const take = dupGroupsQueue.splice(0, Math.min(60, dupGroupsQueue.length));
      if (take.length) {
        dupGroups = [...dupGroups, ...take];
        const flat = take.flatMap((g) => (g.files || []).map((p) => ({ path: p as string, size: g.size })));
        if (flat.length) duplicateFiles = [...duplicateFiles, ...flat];
      }
      if (dupGroupsQueue.length > 0) scheduleDupFlush();
    };
    dupFlushRaf = (('requestIdleCallback' in window) ? (window as any).requestIdleCallback(run, { timeout: 120 }) : setTimeout(run, 0)) as unknown as number;
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
        if (view === 'unified' && scrollingUp) { emptyFlushRaf = setTimeout(run, 300) as unknown as number; return; }
      } catch {}
      const take = emptyQueue.splice(0, Math.min(1200, emptyQueue.length));
      const next = take.map((p) => ({ path: p }));
      if (next.length) emptyFolders = [...emptyFolders, ...next];
      if (emptyQueue.length > 0) scheduleEmptyFlush();
    };
    emptyFlushRaf = (('requestIdleCallback' in window) ? (window as any).requestIdleCallback(run, { timeout: 120 }) : setTimeout(run, 0)) as unknown as number;
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
        if (view === 'unified' && scrollingUp) { shortcutFlushRaf = setTimeout(run, 300) as unknown as number; return; }
      } catch {}
      const take = shortcutQueue.splice(0, Math.min(1200, shortcutQueue.length));
      const next = take.map((p) => ({ path: p }));
      if (next.length) brokenShortcuts = [...brokenShortcuts, ...next];
      if (shortcutQueue.length > 0) scheduleShortcutFlush();
    };
    shortcutFlushRaf = (('requestIdleCallback' in window) ? (window as any).requestIdleCallback(run, { timeout: 120 }) : setTimeout(run, 0)) as unknown as number;
  }

  // Virtualization for unified table (simple window without skeleton gating)
  let unifiedContainer = $state<HTMLDivElement | null>(null);
  // Track recent user scroll interaction to avoid fighting with anchoring
  let _unifiedLastScrollTs = 0;
  let _unifiedLastScrollTop = 0;
  let _unifiedLastDir: 'up' | 'down' | null = null;
  let UNIFIED_ROW_PX = $state(40); // measured row height for unified list
  const UNIFIED_MAX_DOM = 600;
  // Fallback: disable virtualization to avoid sticking/teleporting under extreme item counts
  // Set to true to restore virtualized windowing.
  let unifiedVirtualize = $state(true);
  let unifiedAutoFallback = $state(false);
  $effect(() => { unifiedVirtualize = (!unifiedAutoFallback) && (scanning || allItems.length > 1500); });
  let unifiedStart = $state(0);
  const unifiedRowsInView = $derived(() => {
    const h = unifiedContainer?.clientHeight ?? 480;
    return Math.ceil(h / UNIFIED_ROW_PX) + 20;
  });
  const unifiedDisplayed = $derived(() => {
    if (!unifiedVirtualize) return allItems;
    const end = Math.min(allItems.length, unifiedStart + Math.min(unifiedRowsInView, UNIFIED_MAX_DOM));
    return allItems.slice(unifiedStart, end);
  });
  const unifiedTopPad = $derived(unifiedVirtualize ? (unifiedStart * UNIFIED_ROW_PX) : 0);
  const unifiedBottomPad = $derived(unifiedVirtualize ? Math.max(0, (allItems.length - (unifiedStart + unifiedDisplayed.length)) * UNIFIED_ROW_PX) : 0);
  // Clamp virtual window when data size or viewport changes to avoid empty windows and stuck scrolling
  $effect(() => {
    if (!unifiedVirtualize) { unifiedStart = 0; return; }
    const maxStart = Math.max(0, allItems.length - Math.min(unifiedRowsInView, UNIFIED_MAX_DOM));
    if (unifiedStart > maxStart) unifiedStart = maxStart;
    if (unifiedStart < 0) unifiedStart = 0;
  });
  // Reset position on major context changes (view, filters, search, exclusions)
  $effect(() => { void view; void filterKind; void qDeb; void exclusions.length; unifiedStart = 0; _unifiedLastScrollTop = 0; });
  // Auto fallback: if content suggests more rows but container has no scroll range, disable virtualization
  $effect(() => {
    const _len = allItems.length; void _len; // track changes
    setTimeout(() => {
      try {
        const el = unifiedContainer as HTMLElement | null;
        if (!el || unifiedAutoFallback) return;
        const rowsVis = Math.ceil((el.clientHeight || 0) / UNIFIED_ROW_PX) + 5;
        if (_len > rowsVis && el.scrollHeight <= el.clientHeight + 1) {
          unifiedAutoFallback = true;
        }
      } catch {}
    }, 0);
  });
  // Measure actual row height to avoid drift due to CSS differences
  function measureUnifiedRowHeight() {
    try {
      const el = unifiedContainer as HTMLElement | null;
      if (!el) return;
      const row = el.querySelector('tbody tr.border-t') as HTMLElement | null;
      const h = Math.round((row?.getBoundingClientRect()?.height ?? 0));
      if (h && isFinite(h) && h > 8 && h < 200) {
        UNIFIED_ROW_PX = h;
      }
    } catch {}
  }
  onMount(() => {
    setTimeout(measureUnifiedRowHeight, 0);
  });
  $effect(() => { void unifiedDisplayed.length; setTimeout(measureUnifiedRowHeight, 0); });
  let _unifiedScrollTick = false;
  function onUnifiedScroll(_event: Event) {
    if (_unifiedScrollTick) return;
    _unifiedScrollTick = true;
    requestAnimationFrame(() => {
      const el = (unifiedContainer as HTMLElement | null);
      if (!el) { _unifiedScrollTick = false; return; }
      const now = Date.now();
      const top = el.scrollTop;
      _unifiedLastDir = top < _unifiedLastScrollTop ? 'up' : top > _unifiedLastScrollTop ? 'down' : _unifiedLastDir;
      _unifiedLastScrollTop = top;
      _unifiedLastScrollTs = now;
      if (unifiedVirtualize) {
        const first = Math.floor(top / UNIFIED_ROW_PX) - 3; // small lookahead, avoid jumpiness
        unifiedStart = Math.max(0, first);
      }
      // If virtualization is enabled and user is moving down near bottom, increase cap to allow more items to be built.
      if (unifiedVirtualize) {
        try {
          const movingDown = _unifiedLastDir === 'down';
          if (movingDown && el.scrollTop + el.clientHeight >= el.scrollHeight - 400) {
            unifiedCap = Math.min(unifiedCap + UNIFIED_BUILD_STEP, unifiedCap + 20000);
          }
        } catch {}
      }
      _unifiedScrollTick = false;
    });
  }

  // Note: no automatic anchoring of unifiedStart. We only update unifiedStart from user scroll.

  // Tabs view: make the tab selection controlled so clicking the triggers actually switches content
  type CleanerTab = 'quick' | 'temp' | 'large' | 'dup' | 'empty' | 'shortcuts' | 'erase' | 'disk';
  let activeTab = $state<CleanerTab>('quick');

  // Virtualized lists (per-tab) with lazy incremental loading + skeletons
  const LIST_ROW_PX = 32;
  // Temp
  let tempViewport: HTMLElement | null = null;
  let tempStart = $state(0);
  let tempLoaded = $state(80);
  let tempLoadingMore = $state(false);
  const tempRowsInView = $derived(() => Math.ceil(((tempViewport?.clientHeight ?? 256) / LIST_ROW_PX)) + 8);
  const tempLen = $derived(tempFiles.length);
  const tempTopPad = $derived(tempStart * LIST_ROW_PX);
  const tempBottomPad = $derived(Math.max(0, (tempLen - Math.min(tempLen, tempStart + tempRowsInView)) * LIST_ROW_PX));
  const tempWindow = $derived.by(() => {
    const start = Math.min(tempStart, Math.max(0, tempLen - tempRowsInView));
    const end = Math.min(tempLen, start + tempRowsInView);
    const out: Array<{ skel: boolean; file?: FileEntry }>=[];
    for (let i=start;i<end;i++){ if (i < tempLoaded) out.push({ skel:false, file: tempFiles[i] }); else out.push({ skel:true }); }
    return out;
  });
  let _tempTick = false;
  function onTempScroll(e: Event){
    if (_tempTick) return; _tempTick = true;
    requestAnimationFrame(()=>{
    _tempLastScrollTs = Date.now();
      const el = (e.currentTarget as HTMLElement) ?? tempViewport; if (!el) { _tempTick = false; return; }
      const first = Math.max(0, Math.floor(el.scrollTop / LIST_ROW_PX) - 2); tempStart = first;
      if (!tempLoadingMore && tempLoaded < tempLen && el.scrollTop + el.clientHeight >= el.scrollHeight - 160){
        tempLoadingMore = true; setTimeout(()=>{ tempLoaded = Math.min(tempLoaded + 200, tempLen); tempLoadingMore = false; }, 220);
      }
      _tempTick = false;
    });
  }
  // Large
  let largeViewportEl: HTMLElement | null = null;
  let largeStart = $state(0);
  let largeLoaded = $state(80);
  let largeLoadingMore = $state(false);
  const largeRowsInView = $derived(() => Math.ceil(((largeViewportEl?.clientHeight ?? 256) / LIST_ROW_PX)) + 8);
  const largeLen = $derived(largeFiles.length);
  const largeTopPad = $derived(largeStart * LIST_ROW_PX);
  const largeBottomPad = $derived(Math.max(0, (largeLen - Math.min(largeLen, largeStart + largeRowsInView)) * LIST_ROW_PX));
  const largeWindow = $derived.by(() => {
    const start = Math.min(largeStart, Math.max(0, largeLen - largeRowsInView));
    const end = Math.min(largeLen, start + largeRowsInView);
    const out: Array<{ skel: boolean; file?: FileEntry }>=[];
    for (let i=start;i<end;i++){ if (i < largeLoaded) out.push({ skel:false, file: largeFiles[i] }); else out.push({ skel:true }); }
    return out;
  });
  let _largeTick = false;
  function onLargeScroll(e: Event){
    if (_largeTick) return; _largeTick = true;
    requestAnimationFrame(()=>{
      const el = (e.currentTarget as HTMLElement) ?? largeViewportEl; if (!el) { _largeTick = false; return; }
      largeStart = Math.max(0, Math.floor(el.scrollTop / LIST_ROW_PX) - 2);
      if (!largeLoadingMore && largeLoaded < largeLen && el.scrollTop + el.clientHeight >= el.scrollHeight - 160){
        largeLoadingMore = true; setTimeout(()=>{ largeLoaded = Math.min(largeLoaded + 200, largeLen); largeLoadingMore = false; }, 220);
      }
      _largeTick = false;
    });
  }
  // Duplicates
  let dupViewportEl: HTMLElement | null = null;
  let dupStart = $state(0);
  let dupLoaded = $state(80);
  let dupLoadingMore = $state(false);
  const dupRowsInView = $derived(() => Math.ceil(((dupViewportEl?.clientHeight ?? 256) / LIST_ROW_PX)) + 8);
  const dupLen = $derived(duplicateFiles.length);
  const dupTopPad = $derived(dupStart * LIST_ROW_PX);
  const dupBottomPad = $derived(Math.max(0, (dupLen - Math.min(dupLen, dupStart + dupRowsInView)) * LIST_ROW_PX));
  const dupWindow = $derived.by(() => {
    const start = Math.min(dupStart, Math.max(0, dupLen - dupRowsInView));
    const end = Math.min(dupLen, start + dupRowsInView);
    const out: Array<{ skel: boolean; file?: FileEntry }>=[];
    for (let i=start;i<end;i++){ if (i < dupLoaded) out.push({ skel:false, file: duplicateFiles[i] }); else out.push({ skel:true }); }
    return out;
  });
  let _dupTick = false;
  function onDupScroll(e: Event){
    if (_dupTick) return; _dupTick = true;
    requestAnimationFrame(()=>{
      const el = (e.currentTarget as HTMLElement) ?? dupViewportEl; if (!el) { _dupTick = false; return; }
      dupStart = Math.max(0, Math.floor(el.scrollTop / LIST_ROW_PX) - 2);
      if (!dupLoadingMore && dupLoaded < dupLen && el.scrollTop + el.clientHeight >= el.scrollHeight - 160){
        dupLoadingMore = true; setTimeout(()=>{ dupLoaded = Math.min(dupLoaded + 200, dupLen); dupLoadingMore = false; }, 220);
      }
      _dupTick = false;
    });
  }
  // Empty folders
  let emptyViewportEl: HTMLElement | null = null;
  let emptyStart = $state(0);
  let emptyLoaded = $state(80);
  let emptyLoadingMore = $state(false);
  const emptyRowsInView = $derived(() => Math.ceil(((emptyViewportEl?.clientHeight ?? 256) / LIST_ROW_PX)) + 8);
  const emptyLen = $derived(emptyFolders.length);
  const emptyTopPad = $derived(emptyStart * LIST_ROW_PX);
  const emptyBottomPad = $derived(Math.max(0, (emptyLen - Math.min(emptyLen, emptyStart + emptyRowsInView)) * LIST_ROW_PX));
  const emptyWindow = $derived.by(() => {
    const start = Math.min(emptyStart, Math.max(0, emptyLen - emptyRowsInView));
    const end = Math.min(emptyLen, start + emptyRowsInView);
    const out: Array<{ skel: boolean; file?: FileEntry }>=[];
    for (let i=start;i<end;i++){ if (i < emptyLoaded) out.push({ skel:false, file: emptyFolders[i] }); else out.push({ skel:true }); }
    return out;
  });
  let _emptyTick = false;
  function onEmptyScroll(e: Event){
    if (_emptyTick) return; _emptyTick = true;
    requestAnimationFrame(()=>{
      const el = (e.currentTarget as HTMLElement) ?? emptyViewportEl; if (!el) { _emptyTick = false; return; }
      emptyStart = Math.max(0, Math.floor(el.scrollTop / LIST_ROW_PX) - 2);
      if (!emptyLoadingMore && emptyLoaded < emptyLen && el.scrollTop + el.clientHeight >= el.scrollHeight - 160){
        emptyLoadingMore = true; setTimeout(()=>{ emptyLoaded = Math.min(emptyLoaded + 200, emptyLen); emptyLoadingMore = false; }, 220);
      }
      _emptyTick = false;
    });
  }
  // Broken shortcuts
  let shortcutViewportEl: HTMLElement | null = null;
  let shortcutStart = $state(0);
  let shortcutLoaded = $state(80);
  let shortcutLoadingMore = $state(false);
  const shortcutRowsInView = $derived(() => Math.ceil(((shortcutViewportEl?.clientHeight ?? 256) / LIST_ROW_PX)) + 8);
  const shortcutLen = $derived(brokenShortcuts.length);
  const shortcutTopPad = $derived(shortcutStart * LIST_ROW_PX);
  const shortcutBottomPad = $derived(Math.max(0, (shortcutLen - Math.min(shortcutLen, shortcutStart + shortcutRowsInView)) * LIST_ROW_PX));
  const shortcutWindow = $derived.by(() => {
    const start = Math.min(shortcutStart, Math.max(0, shortcutLen - shortcutRowsInView));
    const end = Math.min(shortcutLen, start + shortcutRowsInView);
    const out: Array<{ skel: boolean; file?: FileEntry }>=[];
    for (let i=start;i<end;i++){ if (i < shortcutLoaded) out.push({ skel:false, file: brokenShortcuts[i] }); else out.push({ skel:true }); }
    return out;
  });
  let _shortcutTick = false;
  function onShortcutScroll(e: Event){
    if (_shortcutTick) return; _shortcutTick = true;
    requestAnimationFrame(()=>{
      const el = (e.currentTarget as HTMLElement) ?? shortcutViewportEl; if (!el) { _shortcutTick = false; return; }
      shortcutStart = Math.max(0, Math.floor(el.scrollTop / LIST_ROW_PX) - 2);
      if (!shortcutLoadingMore && shortcutLoaded < shortcutLen && el.scrollTop + el.clientHeight >= el.scrollHeight - 160){
        shortcutLoadingMore = true; setTimeout(()=>{ shortcutLoaded = Math.min(shortcutLoaded + 200, shortcutLen); shortcutLoadingMore = false; }, 220);
      }
      _shortcutTick = false;
    });
  }

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
    if (scanning || isLoading) return;
    // reset results
    tempFiles = []; largeFiles = []; duplicateFiles = []; emptyFolders = []; brokenShortcuts = []; dupGroups = [];
    selectedPaths = new Set(); selectedTempFiles = []; selectedLargeFiles = []; selectedDuplicateFiles = []; selectedEmptyFolders = []; selectedBrokenShortcuts = [];
    tempQueue = []; tempTruncated = false; tempReportedTotal = 0;
    scanning = true; progressMessage = ''; message = '';
    try {
      const minBytes = Math.max(1, largeMinMB) * 1024 * 1024;
      // Fire and forget: backend spawns threads and emits incremental events
      void invoke('start_cleaner_scan', { min_size_bytes: minBytes, max_temp: MAX_TEMP_ITEMS });
    } catch (e) {
      console.error(e); toast.error('Scan failed to start'); scanning = false;
    }
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
      // Avoid heavy synchronous size-stat on very large lists; do it only for small sets
      if (tempFiles.length > 0 && tempFiles.length <= 2000) {
        void statTempSizes();
      }
      scanning = false;
      if (tempTruncated) {
        toast.warning(`Showing first ${tempFiles.length} items (truncated). Refine filters.`);
      } else {
        toast.success(`Found ${tempReportedTotal || tempFiles.length} temporary files.`);
      }
    }).then((fn) => unsubs.push(fn)).catch(() => {});

    // Large files (chunked)
    listen('cleaner-large-batch', (event) => {
      try {
        const arr = (event.payload as [string, number][]) || [];
        if (Array.isArray(arr) && arr.length) {
          for (const it of arr) largeQueue.push([String(it[0]), Number(it[1])]);
          scheduleLargeFlush();
        }
      } catch {}
    }).then((fn) => unsubs.push(fn)).catch(() => {});

    // Duplicate groups (chunked)
    listen('cleaner-dup-groups-batch', (event) => {
      try {
        const groups = (event.payload as Array<{ hash: string; size: number; files: string[] }>) || [];
        if (Array.isArray(groups) && groups.length) {
          for (const g of groups) dupGroupsQueue.push(g);
          scheduleDupFlush();
        }
      } catch {}
    }).then((fn) => unsubs.push(fn)).catch(() => {});

    // Empty folders
    listen('cleaner-empty-batch', (event) => {
      try {
        const arr = (event.payload as string[]) || [];
        if (Array.isArray(arr) && arr.length) {
          for (const p of arr) emptyQueue.push(String(p));
          scheduleEmptyFlush();
        }
      } catch {}
    }).then((fn) => unsubs.push(fn)).catch(() => {});

    // Broken shortcuts
    listen('cleaner-shortcut-batch', (event) => {
      try {
        const arr = (event.payload as string[]) || [];
        if (Array.isArray(arr) && arr.length) {
          for (const p of arr) shortcutQueue.push(String(p));
          scheduleShortcutFlush();
        }
      } catch {}
    }).then((fn) => unsubs.push(fn)).catch(() => {});

    // Done/Stopped per-scope → update loading flags
    listen('cleaner-done', (event) => {
      try {
        const scope = (event.payload as any)?.scope as string | undefined;
        if (!scope || scope === 'all') { scanning = false; isLoading = false; }
        else { isLoading = false; }
      } catch {}
    }).then((fn) => unsubs.push(fn)).catch(() => {});
    listen('cleaner-stopped', () => { scanning = false; isLoading = false; }).then((fn) => unsubs.push(fn)).catch(() => {});
    listen('cleaner-error', (ev) => { try { toast.error(String(ev.payload||'Scan error')); } catch {} }).then((fn) => unsubs.push(fn)).catch(() => {});

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
      tempStart = 0; tempLoaded = 200; tempLoadingMore = false;
      tempFiles = [];
      selectedTempFiles = [];
      tempQueue = []; tempTruncated = false; tempReportedTotal = 0;
      message = 'Scanning for temporary files...';
      await invoke('get_temp_files_stream', { batch_size: 100, max: MAX_TEMP_ITEMS });
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
      await invoke('cancel_cleaner_scan');
    } catch {}
    scanning = false;
    tempQueue = [];
    toast.warning('Scan cancelled');
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
    message = 'Scanning for large files...';
    progressMessage = '';
    largeFiles = [];
    try {
      void invoke('start_large_scan');
    } catch (error) {
      message = `Error starting large files scan: ${error}`;
      console.error(error);
      toast.error(message);
      isLoading = false;
    }
  }

  async function deleteSelectedLargeFiles() {
    if (selectedLargeFiles.length === 0) return;
    confirmDeletion(selectedLargeFiles);
  }

  async function findDuplicateFiles() {
    isLoading = true;
    message = 'Scanning for duplicate files...';
    progressMessage = '';
    duplicateFiles = []; dupGroups = [];
    dupStart = 0; dupLoaded = 80; dupLoadingMore = false;
    try {
      void invoke('start_duplicate_groups_scan');
    } catch (error) {
      message = `Error starting duplicate scan: ${error}`;
      console.error(error);
      toast.error(message);
      isLoading = false;
    }
  }

  async function deleteSelectedDuplicateFiles() {
    if (selectedDuplicateFiles.length === 0) return;
    confirmDeletion(selectedDuplicateFiles);
  }

  async function findEmptyFolders() {
    isLoading = true;
    message = 'Scanning for empty folders...';
    progressMessage = '';
    emptyFolders = [];
    emptyStart = 0; emptyLoaded = 80; emptyLoadingMore = false;
    try {
      void invoke('start_empty_scan');
    } catch (error) {
      message = `Error starting empty folders scan: ${error}`;
      console.error(error);
      toast.error(message);
      isLoading = false;
    }
  }

  async function deleteSelectedEmptyFolders() {
    if (selectedEmptyFolders.length === 0) return;
    confirmDeletion(selectedEmptyFolders);
  }

  async function findBrokenShortcuts() {
    isLoading = true;
    message = 'Scanning for broken shortcuts...';
    progressMessage = '';
    brokenShortcuts = [];
    shortcutStart = 0; shortcutLoaded = 80; shortcutLoadingMore = false;
    try {
      void invoke('start_shortcut_scan');
    } catch (error) {
      message = `Error starting broken shortcuts scan: ${error}`;
      console.error(error);
      toast.error(message);
      isLoading = false;
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
            <!-- Large size threshold input removed for simplicity -->
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
          <div class="rounded-md border h-[60vh] overflow-auto" style="overflow-anchor: none;" bind:this={unifiedContainer} onscroll={onUnifiedScroll}>
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
                  {#if scanning || isLoading}
                    {#each Array.from({ length: 16 }) as _, i}
                      <tr class="border-t">
                        <td class="px-3 py-2" colspan="5"><Skeleton class="h-4 w-full" aria-hidden="true" /></td>
                      </tr>
                    {/each}
                  {:else}
                    <tr>
                      <td colspan="5" class="px-3 py-6 text-center text-muted-foreground">No items. Click Scan All.</td>
                    </tr>
                  {/if}
                {:else}
                  {#if unifiedVirtualize && unifiedTopPad > 0}
                    <tr><td colspan="5" style={`height:${unifiedTopPad}px; overflow-anchor: none;`}></td></tr>
                  {/if}
                  {#each (unifiedDisplayed.length > 0 ? unifiedDisplayed : allItems.slice(0, Math.min(allItems.length, UNIFIED_MAX_DOM))) as it (it.path)}
                    <tr class="border-t h-10 align-middle">
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
                  {#if unifiedVirtualize && unifiedBottomPad > 0}
                    <tr><td colspan="5" style={`height:${unifiedBottomPad}px; overflow-anchor: none;`}></td></tr>
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
          <div class="h-64 rounded border overflow-auto" style="overscroll-behavior: contain; overflow-anchor: none;" bind:this={tempViewport} onscroll={onTempScroll}>
            <ul class="text-sm">
              {#if tempTopPad > 0}
                <li style={`height:${tempTopPad}px`} aria-hidden="true"></li>
              {/if}
              {#each tempWindow as row, i (row.file ? row.file.path : `skel-${tempStart + i}`)}
                {#if row.skel}
                  <li class="px-2 py-1"><Skeleton class="h-4 w-5/6" aria-hidden="true" /></li>
                {:else}
                  <li class="flex items-center gap-2 px-2 py-1">
                    <Checkbox
                      checked={selectedTempFiles.includes(row.file!.path)}
                      onCheckedChange={() => handleFileSelection(row.file!.path, 'temp')}
                    />
                    <span class="truncate">{row.file!.path}</span>
                  </li>
                {/if}
              {/each}
              {#if tempBottomPad > 0}
                <li style={`height:${tempBottomPad}px`} aria-hidden="true"></li>
              {/if}
            </ul>
          </div>

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
          <div class="h-64 rounded border overflow-auto" style="overscroll-behavior: contain; overflow-anchor: none;" bind:this={largeViewportEl} onscroll={onLargeScroll}>
            <ul class="text-sm">
              {#if largeTopPad > 0}
                <li style={`height:${largeTopPad}px`} aria-hidden="true"></li>
              {/if}
              {#each largeWindow as row, i (row.file ? row.file.path : `skel-large-${largeStart + i}`)}
                {#if row.skel}
                  <li class="px-2 py-1"><Skeleton class="h-4 w-3/4" aria-hidden="true" /></li>
                {:else}
                  <li class="flex items-center justify-between gap-2 px-2 py-1">
                    <div class="flex items-center gap-2">
                      <Checkbox
                        checked={selectedLargeFiles.includes(row.file!.path)}
                        onCheckedChange={() => handleFileSelection(row.file!.path, 'large')}
                      />
                      <span class="truncate">{row.file!.path}</span>
                    </div>
                    {#if row.file?.size}
                      <span class="text-xs opacity-70 whitespace-nowrap">{formatBytes(row.file!.size!)}</span>
                    {/if}
                  </li>
                {/if}
              {/each}
              {#if largeBottomPad > 0}
                <li style={`height:${largeBottomPad}px`} aria-hidden="true"></li>
              {/if}
            </ul>
          </div>
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
          <div class="h-64 rounded border overflow-auto" style="overscroll-behavior: contain; overflow-anchor: none;" bind:this={dupViewportEl} onscroll={onDupScroll}>
            <ul class="text-sm">
              {#if dupTopPad > 0}
                <li style={`height:${dupTopPad}px`} aria-hidden="true"></li>
              {/if}
              {#each dupWindow as row, i (row.file ? row.file.path : `skel-dup-${dupStart + i}`)}
                {#if row.skel}
                  <li class="px-2 py-1"><Skeleton class="h-4 w-5/6" aria-hidden="true" /></li>
                {:else}
                  <li class="flex items-center gap-2 px-2 py-1">
                    <Checkbox
                      checked={selectedDuplicateFiles.includes(row.file!.path)}
                      onCheckedChange={() => handleFileSelection(row.file!.path, 'duplicate')}
                    />
                    <span class="truncate">{row.file!.path}</span>
                  </li>
                {/if}
              {/each}
              {#if dupBottomPad > 0}
                <li style={`height:${dupBottomPad}px`} aria-hidden="true"></li>
              {/if}
            </ul>
          </div>
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
          <div class="h-64 rounded border overflow-auto" style="overscroll-behavior: contain; overflow-anchor: none;" bind:this={emptyViewportEl} onscroll={onEmptyScroll}>
            <ul class="text-sm">
              {#if emptyTopPad > 0}
                <li style={`height:${emptyTopPad}px`} aria-hidden="true"></li>
              {/if}
              {#each emptyWindow as row, i (row.file ? row.file.path : `skel-empty-${emptyStart + i}`)}
                {#if row.skel}
                  <li class="px-2 py-1"><Skeleton class="h-4 w-5/6" aria-hidden="true" /></li>
                {:else}
                  <li class="flex items-center gap-2 px-2 py-1">
                    <Checkbox
                      checked={selectedEmptyFolders.includes(row.file!.path)}
                      onCheckedChange={() => handleFileSelection(row.file!.path, 'empty')}
                    />
                    <span class="truncate">{row.file!.path}</span>
                  </li>
                {/if}
              {/each}
              {#if emptyBottomPad > 0}
                <li style={`height:${emptyBottomPad}px`} aria-hidden="true"></li>
              {/if}
            </ul>
          </div>
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
          <div class="h-64 rounded border overflow-auto" style="overscroll-behavior: contain; overflow-anchor: none;" bind:this={shortcutViewportEl} onscroll={onShortcutScroll}>
            <ul class="text-sm">
              {#if shortcutTopPad > 0}
                <li style={`height:${shortcutTopPad}px`} aria-hidden="true"></li>
              {/if}
              {#each shortcutWindow as row, i (row.file ? row.file.path : `skel-short-${shortcutStart + i}`)}
                {#if row.skel}
                  <li class="px-2 py-1"><Skeleton class="h-4 w-5/6" aria-hidden="true" /></li>
                {:else}
                  <li class="flex items-center gap-2 px-2 py-1">
                    <Checkbox
                      checked={selectedBrokenShortcuts.includes(row.file!.path)}
                      onCheckedChange={() => handleFileSelection(row.file!.path, 'broken_shortcut')}
                    />
                    <span class="truncate">{row.file!.path}</span>
                  </li>
                {/if}
              {/each}
              {#if shortcutBottomPad > 0}
                <li style={`height:${shortcutBottomPad}px`} aria-hidden="true"></li>
              {/if}
            </ul>
          </div>
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












