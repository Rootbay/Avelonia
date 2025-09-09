<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { slide } from 'svelte/transition';
  import LoadingOverlay from '$lib/components/LoadingOverlay.svelte';
  import { openPath } from '@tauri-apps/plugin-opener';

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
        try { await openPath(f); } catch (e) { console.warn('openPath failed', e); }
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
    if (selected.has(p)) selected.delete(p); else selected.add(p);
    selected = new Set(selected);
  }

  async function disableSelected() {
    if (selected.size === 0) return;
    isBusy = true;
    message = '';
    try {
      const count: number = await invoke('remove_startup_shortcuts', { files: Array.from(selected) });
      message = `Disabled ${count} startup item(s) (moved to Recycle Bin).`;
      await loadStartupItems();
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
    } finally {
      isBusy = false;
    }
  }

  function regId(it: StartupRegItem) { return `${it.hive}|${it.key}|${it.name}`; }

  function toggleReg(it: StartupRegItem) {
    const id = regId(it);
    if (selectedReg.has(id)) selectedReg.delete(id); else selectedReg.add(id);
    selectedReg = new Set(selectedReg);
  }

  async function disableSelectedRegistry() {
    if (selectedReg.size === 0) return;
    isBusy = true;
    message = '';
    try {
      const entries = startupRegItems.filter(it => selectedReg.has(regId(it)));
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

  function confirmFlushDns() { showFlushConfirm = true; }
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

  function confirmResetWinsock() { showWinsockConfirm = true; }
  async function resetWinsock() {
    isBusy = true; message = '';
    try { await invoke('reset_winsock'); message = 'Winsock reset. Reboot recommended.'; }
    catch (e) { console.error(e); message = `Failed: ${e}`; }
    finally { isBusy = false; }
  }

  function confirmRenewIp() { showRenewConfirm = true; }
  async function renewIp() {
    isBusy = true; message = '';
    try { await invoke('renew_ip'); message = 'Renewed IP lease.'; }
    catch (e) { console.error(e); message = `Failed: ${e}`; }
    finally { isBusy = false; }
  }

  // Bulk helpers ------------------------------------------------------
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
    isBusy = true; message = '';
    try {
      const count: number = await invoke('remove_startup_shortcuts', { files: startupItems });
      message = `Disabled ${count} startup item(s) (moved to Recycle Bin).`;
      await loadStartupItems();
    } catch (e) { console.error(e); message = `Failed: ${e}`; }
    finally { isBusy = false; }
  }

  async function disableAllRegistry() {
    if (startupRegItems.length === 0) return;
    isBusy = true; message = '';
    try {
      const count: number = await invoke('remove_registry_run', { entries: startupRegItems });
      message = `Disabled ${count} registry startup entr${count === 1 ? 'y' : 'ies'}.`;
      await loadRegistryItems();
    } catch (e) { console.error(e); message = `Failed: ${e}`; }
    finally { isBusy = false; }
  }

  async function optimizeNetworkAll() {
    isBusy = true; message = '';
    try {
      await invoke('flush_dns');
      await invoke('reset_winsock');
      await invoke('renew_ip');
      message = 'Flushed DNS, reset Winsock, and renewed IP.';
    } catch (e) { console.error(e); message = `Failed: ${e}`; }
    finally { isBusy = false; }
  }

  function revealPath(p: string) {
    try {
      // Open the containing folder; opener will focus it
      const idx = Math.max(p.lastIndexOf('\\'), p.lastIndexOf('/'));
      const dir = idx > 0 ? p.slice(0, idx) : p;
      openPath(dir).catch(err => console.warn('openPath failed', err));
    } catch (e) { console.warn(e); }
  }
  async function copyText(txt: string) {
    try { await navigator.clipboard.writeText(txt); message = 'Copied to clipboard.'; }
    catch (e) { console.error(e); message = 'Copy failed.'; }
  }

  let filteredStartupItems = $derived(startupItems.filter(p =>
    startupQuery.trim() === '' ? true : p.toLowerCase().includes(startupQuery.trim().toLowerCase())
  ));
  let filteredRegistryItems = $derived(startupRegItems.filter(it => {
    const q = registryQuery.trim().toLowerCase();
    if (q === '') return true;
    return it.name.toLowerCase().includes(q)
      || it.command.toLowerCase().includes(q)
      || it.key.toLowerCase().includes(q)
      || it.hive.toLowerCase().includes(q);
  }));
</script>

<div class="main-content">
  <div class="header-card">
    <h1>Optimize</h1>
    <p class="muted">Disable startup apps and run quick tune-ups.</p>
  </div>

<div class="grid">
  <section class="panel">
    <h2>Startup Apps</h2>
    <p class="muted">Disable unwanted startup items (Startup folders).</p>
    <div class="actions">
      <button class="btn" onclick={loadStartupItems} disabled={isBusy}>Refresh</button>
      <button class="btn" onclick={openStartupFolders} disabled={isBusy}>Open Startup Folder(s)</button>
      <button class="btn danger" onclick={disableSelected} disabled={isBusy || selected.size === 0}>Disable Selected</button>
      <button class="btn" onclick={() => showDisableAllStartupConfirm = true} disabled={isBusy || startupItems.length === 0}>Disable All</button>
    </div>
    <div class="toolbar">
      <input class="search" placeholder="Filter by name or path..." bind:value={startupQuery} />
      <div class="spacer"></div>
      <button class="btn sm" onclick={selectAllStartup} disabled={isBusy || filteredStartupItems.length === 0}>Select All</button>
      <button class="btn sm" onclick={clearStartupSelection} disabled={isBusy || selected.size === 0}>Clear</button>
      <button class="btn sm" onclick={invertStartupSelection} disabled={isBusy || filteredStartupItems.length === 0}>Invert</button>
    </div>
    {#if filteredStartupItems.length > 0}
      <ul class="list">
        {#each filteredStartupItems as item (item)}
          <li>
            <label class="row">
              <input type="checkbox" checked={selected.has(item)} onchange={() => toggle(item)} />
              <span class="mono truncate">{item}</span>
              <div class="row gap">
                <button class="link" title="Reveal in Explorer" onclick={() => revealPath(item)}>Reveal</button>
                <button class="link" title="Copy path" onclick={() => copyText(item)}>Copy</button>
              </div>
            </label>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="muted">No startup items found.</p>
    {/if}
  </section>

  <section class="panel">
    <h2>Registry Startup (Run keys)</h2>
    <p class="muted">Entries from HKCU/HKLM Run and RunOnce keys.</p>
    <div class="actions">
      <button class="btn" onclick={loadRegistryItems} disabled={isBusy}>Refresh</button>
      <button class="btn danger" onclick={disableSelectedRegistry} disabled={isBusy || selectedReg.size === 0}>Disable Selected</button>
      <button class="btn" onclick={() => showDisableAllRegistryConfirm = true} disabled={isBusy || startupRegItems.length === 0}>Disable All</button>
    </div>
    <div class="toolbar">
      <input class="search" placeholder="Filter by name, command, or key..." bind:value={registryQuery} />
      <div class="spacer"></div>
      <button class="btn sm" onclick={selectAllRegistry} disabled={isBusy || filteredRegistryItems.length === 0}>Select All</button>
      <button class="btn sm" onclick={clearRegistrySelection} disabled={isBusy || selectedReg.size === 0}>Clear</button>
      <button class="btn sm" onclick={invertRegistrySelection} disabled={isBusy || filteredRegistryItems.length === 0}>Invert</button>
    </div>
    {#if filteredRegistryItems.length > 0}
      <ul class="list">
        {#each filteredRegistryItems as it (regId(it))}
          <li>
            <label class="row">
              <input type="checkbox" checked={selectedReg.has(regId(it))} onchange={() => toggleReg(it)} />
              <span class="semi">{it.name}</span>
            </label>
            <div class="muted" style="font-size: 0.85em;">
              {it.hive}\{it.key}
            </div>
            <div class="muted row between" style="font-size: 0.85em;">
              <span class="mono truncate">{it.command}</span>
              <div class="row gap">
                <button class="link" title="Copy command" onclick={() => copyText(it.command)}>Copy</button>
                <button class="link" title="Copy registry path" onclick={() => copyText(`${it.hive}\\${it.key} :: ${it.name}`)}>Copy Path</button>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="muted">No registry startup entries found.</p>
    {/if}
  </section>

  <section class="panel">
    <h2>Network</h2>
    <p class="muted">Quick network tune-ups.</p>
    <div class="actions">
      <button class="btn" onclick={confirmFlushDns} disabled={isBusy}>Flush DNS Cache</button>
      <button class="btn" onclick={confirmResetWinsock} disabled={isBusy}>Reset Winsock</button>
      <button class="btn" onclick={confirmRenewIp} disabled={isBusy}>Renew IP</button>
      <button class="btn danger" onclick={() => showNetAllConfirm = true} disabled={isBusy}>Optimize Network (All)</button>
    </div>
  </section>

  {#if message}
    <p class="message" transition:slide>{message}</p>
  {/if}

</div>

</div>

<style>
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .panel { background: var(--avelonia-card); border: 1px solid var(--avelonia-border); border-radius: 12px; padding: 16px; }
  .actions { display: flex; gap: 8px; margin: 8px 0 12px; }
  .list { border: 1px solid var(--avelonia-border); border-radius: 8px; padding: 8px; max-height: 300px; overflow: auto; }
  .list li { content-visibility: auto; contain-intrinsic-size: 0 32px; }
  .row { display: flex; gap: 8px; align-items: center; }
  .row.gap { gap: 12px; }
  .row.between { justify-content: space-between; align-items: center; gap: 12px; }
  .muted { color: var(--avelonia-text-muted); }
  .message { margin-top: 12px; color: var(--avelonia-text); }
  .btn { padding: 8px 12px; border-radius: 6px; cursor: pointer; border: 1px solid var(--avelonia-border); background: #1b1c1f; color: #fff; }
  .btn.danger { background: var(--avelonia-danger); border: none; }
  .btn.sm { padding: 6px 10px; font-size: 0.85em; }
  .toolbar { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
  .search { flex: 1 1 auto; padding: 8px 10px; border-radius: 6px; border: 1px solid var(--avelonia-border); background: #131417; color: var(--avelonia-text); }
  .spacer { flex: 1; }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace; }
  .semi { font-weight: 600; }
  .link { background: transparent; border: none; color: var(--avelonia-accent, #7aa2f7); cursor: pointer; padding: 2px 4px; }
  .truncate { max-width: 52ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: inline-block; vertical-align: bottom; }
  @media (max-width: 1024px) { .grid { grid-template-columns: 1fr; } }
  .modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: flex; align-items: center; justify-content: center; z-index: 50; }
  .modal-card { max-width: 520px; width: 100%; }
  .modal-title { font-size: 1.125rem; font-weight: 700; margin-bottom: 0.5rem; }
</style>

<LoadingOverlay show={isBusy} text={'Working...'} />

{#if showWinsockConfirm}
  <div class="modal-overlay">
    <div class="panel modal-card">
      <h2 class="modal-title">Confirm Winsock Reset</h2>
      <p class="muted">This will reset the network stack and may require a reboot. Continue?</p>
      <div class="actions" style="justify-content: flex-end;">
        <button class="btn" onclick={() => showWinsockConfirm = false}>Cancel</button>
        <button class="btn danger" onclick={() => { showWinsockConfirm = false; resetWinsock(); }}>Reset</button>
      </div>
    </div>
  </div>
{/if}

{#if showRenewConfirm}
  <div class="modal-overlay">
    <div class="panel modal-card">
      <h2 class="modal-title">Confirm IP Renew</h2>
      <p class="muted">This will release and renew your IP address and briefly interrupt connectivity. Proceed?</p>
      <div class="actions" style="justify-content: flex-end;">
        <button class="btn" onclick={() => showRenewConfirm = false}>Cancel</button>
        <button class="btn" onclick={() => { showRenewConfirm = false; renewIp(); }}>Renew</button>
      </div>
    </div>
  </div>
{/if}

{#if showFlushConfirm}
  <div class="modal-overlay">
    <div class="panel modal-card">
      <h2 class="modal-title">Confirm Flush DNS</h2>
      <p class="muted">This clears the DNS resolver cache and may temporarily affect name resolution. Proceed?</p>
      <div class="actions" style="justify-content: flex-end;">
        <button class="btn" onclick={() => showFlushConfirm = false}>Cancel</button>
        <button class="btn" onclick={() => { showFlushConfirm = false; flushDns(); }}>Flush</button>
      </div>
    </div>
  </div>
{/if}

{#if showNetAllConfirm}
  <div class="modal-overlay">
    <div class="panel modal-card">
      <h2 class="modal-title">Optimize Network</h2>
      <p class="muted">Run Flush DNS, Reset Winsock, and Renew IP in sequence. This will briefly interrupt connectivity and may require a reboot.</p>
      <div class="actions" style="justify-content: flex-end;">
        <button class="btn" onclick={() => showNetAllConfirm = false}>Cancel</button>
        <button class="btn danger" onclick={() => { showNetAllConfirm = false; optimizeNetworkAll(); }}>Run All</button>
      </div>
    </div>
  </div>
{/if}

{#if showDisableAllStartupConfirm}
  <div class="modal-overlay">
    <div class="panel modal-card">
      <h2 class="modal-title">Disable All Startup Shortcuts</h2>
      <p class="muted">Moves all items from Startup folders to the Recycle Bin. Continue?</p>
      <div class="actions" style="justify-content: flex-end;">
        <button class="btn" onclick={() => showDisableAllStartupConfirm = false}>Cancel</button>
        <button class="btn danger" onclick={() => { showDisableAllStartupConfirm = false; disableAllStartup(); }}>Disable All</button>
      </div>
    </div>
  </div>
{/if}

{#if showDisableAllRegistryConfirm}
  <div class="modal-overlay">
    <div class="panel modal-card">
      <h2 class="modal-title">Disable All Registry Run Entries</h2>
      <p class="muted">Removes all values from common Run and RunOnce keys in HKCU/HKLM. Continue?</p>
      <div class="actions" style="justify-content: flex-end;">
        <button class="btn" onclick={() => showDisableAllRegistryConfirm = false}>Cancel</button>
        <button class="btn danger" onclick={() => { showDisableAllRegistryConfirm = false; disableAllRegistry(); }}>Disable All</button>
      </div>
    </div>
  </div>
{/if}
