import { writable, get } from 'svelte/store';

export type DownloaderSettings = {
  autoInstall: boolean;
  installMode: 'silent' | 'normal';
  elevate: boolean;
  fallbackOpen: boolean;
  verifyInstall: boolean;
  preferWinget: boolean;
};

export type AppSettings = {
  downloader: DownloaderSettings;
};

const STORAGE_KEY = 'avelonia_settings_v1';

function load(): AppSettings {
  if (typeof window === 'undefined') {
    return {
      downloader: {
        autoInstall: false,
        installMode: 'silent',
        elevate: true,
        fallbackOpen: true,
        verifyInstall: false,
        preferWinget: false,
      },
    };
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw)
      return {
        downloader: {
          autoInstall: false,
          installMode: 'silent',
          elevate: true,
          fallbackOpen: true,
          verifyInstall: false,
          preferWinget: false,
        },
      };
    const parsed = JSON.parse(raw);
    const d: DownloaderSettings = {
      autoInstall: !!parsed?.downloader?.autoInstall,
      installMode: parsed?.downloader?.installMode === 'normal' ? 'normal' : 'silent',
      elevate: parsed?.downloader?.elevate ?? true,
      fallbackOpen: parsed?.downloader?.fallbackOpen ?? true,
      verifyInstall: parsed?.downloader?.verifyInstall ?? false,
      preferWinget: parsed?.downloader?.preferWinget ?? false,
    };
    return { downloader: d };
  } catch {
    return {
      downloader: {
        autoInstall: false,
        installMode: 'silent',
        elevate: true,
        fallbackOpen: true,
        verifyInstall: false,
        preferWinget: false,
      },
    };
  }
}

export const settings = writable<AppSettings>(load());

settings.subscribe((s) => {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
  } catch { /* noop */ }
});

export function updateDownloaderSettings(patch: Partial<DownloaderSettings>) {
  settings.update((s) => ({ downloader: { ...s.downloader, ...patch } }));
}

export function getDownloaderSettings(): DownloaderSettings {
  return get(settings).downloader;
}
