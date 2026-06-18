<script lang="ts">
  import { onMount, tick } from 'svelte';
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
    Copy as CopyIcon,
    Search as SearchIcon,
    Network as NetworkIcon,
    RefreshCcw,
    RotateCcw,
    Settings,
    ListChecks,
    Flag,
    Play,
    CircleX,
    Power,
    PowerOff,
  } from '@lucide/svelte';
  import { SvelteSet } from 'svelte/reactivity';

  import { Tabs, TabsContent, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuLabel,
  } from '$lib/components/ui/dropdown-menu';
  import TweaksPanel from './components/TweaksPanel.svelte';
  import StartupPanel from './components/StartupPanel.svelte';

  import {
    listRegistryRun,
    removeRegistryRun,
    forceRemoveRegistryRun,
    listServices,
    stopServices,
    disableServices,
    blockProcessIfeo,
    scheduleDeleteOnReboot,
    purgeStartupApproved,
    deleteTasksByMatch,
    removeWmiSubscriptionsByMatch,
    runPostCleanupDiagnostics,
    isProcessRunning,
    openRegistryKey,
    regId,
  } from './services/registry';
  import type { StartupRegItem, CleanupDiagnostics, RegistryAttempt } from './services/registry';
  import { listScheduledTasks, getTaskDetails, executeTaskAction } from './services/tasks';
  import type { ScheduledTask, TaskAction, TaskActionResult } from './services/tasks';
  import {
    NETWORK_PRESETS,
    flushDns as flushDnsCommand,
    getNetworkSummary,
    renewIp as renewIpCommand,
    resetWinsock as resetWinsockCommand,
    runDnsLookup as runDnsLookupCommand,
    runPing as runPingCommand,
    runTraceroute as runTracerouteCommand,
  } from './services/network';
  import type { NetworkPresetId, NetworkSummary } from './services/network';
  import { getBootTime, restartSystem } from './services/system';
  import {
    extractExeFromCommand,
    extractExePathFromCommand,
    splitTaskName,
  } from './utils/heuristics';

  let message = $state('');
  let startupRegItems = $state<StartupRegItem[]>([]);
  let selectedReg = $state(new Set<string>());
  let registryQuery = $state('');
  let registryLoaded = $state(false);
  let loadingRegistry = $state(false);
  let registryVisible = $state(50);
  let registryStart = $state(0);
  let registryScrollEl = $state<HTMLElement | null>(null);
  let registrySentinel: HTMLElement | null = null;
  let registryQueryDeb = $state('');
  let showRegistryConfirm = $state(false);
  let pendingRegistry: StartupRegItem[] = $state([]);
  let registryForce = $state(false);
  let registryBlockIFEO = $state(false);
  let registryDeleteOnReboot = $state(false);
  let registryPurgeStartupApproved = $state(false);
  let registryDeleteTasksByMatch = $state(false);
  let registryRemoveWMIByMatch = $state(false);
  let showPostCleanup = $state(false);
  let postDiagLoading = $state(false);
  let postDiag: CleanupDiagnostics | null = $state(null);
  let regPreset = $state<'basic' | 'force' | 'aggressive' | 'full'>('basic');
  let activeOptimizeTab = $state<'tweaks' | 'startup' | 'registry' | 'tasks' | 'network'>('tweaks');

  const REGISTRY_MAX_DOM = 300;
  const REGISTRY_ROW_PX = 56;
  const TASKS_MAX_DOM = 600;
  const TASKS_ROW_PX = 64;

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
    void regHasExeImage;
    void regHasExePath;
    applyRegPreset(_p);
  });

  $effect(() => {
    const t = setTimeout(() => (registryQueryDeb = registryQuery), 200);
    return () => clearTimeout(t);
  });

  let registryHistory = $state<Record<string, RegistryAttempt>>({});
  const REG_HISTORY_KEY = 'avelonia_registry_history_v1';
  $effect(() => {
    try {
      const raw = localStorage.getItem(REG_HISTORY_KEY);
      if (raw) registryHistory = JSON.parse(raw) ?? {};
    } catch {
      /* noop */
    }
  });
  function saveRegHistory() {
    try {
      localStorage.setItem(REG_HISTORY_KEY, JSON.stringify(registryHistory));
    } catch {
      /* noop */
    }
  }

  let rebootDetected = $state(false);
  async function initBootCheck() {
    try {
      const nowBoot = await getBootTime();
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
      const removed = registryForce
        ? await forceRemoveRegistryRun(pendingRegistry)
        : await removeRegistryRun(pendingRegistry);
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
      selectedReg = new SvelteSet();
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
        const services = await listServices();
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
            await stopServices(names);
          } catch {
            /* noop */
          }
          try {
            const n = await disableServices(names);
            if (n > 0) toast.message(`Disabled ${n} related service${n === 1 ? '' : 's'}.`);
          } catch (e) {
            pushLog('WARN', `disable_services failed: ${String(e)}`, 'Optimize');
          }
        }
      } catch (e) {
        pushLog('WARN', `list_services failed: ${String(e)}`, 'Optimize');
      }
      if (registryBlockIFEO && images.length) {
        try {
          const n = await blockProcessIfeo(images, true);
          if (n > 0) toast.success(`Blocked ${n} process${n === 1 ? '' : 'es'} via IFEO.`);
        } catch (e) {
          pushLog('WARN', `block_process_ifeo failed: ${String(e)}`, 'Optimize');
        }
      }
      let dorScheduled = false;
      if (registryDeleteOnReboot && paths.length) {
        try {
          const n = await scheduleDeleteOnReboot(paths);
          if (n > 0) {
            toast.message(`Scheduled delete on reboot for ${n} file${n === 1 ? '' : 's'}.`);
            dorScheduled = true;
          }
        } catch (e) {
          pushLog('WARN', `schedule_delete_on_reboot failed: ${String(e)}`, 'Optimize');
        }
      }
      if (registryPurgeStartupApproved) {
        try {
          const names = pendingRegistry.map((r) => r.name);
          const n = await purgeStartupApproved(names);
          if (n > 0) toast.message(`Purged StartupApproved for ${n} entr${n === 1 ? 'y' : 'ies'}.`);
        } catch (e) {
          pushLog('WARN', `purge_startup_approved failed: ${String(e)}`, 'Optimize');
        }
      }
      if (registryDeleteTasksByMatch && (images.length || paths.length)) {
        try {
          const n = await deleteTasksByMatch(images, paths);
          if (n > 0) toast.message(`Deleted ${n} related scheduled task${n === 1 ? '' : 's'}.`);
        } catch (e) {
          pushLog('WARN', `delete_tasks_by_match failed: ${String(e)}`, 'Optimize');
        }
      }
      if (registryRemoveWMIByMatch && (images.length || paths.length)) {
        try {
          const n = await removeWmiSubscriptionsByMatch(images, paths);
          if (n > 0) toast.message('Removed WMI event subscriptions matching target.');
        } catch (e) {
          pushLog('WARN', `remove_wmi_subscriptions_by_match failed: ${String(e)}`, 'Optimize');
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
        pushLog('WARN', `post cleanup diagnostics failed: ${String(e)}`, 'Optimize');
      } finally {
        postDiagLoading = false;
      }
    } catch (e) {
      pushLog('ERROR', `Cleanup diagnostics failed: ${String(e)}`, 'Optimize');
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
            rebootConfirmed: prev.rebootConfirmed || false,
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
      } catch {
        /* noop */
      }
      pendingRegistry = [];
      registryForce = false;
      registryBlockIFEO = false;
      registryDeleteOnReboot = false;
      registryPurgeStartupApproved = false;
      registryDeleteTasksByMatch = false;
      registryRemoveWMIByMatch = false;
    }
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
          const list = await listRegistryRun();
          const set = new Set(list.map(regId));
          const back = [...ids].some((k) => set.has(k));
          if (back) {
            readded = true;
            toast.warning('Watchdog detected: entry reappeared in Startup (Registry).');
            break;
          }
        } catch {
          /* noop */
        }

        try {
          for (const img of images) {
            const running = await isProcessRunning(img);
            if (running && !restarted.includes(img)) restarted.push(img);
          }
        } catch {
          /* noop */
        }

        await new Promise((r) => setTimeout(r, 2000));
      }
      if (restarted.length > 0) {
        toast.warning(`Process running after removal: ${restarted.join(', ')}`);
      }
      if (!readded) {
        toast.success('Removal appears persistent (no reappearance detected).');
      }
    } catch (e) {
      pushLog('WARN', `monitorRegistryWatchdog failed: ${String(e)}`, 'Optimize');
    }
  }

  async function copyText(txt: string) {
    try {
      await navigator.clipboard.writeText(txt);
    } catch (e) {
      pushLog('ERROR', `Registry scan failed: ${String(e)}`, 'Optimize');
    }
  }

  async function loadRegistryItems() {
    if (loadingRegistry || registryLoaded) return;
    loadingRegistry = true;
    try {
      const res = await listRegistryRun();
      startupRegItems = Array.isArray(res) ? res : [];
      pushLog('INFO', `Registry items loaded: ${startupRegItems.length}`, 'Optimize');
      const keep = new SvelteSet(selectedReg);
      const ids = new Set(startupRegItems.map(regId));
      selectedReg = new SvelteSet(Array.from(keep).filter((k) => ids.has(k)));
      registryLoaded = true;
      registryVisible = Math.min(startupRegItems.length, 50);
    } catch (e) {
      pushLog('ERROR', `Suspicious items scan failed: ${String(e)}`, 'Optimize');
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
      const res = await listRegistryRun();
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
        const keep = new SvelteSet(selectedReg);
        selectedReg = new SvelteSet(Array.from(keep).filter((k) => nextSet.has(k)));
        registryVisible = Math.min(Math.max(50, registryVisible), startupRegItems.length);
      }
    } catch {
      pushLog('WARN', 'Failed to poll registry items', 'Optimize');
    } finally {
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
            `Entry reappeared after Full cleanup and reboot. ${hint.join(' · ')}`.trim();
          registryHistory[id] = r;
          updates += 1;
        }
      }
      if (updates > 0) {
        saveRegHistory();
        toast.warning(`Detected ${updates} recurring entries after reboot. Marked as suspicious.`);
      }
    } catch (e) {
      pushLog('WARN', `scanSuspiciousAfterReboot failed: ${String(e)}`, 'Optimize');
    }
  }
  function toggleReg(it: StartupRegItem) {
    const id = regId(it);
    if (selectedReg.has(id)) selectedReg.delete(id);
    else selectedReg.add(id);
    selectedReg = new SvelteSet(selectedReg);
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
  let tasksStart = $state(0);
  let tasksScrollEl = $state<HTMLElement | null>(null);
  let tasksSentinel: HTMLElement | null = null;
  let enrichInFlight = $state(0);
  let enriching = $state(new Set<string>());
  let tasksQueryDeb = $state('');
  let selectedTasks = $state(new Set<string>());
  let taskAction = $state<TaskAction>('');
  let showTaskConfirm = $state(false);
  let pendingAction = $state<TaskAction>('');
  let pendingNames = $state<string[]>([]);
  let taskActionLoading = $state(false);

  $effect(() => {
    const t = setTimeout(() => (tasksQueryDeb = tasksQuery), 220);
    return () => clearTimeout(t);
  });

  async function enrichTaskByName(name: string) {
    if (enriching.has(name)) return;
    enriching.add(name);
    if (enrichInFlight >= 3) {
      enriching.delete(name);
      return;
    }
    enrichInFlight += 1;
    try {
      const current = tasks.find((t) => t.name === name);
      if (current && current.task_to_run && current.author) {
        return;
      }
      const result = await getTaskDetails(name);
      const [task_to_run, author, is_sus, score] = result;

      tasks = tasks.map((t) =>
        t.name === name ? { ...t, task_to_run, author, is_sus, score } : t
      );
    } catch {
      // ignore
    } finally {
      enrichInFlight -= 1;
      enriching.delete(name);
    }
  }

  function queueEnrichVisibleTasks(limit = 10) {
    const needEnrich = tasks.some((t) => !t.task_to_run || !t.author);
    if (!needEnrich) return;
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
    void taskFilter;
    queueEnrichVisibleTasks(20);
  });

  async function runSelectedTasks() {
    const names = Array.from(selectedTasks);
    if (!taskAction || names.length === 0) {
      message = 'Select an action and at least one task.';
      return;
    }
    if (taskActionLoading) return;
    taskActionLoading = true;
    try {
      const res = await executeTaskAction(taskAction, names);
      const normalized: TaskActionResult =
        typeof res === 'number' ? { success: res } : res ?? {};
      const success = Number(normalized.success ?? 0);
      const elevated = Number(normalized.elevated ?? 0);
      const stopped = Number(normalized.stopped ?? 0);
      let parts: string[] = [`${taskAction} affected ${success} task(s)`];
      if (elevated > 0) parts.push(`used elevation for ${elevated}`);
      if (stopped > 0) parts.push(`stopped ${stopped} before delete`);
      message = parts.join(' · ');
      const fails = normalized.failures;
      if (Array.isArray(fails) && fails.length) {
        try {
          pushLog('WARN', `${taskAction} failures (${fails.length})`, 'Optimize');
          for (const f of fails) {
            const stderr = (f?.stderr || '').trim();
            if (stderr) {
              pushLog('WARN', `Task error: ${stderr}`, 'Optimize');
            }
          }
        } catch {
          // ignore
        }
      }
      await reloadTasks();
      selectedTasks = new SvelteSet();
    } catch (e) {
      const msg = `Action failed: ${String(e)}`;
      pushLog('ERROR', msg, 'Optimize');
      message = msg;
    } finally {
      taskActionLoading = false;
    }
  }

  async function loadTasks() {
    if (loadingTasks || tasksLoaded) return;
    loadingTasks = true;
    await tick();
    try {
      const res = await listScheduledTasks();
      tasks = Array.isArray(res) ? res : [];
      if (selectedTasks.size > 0) {
        const keep = new Set(tasks.map((t) => t.name));
        selectedTasks = new SvelteSet(Array.from(selectedTasks).filter((n) => keep.has(n)));
      }
      tasksLoaded = true;
      tasksVisible = Math.min(tasks.length, 50);
      queueEnrichVisibleTasks(12);
    } catch (e) {
      pushLog('ERROR', `Failed to load scheduled tasks: ${String(e)}`, 'Optimize');
    } finally {
      loadingTasks = false;
    }
  }
  async function reloadTasks() {
    tasksLoaded = false;
    await loadTasks();
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
      .filter(([, v]) => !!v?.suspicious)
      .map(([id, v]) => {
        const [hive, key, name] = id.split('|');
        const reason = v?.suspiciousReason || 'Reappeared after cleanup';
        const lastStrategy = v?.lastStrategy || '';
        return { id, hive, key, name, reason, lastStrategy };
      })
  );

  const allRegistrySelected = $derived(
    filteredRegistryItems.length > 0 &&
      filteredRegistryItems.every((it) => selectedReg.has(regId(it)))
  );

  function toggleAllRegistry() {
    if (filteredRegistryItems.length === 0) return;
    selectedReg = allRegistrySelected
      ? new SvelteSet()
      : new SvelteSet(filteredRegistryItems.map(regId));
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

  function isMicrosoftTask(t: ScheduledTask): boolean {
    const author = (t.author || '').toLowerCase();
    const name = (t.name || '').toLowerCase();
    return author.includes('microsoft') || name.startsWith('\\microsoft\\windows');
  }

  const filteredTasks = $derived(
    tasks.filter((t) => {
      const q = tasksQueryDeb.trim().toLowerCase();
      if (taskFilter === 'sus' && !t.is_sus) return false;
      if (taskFilter === 'sus' && !includeMicrosoftInSus && isMicrosoftTask(t)) return false;
      const status = (t.status || '').toLowerCase();
      if (!includeDisabled && status.includes('disable')) return false;
      const next = (t.next_run_time || '').trim().toLowerCase();
      const hasNext = next !== '' && next !== 'n/a' && next !== '—' && next !== '-';
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
        if (!v || v.toLowerCase() === 'n/a' || v === '—' || v === '-')
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
    selectedTasks = new SvelteSet(selectedTasks);
  }

  function toggleAllTasks() {
    const slice = sortedTasks.slice(tasksStart, tasksVisible);
    if (slice.length === 0) return;
    if (allTasksSelected) selectedTasks = new SvelteSet();
    else selectedTasks = new SvelteSet(slice.map((t) => t.name));
  }

  function isLikelyProtected(name: string): boolean {
    const t = tasks.find((x) => x.name === name);
    if (!t) return false;
    const parts = splitTaskName(t.name);
    return !!t.is_sus || (parts.folder || '').startsWith('\\Microsoft\\Windows');
  }

  function requestRunActionDirectly(action: TaskAction) {
    const names = Array.from(selectedTasks);
    if (names.length === 0) {
      message = 'Select at least one task.';
      return;
    }
    taskAction = action;
    pendingAction = action;
    pendingNames = names;
    showTaskConfirm = true;
  }


  function confirmRunAction() {
    showTaskConfirm = false;
    taskAction = pendingAction;
    selectedTasks = new SvelteSet(pendingNames);
    void runSelectedTasks();
  }

  function switchToDisableAndRun() {
    pendingAction = 'disable';
    confirmRunAction();
  }

  type NetworkHistoryEntry = {
    id: string;
    label: string;
    result: string;
    success: boolean;
    timestamp: number;
  };

  const NETWORK_SUMMARY_TTL = 5 * 60 * 1000;
  let activeNetworkPreset = $state<NetworkPresetId>(NETWORK_PRESETS[0].id);
  let networkSummary = $state<NetworkSummary | null>(null);
  let networkInfoLoading = $state(false);
  let networkSummaryFetchedAt = $state<number | null>(null);
  let networkHistory = $state<NetworkHistoryEntry[]>([]);
  let pingTarget = $state('1.1.1.1');
  let tracerouteTarget = $state('1.1.1.1');
  let dnsLookupTarget = $state('example.com');
  let networkTestLoading = $state(false);
  let networkActionLoading = $state(false);
  let networkTestResult = $state('');
  let networkTestLabel = $state('');

  function addNetworkHistory(label: string, result: string, success: boolean) {
    const safe = result
      .split('\n')
      .filter((line) => line.trim().length > 0)
      .slice(0, 6)
      .join('\n')
      .trim();
    const entry: NetworkHistoryEntry = {
      id: `${label}-${Date.now()}`,
      label,
      result: safe || 'No output was returned.',
      success,
      timestamp: Date.now(),
    };
    networkHistory = [entry, ...networkHistory].slice(0, 6);
  }

  async function refreshNetworkStatus() {
    if (networkInfoLoading) return;
    networkInfoLoading = true;
    await tick();
    try {
      networkSummary = await getNetworkSummary();
      networkSummaryFetchedAt = Date.now();
    } catch (error: unknown) {
      pushLog('ERROR', `network summary failed: ${String(error)}`, 'Optimize');
      const text = error instanceof Error ? error.message : 'Failed to refresh network summary';
      toast.error(text);
      networkSummary = null;
    } finally {
      networkInfoLoading = false;
    }
  }

  async function runNetworkTest(label: string, action: () => Promise<string>) {
    if (networkTestLoading || networkActionLoading) return;
    networkTestLoading = true;
    networkTestLabel = label;
    networkTestResult = '';
    try {
      const output = (await action()).trim();
      const result = output || `${label} completed`;
      networkTestResult = result;
      addNetworkHistory(label, result, true);
    } catch (error: unknown) {
      const text = error instanceof Error ? error.message : `${label} test failed`;
      networkTestResult = text;
      addNetworkHistory(label, text, false);
      toast.error(text);
    } finally {
      networkTestLoading = false;
    }
  }

  async function runPingTest() {
    await runNetworkTest('Ping', () => runPingCommand(pingTarget, 4));
  }

  async function runTracerouteTest() {
    await runNetworkTest('Traceroute', () => runTracerouteCommand(tracerouteTarget));
  }

  async function runDnsLookupTest() {
    await runNetworkTest('DNS Lookup', () => runDnsLookupCommand(dnsLookupTarget));
  }

  const selectedPreset = $derived(NETWORK_PRESETS.find((item) => item.id === activeNetworkPreset));

  async function applyNetworkPreset(id: NetworkPresetId) {
    if (networkTestLoading || networkActionLoading) return;
    const preset = NETWORK_PRESETS.find((item) => item.id === id);
    if (!preset) return;
    activeNetworkPreset = id;
    for (const action of preset.actions) {
      let ok = true;
      if (action === 'flush_dns') ok = await flushDns();
      else if (action === 'reset_winsock') ok = await resetWinsock();
      else if (action === 'renew_ip') ok = await renewIp();
      if (!ok) break;
    }
    await refreshNetworkStatus();
  }

  async function runNetworkAction(label: string, action: () => Promise<string>): Promise<boolean> {
    if (networkActionLoading || networkTestLoading) return false;
    networkActionLoading = true;
    try {
      const output = await action();
      const message = (output ?? '').trim();
      const display = message || `${label} completed`;
      toast.success(display);
      addNetworkHistory(label, display, true);
      await refreshNetworkStatus();
      return true;
    } catch (error: unknown) {
      pushLog('ERROR', `Optimize error: ${String(error)}`, 'Optimize');
      const text =
        error instanceof Error
          ? error.message
          : typeof error === 'string'
            ? error
            : `${label} failed`;
      toast.error(text);
      addNetworkHistory(label, text, false);
      return false;
    } finally {
      networkActionLoading = false;
    }
  }

  async function flushDns() {
    return runNetworkAction('Flush DNS', flushDnsCommand);
  }

  async function resetWinsock() {
    return runNetworkAction('Reset Winsock', resetWinsockCommand);
  }

  async function renewIp() {
    return runNetworkAction('Renew IP', renewIpCommand);
  }

  onMount(() => {
    void initBootCheck();
    const io = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          if (registrySentinel && entry.target === registrySentinel) loadRegistryItems();
          if (tasksSentinel && entry.target === tasksSentinel) loadTasks();
        }
      },
      { root: null, rootMargin: '0px', threshold: 0.1 }
    );
    if (registrySentinel) io.observe(registrySentinel);
    if (tasksSentinel) io.observe(tasksSentinel);
    try {
      _registryPollTimer = setInterval(pollRegistryOnce, 6000) as unknown as number;
    } catch {
      /* noop */
    }
    let networkPrefetchHandle: number | null = null;
    const scheduleNetworkRefresh = () => {
      if (!networkSummary && !networkInfoLoading) {
        void refreshNetworkStatus();
      }
    };
    if (typeof requestIdleCallback === 'function') {
      networkPrefetchHandle = requestIdleCallback(scheduleNetworkRefresh);
    } else {
      networkPrefetchHandle = window.setTimeout(scheduleNetworkRefresh, 250);
    }

    return () => {
      io.disconnect();
      try {
        if (_registryPollTimer) clearInterval(_registryPollTimer as unknown as number);
      } catch {
        /* noop */
      }
      _registryPollTimer = null;
      if (networkPrefetchHandle !== null) {
        if (typeof cancelIdleCallback === 'function') {
          cancelIdleCallback(networkPrefetchHandle);
        } else {
          window.clearTimeout(networkPrefetchHandle);
        }
      }
    };
  });

  $effect(() => {
    if (activeOptimizeTab === 'registry') {
      void loadRegistryItems();
    } else if (activeOptimizeTab === 'tasks') {
      void loadTasks();
    } else if (activeOptimizeTab === 'network' && !networkInfoLoading) {
      const elapsed =
        networkSummaryFetchedAt === null ? Infinity : Date.now() - networkSummaryFetchedAt;
      if (!networkSummary || elapsed > NETWORK_SUMMARY_TTL) {
        void refreshNetworkStatus();
      }
    }
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
        >Manage startup items, registry Run keys, and scheduled tasks.</CardDescription
      >
    </CardHeader>
  </Card>

  <Tabs
    value={activeOptimizeTab}
    onValueChange={(val: string) => {
      activeOptimizeTab = val as 'tweaks' | 'startup' | 'registry' | 'tasks' | 'network';
    }}
    class="space-y-4"
  >
    <TabsList class="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-5 h-auto *:cursor-pointer">
      <TabsTrigger value="tweaks">Tweaks</TabsTrigger>
      <TabsTrigger value="startup">Startup items</TabsTrigger>
      <TabsTrigger value="registry">Registry</TabsTrigger>
      <TabsTrigger value="tasks">Scheduled tasks</TabsTrigger>
      <TabsTrigger value="network">Network</TabsTrigger>
    </TabsList>

    <TabsContent value="tweaks" class="space-y-4">
      <TweaksPanel />
    </TabsContent>

    <TabsContent value="startup" class="space-y-4">
      <StartupPanel
        active={activeOptimizeTab === 'startup'}
        on:message={(event) => (message = event.detail)}
      />
    </TabsContent>

    <TabsContent value="registry" class="space-y-4">
      <AlertDialog open={showRegistryConfirm} onOpenChange={(v) => (showRegistryConfirm = !!v)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove Registry Startup</AlertDialogTitle>
            <AlertDialogDescription>
              This removes selected Run/RunOnce entries. The app will attempt forced removal if
              needed (may prompt for admin).
            </AlertDialogDescription>
          </AlertDialogHeader>
          {#if regPreset === 'full'}
            <Alert class="mb-2">
              <AlertDescription>
                Full cleanup runs an aggressive cleanup: forced removal, IFEO blocking, and
                delete-on-reboot, purging StartupApproved and removing related Scheduled Tasks and
                WMI subscriptions You may get UAC prompts. Save your work and close unnecessary apps
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
                Basic: attempts normal removal without elevation. Fastest path when nothing is
                locking the entry. If it fails, try Force or Aggressive.
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
                {#each pendingRegistry.slice(0, 10) as it (regId(it))}
                  <li class="truncate">
                    {it.name}
                    <span class="text-muted-foreground"> - {it.hive}\{it.key}</span>
                  </li>
                {/each}
                {#if pendingRegistry.length > 10}
                  <li class="text-muted-foreground">
                    . and {pendingRegistry.length - 10} more
                  </li>
                {/if}
              </ul>
            {/if}
          </div>
          <AlertDialogFooter class="pr-2">
            <AlertDialogCancel onclick={() => (showRegistryConfirm = false)}
              >Cancel</AlertDialogCancel
            >
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
              {#each suspectEntries as s (s.id)}
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
            <div class="text-sm text-muted-foreground">Running system check…</div>
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
                    {#each postDiag.removedRegistry.stillPresent.slice(0, 6) as r (r)}
                      <li class="break-all">{r}</li>
                    {/each}
                    {#if postDiag.removedRegistry.stillPresent.length > 6}
                      <li>…and {postDiag.removedRegistry.stillPresent.length - 6} more</li>
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
                    {#each postDiag.taskMatches.remaining.slice(0, 8) as t (t)}
                      <li class="break-all">{t}</li>
                    {/each}
                    {#if postDiag.taskMatches.remaining.length > 8}
                      <li>…and {postDiag.taskMatches.remaining.length - 8} more</li>
                    {/if}
                  </ul>
                {/if}
              </div>

              <div>
                <p class="font-medium">Services</p>
                {#if postDiag.serviceMatches.running.length === 0 && postDiag.serviceMatches.disabled.length === 0}
                  <p class="text-emerald-600 dark:text-emerald-400">
                    No related services were found.
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
                      Close or uninstall the processes still running:
                      {postDiag.runningImages.running.join(', ')}.
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
                    await restartSystem();
                  } catch (e) {
                    pushLog('ERROR', `Registry history check failed: ${String(e)}`, 'Optimize');
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
            {#if Object.values(registryHistory).some((r) => r?.suspicious)}
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
          {#if rebootDetected}
            <Badge variant="secondary" class="text-[10px] uppercase tracking-wide">
              Reboot detected
            </Badge>
          {/if}
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
                {#each Array.from({ length: 8 }) as _, i (i)}
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
                        {#if registryHistory[regId(it)]?.suspicious}
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
                          onclick={() => copyText(it.command)}
                        >
                          <CopyIcon class="size-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Open in Registry"
                          aria-label="Open in Registry"
                          onclick={() => {
                            void openRegistryKey(it.hive, it.key);
                          }}
                        >
                          <FolderOpen class="size-4" />
                        </Button>
                      </div>
                    </div>
                  </li>
                {/each}
                {#if registryVisible < filteredRegistryItems.length}
                  <li class="px-2 py-2"><Skeleton class="h-5 w-full" aria-hidden="true" /></li>
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
                {#each Array.from({ length: 6 }) as _, i (i)}
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
    </TabsContent>

    <TabsContent value="tasks" class="space-y-4">
      <Card class="gap-4 py-4">
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <ListChecks class="size-5" /> Scheduled Tasks
          </CardTitle>
          <CardDescription>
            Inspect Task Scheduler entries and highlight suspicious ones.
          </CardDescription>
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
                    {taskFilter === 'sus' &&
                    !includeDisabled &&
                    !includeNoNext &&
                    !includeMicrosoftInSus
                      ? 'Suspicious only'
                      : taskFilter === 'sus' &&
                          (includeDisabled || includeNoNext || includeMicrosoftInSus)
                        ? 'Suspicious + extras'
                        : taskFilter === 'all' &&
                            (includeDisabled || includeNoNext || includeMicrosoftInSus)
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
                variant="outline"
                onclick={reloadTasks}
                title="Refresh tasks"
                aria-label="Refresh tasks"
                disabled={loadingTasks || taskActionLoading}
                class="px-2.5"
              >
                <RefreshCw class="size-4 {loadingTasks ? 'animate-spin' : ''}" />
              </Button>
              <DropdownMenu>
                <DropdownMenuTrigger>
                  {#snippet child({ props })}
                    <Button variant="outline" size="sm" class="flex items-center gap-2" {...props}>
                      <ListChecks class="size-4" />
                      <span>Actions</span>
                      {#if selectedTasks.size > 0}
                        <Badge variant="secondary" class="ml-1 px-1.5 py-0.5 text-[10px]">
                          {selectedTasks.size}
                        </Badge>
                      {/if}
                    </Button>
                  {/snippet}
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" class="w-48">
                  <DropdownMenuLabel class="">Task Actions</DropdownMenuLabel>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    disabled={selectedTasks.size === 0 || taskActionLoading}
                    onclick={() => requestRunActionDirectly('run')}
                  >
                    <Play class="mr-2 size-4 text-muted-foreground" /> Run now
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    disabled={selectedTasks.size === 0 || taskActionLoading}
                    onclick={() => requestRunActionDirectly('end')}
                  >
                    <CircleX class="mr-2 size-4 text-muted-foreground" /> End
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    disabled={selectedTasks.size === 0 || taskActionLoading}
                    onclick={() => requestRunActionDirectly('enable')}
                  >
                    <Power class="mr-2 size-4 text-muted-foreground" /> Enable
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    disabled={selectedTasks.size === 0 || taskActionLoading}
                    onclick={() => requestRunActionDirectly('disable')}
                  >
                    <PowerOff class="mr-2 size-4 text-muted-foreground" /> Disable
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    disabled={selectedTasks.size === 0 || taskActionLoading}
                    class="text-destructive focus:text-destructive"
                    onclick={() => requestRunActionDirectly('delete')}
                  >
                    <Trash2 class="mr-2 size-4 text-destructive" /> Delete
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    disabled={sortedTasks.length === 0}
                    onclick={toggleAllTasks}
                  >
                    {allTasksSelected ? 'Deselect all' : 'Select all'}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
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
                {#each Array.from({ length: 10 }) as _, i (i)}
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
                            {#if t.is_sus}
                              <Badge
                                variant="outline"
                                class="text-[10px] border-red-500/30 text-red-600 bg-red-500/10"
                                >SUS ({t.score})</Badge
                              >
                            {:else if t.score !== undefined}
                              <Badge variant="outline" class="text-[10px] opacity-60"
                                >{t.score}</Badge
                              >
                            {/if}
                          </div>
                          <div
                            class="mt-1 text-xs text-muted-foreground font-mono truncate max-w-[80ch]"
                          >
                            {parts.folder}
                          </div>
                        </div>
                      </div>
                      <div class="shrink-0 text-right">
                        <div class="text-xs text-muted-foreground">{t.status || '-'}</div>
                        <div class="text-[10px] text-muted-foreground">
                          Next: {t.next_run_time || '-'}
                        </div>
                      </div>
                    </div>
                  </li>
                {/each}
                {#if tasksVisible < sortedTasks.length}
                  <li class="px-2 py-2"><Skeleton class="h-5 w-full" aria-hidden="true" /></li>
                {/if}
                {#if sortedTasks.length === 0 && tasksLoaded}
                  <li class="px-3 py-8 text-center text-xs text-muted-foreground">
                    No tasks match.
                  </li>
                {/if}
              </ul>
            </div>
          {/if}
        </CardContent>
      </Card>

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
                {#each pendingNames.slice(0, 8) as nm (nm)}
                  {@const p = splitTaskName(nm)}
                  <li class="truncate">
                    {p.base}
                    <span class="text-muted-foreground">{p.folder}</span>{#if isLikelyProtected(nm)}
                      <span class="ml-1 text-[10px] text-destructive">protected</span>{/if}
                  </li>
                {/each}
                {#if pendingNames.length > 8}
                  <li class="text-muted-foreground">. and {pendingNames.length - 8} more</li>
                {/if}
              </ul>
            {/if}
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel onclick={() => (showTaskConfirm = false)}>Cancel</AlertDialogCancel>
            {#if pendingAction === 'delete'}
              <AlertDialogAction onclick={switchToDisableAndRun}
                >Switch to Disable</AlertDialogAction
              >
            {/if}
            <AlertDialogAction onclick={confirmRunAction}>Continue</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </TabsContent>

    <TabsContent value="network" class="space-y-4">
      <div class="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader class="flex flex-row items-center justify-between gap-4">
            <div class="space-y-1">
              <CardTitle class="flex items-center gap-2">
                <NetworkIcon class="size-5 text-primary" /> Diagnostics
              </CardTitle>
              <CardDescription>Snapshot of adapters, IP ranges and DNS paths.</CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              disabled={networkInfoLoading}
              onclick={refreshNetworkStatus}
              class="flex items-center gap-1.5"
            >
              <RefreshCw class="size-4 {networkInfoLoading ? 'animate-spin' : ''}" />
              <span>Refresh</span>
            </Button>
          </CardHeader>
          <CardContent class="space-y-4">
            {#if networkInfoLoading}
              <div class="grid gap-3 sm:grid-cols-2">
                <Skeleton class="h-[68px] w-full rounded-lg" aria-hidden="true" />
                <Skeleton class="h-[68px] w-full rounded-lg" aria-hidden="true" />
                <Skeleton class="h-[68px] w-full rounded-lg" aria-hidden="true" />
                <Skeleton class="h-[68px] w-full rounded-lg" aria-hidden="true" />
              </div>
            {:else if !networkSummary}
              <p class="text-xs text-muted-foreground text-center py-6">Unable to read network information.</p>
            {:else}
              <div class="space-y-3">
                <div class="grid gap-3 sm:grid-cols-2">
                  <div class="rounded-lg border bg-muted/10 p-3 shadow-xs">
                    <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Primary Adapter</p>
                    <p class="mt-1 text-sm font-medium truncate" title={networkSummary.primaryAdapter}>{networkSummary.primaryAdapter ?? '—'}</p>
                  </div>
                  <div class="rounded-lg border bg-muted/10 p-3 shadow-xs">
                    <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">IPv4 Address</p>
                    <p class="mt-1 text-sm font-mono font-medium">{networkSummary.ipv4 ?? '—'}</p>
                  </div>
                </div>

                <div class="grid gap-3 sm:grid-cols-2">
                  <div class="rounded-lg border bg-muted/10 p-3 shadow-xs">
                    <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">DNS Servers</p>
                    {#if networkSummary.dnsServers.length}
                      <div class="mt-1 space-y-1 font-mono text-xs">
                        {#each networkSummary.dnsServers as dns (dns)}
                          <div class="text-foreground">{dns}</div>
                        {/each}
                      </div>
                    {:else}
                      <p class="mt-1 text-xs text-muted-foreground">—</p>
                    {/if}
                  </div>
                  <div class="rounded-lg border bg-muted/10 p-3 shadow-xs">
                    <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Gateway / Router</p>
                    {#if networkSummary.gateways.length}
                      <div class="mt-1 space-y-1 font-mono text-xs">
                        {#each networkSummary.gateways as gateway (gateway)}
                          <div class="text-foreground">{gateway}</div>
                        {/each}
                      </div>
                    {:else}
                      <p class="mt-1 text-xs text-muted-foreground">—</p>
                    {/if}
                  </div>
                </div>

                {#if networkSummary.ipv6}
                  <div class="rounded-lg border bg-muted/10 p-3 shadow-xs">
                    <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">IPv6 Address</p>
                    <p class="mt-1 text-xs font-mono break-all truncate" title={networkSummary.ipv6}>{networkSummary.ipv6}</p>
                  </div>
                {/if}
              </div>
            {/if}
          </CardContent>
        </Card>
        <Card class="space-y-3">
          <CardHeader>
            <div class="flex items-center gap-2">
              <Badge variant="secondary" class="px-3 py-1 text-[10px] uppercase">Adapters</Badge>
              <CardTitle class="m-0">Interfaces</CardTitle>
            </div>
            <CardDescription>Reported adapter status and media information.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-3">
            {#if networkInfoLoading}
              <div class="space-y-2">
                <Skeleton class="h-5 w-full" aria-hidden="true" />
                <Skeleton class="h-5 w-full" aria-hidden="true" />
              </div>
            {:else if !networkSummary || networkSummary.adapters.length === 0}
              <p class="text-xs text-muted-foreground">No adapters detected.</p>
            {:else}
              <div class="space-y-2">
                {#each networkSummary.adapters as adapter (adapter.name)}
                  <div class="rounded-md border border-border/60 bg-muted/10 p-3">
                    <div class="flex items-center justify-between gap-3">
                      <span class="font-semibold">{adapter.name}</span>
                      {#if adapter.status}
                        <Badge
                          variant={adapter.status?.toLowerCase().includes('up')
                            ? 'secondary'
                            : 'outline'}
                          class="text-[10px]">{adapter.status}</Badge
                        >
                      {/if}
                    </div>
                    <div class="mt-1 grid gap-2 text-[11px] text-muted-foreground sm:grid-cols-2">
                      {#if adapter.linkSpeed}
                        <div>
                          Speed:
                          <span class="font-mono text-foreground">{adapter.linkSpeed}</span>
                        </div>
                      {/if}
                      {#if adapter.media}
                        <div>Media: <span class="text-foreground">{adapter.media}</span></div>
                      {/if}
                      {#if adapter.mac}
                        <div>MAC: <span class="font-mono text-foreground">{adapter.mac}</span></div>
                      {/if}
                      {#if adapter.linkState}
                        <div>State: <span class="text-foreground">{adapter.linkState}</span></div>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </CardContent>
        </Card>
      </div>
      <Card class="space-y-4">
        <CardHeader class="items-start gap-2">
          <CardTitle class="flex items-center gap-2">
            <NetworkIcon class="size-5" /> Network tweaks
          </CardTitle>
          <CardDescription>Quick network tune-ups and presets.</CardDescription>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="flex flex-wrap gap-2">
            <Button
              variant="ghost"
              size="sm"
              onclick={flushDns}
              title="Flush DNS Cache"
              aria-label="Flush DNS Cache"
              disabled={networkActionLoading || networkTestLoading}
            >
              <RefreshCcw class="size-4" /><span class="ml-1 hidden sm:inline">Flush DNS</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onclick={resetWinsock}
              title="Reset Winsock"
              aria-label="Reset Winsock"
              disabled={networkActionLoading || networkTestLoading}
            >
              <RotateCcw class="size-4" /><span class="ml-1 hidden sm:inline">Reset Winsock</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onclick={renewIp}
              title="Renew IP"
              aria-label="Renew IP"
              disabled={networkActionLoading || networkTestLoading}
            >
              <RefreshCw class="size-4" /><span class="ml-1 hidden sm:inline">Renew IP</span>
            </Button>
          </div>
          <p class="text-xs text-muted-foreground">
            Actions may briefly interrupt connectivity. Some changes can require a reboot.
          </p>
          <div class="space-y-2 border-t border-border/60 pt-3">
            <p class="text-xs uppercase text-muted-foreground">Automation presets</p>
            <div class="flex flex-wrap gap-2">
              {#each NETWORK_PRESETS as preset (preset.id)}
                <Button
                  size="sm"
                  variant={activeNetworkPreset === preset.id ? 'secondary' : 'outline'}
                  onclick={() => applyNetworkPreset(preset.id)}
                  disabled={networkTestLoading || networkActionLoading}
                >
                  {preset.label}
                </Button>
              {/each}
            </div>
            {#if selectedPreset}
              <p class="text-xs text-muted-foreground">{selectedPreset.description}</p>
            {/if}
          </div>
        </CardContent>
      </Card>
      <Card class="space-y-3">
        <CardHeader class="items-center gap-2">
          <div class="flex items-center gap-2">
            <SearchIcon class="size-5" />
            <CardTitle>Quick tests</CardTitle>
          </div>
          <CardDescription
            >Ping, traceroute, and DNS lookups without leaving the app.</CardDescription
          >
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="grid gap-3 md:grid-cols-3">
            <div class="space-y-1">
              <p class="text-xs text-muted-foreground">Ping target</p>
              <div class="flex items-center gap-2">
                <Input class="flex-1" placeholder="1.1.1.1" bind:value={pingTarget} />
                <Button
                  size="sm"
                  variant="outline"
                  onclick={runPingTest}
                  disabled={networkTestLoading || networkActionLoading}
                >
                  Ping
                </Button>
              </div>
            </div>
            <div class="space-y-1">
              <p class="text-xs text-muted-foreground">Traceroute</p>
              <div class="flex items-center gap-2">
                <Input class="flex-1" placeholder="example.com" bind:value={tracerouteTarget} />
                <Button
                  size="sm"
                  variant="outline"
                  onclick={runTracerouteTest}
                  disabled={networkTestLoading || networkActionLoading}
                >
                  Trace
                </Button>
              </div>
            </div>
            <div class="space-y-1">
              <p class="text-xs text-muted-foreground">DNS lookup</p>
              <div class="flex items-center gap-2">
                <Input class="flex-1" placeholder="example.com" bind:value={dnsLookupTarget} />
                <Button
                  size="sm"
                  variant="outline"
                  onclick={runDnsLookupTest}
                  disabled={networkTestLoading || networkActionLoading}
                >
                  Lookup
                </Button>
              </div>
            </div>
          </div>
          <div
            class="rounded-md border border-border/60 bg-muted/10 p-3 text-xs font-mono whitespace-pre-wrap wrap-break-word max-h-40 overflow-auto"
          >
            {#if networkTestLoading}
              <p class="text-muted-foreground">Running {networkTestLabel}...</p>
            {:else if networkTestResult}
              {networkTestResult}
            {:else}
              Results will appear here.
            {/if}
          </div>
        </CardContent>
      </Card>
      <Card class="space-y-2">
        <CardHeader class="items-center gap-2">
          <div class="flex items-center gap-2">
            <ListChecks class="size-5" />
            <CardTitle>Network history</CardTitle>
          </div>
          <CardDescription>Records of recent tweaks and connectivity tests.</CardDescription>
          <Button
            size="sm"
            variant="ghost"
            class="w-30"
            onclick={() => (networkHistory = [])}
            disabled={networkHistory.length === 0}
          >
            Clear
          </Button>
        </CardHeader>
        <CardContent class="space-y-3">
          {#if networkHistory.length === 0}
            <p class="text-xs text-muted-foreground">No recent actions recorded.</p>
          {:else}
            <ul class="space-y-2">
              {#each networkHistory as entry (entry.id)}
                <li class="rounded-md border border-border/60 bg-muted/10 p-3">
                  <div class="flex items-center justify-between text-[10px] text-muted-foreground">
                    <div class="flex items-center gap-1">
                      <Badge
                        variant={entry.success ? 'secondary' : 'destructive'}
                        class="text-[10px]"
                      >
                        {entry.success ? 'OK' : 'Fail'}
                      </Badge>
                      <span class="font-medium text-[11px] text-foreground">{entry.label}</span>
                    </div>
                    <span>{new Date(entry.timestamp).toLocaleTimeString()}</span>
                  </div>
                  <pre
                    class="mt-1 text-[11px] font-mono text-muted-foreground whitespace-pre-wrap wrap-break-word max-h-28 overflow-auto">{entry.result}</pre>
                </li>
              {/each}
            </ul>
          {/if}
        </CardContent>
      </Card>
    </TabsContent>
  </Tabs>

  {#if message}
    <Alert><AlertDescription>{message}</AlertDescription></Alert>
  {/if}
</div>
