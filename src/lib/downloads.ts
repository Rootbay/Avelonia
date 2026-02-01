import { get, writable } from 'svelte/store';
import type { Download } from './downloadManager';
import { settings, updateDownloaderSettings } from '$lib/settings';
import { invoke } from '@tauri-apps/api/core';
import { appDataDir, join } from '@tauri-apps/api/path';
import { loadBuiltInDownloads } from './builtInDownloads';
import { resolveBuiltInDownloadSizes } from './downloadSizeResolver';

const DOWNLOADS_STORAGE_KEY = 'avelonia_downloads';
const DEFAULT_CATALOG_FILENAME = 'avelonia-downloads.json';
const DOWNLOADS_PERSIST_DELAY_MS = 500;
let downloadsPersistTimer: ReturnType<typeof setTimeout> | null = null;
let pendingDownloadsPersist: Download[] | null = null;
let defaultCatalogPathPromise: Promise<string> | null = null;

export type NewDownloadEntry = {
  name: string;
  description?: string;
  size?: string;
  fileType?: string;
  category: string;
  tags?: string[];
  downloadLink: string;
};

function normalizeStoredDownload(download: Download): Download {
  let status = download.status;
  if (status === 'downloading' || status === 'pending' || status === 'queued' || status === 'failed') {
    status = 'available';
  }
  return { ...download, status };
}

function loadDownloadsFromStorage(): Download[] | null {
  if (typeof window === 'undefined') {
    return null;
  }
  const stored = localStorage.getItem(DOWNLOADS_STORAGE_KEY);
  if (!stored) {
    return null;
  }
  try {
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) {
      return null;
    }
    return parsed.map((d: Download) => normalizeStoredDownload(d));
  } catch (error) {
    console.error('Error parsing downloads from localStorage', error);
    return null;
  }
}

const initialStoredDownloads = loadDownloadsFromStorage();
export const downloads = writable<Download[]>(initialStoredDownloads ?? []);

let downloadsInitialized = false;
async function initializeDownloads(hasStored: Download[] | null) {
  if (downloadsInitialized) return;
  downloadsInitialized = true;
  if (typeof window === 'undefined') {
    return;
  }
  try {
    const builtIn = await loadBuiltInDownloads();
    if (!hasStored) {
      downloads.set(builtIn);
    } else {
      await syncBuiltInDownloads(builtIn);
    }
  } catch (error) {
    console.error('Failed to load default downloads', error);
  }
}

async function syncBuiltInDownloads(builtIn: Download[]) {
  downloads.update((current) => {
    let hasChange = false;
    const next = current.map((item) => {
      if (item.id >= 100) return item;

      const match = builtIn.find((b) => b.name === item.name);
      if (match && match.downloadLink !== item.downloadLink) {
        hasChange = true;
        return { ...item, downloadLink: match.downloadLink, size: match.size };
      }
      return item;
    });
    return hasChange ? next : current;
  });
}

if (typeof window !== 'undefined') {
  void initializeDownloads(initialStoredDownloads);
}

async function refreshBuiltInDownloadSizes() {
  if (typeof window === 'undefined') return;
  try {
    const sizeMap = await resolveBuiltInDownloadSizes();
    const links = Object.keys(sizeMap);
    if (!links.length) return;
    downloads.update((current) => {
      let hasChange = false;
      const next = current.map((item) => {
        const nextSize = sizeMap[item.downloadLink];
        if (!nextSize || item.size === nextSize) {
          return item;
        }
        hasChange = true;
        return { ...item, size: nextSize };
      });
      return hasChange ? next : current;
    });
  } catch (error) {
    console.warn('Failed to refresh built-in download sizes', error);
  }
}

function scheduleDownloadPersistence(list: Download[]) {
  if (typeof window === 'undefined') return;
  pendingDownloadsPersist = list;
  if (downloadsPersistTimer !== null) return;

  downloadsPersistTimer = window.setTimeout(() => {
    downloadsPersistTimer = null;
    const payload = pendingDownloadsPersist;
    pendingDownloadsPersist = null;
    if (!payload) return;
    try {
      localStorage.setItem(DOWNLOADS_STORAGE_KEY, JSON.stringify(payload));
    } catch (error) {
      console.error('Failed to persist downloads state', error);
    }
  }, DOWNLOADS_PERSIST_DELAY_MS);
}

downloads.subscribe((currentDownloads) => {
  scheduleDownloadPersistence(currentDownloads);
});

const initialCatalogPath = get(settings).downloader.downloadCatalogPath ?? '';
let catalogWritePath = initialCatalogPath;
let lastHandledCatalogPath = '';
let persistTimer: ReturnType<typeof setTimeout> | null = null;
let pendingCatalogWrite: Download[] | null = null;

if (typeof window !== 'undefined') {
  settings.subscribe((value) => {
    const nextPath = value.downloader.downloadCatalogPath ?? '';
    void handleCatalogPathChange(nextPath);
  });

  downloads.subscribe((list) => {
    if (catalogWritePath) {
      scheduleCatalogPersist(list);
    }
  });

  void ensureDefaultCatalogPath(initialCatalogPath);
  void refreshBuiltInDownloadSizes();
}

function scheduleCatalogPersist(list: Download[]) {
  if (typeof window === 'undefined' || !catalogWritePath) return;
  pendingCatalogWrite = list;
  if (persistTimer !== null) return;
  persistTimer = window.setTimeout(() => {
    persistTimer = null;
    const target = pendingCatalogWrite;
    pendingCatalogWrite = null;
    if (target && catalogWritePath) {
      void persistCatalogFile(catalogWritePath, target);
    }
  }, 800);
}

async function persistCatalogFile(path: string, list: Download[]) {
  if (!path) return;
  try {
    let source = list;
    if (list.length === 0) {
      source = await loadBuiltInDownloads();
      downloads.set(source);
    }
    const entries = source.map((download) => ({
      name: download.name,
      description: download.description || undefined,
      size: download.size,
      fileType: download.fileType,
      category: download.category,
      tags: download.tags,
      downloadLink: download.downloadLink,
    }));
    await invoke('write_download_catalog', {
      path,
      contents: JSON.stringify(entries, null, 2),
    });
  } catch (error) {
    console.error('Failed to persist download catalog', error);
  }
}

async function seedCatalogFromFile(path: string) {
  if (!path || typeof window === 'undefined') return;
  try {
    const raw = (await invoke('read_download_catalog', { path })) as string | null;
    const normalized = raw?.trim();
    if (!normalized) {
      await persistCatalogFile(path, get(downloads));
      return;
    }
    const parsed = JSON.parse(normalized);
    if (!Array.isArray(parsed)) {
      return;
    }
    const entries = parsed
      .map((value) => normalizeCatalogEntry(value))
      .filter((entry): entry is NewDownloadEntry => entry !== null);
    if (entries.length === 0) {
      await persistCatalogFile(path, get(downloads));
      return;
    }
    downloads.update((list) => {
      const existingLinks = new Set(list.map((dl) => dl.downloadLink));
      let nextId = nextDownloadId(list);
      const additions: Download[] = [];
      for (const entry of entries) {
        if (existingLinks.has(entry.downloadLink)) continue;
        additions.push(createDownloadFromEntry(entry, nextId++));
        existingLinks.add(entry.downloadLink);
      }
      return additions.length ? [...list, ...additions] : list;
    });
  } catch (error) {
    console.error('Failed to load download catalog', error);
  }
}

function normalizeCatalogEntry(value: unknown): NewDownloadEntry | null {
  if (!value || typeof value !== 'object') return null;
  const raw = value as Record<string, unknown>;
  const name = typeof raw.name === 'string' ? raw.name.trim() : '';
  const downloadLink = typeof raw.downloadLink === 'string' ? raw.downloadLink.trim() : '';
  if (!name || !downloadLink) return null;
  const entry: NewDownloadEntry = {
    name,
    downloadLink,
    category:
      typeof raw.category === 'string' && raw.category.trim()
        ? raw.category.trim()
        : 'General',
  };
  if (typeof raw.description === 'string' && raw.description.trim()) {
    entry.description = raw.description.trim();
  }
  if (typeof raw.size === 'string' && raw.size.trim()) {
    entry.size = raw.size.trim();
  }
  if (typeof raw.fileType === 'string' && raw.fileType.trim()) {
    entry.fileType = raw.fileType.trim();
  }
  if (Array.isArray(raw.tags)) {
    const tags = raw.tags
      .filter((tag) => typeof tag === 'string' && tag.trim())
      .map((tag) => tag.trim());
    if (tags.length) {
      entry.tags = tags;
    }
  }
  return entry;
}

function createDownloadFromEntry(entry: NewDownloadEntry, id: number): Download {
  return {
    name: entry.name,
    description: entry.description ?? '',
    size: entry.size ?? 'N/A',
    fileType: entry.fileType ?? '',
    category: entry.category,
    tags: entry.tags && entry.tags.length ? entry.tags : [entry.category],
    downloadLink: entry.downloadLink,
    id,
    eta: 'N/A',
    status: 'available',
    progress: 0,
  };
}

async function resolveDefaultCatalogPath(): Promise<string> {
  if (!defaultCatalogPathPromise) {
    defaultCatalogPathPromise = (async () => {
      const dir = await appDataDir();
      return await join(dir, DEFAULT_CATALOG_FILENAME);
    })();
  }
  return defaultCatalogPathPromise;
}

async function ensureDefaultCatalogPath(currentPath: string): Promise<string> {
  if (currentPath) return currentPath;
  const defaultPath = await resolveDefaultCatalogPath();
  const stored = get(settings).downloader.downloadCatalogPath ?? '';
  if (stored && stored !== defaultPath) {
    return stored;
  }
  if (stored === defaultPath) {
    catalogWritePath = defaultPath;
    return defaultPath;
  }
  updateDownloaderSettings({ downloadCatalogPath: defaultPath });
  return defaultPath;
}

async function handleCatalogPathChange(nextPath: string) {
  if (!nextPath) {
    await ensureDefaultCatalogPath(nextPath);
    return;
  }
  if (nextPath === lastHandledCatalogPath) {
    return;
  }
  const previousPath = catalogWritePath;
  catalogWritePath = nextPath;
  lastHandledCatalogPath = nextPath;
  if (previousPath && previousPath !== nextPath) {
    try {
      await invoke('move_download_catalog', { from: previousPath, to: nextPath });
    } catch (error) {
      console.warn('Failed to move catalog file', error);
    }
  }
  await seedCatalogFromFile(nextPath);
  scheduleCatalogPersist(get(downloads));
}

export function nextDownloadId(list: Download[]): number {
  let max = 100;
  for (const d of list) max = Math.max(max, d.id);
  return max + 1;
}

export function addDownload(entry: NewDownloadEntry) {
  downloads.update((list) => {
    const id = nextDownloadId(list);
    const item: Download = {
      name: entry.name,
      description: entry.description ?? '',
      size: entry.size ?? 'N/A',
      fileType: entry.fileType ?? '',
      category: entry.category,
      tags: entry.tags && entry.tags.length ? entry.tags : [entry.category],
      downloadLink: entry.downloadLink,
      id,
      eta: 'N/A',
      status: 'available',
      progress: 0,
    };
    return [...list, item];
  });
}

export function removeDownloadsByIds(ids: number[]) {
  const set = new Set(ids);
  downloads.update((list) => list.filter((d) => !set.has(d.id)));
}
