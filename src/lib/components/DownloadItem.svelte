<script lang="ts">
  import type { Download, DownloadRelease } from '$lib/downloadManager';
  import { getDownloadPath, setDownloadRelease } from '$lib/downloadManager';
  import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { TableRow, TableCell } from '$lib/components/ui/table';
  import { toast } from '$lib/components/ui/sonner';
  import { Ellipsis } from '@lucide/svelte';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';

  type ButtonSnippetContext = { props?: Record<string, unknown> & { class?: string } };

  const { download, selected, onToggleSelect, startDownload } = $props<{
    download: Download;
    selected: boolean;
    onToggleSelect?: (payload: { checked: boolean; shiftKey: boolean }) => void;
    startDownload: (id: number) => void;
  }>();

  const statusTone = $derived.by(() => {
    switch (download.status) {
      case 'completed':
        return 'text-emerald-400';
      case 'installed':
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

  function handleReleaseChange(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    if (value && value !== activeReleaseLabel) {
      setDownloadRelease(download.id, value);
    }
  }

  const releaseOptions = $derived.by(() => download.releases ?? []);
  const defaultReleaseLabel = $derived.by(
    () => releaseOptions[0]?.label ?? ''
  );
  const activeReleaseLabel = $derived.by(
    () => download.selectedReleaseLabel ?? defaultReleaseLabel
  );
</script>

{#snippet DetailsTrigger({ props }: ButtonSnippetContext)}
  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
  {@const { class: propsClass, ...restWithoutClass } = rawProps}
  {@const restProps = restWithoutClass as Record<string, unknown>}
  <span role="none" onclick={(e: MouseEvent) => e.stopPropagation()}>
    <Button
      {...restProps}
      type="button"
      variant="ghost"
      size="sm"
      aria-label="Details"
      class={propsClass}
    >
      <Ellipsis class="size-4" />
    </Button>
  </span>
{/snippet}

<TableRow
  data-status={download.status}
  data-no-hover-bg
  data-hover-soft
  onclick={(event: MouseEvent) => {
    const next = !selected;
    onToggleSelect?.({ checked: next, shiftKey: event.shiftKey });
  }}
  onkeydown={(event: KeyboardEvent) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const next = !selected;
      onToggleSelect?.({ checked: next, shiftKey: event.shiftKey });
    }
  }}
>
  <TableCell class="w-[60px]">
    <div
      class="flex items-center gap-2"
      role="checkbox"
      aria-checked={selected}
      tabindex="0"
      onclick={(event: MouseEvent) => {
        event.stopPropagation();
        const next = !selected;
        onToggleSelect?.({ checked: next, shiftKey: event.shiftKey });
      }}
      onkeydown={(event: KeyboardEvent) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          const next = !selected;
          onToggleSelect?.({ checked: next, shiftKey: event.shiftKey });
        }
      }}
    >
      <Checkbox checked={selected} title="Select download" />
      <span
        class="flex size-8 items-center justify-center rounded-md border bg-muted text-sm font-semibold uppercase text-muted-foreground"
        >{download.name?.[0] ?? '?'}
      </span>
    </div>
  </TableCell>

  <TableCell>
    <div class="flex flex-col gap-1">
      <span class="line-clamp-1 font-medium">{download.name}</span>
      <div class="flex flex-nowrap gap-2 text-xs text-muted-foreground md:hidden overflow-x-auto">
        <span>{download.size}</span>
        {#if download.fileType}<span>- {download.fileType}</span>{/if}
        {#if download.category}<span>- {download.category}</span>{/if}
        {#if download.eta}<span>- {download.eta}</span>{/if}
      </div>
      {#if releaseOptions.length > 1}
        <div class="flex flex-wrap items-center gap-2 text-[0.65rem] uppercase tracking-wide text-muted-foreground">
          <span>Version</span>
          <select
            class="rounded border border-input bg-transparent px-2 py-1 text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            value={activeReleaseLabel}
            onchange={handleReleaseChange}
          >
            {#each releaseOptions as release}
              <option value={release.label}>{release.label}</option>
            {/each}
          </select>
        </div>
      {/if}
    </div>
  </TableCell>

  <TableCell class="hidden md:table-cell text-muted-foreground">{download.size}</TableCell>
  <TableCell class="hidden md:table-cell text-muted-foreground">{download.fileType}</TableCell>
  <TableCell class="hidden md:table-cell text-muted-foreground">{download.category}</TableCell>
  <TableCell class="hidden md:table-cell text-muted-foreground">{download.eta}</TableCell>

  <TableCell
    class="w-[180px] pl-6 sm:pl-8 flex flex-col gap-2 md:flex-row md:items-center md:justify-start"
  >
    <span class={`text-xs font-semibold uppercase tracking-wide ${statusTone}`}>
      {download.status}
      {#if download.progress < 0}
        <span class="ml-1 text-muted-foreground">(preparing)</span>
      {/if}
      {#if download.status === 'downloading' && download.speed}
        <span
          class="ml-1 text-muted-foreground whitespace-nowrap max-w-40 overflow-hidden text-ellipsis hidden md:inline"
          >- {download.speed}
        </span>
      {/if}
    </span>
    <div class="flex flex-wrap items-center gap-2">
      {#if download.status === 'completed'}
        <DropdownMenu>
          <DropdownMenuTrigger child={DetailsTrigger} />
          <DropdownMenuContent align="end" onclick={(e: MouseEvent) => e.stopPropagation()}>
            <DropdownMenuLabel class="max-w-64 truncate" title={download.name}
              >{download.name}</DropdownMenuLabel
            >
            <DropdownMenuSeparator />
            <DropdownMenuItem
              onclick={(e: MouseEvent) => {
                e.stopPropagation();
                void openFile(e);
              }}>Open file</DropdownMenuItem
            >
            <DropdownMenuItem
              onclick={(e: MouseEvent) => {
                e.stopPropagation();
                void showInFolder(e);
              }}>Show in folder</DropdownMenuItem
            >
          </DropdownMenuContent>
        </DropdownMenu>
      {:else if download.status === 'failed'}
        <Button type="button" variant="ghost" size="sm" onclick={retry}>Retry</Button>
      {/if}
    </div>
  </TableCell>
</TableRow>
