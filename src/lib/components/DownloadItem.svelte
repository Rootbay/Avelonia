<script lang="ts">
  import type { Download } from '$lib/downloadManager';
  import { getDownloadPath } from '$lib/downloadManager';
  import { downloadDir, join } from '@tauri-apps/api/path';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { createEventDispatcher } from 'svelte';

  export let download: Download;
  export let startDownload: (id: number) => void;
  export let cancelDownload: (id: number) => void;
  export let selected: boolean = false;

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
      handleClick();
    }
  }

  $: actionLabel = (download.status === 'downloading' || download.status === 'pending' || download.status === 'queued')
    ? 'Cancel download'
    : 'Start download';

  async function ensurePath(): Promise<string | null> {
    return await getDownloadPath(download);
  }

  async function openFile(e: MouseEvent) {
    e.stopPropagation();
    const p = await ensurePath();
    if (p) {
      try { await openPath(p); } catch (err) { console.warn('openPath failed', err); }
    }
  }

  async function showInFolder(e: MouseEvent) {
    e.stopPropagation();
    const p = await ensurePath();
    if (p) {
      try { await revealItemInDir(p); } catch (err) { console.warn('revealItemInDir failed', err); }
    }
  }

  function retry(e: MouseEvent) {
    e.stopPropagation();
    startDownload(download.id);
  }

  async function copyLink(e: MouseEvent) {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(download.downloadLink);
    } catch (err) {
      console.warn('clipboard failed', err);
    }
  }
</script>

<div
  role="button"
  tabindex="0"
  class="program-list-item"
  onclick={handleClick}
  onkeydown={handleKeydown}
  aria-label={actionLabel}
  title={actionLabel}
  data-status={download.status}
>
  <span>
    <input
      type="checkbox"
      checked={selected}
      onchange={(e) => {
        e.stopPropagation();
        const target = e.target as HTMLInputElement;
        dispatch('toggleSelect', { checked: target.checked, shiftKey: e.shiftKey });
      }}
      title="Select download"
      class="row-check"
    />
    <span class="avatar" aria-hidden="true">{(download.name?.[0] || '').toUpperCase()}</span>
  </span>
  <span>{download.name}</span>
  <span>{download.size}</span>
  <span>{download.fileType}</span>
  <span>{download.category}</span>
  <span>{download.eta}</span>
  <span class="actions">
    <span class="status">
      {download.status}
      {#if download.progress < 0}
        <span class="muted"> (preparing)</span>
      {/if}
      {#if download.status === 'downloading' && download.speed}
        <span class="muted"> · {download.speed}</span>
      {/if}
    </span>
    <button class="btn" onclick={copyLink} title="Copy download link">Copy</button>
    {#if download.status === 'completed'}
      <button class="btn" onclick={openFile} title="Open file">Open</button>
      <button class="btn" onclick={showInFolder} title="Show in folder">Show</button>
    {:else if download.status === 'failed'}
      <button class="btn" onclick={retry} title="Retry download">Retry</button>
    {/if}
  </span>
  {#if download.progress >= 0 && (download.status === 'downloading' || download.status === 'pending')}
    <div class="progress" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.floor(download.progress)} aria-label="Download progress">
      <div class="bar" style={`width: ${Math.min(100, Math.max(0, download.progress)).toFixed(2)}%`}></div>
    </div>
  {/if}
</div>

<style>
  .program-list-item {
    display: grid;
    grid-template-columns: 40px 1.5fr 1fr 1fr 1fr 1fr 1fr;
    align-items: center;
    padding: 10px;
    cursor: pointer;
    content-visibility: auto;
    contain-intrinsic-size: 0 46px;
    border-bottom: 1px solid var(--avelonia-border);
    position: relative;
  }
  .avatar {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--avelonia-border);
    color: var(--avelonia-text);
    font-size: 0.8rem;
  }
  .actions { justify-self: end; display: flex; align-items: center; gap: 8px; }
  .status { display: inline-flex; gap: 6px; align-items: baseline; }
  .muted { color: var(--avelonia-text-muted); }
  .btn { padding: 3px 8px; border-radius: 6px; border: 1px solid var(--avelonia-border); background: #1b1c1f; color: #fff; cursor: pointer; }
  .row-check { margin-right: 8px; accent-color: var(--avelonia-purple); }

  .progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    background: rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }
  .progress .bar {
    height: 100%;
    background: var(--avelonia-blue);
  }

  /* Subtle status accent */
  .program-list-item[data-status="downloading"] { border-left: 3px solid var(--avelonia-blue); }
  .program-list-item[data-status="pending"],
  .program-list-item[data-status="queued"] { border-left: 3px solid var(--avelonia-warning); }
  .program-list-item[data-status="completed"] { border-left: 3px solid var(--avelonia-success); }
  .program-list-item[data-status="failed"] { border-left: 3px solid var(--avelonia-danger); }
</style>



