import { beforeEach, describe, expect, it, mock } from 'bun:test';

mock.module('@tauri-apps/api/path', () => ({
  appDataDir: async () => '/appdata',
  downloadDir: async () => '/downloads',
  join: async (dir: string, name: string) => `${dir}/${name}`,
}));

type Download = import('../src/lib/downloadManager').Download;

const baseDownload: Omit<Download, 'id' | 'name' | 'downloadLink'> = {
  description: '',
  size: 'N/A',
  fileType: 'exe',
  category: 'Test',
  tags: ['Test'],
  eta: 'N/A',
  status: 'available',
  progress: 0,
};

function makeDownload(partial: Partial<Download> & Pick<Download, 'id' | 'name' | 'downloadLink'>) {
  return {
    ...baseDownload,
    ...partial,
  } satisfies Download;
}

describe('getDownloadPath', () => {
  beforeEach(async () => {
    const { downloads } = await import('../src/lib/downloads');
    downloads.set([]);
  });

  it('reuses existing targetPath for the same download when available', async () => {
    const { downloads } = await import('../src/lib/downloads');
    const { getDownloadPath } = await import('../src/lib/downloadManager');

    const existing = makeDownload({
      id: 1,
      name: 'Test App',
      downloadLink: 'https://example.com/test.exe',
      targetPath: '/downloads/test-app.exe',
    });

    downloads.set([existing]);

    const resolved = await getDownloadPath(existing);
    expect(resolved).toBe('/downloads/test-app.exe');
  });

  it('chooses a new path when another download already owns the targetPath', async () => {
    const { downloads } = await import('../src/lib/downloads');
    const { getDownloadPath } = await import('../src/lib/downloadManager');

    const other = makeDownload({
      id: 2,
      name: 'Other App',
      downloadLink: 'https://example.com/other.exe',
      targetPath: '/downloads/dup.exe',
    });
    const current = makeDownload({
      id: 1,
      name: 'My App',
      downloadLink: 'https://example.com/my.exe',
      targetPath: '/downloads/dup.exe',
    });

    downloads.set([other, current]);

    const resolved = await getDownloadPath(current);
    expect(resolved).toBe('/downloads/My_App.exe');
  });
});
