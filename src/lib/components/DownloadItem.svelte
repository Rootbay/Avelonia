<script lang="ts">
  import type { Download } from '$lib/downloadManager';
  import { getDownloadPath } from '$lib/downloadManager';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { createEventDispatcher } from 'svelte';
  import { Button } from '$lib/components/ui/button';

  export let download: Download;
  export let startDownload: (id: number) => void;
  export let cancelDownload: (id: number) => void;
  export let selected = false;

  const dispatch = createEventDispatcher();

  function handleClick() {
    const downloading = download.status === 'downloading' || download.status === 'pending' || download.status === 'queued';
    if (downloading) {
      cancelDownload(download.id);
    } else {
      startDownload(download.id);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleClick();
    }
  }

  $: actionLabel = (download.status === 'downloading' || download.status === 'pending' || download.status === 'queued')
    ? 'Cancel download'
    : 'Start download';

  const statusTone = $derived(() => {
    switch (download.status) {
      case 'completed':
        return 'text-emerald-400';
      case 'failed':
        return 'text-destructive';
      case 'downloading':
      case 'pending':
      case 'queued':
        return 'text-primary';
      default:
        return 'text-muted-foreground';
    }
  });

  async function ensurePath(): Promise<string | null> {
    return await getDownloadPath(download);
  }

  async function openFile(event: MouseEvent) {
    event.stopPropagation();
    const path = await ensurePath();
    if (path) {
      try {
        await openPath(path);
      } catch (error) {
        console.warn('openPath failed', error);
      }
    }
  }

  async function showInFolder(event: MouseEvent) {
    event.stopPropagation();
    const path = await ensurePath();
    if (path) {
      try {
        await revealItemInDir(path);
      } catch (error) {
        console.warn('revealItemInDir failed', error);
      }
    }
  }

  function retry(event: MouseEvent) {
    event.stopPropagation();
    startDownload(download.id);
  }

  async function copyLink(event: MouseEvent) {
    event.stopPropagation();
    try {
      await navigator.clipboard.writeText(download.downloadLink);
    } catch (error) {
      console.warn('clipboard failed', error);
    }
  }
</script>

<div
  role="button"
  tabindex="0"
  class="relative grid gap-4 border-l-2 border-transparent px-4 py-3 text-sm transition-colors duration-150 hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring md:grid-cols-[auto,minmax(220px,1fr),repeat(4,minmax(120px,0.6fr)),minmax(160px,0.8fr)] md:items-center data-[status=downloading]:border-primary data-[status=pending]:border-primary/70 data-[status=queued]:border-primary/70 data-[status=completed]:border-emerald-500 data-[status=failed]:border-destructive"
  data-status={download.status}
  on:click={handleClick}
  on:keydown={handleKeydown}
  aria-label={actionLabel}
  title={actionLabel}
>
  <div class="flex items-center gap-3">
    <input
      type="checkbox"
      checked={selected}
      class="h-4 w-4 rounded border-border bg-background text-primary focus:ring-ring"
      on:change={(event) => {
        event.stopPropagation();
        const target = event.target as HTMLInputElement;
        dispatch('toggleSelect', { checked: target.checked, shiftKey: event.shiftKey });
      }}
      title="Select download"
    />
    <span class="flex size-8 items-center justify-center rounded-md border border-border bg-muted text-sm font-semibold uppercase text-muted-foreground">
      {(download.name?.[0] ?? '?')}
    </span>
  </div>

  <div class="flex flex-col gap-1 md:col-start-2">
    <span class="line-clamp-1 font-medium text-foreground">{download.name}</span>
    <div class="flex flex-wrap gap-3 text-xs text-muted-foreground md:hidden">
      <span>{download.size}</span>
      {#if download.fileType}<span>- {download.fileType}</span>{/if}
      {#if download.category}<span>- {download.category}</span>{/if}
      {#if download.eta}<span>- {download.eta}</span>{/if}
    </div>
  </div>

  <span class="hidden text-sm text-muted-foreground md:block">{download.size}</span>
  <span class="hidden text-sm text-muted-foreground md:block">{download.fileType}</span>
  <span class="hidden text-sm text-muted-foreground md:block">{download.category}</span>
  <span class="hidden text-sm text-muted-foreground md:block">{download.eta}</span>

  <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-end">
    <span class={`text-xs font-semibold uppercase tracking-wide ${statusTone}`}>
      {download.status}
      {#if download.progress < 0}
        <span class="ml-1 text-muted-foreground">(preparing)</span>
      {/if}
      {#if download.status === 'downloading' && download.speed}
        <span class="ml-1 text-muted-foreground">- {download.speed}</span>
      {/if}
    </span>
    <div class="flex flex-wrap items-center gap-2">
      <Button type="button" variant="ghost" size="sm" on:click={copyLink}>Copy</Button>
      {#if download.status === 'completed'}
        <Button type="button" variant="ghost" size="sm" on:click={openFile}>Open</Button>
        <Button type="button" variant="ghost" size="sm" on:click={showInFolder}>Show</Button>
      {:else if download.status === 'failed'}
        <Button type="button" variant="ghost" size="sm" on:click={retry}>Retry</Button>
      {/if}
    </div>
  </div>

  {#if download.progress >= 0 && (download.status === 'downloading' || download.status === 'pending')}
    <div
      class="absolute inset-x-0 bottom-0 h-1 overflow-hidden rounded-full bg-muted"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.floor(download.progress)}
      aria-label="Download progress"
    >
      <div
        class="h-full bg-primary transition-[width]"
        style={`width: ${Math.min(100, Math.max(0, download.progress)).toFixed(2)}%`}
      />
    </div>
  {/if}
</div>

