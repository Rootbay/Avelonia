<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { Button } from '$lib/components/ui/button';
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from '$lib/components/ui/card';
  import { Input } from '$lib/components/ui/input';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Badge } from '$lib/components/ui/badge';
  import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { Alert, AlertDescription } from '$lib/components/ui/alert';
  import { toast } from '$lib/components/ui/sonner';
  import { pushLog } from '$lib/logStore';
  import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogClose,
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
  import {
    RefreshCw,
    FolderOpen,
    Trash2,
    Eye,
    Copy as CopyIcon,
    Search as SearchIcon,
    Network as NetworkIcon,
    RefreshCcw,
    RotateCcw,
    Settings,
    ListChecks,
    Flag,
  } from '@lucide/svelte';

  let message = $state('');
  type ButtonSnippetContext = { props?: Record<string, unknown> & { class?: string } };
  type StartupItem = { path: string; name: string };
  let startupItems = $state<StartupItem[]>([]);
  let selectedStartup = $state(new Set<string>());
  let startupQuery = $state('');
  let startupLoaded = $state(false);
  let loadingStartup = $state(false);
  let startupVisible = $state(50);
  let startupSentinel: HTMLElement | null = null;

  const STARTUP_MAX_DOM = 300;
  const REGISTRY_MAX_DOM = 300;
  const TASKS_MAX_DOM = 600;
  const STARTUP_ROW_PX = 56;
  const REGISTRY_ROW_PX = 56;
  const TASKS_ROW_PX = 64;

  let startupStart = $state(0);
  let registryStart = $state(0);
  let tasksStart = $state(0);
  let startupScrollEl = $state<HTMLElement | null>(null);
  let registryScrollEl = $state<HTMLElement | null>(null);
  let tasksScrollEl = $state<HTMLElement | null>(null);
  let startupQueryDeb = $state('');

  $effect(() => {
    const t = setTimeout(() => (startupQueryDeb = startupQuery), 180);
    return () => clearTimeout(t);
  });

  async function loadStartupItems() {
    if (loadingStartup || startupLoaded) return;
    loadingStartup = true;
    try {
      const items = (await invoke('list_startup_shortcuts')) as StartupItem[];
      startupItems = Array.isArray(items) ? items : [];
      try {
        console.debug(
          '[VT] startup items loaded:',
          startupItems.length,
          startupItems.map((i) => i.name).slice(0, 10)
        );
      } catch { /* noop */ }
      selectedStartup = new Set();
      startupLoaded = true;
      startupVisible = Math.min(startupItems.length, 50);
    } catch (e) {
      console.error(e);
    } finally {
      loadingStartup = false;
    }
  }

  async function reloadStartupItems() {
    startupLoaded = false;
    await loadStartupItems();
  }

  let _startupPollTimer: number | null = null;
  let _startupPollBusy = false;
  async function pollStartupOnce() {
    if (!startupLoaded || _startupPollBusy) return;
    _startupPollBusy = true;
    try {
      const items = (await invoke('list_startup_shortcuts')) as StartupItem[];
      const next = Array.isArray(items) ? items : [];
      const curSet = new Set(startupItems.map((i) => i.path));
      const nextSet = new Set(next.map((i) => i.path));
      let changed = curSet.size !== nextSet.size;
      if (!changed) {
        for (const p of nextSet) {
          if (!curSet.has(p)) {
            changed = true;
            break;
          }
        }
      }
      if (changed) {
        const keep = new Set(selectedStartup);
        startupItems = next;
        selectedStartup = new Set(Array.from(keep).filter((p) => nextSet.has(p)));
        startupVisible = Math.min(Math.max(50, startupVisible), startupItems.length);
      }
    } catch { /* noop */ } finally {
      _startupPollBusy = false;
    }
  }

  function normalizeWinPath(p: string) {
    try {
      return p.replace(/\//g, '\\');
    } catch {
      return p;
    }
  }
  async function copyText(txt: string) {
    try {
      await navigator.clipboard.writeText(txt);
    } catch (e) {
      console.error(e);
    }
  }
  function toggleStartup(p: string) {
    if (selectedStartup.has(p)) selectedStartup.delete(p);
    else selectedStartup.add(p);
    selectedStartup = new Set(selectedStartup);
  }
  let _startupScrollTick = false;
  function onStartupScroll(event: Event) {
    if (_startupScrollTick) return;
    _startupScrollTick = true;
    const target = event.currentTarget as HTMLElement | null;
    requestAnimationFrame(() => {
      const el = (target as HTMLElement | null) || startupScrollEl;
      if (!el) {
        _startupScrollTick = false;
        return;
      }
      if (el.scrollTop + el.clientHeight >= el.scrollHeight - 200) {
        startupVisible = Math.min(startupVisible + 100, filteredStartupItems.length);
        if (startupVisible - startupStart > STARTUP_MAX_DOM) {
          startupStart = Math.max(0, startupVisible - STARTUP_MAX_DOM);
        }
      }
      _startupScrollTick = false;
    });
  }

  let showStartupConfirm = $state(false);
  let pendingStartup: string[] = $state([]);
  function requestRemoveSelectedStartup() {
    if (selectedStartup.size === 0) return;
    pendingStartup = Array.from(selectedStartup);
    showStartupConfirm = true;
  }

  async function confirmRemoveStartup() {
    showStartupConfirm = false;
    try {
      const removed: number = await invoke('remove_startup_shortcuts', { files: pendingStartup });
      message = `Removed ${removed} startup shortcut${removed === 1 ? '' : 's'}.`;
      if (removed > 0) {
        toast.success(message);
        pushLog('SUCCESS', message, 'Optimize');
      } else {
        toast.info('No startup shortcuts removed');
        pushLog('INFO', 'No startup shortcuts removed', 'Optimize');
      }
      selectedStartup = new Set();
      await reloadStartupItems();
    } catch (e) {
      console.error(e);
      message = `Failed to remove startup items: ${e}`;
      toast.error('Failed to remove startup items');
      pushLog('ERROR', `Failed to remove startup items: ${String(e)}`, 'Optimize');
    } finally {
      pendingStartup = [];
    }
  }

  async function openStartupFolders() {
    try {
      const folders: string[] = await invoke('get_startup_folders');
      for (const f of folders) {
        try {
          await openPath(f);
        } catch { /* noop */ }
      }
    } catch (e) {
      console.error(e);
    }
  }

  type StartupRegItem = { hive: string; key: string; name: string; command: string };
  let startupRegItems = $state<StartupRegItem[]>([]);
  let selectedReg = $state(new Set<string>());
  let registryQuery = $state('');
  let registryLoaded = $state(false);
  let loadingRegistry = $state(false);
  let registryVisible = $state(50);
  let registrySentinel: HTMLElement | null = null;
  const regId = (it: StartupRegItem) => `${it.hive}|${it.key}|${it.name}`;
  let registryQueryDeb = $state('');
  let showRegistryConfirm = $state(false);
  let pendingRegistry: StartupRegItem[] = $state([]);
  let registryForce = $state(false);
  let registryBlockIFEO = $state(false);
  let registryDeleteOnReboot = $state(false);
  let registryPurgeStartupApproved = $state(false);
  let registryDeleteTasksByMatch = $state(false);
  let registryRemoveWMIByMatch = $state(false);
  let registryUserRebooted = $state(false);
  let showPostCleanup = $state(false);
  let postDiagLoading = $state(false);
  let postDiag: CleanupDiagnostics | null = $state(null);
  let regPreset = $state<'basic' | 'force' | 'aggressive' | 'full'>('basic');

  type CleanupDiagnostics = {
    removedRegistry: { ok: string[]; stillPresent: string[] };
    runningImages: { running: string[]; stopped: string[] };
    taskMatches: { remaining: string[] };
    serviceMatches: { running: string[]; disabled: string[] };
    rebootRecommended: boolean;
  };

  function applyRegPreset(p: 'basic' | 'force' | 'aggressive' | 'full') {
    if (p === 'basic') {
      registryForce = false;
      registryBlockIFEO = false;
      registryDeleteOnReboot = false;
      registryPurgeStartupApproved = false;
      registryDeleteTasksByMatch = false;
      registryRemoveWMIByMatch = false;
    } else if (p === 'force') {
      registryForce = true;
      registryBlockIFEO = false;
      registryDeleteOnReboot = false;
      registryPurgeStartupApproved = false;
      registryDeleteTasksByMatch = false;
      registryRemoveWMIByMatch = false;
    } else if (p === 'aggressive') {
      registryForce = true;
      registryBlockIFEO = regHasExeImage;
      registryDeleteOnReboot = regHasExePath;
      registryPurgeStartupApproved = false;
      registryDeleteTasksByMatch = false;
      registryRemoveWMIByMatch = false;
    } else {
      registryForce = true;
      registryBlockIFEO = regHasExeImage;
      registryDeleteOnReboot = regHasExePath;
      registryPurgeStartupApproved = true;
      registryDeleteTasksByMatch = true;
      registryRemoveWMIByMatch = true;
    }
  }

  $effect(() => {
    const _p = regPreset;
    applyRegPreset(_p);
  });

  $effect(() => {
    const t = setTimeout(() => (registryQueryDeb = registryQuery), 200);
    return () => clearTimeout(t);
  });

  type RegistryAttempt = {
    attempts: number;
    rebootConfirmed: boolean;
    lastOptions: {
      force: boolean;
      ifeo: boolean;
      dor: boolean;
      purge: boolean;
      tasks: boolean;
      wmi: boolean;
    };
    lastStrategy?: 'basic' | 'force' | 'aggressive' | 'full';
    fullCleanupUsed?: boolean;
    pendingVerification?: boolean;
    suspicious?: boolean;
    suspiciousReason?: string;
    lastImages?: string[];
    lastPaths?: string[];
    lastSeenAt?: number;
  };
  let registryHistory = $state<Record<string, RegistryAttempt>>({});
  const REG_HISTORY_KEY = 'avelonia_registry_history_v1';
  $effect(() => {
    try {
      const raw = localStorage.getItem(REG_HISTORY_KEY);
      if (raw) registryHistory = JSON.parse(raw) ?? {};
    } catch { /* noop */ }
  });
  function saveRegHistory() {
    try {
      localStorage.setItem(REG_HISTORY_KEY, JSON.stringify(registryHistory));
    } catch { /* noop */ }
  }

  let rebootDetected = $state(false);
  async function initBootCheck() {
    try {
      const nowBoot = (await invoke('get_boot_time')) as number;
      const key = 'avelonia_boot_time_v1';
      const prev = Number(localStorage.getItem(key) || '0');
      localStorage.setItem(key, String(nowBoot));
      rebootDetected = prev > 0 && prev !== nowBoot;
    } catch {
      rebootDetected = false;
    }
  }

  const regHasExeImage = $derived(pendingRegistry.some((r) => !!extractExeFromCommand(r.command)));
  const regHasExePath = $derived(
    pendingRegistry.some((r) => !!extractExePathFromCommand(r.command))
  );
  const regSuggestAdvanced = $derived.by(() => {
    if (registryUserRebooted) return true;
    const ids = pendingRegistry.map(regId);
    let anyRebooted = false;
    let triedBasic = false;
    for (const id of ids) {
      const h = registryHistory[id];
      if (!h) continue;
      if (h.rebootConfirmed) anyRebooted = true;
      if (h.attempts >= 1 && (h.lastOptions.force || h.lastOptions.ifeo || h.lastOptions.dor))
        triedBasic = true;
    }
    return anyRebooted && triedBasic;
  });

  function requestRemoveSelectedRegistry() {
    const entries = startupRegItems.filter((it) => selectedReg.has(regId(it)));
    if (entries.length === 0) return;
    pendingRegistry = entries;
    showRegistryConfirm = true;
    registryForce = false;
    registryBlockIFEO = false;
    registryDeleteOnReboot = false;
    registryPurgeStartupApproved = false;
    registryDeleteTasksByMatch = false;
    registryRemoveWMIByMatch = false;
    registryUserRebooted = false;

    const ids = pendingRegistry.map(regId);
    const anyRebooted = ids.some((id) => registryHistory[id]?.rebootConfirmed);
    const triedBasic = ids.some((id) => {
      const h = registryHistory[id];
      return (
        h && h.attempts >= 1 && (h.lastOptions.force || h.lastOptions.ifeo || h.lastOptions.dor)
      );
    });
    if (anyRebooted && triedBasic) {
      registryPurgeStartupApproved = true;
      registryDeleteTasksByMatch = true;
      registryRemoveWMIByMatch = true;
    }
  }

  async function confirmRemoveRegistry() {
    showRegistryConfirm = false;
    try {
      const strategyUsed = regPreset;
      const removed: number = await invoke(
        registryForce ? 'force_remove_registry_run' : 'remove_registry_run',
        { entries: pendingRegistry }
      );
      message = `Removed ${removed} registry startup entr${removed === 1 ? 'y' : 'ies'}.`;
      if (removed > 0) {
        toast.success(message);
        pushLog(
          'SUCCESS',
          message + (strategyUsed ? ` (preset: ${strategyUsed})` : ''),
          'Optimize'
        );
      } else {
        toast.info('No entries removed');
        pushLog('INFO', 'No registry entries removed', 'Optimize');
      }
      selectedReg = new Set();
      await reloadRegistryItems();
      monitorRegistryWatchdog([...pendingRegistry]);

      const images = Array.from(
        new Set(
          pendingRegistry
            .map((r) => extractExeFromCommand(r.command))
            .filter((x): x is string => !!x)
        )
      );
      const paths = Array.from(
        new Set(
          pendingRegistry
            .map((r) => extractExePathFromCommand(r.command))
            .filter((x): x is string => !!x)
        )
      );
      try {
        const services = (await invoke('list_services')) as Array<{
          name: string;
          display_name: string;
          state: string;
          start_mode: string;
          path: string;
        }>;
        const lower = (s: string) => s?.toLowerCase?.() ?? '';
        const suspects = new Set(
          services
            .filter((svc) => {
              const p = lower(svc.path);
              return (
                images.some((img) => p.includes(lower(img))) ||
                paths.some((full) => p.includes(lower(full)))
              );
            })
            .map((s) => s.name)
        );
        if (suspects.size > 0) {
          const names = Array.from(suspects);
          try {
            await invoke('stop_services', { names });
          } catch { /* noop */ }
          try {
            const n = (await invoke('disable_services', { names })) as number;
            if (n > 0) toast.message(`Disabled ${n} related service${n === 1 ? '' : 's'}.`);
          } catch (e) {
            console.warn('disable_services failed', e);
          }
        }
      } catch (e) {
        console.warn('list_services failed', e);
      }
      if (registryBlockIFEO && images.length) {
        try {
          const n = (await invoke('block_process_ifeo', { images, enable: true })) as number;
          if (n > 0) toast.success(`Blocked ${n} process${n === 1 ? '' : 'es'} via IFEO.`);
        } catch (e) {
          console.warn('block_process_ifeo failed', e);
        }
      }
      let dorScheduled = false;
      if (registryDeleteOnReboot && paths.length) {
        try {
          const n = (await invoke('schedule_delete_on_reboot', { paths })) as number;
          if (n > 0) {
            toast.message(`Scheduled delete on reboot for ${n} file${n === 1 ? '' : 's'}.`);
            dorScheduled = true;
          }
        } catch (e) {
          console.warn('schedule_delete_on_reboot failed', e);
        }
      }
      if (registryPurgeStartupApproved) {
        try {
          const names = pendingRegistry.map((r) => r.name);
          const n = (await invoke('purge_startup_approved', { names })) as number;
          if (n > 0) toast.message(`Purged StartupApproved for ${n} entr${n === 1 ? 'y' : 'ies'}.`);
        } catch (e) {
          console.warn('purge_startup_approved failed', e);
        }
      }
      if (registryDeleteTasksByMatch && (images.length || paths.length)) {
        try {
          const n = (await invoke('delete_tasks_by_match', { images, paths })) as number;
          if (n > 0) toast.message(`Deleted ${n} related scheduled task${n === 1 ? '' : 's'}.`);
        } catch (e) {
          console.warn('delete_tasks_by_match failed', e);
        }
      }
      if (registryRemoveWMIByMatch && (images.length || paths.length)) {
        try {
          const n = (await invoke('remove_wmi_subscriptions_by_match', {
            images,
            paths,
          })) as number;
          if (n > 0) toast.message('Removed WMI event subscriptions matching target.');
        } catch (e) {
          console.warn('remove_wmi_subscriptions_by_match failed', e);
        }
      }

      try {
        postDiagLoading = true;
        const shouldSuggestReboot =
          dorScheduled || strategyUsed === 'full' || strategyUsed === 'aggressive';
        const diag = await runPostCleanupDiagnostics(pendingRegistry, {
          images,
          paths,
          rebootRecommended: shouldSuggestReboot,
        });
        postDiag = diag;
        showPostCleanup = true;
      } catch (e) {
        console.warn('post cleanup diagnostics failed', e);
      } finally {
        postDiagLoading = false;
      }
    } catch (e) {
      console.error(e);
      message = `Failed to remove registry entries: ${e}`;
      toast.error(message);
    } finally {
      try {
        const strategy = regPreset;
        const images = Array.from(
          new Set(
            pendingRegistry
              .map((r) => extractExeFromCommand(r.command))
              .filter((x): x is string => !!x)
          )
        );
        const paths = Array.from(
          new Set(
            pendingRegistry
              .map((r) => extractExePathFromCommand(r.command))
              .filter((x): x is string => !!x)
          )
        );
        for (const it of pendingRegistry) {
          const id = regId(it);
          const prev = (registryHistory[id] ?? {
            attempts: 0,
            rebootConfirmed: false,
            lastOptions: {
              force: false,
              ifeo: false,
              dor: false,
              purge: false,
              tasks: false,
              wmi: false,
            },
          }) as RegistryAttempt;
          registryHistory[id] = {
            attempts: (prev.attempts ?? 0) + 1,
            rebootConfirmed: prev.rebootConfirmed || registryUserRebooted,
            lastOptions: {
              force: registryForce,
              ifeo: registryBlockIFEO,
              dor: registryDeleteOnReboot,
              purge: registryPurgeStartupApproved,
              tasks: registryDeleteTasksByMatch,
              wmi: registryRemoveWMIByMatch,
            },
            lastStrategy: strategy,
            fullCleanupUsed: strategy === 'full',
            pendingVerification: strategy === 'full',
            suspicious: prev.suspicious || false,
            suspiciousReason: prev.suspiciousReason || '',
            lastImages: images,
            lastPaths: paths,
            lastSeenAt: Date.now(),
          } as RegistryAttempt;
        }
        saveRegHistory();
      } catch { /* noop */ }
      pendingRegistry = [];
      registryForce = false;
      registryBlockIFEO = false;
      registryDeleteOnReboot = false;
      registryPurgeStartupApproved = false;
      registryDeleteTasksByMatch = false;
      registryRemoveWMIByMatch = false;
      registryUserRebooted = false;
    }
  }

  async function runPostCleanupDiagnostics(
    targets: StartupRegItem[],
    opts: { images: string[]; paths: string[]; rebootRecommended?: boolean }
  ): Promise<CleanupDiagnostics> {
    const diag: CleanupDiagnostics = {
      removedRegistry: { ok: [], stillPresent: [] },
      runningImages: { running: [], stopped: [] },
      taskMatches: { remaining: [] },
      serviceMatches: { running: [], disabled: [] },
      rebootRecommended: !!opts.rebootRecommended,
    };

    try {
      const list = (await invoke('list_registry_run')) as StartupRegItem[];
      const present = new Set(list.map(regId));
      for (const t of targets) {
        const id = regId(t);
        const label = `${t.hive} \\ ${t.key} -> ${t.name}`;
        if (present.has(id)) diag.removedRegistry.stillPresent.push(label);
        else diag.removedRegistry.ok.push(label);
      }
    } catch (e) {
      console.warn('diagnostics: list_registry_run failed', e);
    }

    for (const img of opts.images) {
      try {
        const running = (await invoke('is_process_running', { image: img })) as boolean;
        if (running) diag.runningImages.running.push(img);
        else diag.runningImages.stopped.push(img);
      } catch { /* noop */ }
    }

    try {
      const tasks = (await invoke('list_scheduled_tasks')) as Array<{
        name: string;
        task_to_run?: string;
      }>;
      const matches: string[] = [];
      for (const t of tasks) {
        let cmd = (t as any)?.task_to_run || '';
        if (!cmd) {
          try {
            const details = (await invoke('get_task_details', { task_name: t.name })) as
              | [string, string]
              | any;
            cmd = Array.isArray(details) ? (details[0] ?? '') : (details?.task_to_run ?? '');
          } catch { /* noop */ }
        }
        const lower = (cmd || '').toLowerCase();
        if (!lower) continue;
        const hit =
          opts.images.some((i) => lower.includes(i.toLowerCase())) ||
          opts.paths.some((p) => lower.includes(p.toLowerCase()));
        if (hit) matches.push(t.name);
      }
      diag.taskMatches.remaining = Array.from(new Set(matches));
    } catch (e) {
      console.warn('diagnostics: list_scheduled_tasks failed', e);
    }

    try {
      const services = (await invoke('list_services')) as Array<{
        name: string;
        state: string;
        start_mode: string;
        path: string;
      }>;
      for (const s of services) {
        const p = (s.path || '').toLowerCase();
        if (!p) continue;
        const hit =
          opts.images.some((i) => p.includes(i.toLowerCase())) ||
          opts.paths.some((q) => p.includes(q.toLowerCase()));
        if (!hit) continue;
        const state = (s.state || '').toLowerCase();
        const mode = (s.start_mode || '').toLowerCase();
        if (state.includes('running') || mode === 'auto' || mode === 'automatic')
          diag.serviceMatches.running.push(s.name);
        else diag.serviceMatches.disabled.push(s.name);
      }
      diag.serviceMatches.running = Array.from(new Set(diag.serviceMatches.running));
      diag.serviceMatches.disabled = Array.from(new Set(diag.serviceMatches.disabled));
    } catch (e) {
      console.warn('diagnostics: list_services failed', e);
    }

    return diag;
  }

  function extractExeFromCommand(cmd: string): string | null {
    if (!cmd) return null;
    const quoted = cmd.match(/"([^"\\]+?\.exe)"/i);
    if (quoted?.[1]) return quoted[1].split('\\').pop() || quoted[1];
    const bare = cmd.match(/\b([\w .-]+\.exe)\b/i);
    if (bare?.[1]) return bare[1].split('\\').pop() || bare[1];
    return null;
  }

  function extractExePathFromCommand(cmd: string): string | null {
    if (!cmd) return null;
    const q = cmd.match(/"([^"\\]+?\.exe)"/i);
    if (q?.[1]) return q[1];
    const b = cmd.match(/\b([a-zA-Z]:\\[^\s"]+?\.exe)\b/);
    if (b?.[1]) return b[1];
    return null;
  }

  async function monitorRegistryWatchdog(targets: StartupRegItem[]) {
    try {
      const ids = new Set(targets.map(regId));
      const images = new Set(
        targets
          .map((t) => extractExeFromCommand(t.command))
          .filter((x): x is string => typeof x === 'string' && x.length > 0)
      );

      let readded = false;
      let restarted: string[] = [];
      for (let i = 0; i < 6; i++) {
        try {
          const list = (await invoke('list_registry_run')) as StartupRegItem[];
          const set = new Set(list.map(regId));
          const back = [...ids].some((k) => set.has(k));
          if (back) {
            readded = true;
            toast.warning('Watchdog detected: entry reappeared in Startup (Registry).');
            break;
          }
        } catch { /* noop */ }

        try {
          for (const img of images) {
            const running = (await invoke('is_process_running', { image: img })) as boolean;
            if (running && !restarted.includes(img)) restarted.push(img);
          }
        } catch { /* noop */ }

        await new Promise((r) => setTimeout(r, 2000));
      }
      if (restarted.length > 0) {
        toast.warning(`Process running after removal: ${restarted.join(', ')}`);
      }
      if (!readded) {
        toast.success('Removal appears persistent (no reappearance detected).');
      }
    } catch (e) {
      console.warn('monitorRegistryWatchdog failed', e);
    }
  }

  async function loadRegistryItems() {
    if (loadingRegistry || registryLoaded) return;
    loadingRegistry = true;
    try {
      const res = (await invoke('list_registry_run')) as StartupRegItem[];
      startupRegItems = Array.isArray(res) ? res : [];
      try {
        console.debug(
          '[VT] registry items loaded:',
          startupRegItems.length,
          startupRegItems.map((i) => i.name).slice(0, 10)
        );
      } catch { /* noop */ }
      selectedReg = new Set();
      registryLoaded = true;
      registryVisible = Math.min(startupRegItems.length, 50);
    } catch (e) {
      console.error(e);
    } finally {
      loadingRegistry = false;
    }
  }
  async function reloadRegistryItems() {
    registryLoaded = false;
    await loadRegistryItems();
  }

  let _registryPollTimer: number | null = null;
  let _registryPollBusy = false;
  async function pollRegistryOnce() {
    if (!registryLoaded || _registryPollBusy) return;
    _registryPollBusy = true;
    try {
      const res = (await invoke('list_registry_run')) as StartupRegItem[];
      const next = Array.isArray(res) ? res : [];
      const id = (it: StartupRegItem) => `${it.hive}|${it.key}|${it.name}`;
      const curSet = new Set(startupRegItems.map(id));
      const nextSet = new Set(next.map(id));
      let changed = curSet.size !== nextSet.size;
      if (!changed) {
        for (const k of nextSet) {
          if (!curSet.has(k)) {
            changed = true;
            break;
          }
        }
      }
      if (changed) {
        startupRegItems = next;
        const keep = new Set(selectedReg);
        selectedReg = new Set(Array.from(keep).filter((k) => nextSet.has(k)));
        registryVisible = Math.min(Math.max(50, registryVisible), startupRegItems.length);
      }
    } catch { /* noop */ } finally {
      _registryPollBusy = false;
    }
  }

  async function scanSuspiciousAfterReboot() {
    try {
      if (!registryLoaded || !rebootDetected) return;
      const present = new Set(startupRegItems.map(regId));
      let updates = 0;
      for (const [id, rec] of Object.entries(registryHistory)) {
        if (!rec || typeof rec !== 'object') continue;
        const wasFull = !!(rec as RegistryAttempt).fullCleanupUsed;
        const pending = !!(rec as RegistryAttempt).pendingVerification;
        if (wasFull && pending && present.has(id)) {
          const r = (registryHistory[id] as RegistryAttempt) || ({} as RegistryAttempt);
          r.pendingVerification = false;
          r.rebootConfirmed = true;
          r.suspicious = true;
          const hint: string[] = [];
          if (r.lastOptions?.ifeo) hint.push('IFEO block was active during the last action');
          if (r.lastOptions?.dor) hint.push('Delete on reboot was active');
          if (Array.isArray(r.lastImages) && r.lastImages.length) {
            hint.push(`Process image(s): ${r.lastImages.join(', ')}`);
          }
          r.suspiciousReason =
            `Entry reappeared after Full cleanup and reboot. ${hint.join(' Ã‚Â· ')}`.trim();
          registryHistory[id] = r;
          updates += 1;
        }
      }
      if (updates > 0) {
        saveRegHistory();
        toast.warning(`Detected ${updates} recurring entries after reboot. Marked as suspicious.`);
      }
    } catch (e) {
      console.warn('scanSuspiciousAfterReboot failed', e);
    }
  }
  function toggleReg(it: StartupRegItem) {
    const id = regId(it);
    if (selectedReg.has(id)) selectedReg.delete(id);
    else selectedReg.add(id);
    selectedReg = new Set(selectedReg);
  }
  let _registryScrollTick = false;
  function onRegistryScroll(event: Event) {
    if (_registryScrollTick) return;
    _registryScrollTick = true;
    const target = event.currentTarget as HTMLElement | null;
    requestAnimationFrame(() => {
      const el = (target as HTMLElement | null) || registryScrollEl;
      if (!el) {
        _registryScrollTick = false;
        return;
      }
      if (el.scrollTop + el.clientHeight >= el.scrollHeight - 200) {
        registryVisible = Math.min(registryVisible + 100, filteredRegistryItems.length);
        if (registryVisible - registryStart > REGISTRY_MAX_DOM) {
          registryStart = Math.max(0, registryVisible - REGISTRY_MAX_DOM);
        }
      }
      _registryScrollTick = false;
    });
  }
  async function disableSelectedRegistry() {
    const entries = startupRegItems.filter((it) => selectedReg.has(regId(it)));
    if (entries.length === 0) return;
    try {
      await invoke('remove_registry_run', { entries });
      pushLog(
        'SUCCESS',
        `Removed ${entries.length} registry startup entr${entries.length === 1 ? 'y' : 'ies'}`,
        'Optimize'
      );
      toast.success(`Removed ${entries.length} registry entr${entries.length === 1 ? 'y' : 'ies'}`);
      await reloadRegistryItems();
    } catch (e) {
      console.error(e);
    }
  }

  type ScheduledTask = {
    name: string;
    next_run_time: string;
    status: string;
    task_to_run: string;
    author: string;
    is_sus: boolean;
  };
  let tasks = $state<ScheduledTask[]>([]);
  let tasksQuery = $state('');
  let taskFilter = $state<'all' | 'sus'>('all');
  let includeDisabled = $state(true);
  let includeNoNext = $state(true);
  let taskSort = $state<'suspicious' | 'name' | 'next' | 'status' | 'author' | 'command'>(
    'suspicious'
  );
  let includeMicrosoftInSus = $state(false);
  let tasksLoaded = $state(false);
  let loadingTasks = $state(false);
  let tasksVisible = $state(50);
  let tasksSentinel: HTMLElement | null = null;
  let enrichInFlight = $state(0);
  let enriching = $state(new Set<string>());
  let tasksQueryDeb = $state('');
  let selectedTasks = $state(new Set<string>());
  type TaskAction = 'disable' | 'enable' | 'delete' | 'run' | 'end' | '';
  let taskAction = $state<TaskAction>('');
  let showTaskConfirm = $state(false);
  let pendingAction = $state<TaskAction>('');
  let pendingNames = $state<string[]>([]);

  $effect(() => {
    const t = setTimeout(() => (tasksQueryDeb = tasksQuery), 220);
    return () => clearTimeout(t);
  });

  function isCommandSuspicious(cmd: string): boolean {
    const c = (cmd || '').toLowerCase();
    return (
      c.includes('powershell') ||
      c.includes('wscript') ||
      c.includes('cscript') ||
      c.includes('mshta') ||
      c.includes('regsvr32') ||
      c.includes('rundll32') ||
      c.includes('cmd.exe /c') ||
      c.includes('/b64') ||
      c.includes(' -enc ') ||
      c.includes('%temp%') ||
      c.includes('appdata') ||
      c.includes('http://') ||
      c.includes('https://')
    );
  }

  function isHardCommandSuspicious(cmd: string): boolean {
    const c = (cmd || '').toLowerCase();
    return (
      c.includes(' -enc ') || c.includes('/b64') || c.includes('http://') || c.includes('https://')
    );
  }

  function isUnderMicrosoft(name: string): boolean {
    const parts = splitTaskName(name);
    return (parts.folder || '').startsWith('\\Microsoft\\Windows');
  }

  async function enrichTaskByName(name: string) {
    if (enriching.has(name)) return;
    enriching.add(name);
    if (enrichInFlight >= 3) {
      enriching.delete(name);
      return;
    }
    enrichInFlight += 1;
    try {
      const result = (await invoke('get_task_details', { task_name: name })) as
        | [string, string]
        | any;
      let task_to_run = '';
      let author = '';
      if (Array.isArray(result)) {
        task_to_run = result[0] ?? '';
        author = result[1] ?? '';
      } else if (result && typeof result === 'object') {
        task_to_run = result.task_to_run ?? '';
        author = result.author ?? '';
      }
      const baseSus = isCommandSuspicious(task_to_run);
      let finalSus = baseSus;
      if (!includeMicrosoftInSus && isUnderMicrosoft(name)) {
        finalSus = isHardCommandSuspicious(task_to_run);
      }
      tasks = tasks.map((t) =>
        t.name === name ? { ...t, task_to_run, author, is_sus: finalSus } : t
      );
    } catch (e) {
      // ignore
    } finally {
      enrichInFlight -= 1;
      enriching.delete(name);
    }
  }

  function queueEnrichVisibleTasks(limit = 10) {
    const slice = tasks.slice(0, Math.max(tasksVisible, 200));
    let queued = 0;
    for (const t of slice) {
      if (!t.task_to_run && !enriching.has(t.name)) {
        enrichTaskByName(t.name);
        queued += 1;
        if (queued >= limit) break;
      }
    }
  }

  $effect(() => {
    const _ = taskFilter;
    queueEnrichVisibleTasks(20);
  });

  function splitTaskName(full: string): { base: string; folder: string } {
    if (!full) return { base: '', folder: '' };
    const idx = full.lastIndexOf('\\');
    if (idx <= 0) return { base: full.replace(/^\\+/, ''), folder: '\\' };
    const folder = full.slice(0, idx) || '\\';
    const base = full.slice(idx + 1);
    return { base, folder };
  }

  async function runSelectedTasks() {
    const names = Array.from(selectedTasks);
    if (!taskAction || names.length === 0) {
      message = 'Select an action and at least one task.';
      return;
    }
    try {
      let res: any = null;
      if (taskAction === 'disable') res = await invoke('disable_scheduled_tasks', { names });
      else if (taskAction === 'enable') res = await invoke('enable_scheduled_tasks', { names });
      else if (taskAction === 'delete') res = await invoke('delete_scheduled_tasks', { names });
      else if (taskAction === 'run') res = await invoke('run_scheduled_tasks', { names });
      else if (taskAction === 'end') res = await invoke('end_scheduled_tasks', { names });
      const success = Number((res as any)?.success ?? (res as any) ?? 0);
      const elevated = Number((res as any)?.elevated ?? 0);
      const stopped = Number((res as any)?.stopped ?? 0);
      let parts: string[] = [`${taskAction} affected ${success} task(s)`];
      if (elevated > 0) parts.push(`used elevation for ${elevated}`);
      if (stopped > 0) parts.push(`stopped ${stopped} before delete`);
      message = parts.join(' Ã‚Â· ');
      const fails = (res as any)?.failures as Array<any> | undefined;
      if (Array.isArray(fails) && fails.length) {
        try {
          console.groupCollapsed(`[tasks] ${taskAction} failures (${fails.length})`);
          for (const f of fails) {
            const name = f?.name || 'unknown';
            const step = f?.step || 'n/a';
            const elevated = !!f?.elevated;
            const stderr = (f?.stderr || '').trim();
            const stdout = (f?.stdout || '').trim();
            console.log({ name, step, elevated, stderr, stdout });
          }
          console.groupEnd();
        } catch {
          // ignore
        }
      }
      await loadTasks();
      selectedTasks = new Set();
    } catch (e) {
      console.error(e);
      message = `Action failed: ${e}`;
    }
  }

  async function loadTasks() {
    if (loadingTasks || tasksLoaded) return;
    loadingTasks = true;
    try {
      const res = (await invoke('list_scheduled_tasks')) as ScheduledTask[];
      tasks = Array.isArray(res) ? res : [];
      tasksLoaded = true;
      tasksVisible = Math.min(tasks.length, 50);
      try {
        const susNames = (await invoke('list_suspicious_tasks')) as string[];
        if (Array.isArray(susNames) && susNames.length) {
          const initial = includeMicrosoftInSus
            ? susNames
            : susNames.filter((nm) => !isUnderMicrosoft(nm));
          const susSet = new Set(initial);
          tasks = tasks.map((t) => (susSet.has(t.name) ? { ...t, is_sus: true } : t));
        }
      } catch { /* noop */ }
      queueEnrichVisibleTasks(12);
    } catch (e) {
      console.error(e);
    } finally {
      loadingTasks = false;
    }
  }
  function onTasksScroll(event: Event) {
    const target = event.currentTarget as HTMLElement | null;
    const el = (target as HTMLElement | null) || tasksScrollEl;
    if (!el) return;
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 200) {
      tasksVisible = Math.min(tasksVisible + 200, sortedTasks.length);
      queueEnrichVisibleTasks(8);
    }
  }

  const filteredStartupItems = $derived(
    startupItems.filter((it) => {
      const q = startupQueryDeb.trim().toLowerCase();
      if (q === '') return true;
      return it.name.toLowerCase().includes(q) || it.path.toLowerCase().includes(q);
    })
  );
  const allStartupSelected = $derived(
    filteredStartupItems.length > 0 &&
      filteredStartupItems.every((it) => selectedStartup.has(it.path))
  );
  function toggleAllStartup() {
    if (filteredStartupItems.length === 0) return;
    selectedStartup = allStartupSelected
      ? new Set()
      : new Set(filteredStartupItems.map((it) => it.path));
  }

  $effect(() => {
    const total = filteredStartupItems.length;
    if (startupVisible > total) startupVisible = total;
    if (startupVisible - startupStart > STARTUP_MAX_DOM) {
      startupStart = Math.max(0, startupVisible - STARTUP_MAX_DOM);
    }
    if (startupStart > startupVisible) startupStart = 0;
  });

  const filteredRegistryItems = $derived(
    startupRegItems.filter((it) => {
      const q = registryQueryDeb.trim().toLowerCase();
      if (q === '') return true;
      return (
        it.name.toLowerCase().includes(q) ||
        it.command.toLowerCase().includes(q) ||
        it.key.toLowerCase().includes(q) ||
        it.hive.toLowerCase().includes(q)
      );
    })
  );

  const suspectEntries = $derived(
    Object.entries(registryHistory)
      .filter(([, v]) => !!(v as any)?.suspicious)
      .map(([id, v]) => {
        const [hive, key, name] = id.split('|');
        const reason = (v as any)?.suspiciousReason || 'Reappeared after cleanup';
        const lastStrategy = (v as any)?.lastStrategy || '';
        return { id, hive, key, name, reason, lastStrategy };
      })
  );

  const allRegistrySelected = $derived(
    filteredRegistryItems.length > 0 &&
      filteredRegistryItems.every((it) => selectedReg.has(regId(it)))
  );

  function toggleAllRegistry() {
    if (filteredRegistryItems.length === 0) return;
    selectedReg = allRegistrySelected ? new Set() : new Set(filteredRegistryItems.map(regId));
  }

  $effect(() => {
    const total = filteredRegistryItems.length;
    if (registryVisible > total) registryVisible = total;
    if (registryVisible - registryStart > REGISTRY_MAX_DOM) {
      registryStart = Math.max(0, registryVisible - REGISTRY_MAX_DOM);
    }
    if (registryStart > registryVisible) registryStart = 0;
  });

  let showSuspectLog = $state(false);

  const filteredTasks = $derived(
    tasks.filter((t) => {
      const q = tasksQueryDeb.trim().toLowerCase();
      if (taskFilter === 'sus' && !t.is_sus) return false;
      const status = (t.status || '').toLowerCase();
      if (!includeDisabled && status.includes('disable')) return false;
      const next = (t.next_run_time || '').trim().toLowerCase();
      const hasNext = next !== '' && next !== 'n/a' && next !== 'Ã¢â‚¬â€' && next !== '-';
      if (!includeNoNext && !hasNext) return false;
      if (q === '') return true;
      return (
        (t.name || '').toLowerCase().includes(q) ||
        (t.task_to_run || '').toLowerCase().includes(q) ||
        status.includes(q) ||
        (t.author || '').toLowerCase().includes(q)
      );
    })
  );
  const sortedTasks = $derived.by(() => {
    const arr: ScheduledTask[] = [...filteredTasks];
    const cmpStr = (a?: string, b?: string) =>
      (a ?? '').localeCompare(b ?? '', undefined, { sensitivity: 'base' });
    if (taskSort === 'suspicious') {
      arr.sort((a, b) => (a.is_sus === b.is_sus ? cmpStr(a.name, b.name) : a.is_sus ? -1 : 1));
    } else if (taskSort === 'name') {
      arr.sort((a, b) => cmpStr(a.name, b.name));
    } else if (taskSort === 'next') {
      const toTime = (s?: string) => {
        const v = (s || '').trim();
        if (!v || v.toLowerCase() === 'n/a' || v === 'Ã¢â‚¬â€' || v === '-')
          return Number.POSITIVE_INFINITY;
        const t = Date.parse(v);
        return isNaN(t) ? Number.POSITIVE_INFINITY : t;
      };
      arr.sort((a, b) => toTime(a.next_run_time) - toTime(b.next_run_time));
    } else if (taskSort === 'status') {
      arr.sort((a, b) => cmpStr(a.status, b.status));
    } else if (taskSort === 'author') {
      arr.sort((a, b) => cmpStr(a.author, b.author));
    } else if (taskSort === 'command') {
      arr.sort((a, b) => cmpStr(a.task_to_run, b.task_to_run));
    }
    return arr as ScheduledTask[];
  });

  $effect(() => {
    const total = sortedTasks.length;
    if (tasksVisible > total) tasksVisible = total;
    if (tasksVisible - tasksStart > TASKS_MAX_DOM) {
      tasksStart = Math.max(0, tasksVisible - TASKS_MAX_DOM);
    }
    if (tasksStart > tasksVisible) tasksStart = 0;
  });

  const allTasksSelected = $derived(
    sortedTasks.length > 0 &&
      sortedTasks.slice(tasksStart, tasksVisible).every((t) => selectedTasks.has(t.name))
  );

  function toggleTask(name: string) {
    if (selectedTasks.has(name)) selectedTasks.delete(name);
    else selectedTasks.add(name);
    selectedTasks = new Set(selectedTasks);
  }

  function toggleAllTasks() {
    const slice = sortedTasks.slice(tasksStart, tasksVisible);
    if (slice.length === 0) return;
    if (allTasksSelected) selectedTasks = new Set();
    else selectedTasks = new Set(slice.map((t) => t.name));
  }

  function isLikelyProtected(name: string): boolean {
    const t = tasks.find((x) => x.name === name);
    if (!t) return false;
    const parts = splitTaskName(t.name);
    return !!t.is_sus || (parts.folder || '').startsWith('\\Microsoft\\Windows');
  }

  function requestRunSelected() {
    const names = Array.from(selectedTasks);
    if (!taskAction || names.length === 0) {
      message = 'Select an action and at least one task.';
      return;
    }
    pendingAction = taskAction;
    pendingNames = names;
    showTaskConfirm = true;
  }

  function confirmRunAction() {
    showTaskConfirm = false;
    taskAction = pendingAction;
    selectedTasks = new Set(pendingNames);
    void runSelectedTasks();
  }

  function switchToDisableAndRun() {
    pendingAction = 'disable';
    confirmRunAction();
  }

  async function runNetworkAction(
    label: string,
    action: () => Promise<string>
  ) {
    try {
      const output = await action();
      const message = (output ?? '').trim();
      if (message) {
        toast.success(message);
      } else {
        toast.success(`${label} completed`);
      }
    } catch (error: unknown) {
      console.error(error);
      const text =
        error instanceof Error
          ? error.message
          : typeof error === 'string'
          ? error
          : `${label} failed`;
      toast.error(text);
    }
  }

  async function flushDns() {
    await runNetworkAction('Flush DNS', () => invoke<string>('flush_dns'));
  }

  async function resetWinsock() {
    await runNetworkAction('Reset Winsock', () => invoke<string>('reset_winsock'));
  }

  async function renewIp() {
    await runNetworkAction('Renew IP', () => invoke<string>('renew_ip'));
  }

  onMount(() => {
    void initBootCheck();
    const io = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          if (startupSentinel && entry.target === startupSentinel) loadStartupItems();
          if (registrySentinel && entry.target === registrySentinel) loadRegistryItems();
          if (tasksSentinel && entry.target === tasksSentinel) loadTasks();
        }
      },
      { root: null, rootMargin: '0px', threshold: 0.1 }
    );
    if (startupSentinel) io.observe(startupSentinel);
    if (registrySentinel) io.observe(registrySentinel);
    if (tasksSentinel) io.observe(tasksSentinel);
    try {
      _startupPollTimer = setInterval(pollStartupOnce, 5000) as unknown as number;
    } catch { /* noop */ }
    try {
      _registryPollTimer = setInterval(pollRegistryOnce, 6000) as unknown as number;
    } catch { /* noop */ }
    return () => {
      io.disconnect();
      try {
        if (_startupPollTimer) clearInterval(_startupPollTimer as unknown as number);
      } catch { /* noop */ }
      try {
        if (_registryPollTimer) clearInterval(_registryPollTimer as unknown as number);
      } catch { /* noop */ }
      _startupPollTimer = null;
      _registryPollTimer = null;
    };
  });

  $effect(() => {
    if (registryLoaded) {
      void scanSuspiciousAfterReboot();
    }
  });
</script>

<div class="space-y-6 text-foreground">
  <Card>
    <CardHeader>
      <CardTitle class="text-2xl">Optimize</CardTitle>
      <CardDescription
        >Manage startup items, registry Run keys, and scheduled tasks.</CardDescription>
    </CardHeader>
  </Card>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
    <Card class="gap-4 py-4">
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <Trash2 class="size-5" /> Startup Apps
        </CardTitle>
        <CardDescription>Disable unwanted startup items (Startup folders).</CardDescription>
      </CardHeader>
      <CardContent class="space-y-2">
        <div class="flex items-center gap-1 rounded-md bg-muted/20 p-1 w-fit">
          <Button
            variant="ghost"
            size="icon"
            title="Refresh"
            aria-label="Refresh"
            onclick={reloadStartupItems}
          >
            <RefreshCw class="size-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            title="Open Startup Folders"
            aria-label="Open Startup Folders"
            onclick={openStartupFolders}
          >
            <FolderOpen class="size-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            title="Disable Selected"
            aria-label="Disable Selected"
            onclick={requestRemoveSelectedStartup}
            disabled={selectedStartup.size === 0}
          >
            <Trash2 class="size-4" />
          </Button>
        </div>
        <div class="flex items-center gap-2">
          <div class="relative flex-1">
            <SearchIcon
              class="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground"
            />
            <Input class="pl-9" placeholder="Filter by name or path..." bind:value={startupQuery} />
          </div>
          <div class="flex gap-1">
            <Button
              size="sm"
              variant="ghost"
              onclick={toggleAllStartup}
              disabled={filteredStartupItems.length === 0}
              >{allStartupSelected ? 'Deselect All' : 'Select All'}</Button
            >
          </div>
        </div>
        <div bind:this={startupSentinel} class="h-0" aria-hidden="true"></div>
        {#if loadingStartup && startupItems.length === 0}
          <div
            role="status"
            aria-busy="true"
            class="h-[300px] rounded-md border border-border/60 bg-muted/20 p-2 overflow-hidden"
          >
            <ul class="divide-y divide-border/60">
              {#each Array.from({ length: 8 }) as _, i}
                <li class="px-2 py-2">
                  <div class="flex items-center gap-3">
                    <Skeleton class="h-5 w-5 rounded-md" aria-hidden="true" />
                    <div class="flex-1 space-y-2">
                      <Skeleton class="h-3 w-2/3" aria-hidden="true" />
                      <Skeleton class="h-3 w-5/6" aria-hidden="true" />
                    </div>
                    <Skeleton class="h-4 w-10" aria-hidden="true" />
                  </div>
                </li>
              {/each}
            </ul>
          </div>
        {:else if filteredStartupItems.length > 0}
          <div
            class="h-[300px] rounded-md bg-muted/10 overflow-y-auto"
            bind:this={startupScrollEl}
            data-vt-scope="startup-list"
            onscroll={onStartupScroll}
          >
            <ul class="divide-y divide-border/60">
              {#if startupStart > 0}
                <li style={`height:${startupStart * STARTUP_ROW_PX}px`} aria-hidden="true"></li>
              {/if}
              {#each filteredStartupItems.slice(startupStart, startupVisible) as item (item.path)}
                <li class="px-2 py-2 rounded-sm hover:bg-muted/30 transition-colors">
                  <label class="flex items-start justify-between gap-3">
                    <div class="flex items-start gap-3 min-w-0 flex-1">
                      <Checkbox
                        checked={selectedStartup.has(item.path)}
                        onCheckedChange={() => toggleStartup(item.path)}
                      />
                      <div class="min-w-0">
                        <div class="font-semibold truncate">{item.name}</div>
                        <div class="text-xs text-muted-foreground font-mono truncate max-w-[60ch]">
                          {normalizeWinPath(item.path)}
                        </div>
                      </div>
                    </div>
                    <div class="flex items-center gap-1 shrink-0">
                      <Button
                        variant="ghost"
                        size="icon"
                        title="Open"
                        aria-label="Open"
                        onclick={() => openPath(normalizeWinPath(item.path))}
                        ><Eye class="size-4" /></Button
                      >
                      <Button
                        variant="ghost"
                        size="icon"
                        title="Show in Explorer"
                        aria-label="Show in Explorer"
                        onclick={() => revealItemInDir(normalizeWinPath(item.path))}
                        ><FolderOpen class="size-4" /></Button
                      >
                    </div>
                  </label>
                </li>
              {/each}
              {#if startupVisible < filteredStartupItems.length}
                <li class="px-2 py-2"><Skeleton class="h-[20px] w-full" aria-hidden="true" /></li>
              {/if}
            </ul>
          </div>
        {:else if startupLoaded}
          <p class="text-sm text-muted-foreground">No startup items found.</p>
        {:else}
          <div
            role="status"
            aria-busy="true"
            class="h-[300px] rounded-md border border-border/60 bg-muted/20 p-2 overflow-hidden"
          >
            <ul class="divide-y divide-border/60">
              {#each Array.from({ length: 6 }) as _, i}
                <li class="px-2 py-2">
                  <div class="flex items-center gap-3">
                    <Skeleton class="h-5 w-5 rounded-md" aria-hidden="true" />
                    <div class="flex-1 space-y-2">
                      <Skeleton class="h-3 w-2/3" aria-hidden="true" />
                      <Skeleton class="h-3 w-5/6" aria-hidden="true" />
                    </div>
                    <Skeleton class="h-4 w-10" aria-hidden="true" />
                  </div>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </CardContent>
    </Card>

    <AlertDialog open={showRegistryConfirm} onOpenChange={(v) => (showRegistryConfirm = !!v)}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Remove Registry Startup</AlertDialogTitle>
          <AlertDialogDescription>
            This removes selected Run/RunOnce entries. The app will attempt forced removal if needed
            (may prompt for admin).
          </AlertDialogDescription>
        </AlertDialogHeader>
        {#if regPreset === 'full'}
          <Alert class="mb-2">
            <AlertDescription>
              Full cleanup runs an aggressive cleanup: forced removal, IFEO blocking, and
              delete-on-reboot, purging StartupApproved and removing related Scheduled Tasks and WMI
              subscriptions You may get UAC prompts. Save your work and close unnecessary apps
              before continuing.
            </AlertDescription>
          </Alert>
        {:else if regPreset === 'aggressive'}
          <Alert class="mb-2">
            <AlertDescription>
              Aggressive: tries forced removal, blocks execution via IFEO (if an exe image is
              found), and schedules delete-on-reboot when a full path exists. Does not purge
              StartupApproved, Scheduled Tasks, or WMI automatically. May prompt UAC.
            </AlertDescription>
          </Alert>
        {:else if regPreset === 'force'}
          <Alert class="mb-2">
            <AlertDescription>
              Force only: takes ownership/permissions on the key and removes the value with
              elevation. Does not use IFEO, delete-on-reboot or workarounds. Good when a simple
              removal is denied.
            </AlertDescription>
          </Alert>
        {:else}
          <Alert class="mb-2">
            <AlertDescription>
              Basic: attempts normal removal without elevation. Fastest path when nothing is locking
              the entry. If it fails, try Force or Aggressive.
            </AlertDescription>
          </Alert>
        {/if}
        <div class="space-y-4 text-sm">
          <div class="flex items-center justify-between gap-2 pr-2">
            <div>Selected: {pendingRegistry.length}</div>
            <Select type="single" bind:value={regPreset}>
              <SelectTrigger class="w-40">
                <p>{regPreset}</p>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="basic">Basic</SelectItem>
                <SelectItem value="force">Force only</SelectItem>
                <SelectItem value="aggressive">Aggressive</SelectItem>
                <SelectItem value="full">Full cleanup</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {#if pendingRegistry.length}
            <ul class="max-h-40 overflow-auto rounded bg-muted/10 p-2 text-xs">
              {#each pendingRegistry.slice(0, 10) as it}
                <li class="truncate">
                  {it.name}
                  <span class="text-muted-foreground"> — {it.hive}\{it.key}</span>
                </li>
              {/each}
              {#if pendingRegistry.length > 10}
                <li class="text-muted-foreground">
                  … and {pendingRegistry.length - 10} more
                </li>
              {/if}
            </ul>
          {/if}
        </div>
        <AlertDialogFooter class="pr-2">
          <AlertDialogCancel onclick={() => (showRegistryConfirm = false)}>Cancel</AlertDialogCancel>
          <AlertDialogAction onclick={confirmRemoveRegistry}>Continue</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <Dialog open={showSuspectLog} onOpenChange={(v) => (showSuspectLog = !!v)}>
      <DialogContent class="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Suspicious Entries (Registry Startup)</DialogTitle>
          <DialogDescription
            >Entries that reappeared after Full cleanup and reboot.</DialogDescription
          >
        </DialogHeader>
        {#if suspectEntries.length === 0}
          <p class="text-sm text-muted-foreground">No suspicious entries recorded.</p>
        {:else}
          <div class="max-h-[50vh] overflow-y-auto space-y-3">
            {#each suspectEntries as s}
              <div class="rounded-md border p-3 bg-muted/10">
                <div class="flex items-center gap-2">
                  <Badge variant="destructive">Suspicious</Badge>
                  <span class="font-medium">{s.name}</span>
                  {#if s.lastStrategy}
                    <Badge variant="secondary" class="ml-auto">{s.lastStrategy}</Badge>
                  {/if}
                </div>
                <div class="mt-1 text-xs text-muted-foreground font-mono break-all">
                  {s.hive}\{s.key}
                </div>
                <p class="mt-2 text-sm">{s.reason}</p>
              </div>
            {/each}
          </div>
        {/if}
        <DialogFooter>
          <Button variant="secondary" onclick={() => (showSuspectLog = false)}>Close</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <Dialog open={showPostCleanup} onOpenChange={(v) => (showPostCleanup = !!v)}>
      <DialogContent class="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Action Complete</DialogTitle>
          <DialogDescription>Quick system check and recommended next steps.</DialogDescription>
        </DialogHeader>

        {#if postDiagLoading}
          <div class="text-sm text-muted-foreground">Running system checkâ€¦</div>
        {:else if postDiag}
          <div class="space-y-4 text-sm">
            <div>
              <p class="font-medium">Registry Entries</p>
              {#if postDiag.removedRegistry.stillPresent.length === 0}
                <p class="text-emerald-600 dark:text-emerald-400">
                  All selected entries appear removed.
                </p>
              {:else}
                <p class="text-destructive">Some entries remain:</p>
                <ul class="list-disc pl-5 mt-1">
                  {#each postDiag.removedRegistry.stillPresent.slice(0, 6) as r}
                    <li class="break-all">{r}</li>
                  {/each}
                  {#if postDiag.removedRegistry.stillPresent.length > 6}
                    <li>â€¦and {postDiag.removedRegistry.stillPresent.length - 6} more</li>
                  {/if}
                </ul>
              {/if}
            </div>

            <div>
              <p class="font-medium">Processes</p>
              {#if postDiag.runningImages.running.length === 0}
                <p class="text-emerald-600 dark:text-emerald-400">
                  No related processes are running.
                </p>
              {:else}
                <p class="text-destructive">Processes still running:</p>
                <p class="font-mono">{postDiag.runningImages.running.join(', ')}</p>
              {/if}
            </div>

            <div>
              <p class="font-medium">Scheduled Tasks</p>
              {#if postDiag.taskMatches.remaining.length === 0}
                <p class="text-emerald-600 dark:text-emerald-400">No related tasks found.</p>
              {:else}
                <p class="text-destructive">Related tasks remaining:</p>
                <ul class="list-disc pl-5 mt-1">
                  {#each postDiag.taskMatches.remaining.slice(0, 8) as t}
                    <li class="break-all">{t}</li>
                  {/each}
                  {#if postDiag.taskMatches.remaining.length > 8}
                    <li>â€¦and {postDiag.taskMatches.remaining.length - 8} more</li>
                  {/if}
                </ul>
              {/if}
            </div>

            <div>
              <p class="font-medium">Services</p>
              {#if postDiag.serviceMatches.running.length === 0 && postDiag.serviceMatches.disabled.length === 0}
                <p class="text-emerald-600 dark:text-emerald-400">
                  Inga related services hittades.
                </p>
              {:else}
                {#if postDiag.serviceMatches.running.length > 0}
                  <p class="text-destructive">
                    Running services: {postDiag.serviceMatches.running.join(', ')}
                  </p>
                {/if}
                {#if postDiag.serviceMatches.disabled.length > 0}
                  <p class="text-muted-foreground">
                    Disabled: {postDiag.serviceMatches.disabled.join(', ')}
                  </p>
                {/if}
              {/if}
            </div>

            <div class="space-y-2">
              <p class="font-medium">Recommended Next Steps</p>
              <ul class="list-disc pl-5">
                {#if postDiag.rebootRecommended}
                  <li>Restart your computer to complete delete-on-reboot.</li>
                {/if}
                {#if postDiag.runningImages.running.length > 0}
                  <li>
                    Close or uninstall the processes still running: {postDiag.runningImages.running.join(
                      ', '
                    )}.
                  </li>
                {/if}
                {#if postDiag.taskMatches.remaining.length > 0}
                  <li>Open Task Scheduler and remove the remaining tasks above.</li>
                {/if}
                {#if postDiag.serviceMatches.running.length > 0}
                  <li>Stop/Disable related services and verify nothing re-enables them.</li>
                {/if}
                <li>Run an antivirus/antimalware scan if the entries were malicious.</li>
              </ul>
            </div>
          </div>
        {/if}

        <DialogFooter class="flex flex-wrap gap-2">
          {#if postDiag?.rebootRecommended}
            <Button
              onclick={async () => {
                try {
                  await invoke('restart_system');
                } catch (e) {
                  console.error(e);
                  toast.error('Could not restart the system');
                }
              }}
            >
              Starta om
            </Button>
          {/if}
          <Button variant="outline" onclick={() => (showPostCleanup = false)}>Senare</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <Card class="gap-4 py-4">
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <Settings class="size-5" /> Registry Startup (Run keys)
        </CardTitle>
        <CardDescription>Entries from HKCU/HKLM Run and RunOnce keys.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-2">
        <div class="flex items-center gap-1 rounded-md bg-muted/20 p-1 w-fit">
          <Button
            variant="ghost"
            size="icon"
            title="Refresh"
            aria-label="Refresh"
            onclick={reloadRegistryItems}
          >
            <RefreshCw class="size-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            title="Disable Selected"
            aria-label="Disable Selected"
            onclick={requestRemoveSelectedRegistry}
            disabled={selectedReg.size === 0}
          >
            <Trash2 class="size-4" />
          </Button>
          {#if Object.values(registryHistory).some((r: any) => r?.suspicious)}
            <Button
              variant="ghost"
              size="icon"
              title="Show suspicious log"
              aria-label="Show suspicious log"
              onclick={() => (showSuspectLog = true)}
            >
              <Flag class="size-4 text-destructive" />
            </Button>
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <div class="relative flex-1">
            <SearchIcon
              class="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground"
            />
            <Input
              class="pl-9"
              placeholder="Filter by name, command, or key..."
              bind:value={registryQuery}
            />
          </div>
          <div class="flex gap-1">
            <Button
              size="sm"
              variant="ghost"
              onclick={toggleAllRegistry}
              disabled={filteredRegistryItems.length === 0}
            >
              {allRegistrySelected ? 'Deselect All' : 'Select All'}
            </Button>
          </div>
        </div>
        <div bind:this={registrySentinel} class="h-0" aria-hidden="true"></div>
        {#if loadingRegistry && startupRegItems.length === 0}
          <div
            role="status"
            aria-busy="true"
            class="h-[300px] rounded-md border border-border/60 bg-muted/20 p-2 overflow-hidden"
          >
            <ul class="divide-y divide-border/60">
              {#each Array.from({ length: 8 }) as _, i}
                <li class="px-2 py-2">
                  <div class="flex items-center gap-3">
                    <Skeleton class="h-5 w-5 rounded-md" aria-hidden="true" />
                    <div class="flex-1 space-y-2">
                      <Skeleton class="h-3 w-1/2" aria-hidden="true" />
                      <Skeleton class="h-3 w-4/5" aria-hidden="true" />
                    </div>
                    <Skeleton class="h-4 w-10" aria-hidden="true" />
                  </div>
                </li>
              {/each}
            </ul>
          </div>
        {:else if filteredRegistryItems.length > 0}
          <div
            class="h-[300px] rounded-md bg-muted/10 overflow-y-auto"
            bind:this={registryScrollEl}
            data-vt-scope="registry-list"
            onscroll={onRegistryScroll}
          >
            <ul class="divide-y divide-border/60">
              {#if registryStart > 0}
                <li style={`height:${registryStart * REGISTRY_ROW_PX}px`} aria-hidden="true"></li>
              {/if}
              {#each filteredRegistryItems.slice(registryStart, registryVisible) as it (regId(it))}
                <li class="px-2 py-2 space-y-1 rounded-sm hover:bg-muted/30 transition-colors">
                  <label class="flex items-center gap-3">
                    <Checkbox
                      checked={selectedReg.has(regId(it))}
                      onCheckedChange={() => toggleReg(it)}
                    />
                    <span class="font-semibold">
                      {it.name}
                      {#if (registryHistory[regId(it)] as any)?.suspicious}
                        <Badge variant="destructive" class="ml-2">Suspicious</Badge>
                      {/if}
                    </span>
                  </label>
                  <div class="text-xs text-muted-foreground">{it.hive}\{it.key}</div>
                  <div
                    class="flex items-center justify-between gap-3 text-xs text-muted-foreground"
                  >
                    <span class="font-mono truncate max-w-[52ch]">{it.command}</span>
                    <div class="flex items-center gap-1 shrink-0">
                      <Button
                        variant="ghost"
                        size="icon"
                        title="Copy command"
                        aria-label="Copy command"
                        onclick={() => copyText(it.command)}><CopyIcon class="size-4" /></Button
                      >
                      <Button
                        variant="ghost"
                        size="icon"
                        title="Open in Registry"
                        aria-label="Open in Registry"
                        onclick={() => {
                          void invoke('open_registry_key', { hive: it.hive, key: it.key });
                        }}
                      >
                        <FolderOpen class="size-4" />
                      </Button>
                    </div>
                  </div>
                </li>
              {/each}
              {#if registryVisible < filteredRegistryItems.length}
                <li class="px-2 py-2"><Skeleton class="h-[20px] w-full" aria-hidden="true" /></li>
              {/if}
            </ul>
          </div>
        {:else if registryLoaded}
          <p class="text-sm text-muted-foreground">No registry startup entries found.</p>
        {:else}
          <div
            role="status"
            aria-busy="true"
            class="h-[300px] rounded-md border border-border/60 bg-muted/20 p-2 overflow-hidden"
          >
            <ul class="divide-y divide-border/60">
              {#each Array.from({ length: 6 }) as _, i}
                <li class="px-2 py-2">
                  <div class="flex items-center gap-3">
                    <Skeleton class="h-5 w-5 rounded-md" aria-hidden="true" />
                    <div class="flex-1 space-y-2">
                      <Skeleton class="h-3 w-1/2" aria-hidden="true" />
                      <Skeleton class="h-3 w-4/5" aria-hidden="true" />
                    </div>
                    <Skeleton class="h-4 w-10" aria-hidden="true" />
                  </div>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </CardContent>
    </Card>

    <AlertDialog open={showStartupConfirm} onOpenChange={(v) => (showStartupConfirm = !!v)}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Remove Startup Shortcuts</AlertDialogTitle>
          <AlertDialogDescription>
            This disables selected startup apps by removing their shortcuts. The app will attempt
            forced removal if needed (may prompt for admin).
          </AlertDialogDescription>
        </AlertDialogHeader>
        <div class="space-y-2 text-sm">
          <div>Selected: {pendingStartup.length}</div>
          {#if pendingStartup.length}
            <ul class="max-h-40 overflow-auto rounded bg-muted/10 p-2 text-xs">
              {#each pendingStartup.slice(0, 10) as p}
                <li class="truncate">{normalizeWinPath(p)}</li>
              {/each}
              {#if pendingStartup.length > 10}
                <li class="text-muted-foreground">… and {pendingStartup.length - 10} more</li>
              {/if}
            </ul>
          {/if}
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel onclick={() => (showStartupConfirm = false)}>Cancel</AlertDialogCancel>
          <AlertDialogAction onclick={confirmRemoveStartup}>Continue</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <Card class="gap-4 py-4">
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <NetworkIcon class="size-5" /> Network
        </CardTitle>
        <CardDescription>Quick network tune-ups.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-2">
        <div class="flex items-center justify-between gap-2 flex-wrap">
          <div class="flex items-center gap-1 rounded-md bg-muted/20 p-1">
            <Button
              variant="ghost"
              size="sm"
              onclick={flushDns}
              title="Flush DNS Cache"
              aria-label="Flush DNS Cache"
              ><RefreshCcw class="size-4" /><span class="ml-1 hidden sm:inline">Flush DNS</span
              ></Button
            >
            <Button
              variant="ghost"
              size="sm"
              onclick={resetWinsock}
              title="Reset Winsock"
              aria-label="Reset Winsock"
              ><RotateCcw class="size-4" /><span class="ml-1 hidden sm:inline">Reset Winsock</span
              ></Button
            >
            <Button
              variant="ghost"
              size="sm"
              onclick={renewIp}
              title="Renew IP"
              aria-label="Renew IP"
              ><RefreshCw class="size-4" /><span class="ml-1 hidden sm:inline">Renew IP</span
              ></Button
            >
          </div>
        </div>
        <p class="text-xs text-muted-foreground">
          Actions may briefly interrupt connectivity. Some changes can require a reboot.
        </p>
      </CardContent>
    </Card>

    <Card class="gap-4 py-4">
      <CardHeader>
        <CardTitle class="flex items-center gap-2"
          ><ListChecks class="size-5" /> Scheduled Tasks</CardTitle
        >
        <CardDescription
          >Inspect Task Scheduler entries and highlight suspicious ones.</CardDescription
        >
      </CardHeader>
      <CardContent class="space-y-2">
        <div class="flex items-center gap-2 flex-wrap">
          <div class="relative flex-1">
            <SearchIcon
              class="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground"
            />
            <Input
              class="pl-9"
              placeholder="Search name, command, status or author..."
              bind:value={tasksQuery}
            />
          </div>
          <div class="flex items-center gap-2 flex-wrap">
            <Select type="single" bind:value={taskSort}>
              <SelectTrigger class="w-40">
                <p>
                  {taskSort === 'suspicious'
                    ? 'Suspicious first'
                    : taskSort === 'name'
                    ? 'Name'
                    : taskSort === 'next'
                    ? 'Next run'
                    : taskSort === 'status'
                    ? 'Status'
                    : taskSort === 'author'
                    ? 'Author'
                    : taskSort === 'command'
                    ? 'Command'
                    : 'Sort by'}
                </p>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="suspicious">Suspicious first</SelectItem>
                <SelectItem value="name">Name</SelectItem>
                <SelectItem value="next">Next run</SelectItem>
                <SelectItem value="status">Status</SelectItem>
                <SelectItem value="author">Author</SelectItem>
                <SelectItem value="command">Command</SelectItem>
              </SelectContent>
            </Select>
            <Select type="single">
              <SelectTrigger class="w-40">
                <p>
                  {taskFilter === 'sus' && !includeDisabled && !includeNoNext && !includeMicrosoftInSus
                    ? 'Suspicious only'
                    : taskFilter === 'sus' && (includeDisabled || includeNoNext || includeMicrosoftInSus)
                    ? 'Suspicious + extras'
                    : !taskFilter && (includeDisabled || includeNoNext || includeMicrosoftInSus)
                    ? 'Custom filters'
                    : 'Filters'}
                </p>
              </SelectTrigger>
              <SelectContent>
                <div class="p-2 space-y-2 min-w-[16rem]">
                  <label class="flex items-center gap-2 text-sm whitespace-nowrap">
                    <Checkbox
                      checked={taskFilter === 'sus'}
                      onCheckedChange={() => (taskFilter = taskFilter === 'sus' ? 'all' : 'sus')}
                    />
                    <span class="flex items-center gap-1"
                      ><Flag class="size-4 text-destructive" /> Suspicious only</span
                    >
                  </label>
                  <label class="flex items-center gap-2 text-sm whitespace-nowrap">
                    <Checkbox
                      checked={includeDisabled}
                      onCheckedChange={() => (includeDisabled = !includeDisabled)}
                    />
                    <span>Show disabled</span>
                  </label>
                  <label class="flex items-center gap-2 text-sm whitespace-nowrap">
                    <Checkbox
                      checked={includeNoNext}
                      onCheckedChange={() => (includeNoNext = !includeNoNext)}
                    />
                    <span>Show 'No next run'</span>
                  </label>
                  <label class="flex items-center gap-2 text-sm whitespace-nowrap">
                    <Checkbox
                      checked={includeMicrosoftInSus}
                      onCheckedChange={() => (includeMicrosoftInSus = !includeMicrosoftInSus)}
                    />
                    <span>Include Microsoft system tasks in SUS</span>
                  </label>
                </div>
              </SelectContent>
            </Select>
            <Button
              size="sm"
              variant="secondary"
              onclick={loadTasks}
              title="Refresh tasks"
              aria-label="Refresh tasks"><RefreshCw class="mr-2 size-4" /> Refresh</Button
            >
            <Select type="single" bind:value={taskAction}>
              <SelectTrigger class="w-40">
                <p>
                  {taskAction === 'disable'
                    ? 'Disable'
                    : taskAction === 'enable'
                    ? 'Enable'
                    : taskAction === 'delete'
                    ? 'Delete'
                    : taskAction === 'run'
                    ? 'Run now'
                    : taskAction === 'end'
                    ? 'End'
                    : 'Action'}
                </p>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="disable">Disable</SelectItem>
                <SelectItem value="enable">Enable</SelectItem>
                <SelectItem value="delete">Delete</SelectItem>
                <SelectItem value="run">Run now</SelectItem>
                <SelectItem value="end">End</SelectItem>
              </SelectContent>
            </Select>
            <Button
              size="sm"
              onclick={requestRunSelected}
              disabled={!taskAction || selectedTasks.size === 0}
              title="Run action"
              aria-label="Run action">Run</Button
            >
            <Button
              size="sm"
              variant="ghost"
              onclick={toggleAllTasks}
              disabled={sortedTasks.length === 0}
              >{allTasksSelected ? 'Deselect All' : 'Select All'}</Button
            >
          </div>
        </div>
        <div bind:this={tasksSentinel} class="h-0" aria-hidden="true"></div>
        {#if loadingTasks && tasks.length === 0}
          <div
            role="status"
            aria-busy="true"
            class="h-[300px] rounded-md border border-border/60 bg-muted/20 p-2 overflow-hidden"
          >
            <ul class="divide-y divide-border/60">
              {#each Array.from({ length: 10 }) as _, i}
                <li class="px-3 py-2">
                  <div class="flex items-center justify-between gap-3">
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2">
                        <Skeleton class="h-4 w-60" aria-hidden="true" />
                        <Skeleton class="h-4 w-10 rounded-sm" aria-hidden="true" />
                      </div>
                      <Skeleton class="mt-2 h-3 w-5/6" aria-hidden="true" />
                    </div>
                    <div class="shrink-0 text-right space-y-2">
                      <Skeleton class="h-3 w-16" aria-hidden="true" />
                      <Skeleton class="h-3 w-24" aria-hidden="true" />
                    </div>
                  </div>
                </li>
              {/each}
            </ul>
          </div>
        {:else}
          <div
            class="h-[300px] rounded-md bg-muted/10 overflow-y-auto"
            bind:this={tasksScrollEl}
            onscroll={onTasksScroll}
          >
            <ul class="divide-y divide-border/60">
              {#if tasksStart > 0}
                <li style={`height:${tasksStart * TASKS_ROW_PX}px`} aria-hidden="true"></li>
              {/if}
              {#each sortedTasks.slice(tasksStart, tasksVisible) as t, i (t.name + '|' + (t.task_to_run || '') + '|' + i)}
                {@const parts = splitTaskName(t.name)}
                <li class="px-3 py-2 hover:bg-muted/30 transition-colors">
                  <div class="flex items-center justify-between gap-3">
                    <div class="flex items-start gap-3 min-w-0 flex-1">
                      <Checkbox
                        checked={selectedTasks.has(t.name)}
                        onCheckedChange={() => toggleTask(t.name)}
                      />
                      <div class="min-w-0">
                        <div class="flex items-center gap-2">
                          <span class="font-semibold truncate max-w-[60ch]"
                            >{parts.base || t.name}</span
                          >
                          {#if t.is_sus}<Badge
                              variant="outline"
                              class="text-[10px] border-red-500/30 text-red-600 bg-red-500/10"
                              >SUS</Badge
                            >{/if}
                        </div>
                        <div
                          class="mt-1 text-xs text-muted-foreground font-mono truncate max-w-[80ch]"
                        >
                          {parts.folder}
                        </div>
                      </div>
                    </div>
                    <div class="shrink-0 text-right">
                      <div class="text-xs text-muted-foreground">{t.status || 'Ã¢â‚¬â€'}</div>
                      <div class="text-[10px] text-muted-foreground">
                        Next: {t.next_run_time || 'Ã¢â‚¬â€'}
                      </div>
                    </div>
                  </div>
                </li>
              {/each}
              {#if tasksVisible < sortedTasks.length}
                <li class="px-2 py-2"><Skeleton class="h-[20px] w-full" aria-hidden="true" /></li>
              {/if}
              {#if sortedTasks.length === 0 && tasksLoaded}
                <li class="px-3 py-8 text-center text-xs text-muted-foreground">No tasks match.</li>
              {/if}
            </ul>
          </div>
        {/if}
      </CardContent>
    </Card>
  </div>

  {#if message}
    <div class="lg:col-span-2">
      <Alert><AlertDescription>{message}</AlertDescription></Alert>
    </div>
  {/if}
</div>

<AlertDialog open={showTaskConfirm} onOpenChange={(v) => (showTaskConfirm = !!v)}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Confirm Task Action</AlertDialogTitle>
      <AlertDialogDescription>
        This may require administrator permissions (UAC).
        {#if pendingAction === 'delete'}
          <br />Deleting protected system tasks often fails. Disabling is safer.
        {/if}
      </AlertDialogDescription>
    </AlertDialogHeader>
    <div class="space-y-2 text-sm">
      <div>Action: <span class="font-semibold uppercase">{pendingAction}</span></div>
      <div>Selected: {pendingNames.length}</div>
      {#if pendingNames.length}
        <ul class="max-h-32 overflow-auto rounded bg-muted/10 p-2 text-xs">
          {#each pendingNames.slice(0, 8) as nm}
            {@const p = splitTaskName(nm)}
            <li class="truncate">
              {p.base}
              <span class="text-muted-foreground">{p.folder}</span>{#if isLikelyProtected(nm)}
                <span class="ml-1 text-[10px] text-destructive">protected</span>{/if}
            </li>
          {/each}
          {#if pendingNames.length > 8}
            <li class="text-muted-foreground">Ã¢â‚¬Â¦ and {pendingNames.length - 8} more</li>
          {/if}
        </ul>
      {/if}
    </div>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={() => (showTaskConfirm = false)}>Cancel</AlertDialogCancel>
      {#if pendingAction === 'delete'}
        <AlertDialogAction onclick={switchToDisableAndRun}>Switch to Disable</AlertDialogAction>
      {/if}
      <AlertDialogAction onclick={confirmRunAction}>Continue</AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
