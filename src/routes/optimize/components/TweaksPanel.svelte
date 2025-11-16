<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from '$lib/components/ui/card';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { toast } from '$lib/components/ui/sonner';
  import {
    configItems,
    customizeTweaks,
    essentialTweaks,
    advancedTweaks,
    fixActions,
  } from '../tweakLibrary';

  type ProfileId = 'standard' | 'minimal' | 'privacy' | 'custom';

  const profilePresets: Record<Exclude<ProfileId, 'custom'>, string[]> = {
    standard: [
      ...essentialTweaks.map((item) => item.id),
      'dark_theme',
      'show_hidden_files',
      'show_file_extensions',
      'center_taskbar_items',
      'snap_window',
    ],
    minimal: [
      'create_restore_point',
      'disable_consumer_features',
      'disable_telemetry',
      'disable_activity_history',
      'disable_storage_sense',
      'disable_location_tracking',
      'disable_wifi_sense',
      'disable_recall',
    ],
    privacy: [
      ...essentialTweaks.map((item) => item.id),
      ...advancedTweaks.map((item) => item.id),
      'remove_settings_home',
      'disable_background_apps',
      'prefer_ipv4',
    ],
  };

  const profileOptions: Array<{
    id: Exclude<ProfileId, 'custom'>;
    label: string;
    description: string;
  }> = [
    {
      id: 'standard',
      label: 'Standard',
      description: 'Balanced tweak set with essentials plus friendly preferences.',
    },
    {
      id: 'minimal',
      label: 'Minimal',
      description: 'Only the most essential privacy/security adjustments.',
    },
    {
      id: 'privacy',
      label: 'Privacy',
      description: 'Locks down telemetry, networking, and background services.',
    },
  ];

  const defaultProfile: ProfileId = 'standard';

  const dispatcher = createEventDispatcher<{ message: string }>();

  const allTweakItems = [...essentialTweaks, ...advancedTweaks, ...customizeTweaks];

  const createTweakState = (): Record<string, boolean> => {
    const base: Record<string, boolean> = {};
    for (const item of allTweakItems) {
      base[item.id] = profilePresets[defaultProfile].includes(item.id);
    }
    return base;
  };

  const createConfigState = (): Record<string, boolean> => {
    const base: Record<string, boolean> = {};
    for (const item of configItems) {
      base[item.id] = false;
    }
    return base;
  };

  const createFixStatus = (): Record<string, 'idle' | 'done'> => {
    const base: Record<string, 'idle' | 'done'> = {};
    for (const action of fixActions) {
      base[action.id] = 'idle';
    }
    return base;
  };

  type UpdateProfileId = 'default' | 'security' | 'disable';

  const updateProfileOptions: Array<{
    id: UpdateProfileId;
    label: string;
    description: string;
  }> = [
    {
      id: 'default',
      label: 'Default',
      description: "Let Windows Update follow Microsoft's normal cadence.",
    },
    {
      id: 'security',
      label: 'Security',
      description: 'Delay feature updates by 2 years and security updates by 4 days.',
    },
    {
      id: 'disable',
      label: 'Disable All Updates',
      description: 'Pause updates entirely until this profile changes.',
    },
  ];

  let selectedProfile = $state<ProfileId>(defaultProfile);
  let tweakStates = $state<Record<string, boolean>>(createTweakState());
  let configStates = $state<Record<string, boolean>>(createConfigState());
  let fixStatus = $state<Record<string, 'idle' | 'done'>>(createFixStatus());
  let selectedUpdateProfile = $state<UpdateProfileId>('default');

  function applyProfile(profile: Exclude<ProfileId, 'custom'>) {
    selectedProfile = profile;
    const preset = profilePresets[profile];
    const next: Record<string, boolean> = {};
    for (const item of allTweakItems) {
      next[item.id] = preset.includes(item.id);
    }
    tweakStates = next;
  }

  function handleTweakToggle(id: string, value: boolean) {
    selectedProfile = 'custom';
    tweakStates = { ...tweakStates, [id]: value };
  }

  function toggleConfig(id: string) {
    configStates = { ...configStates, [id]: !configStates[id] };
  }

  async function applySelectedTweaks() {
    const selectedTweaks = Object.entries(tweakStates)
      .filter(([, value]) => value)
      .map(([key]) => key);
    const selectedConfigs = Object.entries(configStates)
      .filter(([, value]) => value)
      .map(([key]) => key);
    try {
      const res = (await invoke('apply_tweaks', {
        tweaks: selectedTweaks,
        configs: selectedConfigs,
        updateProfile: selectedUpdateProfile,
      })) as { tweaks_applied: number; configs_applied: number };
      toast.success(
        `Queued ${res.tweaks_applied} tweak(s) and ${res.configs_applied} config(s).`
      );
    } catch (error) {
      console.error('Failed to apply tweaks', error);
      toast.error('Unable to apply tweaks right now.');
    }
  }

  async function handleFixAction(id: string) {
    try {
      await invoke('run_fix_action', { actionId: id });
      fixStatus = { ...fixStatus, [id]: 'done' };
      toast.success('Fix action queued.');
    } catch (error) {
      console.error('Fix action failed', error);
      toast.error('Fix action failed.');
      fixStatus = { ...fixStatus, [id]: 'idle' };
    }
  }

  async function applyUpdateProfile(profile: UpdateProfileId) {
    try {
      await invoke('apply_update_profile', { profile });
      selectedUpdateProfile = profile;
      toast.success(`Update profile set to ${profile}.`);
      dispatcher('message', `Update profile set to ${profile}.`);
    } catch (error) {
      console.error('Update profile failed', error);
      toast.error('Unable to change update profile.');
    }
  }
</script>

<div class="space-y-6">
  <Card>
    <CardHeader>
      <CardTitle class="text-lg">Recommended Profiles</CardTitle>
      <CardDescription>Choose a proven bundle of tweaks to prefill the lists below.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div class="flex flex-wrap gap-2">
        {#each profileOptions as profile (profile.id)}
        <Button
            size="lg"
            variant={selectedProfile === profile.id ? 'secondary' : 'outline'}
            class="flex-1 min-w-[170px] justify-start px-2 py-5 h-auto!"
            onclick={() => applyProfile(profile.id)}
          >
            <div class="flex flex-col items-start gap-1 text-left">
              <span class="text-sm font-semibold">{profile.label}</span>
              <span class="text-xs text-muted-foreground whitespace-normal">{profile.description}</span>
            </div>
          </Button>
        {/each}
      </div>
      <p class="text-sm text-muted-foreground">
        Start with a recommended profile, then toggle the individual tweaks below to refine the behavior.
      </p>
      <div class="flex gap-2">
        <Button size="sm" variant="secondary" onclick={applySelectedTweaks}>
          Apply Selected Tweaks
        </Button>
      </div>
    </CardContent>
  </Card>

  <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
    <Card>
      <CardHeader>
        <CardTitle>Essential Tweaks</CardTitle>
        <CardDescription>Baseline adjustments before unlocking advanced options.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-3 max-h-[440px] overflow-y-auto pr-1">
        {#each essentialTweaks as tweak (tweak.id)}
          <label
            class="flex gap-3 rounded-md px-2 py-2 transition hover:bg-muted/30"
            aria-label={tweak.label}
          >
            <Checkbox
              checked={tweakStates[tweak.id]}
              onCheckedChange={(value) => handleTweakToggle(tweak.id, !!value)}
            />
            <div class="flex flex-col gap-1 text-sm">
              <span class="font-semibold">{tweak.label}</span>
              <span class="text-xs text-muted-foreground leading-tight">{tweak.description}</span>
            </div>
          </label>
        {/each}
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <CardTitle>Advanced Tweaks</CardTitle>
        <CardDescription>Deeper system, network, and service changes.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-3 max-h-[440px] overflow-y-auto pr-1">
        {#each advancedTweaks as tweak (tweak.id)}
          <label
            class="flex gap-3 rounded-md px-2 py-2 transition hover:bg-muted/30"
            aria-label={tweak.label}
          >
            <Checkbox
              checked={tweakStates[tweak.id]}
              onCheckedChange={(value) => handleTweakToggle(tweak.id, !!value)}
            />
            <div class="flex flex-col gap-1 text-sm">
              <span class="font-semibold">{tweak.label}</span>
              <span class="text-xs text-muted-foreground leading-tight">{tweak.description}</span>
            </div>
          </label>
        {/each}
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <CardTitle>Customize Preferences</CardTitle>
        <CardDescription>Make the desktop behave the way you like it.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-3 max-h-[460px] overflow-y-auto pr-1">
        {#each customizeTweaks as tweak (tweak.id)}
          <label
            class="flex gap-3 rounded-md px-2 py-2 transition hover:bg-muted/30"
            aria-label={tweak.label}
          >
            <Checkbox
              checked={tweakStates[tweak.id]}
              onCheckedChange={(value) => handleTweakToggle(tweak.id, !!value)}
            />
            <div class="flex flex-col gap-1 text-sm">
              <span class="font-semibold">{tweak.label}</span>
              <span class="text-xs text-muted-foreground leading-tight">{tweak.description}</span>
            </div>
          </label>
        {/each}
      </CardContent>
    </Card>
  </div>

  <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
    <Card>
      <CardHeader>
        <CardTitle>Configuration & Features</CardTitle>
        <CardDescription>Toggle Windows capabilities and legacy components.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-3 max-h-[520px] overflow-y-auto pr-1">
        {#each configItems as config (config.id)}
          <label
            class="flex gap-3 rounded-md px-2 py-2 transition hover:bg-muted/30"
            aria-label={config.label}
          >
            <Checkbox
              checked={configStates[config.id]}
              onCheckedChange={() => toggleConfig(config.id)}
            />
            <div class="flex flex-col gap-1 text-sm">
              <span class="font-semibold">{config.label}</span>
              <span class="text-xs text-muted-foreground leading-tight">
                {config.description}
              </span>
            </div>
          </label>
        {/each}
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <CardTitle>Fixes</CardTitle>
        <CardDescription>Quick remediation actions that can be run on demand.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        {#each fixActions as action (action.id)}
          <div class="flex flex-col gap-2 rounded-md border border-border/60 px-3 py-2">
            <div class="flex items-center justify-between gap-3">
              <span class="font-semibold">{action.label}</span>
              <Button
                size="sm"
                variant={fixStatus[action.id] === 'done' ? 'secondary' : 'outline'}
                onclick={() => handleFixAction(action.id)}
              >
                {fixStatus[action.id] === 'done' ? 'Done' : 'Run Fix'}
              </Button>
            </div>
            <p class="text-xs text-muted-foreground">{action.description}</p>
          </div>
        {/each}
      </CardContent>
    </Card>
  </div>

  <Card>
    <CardHeader>
      <CardTitle>Windows Update Profiles</CardTitle>
      <CardDescription>Define how Windows Update behaves on this device.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-3">
      <div class="flex flex-wrap gap-2">
        {#each updateProfileOptions as profile (profile.id)}
        <Button
            size="sm"
            variant={selectedUpdateProfile === profile.id ? 'secondary' : 'outline'}
            class="flex-1 min-w-[140px] justify-start py-3 h-auto!"
            onclick={() => applyUpdateProfile(profile.id)}
          >
            <div class="flex flex-col items-start gap-1 text-left">
              <span class="text-sm font-semibold">{profile.label}</span>
              <span class="text-xs text-muted-foreground whitespace-normal">{profile.description}</span>
            </div>
          </Button>
        {/each}
      </div>
      <p class="text-sm text-muted-foreground">
        Profiles adjust deferred update policies in the backend; run the matching service tweaks to apply them.
      </p>
    </CardContent>
  </Card>
</div>
