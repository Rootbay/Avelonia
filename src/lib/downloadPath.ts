const DEFAULT_FILENAME = 'download';
const MAX_FILENAME_LENGTH = 80;

export function sanitizeFileName(input: string): string {
  const normalized = (input ?? '').normalize('NFKD').replace(/[^a-zA-Z0-9._-]+/g, '_');

  const collapsed = normalized.replace(/_+/g, '_');
  const trimmed = collapsed.replace(/^[_.]+|[_.]+$/g, '');
  const safe = trimmed || DEFAULT_FILENAME;

  return safe.slice(0, MAX_FILENAME_LENGTH);
}

export function normalizeExtension(fileType?: string | null): string {
  if (!fileType) return '';
  const segments = fileType
    .normalize('NFKD')
    .split('.')
    .map((segment) => segment.replace(/[^a-zA-Z0-9]+/g, '').toLowerCase())
    .filter((segment) => segment.length > 0);

  if (segments.length === 0) {
    return '';
  }

  return `.${segments.join('.')}`;
}

export function* candidateFileNames(base: string, extension: string): Generator<string> {
  yield `${base}${extension}`;
  let counter = 1;
  while (true) {
    yield `${base} (${counter})${extension}`;
    counter += 1;
  }
}

export const __testables__ = {
  DEFAULT_FILENAME,
  MAX_FILENAME_LENGTH,
};
