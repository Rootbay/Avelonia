<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
  } from '$lib/components/ui/dialog';
  import { toast } from '$lib/components/ui/sonner';
  import { invoke } from '@tauri-apps/api/core';
  import { addDownload } from '$lib/downloads';
  import { tags as customTags, addTag, BUILT_IN_TAGS } from '$lib/tags';
  import { sanitizeFileName, normalizeExtension } from '$lib/downloadPath';
  import { prettifyDisplayName } from '$lib/name';

  let { open = $bindable(false) } = $props();

  let addUrl = $state('');
  let addCategory = $state('');
  let addName = $state('');
  let nameTouched = $state(false);
  let probing = $state(false);
  let probeName = $state('');
  let probeExt = $state('');
  let probeSize = $state(0);

  const tagOptions = $derived([...new Set([...BUILT_IN_TAGS, ...$customTags])]);
  const guessed = $derived.by(() => guessFromUrl(addUrl));
  const effectiveExt = $derived(probeExt || guessed.ext);
  const effectiveName = $derived(prettifyDisplayName(probeName || guessed.name, effectiveExt));

  function guessFromUrl(url: string): { name: string; ext: string } {
    try {
      const u = new URL(url);
      let last = decodeURIComponent(u.pathname.split('/').filter(Boolean).pop() || '');
      last = last.replace(/[?#].*$/, '');
      if (!last) return { name: 'download', ext: '' };
      const lower = last.toLowerCase();
      const multi = ['.tar.gz', '.tar.bz2', '.tar.xz', '.tar.zst'];
      for (const m of multi) {
        if (lower.endsWith(m)) {
          const base = last.slice(0, -m.length);
          const ext = m.slice(1);
          return { name: sanitizeFileName(base), ext };
        }
      }
      const idx = last.lastIndexOf('.');
      if (idx > 0 && idx < last.length - 1) {
        const base = last.slice(0, idx);
        const ext = last.slice(idx + 1);
        return { name: sanitizeFileName(base), ext };
      }
      return { name: sanitizeFileName(last), ext: '' };
    } catch {
      return { name: 'download', ext: '' };
    }
  }

  $effect(() => {
    if (!nameTouched) {
      addName = effectiveName;
    }
  });

  $effect(() => {
    if (open) {
      nameTouched = false;
    }
  });

  $effect(() => {
    const u = addUrl.trim();
    if (!/^https?:\/\//i.test(u)) {
      probeName = '';
      probeExt = '';
      return;
    }
    const t = setTimeout(async () => {
      try {
        probing = true;
        const res = (await invoke('probe_download', { url: u })) as {
          filename?: string;
          ext?: string;
          size?: number;
        };
        probeName = res?.filename || '';
        probeExt = res?.ext || '';
        probeSize = typeof res?.size === 'number' ? (res!.size as number) : 0;
      } catch {
        probeName = '';
        probeExt = '';
        probeSize = 0;
      } finally {
        probing = false;
      }
    }, 250);
    return () => clearTimeout(t);
  });

  function formatBytes(bytes: number): string {
    if (!isFinite(bytes) || bytes <= 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
    let i = 0;
    while (bytes >= 1024 && i < units.length - 1) {
      bytes /= 1024;
      i++;
    }
    return `${bytes.toFixed(bytes >= 100 ? 0 : 1)} ${units[i]}`;
  }

  function addNewDownload() {
    const url = addUrl.trim();
    if (!url) {
      toast.error('Enter a valid URL');
      return;
    }
    if (!/^https?:\/\//i.test(url)) {
      toast.error('Only http(s) URLs are supported');
      return;
    }
    const name = (addName || '').trim() || effectiveName;
    const ext = effectiveExt;
    const normExt = normalizeExtension(ext).replace(/^\./, '');
    addDownload({
      name,
      description: '',
      size: probeSize > 0 ? formatBytes(probeSize) : 'N/A',
      fileType: normExt,
      category: addCategory.trim() || 'General',
      tags: addCategory.trim() ? [addCategory.trim()] : ['General'],
      downloadLink: url,
    });
    const cat = addCategory.trim();
    if (cat && !tagOptions.includes(cat)) addTag(cat);
    open = false;
    addUrl = '';
    addCategory = '';
    toast.success('Added to list');
  }
</script>

<Dialog bind:open>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Add download</DialogTitle>
      <DialogDescription>Paste a direct download link and choose a category.</DialogDescription>
    </DialogHeader>
    <div class="grid gap-3">
      <div class="space-y-1">
        <label class="text-sm font-medium" for="add-url">URL</label>
        <Input id="add-url" placeholder="https://example.com/file.exe" bind:value={addUrl} />
      </div>
      <div class="space-y-1">
        <label class="text-sm font-medium" for="add-cat">Category</label>
        <Input
          id="add-cat"
          placeholder="e.g. Utilities"
          bind:value={addCategory}
          list="category-options"
        />
        <datalist id="category-options">
          {#each tagOptions as t (t)}
            <option value={t}></option>
          {/each}
        </datalist>
      </div>
      <div class="grid grid-cols-2 gap-3">
        <div class="space-y-1">
          <label class="text-sm font-medium" for="add-name">Name</label>
          <Input
            id="add-name"
            placeholder="App name"
            bind:value={addName}
            oninput={() => (nameTouched = true)}
          />
          {#if !nameTouched}
            <p class="text-xs text-muted-foreground">
              Suggested: {effectiveName}{#if probing}
                (detecting...){/if}
            </p>
          {/if}
        </div>
        <div class="space-y-1">
          <div class="text-sm font-medium">Type</div>
          <div class="text-sm text-foreground">{normalizeExtension(effectiveExt) || '—'}</div>
        </div>
      </div>
    </div>
    <DialogFooter>
      <Button type="button" variant="secondary" onclick={() => (open = false)}>Cancel</Button>
      <Button type="button" onclick={addNewDownload}>Add</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
