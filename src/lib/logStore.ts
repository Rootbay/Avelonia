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
const DEDUPE_WINDOW_MS = 1500;
const MESSAGE_COOLDOWN_MS: Record<string, number> = {
  'VirusTotal is ready.': 60_000,
  'VirusTotal scan scheduled in background.': 60_000,
  'VirusTotal up to date — scan skipped.': 60_000,
};
let lastSig = '';
let lastTime = 0;
const recentMsgTimes = new Map<string, number>();

function nowTs(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

function logStorageError(context: string, error: unknown) {
  void context;
  void error;
}

function load(): LogEntry[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const arr = raw ? (JSON.parse(raw) as LogEntry[]) : [];
    if (!Array.isArray(arr)) return [];
    return arr.filter((e) => !!e && typeof e.message === 'string');
  } catch (error) {
    logStorageError('load', error);
    return [];
  }
}

export const systemLogs = writable<LogEntry[]>(load());

systemLogs.subscribe((list) => {
  if (typeof window === 'undefined') return;
  try {
    const trimmed = list.slice(0, MAX_LOG_ENTRIES);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmed));
  } catch (error) {
    logStorageError('persist', error);
  }
});

function friendlyMap(
  level: LogLevel,
  message: string,
  category: LogCategory
): { level: LogLevel; message: string; category: LogCategory } | null {
  const cat = category || 'General';
  const msg = String(message || '');
  if (cat === 'Optimize') {
    if (/^VT report:/i.test(msg)) {
      return null;
    }
    if (/^VT detection:/i.test(msg)) {
      const m = msg.match(/^VT detection:\s*(.+?)(?:\s*\((\d+)\s*vendors\))?/i);
      const name = m?.[1]?.trim() || 'Item';
      const vendors = m?.[2] ? ` — flagged by ${m[2]} vendors` : '';
      return { level, message: `Security alert: ${name}${vendors}`, category: 'Optimize' };
    }
    if (/^VT cache loaded:/i.test(msg)) {
      return { level: 'INFO', message: 'VirusTotal is ready.', category: 'Optimize' };
    }
    if (/^VT key detected/i.test(msg)) {
      return {
        level: 'INFO',
        message: 'VirusTotal scan scheduled in background.',
        category: 'Optimize',
      };
    }
    if (/^VT scan starting/i.test(msg)) {
      return { level: 'INFO', message: 'VirusTotal scan started.', category: 'Optimize' };
    }
    if (/^VT scan finished/i.test(msg)) {
      const m = msg.match(/startup\s+(\d+).*registry\s+(\d+)/i);
      const total = m ? parseInt(m[1] || '0') + parseInt(m[2] || '0') : undefined;
      const suffix = Number.isFinite(total as number) ? ` — checked ${total} items` : '';
      return {
        level: 'SUCCESS',
        message: `VirusTotal scan completed${suffix}.`,
        category: 'Optimize',
      };
    }
    if (/^VT scan skipped/i.test(msg)) {
      return {
        level: 'INFO',
        message: 'VirusTotal up to date — scan skipped.',
        category: 'Optimize',
      };
    }
    if (/^VT scan failed/i.test(msg)) {
      return { level: 'ERROR', message: 'VirusTotal scan failed.', category: 'Optimize' };
    }
    if (/^Saving VT key failed/i.test(msg) || /^Failed to save VirusTotal key/i.test(msg)) {
      return { level: 'ERROR', message: 'Could not save VirusTotal key.', category: 'Optimize' };
    }
    if (/^VT key saved/i.test(msg)) {
      return { level: 'SUCCESS', message: 'VirusTotal key saved.', category: 'Optimize' };
    }
  }
  return { level, message, category: cat };
}

export function pushLog(level: LogLevel, message: string, category: LogCategory = 'General') {
  const ts = Date.now();
  const mapped = friendlyMap(level, message, category);
  if (!mapped) return;
  const sig = `${mapped.level}|${mapped.category}|${mapped.message}`;
  if (sig === lastSig && ts - lastTime <= DEDUPE_WINDOW_MS) {
    return;
  }
  const lastMsgTs = recentMsgTimes.get(mapped.message) || 0;
  if (ts - lastMsgTs <= DEDUPE_WINDOW_MS) {
    return;
  }
  const cd = MESSAGE_COOLDOWN_MS[mapped.message];
  if (typeof cd === 'number' && cd > 0) {
    if (ts - lastMsgTs <= cd) return;
  }
  lastSig = sig;
  lastTime = ts;
  recentMsgTimes.set(mapped.message, ts);
  const entry: LogEntry = {
    timestamp: nowTs(),
    level: mapped.level,
    message: mapped.message,
    category: mapped.category,
  };
  systemLogs.update((list) => [entry, ...list].slice(0, MAX_LOG_ENTRIES));
}

export function clearLogs() {
  try {
    systemLogs.set([]);
    if (typeof window !== 'undefined') localStorage.removeItem(STORAGE_KEY);
  } catch (error) {
    logStorageError('clear', error);
  }
}


