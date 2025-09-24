import { describe, expect, it } from 'bun:test';
import {
  candidateFileNames,
  normalizeExtension,
  sanitizeFileName,
  __testables__,
} from '../src/lib/downloadPath';

describe('sanitizeFileName', () => {
  it('strips unsupported characters and collapses whitespace', () => {
    expect(sanitizeFileName('Game: Deluxe Edition!')).toBe('Game_Deluxe_Edition');
  });

  it('falls back to a safe placeholder when the result is empty', () => {
    expect(sanitizeFileName('...___')).toBe('download');
  });

  it('caps the length to avoid overly long file names', () => {
    const longName = 'a'.repeat(200);
    expect(sanitizeFileName(longName).length).toBeLessThanOrEqual(
      __testables__.MAX_FILENAME_LENGTH
    );
  });
});

describe('normalizeExtension', () => {
  it('normalises casing and strips unsafe characters', () => {
    expect(normalizeExtension('EXE')).toBe('.exe');
    expect(normalizeExtension(' tar.gz ')).toBe('.tar.gz');
  });

  it('omits the dot when no valid characters remain', () => {
    expect(normalizeExtension('!!!')).toBe('');
  });
});

describe('candidateFileNames', () => {
  it('yields deterministic variants following the familiar pattern', () => {
    const iterator = candidateFileNames('setup', '.exe');
    const variants = [
      iterator.next().value,
      iterator.next().value,
      iterator.next().value,
      iterator.next().value,
    ];
    expect(variants).toEqual(['setup.exe', 'setup (1).exe', 'setup (2).exe', 'setup (3).exe']);
  });
});
