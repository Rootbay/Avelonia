<script lang="ts">
  import type { Download } from '$lib/downloadManager';

  export let download: Download;
  export let selectedDownloads: number[];
  export let startDownload: (id: number) => void;
  export let pauseDownload: (id: number) => void;
  export let resumeDownload: (id: number) => void;
  export let cancelDownload: (id: number) => void;
  export let retryDownload: (id: number) => void;

  function toggleSelect() {
    if (selectedDownloads.includes(download.id)) {
      selectedDownloads = selectedDownloads.filter((id) => id !== download.id);
    } else {
      selectedDownloads = [...selectedDownloads, download.id];
    }
  }

  function handlePrimaryAction() {
    if (download.status === 'available' || download.status === 'queued' || download.status === 'failed') {
      startDownload(download.id);
    } else if (download.status === 'paused') {
      resumeDownload(download.id);
    } else if (download.status === 'downloading' || download.status === 'pending') {
      pauseDownload(download.id);
    }
  }
</script>

<div class="program-list-item {selectedDownloads.includes(download.id) ? 'selected' : ''}" onclick={toggleSelect}>
  <span>
    <!-- Placeholder icon -->
    📦
  </span>
  <span>{download.name}</span>
  <span>{download.size}</span>
  <span>{download.fileType}</span>
  <span>{download.category}</span>
  <span>{download.eta}</span>
  <span style="justify-self: end; display: flex; align-items: center; gap: 8px;">
    <span>{download.status}</span>
    <div class="download-actions" onclick={(e) => e.stopPropagation()}>
      <button class="action-button" onclick={handlePrimaryAction} disabled={download.status === 'completed'}>
        {#if download.status === 'downloading' || download.status === 'pending'}
          Pause
        {:else if download.status === 'paused'}
          Resume
        {:else}
          Start
        {/if}
      </button>
      <button class="action-button" onclick={() => cancelDownload(download.id)} disabled={download.status === 'available'}>
        Cancel
      </button>
      {#if download.status === 'failed'}
        <button class="action-button" onclick={() => retryDownload(download.id)}>Retry</button>
      {/if}
    </div>
  </span>
</div>

<style>
  .action-button {
    padding: 4px 8px;
    border-radius: 4px;
    border: 1px solid #444;
    background: #2a2a2a;
    color: #fff;
    cursor: pointer;
  }
  .action-button:hover:enabled {
    background: #3a3a3a;
  }
</style>

