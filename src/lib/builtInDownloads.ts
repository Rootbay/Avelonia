import type { Download } from './downloadManager';

function cloneDownload(download: Download): Download {
  return {
    ...download,
    tags: download.tags ? [...download.tags] : undefined,
    releases: download.releases
      ? download.releases.map((release) => ({ ...release }))
      : undefined,
  };
}

let cachedRawDownloads: Download[] | null = null;

async function loadBuiltInRawDownloads(): Promise<Download[]> {
  if (cachedRawDownloads) {
    return cachedRawDownloads;
  }
  const module = await import('./builtInDownloadsData');
  cachedRawDownloads = module.BUILT_IN_DOWNLOAD_GROUPS.flatMap((group) => group.downloads);
  return cachedRawDownloads;
}

export async function loadBuiltInDownloads(): Promise<Download[]> {
  const raw = await loadBuiltInRawDownloads();
  return raw.map(cloneDownload);
}
