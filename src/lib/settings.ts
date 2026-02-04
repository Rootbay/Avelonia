import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { pushLog } from '$lib/logStore';

export type DownloaderSettings = {
  autoInstall: boolean;
  installMode: 'silent' | 'normal';
  elevate: boolean;
  fallbackOpen: boolean;
  verifyInstall: boolean;
  preferWinget: boolean;
  downloadCatalogPath: string;
};

export type AppSettings = {
  downloader: DownloaderSettings;
};

const STORAGE_KEY = 'avelonia_settings_v1';
const SETTINGS_WRITE_DEBOUNCE_MS = 200;

const defaultSettings: AppSettings = {
  downloader: {
    autoInstall: false,
    installMode: 'silent',
    elevate: true,
    fallbackOpen: true,
    verifyInstall: false,
    preferWinget: false,
    downloadCatalogPath: '',
  },
};

let initialized = false;
let persistEnabled = false;
let persistTimer: ReturnType<typeof setTimeout> | null = null;

function logSettingsError(context: string, error: unknown) {
  pushLog('WARN', `Settings ${context} failed: ${String(error)}`, 'System');
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function normalizeSettings(parsed: unknown): AppSettings {
  const p =
    typeof parsed === 'object' && parsed !== null
      ? (parsed as { downloader?: Partial<DownloaderSettings> })
      : {};
  const d = p.downloader ?? {};
  const normalized: DownloaderSettings = {
    autoInstall: !!d.autoInstall,
    installMode: d.installMode === 'normal' ? 'normal' : 'silent',
    elevate: d.elevate ?? true,
    fallbackOpen: d.fallbackOpen ?? true,
    verifyInstall: d.verifyInstall ?? false,
    preferWinget: d.preferWinget ?? false,
    downloadCatalogPath: typeof d.downloadCatalogPath === 'string' ? d.downloadCatalogPath : '',
  };
  return { downloader: normalized };
}

function loadFromLocalStorage(): AppSettings | null {
  if (typeof window === 'undefined') return null;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    return normalizeSettings(parsed);
  } catch (error) {
    logSettingsError('localStorage load', error);
    return null;
  }
}

async function loadFromBackend(): Promise<AppSettings | null> {
  try {
    const raw = (await invoke('settings_read')) as string | null;
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    return normalizeSettings(parsed);
  } catch (error) {
    logSettingsError('backend load', error);
    return null;
  }
}

async function persistToBackend(s: AppSettings) {
  try {
    await invoke('settings_write', { contents: JSON.stringify(s) });
  } catch (error) {
    logSettingsError('backend persist', error);
  }
}

function persistToLocalStorage(s: AppSettings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
  } catch (error) {
    logSettingsError('localStorage persist', error);
  }
}

async function initSettings() {
  if (initialized || typeof window === 'undefined') return;
  initialized = true;

  let loaded: AppSettings | null = null;
  if (isTauriRuntime()) {
    loaded = await loadFromBackend();
    if (!loaded) {
      const migrated = loadFromLocalStorage();
      if (migrated) {
        loaded = migrated;
        void persistToBackend(migrated);
      }
    }
  } else {
    loaded = loadFromLocalStorage();
  }

  if (loaded) {
    settings.set(loaded);
  }
  persistEnabled = true;
}

export const settings = writable<AppSettings>(defaultSettings);

void initSettings();

settings.subscribe((s) => {
  if (typeof window === 'undefined') return;
  if (!persistEnabled) return;
  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    if (isTauriRuntime()) {
      void persistToBackend(s);
    } else {
      persistToLocalStorage(s);
    }
  }, SETTINGS_WRITE_DEBOUNCE_MS);
});

export function updateDownloaderSettings(patch: Partial<DownloaderSettings>) {
  settings.update((s) => ({ downloader: { ...s.downloader, ...patch } }));
}

export function getDownloaderSettings(): DownloaderSettings {
  return get(settings).downloader;
}
