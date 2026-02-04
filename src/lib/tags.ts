import { writable } from 'svelte/store';
import { pushLog } from '$lib/logStore';

const STORAGE_KEY = 'avelonia_tags_custom';

function logTagsError(context: string, error: unknown) {
  pushLog('WARN', `Tags ${context} failed: ${String(error)}`, 'General');
}

export const BUILT_IN_TAGS = [
  'Browser',
  'Utility',
  'Multimedia',
  'Communication',
  'Security',
  'Productivity',
  'Development',
  'Games',
  'Drivers',
  'Compression',
  'Office',
];

function loadCustom(): string[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const arr = JSON.parse(raw);
    return Array.isArray(arr) ? arr.filter((s) => typeof s === 'string') : [];
  } catch (error) {
    logTagsError('loadCustom', error);
    return [];
  }
}

export const tags = writable<string[]>(loadCustom());

tags.subscribe((list) => {
  if (typeof window === 'undefined') return;
  try {
    const uniq = Array.from(new Set(list.map((s) => s.trim()).filter(Boolean)));
    localStorage.setItem(STORAGE_KEY, JSON.stringify(uniq));
  } catch (error) {
    logTagsError('persist', error);
  }
});

export function addTag(name: string) {
  const n = name.trim();
  if (!n) return;
  tags.update((list) => {
    if (list.includes(n)) return list;
    return [...list, n];
  });
}
