<script lang="ts">
  import { vtScan } from '$lib/scanStatus.svelte';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogDescription,
    DialogFooter,
    DialogClose,
  } from '$lib/components/ui/dialog';
  import { Ellipsis } from '@lucide/svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { openUrl as openExternal } from '@tauri-apps/plugin-opener';
  import { i18n } from '$lib/i18n.svelte';

  let { open = $bindable(false) } = $props();

  let vtExpanded = $state(new Set<string>());
  function vtKeyOf(it: { subject: string; source?: string }) {
    return `${it?.source || 'startup'}|${(it?.subject || '').toString().trim().toLowerCase()}`;
  }
  function toggleVtDetails(it: { subject: string; source?: string }) {
    const k = vtKeyOf(it);
    if (vtExpanded.has(k)) vtExpanded.delete(k);
    else vtExpanded.add(k);
    vtExpanded = new SvelteSet(vtExpanded);
  }

  type VtTotalsCounts = {
    clean: number;
    detected: number;
    notScanned: number;
    total: number;
  };

  const vtTotals = $derived((): VtTotalsCounts => {
    const items = (vtScan?.items ?? []) as Array<{ verdict?: string }>;
    let clean = 0,
      detected = 0,
      notScanned = 0;
    for (const it of items) {
      const v = String(it?.verdict || '').toLowerCase();
      if (v === 'clean') clean += 1;
      else if (v === 'malicious' || v === 'suspicious') detected += 1;
      else notScanned += 1;
    }
    return { clean, detected, notScanned, total: items.length };
  });
</script>

<Dialog bind:open>
  <DialogContent class="sm:max-w-2xl">
    <DialogHeader>
      <DialogTitle>{i18n.t('vt_scan.title')}</DialogTitle>
      <DialogDescription>
        {#if vtScan.phase === 'running'}
          {i18n.t('vt_scan.running')}
        {:else if vtScan.phase === 'done'}
          {i18n.t('vt_scan.finished')}
        {:else}
          {i18n.t('vt_scan.idle')}
        {/if}
      </DialogDescription>
    </DialogHeader>
    <div class="space-y-2 text-sm">
      <p class="text-xs text-muted-foreground">
        {#if vtScan.phase === 'running'}
          {i18n.t('vt_scan.source', { source: vtScan.source ?? 'N/A' })}
        {:else}
          {#if vtScan.startedAt}{i18n.t('vt_scan.started', {
              time: new Date(vtScan.startedAt).toLocaleTimeString(),
            })}{/if}
          {#if vtScan.finishedAt}
            • {i18n.t('vt_scan.finished_at', {
              time: new Date(vtScan.finishedAt).toLocaleTimeString(),
            })}{/if}
          • {i18n.t('vt_scan.processed', { count: vtScan.items?.length ?? 0 })}
          {#if (vtScan.expectedStartup ?? undefined) !== undefined || (vtScan.expectedRegistry ?? undefined) !== undefined}
            • {i18n.t('vt_scan.expected', {
              startup: vtScan.expectedStartup ?? '?',
              registry: vtScan.expectedRegistry ?? '?',
            })}
          {/if}
        {/if}
      </p>
      <div class="mb-1 flex flex-wrap gap-2">
        <Badge variant="secondary"
          >{i18n.t('vt_scan.detected', { count: vtTotals().detected })}</Badge
        >
        <Badge class="border-green-500/30 text-green-700 bg-green-500/10"
          >{i18n.t('vt_scan.clean', { count: vtTotals().clean })}</Badge
        >
        <Badge class="border-yellow-500/30 text-yellow-700 bg-yellow-500/10"
          >{i18n.t('vt_scan.not_scanned', { count: vtTotals().notScanned })}</Badge
        >
      </div>
      <div class="max-h-64 overflow-auto rounded-md border border-border/60 bg-muted/10">
        <table class="w-full text-sm">
          <thead class="sticky top-0 bg-card/80 backdrop-blur supports-backdrop-filter:bg-card/70">
            <tr>
              <th class="text-left px-2 py-1">{i18n.t('vt_scan.subject')}</th>
              <th class="text-left px-2 py-1">{i18n.t('vt_scan.from')}</th>
              <th class="text-left px-2 py-1">{i18n.t('vt_scan.verdict')}</th>
              <th class="text-left px-2 py-1">{i18n.t('vt_scan.not_detected')}</th>
              <th class="text-left px-2 py-1">{i18n.t('vt_scan.details')}</th>
            </tr>
          </thead>
          <tbody>
            {#each vtScan.items as it (vtKeyOf(it))}
              <tr>
                <td class="px-2 py-1 truncate max-w-[40ch]">{it.subject}</td>
                <td class="px-2 py-1">{it.source}</td>
                <td class="px-2 py-1"
                  >{it.verdict ||
                    '-'}{#if it.reason && (!it.verdict || it.verdict.toLowerCase() === 'unknown')}
                    <span class="text-muted-foreground">({it.reason})</span>{/if}</td
                >
                <td class="px-2 py-1">
                  {#if typeof it.total_vendors === 'number'}
                    {Math.max(0, (it.total_vendors || 0) - (it.positives || 0))}
                  {:else if typeof it.harmless === 'number' || typeof it.undetected === 'number'}
                    {(it.harmless || 0) + (it.undetected || 0)}
                  {:else}
                    -
                  {/if}
                </td>
                <td class="px-2 py-1">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    aria-label="Details"
                    onclick={(e) => {
                      e.stopPropagation();
                      toggleVtDetails(it);
                    }}
                  >
                    <Ellipsis class="size-4" />
                  </Button>
                </td>
              </tr>
              {#if vtExpanded.has(vtKeyOf(it))}
                <tr>
                  <td class="px-2 py-2 text-xs text-muted-foreground" colspan="5">
                    <div class="grid grid-cols-2 gap-2">
                      <div>
                        {i18n.t('vt_scan.malicious', {
                          count: typeof it.malicious === 'number' ? it.malicious : '-',
                        })}
                      </div>
                      <div>
                        {i18n.t('vt_scan.suspicious', {
                          count: typeof it.suspicious === 'number' ? it.suspicious : '-',
                        })}
                      </div>
                      <div>
                        {i18n.t('vt_scan.harmless', {
                          count: typeof it.harmless === 'number' ? it.harmless : '-',
                        })}
                      </div>
                      <div>
                        {i18n.t('vt_scan.undetected', {
                          count: typeof it.undetected === 'number' ? it.undetected : '-',
                        })}
                      </div>
                    </div>
                    {#if it.reason}
                      <div class="mt-1">{i18n.t('vt_scan.reason', { reason: it.reason })}</div>
                    {/if}
                    {#if it.permalink}
                      <div class="mt-2">
                        <button
                          type="button"
                          class="px-0 text-xs text-white hover:text-emerald-600 underline-offset-4 hover:underline"
                          onclick={() => {
                            try {
                              void openExternal(it.permalink as string);
                            } catch {
                              /* noop */
                            }
                          }}
                        >
                          {i18n.t('vt_scan.open_vt')}
                        </button>
                      </div>
                    {/if}
                  </td>
                </tr>
              {/if}
            {/each}
            {#if (vtScan.items?.length ?? 0) === 0}
              <tr
                ><td colspan="5" class="px-2 py-3 text-center text-muted-foreground"
                  >{i18n.t('vt_scan.no_items')}</td
                ></tr
              >
            {/if}
          </tbody>
        </table>
      </div>
    </div>
    <DialogFooter>
      <DialogClose>
        <Button>{i18n.t('common.close')}</Button>
      </DialogClose>
    </DialogFooter>
  </DialogContent>
</Dialog>
