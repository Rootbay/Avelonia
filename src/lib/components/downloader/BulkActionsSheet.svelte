<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Separator } from '$lib/components/ui/separator';
  import {
    Sheet,
    SheetContent,
    SheetDescription,
    SheetHeader,
    SheetTitle,
  } from '$lib/components/ui/sheet';
  import {
    Play,
    CircleX,
    RefreshCcw,
    Clipboard as ClipboardIcon,
    FileDown,
    Eye,
    FolderOpen,
    Trash2,
  } from '@lucide/svelte';
  import { i18n } from '$lib/i18n.svelte';

  interface GlobalStats {
    active: number;
    completed: number;
    failed: number;
    startable: number;
    cancelable: number;
    total: number;
  }

  interface FilteredStats {
    count: number;
    startable: number;
    cancelable: number;
    failed: number;
    deletable: number;
  }

  interface SelectedStats {
    count: number;
    startable: number;
    cancelable: number;
    completed: number;
    deletable: number;
  }

  let {
    open = $bindable(false),
    globalStats,
    filteredStats,
    selectedStats,
    onAction,
  } = $props<{
    open: boolean;
    globalStats: GlobalStats;
    filteredStats: FilteredStats;
    selectedStats: SelectedStats;
    onAction: (action: string) => void;
  }>();
</script>

<Sheet bind:open>
  <SheetContent side="right" class="w-85 sm:w-95 p-4 sm:p-6">
    <SheetHeader class="space-y-1 p-0">
      <SheetTitle>{i18n.t('downloader.bulk_title')}</SheetTitle>
      <SheetDescription>{i18n.t('downloader.bulk_desc')}</SheetDescription>
    </SheetHeader>
    <div class="mt-3 space-y-3">
      <div class="space-y-2">
        <p class="text-xs uppercase tracking-wide text-muted-foreground">{i18n.t('downloader.bulk_all_downloads')}</p>
        <div class="grid gap-2">
          <Button
            type="button"
            variant="outline"
            class="justify-between"
            onclick={() => {
              onAction('startAll');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"><Play class="size-4" /> {i18n.t('downloader.bulk_start_all')}</span>
            {#if globalStats.startable > 0}
              <span class="text-xs text-muted-foreground tabular-nums">{globalStats.startable}</span
              >
            {/if}
          </Button>
          <Button
            type="button"
            variant="outline"
            class="justify-between"
            disabled={globalStats.cancelable === 0}
            onclick={() => {
              onAction('cancelAllActive');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"><CircleX class="size-4" /> {i18n.t('downloader.bulk_cancel_active')}</span>
            {#if globalStats.cancelable > 0}
              <span class="text-xs text-muted-foreground tabular-nums"
                >{globalStats.cancelable}</span
              >
            {/if}
          </Button>
        </div>
      </div>
      <Separator />
      <div class="space-y-2">
        <p class="text-xs uppercase tracking-wide text-muted-foreground">{i18n.t('downloader.bulk_current_view')}</p>
        <div class="grid gap-2">
          <Button
            type="button"
            variant="outline"
            class="justify-between"
            disabled={filteredStats.startable === 0}
            onclick={() => {
              onAction('startAllFiltered');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"><Play class="size-4" /> {i18n.t('downloader.bulk_start_filtered')}</span>
            {#if filteredStats.startable > 0}
              <span class="text-xs text-muted-foreground tabular-nums"
                >{filteredStats.startable}</span
              >
            {/if}
          </Button>
          <Button
            type="button"
            variant="outline"
            class="justify-between"
            disabled={filteredStats.cancelable === 0}
            onclick={() => {
              onAction('cancelAllFiltered');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"><CircleX class="size-4" /> {i18n.t('downloader.bulk_cancel_filtered')}</span>
            {#if filteredStats.cancelable > 0}
              <span class="text-xs text-muted-foreground tabular-nums"
                >{filteredStats.cancelable}</span
              >
            {/if}
          </Button>
          <Button
            type="button"
            variant="outline"
            class="justify-between"
            disabled={filteredStats.deletable === 0}
            onclick={() => {
              onAction('deleteFiltered');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"><Trash2 class="size-4" /> {i18n.t('downloader.bulk_delete_filtered')}</span>
            {#if filteredStats.deletable > 0}
              <span class="text-xs text-muted-foreground tabular-nums"
                >{filteredStats.deletable}</span
              >
            {/if}
          </Button>
          <Button
            type="button"
            variant="outline"
            class="justify-between"
            disabled={filteredStats.failed === 0}
            onclick={() => {
              onAction('retryFailedFiltered');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"
              ><RefreshCcw class="size-4" /> {i18n.t('downloader.bulk_retry_failed_filtered')}</span
            >
            {#if filteredStats.failed > 0}
              <span class="text-xs text-muted-foreground tabular-nums">{filteredStats.failed}</span>
            {/if}
          </Button>
          <Button
            type="button"
            variant="ghost"
            class="justify-between"
            onclick={() => {
              onAction('exportFilteredCSV');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"><FileDown class="size-4" /> {i18n.t('downloader.bulk_export_csv')}</span>
          </Button>
        </div>
      </div>
      <Separator />
      <div class="space-y-2">
        <p class="text-xs uppercase tracking-wide text-muted-foreground">{i18n.t('downloader.bulk_selected')}</p>
        <div class="grid gap-2">
          <Button
            type="button"
            variant="outline"
            class="justify-between"
            disabled={selectedStats.startable === 0}
            onclick={() => {
              onAction('startSelected');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"><Play class="size-4" /> {i18n.t('downloader.bulk_start_selected')}</span>
            {#if selectedStats.startable > 0}
              <span class="text-xs text-muted-foreground tabular-nums"
                >{selectedStats.startable}</span
              >
            {/if}
          </Button>
          <Button
            type="button"
            variant="outline"
            class="justify-between"
            disabled={selectedStats.cancelable === 0}
            onclick={() => {
              onAction('cancelSelected');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"><CircleX class="size-4" /> {i18n.t('downloader.bulk_cancel_selected')}</span>
            {#if selectedStats.cancelable > 0}
              <span class="text-xs text-muted-foreground tabular-nums"
                >{selectedStats.cancelable}</span
              >
            {/if}
          </Button>
          <Button
            variant="outline"
            class="justify-between"
            disabled={selectedStats.deletable === 0}
            onclick={() => {
              onAction('deleteSelected');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"><Trash2 class="size-4" /> {i18n.t('downloader.bulk_delete_selected')}</span>
            {#if selectedStats.deletable > 0}
              <span class="text-xs text-muted-foreground tabular-nums"
                >{selectedStats.deletable}</span
              >
            {/if}
          </Button>
          <div class="grid grid-cols-2 gap-2">
            <Button
              variant="outline"
              class="justify-center"
              disabled={selectedStats.completed === 0}
              onclick={() => {
                onAction('openSelectedCompleted');
                open = false;
              }}
            >
              <Eye class="size-4" />
              <span class="ml-2">{i18n.t('downloader.bulk_open')}</span>
            </Button>
            <Button
              variant="outline"
              class="justify-center"
              disabled={selectedStats.completed === 0}
              onclick={() => {
                onAction('showSelectedCompleted');
                open = false;
              }}
            >
              <FolderOpen class="size-4" />
              <span class="ml-2">{i18n.t('downloader.bulk_show')}</span>
            </Button>
          </div>
          <div class="grid grid-cols-2 gap-2">
            <Button
              variant="ghost"
              class="justify-center"
              disabled={selectedStats.count === 0}
              onclick={() => {
                onAction('copySelectedLinks');
                open = false;
              }}
            >
              <ClipboardIcon class="size-4" />
              <span class="ml-2">{i18n.t('downloader.bulk_copy_links')}</span>
            </Button>
            <Button
              variant="ghost"
              class="justify-center"
              onclick={() => {
                onAction('exportFilteredCSV');
                open = false;
              }}
            >
              <FileDown class="size-4" />
              <span class="ml-2">{i18n.t('downloader.bulk_export')}</span>
            </Button>
          </div>
        </div>
      </div>
      <Separator />
      <div class="space-y-2">
        <p class="text-xs uppercase tracking-wide text-muted-foreground">{i18n.t('downloader.bulk_failed')}</p>
        <div class="grid gap-2">
          <Button
            variant="outline"
            class="justify-between"
            disabled={globalStats.failed === 0}
            onclick={() => {
              onAction('retryAllFailed');
              open = false;
            }}
          >
            <span class="flex items-center gap-2"
              ><RefreshCcw class="size-4" /> {i18n.t('downloader.bulk_retry_all_failed')}</span
            >
            {#if globalStats.failed > 0}
              <span class="text-xs text-muted-foreground tabular-nums">{globalStats.failed}</span>
            {/if}
          </Button>
        </div>
      </div>
    </div>
  </SheetContent>
</Sheet>
