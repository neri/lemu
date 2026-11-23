// Same module and process; preparation/warmup are outside each timed sample.
import { readFileSync, writeFileSync } from 'node:fs';
const [path, output] = process.argv.slice(2);
if (!path || !output) throw Error('Usage: node scripts/bench/bench-plum-flat-wasm.mjs bench.wasm output.json');
const { instance } = await WebAssembly.instantiate(readFileSync(path));
const e = instance.exports;
const rows = [];
for (let c = 0; c < 4; c++) {
  const iterations = c < 2 ? 64 : 16384;
  const samples = [[], []];
  for (let sample = 0; sample < 9; sample++) {
    for (const mode of c === 3 ? [1] : (sample % 2 ? [1, 0] : [0, 1])) {
      e.prepare_flat_bench(c, mode);
      e.run_flat_bench(c, 2);
      const start = performance.now();
      const checksum = e.run_flat_bench(c, iterations);
      const ms = performance.now() - start;
      samples[mode].push(ms);
      rows.push({ case: c, flat: !!mode, sample, iterations, checksum, ms });
    }
  }
  const medians = samples.map(s => s.length ? s.toSorted((a, b) => a - b)[4] : null);
  console.log(JSON.stringify({ case: c, split_ms: medians[0], flat_ms: medians[1], ratio: medians[0] ? medians[1] / medians[0] : null }));
}
writeFileSync(output, JSON.stringify({ node: process.version, path, rows }, null, 2));
