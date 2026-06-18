<script lang="ts">
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Button } from '$lib/components/ui/button';
  import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
  import {
    Sheet,
    SheetContent,
    SheetDescription,
    SheetHeader,
    SheetTitle,
  } from '$lib/components/ui/sheet';
  import { settings, updateDownloaderSettings } from '$lib/settings';
  import { i18n } from '$lib/i18n.svelte';

  let { open = $bindable(false), onShowInfo } = $props<{
    open: boolean;
    onShowInfo: (type: 'install' | 'verify') => void;
  }>();

  let autoInstall = $state($settings.downloader.autoInstall);
  let installMode = $state($settings.downloader.installMode);
  let elevateInstall = $state($settings.downloader.elevate);
  let fallbackOpen = $state($settings.downloader.fallbackOpen);
  let verifyInstall = $state($settings.downloader.verifyInstall);

  $effect(() => {
    autoInstall = $settings.downloader.autoInstall;
    installMode = $settings.downloader.installMode;
    elevateInstall = $settings.downloader.elevate;
    fallbackOpen = $settings.downloader.fallbackOpen;
    verifyInstall = $settings.downloader.verifyInstall;
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
</script>

<Sheet bind:open>
  <SheetContent side="right" class="w-85 sm:w-95 p-4 sm:p-6">
    <SheetHeader class="space-y-1 p-0">
      <SheetTitle>{i18n.t('downloader.options_beta_title')}</SheetTitle>
      <SheetDescription>{i18n.t('downloader.options_beta_desc')}</SheetDescription>
    </SheetHeader>
    <div class="mt-3 space-y-6 text-sm">
      <div class="space-y-2">
        <p class="font-medium">{i18n.t('downloader.options_install_after')}</p>
        <label class="inline-flex items-center gap-2">
          <Checkbox bind:checked={autoInstall} class="h-4 w-4" />
          {i18n.t('downloader.options_auto_install')}
        </label>
      </div>
      <div class="space-y-2">
        <p class="font-medium">{i18n.t('downloader.options_install_mode')}</p>
        <Select type="single" bind:value={installMode}>
          <SelectTrigger class="w-44">
            <p>
              {installMode === 'silent' ? i18n.t('downloader.options_mode_silent') : i18n.t('downloader.options_mode_normal')}
            </p>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="silent">{i18n.t('downloader.options_mode_silent')}</SelectItem>
            <SelectItem value="normal">{i18n.t('downloader.options_mode_normal')}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="space-y-2">
        <p class="font-medium">{i18n.t('downloader.options_advanced')}</p>
        <label class="inline-flex items-center gap-2">
          <Checkbox bind:checked={elevateInstall} class="h-4 w-4" />
          {i18n.t('downloader.options_run_elevated')}
        </label>
        <label class="inline-flex items-center gap-2">
          <Checkbox bind:checked={fallbackOpen} class="h-4 w-4" />
          {i18n.t('downloader.options_fallback')}
        </label>
      </div>
      <div class="flex flex-wrap gap-2">
        <Button variant="secondary" size="sm" onclick={() => onShowInfo('install')}
          >{i18n.t('downloader.options_btn_silent_q')}</Button
        >
        <Button variant="secondary" size="sm" onclick={() => onShowInfo('verify')}
          >{i18n.t('downloader.options_btn_verify_q')}</Button
        >
      </div>

      <div class="space-y-2">
        <p class="font-medium">{i18n.t('downloader.options_verification')}</p>
        <label class="inline-flex items-center gap-2">
          <Checkbox bind:checked={verifyInstall} class="h-4 w-4" />
          {i18n.t('downloader.options_verify_registry')}
        </label>
        <p class="text-xs text-muted-foreground">
          {i18n.t('downloader.options_verify_desc')}
        </p>
      </div>
    </div>
  </SheetContent>
</Sheet>
