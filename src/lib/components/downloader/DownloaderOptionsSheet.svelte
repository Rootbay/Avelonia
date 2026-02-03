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
      <SheetTitle>Beta options</SheetTitle>
      <SheetDescription>Defaults for post-download behavior.</SheetDescription>
    </SheetHeader>
    <div class="mt-3 space-y-6 text-sm">
      <div class="space-y-2">
        <p class="font-medium">Install after download</p>
        <label class="inline-flex items-center gap-2">
          <Checkbox bind:checked={autoInstall} class="h-4 w-4" />
          Auto install after download
        </label>
      </div>
      <div class="space-y-2">
        <p class="font-medium">Install mode</p>
        <Select type="single" bind:value={installMode}>
          <SelectTrigger class="w-44">
            <p>
              {installMode === 'silent' ? 'Silent' : 'Normal'}
            </p>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="silent">Silent</SelectItem>
            <SelectItem value="normal">Normal</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="space-y-2">
        <p class="font-medium">Advanced</p>
        <label class="inline-flex items-center gap-2">
          <Checkbox bind:checked={elevateInstall} class="h-4 w-4" />
          Run elevated (may prompt UAC)
        </label>
        <label class="inline-flex items-center gap-2">
          <Checkbox bind:checked={fallbackOpen} class="h-4 w-4" />
          If silent fails, open installer normally
        </label>
      </div>
      <div class="flex flex-wrap gap-2">
        <Button variant="secondary" size="sm" onclick={() => onShowInfo('install')}
          >Silent install?</Button
        >
        <Button variant="secondary" size="sm" onclick={() => onShowInfo('verify')}
          >Verify install?</Button
        >
      </div>

      <div class="space-y-2">
        <p class="font-medium">Verification</p>
        <label class="inline-flex items-center gap-2">
          <Checkbox bind:checked={verifyInstall} class="h-4 w-4" />
          Verify installation via system registry (Windows)
        </label>
        <p class="text-xs text-muted-foreground">
          Checks Uninstall entries after installer exits; helps confirm success.
        </p>
      </div>
    </div>
  </SheetContent>
</Sheet>
