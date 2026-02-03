import { beforeEach, describe, expect, it } from 'bun:test';
import { get } from 'svelte/store';
import { addDownload, downloads, nextDownloadId, removeDownloadsByIds } from '../src/lib/downloads';
import type { Download } from '../src/lib/downloadManager';

const base = (overrides: Partial<Download>): Download => ({
  id: 100,
  name: 'Test',
  description: '',
  size: 'N/A',
  fileType: '',
  category: 'General',
  tags: ['General'],
  downloadLink: 'https://example.com/test.exe',
  eta: 'N/A',
  status: 'available',
  progress: 0,
  ...overrides,
});

describe('downloads store helpers', () => {
  beforeEach(() => {
    downloads.set([]);
  });

  it('nextDownloadId picks the next highest id', () => {
    const list = [base({ id: 100 }), base({ id: 150 }), base({ id: 3 })];
    expect(nextDownloadId(list)).toBe(151);
  });

  it('addDownload inserts an entry with defaults', () => {
    addDownload({
      name: 'My App',
      category: 'Utilities',
      downloadLink: 'https://example.com/app.exe',
    });

    const list = get(downloads);
    expect(list).toHaveLength(1);
    expect(list[0]).toMatchObject({
      name: 'My App',
      category: 'Utilities',
      description: '',
      size: 'N/A',
      fileType: '',
      status: 'available',
      progress: 0,
      eta: 'N/A',
      downloadLink: 'https://example.com/app.exe',
    });
    expect(list[0].tags).toEqual(['Utilities']);
  });

  it('removeDownloadsByIds drops only matching entries', () => {
    downloads.set([base({ id: 1 }), base({ id: 2 }), base({ id: 3 })]);

    removeDownloadsByIds([2, 99]);

    const ids = get(downloads).map((d) => d.id);
    expect(ids).toEqual([1, 3]);
  });
});
