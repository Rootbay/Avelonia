<script lang="ts">
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';
  import { initDownloadListener, disposeDownloadListener } from '$lib/downloadManager';
  import { downloads } from '$lib/downloads';
  import type { Component, Snippet } from 'svelte';
  import type { IconProps } from '@lucide/svelte';
  import {
    LayoutDashboard,
    Gauge,
    Download,
    Eraser,
    ChevronLeft,
    ChevronRight,
    Sun as SunIcon,
    Moon as MoonIcon,
    Settings as SettingsIcon,
  } from '@lucide/svelte';
  // old settings icons removed
  import { ModeWatcher, toggleMode } from 'mode-watcher';

  import {
    Sidebar,
    SidebarContent,
    SidebarGroup,
    SidebarGroupContent,
    SidebarMenu,
    SidebarMenuItem,
    SidebarMenuButton,
    SidebarProvider,
  } from '$lib/components/ui/sidebar';
  import { Tooltip, TooltipTrigger, TooltipContent } from '$lib/components/ui/tooltip';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Toaster } from '$lib/components/ui/sonner';
  import { toast } from '$lib/components/ui/sonner';
  import { pushLog, clearLogs } from '$lib/logStore';
  import { cn } from '$lib/utils.js';
  import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogDescription,
    DialogFooter,
    DialogClose,
    DialogTrigger,
  } from '$lib/components/ui/dialog';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
  import { settings, updateDownloaderSettings } from '$lib/settings';
  import { startDownloadIntegrityWatch, stopDownloadIntegrityWatch } from '$lib/downloadIntegrity';
  import { startInstallPresenceWatch, stopInstallPresenceWatch } from '$lib/downloadManager';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { openUrl as openExternal } from '@tauri-apps/plugin-opener';
  import { vtVerdicts, setVerdictFromReport, setVerdict, reasonFor } from '$lib/vtVerdicts';
  import { MoreHorizontal } from '@lucide/svelte';
  import { vtScan, beginScan, endScan, pushReport as pushScanReport } from '$lib/scanStatus';
  import { cleanerScan, beginCleanerScan, incCleanerFound, setCleanerMessage, endCleanerScan } from '$lib/cleanerScan';

  import '../app.css';
  const scan = $derived($vtScan);

  onMount(() => {
    initDownloadListener();
    startDownloadIntegrityWatch(20000);
    startInstallPresenceWatch(20000);
    // Initialize VirusTotal cache and optionally run a background scan if an API key is set.
    (async () => {
      try {
        const loaded = (await invoke('vt_load_cache')) as number;
        // log suppressed: VT cache loaded
        const status = (await invoke('vt_get_status')) as { key_set?: boolean };
        if (status && (status as any).key_set) {
          // Stagger a bit to avoid contention with first paint
          // log suppressed: VT key detected. Scheduling background scan (limit 50).
          try { await invoke('vt_auto_maybe_scan'); } catch {}
          const iv = setInterval(() => { void (async () => { try { await invoke('vt_auto_maybe_scan'); } catch {} })(); }, 60_000);
          unlistenFns.push(() => { try { clearInterval(iv as unknown as number); } catch {} });
        } else {
          pushLog('INFO', 'VT key not set. Reputation scans disabled.', 'Optimize');
        }
      } catch {}
    })();

        // Auto-scan notifications (deduplicated)
    const unAutoStart = listen('vt-autoscan-start', (ev) => {
      toast.message('VirusTotal scan started (auto)', {
        action: { label: 'Open details', onClick: () => { try { scanDialogOpen = true; } catch {} } }
      });
      pushLog('INFO', 'VT scan starting (auto): ' + (((ev as any)?.payload as any)?.reason || 'auto'), 'Optimize');
    });
    unlistenFns.push(() => { unAutoStart.then((f)=>f()).catch(()=>{}); });
    const unAutoDone = listen('vt-autoscan-done', (ev) => {
      const p = ev.payload as any;
      toast.success('VirusTotal scan completed (auto)', {
        action: { label: 'Open details', onClick: () => { try { scanDialogOpen = true; } catch {} } }
      });
      pushLog('SUCCESS', 'VT scan finished (auto): startup ' + (Number((p as any)?.startup) || 0) + ', registry ' + (Number((p as any)?.registry) || 0) + '.', 'Optimize');
    });
    unlistenFns.push(() => { unAutoDone.then((f)=>f()).catch(()=>{}); });
    // Security alert listener (VirusTotal findings)
    const un = listen('vt-alert', (ev) => {
      const p = ev.payload as { subject?: string; verdict?: string; positives?: number; permalink?: string; source?: string };
      const name = p?.subject || 'Startup item';
      const src = (p?.source || 'startup').toString();
      const sev = (p?.verdict || '').toString().toUpperCase();
      const msg = `${sev === 'MALICIOUS' ? 'Malicious' : 'Suspicious'} ${src === 'registry' ? 'registry item' : 'startup item'}: ${name}`;
      toast.error(msg, {
        action: p?.permalink
          ? {
              label: 'Open VirusTotal',
              onClick: async () => {
                try { await openExternal(p.permalink as string); } catch {}
              },
            }
          : undefined,
      });
      // Also log in system logs with severity mapping
      const lvl = sev === 'MALICIOUS' ? 'ERROR' : 'WARN';
      const pos = typeof p?.positives === 'number' ? ` (${p?.positives} vendors)` : '';
      pushLog(lvl as any, `VT detection: ${name}${pos}. ${p?.permalink ? 'Report available.' : ''}`, 'Optimize');
      // Update verdict store so UI can show a tag
      setVerdict(name, 'Sus');
      // Apply badges immediately for visible Optimize lists
      setTimeout(() => { try { applyVtBadges(); } catch {} }, 0);
    });
    // No need to await; unlisten automatically on destroy
    unlistenFns.push(() => { un.then((f)=>f()).catch(()=>{}); });
    // General report listener (for Safe / Clean verdicts too)
    const unReport = listen('vt-report', (ev) => {
      const rep = ev.payload as any;
      try {
        setVerdictFromReport(rep); try { pushScanReport(rep); } catch {}
        const v = String(rep?.verdict || '').toUpperCase();
        const pos = typeof rep?.positives === 'number' ? ` (${rep?.positives} vendors)` : '';
        pushLog('INFO', `VT report: ${rep?.subject ?? 'item'} -> ${v}${pos}`, 'Optimize');
        setTimeout(() => { try { applyVtBadges(); } catch {} }, 0);
      } catch {}
    });
    unlistenFns.push(() => { unReport.then((f)=>f()).catch(()=>{}); });

    // Cleaner temp scan: global live notification and counts
    let cleanerToastShown = false;
    const cleanerToastId = 'cleaner-scan';
    const unScanProg = listen('scan_progress', (ev) => {
      try {
        const msg = String((ev as any)?.payload || '');
        setCleanerMessage(msg);
        if (cleanerToastShown) {
          toast.message(`${msg}`, { id: cleanerToastId, duration: Infinity });
        }
      } catch {}
    });
    unlistenFns.push(() => { unScanProg.then((f)=>f()).catch(()=>{}); });
    const unTempBatch = listen('cleaner-temp-batch', (ev) => {
      try {
        const arr = (ev as any)?.payload as unknown as string[];
        const n = Array.isArray(arr) ? arr.length : 0;
        if (n > 0) {
          beginCleanerScan();
          incCleanerFound(n);
          if (!cleanerToastShown) {
            cleanerToastShown = true;
            toast.message('Scanning temporary files…', {
              id: cleanerToastId,
              duration: Infinity,
              action: {
                label: 'Stop',
                onClick: async () => { try { await invoke('cancel_temp_scan'); } catch {} },
              },
            });
          }
          // Update live count display
          cleanerScan.subscribe((s) => {
            if (s.phase === 'running') {
              const label = s.message && s.message.length > 0 ? s.message : 'Scanning temporary files…';
              toast.message(`${label} (${s.found.toLocaleString()} found)`, { id: cleanerToastId, duration: Infinity });
            }
          })();
        }
      } catch {}
    });
    unlistenFns.push(() => { unTempBatch.then((f)=>f()).catch(()=>{}); });
    const unTempDone = listen('cleaner-temp-done', (ev) => {
      try {
        const total = Number(((ev as any)?.payload as any)?.total || 0);
        endCleanerScan(total);
        toast.success(`Temp scan complete • ${Number.isFinite(total) ? total.toLocaleString() : '0'} files`);
        cleanerToastShown = false;
        // Dismiss the persistent toast if the lib supports id updates
        try { (toast as any)?.dismiss?.(cleanerToastId); } catch {}
      } catch {}
    });
    unlistenFns.push(() => { unTempDone.then((f)=>f()).catch(()=>{}); });
  });
  onDestroy(() => {
    disposeDownloadListener();
    stopDownloadIntegrityWatch();
    stopInstallPresenceWatch();
    // cleanup VT listener(s)
    for (const fn of unlistenFns) { try { fn(); } catch {} }
  });

  let { children }: { children?: Snippet } = $props();
  let open = $state(true);
  const collapsed = $derived(!open);
  let scanDialogOpen = $state(false);
  // Expanded item details in VT dialog
  let vtExpanded = $state(new Set<string>());
  function vtKeyOf(it: { subject: string; source?: string }) { return `${(it?.source||'startup')}|${(it?.subject||'').toString().trim().toLowerCase()}`; }
  function toggleVtDetails(it: { subject: string; source?: string }) {
    const k = vtKeyOf(it);
    if (vtExpanded.has(k)) vtExpanded.delete(k); else vtExpanded.add(k);
    vtExpanded = new Set(vtExpanded);
  }
  const vtTotals = $derived(() => {
    const items = (scan?.items ?? []) as Array<{ verdict?: string }>;
    let clean = 0, detected = 0, notScanned = 0;
    for (const it of items) {
      const v = String(it?.verdict || '').toLowerCase();
      if (v === 'clean') clean += 1;
      else if (v === 'malicious' || v === 'suspicious') detected += 1;
      else notScanned += 1;
    }
    return { clean, detected, notScanned, total: items.length };
  });

  type MenuIcon = Component<IconProps>;

  type ButtonSnippetContext = {
    props?: Record<string, unknown> & { class?: string };
  };

  const menuItems: Array<{
    href: string;
    label: string;
    icon: MenuIcon;
    showBadge?: boolean;
  }> = [
    { href: '/dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { href: '/optimize', label: 'Optimize', icon: Gauge },
    { href: '/downloader', label: 'Downloader', icon: Download, showBadge: true },
    { href: '/cleaner', label: 'Cleaner', icon: Eraser },
  ];

  const activeCount = $derived(
    $downloads.filter(
      (d) => d.status === 'downloading' || d.status === 'pending' || d.status === 'queued'
    ).length
  );

  function isActive(path: string) {
    // Deprecated: avoid using store inside a function to preserve reactivity
    // Kept for backward compatibility if used elsewhere
    return $page.url.pathname === path;
  }

  // (old PagePreference type removed as part of the simplified Settings UI)

  let settingsOpen = $state(false);

  // Local unlisten registry for VT alerts
  const unlistenFns: Array<() => void> = [];

  // VirusTotal settings (backend stores persisted key securely in app config folder)
  let vtKey = $state('');
  let vtPersist = $state(true);
  let vtKeySet = $state(false);
  let vtBusy = $state(false);

  // Downloader settings (simple, user-friendly bindings)
  let autoInstall = $state($settings.downloader.autoInstall);
  let installMode = $state($settings.downloader.installMode);
  let elevateInstall = $state($settings.downloader.elevate);
  let fallbackOpen = $state($settings.downloader.fallbackOpen);
  let verifyInstall = $state($settings.downloader.verifyInstall);

  // Keep local toggles in sync with global store
  $effect(() => {
    autoInstall = $settings.downloader.autoInstall;
    installMode = $settings.downloader.installMode;
    elevateInstall = $settings.downloader.elevate;
    fallbackOpen = $settings.downloader.fallbackOpen;
    verifyInstall = $settings.downloader.verifyInstall;
  });
  // Write-through on change
  $effect(() => {
    updateDownloaderSettings({
      autoInstall,
      installMode,
      elevate: elevateInstall,
      fallbackOpen,
      verifyInstall,
    });
  });
  $effect(() => {
    // Whenever dialog opens, refresh VT status
    if (settingsOpen) {
      (async () => { try { const st = (await invoke('vt_get_status')) as { key_set?: boolean }; vtKeySet = !!(st as any)?.key_set; } catch {} })();
    }
  });
  async function saveVtKey() {
    try {
      vtBusy = true;
      await invoke('vt_set_api_key', { key: vtKey || null, persist: vtPersist });
      const st = (await invoke('vt_get_status')) as { key_set?: boolean };
      vtKeySet = !!(st as any)?.key_set;
      toast.success('VirusTotal key saved');
      pushLog('SUCCESS', `VT key saved${vtPersist ? ' (persisted)' : ''}.`, 'Optimize');
    } catch (e) {
      console.error(e);
      toast.error('Failed to save VirusTotal key');
      pushLog('ERROR', `Saving VT key failed: ${String(e)}`, 'Optimize');
    } finally { vtBusy = false; }
  }

  // Best-effort DOM badge injection for Optimize page without touching its markup.
  function applyVtBadges() {
    try {
      if ($page.url.pathname !== '/optimize') return;
      const verdicts = $vtVerdicts;
      if (!(verdicts instanceof Map)) return;
      const container = document.querySelector('main');
      if (!container) return;
      // Restrict to Startup/Registry lists only to avoid Scheduled Tasks and other sections
      const nameEls = container.querySelectorAll('[data-vt-scope="startup-list"] li .font-semibold, [data-vt-scope="registry-list"] li .font-semibold');
      try {
        // Debug: print current verdict keys snapshot and matches count
        const k = Array.from((($vtVerdicts as unknown as Map<string, string>) || new Map()).keys());
        console.debug('[VT] applyVtBadges: verdict keys(n)=', k.length, 'sample=', k.slice(0, 10));
        console.debug('[VT] applyVtBadges: candidates=', nameEls.length);
      } catch {}
      nameEls.forEach((el) => {
        if (!(el instanceof HTMLElement)) return;
        // Extract the first non-empty text node as the item name.
        // This avoids including any existing badges or nested elements.
        let name = '';
        for (const n of Array.from(el.childNodes)) {
          if (n.nodeType === Node.TEXT_NODE) {
            const t = (n.nodeValue || '').trim();
            if (t) { name = t; break; }
          }
        }
        if (!name) {
          // Fallback: join all text nodes only (ignore element children like badges)
          name = Array.from(el.childNodes)
            .filter((n) => n.nodeType === Node.TEXT_NODE)
            .map((n) => String((n as Text).nodeValue || '').trim())
            .filter(Boolean)
            .join(' ');
        }
        if (!name) return;
        const key = name.trim().toLowerCase();
        const lab = verdicts.get(key) as string | undefined;
        const parent = el;
        const existing = parent.querySelector(':scope > .vt-badge') as HTMLElement | null;
        if (!lab) { if (existing) existing.remove(); return; }
        const reason = lab === 'Not' ? (reasonFor(name) || 'Not Scanned') : '';
        const cls = lab === 'Safe'
          ? 'text-[10px] border-green-500/30 text-green-600 bg-green-500/10'
          : lab === 'Sus'
            ? 'text-[10px] border-red-500/30 text-red-600 bg-red-500/10'
            : 'text-[10px] border-yellow-500/30 text-yellow-700 bg-yellow-500/10';
        if (!existing) {
          const span = document.createElement('span');
          span.className = `vt-badge inline-flex items-center rounded border px-1 ml-2 ${cls}`;
          span.textContent = lab as any;
          if (reason) span.title = reason;
          parent.appendChild(span);
        } else {
          existing.className = `vt-badge inline-flex items-center rounded border px-1 ml-2 ${cls}`;
          (existing as HTMLElement).textContent = lab as any;
          if (reason) existing.title = reason; else existing.removeAttribute('title');
        }
      });
    } catch {}
  }

  let vtApplyTimer: number | null = null;
  let vtObserver: MutationObserver | null = null;
  $effect(() => {
    // Re-apply when verdicts or route changes
    // Accessing $vtVerdicts makes this reactive
    const _m = $vtVerdicts;
    const p = $page.url.pathname;
    if (p === '/optimize') {
      if (vtApplyTimer !== null) clearTimeout(vtApplyTimer as unknown as number);
      vtApplyTimer = setTimeout(() => applyVtBadges(), 120) as unknown as number;
      // Setup a MutationObserver once to keep badges in sync as lists render
      try {
        const container = document.querySelector('main');
        if (container && !vtObserver) {
          vtObserver = new MutationObserver(() => {
            // Debounce heavy DOM scans
            if (vtApplyTimer !== null) clearTimeout(vtApplyTimer as unknown as number);
            vtApplyTimer = setTimeout(() => applyVtBadges(), 100) as unknown as number;
          });
          vtObserver.observe(container, { childList: true, subtree: true });
        }
      } catch {}
    } else {
      if (vtObserver) { vtObserver.disconnect(); vtObserver = null; }
    }
  });

  // When user navigates to Optimize, kick off a VT scan of Startup Apps and Registry (if key present)
  $effect(() => {
    const p = $page.url.pathname;
    if (p !== '/optimize') return;
    (async () => {
      try {
        const st = (await invoke('vt_get_status')) as { key_set?: boolean };
        if ((st as any)?.key_set) {
          // Avoid overlapping with manual/background runs
          if (!vtBusy) {
            vtBusy = true;
            const need = (await invoke('vt_scan_needed', { limit: 50 })) as [number, number];
            const ns = Array.isArray(need) ? (need[0] ?? 0) : 0;
            const nr = Array.isArray(need) ? (need[1] ?? 0) : 0;
            if ((ns + nr) === 0) { vtBusy = false; return; }
            pushLog('INFO', `VT scan starting (optimize): needed startup ${ns}, registry ${nr}.`, 'Optimize');
            beginScan('optimize', { startup: ns, registry: nr });
            toast.message('VirusTotal scan started', { action: { label: 'Open details', onClick: () => { try { scanDialogOpen = true; } catch {} } } });
            const res = (await invoke('vt_scan_all', { limit: 50, force: false })) as [number, number];
            endScan({ startup: res?.[0], registry: res?.[1] });
            toast.success('VirusTotal scan completed', { action: { label: 'Open details', onClick: () => { try { scanDialogOpen = true; } catch {} } } });
            pushLog('SUCCESS', 'VT scan finished (optimize).', 'Optimize');
          }
        }
      } catch {}
      finally { vtBusy = false; }
    })();
  });

  async function runVtScanNow() {
    try {
      vtBusy = true;
      pushLog('INFO', 'VT scan starting (manual).', 'Optimize');
      beginScan('manual');
      toast.message('VirusTotal scan started', { action: { label: 'Open details', onClick: () => { try { scanDialogOpen = true; } catch {} } } });
      const res = (await invoke('vt_scan_all', { limit: 50, force: true })) as [number, number];
      endScan({ startup: res?.[0], registry: res?.[1] });
      toast.success('VirusTotal scan completed', { action: { label: 'Open details', onClick: () => { try { scanDialogOpen = true; } catch {} } } });
      pushLog('SUCCESS', `VT scan finished (manual): startup ${res?.[0] ?? 0}, registry ${res?.[1] ?? 0}.`, 'Optimize');
    } catch (e) {
      console.error(e);
      toast.error('VirusTotal scan failed (set API key?)');
      pushLog('ERROR', `VT scan failed (manual): ${String(e)}`, 'Optimize');
    } finally { vtBusy = false; }
  }
</script>

<ModeWatcher />

<SidebarProvider bind:open style="--sidebar-width-icon: 60px;" class="h-screen w-full">
  <Sidebar
    collapsible="icon"
    class="group/sidebar relative transition-[width] duration-200"
    aria-expanded={open}
  >
    <SidebarContent class="relative overflow-x-hidden">
      <button
        type="button"
        class="absolute top-1/2 -translate-y-1/2 right-0 z-20 opacity-0 pointer-events-none group-hover/sidebar:opacity-100 group-hover/sidebar:pointer-events-auto h-8 w-8 flex items-center justify-center rounded-l-md rounded-r-none border bg-background text-foreground shadow-xs hover:bg-accent hover:text-accent-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring/50 transition"
        aria-label={open ? 'Collapse sidebar' : 'Expand sidebar'}
        title={open ? 'Collapse sidebar' : 'Expand sidebar'}
        onclick={() => (open = !open)}
      >
        {#if open}
          <ChevronLeft class="size-5" />
        {:else}
          <ChevronRight class="size-5" />
        {/if}
      </button>
      <SidebarGroup>
        <SidebarGroupContent>
          {#if collapsed}
            <div class="flex justify-center px-3 py-3">
              <div class="relative flex h-10 w-10 items-center justify-center">
                <img
                  src="/favicon.png"
                  alt="Avelonia Logo"
                  class="h-10 w-10 rounded-full aspect-square object-contain"
                  width="25"
                  height="25"
                />
              </div>
            </div>
          {:else}
            <div class="flex items-center px-3 py-3">
              <div class="flex items-center gap-3">
                <img
                  src="/favicon.png"
                  alt="Avelonia Logo"
                  class="h-10 w-10 rounded-full aspect-square object-contain transition-transform duration-200"
                  width="25"
                  height="25"
                />
                <p class="font-medium text-base">Avelonia</p>
              </div>
            </div>
          {/if}
        </SidebarGroupContent>
      </SidebarGroup>

      <SidebarGroup>
        <SidebarGroupContent>
          <SidebarMenu>
            {#each menuItems as item}
              <SidebarMenuItem>
                <SidebarMenuButton
                  isActive={$page.url.pathname === item.href}
                  tooltipContent={collapsed ? item.label : undefined}
                >
                  {#snippet child({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
                  {@const { class: propsClass, ...restWithoutClass } = rawProps}
                  {@const restProps = restWithoutClass as Record<string, unknown>}
                  {@const Icon = item.icon}
                  <a
                    {...restProps}
                    href={item.href}
                    data-sveltekit-preload-data={['/optimize','/downloader','/cleaner'].includes(item.href) ? 'off' : undefined}
                    data-sveltekit-preload-code={['/optimize','/downloader','/cleaner'].includes(item.href) ? 'off' : undefined}
                    aria-current={$page.url.pathname === item.href ? 'page' : undefined}
                    class={cn(
                      'flex items-center transition-colors',
                      collapsed ? 'justify-center relative mx-auto' : 'gap-3 w-full',
                      propsClass
                    )}
                  >
                    <Icon
                      class={cn(
                        'size-5 shrink-0',
                        collapsed && $page.url.pathname === item.href
                          ? 'text-[var(--sidebar-accent-foreground)]'
                          : 'text-current'
                      )}
                      style={$page.url.pathname === item.href ? 'stroke-width:1.5' : 'stroke-width:1'}
                      aria-hidden="true"
                    />
                    {#if !collapsed}
                      <span>{item.label}</span>
                    {/if}
                    {#if item.showBadge && activeCount > 0}
                      {#if collapsed}
                        <Badge
                          variant="secondary"
                          class="absolute -top-1 -right-1 h-4 min-w-4 px-1 text-[10px] leading-none flex items-center justify-center"
                          aria-label={`Active downloads: ${activeCount}`}
                        >
                          {activeCount}
                        </Badge>
                      {:else}
                        <Badge
                          variant="secondary"
                          class="ml-auto"
                          aria-label={`Active downloads: ${activeCount}`}
                        >
                          {activeCount}
                        </Badge>
                      {/if}
                    {/if}
                  </a>
                  {/snippet}
                </SidebarMenuButton>
             </SidebarMenuItem>
            {/each}
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>

      <SidebarGroup class="mt-auto pb-4">
        <SidebarGroupContent>
          <Dialog bind:open={settingsOpen}>
            <div class="px-3 py-3">
              {#if collapsed}
                <div class="flex flex-col items-center gap-2">
                  {#snippet ThemeToggleTrigger({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
                  {@const { class: propsClass, ...restWithoutClass } = rawProps}
                  {@const restProps = restWithoutClass as Record<string, unknown>}
                  <Button
                    {...restProps}
                    variant="outline"
                    size="icon"
                    class={cn('relative', propsClass)}
                    onclick={toggleMode}
                    aria-label="Toggle theme"
                  >
                    <SunIcon class="h-[1.2rem] w-[1.2rem] text-current rotate-0 scale-100 !transition-all dark:-rotate-90 dark:scale-0" />
                    <MoonIcon class="absolute h-[1.2rem] w-[1.2rem] text-current rotate-90 scale-0 !transition-all dark:rotate-0 dark:scale-100" />
                  </Button>
                  {/snippet}
                  <Tooltip>
                    <TooltipTrigger child={ThemeToggleTrigger} />
                    <TooltipContent side="right" align="center">Toggle theme</TooltipContent>
                  </Tooltip>

                  {#snippet SettingsButtonContent({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
                  {@const { class: propsClass, ...restWithoutClass } = rawProps}
                  {@const restProps = restWithoutClass as Record<string, unknown>}
                  <Button
                    {...restProps}
                    variant="outline"
                    size="icon"
                    class={cn('relative', propsClass)}
                    aria-label="Open settings"
                  >
                    <SettingsIcon class="h-[1.2rem] w-[1.2rem] text-current" />
                  </Button>
                  {/snippet}
                  {#snippet SettingsTrigger({ props }: ButtonSnippetContext)}
                  <DialogTrigger {...((props ?? {}) as Record<string, unknown>)} child={SettingsButtonContent} />
                  {/snippet}
                  <Tooltip>
                    <TooltipTrigger child={SettingsTrigger} />
                    <TooltipContent side="right" align="center">Settings</TooltipContent>
                  </Tooltip>
                </div>
              {:else}
                <div class="flex items-center gap-3">
                  {#snippet SettingsButtonExpanded({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
                  {@const { class: propsClass, ...restWithoutClass } = rawProps}
                  {@const restProps = restWithoutClass as Record<string, unknown>}
                  <Button
                    {...restProps}
                    variant="outline"
                    size="sm"
                    class={cn('flex items-center gap-2', propsClass)}
                    aria-label="Open settings"
                  >
                    <SettingsIcon class="h-[1.2rem] w-[1.2rem] text-current" />
                    <span>Settings</span>
                  </Button>
                  {/snippet}
                  <DialogTrigger child={SettingsButtonExpanded} />
                  <Button
                    onclick={toggleMode}
                    variant="outline"
                    size="icon"
                    class="relative ml-auto"
                    aria-label="Toggle theme"
                  >
                    <SunIcon
                      class="h-[1.2rem] w-[1.2rem] text-current rotate-0 scale-100 !transition-all dark:-rotate-90 dark:scale-0"
                    />
                    <MoonIcon
                      class="absolute h-[1.2rem] w-[1.2rem] text-current rotate-90 scale-0 !transition-all dark:rotate-0 dark:scale-100"
                    />
                  </Button>
                </div>
              {/if}
            </div>

                        <DialogContent class="sm:max-w-xl">
              <div class="flex flex-col">
              <DialogHeader>
                <DialogTitle>Settings</DialogTitle>
                <DialogDescription>Configure Avelonia preferences.</DialogDescription>
              </DialogHeader>

              <div class="space-y-6">
                <!-- Downloads -->
                <section class="space-y-3">
                  <p class="text-sm font-medium">Downloads</p>
                  <div class="grid gap-3">
                    <label class="flex items-center gap-2 text-sm">
                      <Checkbox bind:checked={autoInstall} aria-controls="auto-install-advanced" aria-expanded={autoInstall} />
                      <span>Auto-install downloaded installers</span>
                    </label>

                    {#if autoInstall}
                      <div id="auto-install-advanced" class="rounded-md border border-border/60 bg-muted/10 p-3 sm:p-4 space-y-3 ml-0 sm:ml-4">
                        <div class="flex flex-col gap-2">
                          <Label class="text-xs text-muted-foreground">Install mode</Label>
                          <div class="max-w-[220px]">
                            <Select type="single" bind:value={installMode}>
                              <SelectTrigger placeholder="Select mode" />
                              <SelectContent>
                                <SelectItem value="silent">Silent</SelectItem>
                                <SelectItem value="normal">Normal</SelectItem>
                              </SelectContent>
                            </Select>
                          </div>
                        </div>
                        <label class="flex items-center gap-2 text-sm">
                          <Checkbox bind:checked={elevateInstall} />
                          <span>Run installers elevated (UAC)</span>
                        </label>
                        <label class="flex items-center gap-2 text-sm">
                          <Checkbox bind:checked={fallbackOpen} />
                          <span>Open normally if silent install fails</span>
                        </label>
                      </div>
                    {/if}

                    <label class="flex items-center gap-2 text-sm">
                      <Checkbox bind:checked={verifyInstall} />
                      <span>Verify installation in Uninstall registry</span>
                    </label>
                  </div>
                </section>

                <!-- Security / VirusTotal -->
                <section class="space-y-3">
                  <p class="text-sm font-medium">Security / VirusTotal</p>
                  <div class="space-y-2">
                    <Label class="text-xs text-muted-foreground">API key</Label>
                    <Input type="password" placeholder="Paste your VT API key" bind:value={vtKey} />
                    <label class="flex items-center gap-2 text-sm">
                      <Checkbox bind:checked={vtPersist} />
                      <span>Save key on this device</span>
                    </label>
                    <div class="flex gap-2">
                      <Button onclick={saveVtKey} disabled={vtBusy}>Save key</Button>
                      <Button variant="secondary" onclick={() => { void runVtScanNow(); }} disabled={!vtKeySet || vtBusy}>Run scan now</Button>
                    </div>
                    {#if !vtKeySet}
                      <p class="text-xs text-muted-foreground">Set an API key to enable reputation scans.</p>
                    {/if}
                  </div>
                </section>

                <!-- Privacy & Data -->
                <section class="space-y-3">
                  <p class="text-sm font-medium">Privacy & Data</p>
                  <div class="flex items-center justify-between gap-3">
                    <div>
                      <p class="text-sm">System logs</p>
                      <p class="text-xs text-muted-foreground">Clear all logs stored locally.</p>
                    </div>
                    <Button variant="secondary" onclick={() => { try { clearLogs(); toast.success('Logs cleared'); } catch {} }}>Clear logs</Button>
                  </div>
                </section>
              </div>


              <DialogFooter class="mt-6">
                <DialogClose>
                  <Button variant="ghost">Close</Button>
                </DialogClose>
                <DialogClose>
                  <Button>Done</Button>
                </DialogClose>
              </DialogFooter>
              </div>
            </DialogContent>
          </Dialog>
        </SidebarGroupContent>
      </SidebarGroup>
    </SidebarContent>
  </Sidebar>

  <main class="flex-1 overflow-y-auto p-8">
    {@render children?.()}
  </main>
</SidebarProvider>

{#snippet VtDetailsTrigger({ props })}
  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
  {@const { class: propsClass, ...restWithoutClass } = rawProps}
  {@const restProps = restWithoutClass as Record<string, unknown>}
  <span role="none" onclick={(e: MouseEvent) => e.stopPropagation()}>
    <Button {...restProps} type="button" variant="ghost" size="sm" aria-label="Details" class={propsClass}>
      <MoreHorizontal class="size-4" />
    </Button>
  </span>
{/snippet}

<!-- VT Scan Details Dialog -->
<Dialog bind:open={scanDialogOpen}>
  <DialogContent class="sm:max-w-2xl">
    <DialogHeader>
      <DialogTitle>VirusTotal Scan</DialogTitle>
      <DialogDescription>
        {#if (scan.phase === 'running')}
          Scanning startup and registry items...
        {:else if (scan.phase === 'done')}
          Scan finished.
        {:else}
          Idle. Trigger a scan from Settings.
        {/if}
      </DialogDescription>
    </DialogHeader>
    <div class="space-y-2 text-sm">
      <p class="text-xs text-muted-foreground">
        {#if (scan.phase === 'running')}
          Source: {scan.source ?? 'N/A'}
        {:else}
          {#if scan.startedAt}Started {new Date(scan.startedAt).toLocaleTimeString()}{/if}
          {#if scan.finishedAt} ? Finished {new Date(scan.finishedAt).toLocaleTimeString()}{/if}
          ? Processed {(scan.items?.length ?? 0)} items
          {#if (scan.expectedStartup ?? undefined) !== undefined || (scan.expectedRegistry ?? undefined) !== undefined}
            ? Expected {(scan.expectedStartup ?? '?')}/{(scan.expectedRegistry ?? '?')}
          {/if}
        {/if}
      </p>
      <div class="mb-1 flex flex-wrap gap-2">
        <Badge variant="secondary">Detected {vtTotals.detected}</Badge>
        <Badge class="border-green-500/30 text-green-700 bg-green-500/10">Clean {vtTotals.clean}</Badge>
        <Badge class="border-yellow-500/30 text-yellow-700 bg-yellow-500/10">Not Scanned {vtTotals.notScanned}</Badge>
      </div>
      <div class="max-h-64 overflow-auto rounded-md border border-border/60 bg-muted/10">
        <table class="w-full text-sm">
          <thead class="sticky top-0 bg-card/80 backdrop-blur supports-[backdrop-filter]:bg-card/70">
            <tr>
              <th class="text-left px-2 py-1">Subject</th>
              <th class="text-left px-2 py-1">From</th>
              <th class="text-left px-2 py-1">Verdict</th>
              <th class="text-left px-2 py-1">Not detected</th>
              <th class="text-left px-2 py-1">Details</th>
            </tr>
          </thead>
          <tbody>
            {#each scan.items as it}
              <tr>
                <td class="px-2 py-1 truncate max-w-[40ch]">{it.subject}</td>
                <td class="px-2 py-1">{it.source}</td>
                <td class="px-2 py-1">{it.verdict || '-'}{#if it.reason && (!it.verdict || it.verdict.toLowerCase() === 'unknown')} <span class="text-muted-foreground">({it.reason})</span>{/if}</td>
                <td class="px-2 py-1">
                  {#if typeof it.total_vendors === 'number'}
                    {Math.max(0, (it.total_vendors || 0) - (it.positives || 0))}
                  {:else if typeof it.harmless === 'number' || typeof it.undetected === 'number'}
                    {(it.harmless || 0) + (it.undetected || 0)}
                  {:else}
                    -
                  {/if}
                </td>
                <td class="px-2 py-1">
                  {@render VtDetailsTrigger({ props: { onclick: () => toggleVtDetails(it) } })}
                </td>
              </tr>
              {#if vtExpanded.has(vtKeyOf(it))}
                <tr>
                  <td class="px-2 py-2 text-xs text-muted-foreground" colspan="5">
                    <div class="grid grid-cols-2 gap-2">
                      <div>Malicious: {typeof it.malicious === 'number' ? it.malicious : '-'}</div>
                      <div>Suspicious: {typeof it.suspicious === 'number' ? it.suspicious : '-'}</div>
                      <div>Harmless: {typeof it.harmless === 'number' ? it.harmless : '-'}</div>
                      <div>Undetected: {typeof it.undetected === 'number' ? it.undetected : '-'}</div>
                    </div>
                    {#if it.reason}
                      <div class="mt-1">Reason: {it.reason}</div>
                    {/if}
                    {#if it.permalink}
                      <div class="mt-2">
                        <button type="button" class="px-0 text-xs text-white hover:text-emerald-600 underline-offset-4 hover:underline" onclick={() => { try { void openExternal(it.permalink as string); } catch {} }}>
                          Open on VirusTotal
                        </button>
                      </div>
                    {/if}
                  </td>
                </tr>
              {/if}
            {/each}
            {#if (scan.items?.length ?? 0) === 0}
              <tr><td colspan="5" class="px-2 py-3 text-center text-muted-foreground">No items yet.</td></tr>
            {/if}
          </tbody>
        </table>
      </div>
    </div>
    <DialogFooter>
      <DialogClose>
        <Button>Close</Button>
      </DialogClose>
    </DialogFooter>
  </DialogContent>
</Dialog>

<Toaster richColors closeButton duration={4000} position="bottom-right" />

