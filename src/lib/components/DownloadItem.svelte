<script lang="ts">
  import type { Download } from '$lib/downloadManager';
  import { getDownloadPath } from '$lib/downloadManager';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Progress } from '$lib/components/ui/progress';
  import { TableRow, TableCell } from '$lib/components/ui/table';
  import { toast } from '$lib/components/ui/sonner';

  const { download, selected, onToggleSelect, startDownload, cancelDownload } = $props<{
    download: Download;
    selected: boolean;
    onToggleSelect?: (payload: { checked: boolean; shiftKey: boolean }) => void;
    startDownload: (id: number) => void;
    cancelDownload: (id: number) => void;
  }>();

  const downloading = $derived(
    download.status === 'downloading' ||
      download.status === 'pending' ||
      download.status === 'queued'
  );

  const actionLabel = $derived(downloading ? 'Cancel download' : 'Start download');

  const statusTone = $derived.by(() => {
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

  function handleClick() {
    if (downloading) {
      cancelDownload(download.id);
      toast.success(`Canceled ${download.name ?? 'download'}`);
    } else {
      startDownload(download.id);
      toast.success(`Queued ${download.name ?? 'download'}`);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleClick();
    }
  }

  async function ensurePath(): Promise<string | null> {
    return await getDownloadPath(download);
  }

  async function openFile(event: MouseEvent) {
    event.stopPropagation();
    const path = await ensurePath();
    if (!path) {
      toast.error('File path is not available yet');
      return;
    }
    try {
      await openPath(path);
      toast.success(`Opened ${download.name ?? 'file'}`);
    } catch (error) {
      console.warn('openPath failed', error);
      toast.error('Unable to open file');
    }
  }

  async function showInFolder(event: MouseEvent) {
    event.stopPropagation();
    const path = await ensurePath();
    if (!path) {
      toast.error('File path is not available yet');
      return;
    }
    try {
      await revealItemInDir(path);
      toast.success(`Showing ${download.name ?? 'file'} in folder`);
    } catch (error) {
      console.warn('revealItemInDir failed', error);
      toast.error('Unable to open containing folder');
    }
  }

  function retry(event: MouseEvent) {
    event.stopPropagation();
    startDownload(download.id);
    toast.success(`Retrying ${download.name ?? 'download'}`);
  }

  async function copyLink(event: MouseEvent) {
    event.stopPropagation();
    if (!download.downloadLink) {
      toast.error('No download link available');
      return;
    }
    try {
      await navigator.clipboard.writeText(download.downloadLink);
      toast.success('Copied download link');
    } catch (error) {
      console.warn('clipboard failed', error);
      toast.error('Unable to copy link');
    }
  }
</script>

<TableRow
  data-status={download.status}
  onclick={handleClick}
  onkeydown={handleKeydown}
  aria-label={actionLabel}
  title={actionLabel}
  class="data-[status=downloading]:border-l-2 data-[status=downloading]:border-primary
         data-[status=pending]:border-l-2 data-[status=pending]:border-primary/70
         data-[status=queued]:border-l-2 data-[status=queued]:border-primary/70
         data-[status=completed]:border-l-2 data-[status=completed]:border-emerald-500
         data-[status=failed]:border-l-2 data-[status=failed]:border-destructive"
>
  <TableCell class="w-[60px]">
    <div class="flex items-center gap-2">
      <Checkbox
        checked={selected}
        onchange={(event: Event) => {
          event.stopPropagation();
          const target = event.target as HTMLInputElement;
          onToggleSelect?.({ checked: target.checked, shiftKey: (event as MouseEvent).shiftKey });
        }}
        title="Select download"
      />
      <span
        class="flex size-8 items-center justify-center rounded-md border bg-muted text-sm font-semibold uppercase text-muted-foreground"
      >
        {download.name?.[0] ?? '?'}
      </span>
    </div>
  </TableCell>

  <TableCell>
    <div class="flex flex-col gap-1">
      <span class="line-clamp-1 font-medium">{download.name}</span>
      <div class="flex flex-wrap gap-2 text-xs text-muted-foreground md:hidden">
        <span>{download.size}</span>
        {#if download.fileType}<span>- {download.fileType}</span>{/if}
        {#if download.category}<span>- {download.category}</span>{/if}
        {#if download.eta}<span>- {download.eta}</span>{/if}
      </div>
    </div>
  </TableCell>

  <TableCell class="hidden md:table-cell text-muted-foreground">
    {download.size}
  </TableCell>

  <TableCell class="hidden md:table-cell text-muted-foreground">
    {download.fileType}
  </TableCell>

  <TableCell class="hidden md:table-cell text-muted-foreground">
    {download.category}
  </TableCell>

  <TableCell class="hidden md:table-cell text-muted-foreground">
    {download.eta}
  </TableCell>

  <TableCell class="flex flex-col gap-2 md:flex-row md:items-center md:justify-end">
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
      <Button type="button" variant="ghost" size="sm" onclick={copyLink}>Copy</Button>
      {#if download.status === 'completed'}
        <Button type="button" variant="ghost" size="sm" onclick={openFile}>Open</Button>
        <Button type="button" variant="ghost" size="sm" onclick={showInFolder}>Show</Button>
      {:else if download.status === 'failed'}
        <Button type="button" variant="ghost" size="sm" onclick={retry}>Retry</Button>
      {/if}
    </div>
  </TableCell>

  {#if download.progress >= 0 && (download.status === 'downloading' || download.status === 'pending')}
    <TableCell colspan={7}>
      <Progress value={Math.floor(download.progress)} max={100} aria-label="Download progress" />
    </TableCell>
  {/if}
</TableRow>
