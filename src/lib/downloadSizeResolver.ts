import { BUILT_IN_DOWNLOADS } from './builtInDownloads';

const SIZE_CACHE_KEY = 'avelonia_builtin_download_size_cache';
const CACHE_TTL_MS = 7 * 24 * 60 * 60 * 1000; // refresh once per week

const MAX_SIZE_FETCH_CONCURRENCY = 4;
const IDLE_WAIT_TIMEOUT_MS = 150;

type SizeCacheEntry = {
  size: string;
  updatedAt: number;
};
type SizeCache = Record<string, SizeCacheEntry>;

function loadSizeCache(): SizeCache {
  if (typeof window === 'undefined') return {};
  const raw = localStorage.getItem(SIZE_CACHE_KEY);
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return {};
    const result: SizeCache = {};
    for (const [key, value] of Object.entries(parsed)) {
      if (!value || typeof value !== 'object') continue;
      const entry = value as Record<string, unknown>;
      if (
        typeof entry.size === 'string' &&
        typeof entry.updatedAt === 'number'
      ) {
        result[key] = {
          size: entry.size,
          updatedAt: entry.updatedAt,
        };
      }
    }
    return result;
  } catch {
    return {};
  }
}

function saveSizeCache(cache: SizeCache) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(SIZE_CACHE_KEY, JSON.stringify(cache));
  } catch {
    // best-effort cache
  }
}

function formatBytes(bytes: number, decimals = 1): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return '0 B';
  }
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  const rounded = value.toFixed(decimals).replace(/\.0+$/, '');
  return `${rounded} ${units[index]}`;
}

async function waitForIdle(timeout = IDLE_WAIT_TIMEOUT_MS): Promise<void> {
  if (typeof window !== 'undefined' && typeof window.requestIdleCallback === 'function') {
    return new Promise((resolve) => window.requestIdleCallback(() => resolve(), { timeout }));
  }
  return new Promise((resolve) => setTimeout(resolve, timeout));
}

async function fetchContentLength(url: string): Promise<number | null> {
  if (typeof fetch === 'undefined') {
    return null;
  }
  try {
    const response = await fetch(url, { method: 'HEAD', cache: 'no-cache' });
    if (!response.ok) {
      return null;
    }
    const length = response.headers.get('content-length') ?? response.headers.get('Content-Length');
    if (!length) {
      return null;
    }
    const parsed = Number(length);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

export async function resolveBuiltInDownloadSizes(): Promise<Record<string, string>> {
  if (typeof window === 'undefined') {
    return {};
  }
  const now = Date.now();
  const cache = loadSizeCache();
  const result: Record<string, string> = {};
  const refreshQueue: Array<{ link: string; fallbackSize: string }> = [];

  for (const download of BUILT_IN_DOWNLOADS) {
    const link = download.downloadLink;
    const cached = cache[link];
    const needsRefresh = !cached || now - cached.updatedAt > CACHE_TTL_MS;
    const fallbackSize = cached?.size ?? download.size ?? 'N/A';

    if (!needsRefresh) {
      result[link] = cached!.size;
      continue;
    }

    refreshQueue.push({ link, fallbackSize });
  }

  if (refreshQueue.length > 0) {
    await waitForIdle();
    const queue = refreshQueue.slice();
    const worker = async () => {
      while (true) {
        const job = queue.shift();
        if (!job) {
          return;
        }
        const { link, fallbackSize } = job;
        try {
          const bytes = await fetchContentLength(link);
          const resolvedSize = bytes === null ? fallbackSize : formatBytes(bytes);
          cache[link] = { size: resolvedSize, updatedAt: now };
          result[link] = resolvedSize;
        } catch {
          cache[link] = { size: fallbackSize, updatedAt: now };
          result[link] = fallbackSize;
        }
      }
    };
    const concurrency = Math.min(MAX_SIZE_FETCH_CONCURRENCY, queue.length);
    await Promise.all(Array.from({ length: concurrency }, () => worker()));
    saveSizeCache(cache);
  }

  return result;
}
