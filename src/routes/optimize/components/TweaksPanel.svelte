<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher, onMount } from 'svelte';
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
  import { pushLog } from '$lib/logStore';
  import { Loader2, CheckCircle2, AlertCircle, Circle } from '@lucide/svelte';
  import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
  } from '$lib/components/ui/alert-dialog';

  import {
    configItems,
    customizeTweaks,
    essentialTweaks,
    advancedTweaks,
    fixActions,
  } from '../tweakLibrary';

  type ProfileId = 'standard' | 'minimal' | 'performance' | 'custom';

  const profilePresets: Record<Exclude<ProfileId, 'custom'>, string[]> = {
    minimal: [
      'create_restore_point',
      'disable_consumer_features',
      'disable_telemetry',
      'disable_activity_history',
      'disable_recall',
      'disable_chat',
      'disable_task_view',
      'disable_widgets',
      'disable_search',
      'dark_theme',
    ],
    standard: [
      'create_restore_point',
      'disable_consumer_features',
      'disable_telemetry',
      'disable_activity_history',
      'disable_recall',
      'disable_explorer_discovery',
      'disable_gamedvr',
      'enable_end_task',
      'dark_theme',
      'show_file_extensions',
      'center_taskbar_items',
      'snap_window',
      'disable_chat',
      'disable_task_view',
      'disable_widgets',
      'disable_search',
    ],
    performance: [
      'create_restore_point',
      'disable_consumer_features',
      'disable_telemetry',
      'disable_activity_history',
      'disable_recall',
      'disable_gamedvr',
      'enable_end_task',
      'disable_background_apps',
      'disable_fullscreen_optimizations',
      'mouse_acceleration',
      'enable_game_mode',
      'enable_hags',
      'disable_nagle',
      'disable_network_throttling',
      'optimize_system_responsiveness',
      'optimize_game_priority',
      'ethernet_low_latency',
      'disable_power_throttling',
      'disable_hpet',
      'dark_theme',
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
      id: 'performance',
      label: 'Performance',
      description: 'Maximizes FPS, reduces network latency, disables mouse acceleration, and enables Game Mode/HAGS.',
    },
  ];

  const defaultProfile: ProfileId = 'standard';

  const dispatcher = createEventDispatcher<{ message: string }>();

  const allTweakItems = [...essentialTweaks, ...advancedTweaks, ...customizeTweaks];
  const ACTION_ONLY_TWEAKS = ['create_restore_point', 'run_disk_cleanup'];

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

  let tweakAppliedStates = $state<Record<string, 'on' | 'off' | 'applying' | 'failed'>>({});
  let isApplying = $state(false);
  let hasInitializedStates = $state(false);
  let isActivated = $state(true);
  let isWatermarkRemoved = $state(false);
  let showWatermarkConfirm = $state(false);

  async function refreshTweaksStatus() {
    try {
      const status = (await invoke('get_tweaks_status')) as Record<string, boolean>;
      const next: Record<string, 'on' | 'off' | 'applying' | 'failed'> = {};
      for (const [id, value] of Object.entries(status)) {
        next[id] = value ? 'on' : 'off';
      }
      tweakAppliedStates = next;

      if (!hasInitializedStates) {
        const nextTweaks = { ...tweakStates };
        for (const item of allTweakItems) {
          if (status[item.id] !== undefined) {
            nextTweaks[item.id] = status[item.id];
          }
        }
        tweakStates = nextTweaks;

        const nextConfigs = { ...configStates };
        for (const item of configItems) {
          if (status[item.id] !== undefined) {
            nextConfigs[item.id] = status[item.id];
          }
        }
        configStates = nextConfigs;

        hasInitializedStates = true;
      }
    } catch (error) {
      pushLog('ERROR', `Failed to query tweaks status: ${String(error)}`, 'Optimize');
    }
  }

  onMount(() => {
    void refreshTweaksStatus();
    invoke('is_windows_activated')
      .then((val) => {
        isActivated = val as boolean;
        pushLog('SUCCESS', `Windows activation status query returned: ${isActivated ? 'Activated' : 'Unactivated'}`, 'Optimize');
      })
      .catch((err) => {
        pushLog('ERROR', `Failed to query Windows activation status: ${String(err)}`, 'Optimize');
      });

    invoke('is_watermark_removed')
      .then((val) => {
        isWatermarkRemoved = val as boolean;
        pushLog('SUCCESS', `Windows watermark removal status check returned: ${isWatermarkRemoved ? 'Applied' : 'Not applied'}`, 'Optimize');
      })
      .catch((err) => {
        pushLog('ERROR', `Failed to query watermark status: ${String(err)}`, 'Optimize');
      });

    invoke('get_update_profile')
      .then((val) => {
        selectedUpdateProfile = val as UpdateProfileId;
        pushLog('SUCCESS', `Active Windows Update profile queried: ${selectedUpdateProfile}`, 'Optimize');
      })
      .catch((err) => {
        pushLog('ERROR', `Failed to query active update profile: ${String(err)}`, 'Optimize');
      });
  });

  function applyProfile(profile: Exclude<ProfileId, 'custom'>) {
    selectedProfile = profile;
    const preset = profilePresets[profile];
    const next: Record<string, boolean> = {};
    for (const item of allTweakItems) {
      next[item.id] = preset.includes(item.id) || tweakAppliedStates[item.id] === 'on';
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
    if (isApplying) return;

    const EXPLORER_RELATED_TWEAKS = [
      'disable_search',
      'disable_task_view',
      'disable_widgets',
      'disable_chat',
      'center_taskbar_items',
      'show_file_extensions',
      'show_hidden_files'
    ];

    const tweakChanges: Array<{ id: string; enabled: boolean }> = [];
    for (const item of allTweakItems) {
      const desired = !!tweakStates[item.id];
      const current = tweakAppliedStates[item.id] === 'on';

      if (ACTION_ONLY_TWEAKS.includes(item.id)) {
        if (desired) {
          tweakChanges.push({ id: item.id, enabled: true });
        }
      } else {
        if (desired !== current) {
          tweakChanges.push({ id: item.id, enabled: desired });
        }
      }
    }

    const configChanges: Array<{ id: string; enabled: boolean }> = [];
    for (const item of configItems) {
      const desired = !!configStates[item.id];
      const current = tweakAppliedStates[item.id] === 'on';

      if (desired !== current) {
        configChanges.push({ id: item.id, enabled: desired });
      }
    }

    const allChanges = [...tweakChanges, ...configChanges];

    if (allChanges.length === 0 && !selectedUpdateProfile) {
      toast.error('No tweaks or configs require state changes.');
      return;
    }

    const needsExplorerRestart = allChanges.some(change =>
      EXPLORER_RELATED_TWEAKS.includes(change.id)
    );

    isApplying = true;

    let elevated = false;
    try {
      elevated = (await invoke('is_elevated_command')) as boolean;
    } catch {
      elevated = false;
    }

    if (elevated) {
      for (const change of allChanges) {
        tweakAppliedStates[change.id] = 'applying';
        try {
          await invoke('apply_tweak_state', { id: change.id, enabled: change.enabled });
          tweakAppliedStates[change.id] = change.enabled ? 'on' : 'off';
          pushLog(
            'SUCCESS',
            `Successfully transitioned setting ${change.id} to ${change.enabled ? 'on' : 'off'}.`,
            'Optimize'
          );
        } catch (error) {
          tweakAppliedStates[change.id] = 'failed';
          pushLog(
            'ERROR',
            `Failed to transition setting ${change.id} to ${change.enabled ? 'on' : 'off'}: ${String(error)}`,
            'Optimize'
          );
        }
      }

      if (selectedUpdateProfile) {
        try {
          await invoke('apply_update_profile', { profile: selectedUpdateProfile });
          pushLog('SUCCESS', `Successfully set update profile to ${selectedUpdateProfile}.`, 'Optimize');
          toast.success(`Update profile set to ${selectedUpdateProfile}.`);
        } catch (error) {
          pushLog('ERROR', `Update profile failed: ${String(error)}`, 'Optimize');
          toast.error('Unable to change update profile.');
        }
      }

      if (needsExplorerRestart) {
        try {
          await invoke('restart_explorer');
          pushLog('SUCCESS', 'Successfully restarted Windows Explorer to reload registry changes.', 'Optimize');
          await new Promise((resolve) => setTimeout(resolve, 1500));
        } catch (error) {
          pushLog('ERROR', `Failed to restart Windows Explorer: ${String(error)}`, 'Optimize');
        }
      }

      await refreshTweaksStatus();
    } else {
      for (const change of allChanges) {
        tweakAppliedStates[change.id] = 'applying';
      }

      try {
        await invoke('apply_tweaks_state_batch', { changes: allChanges });
        pushLog(
          'SUCCESS',
          `Successfully applied tweaks batch: ${allChanges.map(c => `${c.id}=${c.enabled}`).join(', ')}`,
          'Optimize'
        );

        if (selectedUpdateProfile) {
          try {
            await invoke('apply_update_profile', { profile: selectedUpdateProfile });
            pushLog('SUCCESS', `Successfully set update profile to ${selectedUpdateProfile}.`, 'Optimize');
          } catch (error) {
            pushLog('ERROR', `Update profile failed: ${String(error)}`, 'Optimize');
          }
        }

        if (needsExplorerRestart) {
          try {
            await invoke('restart_explorer');
            pushLog('SUCCESS', 'Successfully restarted Windows Explorer to reload registry changes.', 'Optimize');
          } catch (error) {
            pushLog('ERROR', `Failed to restart Windows Explorer: ${String(error)}`, 'Optimize');
          }
        }

        toast.success(`Applying setting adjustments...`);
        await new Promise((resolve) => setTimeout(resolve, 2000));
        await refreshTweaksStatus();
      } catch (error) {
        for (const change of allChanges) {
          tweakAppliedStates[change.id] = 'failed';
        }
        pushLog('ERROR', `Failed to apply settings batch: ${String(error)}`, 'Optimize');
        toast.error('Unable to apply tweaks right now.');
      }
    }

    // Verification Step:
    const getLabel = (id: string) => {
      const item = allTweakItems.find(t => t.id === id) || configItems.find(c => c.id === id);
      return item ? item.label : id;
    };

    const successful: string[] = [];
    const failed: string[] = [];

    for (const change of allChanges) {
      if (ACTION_ONLY_TWEAKS.includes(change.id)) {
        // Skip verification check for action-only tweaks as they don't map to a queryable state
        continue;
      }

      const label = getLabel(change.id);
      const isAppliedNow = tweakAppliedStates[change.id] === 'on';

      if (isAppliedNow === change.enabled) {
        successful.push(label);
        pushLog(
          'SUCCESS',
          `Verified: Tweak "${label}" successfully applied. System state is ${isAppliedNow ? 'ON' : 'OFF'}.`,
          'Optimize'
        );
      } else {
        failed.push(label);
        tweakAppliedStates[change.id] = 'failed';
        pushLog(
          'ERROR',
          `Verification failed: Tweak "${label}" (${change.id}) was not applied. System state is ${isAppliedNow ? 'ON' : 'OFF'} but desired state was ${change.enabled ? 'ON' : 'OFF'}.`,
          'Optimize'
        );
      }
    }

    if (failed.length > 0) {
      toast.error(`Applied with warnings. Failed to set: ${failed.join(', ')}`);
      pushLog(
        'WARNING',
        `Batch execution completed with ${failed.length} failure(s). Successfully applied: ${successful.join(', ') || 'None'}. Failed: ${failed.join(', ')}.`,
        'Optimize'
      );
    } else if (successful.length > 0) {
      toast.success('All selected settings successfully applied!');
      pushLog(
        'SUCCESS',
        `Successfully verified all selected settings: ${successful.join(', ')}`,
        'Optimize'
      );
    }

    isApplying = false;
  }

  async function handleFixAction(id: string) {
    try {
      await invoke('run_fix_action', { actionId: id });
      pushLog('SUCCESS', `Successfully executed fix action: ${id}`, 'Optimize');
      fixStatus = { ...fixStatus, [id]: 'done' };
      if (id === 'remove_activation_watermark') {
        isWatermarkRemoved = true;
      }
      toast.success('Fix action queued.');
    } catch (error) {
      pushLog('ERROR', `Fix action failed: ${String(error)}`, 'Optimize');
      toast.error('Fix action failed.');
      fixStatus = { ...fixStatus, [id]: 'idle' };
    }
  }

  function triggerFixAction(id: string) {
    if (id === 'remove_activation_watermark') {
      showWatermarkConfirm = true;
    } else {
      void handleFixAction(id);
    }
  }

  function confirmWatermarkFix() {
    showWatermarkConfirm = false;
    void handleFixAction('remove_activation_watermark');
  }

  async function applyUpdateProfile(profile: UpdateProfileId) {
    try {
      await invoke('apply_update_profile', { profile });
      pushLog('SUCCESS', `Successfully set update profile to ${profile}`, 'Optimize');
      selectedUpdateProfile = profile;
      toast.success(`Update profile set to ${profile}.`);
      dispatcher('message', `Update profile set to ${profile}.`);
    } catch (error) {
      pushLog('ERROR', `Update profile failed: ${String(error)}`, 'Optimize');
      toast.error('Unable to change update profile.');
    }
  }
</script>

<div class="space-y-6">
  <Card>
    <CardHeader>
      <CardTitle class="text-lg">Recommended Profiles</CardTitle>
      <CardDescription>Choose a proven bundle of tweaks to prefill the lists below.</CardDescription
      >
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
              <span class="text-xs text-muted-foreground whitespace-normal"
                >{profile.description}</span
              >
            </div>
          </Button>
        {/each}
      </div>
      <p class="text-sm text-muted-foreground">
        Select a profile preset, then customize individual tweaks below.
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
              <span class="font-semibold flex items-center gap-1.5 flex-wrap">
                {tweak.label}
                {#if tweakAppliedStates[tweak.id] === 'applying'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-blue-500 font-medium">
                    <Loader2 class="size-3 animate-spin" /> Applying...
                  </span>
                {:else if tweakAppliedStates[tweak.id] === 'on'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-emerald-500 font-medium bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/20">
                    <CheckCircle2 class="size-3" /> Active
                  </span>
                {:else if tweakAppliedStates[tweak.id] === 'failed'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-destructive font-medium bg-destructive/10 px-1.5 py-0.5 rounded border border-destructive/20">
                    <AlertCircle class="size-3" /> Failed
                  </span>
                {:else if !ACTION_ONLY_TWEAKS.includes(tweak.id)}
                  <span class="inline-flex items-center gap-1 text-[10px] text-muted-foreground font-medium bg-muted px-1.5 py-0.5 rounded border border-border">
                    <Circle class="size-3" /> Inactive
                  </span>
                {/if}
              </span>
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
              <span class="font-semibold flex items-center gap-1.5 flex-wrap">
                {tweak.label}
                {#if tweakAppliedStates[tweak.id] === 'applying'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-blue-500 font-medium">
                    <Loader2 class="size-3 animate-spin" /> Applying...
                  </span>
                {:else if tweakAppliedStates[tweak.id] === 'on'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-emerald-500 font-medium bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/20">
                    <CheckCircle2 class="size-3" /> Active
                  </span>
                {:else if tweakAppliedStates[tweak.id] === 'failed'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-destructive font-medium bg-destructive/10 px-1.5 py-0.5 rounded border border-destructive/20">
                    <AlertCircle class="size-3" /> Failed
                  </span>
                {:else if !ACTION_ONLY_TWEAKS.includes(tweak.id)}
                  <span class="inline-flex items-center gap-1 text-[10px] text-muted-foreground font-medium bg-muted px-1.5 py-0.5 rounded border border-border">
                    <Circle class="size-3" /> Inactive
                  </span>
                {/if}
              </span>
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
              <span class="font-semibold flex items-center gap-1.5 flex-wrap">
                {tweak.label}
                {#if tweakAppliedStates[tweak.id] === 'applying'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-blue-500 font-medium">
                    <Loader2 class="size-3 animate-spin" /> Applying...
                  </span>
                {:else if tweakAppliedStates[tweak.id] === 'on'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-emerald-500 font-medium bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/20">
                    <CheckCircle2 class="size-3" /> Active
                  </span>
                {:else if tweakAppliedStates[tweak.id] === 'failed'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-destructive font-medium bg-destructive/10 px-1.5 py-0.5 rounded border border-destructive/20">
                    <AlertCircle class="size-3" /> Failed
                  </span>
                {:else if !ACTION_ONLY_TWEAKS.includes(tweak.id)}
                  <span class="inline-flex items-center gap-1 text-[10px] text-muted-foreground font-medium bg-muted px-1.5 py-0.5 rounded border border-border">
                    <Circle class="size-3" /> Inactive
                  </span>
                {/if}
              </span>
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
              <span class="font-semibold flex items-center gap-1.5 flex-wrap">
                {config.label}
                {#if tweakAppliedStates[config.id] === 'applying'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-blue-500 font-medium">
                    <Loader2 class="size-3 animate-spin" /> Applying...
                  </span>
                {:else if tweakAppliedStates[config.id] === 'on'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-emerald-500 font-medium bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/20">
                    <CheckCircle2 class="size-3" /> Active
                  </span>
                {:else if tweakAppliedStates[config.id] === 'failed'}
                  <span class="inline-flex items-center gap-1 text-[10px] text-destructive font-medium bg-destructive/10 px-1.5 py-0.5 rounded border border-destructive/20">
                    <AlertCircle class="size-3" /> Failed
                  </span>
                {:else if !ACTION_ONLY_TWEAKS.includes(config.id)}
                  <span class="inline-flex items-center gap-1 text-[10px] text-muted-foreground font-medium bg-muted px-1.5 py-0.5 rounded border border-border">
                    <Circle class="size-3" /> Inactive
                  </span>
                {/if}
              </span>
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
          {@const isRecommendedWatermark = action.id === 'remove_activation_watermark' && !isActivated && !isWatermarkRemoved}
          <div class="flex flex-col gap-2 rounded-md border px-3 py-2 transition-colors {isRecommendedWatermark ? 'border-emerald-500/30 bg-emerald-500/5 dark:bg-emerald-500/10' : 'border-border/60'}">
            <div class="flex items-center justify-between gap-3">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="font-semibold">{action.label}</span>
                {#if isRecommendedWatermark}
                  <span class="inline-flex items-center gap-0.5 rounded bg-emerald-500/20 px-1.5 py-0.5 text-[10px] font-medium tracking-wide uppercase text-emerald-600 dark:text-emerald-400">
                    Recommended
                  </span>
                {/if}
              </div>
              <Button
                size="sm"
                variant={fixStatus[action.id] === 'done' || (action.id === 'remove_activation_watermark' && isWatermarkRemoved) ? 'secondary' : (isRecommendedWatermark ? 'default' : 'outline')}
                class={isRecommendedWatermark && fixStatus[action.id] !== 'done' ? 'bg-emerald-600 hover:bg-emerald-700 text-white dark:bg-emerald-500 dark:hover:bg-emerald-600 border-none' : ''}
                onclick={() => triggerFixAction(action.id)}
              >
                {fixStatus[action.id] === 'done' || (action.id === 'remove_activation_watermark' && isWatermarkRemoved) ? 'Applied' : 'Run Fix'}
              </Button>
            </div>
            <p class="text-xs {isRecommendedWatermark ? 'text-emerald-700/80 dark:text-emerald-400/80' : 'text-muted-foreground'}">{action.description}</p>
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
              <span class="text-xs text-muted-foreground whitespace-normal"
                >{profile.description}</span
              >
            </div>
          </Button>
        {/each}
      </div>
      <p class="text-sm text-muted-foreground">
        Profiles adjust deferred update policies in the backend; run the matching service tweaks to
        apply them.
      </p>
    </CardContent>
  </Card>

  <AlertDialog open={showWatermarkConfirm} onOpenChange={(v) => (showWatermarkConfirm = !!v)}>
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>Restart Required</AlertDialogTitle>
        <AlertDialogDescription>
          Applying the activation watermark fix requires a system restart (reboot) to take effect fully.
          <br /><br />
          Your screen and Windows Explorer will restart briefly during the process, but the watermark will only disappear permanently after you reboot your computer.
          <br /><br />
          Do you want to continue?
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel onclick={() => (showWatermarkConfirm = false)}>Cancel</AlertDialogCancel>
        <AlertDialogAction onclick={confirmWatermarkFix}>Continue</AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</div>
