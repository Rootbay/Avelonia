import { createGzip, constants, createBrotliCompress } from 'node:zlib';
import { createReadStream, createWriteStream, promises as fs } from 'node:fs';
import path from 'node:path';

const root = path.resolve(process.cwd(), 'build');

async function* walk(dir) {
  for (const dirent of await fs.readdir(dir, { withFileTypes: true })) {
    const res = path.resolve(dir, dirent.name);
    if (dirent.isDirectory()) yield* walk(res);
    else yield res;
  }
}

async function compressFile(file) {
  const ext = path.extname(file);
  if (!['.js', '.css', '.html', '.svg', '.json', '.txt', '.xml', '.ico'].includes(ext)) return;

  await Promise.all([
    new Promise((resolve, reject) => {
      const inp = createReadStream(file);
      const out = createWriteStream(file + '.gz');
      const gz = createGzip({ level: constants.Z_BEST_COMPRESSION });
      inp.on('error', reject);
      out.on('error', reject);
      out.on('finish', resolve);
      inp.pipe(gz).pipe(out);
    }),
    new Promise((resolve, reject) => {
      const inp = createReadStream(file);
      const out = createWriteStream(file + '.br');
      const br = createBrotliCompress({ params: { [constants.BROTLI_PARAM_QUALITY]: 11 } });
      inp.on('error', reject);
      out.on('error', reject);
      out.on('finish', resolve);
      inp.pipe(br).pipe(out);
    }),
  ]);
}

(async () => {
  try {
    let count = 0;
    for await (const file of walk(root)) {
      await compressFile(file);
      count++;
    }
    console.log(`[precompress] processed ${count} files under ${root}`);
  } catch (e) {
    console.error('[precompress] failed', e);
    process.exit(1);
  }
})();
