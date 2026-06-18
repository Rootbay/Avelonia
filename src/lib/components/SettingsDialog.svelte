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
  import { settings, updateDownloaderSettings, updateLanguage } from '$lib/settings';
  import { invoke } from '@tauri-apps/api/core';
  import { toast } from '$lib/components/ui/sonner';
  import { pushLog, clearLogs } from '$lib/logStore';
  import { save } from '@tauri-apps/plugin-dialog';
  import { beginScan, endScan } from '$lib/scanStatus.svelte';
  import { i18n } from '$lib/i18n.svelte';

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
  let language = $state($settings.language);

  $effect(() => {
    autoInstall = $settings.downloader.autoInstall;
    installMode = $settings.downloader.installMode;
    elevateInstall = $settings.downloader.elevate;
    fallbackOpen = $settings.downloader.fallbackOpen;
    verifyInstall = $settings.downloader.verifyInstall;
    catalogFilePath = $settings.downloader.downloadCatalogPath ?? '';
    language = $settings.language;
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
    updateLanguage(language);
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
      toast.success(i18n.t('settings.toast_vt_saved'));
      pushLog('SUCCESS', `VT key saved${vtPersist ? ' (persisted)' : ''}.`, 'Optimize');
    } catch (e) {
      toast.error(i18n.t('settings.toast_vt_save_failed'));
      pushLog('ERROR', `Saving VT key failed: ${String(e)}`, 'Optimize');
    } finally {
      vtBusy = false;
    }
  }

  async function chooseCatalogFile() {
    try {
      const selected = await save({
        title: i18n.t('settings.choose_file'),
        filters: [{ name: 'JSON', extensions: ['json'] }],
        defaultPath: catalogFilePath || 'avelonia-downloads.json',
      });
      if (typeof selected === 'string' && selected) {
        updateDownloaderSettings({ downloadCatalogPath: selected });
        toast.success(i18n.t('settings.toast_catalog_selected'));
      }
    } catch (error) {
      pushLog('ERROR', `Failed to select catalog file: ${String(error)}`, 'Downloader');
      toast.error('Unable to pick a catalog file');
    }
  }

  function clearCatalogFile() {
    if (!catalogFilePath) return;
    updateDownloaderSettings({ downloadCatalogPath: '' });
    toast.info(i18n.t('settings.toast_catalog_clear'));
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
        <DialogTitle>{i18n.t('settings.title')}</DialogTitle>
        <DialogDescription>{i18n.t('settings.description')}</DialogDescription>
      </DialogHeader>

      <div class="space-y-6">
        <section class="space-y-3">
          <p class="text-sm font-medium">{i18n.t('settings.language')}</p>
          <div class="flex flex-col gap-2">
            <Label class="text-xs text-muted-foreground">{i18n.t('settings.select_language')}</Label
            >
            <div class="max-w-[220px]">
              <Select type="single" bind:value={language}>
                <SelectTrigger class="w-44">
                  <p>
                    {language === 'en'
                      ? 'English'
                      : language === 'sv'
                        ? 'Svenska'
                        : language === 'de'
                          ? 'Deutsch'
                          : language === 'fr'
                            ? 'Français'
                            : language === 'es'
                              ? 'Español'
                              : 'English'}
                  </p>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="en">English</SelectItem>
                  <SelectItem value="sv">Svenska</SelectItem>
                  <SelectItem value="de">Deutsch</SelectItem>
                  <SelectItem value="fr">Français</SelectItem>
                  <SelectItem value="es">Español</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </section>

        <section class="space-y-3">
          <p class="text-sm font-medium">{i18n.t('settings.downloads')}</p>
          <div class="grid gap-3">
            <label class="flex items-center gap-2 text-sm">
              <Checkbox
                bind:checked={autoInstall}
                aria-controls="auto-install-advanced"
                aria-expanded={autoInstall}
              />
              <span>{i18n.t('settings.auto_install')}</span>
            </label>

            {#if autoInstall}
              <div
                id="auto-install-advanced"
                class="rounded-md border border-border/60 bg-muted/10 p-3 sm:p-4 space-y-3 ml-0 sm:ml-4"
              >
                <div class="flex flex-col gap-2">
                  <Label class="text-xs text-muted-foreground"
                    >{i18n.t('settings.install_mode')}</Label
                  >
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
                  <span>{i18n.t('settings.elevate')}</span>
                </label>
                <label class="flex items-center gap-2 text-sm">
                  <Checkbox bind:checked={fallbackOpen} />
                  <span>{i18n.t('settings.fallback')}</span>
                </label>
              </div>
            {/if}

            <label class="flex items-center gap-2 text-sm">
              <Checkbox bind:checked={verifyInstall} />
              <span>{i18n.t('settings.verify')}</span>
            </label>
            <div class="space-y-2">
              <Label class="text-xs text-muted-foreground">{i18n.t('settings.catalog')}</Label>
              <Input
                value={catalogFilePath || 'Not configured'}
                readonly
                title={catalogFilePath || 'No catalog file selected'}
              />
              <div class="flex flex-wrap gap-2">
                <Button size="sm" variant="secondary" onclick={chooseCatalogFile}>
                  {i18n.t('settings.choose_file')}
                </Button>
                {#if catalogFilePath}
                  <Button size="sm" variant="outline" onclick={clearCatalogFile}
                    >{i18n.t('settings.clear')}</Button
                  >
                {/if}
              </div>
              <p class="text-xs text-muted-foreground">
                {i18n.t('settings.catalog_desc')}
              </p>
            </div>
          </div>
        </section>

        <section class="space-y-3">
          <p class="text-sm font-medium">{i18n.t('settings.security_vt')}</p>
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground">{i18n.t('settings.api_key')}</Label>
            <Input type="password" placeholder="Paste your VT API key" bind:value={vtKey} />
            <label class="flex items-center gap-2 text-sm">
              <Checkbox bind:checked={vtPersist} />
              <span>{i18n.t('settings.save_key')}</span>
            </label>
            <div class="flex gap-2">
              <Button onclick={saveVtKey} disabled={vtBusy}
                >{i18n.t('settings.btn_save_key')}</Button
              >
              <Button variant="secondary" onclick={runVtScanNow} disabled={!vtKeySet || vtBusy}
                >{i18n.t('settings.btn_scan_now')}</Button
              >
            </div>
            {#if !vtKeySet}
              <p class="text-xs text-muted-foreground">
                {i18n.t('settings.vt_desc')}
              </p>
            {/if}
          </div>
        </section>

        <section class="space-y-3">
          <p class="text-sm font-medium">{i18n.t('settings.privacy_data')}</p>
          <div class="flex items-center justify-between gap-3">
            <div>
              <p class="text-sm">{i18n.t('settings.system_logs')}</p>
              <p class="text-xs text-muted-foreground">{i18n.t('settings.clear_logs_desc')}</p>
            </div>
            <Button
              variant="secondary"
              onclick={() => {
                try {
                  clearLogs();
                  toast.success(i18n.t('settings.toast_logs_cleared'));
                } catch {
                  /* noop */
                }
              }}>{i18n.t('settings.btn_clear_logs')}</Button
            >
          </div>
        </section>
      </div>

      <DialogFooter class="mt-6">
        <DialogClose>
          <Button variant="ghost">{i18n.t('common.close')}</Button>
        </DialogClose>
        <DialogClose>
          <Button>{i18n.t('common.done')}</Button>
        </DialogClose>
      </DialogFooter>
    </div>
  </DialogContent>
</Dialog>
