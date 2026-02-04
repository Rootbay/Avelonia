import { invoke } from '@tauri-apps/api/core';
import { pushLog } from '$lib/logStore';

export type StartupRegItem = { hive: string; key: string; name: string; command: string };

export type CleanupDiagnostics = {
  removedRegistry: { ok: string[]; stillPresent: string[] };
  runningImages: { running: string[]; stopped: string[] };
  taskMatches: { remaining: string[] };
  serviceMatches: { running: string[]; disabled: string[] };
  rebootRecommended: boolean;
};

export type RegistryAttempt = {
  attempts: number;
  rebootConfirmed: boolean;
  lastOptions: {
    force: boolean;
    ifeo: boolean;
    dor: boolean;
    purge: boolean;
    tasks: boolean;
    wmi: boolean;
  };
  lastStrategy?: 'basic' | 'force' | 'aggressive' | 'full';
  fullCleanupUsed?: boolean;
  pendingVerification?: boolean;
  suspicious?: boolean;
  suspiciousReason?: string;
  lastImages?: string[];
  lastPaths?: string[];
  lastSeenAt?: number;
};

export const regId = (it: StartupRegItem) => `${it.hive}|${it.key}|${it.name}`;

export async function listRegistryRun(): Promise<StartupRegItem[]> {
  const res = (await invoke('list_registry_run')) as StartupRegItem[];
  return Array.isArray(res) ? res : [];
}

export async function removeRegistryRun(entries: StartupRegItem[]): Promise<number> {
  return (await invoke('remove_registry_run', { entries })) as number;
}

export async function forceRemoveRegistryRun(entries: StartupRegItem[]): Promise<number> {
  return (await invoke('force_remove_registry_run', { entries })) as number;
}

export type ServiceInfo = {
  name: string;
  display_name: string;
  state: string;
  start_mode: string;
  path: string;
};

export async function listServices(): Promise<ServiceInfo[]> {
  const res = (await invoke('list_services')) as ServiceInfo[];
  return Array.isArray(res) ? res : [];
}

export async function stopServices(names: string[]): Promise<void> {
  await invoke('stop_services', { names });
}

export async function disableServices(names: string[]): Promise<number> {
  return (await invoke('disable_services', { names })) as number;
}

export async function blockProcessIfeo(images: string[], enable: boolean): Promise<number> {
  return (await invoke('block_process_ifeo', { images, enable })) as number;
}

export async function scheduleDeleteOnReboot(paths: string[]): Promise<number> {
  return (await invoke('schedule_delete_on_reboot', { paths })) as number;
}

export async function purgeStartupApproved(names: string[]): Promise<number> {
  return (await invoke('purge_startup_approved', { names })) as number;
}

export async function deleteTasksByMatch(images: string[], paths: string[]): Promise<number> {
  return (await invoke('delete_tasks_by_match', { images, paths })) as number;
}

export async function removeWmiSubscriptionsByMatch(
  images: string[],
  paths: string[]
): Promise<number> {
  return (await invoke('remove_wmi_subscriptions_by_match', { images, paths })) as number;
}

export async function isProcessRunning(image: string): Promise<boolean> {
  return (await invoke('is_process_running', { image })) as boolean;
}

export async function runPostCleanupDiagnostics(
  targets: StartupRegItem[],
  opts: { images: string[]; paths: string[]; rebootRecommended?: boolean }
): Promise<CleanupDiagnostics> {
  const diag: CleanupDiagnostics = {
    removedRegistry: { ok: [], stillPresent: [] },
    runningImages: { running: [], stopped: [] },
    taskMatches: { remaining: [] },
    serviceMatches: { running: [], disabled: [] },
    rebootRecommended: !!opts.rebootRecommended,
  };

  try {
    const list = await listRegistryRun();
    const present = new Set(list.map(regId));
    for (const t of targets) {
      const id = regId(t);
      const label = `${t.hive} \\ ${t.key} -> ${t.name}`;
      if (present.has(id)) diag.removedRegistry.stillPresent.push(label);
      else diag.removedRegistry.ok.push(label);
    }
  } catch (e) {
    pushLog('WARN', `diagnostics: list_registry_run failed: ${String(e)}`, 'Optimize');
  }

  for (const img of opts.images) {
    try {
      const running = await isProcessRunning(img);
      if (running) diag.runningImages.running.push(img);
      else diag.runningImages.stopped.push(img);
    } catch {
      /* noop */
    }
  }

  try {
    const tasks = (await invoke('list_scheduled_tasks')) as Array<{
      name: string;
      task_to_run?: string;
    }>;
    const matches: string[] = [];
    for (const t of tasks) {
      let cmd = t.task_to_run || '';
      if (!cmd) {
        try {
          const details = (await invoke('get_task_details', { task_name: t.name })) as
            | [string, string, boolean, number]
            | { task_to_run?: string };
          cmd = Array.isArray(details) ? (details[0] ?? '') : (details?.task_to_run ?? '');
        } catch {
          /* noop */
        }
      }
      const lower = (cmd || '').toLowerCase();
      if (!lower) continue;
      const hit =
        opts.images.some((i) => lower.includes(i.toLowerCase())) ||
        opts.paths.some((p) => lower.includes(p.toLowerCase()));
      if (hit) matches.push(t.name);
    }
    diag.taskMatches.remaining = Array.from(new Set(matches));
  } catch (e) {
    pushLog('WARN', `diagnostics: list_scheduled_tasks failed: ${String(e)}`, 'Optimize');
  }

  try {
    const services = (await invoke('list_services')) as Array<{
      name: string;
      state: string;
      start_mode: string;
      path: string;
    }>;
    for (const s of services) {
      const p = (s.path || '').toLowerCase();
      if (!p) continue;
      const hit =
        opts.images.some((i) => p.includes(i.toLowerCase())) ||
        opts.paths.some((q) => p.includes(q.toLowerCase()));
      if (!hit) continue;
      const state = (s.state || '').toLowerCase();
      const mode = (s.start_mode || '').toLowerCase();
      if (state.includes('running') || mode === 'auto' || mode === 'automatic')
        diag.serviceMatches.running.push(s.name);
      else diag.serviceMatches.disabled.push(s.name);
    }
    diag.serviceMatches.running = Array.from(new Set(diag.serviceMatches.running));
    diag.serviceMatches.disabled = Array.from(new Set(diag.serviceMatches.disabled));
  } catch (e) {
    pushLog('WARN', `diagnostics: list_services failed: ${String(e)}`, 'Optimize');
  }

  return diag;
}

export async function openRegistryKey(hive: string, key: string): Promise<void> {
  await invoke('open_registry_key', { hive, key });
}
