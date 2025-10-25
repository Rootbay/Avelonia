import { writable } from 'svelte/store';

export type LogLevel = 'INFO' | 'SUCCESS' | 'WARN' | 'ERROR';
export type LogCategory = 'General' | 'Downloader' | 'Cleaner' | 'Optimize' | 'System';
export interface LogEntry {
  timestamp: string;
  level: LogLevel;
  message: string;
  category?: LogCategory;
}

const STORAGE_KEY = 'avelonia_system_logs_v1';
const MAX_LOG_ENTRIES = 500;
// Suppress exact duplicate messages within this window (ms)
const DEDUPE_WINDOW_MS = 1500;

function nowTs(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

function load(): LogEntry[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const arr = raw ? (JSON.parse(raw) as LogEntry[]) : [];
    if (!Array.isArray(arr)) return [];
    return arr.filter((e) => !!e && typeof e.message === 'string');
  } catch {
    return [];
  }
}

export const systemLogs = writable<LogEntry[]>(load());

systemLogs.subscribe((list) => {
  if (typeof window === 'undefined') return;
  try {
    const trimmed = list.slice(-MAX_LOG_ENTRIES);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmed));
  } catch {}
});

// Keep last pushed signature for dedupe
let lastSig = '';
let lastTime = 0;

export function pushLog(level: LogLevel, message: string, category: LogCategory = 'General') {
  const ts = Date.now();
  const sig = `${level}|${category}|${message}`;
  if (sig === lastSig && ts - lastTime <= DEDUPE_WINDOW_MS) {
    return; // suppress burst duplicate
  }
  lastSig = sig;
  lastTime = ts;
  const entry: LogEntry = { timestamp: nowTs(), level, message, category };
  systemLogs.update((list) => [...list.slice(-MAX_LOG_ENTRIES + 1), entry]);
}
