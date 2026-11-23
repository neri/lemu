// Compare revisions in one process, preserving the selected memory mode.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const [before, after, output] = process.argv.slice(2);
const paths = [before, after];
if (!before || !after || !output) throw Error('Usage: node scripts/bench/bench-plum-flat-frequency.mjs before.wasm after.wasm output.json');
const manifest = JSON.parse(readFileSync(new URL('../fixtures/frequency-v1/manifest.json', import.meta.url)));
const modules = await Promise.all(paths.map(p => WebAssembly.compile(readFileSync(p))));
const producers = modules.map(m => WebAssembly.Module.customSections(m, 'producers').map(s => new TextDecoder().decode(s)).join(''));
if (!producers[0] || producers[0] !== producers[1]) throw Error('Compiler mismatch');
const entries = await Promise.all(modules.map(async m => (await WebAssembly.instantiate(m)).exports));
const mode = Number(process.argv.find(x => x.startsWith('--mode='))?.slice(7) ?? 1);
if (entries.some(e => e.frequency_uop_size() !== 8)) throw Error('Unexpected Uop size');
const rows = [];
const rounds = Number(process.argv.find(x => x.startsWith('--rounds='))?.slice(9) ?? 3);
const selected = process.argv.find(x => x.startsWith('--cases='))?.slice(8).split(',').map(Number);
for (let round = 0; round < rounds; round++) {
  for (const c of manifest.cases) {
    if (selected && !selected.includes(c.id)) continue;
    const iterations = c.iterations;
    const checksums = entries.map(e => (e.prepare_frequency_mode ? e.prepare_frequency_mode(c.id, mode) : e.prepare_frequency(c.id)) >>> 0);
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
  writeFileSync(output, JSON.stringify({ node: process.version, paths, mode, producers, hashes: paths.map(p => createHash('sha256').update(readFileSync(p)).digest('hex')), rows }, null, 2));
}
