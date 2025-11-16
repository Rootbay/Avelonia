import { writable, get } from 'svelte/store';

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

function logSettingsError(context: string, error: unknown) {
  console.warn(`[Settings] ${context}`, error);
}

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
        downloadCatalogPath: '',
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
          downloadCatalogPath: '',
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
      downloadCatalogPath: typeof parsed?.downloader?.downloadCatalogPath === 'string'
        ? parsed.downloader.downloadCatalogPath
        : '',
    };
    return { downloader: d };
  } catch (error) {
    logSettingsError('load', error);
    return {
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
  }
}

export const settings = writable<AppSettings>(load());

settings.subscribe((s) => {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
  } catch (error) {
    logSettingsError('persist', error);
  }
});

export function updateDownloaderSettings(patch: Partial<DownloaderSettings>) {
  settings.update((s) => ({ downloader: { ...s.downloader, ...patch } }));
}

export function getDownloaderSettings(): DownloaderSettings {
  return get(settings).downloader;
}
