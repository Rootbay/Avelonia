const START_NOISE = [
  'setup',
  'installer',
  'install',
  'download',
  'official',
];

const END_NOISE = [
  'x64',
  'x86',
  'win',
  'win64',
  'win32',
  '64bit',
  '32bit',
  '64-bit',
  '32-bit',
  'latest',
];

const REGION_SUFFIX = [
  'na','us','uk','eu','en','se','de','fr','es','pt','ru','jp','cn','br','it','nl','pl','tr','cz','sk','fi','no','dk',
  'en-us','en-gb','es-es','pt-br','zh-cn','zh-tw'
];

const STOP_WORDS = new Set(['of','and','the','for','to','a','an','in','on','at','by','with']);

function stripExtension(name: string, ext?: string): string {
  if (ext) {
    const norm = ext.replace(/^\./, '').toLowerCase();
    const rx = new RegExp(`\\.${norm}$`, 'i');
    return name.replace(rx, '');
  }
  const idx = name.lastIndexOf('.');
  if (idx > 0) return name.slice(0, idx);
  return name;
}

function titleCase(input: string): string {
  const parts = input.split(/\s+/).filter(Boolean);
  return parts
    .map((w, i) => {
      const lower = w.toLowerCase();
      if (i > 0 && STOP_WORDS.has(lower)) return lower;
      return lower.charAt(0).toUpperCase() + lower.slice(1);
    })
    .join(' ');
}

function removeNoise(raw: string): string {
  let s = raw.trim();
  // leading noise like "Install:", "Setup -"
  s = s.replace(new RegExp(`^(?:${START_NOISE.join('|')})[\s:,-]+`, 'i'), '');
  // replace separators with spaces
  s = s.replace(/[._-]+/g, ' ');
  s = s.replace(/\s+/g, ' ').trim();

  // remove trailing bracketed qualifiers if they are only short codes
  s = s.replace(/\s*[([\{]\s*([\w-]{1,8})\s*[)\]\}]\s*$/i, (m, g1) => {
    const token = String(g1).toLowerCase();
    if (REGION_SUFFIX.includes(token) || END_NOISE.includes(token)) return '';
    return m; // keep if not known noise
  });

  // drop known trailing noise tokens
  const tokens = s.split(' ');
  while (tokens.length > 1) {
    const last = tokens[tokens.length - 1].toLowerCase();
    const lastClean = last.replace(/[^a-z0-9-]/gi, '');
    if (END_NOISE.includes(lastClean) || REGION_SUFFIX.includes(lastClean)) {
      tokens.pop();
    } else {
      break;
    }
  }
  return tokens.join(' ').trim();
}

export function prettifyDisplayName(raw: string, ext?: string): string {
  let base = stripExtension(raw ?? '', ext);
  base = base.normalize('NFKC');
  base = removeNoise(base);
  base = base.replace(/^\s+|\s+$/g, '');
  base = base.replace(/\s{2,}/g, ' ');
  if (!base) base = 'download';
  return titleCase(base);
}

