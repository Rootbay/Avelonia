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
      <DialogTitle>VirusTotal Scan</DialogTitle>
      <DialogDescription>
        {#if vtScan.phase === 'running'}
          Scanning startup and registry items...
        {:else if vtScan.phase === 'done'}
          Scan finished.
        {:else}
          Idle. Trigger a scan from Settings.
        {/if}
      </DialogDescription>
    </DialogHeader>
    <div class="space-y-2 text-sm">
      <p class="text-xs text-muted-foreground">
        {#if vtScan.phase === 'running'}
          Source: {vtScan.source ?? 'N/A'}
        {:else}
          {#if vtScan.startedAt}Started {new Date(vtScan.startedAt).toLocaleTimeString()}{/if}
          {#if vtScan.finishedAt}
            • Finished {new Date(vtScan.finishedAt).toLocaleTimeString()}{/if}
          • Processed {vtScan.items?.length ?? 0} items
          {#if (vtScan.expectedStartup ?? undefined) !== undefined || (vtScan.expectedRegistry ?? undefined) !== undefined}
            • Expected {vtScan.expectedStartup ?? '?'}/{vtScan.expectedRegistry ?? '?'}
          {/if}
        {/if}
      </p>
      <div class="mb-1 flex flex-wrap gap-2">
        <Badge variant="secondary">Detected {vtTotals().detected}</Badge>
        <Badge class="border-green-500/30 text-green-700 bg-green-500/10"
          >Clean {vtTotals().clean}</Badge
        >
        <Badge class="border-yellow-500/30 text-yellow-700 bg-yellow-500/10"
          >Not Scanned {vtTotals().notScanned}</Badge
        >
      </div>
      <div class="max-h-64 overflow-auto rounded-md border border-border/60 bg-muted/10">
        <table class="w-full text-sm">
          <thead class="sticky top-0 bg-card/80 backdrop-blur supports-backdrop-filter:bg-card/70">
            <tr>
              <th class="text-left px-2 py-1">Subject</th>
              <th class="text-left px-2 py-1">From</th>
              <th class="text-left px-2 py-1">Verdict</th>
              <th class="text-left px-2 py-1">Not detected</th>
              <th class="text-left px-2 py-1">Details</th>
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
                      <div>Malicious: {typeof it.malicious === 'number' ? it.malicious : '-'}</div>
                      <div>
                        Suspicious: {typeof it.suspicious === 'number' ? it.suspicious : '-'}
                      </div>
                      <div>Harmless: {typeof it.harmless === 'number' ? it.harmless : '-'}</div>
                      <div>
                        Undetected: {typeof it.undetected === 'number' ? it.undetected : '-'}
                      </div>
                    </div>
                    {#if it.reason}
                      <div class="mt-1">Reason: {it.reason}</div>
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
                          Open on VirusTotal
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
                  >No items yet.</td
                ></tr
              >
            {/if}
          </tbody>
        </table>
      </div>
    </div>
    <DialogFooter>
      <DialogClose>
        <Button>Close</Button>
      </DialogClose>
    </DialogFooter>
  </DialogContent>
</Dialog>
