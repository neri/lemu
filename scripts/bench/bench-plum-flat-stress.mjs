import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
const path = 'target/wasm32-unknown-unknown/release/examples/flat_memory_stress.wasm';
const bytes = readFileSync(path);
const { instance } = await WebAssembly.instantiate(bytes);
const e = instance.exports;
// Tier up the write/invalidate/decode paths before collecting short samples.
for (let mode = 0; mode < 3; mode++) {
  e.prepare_stress(16);
  for (let warmup = 0; warmup < 10; warmup++) e.run_stress(mode, 1024);
}
const rows = [];
for (const functions of [16, 256, 4096]) {
  const iterations = 16384 / functions;
  for (let mode = 0; mode < 3; mode++) {
    const samples = [];
    let cached;
    const expected = functions * iterations + (mode === 2 ? iterations / 2 : 0);
    for (let sample = 0; sample < 7; sample++) {
      cached = e.prepare_stress(functions);
      for (let warmup = 0; warmup < 3; warmup++) e.run_stress(mode, iterations);
      const start = performance.now();
      const actual = e.run_stress(mode, iterations) >>> 0;
      samples.push(performance.now() - start);
      if (actual !== expected) throw Error(`Checksum: ${functions}, ${mode}`);
    }
    const row = { functions, mode, iterations, cached_uops: cached, checksum: expected, samples_ms: samples, median_ms: samples.toSorted((a,b) => a-b)[3] };
    rows.push(row);
    console.log(JSON.stringify(row));
  }
}
mkdirSync('target/bench/plum-flat-stress', { recursive: true });
writeFileSync('target/bench/plum-flat-stress/wasm.json', JSON.stringify({ path, node: process.version, sha256: createHash('sha256').update(bytes).digest('hex'), affinity: [0], rows }, null, 2));
