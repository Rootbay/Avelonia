export function extractExeFromCommand(cmd: string): string | null {
  if (!cmd) return null;
  const quoted = cmd.match(/"([^"\\]+?\.exe)"/i);
  if (quoted?.[1]) return quoted[1].split('\\').pop() || quoted[1];
  const bare = cmd.match(/\b([\w .-]+\.exe)\b/i);
  if (bare?.[1]) return bare[1].split('\\').pop() || bare[1];
  return null;
}

export function extractExePathFromCommand(cmd: string): string | null {
  if (!cmd) return null;
  const q = cmd.match(/"([^"\\]+?\.exe)"/i);
  if (q?.[1]) return q[1];
  const b = cmd.match(/\b([a-zA-Z]:\\[^\s"]+?\.exe)\b/);
  if (b?.[1]) return b[1];
  return null;
}

export function splitTaskName(full: string): { base: string; folder: string } {
  if (!full) return { base: '', folder: '' };
  const idx = full.lastIndexOf('\\');
  if (idx <= 0) return { base: full.replace(/^\\+/, ''), folder: '\\' };
  const folder = full.slice(0, idx) || '\\';
  const base = full.slice(idx + 1);
  return { base, folder };
}

export function isCommandSuspicious(cmd: string): boolean {
  const c = (cmd || '').toLowerCase();
  return (
    c.includes('powershell') ||
    c.includes('wscript') ||
    c.includes('cscript') ||
    c.includes('psexec') ||
    c.includes('rundll32') ||
    c.includes('wmic') ||
    c.includes('mshta') ||
    c.includes('regsvr32')
  );
}

export function isHardCommandSuspicious(cmd: string): boolean {
  const lower = (cmd || '').toLowerCase();
  return (
    lower.includes('powershell') ||
    lower.includes('cscript') ||
    lower.includes('wscript') ||
    lower.includes(' PsExec') ||
    lower.includes('\\system32') ||
    lower.includes('\\system32\\windowspowershell') ||
    lower.includes('rundll32') ||
    lower.includes('mshta') ||
    lower.includes('regsvr32')
  );
}

export function isUnderMicrosoft(name: string): boolean {
  const parts = splitTaskName(name);
  return (parts.folder || '').startsWith('\\Microsoft\\Windows');
}
