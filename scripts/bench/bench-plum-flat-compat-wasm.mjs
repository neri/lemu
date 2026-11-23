// Compare the unchanged split-mode CPU guard against a baseline build.
import { readFileSync, writeFileSync } from 'node:fs';
const [before, after, output] = process.argv.slice(2);
if (!before || !after || !output) throw Error('Usage: node scripts/bench/bench-plum-flat-compat-wasm.mjs before.wasm after.wasm output.json');
const exports = await Promise.all([before, after].map(async path =>
  (await WebAssembly.instantiate(readFileSync(path))).instance.exports));
for (const e of exports) e.bench_cpu_dispatch(256);
const rows = [];
for (let round = 0; round < 3; round++) {
  const samples = [[], []];
  let expected;
  for (let sample = 0; sample < 9; sample++) {
    for (const stage of sample % 2 ? [1, 0] : [0, 1]) {
      const start = performance.now();
      const checksum = exports[stage].bench_cpu_dispatch(512);
      const ms = performance.now() - start;
      expected ??= checksum;
      if (checksum !== expected) throw Error('Checksum mismatch');
      samples[stage].push(ms);
      rows.push({ round, stage, sample, checksum, ms });
    }
  }
  console.log(JSON.stringify({ round, medians_ms: samples.map(s => s.toSorted((a, b) => a - b)[4]) }));
}
writeFileSync(output, JSON.stringify({ node: process.version, before, after, rows }, null, 2));
