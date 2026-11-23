// Compare modes using identical compiler-generated guests, separate instances.
import { readFileSync, writeFileSync } from 'node:fs';
const [split, flat, output] = process.argv.slice(2);
const paths = [split, flat];
if (!split || !flat || !output) throw Error('Usage: node scripts/bench/bench-plum-flat-frequency.mjs frequency_bench.wasm flat_frequency_bench.wasm output.json');
const manifest = JSON.parse(readFileSync(new URL('../fixtures/frequency-v1/manifest.json', import.meta.url)));
const modules = await Promise.all(paths.map(path => WebAssembly.compile(readFileSync(path))));
const entries = await Promise.all(modules.map(async module => (await WebAssembly.instantiate(module)).exports));
if (entries.some(e => e.frequency_uop_size() !== 8)) throw Error('Unexpected Uop size');
const rows = [];
const rounds = Number(process.argv.find(x => x.startsWith('--rounds='))?.slice(9) ?? 3);
const selected = process.argv.find(x => x.startsWith('--cases='))?.slice(8).split(',').map(Number);
for (let round = 0; round < rounds; round++) {
  for (const c of manifest.cases) {
    if (selected && !selected.includes(c.id)) continue;
    const iterations = c.iterations;
    const checksums = entries.map(e => e.prepare_frequency(c.id) >>> 0);
    if (checksums[0] !== checksums[1]) throw Error(`Checksum mismatch: ${c.name}`);
    const expected = Math.imul(checksums[0], iterations) >>> 0;
    for (const e of entries) for (let i = 0; i < 3; i++) {
      if ((e.run_frequency(iterations) >>> 0) !== expected) throw Error('Warmup checksum');
    }
    const samples = [[], []];
    for (let sample = 0; sample < 7; sample++) {
      for (const mode of (sample + round) % 2 ? [1, 0] : [0, 1]) {
        const start = performance.now();
        const actual = entries[mode].run_frequency(iterations) >>> 0;
        samples[mode].push(performance.now() - start);
        if (actual !== expected) throw Error(`Sample checksum: ${c.name}`);
      }
    }
    const medians = samples.map(s => s.toSorted((a, b) => a - b)[3]);
    rows.push({ round, case: c.id, name: c.name, iterations, checksum: expected, samples_ms: samples, medians_ms: medians });
    console.log(JSON.stringify({ round, name: c.name, medians_ms: medians, ratio: medians[1] / medians[0] }));
  }
  writeFileSync(output, JSON.stringify({ node: process.version, paths, rows }, null, 2));
}
