#!/usr/bin/env node
import fs from 'node:fs';
const [,, file, startStr, endStr] = process.argv;
const start = Math.max(1, parseInt(startStr||'1', 10));
const end = parseInt(endStr||String(start+200), 10);
const text = fs.readFileSync(file, 'utf8');
const lines = text.split(/\r?\n/);
for (let i = start; i <= Math.min(end, lines.length); i++) {
  console.log(String(i).padStart(4,' ')+': '+lines[i-1]);
}
