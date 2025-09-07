<script lang="ts">
  import type { Download } from '$lib/downloadManager';

  export let download: Download;
  export let startDownload: (id: number) => void;
  export let cancelDownload: (id: number) => void;

  function handleClick() {
    console.log(`handleClick called for download ID: ${download.id}, status: ${download.status}`);
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

  $: progressStyle = `linear-gradient(to right, rgba(0, 255, 0, 0.2) ${download.progress}%, transparent ${download.progress}%)`;
</script>

<div
  role="button"
  tabindex="0"
  class="program-list-item"
  onclick={handleClick}
  onkeydown={handleKeydown}
  style:background={progressStyle}
>
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
  </span>
</div>

<style>
  .program-list-item {
    cursor: pointer;
  }
</style>
