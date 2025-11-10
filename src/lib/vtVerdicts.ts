import { writable } from 'svelte/store';

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

function normalizeKey(s: string): string {
  try {
    return s.trim().toLowerCase();
  } catch {
    return s as unknown as string;
  }
}

const STORAGE_KEY = 'avelonia_vt_verdicts_v1';
const REASONS_KEY = 'avelonia_vt_reasons_v1';

function loadPersisted(): Map<string, VerdictLabel> {
  if (typeof window === 'undefined') return new Map();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return new Map();
    const obj = JSON.parse(raw) as Record<string, VerdictLabel>;
    const m = new Map<string, VerdictLabel>();
    for (const k of Object.keys(obj || {})) {
      const v = obj[k];
      if (v === 'Safe' || v === 'Sus' || v === 'Not') m.set(k, v as VerdictLabel);
    }
    return m;
  } catch {
    return new Map();
  }
}

function persist(map: Map<string, VerdictLabel>) {
  if (typeof window === 'undefined') return;
  try {
    const obj: Record<string, VerdictLabel> = {};
    for (const [k, v] of map.entries()) obj[k] = v;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(obj));
  } catch { /* noop */ }
}

export const vtVerdicts = writable<Map<string, VerdictLabel>>(loadPersisted());

vtVerdicts.subscribe((m) => {
  persist(m);
});

function loadReasons(): Map<string, string> {
  if (typeof window === 'undefined') return new Map();
  try {
    const raw = localStorage.getItem(REASONS_KEY);
    if (!raw) return new Map();
    const obj = JSON.parse(raw) as Record<string, string>;
    const m = new Map<string, string>();
    for (const k of Object.keys(obj || {})) {
      const v = obj[k];
      if (typeof v === 'string') m.set(k, v);
    }
    return m;
  } catch {
    return new Map();
  }
}

function persistReasons(map: Map<string, string>) {
  if (typeof window === 'undefined') return;
  try {
    const obj: Record<string, string> = {};
    for (const [k, v] of map.entries()) obj[k] = v;
    localStorage.setItem(REASONS_KEY, JSON.stringify(obj));
  } catch { /* noop */ }
}

export const vtReasons = writable<Map<string, string>>(loadReasons());
vtReasons.subscribe((m) => persistReasons(m));

export function setVerdict(subject: string, label: VerdictLabel) {
  const key = normalizeKey(subject);
  vtVerdicts.update((m) => {
    const next = new Map(m);
    next.set(key, label);
    return next;
  });
}

export function clearVerdict(subject: string) {
  const key = normalizeKey(subject);
  vtVerdicts.update((m) => {
    const next = new Map(m);
    next.delete(key);
    return next;
  });
  vtReasons.update((m) => {
    const next = new Map(m);
    next.delete(key);
    return next;
  });
}

export function verdictFor(subject: string): VerdictLabel | undefined {
  const key = normalizeKey(subject);
  let out: VerdictLabel | undefined;
  vtVerdicts.subscribe((m) => {
    out = m.get(key);
  })();
  return out;
}

export function reasonFor(subject: string): string | undefined {
  const key = normalizeKey(subject);
  let out: string | undefined;
  vtReasons.subscribe((m) => {
    out = m.get(key);
  })();
  return out;
}

export function setVerdictFromReport(rep: VtReport) {
  const v = String(rep.verdict || '').toLowerCase();
  if (v === 'clean') {
    setVerdict(rep.subject, 'Safe');
    vtReasons.update((m) => {
      const next = new Map(m);
      next.delete(normalizeKey(rep.subject));
      return next;
    });
  } else if (v === 'suspicious' || v === 'malicious') {
    setVerdict(rep.subject, 'Sus');
    vtReasons.update((m) => {
      const next = new Map(m);
      next.delete(normalizeKey(rep.subject));
      return next;
    });
  } else {
    setVerdict(rep.subject, 'Not');
    const rs = (rep.reason || '').toString();
    if (rs) {
      const key = normalizeKey(rep.subject);
      vtReasons.update((m) => {
        const next = new Map(m);
        next.set(key, rs);
        return next;
      });
    }
  }
}
