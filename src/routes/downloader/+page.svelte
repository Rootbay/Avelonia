<script lang="ts">
  import { downloads } from "$lib/downloads";
  import FilterPanel from '$lib/components/FilterPanel.svelte';
  import DownloadItem from '$lib/components/DownloadItem.svelte';
  import { startDownload, cancelDownload, getDownloadPath } from '$lib/downloadManager';
  import type { Download } from '$lib/downloadManager';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';

  let searchTerm = $state("");
  let debouncedSearchTerm = $state("");
  $effect(() => {
    const t = setTimeout(() => { debouncedSearchTerm = searchTerm; }, 150);
    return () => clearTimeout(t);
  });
  let showFilters = $state(false);
  let actionsOpen = $state(false);
  let filters = $state({
    fileType: "",
    category: "",
    minSize: "",
    maxSize: "",
    eta: "",
    status: "",
  });
  // Quick status filter chips
  let statusGroup = $state<'all'|'available'|'active'|'completed'|'failed'>('all');
  let sortBy = $state('name');
  let sortDirection = $state('asc');
  // Selection model for bulk actions
  let selectedIds = $state(new Set<number>());
  const isSelected = (id: number) => selectedIds.has(id);
  // reference for tri-state select-all checkbox
  let selectAllCheckbox: HTMLInputElement | null = null;
  // simple aria-live announcements for actions
  let announce = $state('');
  let showHelp = $state(false);
  // last anchor index for shift-select
  let lastSelectedIndex: number | null = null;
  function toggleSelect(id: number, value?: boolean) {
    if (value === undefined) {
      if (selectedIds.has(id)) selectedIds.delete(id); else selectedIds.add(id);
    } else {
      if (value) selectedIds.add(id); else selectedIds.delete(id);
    }
    selectedIds = new Set(selectedIds);
  }
  function clearSelection() { selectedIds = new Set(); }

  const STORAGE_KEY = 'avelonia_downloader_ui_v1';
  onMount(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const s = JSON.parse(raw);
        if (typeof s?.searchTerm === 'string') searchTerm = s.searchTerm;
        if (s?.filters && typeof s.filters === 'object') {
          filters.fileType = s.filters.fileType ?? '';
          filters.category = s.filters.category ?? '';
          filters.minSize = s.filters.minSize ?? '';
          filters.maxSize = s.filters.maxSize ?? '';
          filters.eta = s.filters.eta ?? '';
          filters.status = s.filters.status ?? '';
        }
        if (typeof s?.sortBy === 'string') sortBy = s.sortBy;
        if (s?.sortDirection === 'asc' || s?.sortDirection === 'desc') sortDirection = s.sortDirection;
        if (s?.statusGroup) statusGroup = s.statusGroup;
      }
    } catch {}
    // Keyboard shortcuts
    const keyHandler = (e: KeyboardEvent) => {
      const meta = e.ctrlKey || e.metaKey;
      if (meta && (e.key === 'a' || e.key === 'A')) {
        e.preventDefault();
        for (const d of filteredDownloads) selectedIds.add(d.id);
        selectedIds = new Set(selectedIds);
        return;
      }
      if (e.key === 'Escape') { clearSelection(); return; }
      if (e.key === 'Delete' || e.key === 'Backspace') { cancelSelected(); return; }
      if (e.key === 'Enter') { startSelected(); return; }
    };
    window.addEventListener('keydown', keyHandler);
    return () => { window.removeEventListener('keydown', keyHandler); };
  });
  $effect(() => {
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ searchTerm, filters, sortBy, sortDirection, statusGroup })
      );
    } catch {}
  });

  // Close actions dropdown when clicking outside
  $effect(() => {
    const onClick = (e: MouseEvent) => {
      if (!actionsOpen) return;
      const container = document.querySelector('.actions');
      if (container && !container.contains(e.target as Node)) {
        actionsOpen = false;
      }
    };
    window.addEventListener('click', onClick);
    return () => window.removeEventListener('click', onClick);
  });

  // Normalize human-readable sizes like "120 MB", "900KB", or raw numbers into bytes
  function toBytes(val: string | number | undefined | null): number {
    if (val === undefined || val === null) return 0;
    const s = String(val).trim();
    if (!s) return 0;
    const m = s.match(/^(\d+(?:\.\d+)?)\s*([kKmMgGtTpP]?[bB])?$/);
    if (!m) return parseFloat(s) || 0;
    const num = parseFloat(m[1]);
    const unit = (m[2] || 'B').toUpperCase();
    const map: Record<string, number> = {
      B: 1,
      KB: 1024,
      MB: 1024 ** 2,
      GB: 1024 ** 3,
      TB: 1024 ** 4,
      PB: 1024 ** 5
    };
    return num * (map[unit] ?? 1);
  }

  // Custom status weight for more intuitive ordering
  const statusWeight: Record<string, number> = {
    available: 1,
    pending: 2,
    queued: 3,
    downloading: 4,
    paused: 5,
    completed: 6,
    failed: 7
  };

  function sortDownloads(a: Download, b: Download) {
    let valA: string | number;
    let valB: string | number;

    switch (sortBy) {
      case 'name':
        valA = a.name.toLowerCase();
        valB = b.name.toLowerCase();
        break;
      case 'size':
        valA = toBytes(a.size);
        valB = toBytes(b.size);
        break;
      case 'status':
        valA = statusWeight[a.status?.toLowerCase?.()] ?? 999;
        valB = statusWeight[b.status?.toLowerCase?.()] ?? 999;
        break;
      case 'eta':
        valA = a.eta.toLowerCase();
        valB = b.eta.toLowerCase();
        break;
      case 'fileType':
        valA = a.fileType.toLowerCase();
        valB = b.fileType.toLowerCase();
        break;
      case 'category':
        valA = a.category.toLowerCase();
        valB = b.category.toLowerCase();
        break;
      default:
        valA = a.id;
        valB = b.id;
    }

    if (valA < valB) {
      return sortDirection === 'asc' ? -1 : 1;
    } else if (valA > valB) {
      return sortDirection === 'asc' ? 1 : -1;
    } else {
      return 0;
    }
  }
  function setSort(key: 'name'|'size'|'status'|'eta'|'fileType'|'category') {
    if (sortBy === key) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = key;
      sortDirection = 'asc';
    }
  }

  const filteredDownloads = $derived(
    $downloads.filter((download) => {
      const matchesSearchTerm = download.name
        .toLowerCase()
        .includes(debouncedSearchTerm.toLowerCase());
      const matchesFileType = filters.fileType
        ? download.fileType.toLowerCase().includes(filters.fileType.toLowerCase())
        : true;
      const matchesCategory = filters.category
        ? download.category.toLowerCase().includes(filters.category.toLowerCase())
        : true;

      // Support unit-aware numeric filtering like "50MB", "1.5 GB", "900KB"
      const downloadSizeBytes = toBytes(download.size);
      const minSizeBytes = filters.minSize !== '' ? toBytes(filters.minSize) : undefined;
      const maxSizeBytes = filters.maxSize !== '' ? toBytes(filters.maxSize) : undefined;

      const matchesMinSize = minSizeBytes !== undefined ? downloadSizeBytes >= minSizeBytes : true;
      const matchesMaxSize = maxSizeBytes !== undefined ? downloadSizeBytes <= maxSizeBytes : true;

      const matchesETA = filters.eta
        ? download.eta.toLowerCase().includes(filters.eta.toLowerCase())
        : true;
      const matchesStatus = filters.status
        ? download.status.toLowerCase() === filters.status.toLowerCase()
        : true;

      const matchesGroup = (() => {
        switch (statusGroup) {
          case 'available': return download.status === 'available';
          case 'completed': return download.status === 'completed';
          case 'failed': return download.status === 'failed';
          case 'active': return download.status === 'downloading' || download.status === 'pending' || download.status === 'queued';
          default: return true;
        }
      })();

      return (
        matchesSearchTerm &&
        matchesFileType &&
        matchesCategory &&
        matchesMinSize &&
        matchesMaxSize &&
        matchesETA &&
        matchesStatus &&
        matchesGroup
      );
    }).sort(sortDownloads)
  );

  const totalDownloads = $derived($downloads.length);
  const availableDownloads = $derived(filteredDownloads.length);
  const activeCount = $derived($downloads.filter(d => d.status === 'downloading' || d.status === 'pending' || d.status === 'queued').length);
  const completedCount = $derived($downloads.filter(d => d.status === 'completed').length);
  const failedCount = $derived($downloads.filter(d => d.status === 'failed').length);
  const selectedCompletedCount = $derived(filteredDownloads.filter(d => selectedIds.has(d.id) && d.status === 'completed').length);

  // Keep the select-all checkbox in sync (checked/indeterminate)
  $effect(() => {
    const filteredIds = new Set(filteredDownloads.map(d => d.id));
    let selectedInFilter = 0;
    for (const id of filteredIds) if (selectedIds.has(id)) selectedInFilter++;
    if (selectAllCheckbox) {
      const total = filteredIds.size;
      const allSelected = total > 0 && selectedInFilter === total;
      const noneSelected = selectedInFilter === 0;
      selectAllCheckbox.checked = allSelected;
      selectAllCheckbox.indeterminate = !allSelected && !noneSelected && total > 0;
    }
  });

  function formatBytes(bytes: number): string {
    if (!isFinite(bytes) || bytes <= 0) return '0 B';
    const units = ['B','KB','MB','GB','TB','PB'];
    let i = 0;
    while (bytes >= 1024 && i < units.length - 1) { bytes /= 1024; i++; }
    return `${bytes.toFixed(bytes >= 100 ? 0 : 1)} ${units[i]}`;
  }
  const filteredTotalBytes = $derived(filteredDownloads.reduce((sum, d) => sum + toBytes(d.size), 0));

  function handleClearFilters() {
    searchTerm = "";
    filters.fileType = "";
    filters.category = "";
    filters.minSize = "";
    filters.maxSize = "";
    filters.eta = "";
    filters.status = "";
  }

  function startAll() {
    const list = get(downloads);
    for (const d of list) {
      if (d.status === 'available' && d.downloadLink) startDownload(d.id);
    }
  }

  function cancelAllActive() {
    const list = get(downloads);
    for (const d of list) {
      if (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued') {
        cancelDownload(d.id);
      }
    }
  }

  function startAllFiltered() {
    for (const d of filteredDownloads) {
      if (d.status === 'available' && d.downloadLink) startDownload(d.id);
    }
  }
  function cancelAllFiltered() {
    for (const d of filteredDownloads) {
      if (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued') cancelDownload(d.id);
    }
  }
  function startSelected() {
    for (const d of filteredDownloads) {
      if (selectedIds.has(d.id) && d.status === 'available' && d.downloadLink) startDownload(d.id);
    }
  }
  function cancelSelected() {
    for (const d of filteredDownloads) {
      if (selectedIds.has(d.id) && (d.status === 'downloading' || d.status === 'pending' || d.status === 'queued')) cancelDownload(d.id);
    }
  }

  async function openSelectedCompleted() {
    const items = filteredDownloads.filter(d => selectedIds.has(d.id) && d.status === 'completed');
    for (const d of items) {
      try {
        const p = await getDownloadPath(d);
        if (p) await openPath(p);
      } catch {}
    }
  }

  async function showSelectedCompleted() {
    const items = filteredDownloads.filter(d => selectedIds.has(d.id) && d.status === 'completed');
    for (const d of items) {
      try {
        const p = await getDownloadPath(d);
        if (p) await revealItemInDir(p);
      } catch {}
    }
  }

  function retryFailedFiltered() {
    for (const d of filteredDownloads) {
      if (d.status === 'failed' && d.downloadLink) startDownload(d.id);
    }
  }
  function retryAllFailed() {
    const list = get(downloads);
    for (const d of list) {
      if (d.status === 'failed' && d.downloadLink) startDownload(d.id);
    }
  }

  // Range selection with Shift
  function toggleSelectRange(currentIndex: number, value: boolean) {
    if (lastSelectedIndex === null) {
      const id = filteredDownloads[currentIndex]?.id;
      if (id !== undefined) {
        if (value) selectedIds.add(id); else selectedIds.delete(id);
      }
    } else {
      const start = Math.min(lastSelectedIndex, currentIndex);
      const end = Math.max(lastSelectedIndex, currentIndex);
      for (let i = start; i <= end; i++) {
        const id = filteredDownloads[i]?.id;
        if (id !== undefined) {
          if (value) selectedIds.add(id); else selectedIds.delete(id);
        }
      }
    }
    selectedIds = new Set(selectedIds);
    lastSelectedIndex = currentIndex;
  }

  function toggleSelectWithIndex(id: number, value: boolean, index: number, shiftKey: boolean) {
    if (shiftKey) {
      toggleSelectRange(index, value);
      announce = `${selectedIds.size} selected`;
      setTimeout(() => (announce = ''), 1200);
      return;
    }
    if (value) selectedIds.add(id); else selectedIds.delete(id);
    selectedIds = new Set(selectedIds);
    lastSelectedIndex = index;
  }

  function invertSelection() {
    const next = new Set<number>();
    const visible = new Set(filteredDownloads.map(d => d.id));
    for (const id of visible) {
      if (!selectedIds.has(id)) next.add(id);
    }
    selectedIds = next;
  }

  // Utility actions
  function handleHeaderKey(e: KeyboardEvent, key: 'name'|'size'|'status'|'eta'|'fileType'|'category') {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setSort(key);
    }
  }

  async function copySelectedLinks() {
    const links = filteredDownloads.filter(d => selectedIds.has(d.id) && d.downloadLink).map(d => d.downloadLink);
    if (links.length === 0) return;
    try {
      await navigator.clipboard.writeText(links.join('\n'));
      announce = `Copied ${links.length} link${links.length === 1 ? '' : 's'} to clipboard`;
      setTimeout(() => (announce = ''), 2000);
    } catch {
      announce = 'Copy failed';
      setTimeout(() => (announce = ''), 2000);
    }
  }

  function exportFilteredCSV() {
    const rows = [
      ['ID','Name','Size','File Type','Category','ETA','Status','Link'],
      ...filteredDownloads.map(d => [
        String(d.id),
        d.name ?? '',
        String(d.size ?? ''),
        d.fileType ?? '',
        d.category ?? '',
        d.eta ?? '',
        d.status ?? '',
        d.downloadLink ?? ''
      ])
    ];
    const csv = rows.map(r => r.map((v) => {
      const s = String(v ?? '');
      return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
    }).join(',')).join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'downloads.csv';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    announce = 'Exported filtered list as CSV';
    setTimeout(() => (announce = ''), 2000);
  }
</script>

<div class="main-content">
  <div class="sr-only" aria-live="polite">{announce}</div>
  <div class="header-card">
    <h1>Downloader</h1>
    <p class="muted">Search, filter and manage app downloads.</p>
  </div>

  <section class="panel">
    <div class="header-section">
      <p>
        Showing {availableDownloads} / {totalDownloads}
        · Size: {formatBytes(filteredTotalBytes)}
        · Active: {activeCount} · Completed: {completedCount} · Failed: {failedCount}
      </p>
      <div class="search-filter-group">
        <div class="status-chips" role="group" aria-label="Quick status filters">
          <button class:active={statusGroup==='all'} onclick={() => statusGroup='all'} title="Show all">All</button>
          <button class:active={statusGroup==='available'} onclick={() => statusGroup='available'} title="Show available">Available</button>
          <button class:active={statusGroup==='active'} onclick={() => statusGroup='active'} title="Show active (downloading/pending/queued)">Active</button>
          <button class:active={statusGroup==='completed'} onclick={() => statusGroup='completed'} title="Show completed">Completed</button>
          <button class:active={statusGroup==='failed'} onclick={() => statusGroup='failed'} title="Show failed">Failed</button>
        </div>
        <input
          type="text"
          placeholder="Search downloads..."
          bind:value={searchTerm}
        />
        <div class="actions" onkeydown={(e) => { if (e.key === 'Escape') actionsOpen = false; }}>
          <button class="btn action-toggle" aria-haspopup="menu" aria-expanded={actionsOpen} onclick={() => actionsOpen = !actionsOpen}>
            Actions ▾
          </button>
          {#if actionsOpen}
            <div class="actions-menu" role="menu" tabindex="-1" onfocusout={(e) => { const r = (e.currentTarget as HTMLElement).contains(e.relatedTarget as Node); if (!r) actionsOpen = false; }}>
              <button role="menuitem" class="menu-item" onclick={() => { startAll(); actionsOpen = false; }}>Start All</button>
              <button role="menuitem" class="menu-item" onclick={() => { cancelAllActive(); actionsOpen = false; }}>Cancel Active</button>
              <hr />
              <button role="menuitem" class="menu-item" onclick={() => { startAllFiltered(); actionsOpen = false; }}>Start Filtered</button>
              <button role="menuitem" class="menu-item" onclick={() => { cancelAllFiltered(); actionsOpen = false; }}>Cancel Filtered</button>
              <hr />
              <button role="menuitem" class="menu-item" onclick={() => { startSelected(); actionsOpen = false; }} disabled={selectedIds.size === 0}>Start Selected</button>
              <button role="menuitem" class="menu-item" onclick={() => { cancelSelected(); actionsOpen = false; }} disabled={selectedIds.size === 0}>Cancel Selected</button>
              <hr />
              <button role="menuitem" class="menu-item" onclick={() => { copySelectedLinks(); actionsOpen = false; }} disabled={selectedIds.size === 0}>Copy Selected Links</button>
              <button role="menuitem" class="menu-item" onclick={() => { exportFilteredCSV(); actionsOpen = false; }}>Export CSV</button>
              <hr />
              <button role="menuitem" class="menu-item" onclick={() => { retryFailedFiltered(); actionsOpen = false; }} title="Retry failed items in current filter">Retry Failed (Filtered)</button>
              <button role="menuitem" class="menu-item" onclick={() => { retryAllFailed(); actionsOpen = false; }} title="Retry all failed items">Retry All Failed</button>
            </div>
          {/if}
        </div>
        <select bind:value={sortBy}>
          <option value="name">Name</option>
          <option value="size">Size</option>
          <option value="status">Status</option>
          <option value="eta">ETA</option>
          <option value="fileType">File Type</option>
          <option value="category">Category</option>
        </select>
        <button class="btn" onclick={() => sortDirection = sortDirection === 'asc' ? 'desc' : 'asc'} aria-label={`Sort direction: ${sortDirection === 'asc' ? 'Ascending' : 'Descending'}`}>
          {sortDirection === 'asc' ? 'Asc' : 'Desc'}
        </button>
        <button class="btn primary" onclick={() => (showFilters = !showFilters)}>Filter</button>
        <button class="btn" onclick={() => showHelp = true} title="View keyboard shortcuts and tips">Shortcuts</button>
      </div>
    </div>

    <FilterPanel
      bind:searchTerm={searchTerm}
      bind:showFilters={showFilters}
      bind:filters={filters}
      onclearFilters={handleClearFilters}
    />
  </section>

  <div class="program-list panel">
    {#if selectedIds.size > 0}
      <div class="selection-bar">
        <span><strong>{selectedIds.size}</strong> selected</span>
        <div class="spacer" />
        <button class="px-3 py-1 rounded-md border-none bg-green-700 text-white cursor-pointer hover:bg-green-800" onclick={startSelected} title="Start selected">Start</button>
        <button class="px-3 py-1 rounded-md border-none bg-red-700 text-white cursor-pointer hover:bg-red-800" onclick={cancelSelected} title="Cancel selected">Cancel</button>
        <button class="px-3 py-1 rounded-md border border-gray-700 bg-gray-800 text-white cursor-pointer hover:bg-gray-700" onclick={copySelectedLinks} title="Copy selected links">Copy Links</button>
        <button class="px-3 py-1 rounded-md border border-gray-700 bg-gray-800 text-white cursor-pointer hover:bg-gray-700" onclick={exportFilteredCSV} title="Export current list as CSV">Export CSV</button>
        <button class="px-3 py-1 rounded-md border border-gray-700 bg-gray-800 text-white cursor-pointer hover:bg-gray-700" onclick={openSelectedCompleted} title="Open selected completed files" disabled={selectedCompletedCount === 0}>Open</button>
        <button class="px-3 py-1 rounded-md border border-gray-700 bg-gray-800 text-white cursor-pointer hover:bg-gray-700" onclick={showSelectedCompleted} title="Show selected completed in folder" disabled={selectedCompletedCount === 0}>Show</button>
        <button class="px-3 py-1 rounded-md border border-gray-700 bg-gray-800 text-white cursor-pointer hover:bg-gray-700" onclick={invertSelection} title="Invert selection">Invert</button>
        <button class="px-3 py-1 rounded-md border-none bg-gray-600 text-white cursor-pointer hover:bg-gray-700" onclick={clearSelection} title="Clear selection">Clear</button>
      </div>
    {/if}
    <div class="program-list-header">
      <span>
        <input bind:this={selectAllCheckbox} type="checkbox" onchange={(e) => {
          const check = (e.target as HTMLInputElement).checked;
          for (const d of filteredDownloads) toggleSelect(d.id, check);
        }} title="Select/Deselect all filtered" />
      </span>
      <span role="button" tabindex="0" aria-sort={sortBy==='name' ? (sortDirection==='asc' ? 'ascending' : 'descending') : 'none'} onclick={() => setSort('name')} onkeydown={(e) => handleHeaderKey(e, 'name')}>Name {sortBy==='name' ? (sortDirection==='asc' ? '▲' : '▼') : ''}</span>
      <span role="button" tabindex="0" aria-sort={sortBy==='size' ? (sortDirection==='asc' ? 'ascending' : 'descending') : 'none'} onclick={() => setSort('size')} onkeydown={(e) => handleHeaderKey(e, 'size')}>Size {sortBy==='size' ? (sortDirection==='asc' ? '▲' : '▼') : ''}</span>
      <span role="button" tabindex="0" aria-sort={sortBy==='fileType' ? (sortDirection==='asc' ? 'ascending' : 'descending') : 'none'} onclick={() => setSort('fileType')} onkeydown={(e) => handleHeaderKey(e, 'fileType')}>File Type {sortBy==='fileType' ? (sortDirection==='asc' ? '▲' : '▼') : ''}</span>
      <span role="button" tabindex="0" aria-sort={sortBy==='category' ? (sortDirection==='asc' ? 'ascending' : 'descending') : 'none'} onclick={() => setSort('category')} onkeydown={(e) => handleHeaderKey(e, 'category')}>Category {sortBy==='category' ? (sortDirection==='asc' ? '▲' : '▼') : ''}</span>
      <span role="button" tabindex="0" aria-sort={sortBy==='eta' ? (sortDirection==='asc' ? 'ascending' : 'descending') : 'none'} onclick={() => setSort('eta')} onkeydown={(e) => handleHeaderKey(e, 'eta')}>ETA {sortBy==='eta' ? (sortDirection==='asc' ? '▲' : '▼') : ''}</span>
      <span role="button" tabindex="0" aria-sort={sortBy==='status' ? (sortDirection==='asc' ? 'ascending' : 'descending') : 'none'} onclick={() => setSort('status')} onkeydown={(e) => handleHeaderKey(e, 'status')}>Status {sortBy==='status' ? (sortDirection==='asc' ? '▲' : '▼') : ''}</span>
    </div>
    {#each filteredDownloads as download, i (download.id)}
      <DownloadItem
        {download}
        {startDownload}
        {cancelDownload}
        selected={isSelected(download.id)}
        ontoggleSelect={(e) => toggleSelectWithIndex(download.id, e.detail?.checked ?? false, i, !!e.detail?.shiftKey)}
      />
    {/each}
    {#if filteredDownloads.length === 0}
      <div class="no-results">
        <p>No downloads match current filters.</p>
        <button class="clear-btn" onclick={() => { handleClearFilters(); statusGroup = 'all'; }}>Clear filters</button>
      </div>
    {/if}
  </div>

  {#if showHelp}
    <div class="modal" role="dialog" aria-modal="true" aria-label="Downloader shortcuts" onclick={() => (showHelp = false)}>
      <div class="modal-card" onclick={(e) => e.stopPropagation()}>
        <h3>Shortcuts</h3>
        <ul>
          <li><strong>Ctrl/Cmd + A:</strong> Select all filtered</li>
          <li><strong>Shift + Click:</strong> Range select</li>
          <li><strong>Enter:</strong> Start/Cancel focused row</li>
          <li><strong>Delete/Backspace:</strong> Cancel selected active</li>
          <li><strong>Esc:</strong> Clear selection</li>
        </ul>
        <button class="btn" onclick={() => (showHelp = false)}>Close</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .header-section {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  .header-section p {
    font-size: 1.1em;
    font-weight: bold;
    margin: 0;
  }

  .search-filter-group {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .status-chips { display: inline-flex; gap: 6px; align-items: center; }
  .status-chips > button {
    padding: 6px 8px;
    border-radius: 999px;
    border: 1px solid var(--avelonia-border);
    background: #1b1c1f;
    color: #fff;
    cursor: pointer;
    font-size: 12px;
  }
  .status-chips > button.active { background: var(--avelonia-purple); border-color: var(--avelonia-purple); }

  .header-section input {
    padding: 4px 8px;
    height: 32px;
    font-size: 12px;
    border-radius: 6px;
    border: 1px solid #333;
    background-color: #2a2a2a;
    color: #ffffff;
    width: 240px;
  }

  .search-filter-group select {
    height: 32px;
    padding: 4px 8px;
    font-size: 12px;
    border-radius: 6px;
    border: 1px solid #333;
    background-color: #2a2a2a;
    color: #ffffff;
  }

  .btn {
    height: 32px;
    padding: 4px 10px;
    font-size: 12px;
    border-radius: 6px;
    border: 1px solid var(--avelonia-border);
    background: #1b1c1f;
    color: #fff;
    cursor: pointer;
  }
  .btn.primary { background: var(--avelonia-purple); border-color: var(--avelonia-purple); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .actions { position: relative; }
  .action-toggle { min-width: 88px; }
  .actions-menu {
    position: absolute;
    top: 36px;
    right: 0;
    min-width: 200px;
    background: #17181b;
    border: 1px solid var(--avelonia-border);
    border-radius: 8px;
    box-shadow: 0 6px 24px rgba(0,0,0,0.35);
    padding: 6px;
    z-index: 10;
  }
  .actions-menu .menu-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 8px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: #fff;
    cursor: pointer;
  }
  .actions-menu .menu-item:hover { background: rgba(255,255,255,0.06); }
  .actions-menu hr { border: none; border-top: 1px solid var(--avelonia-border); margin: 6px 0; }

  .program-list {
    border: none;
    border-radius: 5px;
    overflow: hidden;
    background-color: rgba(0, 0, 0, 0.2);
    flex: 1;
    overflow: auto;
    contain: content;
  }

  .selection-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--avelonia-border);
    background: rgba(20, 20, 25, 0.65);
    backdrop-filter: saturate(1.2) blur(6px);
  }
  .selection-bar .spacer { flex: 1; }

  .program-list-header {
    display: grid;
    grid-template-columns: 40px 1.5fr 1fr 1fr 1fr 1fr 1fr; /* Icon/Select, Name, Size, File Type, Category, ETA, Status */
    padding: 10px;
    align-items: center;
    height: 46px;
    box-sizing: border-box;
    position: sticky;
    top: 0;
    z-index: 1;
    background: rgba(23, 24, 27, 0.8);
    backdrop-filter: saturate(1.2) blur(6px);
  }

  .program-list-header span:nth-child(1) {
    justify-self: center;
  }

  .program-list-header span:nth-child(2),
  .program-list-header span:nth-child(3) {
    justify-self: start;
  }

  .program-list-header span:nth-child(4),
  .program-list-header span:nth-child(5),
  .program-list-header span:nth-child(6),
  .program-list-header span:nth-child(7) {
    justify-self: end;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .no-results { padding: 24px; text-align: center; color: var(--avelonia-text-muted); }
  .no-results .clear-btn {
    margin-top: 8px;
    padding: 6px 10px;
    border-radius: 6px;
    border: 1px solid var(--avelonia-border);
    background: #1b1c1f;
    color: #fff;
    cursor: pointer;
  }

  /* Simple modal */
  .modal {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }
  .modal-card {
    background: #17181b;
    border: 1px solid var(--avelonia-border);
    border-radius: 10px;
    padding: 16px;
    color: var(--avelonia-text);
    width: min(420px, 90vw);
  }
  .modal-card h3 { margin-top: 0; margin-bottom: 8px; }
  .modal-card ul { margin: 0 0 12px 18px; padding: 0; }
  .modal-card li { margin: 6px 0; }
</style>


