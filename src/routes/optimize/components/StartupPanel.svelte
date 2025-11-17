<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher, onMount } from 'svelte';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
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
  import { Button } from '$lib/components/ui/button';
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from '$lib/components/ui/card';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Input } from '$lib/components/ui/input';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { toast } from '$lib/components/ui/sonner';
  import { pushLog } from '$lib/logStore';
  import { Trash2, RefreshCw, FolderOpen, Eye, Search as SearchIcon } from '@lucide/svelte';
  import { SvelteSet } from 'svelte/reactivity';

  type StartupItem = { path: string; name: string };

  const STARTUP_MAX_DOM = 300;
  const STARTUP_ROW_PX = 56;

  const dispatchMessage = createEventDispatcher<{ message: string }>();

  let startupItems = $state<StartupItem[]>([]);
  let selectedStartup = $state(new Set<string>());
  let startupQuery = $state('');
  let startupLoaded = $state(false);
  let loadingStartup = $state(false);
  let startupVisible = $state(50);
  let startupStart = $state(0);
  let startupSentinel: HTMLElement | null = null;
  let startupScrollEl = $state<HTMLElement | null>(null);
  let startupQueryDeb = $state('');
  let showStartupConfirm = $state(false);
  let pendingStartup: string[] = $state([]);
  let _startupPollTimer: number | null = null;
  let _startupPollBusy = false;
  let _startupScrollTick = false;
  let sentinelObserved = false;

  $effect(() => {
    const timer = setTimeout(() => {
      startupQueryDeb = startupQuery;
    }, 180);
    return () => clearTimeout(timer);
  });

  async function loadStartupItems() {
    if (loadingStartup || startupLoaded) return;
    loadingStartup = true;
    try {
      const items = (await invoke('list_startup_shortcuts')) as StartupItem[];
      startupItems = Array.isArray(items) ? items : [];
      console.debug('[VT] startup items loaded:', startupItems.length);
      selectedStartup = new SvelteSet();
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
        const keep = new SvelteSet(selectedStartup);
        startupItems = next;
        selectedStartup = new SvelteSet(Array.from(keep).filter((p) => nextSet.has(p)));
        startupVisible = Math.min(Math.max(50, startupVisible), startupItems.length);
      }
    } catch {
      /* noop */
    } finally {
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

  function toggleStartup(path: string) {
    if (selectedStartup.has(path)) selectedStartup.delete(path);
    else selectedStartup.add(path);
    selectedStartup = new SvelteSet(selectedStartup);
  }

  function onStartupScroll(event: Event) {
    if (_startupScrollTick) return;
    _startupScrollTick = true;
    const target = event.currentTarget as HTMLElement | null;
    requestAnimationFrame(() => {
      const el = target || startupScrollEl;
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

  function requestRemoveSelectedStartup() {
    if (selectedStartup.size === 0) return;
    pendingStartup = Array.from(selectedStartup);
    showStartupConfirm = true;
  }

  async function confirmRemoveStartup() {
    showStartupConfirm = false;
    try {
      const removed: number = await invoke('remove_startup_shortcuts', {
        files: pendingStartup,
      });
      const text = `Removed ${removed} startup shortcut${removed === 1 ? '' : 's'}.`;
      dispatchMessage('message', text);
      if (removed > 0) {
        toast.success(text);
        pushLog('SUCCESS', text, 'Optimize');
      } else {
        const info = 'No startup shortcuts removed';
        toast.info(info);
        pushLog('INFO', info, 'Optimize');
      }
      selectedStartup = new SvelteSet();
      await reloadStartupItems();
    } catch (e) {
      const err = `Failed to remove startup items: ${e}`;
      dispatchMessage('message', err);
      toast.error('Failed to remove startup items');
      pushLog('ERROR', `Failed to remove startup items: ${String(e)}`, 'Optimize');
    } finally {
      pendingStartup = [];
    }
  }

  async function openStartupFolders() {
    try {
      const folders: string[] = await invoke('get_startup_folders');
      for (const folder of folders) {
        try {
          await openPath(folder);
        } catch {
          /* noop */
        }
      }
    } catch (e) {
      console.error(e);
    }
  }

  const filteredStartupItems = $derived(
    startupItems.filter((item) => {
      const q = startupQueryDeb.trim().toLowerCase();
      if (q === '') return true;
      return item.name.toLowerCase().includes(q) || item.path.toLowerCase().includes(q);
    })
  );

  const allStartupSelected = $derived(
    filteredStartupItems.length > 0 &&
      filteredStartupItems.every((item) => selectedStartup.has(item.path))
  );

  function toggleAllStartup() {
    if (filteredStartupItems.length === 0) return;
    selectedStartup = allStartupSelected
      ? new SvelteSet()
      : new SvelteSet(filteredStartupItems.map((item) => item.path));
  }

  $effect(() => {
    const total = filteredStartupItems.length;
    if (startupVisible > total) startupVisible = total;
    if (startupVisible - startupStart > STARTUP_MAX_DOM) {
      startupStart = Math.max(0, startupVisible - STARTUP_MAX_DOM);
    }
    if (startupStart > startupVisible) {
      startupStart = 0;
    }
  });

  onMount(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          if (startupSentinel && entry.target === startupSentinel) {
            loadStartupItems();
          }
        }
      },
      { rootMargin: '0px', threshold: 0.1 }
    );

    const ensureSentinel = () => {
      if (!sentinelObserved && startupSentinel) {
        observer.observe(startupSentinel);
        sentinelObserved = true;
      } else if (!sentinelObserved) {
        requestAnimationFrame(ensureSentinel);
      }
    };

    ensureSentinel();
    loadStartupItems();
    _startupPollTimer = window.setInterval(pollStartupOnce, 5000);

    return () => {
      observer.disconnect();
      if (_startupPollTimer) {
        clearInterval(_startupPollTimer);
        _startupPollTimer = null;
      }
    };
  });
</script>

<div class="space-y-2">
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
          >
            {allStartupSelected ? 'Deselect All' : 'Select All'}
          </Button>
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
                    >
                      <Eye class="size-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      title="Show in Explorer"
                      aria-label="Show in Explorer"
                      onclick={() => revealItemInDir(normalizeWinPath(item.path))}
                    >
                      <FolderOpen class="size-4" />
                    </Button>
                  </div>
                </label>
              </li>
            {/each}
            {#if startupVisible < filteredStartupItems.length}
              <li class="px-2 py-2">
                <Skeleton class="h-5 w-full" aria-hidden="true" />
              </li>
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

  <AlertDialog open={showStartupConfirm} onOpenChange={(value) => (showStartupConfirm = !!value)}>
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
            {#each pendingStartup.slice(0, 10) as entry}
              <li class="truncate">{normalizeWinPath(entry)}</li>
            {/each}
            {#if pendingStartup.length > 10}
              <li class="text-muted-foreground">
                . and {pendingStartup.length - 10} more
              </li>
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
</div>
