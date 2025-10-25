#!/usr/bin/env node
import { spawn } from 'node:child_process';

function run(cmd, args, opts = {}) {
  return new Promise((resolve) => {
    const p = spawn(cmd, args, { stdio: 'inherit', shell: process.platform === 'win32', ...opts });
    p.on('close', (code) => resolve(code ?? 0));
    p.on('error', () => resolve(1));
  });
}

async function main() {
  const results = [];
  console.log('\n=== Frontend: typecheck (svelte-check) ===');
  results.push(['svelte-check', await run('bun', ['run', 'check'])]);

  console.log('\n=== Frontend: ESLint ===');
  results.push(['eslint', await run('bun', ['run', 'lint'])]);

  console.log('\n=== Frontend: Prettier check ===');
  results.push(['prettier', await run('bun', ['run', 'format:check'])]);

  console.log('\n=== Backend: cargo check (src-tauri) ===');
  results.push(['cargo-check', await run('powershell', ['-NoLogo','-NoProfile','-Command','cd src-tauri; cargo check --all-targets --color always'])]);

  console.log('\n=== Summary ===');
  let ok = true;
  for (const [name, code] of results) {
    const status = code === 0 ? 'OK' : 'FAIL';
    console.log(`${name}: ${status}`);
    if (code !== 0) ok = false;
  }
  process.exit(ok ? 0 : 1);
}

main().catch((e) => { console.error(e); process.exit(1); });

