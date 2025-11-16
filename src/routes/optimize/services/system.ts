import { invoke } from '@tauri-apps/api/core';

export async function getBootTime(): Promise<number> {
  return (await invoke('get_boot_time')) as number;
}

export async function restartSystem(): Promise<void> {
  await invoke('restart_system');
}
