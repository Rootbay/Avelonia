import { invoke } from '@tauri-apps/api/core';

export type ScheduledTask = {
  name: string;
  next_run_time: string;
  status: string;
  task_to_run: string;
  author: string;
  is_sus: boolean;
  score: number;
};

export type TaskAction = 'disable' | 'enable' | 'delete' | 'run' | 'end' | '';
export type TaskActionFailure = { stderr?: string; stdout?: string };
export type TaskActionResult = {
  success?: number;
  elevated?: number;
  stopped?: number;
  failures?: TaskActionFailure[];
};

export async function listScheduledTasks(): Promise<ScheduledTask[]> {
  const res = (await invoke('list_scheduled_tasks')) as ScheduledTask[];
  return Array.isArray(res) ? res : [];
}

export async function listSuspiciousTasks(): Promise<string[]> {
  const res = (await invoke('list_suspicious_tasks')) as string[];
  return Array.isArray(res) ? res : [];
}

export async function getTaskDetails(taskName: string): Promise<[string, string, boolean, number]> {
  return (await invoke('get_task_details', { task_name: taskName })) as [
    string,
    string,
    boolean,
    number,
  ];
}

export async function executeTaskAction(
  action: TaskAction,
  names: string[]
): Promise<TaskActionResult | number | null> {
  if (!action || names.length === 0) return null;
  if (action === 'disable') return await invoke('disable_scheduled_tasks', { names });
  if (action === 'enable') return await invoke('enable_scheduled_tasks', { names });
  if (action === 'delete') return await invoke('delete_scheduled_tasks', { names });
  if (action === 'run') return await invoke('run_scheduled_tasks', { names });
  if (action === 'end') return await invoke('end_scheduled_tasks', { names });
  return null;
}
