<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
  import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogDescription,
    DialogFooter,
    DialogClose,
  } from '$lib/components/ui/dialog';
  import { settings, updateDownloaderSettings } from '$lib/settings';
  import { invoke } from '@tauri-apps/api/core';
  import { toast } from '$lib/components/ui/sonner';
  import { pushLog, clearLogs } from '$lib/logStore';
  import { save } from '@tauri-apps/plugin-dialog';
  import { beginScan, endScan } from '$lib/scanStatus.svelte';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  let vtKey = $state('');
  let vtPersist = $state(true);
  let vtKeySet = $state(false);
  let vtBusy = $state(false);

  let autoInstall = $state($settings.downloader.autoInstall);
  let installMode = $state($settings.downloader.installMode);
  let elevateInstall = $state($settings.downloader.elevate);
  let fallbackOpen = $state($settings.downloader.fallbackOpen);
  let verifyInstall = $state($settings.downloader.verifyInstall);
  let catalogFilePath = $state($settings.downloader.downloadCatalogPath ?? '');

  $effect(() => {
    autoInstall = $settings.downloader.autoInstall;
    installMode = $settings.downloader.installMode;
    elevateInstall = $settings.downloader.elevate;
    fallbackOpen = $settings.downloader.fallbackOpen;
    verifyInstall = $settings.downloader.verifyInstall;
    catalogFilePath = $settings.downloader.downloadCatalogPath ?? '';
  });

  $effect(() => {
    updateDownloaderSettings({
      autoInstall,
      installMode,
      elevate: elevateInstall,
      fallbackOpen,
      verifyInstall,
    });
  });

  $effect(() => {
    if (open) {
      (async () => {
        try {
          const st = (await invoke('vt_get_status')) as { key_set?: boolean };
          vtKeySet = !!st?.key_set;
        } catch {
          /* noop */
        }
      })();
    }
  });

  async function saveVtKey() {
    try {
      vtBusy = true;
      await invoke('vt_set_api_key', { key: vtKey || null, persist: vtPersist });
      const st = (await invoke('vt_get_status')) as { key_set?: boolean };
      vtKeySet = !!st?.key_set;
      toast.success('VirusTotal key saved');
      pushLog('SUCCESS', `VT key saved${vtPersist ? ' (persisted)' : ''}.`, 'Optimize');
    } catch (e) {
      toast.error('Failed to save VirusTotal key');
      pushLog('ERROR', `Saving VT key failed: ${String(e)}`, 'Optimize');
    } finally {
      vtBusy = false;
    }
  }

  async function chooseCatalogFile() {
    try {
      const selected = await save({
        title: 'Select catalog (JSON)',
        filters: [{ name: 'JSON', extensions: ['json'] }],
        defaultPath: catalogFilePath || 'avelonia-downloads.json',
      });
      if (typeof selected === 'string' && selected) {
        updateDownloaderSettings({ downloadCatalogPath: selected });
        toast.success('Download catalog file selected');
      }
    } catch (error) {
      pushLog('ERROR', `Failed to select catalog file: ${String(error)}`, 'Downloader');
      toast.error('Unable to pick a catalog file');
    }
  }

  function clearCatalogFile() {
    if (!catalogFilePath) return;
    updateDownloaderSettings({ downloadCatalogPath: '' });
    toast.info('Download catalog cleared');
  }

  async function runVtScanNow() {
    try {
      vtBusy = true;
      pushLog('INFO', 'VT scan starting (manual).', 'Optimize');
      beginScan('manual');
      toast.message('VirusTotal scan started');
      const res = (await invoke('vt_scan_all', { limit: 50, force: true })) as [number, number];
      endScan({ startup: res?.[0], registry: res?.[1] });
      toast.success('VirusTotal scan completed');
      pushLog(
        'SUCCESS',
        `VT scan finished (manual): startup ${res?.[0] ?? 0}, registry ${res?.[1] ?? 0}.`,
        'Optimize'
      );
    } catch (e) {
      toast.error('VirusTotal scan failed (set API key?)');
      pushLog('ERROR', `VT scan failed (manual): ${String(e)}`, 'Optimize');
    } finally {
      vtBusy = false;
    }
  }
</script>

<Dialog bind:open>
  <DialogContent class="sm:max-w-xl">
    <div class="flex flex-col">
      <DialogHeader>
        <DialogTitle>Settings</DialogTitle>
        <DialogDescription>Configure Avelonia preferences.</DialogDescription>
      </DialogHeader>

      <div class="space-y-6">
        <section class="space-y-3">
          <p class="text-sm font-medium">Downloads</p>
          <div class="grid gap-3">
            <label class="flex items-center gap-2 text-sm">
              <Checkbox
                bind:checked={autoInstall}
                aria-controls="auto-install-advanced"
                aria-expanded={autoInstall}
              />
              <span>Auto-install downloaded installers</span>
            </label>

            {#if autoInstall}
              <div
                id="auto-install-advanced"
                class="rounded-md border border-border/60 bg-muted/10 p-3 sm:p-4 space-y-3 ml-0 sm:ml-4"
              >
                <div class="flex flex-col gap-2">
                  <Label class="text-xs text-muted-foreground">Install mode</Label>
                  <div class="max-w-[220px]">
                    <Select type="single" bind:value={installMode}>
                      <SelectTrigger placeholder="Select mode" />
                      <SelectContent>
                        <SelectItem value="silent">Silent</SelectItem>
                        <SelectItem value="normal">Normal</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>
                <label class="flex items-center gap-2 text-sm">
                  <Checkbox bind:checked={elevateInstall} />
                  <span>Run installers elevated (UAC)</span>
                </label>
                <label class="flex items-center gap-2 text-sm">
                  <Checkbox bind:checked={fallbackOpen} />
                  <span>Open normally if silent install fails</span>
                </label>
              </div>
            {/if}

            <label class="flex items-center gap-2 text-sm">
              <Checkbox bind:checked={verifyInstall} />
              <span>Verify installation in Uninstall registry</span>
            </label>
            <div class="space-y-2">
              <Label class="text-xs text-muted-foreground">Download catalog (.json)</Label>
              <Input
                value={catalogFilePath || 'Not configured'}
                readonly
                title={catalogFilePath || 'No catalog file selected'}
              />
              <div class="flex flex-wrap gap-2">
                <Button size="sm" variant="secondary" onclick={chooseCatalogFile}>
                  Choose JSON file
                </Button>
                {#if catalogFilePath}
                  <Button size="sm" variant="outline" onclick={clearCatalogFile}>Clear</Button>
                {/if}
              </div>
              <p class="text-xs text-muted-foreground">
                The selected JSON keeps the downloader catalog in sync with your saved paths.
              </p>
            </div>
          </div>
        </section>

        <section class="space-y-3">
          <p class="text-sm font-medium">Security / VirusTotal</p>
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground">API key</Label>
            <Input type="password" placeholder="Paste your VT API key" bind:value={vtKey} />
            <label class="flex items-center gap-2 text-sm">
              <Checkbox bind:checked={vtPersist} />
              <span>Save key on this device</span>
            </label>
            <div class="flex gap-2">
              <Button onclick={saveVtKey} disabled={vtBusy}>Save key</Button>
              <Button variant="secondary" onclick={runVtScanNow} disabled={!vtKeySet || vtBusy}
                >Run scan now</Button
              >
            </div>
            {#if !vtKeySet}
              <p class="text-xs text-muted-foreground">
                Set an API key to enable reputation scans.
              </p>
            {/if}
          </div>
        </section>

        <section class="space-y-3">
          <p class="text-sm font-medium">Privacy & Data</p>
          <div class="flex items-center justify-between gap-3">
            <div>
              <p class="text-sm">System logs</p>
              <p class="text-xs text-muted-foreground">Clear all logs stored locally.</p>
            </div>
            <Button
              variant="secondary"
              onclick={() => {
                try {
                  clearLogs();
                  toast.success('Logs cleared');
                } catch {
                  /* noop */
                }
              }}>Clear logs</Button
            >
          </div>
        </section>
      </div>

      <DialogFooter class="mt-6">
        <DialogClose>
          <Button variant="ghost">Close</Button>
        </DialogClose>
        <DialogClose>
          <Button>Done</Button>
        </DialogClose>
      </DialogFooter>
    </div>
  </DialogContent>
</Dialog>
