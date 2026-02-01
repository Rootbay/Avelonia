<script lang="ts">
  import type { Download } from '$lib/downloadManager';
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
  import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';

  type ButtonSnippetContext = { props?: Record<string, unknown> & { class?: string } };

  const { download, selected, onToggleSelect, startDownload, cancelDownload } = $props<{
    download: Download;
    selected: boolean;
    onToggleSelect?: (payload: { checked: boolean; shiftKey: boolean }) => void;
    startDownload: (id: number) => void;
    cancelDownload?: (id: number) => Promise<void>;
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

  function cancel(event: MouseEvent) {
    event.stopPropagation();
    if (cancelDownload) {
      void cancelDownload(download.id);
    }
  }

  const releaseOptions = $derived.by(() => download.releases ?? []);
  const defaultReleaseLabel = $derived.by(() => releaseOptions[0]?.label ?? '');
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
  <TableCell class="w-20 pl-2">
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
        class="flex size-8 shrink-0 items-center justify-center rounded-md border bg-muted text-sm font-semibold uppercase text-muted-foreground"
        >{download.name?.[0] ?? '?'}
      </span>
    </div>
  </TableCell>

  <TableCell class="w-[35%]">
    <div class="flex flex-col gap-1 overflow-hidden">
      <div class="flex items-center gap-2">
        <span class="truncate font-medium">{download.name}</span>
        {#if releaseOptions.length > 1}
          <div onclick={(e) => e.stopPropagation()} role="none" class="shrink-0">
            <Select
              type="single"
              value={activeReleaseLabel}
              onValueChange={(v) => setDownloadRelease(download.id, v)}
            >
              <SelectTrigger
                class="h-6 w-35 overflow-hidden px-2 py-0 text-[10px] uppercase tracking-wider"
              >
                <span class="truncate">{activeReleaseLabel}</span>
              </SelectTrigger>
              <SelectContent>
                {#each releaseOptions as release (release.label)}
                  <SelectItem value={release.label} class="text-[10px] uppercase tracking-wider"
                    >{release.label}</SelectItem
                  >
                {/each}
              </SelectContent>
            </Select>
          </div>
        {/if}
      </div>
      <div class="flex flex-nowrap gap-2 text-xs text-muted-foreground md:hidden overflow-x-auto">
        <span>{download.size}</span>
        {#if download.fileType}<span>- {download.fileType}</span>{/if}
        {#if download.category}<span>- {download.category}</span>{/if}
        {#if download.eta}<span>- {download.eta}</span>{/if}
      </div>
    </div>
  </TableCell>

  <TableCell class="w-28 text-muted-foreground">{download.size}</TableCell>
  <TableCell class="w-24 text-muted-foreground">{download.fileType}</TableCell>
  <TableCell class="w-32 text-muted-foreground">{download.category}</TableCell>
  <TableCell class="w-28 text-muted-foreground">{download.eta}</TableCell>

  <TableCell class="w-52 pl-8">
    <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-start overflow-hidden">
      <span class={`text-xs font-semibold uppercase tracking-wide shrink-0 ${statusTone}`}>
        {download.status}
        {#if download.progress < 0}
          <span class="ml-1 text-muted-foreground">(preparing)</span>
        {/if}
        {#if download.status === 'downloading' && download.speed}
          <span class="ml-1 text-muted-foreground whitespace-nowrap hidden md:inline"
            >- {download.speed}
          </span>
        {/if}
      </span>
      <div class="flex shrink-0 items-center gap-2">
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
          <Button type="button" variant="ghost" size="sm" class="h-7" onclick={retry}>Retry</Button>
        {:else if cancelDownload && (download.status === 'downloading' || download.status === 'pending' || download.status === 'queued')}
          <Button type="button" variant="destructive" size="sm" class="h-7" onclick={cancel}
            >Cancel</Button
          >
        {/if}
      </div>
    </div>
  </TableCell>
</TableRow>
