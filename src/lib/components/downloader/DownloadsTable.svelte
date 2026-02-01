<script lang="ts">
  import {
    Table,
    TableHeader,
    TableRow,
    TableHead,
    TableBody,
    TableCell,
  } from '$lib/components/ui/table';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { ArrowUpDown } from '@lucide/svelte';
  import DownloadItem from '$lib/components/DownloadItem.svelte';
  import type { Download } from '$lib/downloadManager';
  import { SvelteSet } from 'svelte/reactivity';

  let {
    downloads,
    initialLoading,
    selectedIds = new SvelteSet<number>(),
    sortBy = $bindable('name'),
    sortDirection = $bindable('asc'),
    onStart,
    onCancel,
  } = $props<{
    downloads: Download[];
    initialLoading: boolean;
    selectedIds: SvelteSet<number>;
    sortBy: string;
    sortDirection: 'asc' | 'desc';
    onStart: (id: number) => void;
    onCancel: (id: number) => Promise<void>;
  }>();

  let tableEl = $state<HTMLTableElement | null>(null);
  let scrollEl = $state<HTMLDivElement | null>(null);
  let maxListHeight = $state(0);
  let minListHeight = $state(0);

  const DOWNLOAD_ROW_PX = 56;
  const DOWNLOAD_MAX_DOM = 600;
  const VIEW_CHUNK = 30;

  let downloadsStart = $state(0);
  let downloadsVisible = $state(10);
  let lastSelectedIndex = $state<number | null>(null);
  let _scrollTick = false;

  const windowedDownloads = $derived(
    downloads.slice(downloadsStart, Math.min(downloadsVisible, downloads.length))
  );

  function setSort(key: string) {
    if (sortBy === key) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = key;
      sortDirection = 'asc';
    }
  }

  function handleHeaderKey(e: KeyboardEvent, key: string) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setSort(key);
    }
  }

  function updateVisibleWindow(el: HTMLElement) {
    const total = downloads.length;
    if (total === 0) {
      downloadsStart = 0;
      downloadsVisible = 0;
      return;
    }

    const scrollTop = Math.max(0, el.scrollTop);
    const clientHeight = Math.max(1, el.clientHeight);
    const firstVisibleIndex = Math.max(0, Math.floor(scrollTop / DOWNLOAD_ROW_PX));
    const viewportRows = Math.ceil(clientHeight / DOWNLOAD_ROW_PX);

    const start = Math.max(0, firstVisibleIndex - VIEW_CHUNK);
    const desiredEnd = Math.min(total, firstVisibleIndex + viewportRows + VIEW_CHUNK);
    const maxEnd = Math.min(total, start + DOWNLOAD_MAX_DOM);
    const end = Math.min(maxEnd, Math.max(desiredEnd, start + VIEW_CHUNK));

    downloadsStart = start;
    downloadsVisible = Math.min(total, Math.max(start + VIEW_CHUNK, end));
  }

  function onScroll(event: Event) {
    if (_scrollTick) return;
    _scrollTick = true;
    const el = (event.currentTarget as HTMLElement) || scrollEl;
    if (!el) {
      _scrollTick = false;
      return;
    }
    requestAnimationFrame(() => {
      updateVisibleWindow(el);
      _scrollTick = false;
    });
  }

  function recomputeMaxHeight() {
    try {
      const header = tableEl?.querySelector('thead') as HTMLElement | null;
      const firstRow = tableEl?.querySelector('tbody tr') as HTMLElement | null;
      const headerH = header ? header.getBoundingClientRect().height : 44;
      const measured = firstRow ? firstRow.getBoundingClientRect().height : 48;
      const rowH = Math.max(36, Math.min(80, measured));
      maxListHeight = Math.ceil(headerH + rowH * 10 + 6);
      minListHeight = Math.ceil(headerH + rowH * 5 + 6);
    } catch {
      maxListHeight = 0;
      minListHeight = 0;
    }
  }

  $effect(() => {
    void downloads.length;
    setTimeout(recomputeMaxHeight, 0);
  });

  $effect(() => {
    if (scrollEl) updateVisibleWindow(scrollEl);
  });

  function toggleSelect(id: number, value: boolean, index: number, shiftKey: boolean) {
    if (shiftKey && lastSelectedIndex !== null) {
      const start = Math.min(lastSelectedIndex, index);
      const end = Math.max(lastSelectedIndex, index);
      for (let i = start; i <= end; i++) {
        const d = downloads[i];
        if (d) {
          if (value) selectedIds.add(d.id);
          else selectedIds.delete(d.id);
        }
      }
    } else {
      if (value) selectedIds.add(id);
      else selectedIds.delete(id);
    }
    lastSelectedIndex = index;
  }
</script>

<div
  class="overflow-auto"
  style:max-height={`${maxListHeight || ''}px`}
  style:min-height={`${minListHeight || ''}px`}
  bind:this={scrollEl}
  onscroll={onScroll}
>
  <Table ref={tableEl} class="min-w-240 table-fixed">
    <TableHeader>
      <TableRow>
        <TableHead class="w-20">
          <span class="flex justify-center">
            <input
              type="checkbox"
              class="h-4 w-4 rounded border-border bg-background text-primary focus:ring-ring"
              checked={downloads.length > 0 &&
                downloads.every((d: Download) => selectedIds.has(d.id))}
              onchange={(e) => {
                const check = (e.target as HTMLInputElement).checked;
                if (check) {
                  for (const d of downloads) selectedIds.add(d.id);
                } else {
                  selectedIds.clear();
                }
              }}
            />
          </span>
        </TableHead>
        {#each [{ key: 'name', label: 'Name', width: 'w-[35%]' }, { key: 'size', label: 'Size', width: 'w-28' }, { key: 'fileType', label: 'Type', width: 'w-24' }, { key: 'category', label: 'Category', width: 'w-32' }, { key: 'eta', label: 'ETA', width: 'w-28' }, { key: 'status', label: 'Status', width: 'w-52 pl-8' }] as col (col.key)}
          <TableHead class={col.width}>
            <button
              class="flex items-center gap-1 text-left transition hover:text-foreground focus-visible:outline-none"
              onclick={() => setSort(col.key)}
              onkeydown={(event) => handleHeaderKey(event, col.key)}
            >
              <span>{col.label}</span>
              {#if sortBy === col.key}<ArrowUpDown
                  class={`size-3 ${sortDirection === 'asc' ? 'rotate-180' : ''}`}
                />{/if}
            </button>
          </TableHead>
        {/each}
      </TableRow>
    </TableHeader>
    <TableBody>
      {#if initialLoading}
        {#each Array.from({ length: 6 }) as _, ii (ii)}
          <TableRow class="border-0!" aria-hidden="true">
            <TableCell class="w-15">
              <div class="flex items-center gap-2">
                <Skeleton class="h-5 w-5 rounded-md" aria-hidden="true" />
                <Skeleton class="h-8 w-8 rounded-md" aria-hidden="true" />
              </div>
            </TableCell>
            <TableCell>
              <div class="flex flex-col gap-2">
                <Skeleton class="h-3 w-2/3" aria-hidden="true" />
                <Skeleton class="h-3 w-1/3" aria-hidden="true" />
              </div>
            </TableCell>
            <TableCell class="hidden md:table-cell"><Skeleton class="h-3 w-10" /></TableCell>
            <TableCell class="hidden md:table-cell"><Skeleton class="h-3 w-12" /></TableCell>
            <TableCell class="hidden md:table-cell"><Skeleton class="h-3 w-20" /></TableCell>
            <TableCell class="hidden md:table-cell"><Skeleton class="h-3 w-12" /></TableCell>
            <TableCell class="w-45 pl-6 sm:pl-8"><Skeleton class="h-3 w-16" /></TableCell>
          </TableRow>
        {/each}
      {:else}
        {#if downloadsStart > 0}
          <tr aria-hidden="true">
            <td
              colspan="7"
              style={`height:${downloadsStart * DOWNLOAD_ROW_PX}px; padding:0; border:0;`}
            ></td>
          </tr>
        {/if}
        {#each windowedDownloads as download, i (download.id)}
          <DownloadItem
            {download}
            startDownload={onStart}
            cancelDownload={onCancel}
            selected={selectedIds.has(download.id)}
            onToggleSelect={(payload) =>
              toggleSelect(download.id, payload.checked, downloadsStart + i, payload.shiftKey)}
          />
        {/each}
        {#if downloads.length === 0}
          <TableRow>
            <TableCell colspan={7} class="py-12 text-center text-sm text-muted-foreground">
              No downloads match the current filters.
            </TableCell>
          </TableRow>
        {/if}
      {/if}
    </TableBody>
  </Table>
</div>
