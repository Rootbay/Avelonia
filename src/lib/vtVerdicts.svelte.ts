import { SvelteMap } from 'svelte/reactivity';
import { pushLog } from '$lib/logStore';

export type VerdictLabel = 'Safe' | 'Sus' | 'Not';

export type VtReport = {
  subject: string;
  sha256: string;
  verdict: 'Clean' | 'Suspicious' | 'Malicious' | 'Unknown';
  positives?: number;
  permalink?: string;
  source?: string;
  malicious?: number;
  suspicious?: number;
  harmless?: number;
  undetected?: number;
  total_vendors?: number;
  reason?: string;
};

function logVtError(context: string, error: unknown) {
  pushLog('WARN', `VT ${context} failed: ${String(error)}`, 'Optimize');
}

function normalizeKey(s: string): string {
  try {
    return s.trim().toLowerCase();
  } catch (error) {
    logVtError('normalizeKey', error);
    return String(s).trim().toLowerCase();
  }
}

const STORAGE_KEY = 'avelonia_vt_verdicts_v1';
const REASONS_KEY = 'avelonia_vt_reasons_v1';

function loadPersisted(): SvelteMap<string, VerdictLabel> {
  if (typeof window === 'undefined') return new SvelteMap();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return new SvelteMap();
    const obj = JSON.parse(raw) as Record<string, VerdictLabel>;
    const m = new SvelteMap<string, VerdictLabel>();
    for (const k of Object.keys(obj || {})) {
      const v = obj[k];
      if (v === 'Safe' || v === 'Sus' || v === 'Not') m.set(k, v as VerdictLabel);
    }
    return m;
  } catch (error) {
    logVtError('loadPersisted', error);
    return new SvelteMap();
  }
}

function persist(map: SvelteMap<string, VerdictLabel> | Map<string, VerdictLabel>) {
  if (typeof window === 'undefined') return;
  try {
    const obj: Record<string, VerdictLabel> = {};
    for (const [k, v] of map.entries()) obj[k] = v;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(obj));
  } catch (error) {
    logVtError('persist', error);
  }
}

function loadReasons(): SvelteMap<string, string> {
  if (typeof window === 'undefined') return new SvelteMap();
  try {
    const raw = localStorage.getItem(REASONS_KEY);
    if (!raw) return new SvelteMap();
    const obj = JSON.parse(raw) as Record<string, string>;
    const m = new SvelteMap<string, string>();
    for (const k of Object.keys(obj || {})) {
      const v = obj[k];
      if (typeof v === 'string') m.set(k, v);
    }
    return m;
  } catch (error) {
    logVtError('loadReasons', error);
    return new SvelteMap();
  }
}

function persistReasons(map: SvelteMap<string, string> | Map<string, string>) {
  if (typeof window === 'undefined') return;
  try {
    const obj: Record<string, string> = {};
    for (const [k, v] of map.entries()) obj[k] = v;
    localStorage.setItem(REASONS_KEY, JSON.stringify(obj));
  } catch (error) {
    logVtError('persistReasons', error);
  }
}

export const vtVerdicts = loadPersisted();
export const vtReasons = loadReasons();

// Persistence logic using $effect (needs to be in a svelte context, or use a workaround)
// For now, we'll manually persist on changes or use a simple subscribe-like pattern if possible.
// Actually, in Svelte 5, we can use a class with getters/setters or just call persist functions.

function handlePersist() {
  persist(vtVerdicts);
}

function handlePersistReasons() {
  persistReasons(vtReasons);
}

export function setVerdict(subject: string, label: VerdictLabel) {
  const key = normalizeKey(subject);
  vtVerdicts.set(key, label);
  handlePersist();
}

export function clearVerdict(subject: string) {
  const key = normalizeKey(subject);
  vtVerdicts.delete(key);
  vtReasons.delete(key);
  handlePersist();
  handlePersistReasons();
}

export function verdictFor(subject: string): VerdictLabel | undefined {
  const key = normalizeKey(subject);
  return vtVerdicts.get(key);
}

export function reasonFor(subject: string): string | undefined {
  const key = normalizeKey(subject);
  return vtReasons.get(key);
}

export function setVerdictFromReport(rep: VtReport) {
  const v = String(rep.verdict || '').toLowerCase();
  const key = normalizeKey(rep.subject);
  if (v === 'clean') {
    setVerdict(rep.subject, 'Safe');
    vtReasons.delete(key);
    handlePersistReasons();
  } else if (v === 'suspicious' || v === 'malicious') {
    setVerdict(rep.subject, 'Sus');
    vtReasons.delete(key);
    handlePersistReasons();
  } else {
    setVerdict(rep.subject, 'Not');
    const rs = (rep.reason || '').toString();
    if (rs) {
      vtReasons.set(key, rs);
      handlePersistReasons();
    }
  }
}
