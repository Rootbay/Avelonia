<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { slide } from 'svelte/transition';
  import { openPath } from '@tauri-apps/plugin-opener';

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
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import { Separator } from '$lib/components/ui/separator';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { Alert, AlertDescription } from '$lib/components/ui/alert';
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
  } from '@lucide/svelte';

  let startupItems = $state<string[]>([]);
  let selected = $state(new Set<string>());
  let startupQuery = $state('');
  type StartupRegItem = { hive: string; key: string; name: string; command: string };
  let startupRegItems = $state<StartupRegItem[]>([]);
  let selectedReg = $state(new Set<string>());
  let registryQuery = $state('');
  let isBusy = $state(false);
  let message = $state('');
  let showWinsockConfirm = $state(false);
  let showRenewConfirm = $state(false);
  let showFlushConfirm = $state(false);
  let showNetAllConfirm = $state(false);
  let showDisableAllStartupConfirm = $state(false);
  let showDisableAllRegistryConfirm = $state(false);

  async function loadStartupItems() {
    try {
      startupItems = await invoke('list_startup_shortcuts');
      selected = new Set();
    } catch (e) {
      console.error(e);
    }
  }

  async function openStartupFolders() {
    try {
      const folders: string[] = await invoke('get_startup_folders');
      for (const f of folders) {
        try {
          await openPath(f);
        } catch (e) {
          console.warn('openPath failed', e);
        }
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function loadRegistryItems() {
    try {
      startupRegItems = await invoke('list_registry_run');
      selectedReg = new Set();
    } catch (e) {
      console.error(e);
    }
  }

  onMount(() => {
    loadStartupItems();
    loadRegistryItems();
  });

  function toggle(p: string) {
    if (selected.has(p)) selected.delete(p);
    else selected.add(p);
    selected = new Set(selected);
  }

  async function disableSelected() {
    if (selected.size === 0) return;
    isBusy = true;
    message = '';
    try {
      const count: number = await invoke('remove_startup_shortcuts', {
        files: Array.from(selected),
      });
      message = `Disabled ${count} startup item(s) (moved to Recycle Bin).`;
      await loadStartupItems();
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
    } finally {
      isBusy = false;
    }
  }

  function regId(it: StartupRegItem) {
    return `${it.hive}|${it.key}|${it.name}`;
  }

  function toggleReg(it: StartupRegItem) {
    const id = regId(it);
    if (selectedReg.has(id)) selectedReg.delete(id);
    else selectedReg.add(id);
    selectedReg = new Set(selectedReg);
  }

  async function disableSelectedRegistry() {
    if (selectedReg.size === 0) return;
    isBusy = true;
    message = '';
    try {
      const entries = startupRegItems.filter((it) => selectedReg.has(regId(it)));
      const count: number = await invoke('remove_registry_run', { entries });
      message = `Disabled ${count} registry startup entr${count === 1 ? 'y' : 'ies'}.`;
      await loadRegistryItems();
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
    } finally {
      isBusy = false;
    }
  }

  function confirmFlushDns() {
    showFlushConfirm = true;
  }
  async function flushDns() {
    isBusy = true;
    message = '';
    try {
      await invoke('flush_dns');
      message = 'Flushed DNS cache.';
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
    } finally {
      isBusy = false;
    }
  }

  function confirmResetWinsock() {
    showWinsockConfirm = true;
  }
  async function resetWinsock() {
    isBusy = true;
    message = '';
    try {
      await invoke('reset_winsock');
      message = 'Winsock reset. Reboot recommended.';
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
    } finally {
      isBusy = false;
    }
  }

  function confirmRenewIp() {
    showRenewConfirm = true;
  }
  async function renewIp() {
    isBusy = true;
    message = '';
    try {
      await invoke('renew_ip');
      message = 'Renewed IP lease.';
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
    } finally {
      isBusy = false;
    }
  }

  function selectAllStartup() {
    selected = new Set(filteredStartupItems);
  }
  function clearStartupSelection() {
    selected = new Set();
  }
  function invertStartupSelection() {
    const next = new Set<string>();
    for (const p of filteredStartupItems) {
      if (!selected.has(p)) next.add(p);
    }
    selected = next;
  }
  function selectAllRegistry() {
    selectedReg = new Set(filteredRegistryItems.map(regId));
  }
  function clearRegistrySelection() {
    selectedReg = new Set();
  }
  function invertRegistrySelection() {
    const ids = filteredRegistryItems.map(regId);
    const next = new Set<string>();
    for (const id of ids) {
      if (!selectedReg.has(id)) next.add(id);
    }
    selectedReg = next;
  }

  async function disableAllStartup() {
    if (startupItems.length === 0) return;
    isBusy = true;
    message = '';
    try {
      const count: number = await invoke('remove_startup_shortcuts', { files: startupItems });
      message = `Disabled ${count} startup item(s) (moved to Recycle Bin).`;
      await loadStartupItems();
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
    } finally {
      isBusy = false;
    }
  }

  async function disableAllRegistry() {
    if (startupRegItems.length === 0) return;
    isBusy = true;
    message = '';
    try {
      const count: number = await invoke('remove_registry_run', { entries: startupRegItems });
      message = `Disabled ${count} registry startup entr${count === 1 ? 'y' : 'ies'}.`;
      await loadRegistryItems();
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
    } finally {
      isBusy = false;
    }
  }

  async function optimizeNetworkAll() {
    isBusy = true;
    message = '';
    try {
      await invoke('flush_dns');
      await invoke('reset_winsock');
      await invoke('renew_ip');
      message = 'Flushed DNS, reset Winsock, and renewed IP.';
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
    } finally {
      isBusy = false;
    }
  }

  function revealPath(p: string) {
    try {
      const idx = Math.max(p.lastIndexOf('\\'), p.lastIndexOf('/'));
      const dir = idx > 0 ? p.slice(0, idx) : p;
      openPath(dir).catch((err) => console.warn('openPath failed', err));
    } catch (e) {
      console.warn(e);
    }
  }
  async function copyText(txt: string) {
    try {
      await navigator.clipboard.writeText(txt);
      message = 'Copied to clipboard.';
    } catch (e) {
      console.error(e);
      message = 'Copy failed.';
    }
  }

  let filteredStartupItems = $derived(
    startupItems.filter((p) =>
      startupQuery.trim() === ''
        ? true
        : p.toLowerCase().includes(startupQuery.trim().toLowerCase())
    )
  );
  let filteredRegistryItems = $derived(
    startupRegItems.filter((it) => {
      const q = registryQuery.trim().toLowerCase();
      if (q === '') return true;
      return (
        it.name.toLowerCase().includes(q) ||
        it.command.toLowerCase().includes(q) ||
        it.key.toLowerCase().includes(q) ||
        it.hive.toLowerCase().includes(q)
      );
    })
  );
</script>

<div class="space-y-6">
  <Card>
    <CardHeader>
      <CardTitle>Optimize</CardTitle>
      <CardDescription>Disable startup apps and run quick tune-ups.</CardDescription>
    </CardHeader>
  </Card>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
    <!-- Startup Apps -->
    <Card>
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <Trash2 class="size-5" /> Startup Apps
        </CardTitle>
        <CardDescription>Disable unwanted startup items (Startup folders).</CardDescription>
      </CardHeader>
      <CardContent class="space-y-3">
        <div class="flex flex-wrap gap-2">
          <Button size="sm" onclick={loadStartupItems} disabled={isBusy} variant="secondary">
            <RefreshCw class="mr-2 size-4" /> Refresh
          </Button>
          <Button size="sm" onclick={openStartupFolders} disabled={isBusy} variant="outline">
            <FolderOpen class="mr-2 size-4" /> Open Startup Folder(s)
          </Button>
          <Button
            size="sm"
            onclick={disableSelected}
            disabled={isBusy || selected.size === 0}
            variant="destructive"
          >
            <Trash2 class="mr-2 size-4" /> Disable Selected
          </Button>
          <Button
            size="sm"
            onclick={() => (showDisableAllStartupConfirm = true)}
            disabled={isBusy || startupItems.length === 0}
            variant="outline"
          >
            Disable All
          </Button>
        </div>

        <div class="flex items-center gap-2">
          <div class="relative flex-1">
            <SearchIcon
              class="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground"
            />
            <Input class="pl-9" placeholder="Filter by name or path..." bind:value={startupQuery} />
          </div>
          <div class="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              onclick={selectAllStartup}
              disabled={isBusy || filteredStartupItems.length === 0}>Select All</Button
            >
            <Button
              size="sm"
              variant="outline"
              onclick={clearStartupSelection}
              disabled={isBusy || selected.size === 0}>Clear</Button
            >
            <Button
              size="sm"
              variant="outline"
              onclick={invertStartupSelection}
              disabled={isBusy || filteredStartupItems.length === 0}>Invert</Button
            >
          </div>
        </div>

        {#if filteredStartupItems.length > 0}
          <ScrollArea class="h-[300px] rounded-md border">
            <ul class="divide-y">
              {#each filteredStartupItems as item (item)}
                <li class="py-2">
                  <label class="flex items-center justify-between gap-3">
                    <div class="flex items-center gap-3 min-w-0 flex-1">
                      <Checkbox checked={selected.has(item)} onclick={() => toggle(item)} />
                      <span class="font-mono truncate max-w-[52ch]">{item}</span>
                    </div>
                    <div class="flex items-center gap-2 shrink-0">
                      <Button
                        variant="link"
                        size="sm"
                        title="Reveal in Explorer"
                        onclick={() => revealPath(item)}
                      >
                        <Eye class="mr-1 size-4" /> Reveal
                      </Button>
                      <Button
                        variant="link"
                        size="sm"
                        title="Copy path"
                        onclick={() => copyText(item)}
                      >
                        <CopyIcon class="mr-1 size-4" /> Copy
                      </Button>
                    </div>
                  </label>
                </li>
              {/each}
            </ul>
          </ScrollArea>
        {:else}
          <p class="text-sm text-muted-foreground">No startup items found.</p>
        {/if}
      </CardContent>
    </Card>

    <!-- Registry Startup -->
    <Card>
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <Settings class="size-5" /> Registry Startup (Run keys)
        </CardTitle>
        <CardDescription>Entries from HKCU/HKLM Run and RunOnce keys.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-3">
        <div class="flex flex-wrap gap-2">
          <Button size="sm" onclick={loadRegistryItems} disabled={isBusy} variant="secondary">
            <RefreshCw class="mr-2 size-4" /> Refresh
          </Button>
          <Button
            size="sm"
            onclick={disableSelectedRegistry}
            disabled={isBusy || selectedReg.size === 0}
            variant="destructive"
          >
            <Trash2 class="mr-2 size-4" /> Disable Selected
          </Button>
          <Button
            size="sm"
            onclick={() => (showDisableAllRegistryConfirm = true)}
            disabled={isBusy || startupRegItems.length === 0}
            variant="outline"
          >
            Disable All
          </Button>
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
          <div class="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              onclick={selectAllRegistry}
              disabled={isBusy || filteredRegistryItems.length === 0}>Select All</Button
            >
            <Button
              size="sm"
              variant="outline"
              onclick={clearRegistrySelection}
              disabled={isBusy || selectedReg.size === 0}>Clear</Button
            >
            <Button
              size="sm"
              variant="outline"
              onclick={invertRegistrySelection}
              disabled={isBusy || filteredRegistryItems.length === 0}>Invert</Button
            >
          </div>
        </div>

        {#if filteredRegistryItems.length > 0}
          <ScrollArea class="h-[300px] rounded-md border">
            <ul class="divide-y">
              {#each filteredRegistryItems as it (regId(it))}
                <li class="py-2 space-y-1">
                  <label class="flex items-center gap-3">
                    <Checkbox checked={selectedReg.has(regId(it))} onclick={() => toggleReg(it)} />
                    <span class="font-semibold">{it.name}</span>
                  </label>
                  <div class="text-xs text-muted-foreground">{it.hive}\{it.key}</div>
                  <div
                    class="flex items-center justify-between gap-3 text-xs text-muted-foreground"
                  >
                    <span class="font-mono truncate max-w-[52ch]">{it.command}</span>
                    <div class="flex items-center gap-2 shrink-0">
                      <Button
                        variant="link"
                        size="sm"
                        title="Copy command"
                        onclick={() => copyText(it.command)}
                      >
                        <CopyIcon class="mr-1 size-4" /> Copy
                      </Button>
                      <Button
                        variant="link"
                        size="sm"
                        title="Copy registry path"
                        onclick={() => copyText(`${it.hive}\\${it.key} :: ${it.name}`)}
                      >
                        <CopyIcon class="mr-1 size-4" /> Copy Path
                      </Button>
                    </div>
                  </div>
                </li>
              {/each}
            </ul>
          </ScrollArea>
        {:else}
          <p class="text-sm text-muted-foreground">No registry startup entries found.</p>
        {/if}
      </CardContent>
    </Card>

    <!-- Network -->
    <Card>
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <NetworkIcon class="size-5" /> Network
        </CardTitle>
        <CardDescription>Quick network tune-ups.</CardDescription>
      </CardHeader>
      <CardContent>
        <div class="flex flex-wrap gap-2">
          <Button size="sm" onclick={confirmFlushDns} disabled={isBusy} variant="secondary">
            <RefreshCcw class="mr-2 size-4" /> Flush DNS Cache
          </Button>
          <Button size="sm" onclick={confirmResetWinsock} disabled={isBusy} variant="outline">
            <RotateCcw class="mr-2 size-4" /> Reset Winsock
          </Button>
          <Button size="sm" onclick={confirmRenewIp} disabled={isBusy} variant="outline">
            <RefreshCw class="mr-2 size-4" /> Renew IP
          </Button>
          <Button
            size="sm"
            onclick={() => (showNetAllConfirm = true)}
            disabled={isBusy}
            variant="destructive"
          >
            <NetworkIcon class="mr-2 size-4" /> Optimize Network (All)
          </Button>
        </div>
      </CardContent>
    </Card>

    {#if message}
      <div class="lg:col-span-2">
        <Alert>
          <AlertDescription>{message}</AlertDescription>
        </Alert>
      </div>
    {/if}
  </div>
</div>

{#if isBusy}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm"
    role="status"
    aria-live="polite"
    aria-busy="true"
  >
    <div class="w-[min(28rem,calc(100%-2rem))] space-y-4 rounded-lg bg-card p-6 shadow-lg">
      <div class="flex items-center gap-3">
        <Skeleton class="size-10 rounded-full" aria-hidden="true" />
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
      <p class="text-sm text-muted-foreground">Working...</p>
    </div>
  </div>
{/if}

<AlertDialog bind:open={showWinsockConfirm}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Confirm Winsock Reset</AlertDialogTitle>
      <AlertDialogDescription>
        This will reset the network stack and may require a reboot. Continue?
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={() => (showWinsockConfirm = false)}>Cancel</AlertDialogCancel>
      <AlertDialogAction
        onclick={() => {
          showWinsockConfirm = false;
          resetWinsock();
        }}>Reset</AlertDialogAction
      >
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>

<AlertDialog bind:open={showRenewConfirm}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Confirm IP Renew</AlertDialogTitle>
      <AlertDialogDescription>
        This will release and renew your IP address and briefly interrupt connectivity. Proceed?
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={() => (showRenewConfirm = false)}>Cancel</AlertDialogCancel>
      <AlertDialogAction
        onclick={() => {
          showRenewConfirm = false;
          renewIp();
        }}>Renew</AlertDialogAction
      >
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>

<AlertDialog bind:open={showFlushConfirm}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Confirm Flush DNS</AlertDialogTitle>
      <AlertDialogDescription>
        This clears the DNS resolver cache and may temporarily affect name resolution. Proceed?
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={() => (showFlushConfirm = false)}>Cancel</AlertDialogCancel>
      <AlertDialogAction
        onclick={() => {
          showFlushConfirm = false;
          flushDns();
        }}>Flush</AlertDialogAction
      >
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>

<AlertDialog bind:open={showNetAllConfirm}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Optimize Network</AlertDialogTitle>
      <AlertDialogDescription>
        Run Flush DNS, Reset Winsock, and Renew IP in sequence. This will briefly interrupt
        connectivity and may require a reboot.
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={() => (showNetAllConfirm = false)}>Cancel</AlertDialogCancel>
      <AlertDialogAction
        onclick={() => {
          showNetAllConfirm = false;
          optimizeNetworkAll();
        }}>Run All</AlertDialogAction
      >
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>

<AlertDialog bind:open={showDisableAllStartupConfirm}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Disable All Startup Shortcuts</AlertDialogTitle>
      <AlertDialogDescription>
        Moves all items from Startup folders to the Recycle Bin. Continue?
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={() => (showDisableAllStartupConfirm = false)}
        >Cancel</AlertDialogCancel
      >
      <AlertDialogAction
        onclick={() => {
          showDisableAllStartupConfirm = false;
          disableAllStartup();
        }}>Disable All</AlertDialogAction
      >
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>

<AlertDialog bind:open={showDisableAllRegistryConfirm}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Disable All Registry Run Entries</AlertDialogTitle>
      <AlertDialogDescription>
        Removes all values from common Run and RunOnce keys in HKCU/HKLM. Continue?
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onclick={() => (showDisableAllRegistryConfirm = false)}
        >Cancel</AlertDialogCancel
      >
      <AlertDialogAction
        onclick={() => {
          showDisableAllRegistryConfirm = false;
          disableAllRegistry();
        }}>Disable All</AlertDialogAction
      >
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
